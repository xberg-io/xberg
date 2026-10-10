//! Sceptre OCR backend.
//!
//! Sceptre runs CRAFT text detection followed by CRNN recognition through ONNX
//! Runtime, the pure-Rust tract engine, or hand-written networks over candle
//! (CPU, Metal, or CUDA). Readers are initialized lazily and cached by their
//! effective model and inference configuration.
//!
//! [`build_document`] applies a document-assembly pass sceptre itself has no
//! concept of: [`assign_sceptre_block_ids`] groups recognized lines into
//! paragraph blocks by geometry, mirroring PaddleOCR's `assign_line_block_ids`.

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex, Weak};

use async_trait::async_trait;
use once_cell::sync::OnceCell;
use sceptre::{Backend, Image, Language, OcrResult, ReadOptions, Reader, TextLine};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::core::config::{ExecutionProviderType, OcrConfig};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::sceptre_languages::{language_group, language_group_name, supported_language_aliases};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{
    BoundingBox, ExtractedDocument, FormatMetadata, Metadata, OcrBoundingGeometry, OcrConfidence, OcrElement,
    OcrElementLevel, OcrMetadata,
};
use crate::{Result, XbergError};

const BACKEND_NAME: &str = "sceptre";
const DEFAULT_LANGUAGE: &str = "eng";
// The metadata key names come from the ungated `crate::ocr_metadata_keys` rather than
// `crate::ocr`, which is gated on `feature = "ocr"` / `"ocr-wasm"` — neither of which a
// Sceptre-only build implies. ~keep
use crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY as PROCESSED_HEIGHT_KEY;
use crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY as PROCESSED_WIDTH_KEY;
#[cfg(auto_rotate)]
use crate::ocr_metadata_keys::{
    OCR_AUTO_ROTATED_METADATA_KEY as AUTO_ROTATED_KEY,
    OCR_ORIENTATION_CONFIDENCE_METADATA_KEY as ORIENTATION_CONFIDENCE_KEY,
    OCR_ORIENTATION_DEGREES_METADATA_KEY as ORIENTATION_DEGREES_KEY,
};
// Each reader retains multiple ONNX sessions and a worker pool. Four entries
// cover common language/configuration reuse while bounding retained resources. ~keep
const READER_CACHE_CAPACITY: usize = 4;
#[cfg(auto_rotate)]
const ORIENTATION_DETECTOR_CACHE_CAPACITY: usize = 4;
// A transient overflow slot prevents requests from blocking when every cached
// reader is active while still bounding heavyweight reader initialization. ~keep
const READER_OVERFLOW_CAPACITY: usize = 1;
// A Sceptre reader already parallelizes detection and recognition internally.
// Admit one operation at a time so cached readers cannot multiply the process
// thread budget during concurrent extraction. ~keep
const EXECUTION_CAPACITY: usize = 1;

type ReaderCell = Arc<ReaderSlot>;

struct ReaderSlot {
    reader: OnceCell<Reader>,
    _overflow_permit: Option<OwnedSemaphorePermit>,
}

impl ReaderSlot {
    fn cached() -> Self {
        Self {
            reader: OnceCell::new(),
            _overflow_permit: None,
        }
    }

    fn overflow(permit: OwnedSemaphorePermit) -> Self {
        Self {
            reader: OnceCell::new(),
            _overflow_permit: Some(permit),
        }
    }
}

struct ReaderCache {
    entries: HashMap<String, ReaderCell>,
    overflow: HashMap<String, Weak<ReaderSlot>>,
    recency: VecDeque<String>,
}

#[cfg(auto_rotate)]
type OrientationDetectorCell = Arc<crate::doc_orientation::DocOrientationDetector>;

#[cfg(auto_rotate)]
struct OrientationDetectorCache {
    entries: HashMap<String, OrientationDetectorCell>,
    recency: VecDeque<String>,
}

#[cfg(auto_rotate)]
impl OrientationDetectorCache {
    fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(ORIENTATION_DETECTOR_CACHE_CAPACITY),
            recency: VecDeque::with_capacity(ORIENTATION_DETECTOR_CACHE_CAPACITY),
        }
    }

    fn detector(
        &mut self,
        cache_dir: &Path,
        acceleration: Option<crate::core::config::AccelerationConfig>,
    ) -> Result<OrientationDetectorCell> {
        let key = orientation_detector_cache_key(cache_dir, acceleration.as_ref())?;
        if let Some(detector) = self.entries.get(&key).cloned() {
            self.mark_recent(&key);
            return Ok(detector);
        }

        if self.entries.len() == ORIENTATION_DETECTOR_CACHE_CAPACITY && !self.evict_oldest_inactive() {
            return Err(ocr_error(format!(
                "Sceptre orientation detector cache is at its bounded capacity of {ORIENTATION_DETECTOR_CACHE_CAPACITY} active configurations"
            )));
        }

        let detector = Arc::new(crate::doc_orientation::DocOrientationDetector::with_acceleration(
            cache_dir.to_path_buf(),
            acceleration,
        ));
        self.entries.insert(key.clone(), Arc::clone(&detector));
        self.recency.push_back(key);
        Ok(detector)
    }

    fn mark_recent(&mut self, key: &str) {
        if let Some(index) = self.recency.iter().position(|cached_key| cached_key == key) {
            self.recency.remove(index);
        }
        self.recency.push_back(key.to_string());
    }

    fn evict_oldest_inactive(&mut self) -> bool {
        let Some(index) = self.recency.iter().position(|key| {
            self.entries
                .get(key)
                .is_some_and(|detector| Arc::strong_count(detector) == 1)
        }) else {
            return false;
        };
        let Some(key) = self.recency.remove(index) else {
            return false;
        };
        self.entries.remove(&key);
        true
    }
}

impl ReaderCache {
    fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(READER_CACHE_CAPACITY),
            overflow: HashMap::with_capacity(READER_OVERFLOW_CAPACITY),
            recency: VecDeque::with_capacity(READER_CACHE_CAPACITY),
        }
    }

    fn reader_cell(&mut self, key: &str) -> Option<ReaderCell> {
        if let Some(cell) = self.entries.get(key).cloned() {
            self.mark_recent(key);
            return Some(cell);
        }

        self.overflow.retain(|_, cell| cell.strong_count() > 0);
        if let Some(cell) = self.overflow.get(key).and_then(Weak::upgrade) {
            return Some(cell);
        }

        if self.entries.len() == READER_CACHE_CAPACITY && !self.evict_oldest_inactive() {
            return None;
        }

        let cell = Arc::new(ReaderSlot::cached());
        self.entries.insert(key.to_string(), Arc::clone(&cell));
        self.recency.push_back(key.to_string());
        Some(cell)
    }

    fn insert_overflow(&mut self, key: &str, permit: OwnedSemaphorePermit) -> ReaderCell {
        let cell = Arc::new(ReaderSlot::overflow(permit));
        self.overflow.insert(key.to_string(), Arc::downgrade(&cell));
        cell
    }

    fn mark_recent(&mut self, key: &str) {
        if let Some(index) = self.recency.iter().position(|cached_key| cached_key == key) {
            self.recency.remove(index);
        }
        self.recency.push_back(key.to_string());
    }

    fn evict_oldest_inactive(&mut self) -> bool {
        let Some(index) = self
            .recency
            .iter()
            .position(|key| self.entries.get(key).is_some_and(|cell| Arc::strong_count(cell) == 1))
        else {
            return false;
        };
        let Some(key) = self.recency.remove(index) else {
            return false;
        };
        self.entries.remove(&key);
        true
    }
}

/// Sceptre-backed OCR implementation.
///
/// Reader construction resolves model files and creates inference sessions, so
/// it is deferred until the first call for an effective configuration. The
/// initialized reader is reused without holding the cache lock during inference;
/// backend-wide admission prevents cached readers from oversubscribing the host.
#[cfg_attr(alef, alef(skip))]
pub struct SceptreOcrBackend {
    readers: Arc<Mutex<ReaderCache>>,
    overflow_slots: Arc<Semaphore>,
    execution_slots: Arc<Semaphore>,
    #[cfg(auto_rotate)]
    orientation_detectors: Arc<Mutex<OrientationDetectorCache>>,
}

impl SceptreOcrBackend {
    /// Create an empty, lazily initialized Sceptre backend.
    pub fn new() -> Result<Self> {
        Ok(Self {
            readers: Arc::new(Mutex::new(ReaderCache::new())),
            overflow_slots: Arc::new(Semaphore::new(READER_OVERFLOW_CAPACITY)),
            execution_slots: Arc::new(Semaphore::new(EXECUTION_CAPACITY)),
            #[cfg(auto_rotate)]
            orientation_detectors: Arc::new(Mutex::new(OrientationDetectorCache::new())),
        })
    }

    async fn reader_cell(&self, key: &str) -> Result<ReaderCell> {
        if let Some(cell) = self.lock_reader_cache()?.reader_cell(key) {
            return Ok(cell);
        }

        let permit = Arc::clone(&self.overflow_slots)
            .acquire_owned()
            .await
            .map_err(|error| ocr_error(format!("Failed to acquire Sceptre reader admission: {error}")))?;
        let mut readers = self.lock_reader_cache()?;
        if let Some(cell) = readers.reader_cell(key) {
            return Ok(cell);
        }
        Ok(readers.insert_overflow(key, permit))
    }

    fn lock_reader_cache(&self) -> Result<std::sync::MutexGuard<'_, ReaderCache>> {
        self.readers.lock().map_err(|error| XbergError::Plugin {
            message: format!("Failed to acquire Sceptre reader cache: {error}"),
            plugin_name: BACKEND_NAME.to_string(),
        })
    }

    #[cfg(auto_rotate)]
    fn orientation_detector_cell(
        &self,
        cache_dir: &Path,
        acceleration: Option<crate::core::config::AccelerationConfig>,
    ) -> Result<OrientationDetectorCell> {
        self.orientation_detectors
            .lock()
            .map_err(|error| XbergError::Plugin {
                message: format!("Failed to acquire Sceptre orientation detector cache: {error}"),
                plugin_name: BACKEND_NAME.to_string(),
            })?
            .detector(cache_dir, acceleration)
    }

    #[cfg(auto_rotate)]
    fn orientation_detector_for_config(
        &self,
        config: &sceptre::OcrConfig,
        acceleration: Option<crate::core::config::AccelerationConfig>,
    ) -> Result<OrientationDetectorCell> {
        let cache_dir = config
            .model
            .cache_dir
            .clone()
            .unwrap_or_else(crate::doc_orientation::resolve_cache_dir);
        self.orientation_detector_cell(&cache_dir, acceleration)
    }

    #[cfg(auto_rotate)]
    fn rotation_context(&self, config: &OcrConfig, sceptre_config: &sceptre::OcrConfig) -> Result<RotationContext> {
        Ok(RotationContext {
            detector: config
                .auto_rotate
                .then(|| self.orientation_detector_for_config(sceptre_config, config.acceleration.clone()))
                .transpose()?,
            enabled: config.auto_rotate,
        })
    }

    fn effective_config(config: &OcrConfig) -> Result<(sceptre::OcrConfig, Vec<String>)> {
        validate_auto_rotate_support(config)?;
        validate_acceleration(config)?;
        let languages = effective_languages(config);
        let group = resolve_language_group(&languages)?;
        let mut sceptre_config = parse_sceptre_options(config)?;
        sceptre_config.model.cache_dir =
            crate::cache_dir::ocr_model_cache_override(sceptre_config.model.cache_dir.as_deref())?;
        if !has_explicit_model_backend(config) {
            #[cfg(all(feature = "sceptre-ocr-tract", not(feature = "sceptre-ocr-ort")))]
            {
                sceptre_config.model.backend = Backend::Tract;
            }
        }
        validate_inference_backend(sceptre_config.model.backend)?;
        let thread_budget = crate::core::config::concurrency::active_thread_budget();
        sceptre_config.concurrency.max_threads = Some(
            sceptre_config
                .concurrency
                .max_threads
                .unwrap_or(thread_budget)
                .clamp(1, thread_budget),
        );
        sceptre_config.model.languages = vec![group];
        Ok((sceptre_config, languages))
    }

    async fn process_owned(&self, image_bytes: Arc<Vec<u8>>, config: &OcrConfig) -> Result<ExtractedDocument> {
        if image_bytes.is_empty() {
            return Err(XbergError::validation("Cannot run Sceptre OCR on empty image data"));
        }

        let (sceptre_config, languages) = Self::effective_config(config)?;
        let cache_key = reader_cache_key(&sceptre_config)?;
        let reader_cell = self.reader_cell(&cache_key).await?;
        let execution_permit = Arc::clone(&self.execution_slots)
            .acquire_owned()
            .await
            .map_err(|error| ocr_error(format!("Failed to acquire Sceptre execution admission: {error}")))?;
        #[cfg(auto_rotate)]
        let rotation = self.rotation_context(config, &sceptre_config)?;
        #[cfg(not(auto_rotate))]
        let rotation = RotationContext {};

        let output = tokio::task::spawn_blocking(move || {
            let _execution_permit = execution_permit;
            run_blocking(image_bytes.as_slice(), sceptre_config, reader_cell, rotation)
        })
        .await
        .map_err(|error| XbergError::Plugin {
            message: format!("Sceptre OCR blocking task failed: {error}"),
            plugin_name: BACKEND_NAME.to_string(),
        })??;

        Ok(build_document(output, languages, config))
    }
}

impl Default for SceptreOcrBackend {
    fn default() -> Self {
        Self::new().expect("constructing an empty Sceptre backend is infallible")
    }
}

impl Plugin for SceptreOcrBackend {
    fn name(&self) -> &str {
        BACKEND_NAME
    }

    fn version(&self) -> String {
        sceptre::VERSION.to_string()
    }
}

#[async_trait]
impl OcrBackend for SceptreOcrBackend {
    async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> Result<ExtractedDocument> {
        self.process_owned(Arc::new(image_bytes.to_vec()), config).await
    }

    async fn process_image_owned(&self, image_bytes: Arc<Vec<u8>>, config: &OcrConfig) -> Result<ExtractedDocument> {
        self.process_owned(image_bytes, config).await
    }

    async fn process_image_file(&self, path: &Path, config: &OcrConfig) -> Result<ExtractedDocument> {
        let bytes = crate::core::io::read_file_async(path).await?;
        self.process_owned(Arc::new(bytes), config).await
    }

    fn supports_language(&self, language: &str) -> bool {
        language_group(language).is_some()
    }

    fn backend_type(&self) -> OcrBackendType {
        // Built-in OCR stages are selected by registry name; keep the exhaustive
        // v1 enum stable while adding Sceptre in a minor release. ~keep
        OcrBackendType::Custom
    }

    fn supported_languages(&self) -> Vec<String> {
        supported_language_aliases().into_iter().map(str::to_string).collect()
    }

    /// Sceptre's page confidence is EasyOCR's length-penalised `custom_mean` rescaled to
    /// 0-100. It is not comparable to Tesseract's scale: on one document a real text page
    /// scored 39 while a pure line-art drawing scored 74, an inverted ordering relative to
    /// legibility. Never gate on this value.
    fn confidence_semantics(&self) -> crate::plugins::ConfidenceSemantics {
        crate::plugins::ConfidenceSemantics::Uncalibrated
    }

    /// Measured on a `/Rotate 270` scanned ordinance: sceptre produced character garbage on the
    /// sideways raster and only read correctly once the same page was rendered upright first.
    /// This is the same value as the trait default, made explicit here because it is a
    /// measurement, not an unverified inheritance.
    fn page_orientation_handling(&self) -> crate::plugins::PageOrientationHandling {
        crate::plugins::PageOrientationHandling::RequiresUpright
    }
}

struct BlockingOutput {
    result: OcrResult,
    width: u32,
    height: u32,
    #[cfg(auto_rotate)]
    orientation: Option<crate::doc_orientation::OrientationResult>,
    #[cfg(auto_rotate)]
    auto_rotated: bool,
}

struct RotationContext {
    #[cfg(auto_rotate)]
    detector: Option<OrientationDetectorCell>,
    #[cfg(auto_rotate)]
    enabled: bool,
}

fn run_blocking(
    image_bytes: &[u8],
    config: sceptre::OcrConfig,
    reader_cell: ReaderCell,
    rotation: RotationContext,
) -> Result<BlockingOutput> {
    #[cfg(not(auto_rotate))]
    let _ = rotation;
    let decoded = decode_sceptre_image(image_bytes)?;
    #[cfg(auto_rotate)]
    let (image, orientation, was_rotated) = decode_and_rotate(decoded, rotation.enabled, rotation.detector.as_deref())?;
    #[cfg(not(auto_rotate))]
    let image = Image::from_rgb8(decoded.width(), decoded.height(), decoded.into_raw()).map_err(map_sceptre_error)?;
    let width = image.width();
    let height = image.height();
    let reader = reader_cell
        .reader
        .get_or_try_init(|| Reader::builder().config(config).build().map_err(map_sceptre_error))?;
    let result = reader
        .recognize(&image, &ReadOptions::default())
        .map_err(map_sceptre_error)?;
    Ok(BlockingOutput {
        result,
        width,
        height,
        #[cfg(auto_rotate)]
        orientation,
        #[cfg(auto_rotate)]
        auto_rotated: was_rotated,
    })
}

fn decode_sceptre_image(image_bytes: &[u8]) -> Result<image::RgbImage> {
    crate::extraction::image_decode::decode_standard_rgb8_with_default_security_limits(image_bytes).map_err(|error| {
        XbergError::Ocr {
            message: "Sceptre OCR operation failed".to_string(),
            source: Some(Box::new(error)),
        }
    })
}

#[cfg(auto_rotate)]
fn decode_and_rotate(
    decoded: image::RgbImage,
    auto_rotate: bool,
    detector: Option<&crate::doc_orientation::DocOrientationDetector>,
) -> Result<(Image, Option<crate::doc_orientation::OrientationResult>, bool)> {
    if !auto_rotate {
        return Image::from_rgb8(decoded.width(), decoded.height(), decoded.into_raw())
            .map(|image| (image, None, false))
            .map_err(map_sceptre_error);
    }
    let detector = detector.ok_or_else(|| ocr_error("Sceptre orientation detector was not initialized"))?;

    let orientation = match detector.detect(&decoded) {
        Ok(orientation) => orientation,
        Err(error) => {
            tracing::warn!(error = %error, "Sceptre orientation detection failed; using the original image");
            return Image::from_rgb8(decoded.width(), decoded.height(), decoded.into_raw())
                .map(|image| (image, None, false))
                .map_err(map_sceptre_error);
        }
    };
    let should_rotate = orientation.degrees != 0 && orientation.confidence >= crate::doc_orientation::MIN_CONFIDENCE;
    let rotated = if should_rotate {
        rotate_to_upright(&decoded, orientation.degrees)
    } else {
        decoded
    };
    Image::from_rgb8(rotated.width(), rotated.height(), rotated.into_raw())
        .map(|image| (image, Some(orientation), should_rotate))
        .map_err(map_sceptre_error)
}

#[cfg(auto_rotate)]
fn rotate_to_upright(image: &image::RgbImage, degrees: u32) -> image::RgbImage {
    match degrees {
        90 => image::imageops::rotate270(image),
        180 => image::imageops::rotate180(image),
        270 => image::imageops::rotate90(image),
        _ => image.clone(),
    }
}

fn build_document(output: BlockingOutput, languages: Vec<String>, config: &OcrConfig) -> ExtractedDocument {
    let line_elements: Vec<OcrElement> = output.result.lines.iter().map(line_to_element).collect();
    let content = output
        .result
        .lines
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let internal_document = build_internal_document(&line_elements);
    let ocr_elements = select_output_elements(&line_elements, config);
    let metadata = build_metadata(&languages, &output);

    ExtractedDocument {
        content,
        mime_type: Cow::Borrowed("text/plain"),
        metadata,
        detected_languages: Some(languages),
        ocr_elements,
        ocr_internal_document: Some(internal_document),
        ..Default::default()
    }
}

fn line_to_element(line: &TextLine) -> OcrElement {
    OcrElement {
        text: line.text.clone(),
        geometry: OcrBoundingGeometry::Quadrilateral {
            points: line
                .quad
                .points
                .into_iter()
                .map(|point| (pixel_coordinate(point.x), pixel_coordinate(point.y)).into())
                .collect(),
        },
        confidence: OcrConfidence {
            detection: None,
            recognition: f64::from(line.confidence).clamp(0.0, 1.0),
        },
        level: OcrElementLevel::Line,
        rotation: None,
        page_number: 1,
        parent_id: None,
        backend_metadata: HashMap::new(),
    }
}

fn pixel_coordinate(value: f32) -> u32 {
    if !value.is_finite() || value <= 0.0 {
        return 0;
    }
    value.round().min(u32::MAX as f32) as u32
}

/// Attribute key `pdf::structure::adapters::hocr_block_id` reads to merge
/// consecutive OCR lines into one `PdfParagraph`. Duplicated as a literal
/// (rather than imported) because it is also defined locally in
/// `ocr::hocr_parser` behind `feature = "ocr"`, which a sceptre-only build
/// does not enable.
const HOCR_BLOCK_ID_ATTRIBUTE: &str = "hocr_block_id";

/// Maximum vertical gap between two consecutive sceptre lines, expressed as a
/// multiple of the running median line height seen so far, for the lines to
/// be grouped into the same paragraph block. Mirrors
/// `MAX_BLOCK_VERTICAL_GAP_IN_LINE_HEIGHTS` in `ocr::conversion`.
const BLOCK_MAX_VERTICAL_GAP_IN_LINE_HEIGHTS: f64 = 0.6;
/// Minimum fraction of the narrower of two consecutive lines' horizontal
/// extent that must overlap for them to be considered the same paragraph
/// column. Mirrors `MIN_BLOCK_HORIZONTAL_OVERLAP_RATIO` in `ocr::conversion`.
const BLOCK_MIN_HORIZONTAL_OVERLAP_RATIO: f64 = 0.3;
/// Maximum left-edge offset between two consecutive lines, expressed as a
/// multiple of the running median line height, allowed as a fallback when the
/// lines don't x-overlap enough on their own. Mirrors
/// `MAX_BLOCK_LEFT_EDGE_OFFSET_IN_LINE_HEIGHTS` in `ocr::conversion`.
const BLOCK_MAX_LEFT_EDGE_OFFSET_IN_LINE_HEIGHTS: f64 = 0.5;
/// Maximum y-center delta between two consecutive elements, expressed as a
/// multiple of the running median line height, for them to be treated as
/// fragments of the same physical text line rather than two different lines.
///
/// Sceptre's `width_ths` detection parameter under-merges letter-spaced or
/// widely-kerned runs on some documents, splitting one physical line into
/// several `TextLine` entries (see `sceptre::config::DetectionConfig::width_ths`,
/// measured on this same 16-page ordinance: 339 detected lines, 85 of them
/// single words, at the current default). Verified against Sceptre's own
/// output for `test_documents/pdf_scanned/ordinance_2197_scanned.pdf`: a line
/// split into 9 word fragments (e.g. "In" / "accordance" / "with" / "Section"
/// / "2-133," / "the" / "PD" / "must" / "be") has consecutive y-center deltas
/// of ~1-3px against line heights of ~32-45px (ratio < 0.1), while genuinely
/// distinct lines on the same page have deltas of at least ~38px (ratio ≥ 0.5
/// even where their AABBs happen to vertically overlap). Fragments have a
/// near-zero vertical gap already, so the vertical-gap check below would let
/// them through regardless, but their narrow, non-overlapping x-extents fail
/// the horizontal overlap/left-edge check meant for paragraph continuation --
/// which produced one spurious block per word. This constant lets same-line
/// fragments skip that horizontal test entirely, mirroring the `ycenter_ths`
/// test `sceptre::detect::group::combine_into_lines` already uses internally
/// to decide the same "is this the same physical line" question.
const LINE_YCENTER_THS: f64 = 0.5;

/// Minimum first-line indent, as a multiple of the running median line height,
/// for a line to start a new paragraph block even though the vertical-gap and
/// horizontal-overlap checks would otherwise continue the current one. Mirrors
/// `MIN_PARAGRAPH_INDENT_IN_LINE_HEIGHTS` in `ocr::conversion`, which was added
/// for PaddleOCR after a 16-page scanned document produced exactly 16
/// paragraphs -- one full-page block per page -- because its paragraph breaks
/// are marked by indentation alone, with uniform leading throughout.
///
/// Unlike the PaddleOCR copy this check must NOT fire for the word fragments of
/// a single physical line (see [`LINE_YCENTER_THS`]): those march rightward
/// across the line and so look arbitrarily "indented" against the block's left
/// margin. [`assign_sceptre_block_ids`] therefore consults it only for lines
/// that are not same-line fragments.
const PARAGRAPH_MIN_INDENT_IN_LINE_HEIGHTS: f64 = 0.75;

/// Group sceptre's per-line elements into paragraph blocks using their own
/// AABB geometry, returning a stable block id per input element in the same
/// order, so `pdf::structure::adapters::ocr_doc_to_paragraphs` can merge
/// consecutive sceptre lines into one `PdfParagraph` instead of treating every
/// line as its own paragraph.
///
/// This is a deliberate, intentionally-flagged duplicate of
/// `crate::ocr::conversion::assign_line_block_ids`, which already gives
/// PaddleOCR the same paragraph grouping it otherwise lacks (#631). That
/// function is `#[cfg(paddle_ocr)]` inside a module gated on `feature =
/// "ocr"`; sceptre-ocr builds (`sceptre-ocr-ort` / `sceptre-ocr-tract`) enable
/// neither, so it cannot be called from here without pulling `feature = "ocr"`
/// into every sceptre-ocr build. Reconcile by extracting a shared, ungated
/// helper once both call sites are stable, rather than letting these two
/// copies drift apart silently.
fn assign_sceptre_block_ids(elements: &[OcrElement]) -> Vec<String> {
    let mut block_ids = Vec::with_capacity(elements.len());
    let mut heights_seen: Vec<f64> = Vec::with_capacity(elements.len());
    let mut block_index: u32 = 0;
    let mut previous: Option<BoundingBox> = None;
    // Leftmost edge established so far by the current block's lines, used to
    // detect an indented paragraph start (see `starts_indented_paragraph`).
    let mut block_left_margin: f64 = 0.0;

    for element in elements {
        let bounds = geometry_bounds(&element.geometry).unwrap_or_default();
        insert_sorted(&mut heights_seen, bounds.y1 - bounds.y0);
        let median_line_height = running_median(&heights_seen);

        let continues_block = previous.as_ref().is_some_and(|previous| {
            lines_share_block(previous, &bounds, median_line_height)
                && (shares_physical_line(previous, &bounds, median_line_height)
                    || !starts_indented_paragraph(bounds.x0, block_left_margin, median_line_height))
        });
        if continues_block {
            block_left_margin = block_left_margin.min(bounds.x0);
        } else {
            block_index += 1;
            block_left_margin = bounds.x0;
        }
        block_ids.push(format!("sceptre-block-{block_index}"));
        previous = Some(bounds);
    }

    block_ids
}

/// Whether `current` is a fragment of the same physical text line as
/// `previous`, judged by their y-centers (see [`LINE_YCENTER_THS`]).
fn shares_physical_line(previous: &BoundingBox, current: &BoundingBox, median_line_height: f64) -> bool {
    if median_line_height <= 0.0 {
        return false;
    }
    let previous_ycenter = (previous.y0 + previous.y1) / 2.0;
    let current_ycenter = (current.y0 + current.y1) / 2.0;
    (current_ycenter - previous_ycenter).abs() < LINE_YCENTER_THS * median_line_height
}

/// Whether `x0` sits far enough right of `block_left_margin` -- the leftmost
/// edge established by the current block's lines so far -- to read as a new
/// paragraph's indented first line, in units of `median_line_height`.
///
/// This is independent of (and can override) the vertical-gap/overlap checks in
/// [`lines_share_block`], because real body text often marks a paragraph break
/// with indentation alone and no extra vertical space (see
/// [`PARAGRAPH_MIN_INDENT_IN_LINE_HEIGHTS`]).
fn starts_indented_paragraph(x0: f64, block_left_margin: f64, median_line_height: f64) -> bool {
    if median_line_height <= 0.0 {
        return false;
    }
    x0 - block_left_margin > median_line_height * PARAGRAPH_MIN_INDENT_IN_LINE_HEIGHTS
}

fn lines_share_block(previous: &BoundingBox, current: &BoundingBox, median_line_height: f64) -> bool {
    if median_line_height <= 0.0 {
        return false;
    }

    if shares_physical_line(previous, current, median_line_height) {
        return true;
    }

    let vertical_gap = if current.y0 >= previous.y1 {
        current.y0 - previous.y1
    } else if previous.y0 >= current.y1 {
        previous.y0 - current.y1
    } else {
        0.0
    };
    if vertical_gap > median_line_height * BLOCK_MAX_VERTICAL_GAP_IN_LINE_HEIGHTS {
        return false;
    }

    let overlap = (previous.x1.min(current.x1) - previous.x0.max(current.x0)).max(0.0);
    let narrower_width = (previous.x1 - previous.x0).min(current.x1 - current.x0);
    let overlap_ratio = if narrower_width > 0.0 {
        overlap / narrower_width
    } else {
        0.0
    };
    let left_edge_offset = (previous.x0 - current.x0).abs();

    overlap_ratio >= BLOCK_MIN_HORIZONTAL_OVERLAP_RATIO
        || left_edge_offset <= median_line_height * BLOCK_MAX_LEFT_EDGE_OFFSET_IN_LINE_HEIGHTS
}

fn insert_sorted(sorted: &mut Vec<f64>, value: f64) {
    let index = sorted.partition_point(|existing| *existing < value);
    sorted.insert(index, value);
}

fn running_median(sorted: &[f64]) -> f64 {
    let len = sorted.len();
    if len == 0 {
        return 0.0;
    }
    if len % 2 == 1 {
        sorted[len / 2]
    } else {
        (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
    }
}

fn build_internal_document(elements: &[OcrElement]) -> InternalDocument {
    let mut document = InternalDocument::new("ocr");
    document.mime_type = "text/plain".to_string();
    document.prebuilt_ocr_elements = Some(elements.to_vec());
    let block_ids = assign_sceptre_block_ids(elements);
    for (element, block_id) in elements.iter().zip(block_ids) {
        let mut internal = InternalElement::text(
            ElementKind::OcrText {
                level: OcrElementLevel::Line,
            },
            &element.text,
            0,
        );
        internal.page = Some(element.page_number);
        internal.bbox = geometry_bounds(&element.geometry);
        internal.ocr_geometry = Some(element.geometry.clone());
        internal.ocr_confidence = Some(element.confidence.clone());
        internal.attributes = Some([(HOCR_BLOCK_ID_ATTRIBUTE.to_string(), block_id)].into_iter().collect());
        document.push_element(internal);
    }
    document
}

fn geometry_bounds(geometry: &OcrBoundingGeometry) -> Option<BoundingBox> {
    let OcrBoundingGeometry::Quadrilateral { points } = geometry else {
        return None;
    };
    let x0 = points.iter().map(|point| point.x).min()?;
    let y0 = points.iter().map(|point| point.y).min()?;
    let x1 = points.iter().map(|point| point.x).max()?;
    let y1 = points.iter().map(|point| point.y).max()?;
    Some(BoundingBox {
        x0: f64::from(x0),
        y0: f64::from(y0),
        x1: f64::from(x1),
        y1: f64::from(y1),
    })
}

fn select_output_elements(elements: &[OcrElement], config: &OcrConfig) -> Option<Vec<OcrElement>> {
    let options = config.element_config.as_ref()?;
    let selected = options.select_elements(elements);
    (!selected.is_empty()).then_some(selected)
}

// Matches the Tesseract backend's low-word-confidence cutoff
// (ocr/processor/execution.rs) so downstream quality gating applies the same
// threshold regardless of which backend produced the metadata. ~keep
const LOW_CONFIDENCE_THRESHOLD_PCT: i64 = 50;

/// Per-line confidence statistics on Tesseract's 0-100 integer percentage
/// scale, so consumers that already gate on Tesseract's `mean_text_conf` /
/// `median_word_conf` / `p10_word_conf` keys (e.g. `extractors/pdf/ocr.rs`,
/// which divides `mean_text_conf` by 100.0) work unchanged against Sceptre.
struct ConfidenceStats {
    mean_pct: i64,
    median_pct: i64,
    p10_pct: i64,
    line_count: usize,
    low_confidence_count: usize,
}

/// Convert a Sceptre line confidence (`0.0..=1.0`) to Tesseract's `0..=100`
/// integer percentage scale.
fn confidence_to_percent(confidence: f32) -> i64 {
    (f64::from(confidence).clamp(0.0, 1.0) * 100.0).round() as i64
}

/// Compute mean/median/p10 confidence and low-confidence-line count from
/// per-line confidence values. Returns `None` when there are no lines, mirroring
/// the Tesseract backend leaving these keys out of `metadata.additional` when it
/// has nothing to report.
fn compute_confidence_stats(lines: &[TextLine]) -> Option<ConfidenceStats> {
    if lines.is_empty() {
        return None;
    }

    let percentages: Vec<i64> = lines
        .iter()
        .map(|line| confidence_to_percent(line.confidence))
        .collect();
    let line_count = percentages.len();
    let low_confidence_count = percentages
        .iter()
        .filter(|&&pct| pct < LOW_CONFIDENCE_THRESHOLD_PCT)
        .count();

    let mean_pct = percentages.iter().sum::<i64>() / line_count as i64;

    let mut sorted = percentages.clone();
    sorted.sort_unstable();
    let median_pct = if line_count.is_multiple_of(2) {
        (sorted[line_count / 2 - 1] + sorted[line_count / 2]) / 2
    } else {
        sorted[line_count / 2]
    };
    let p10_idx = ((line_count as f64 - 1.0) * 0.1).floor() as usize;
    let p10_pct = sorted[p10_idx.min(line_count - 1)];

    Some(ConfidenceStats {
        mean_pct,
        median_pct,
        p10_pct,
        line_count,
        low_confidence_count,
    })
}

/// Publish confidence statistics into `metadata.additional` using the same key
/// names the Tesseract backend uses, so quality-gating code that reads
/// `mean_text_conf` / `median_word_conf` / `p10_word_conf` / `word_count` /
/// `low_conf_word_count` works against Sceptre output too (issue #186). Sceptre
/// has no word-level detections, so line confidence is the finest granularity
/// available and is reported under the same keys.
fn insert_confidence_stats(metadata: &mut Metadata, lines: &[TextLine]) {
    let Some(stats) = compute_confidence_stats(lines) else {
        return;
    };
    metadata
        .additional
        .insert(Cow::Borrowed("mean_text_conf"), serde_json::json!(stats.mean_pct));
    metadata
        .additional
        .insert(Cow::Borrowed("median_word_conf"), serde_json::json!(stats.median_pct));
    metadata
        .additional
        .insert(Cow::Borrowed("p10_word_conf"), serde_json::json!(stats.p10_pct));
    metadata
        .additional
        .insert(Cow::Borrowed("word_count"), serde_json::json!(stats.line_count));
    metadata.additional.insert(
        Cow::Borrowed("low_conf_word_count"),
        serde_json::json!(stats.low_confidence_count),
    );
}

fn build_metadata(languages: &[String], output: &BlockingOutput) -> Metadata {
    let mut metadata = Metadata {
        format: Some(FormatMetadata::Ocr(OcrMetadata {
            language: languages.join("+"),
            // Sceptre has no Tesseract-style Page Segmentation Mode; `psm: 3`
            // (Tesseract's "fully automatic" default) would misrepresent a
            // concept this backend does not have. Leave it at the zero default. ~keep
            output_format: "text".to_string(),
            ..Default::default()
        })),
        ocr_used: true,
        ..Default::default()
    };
    metadata
        .additional
        .insert(Cow::Borrowed(PROCESSED_WIDTH_KEY), serde_json::json!(output.width));
    metadata
        .additional
        .insert(Cow::Borrowed(PROCESSED_HEIGHT_KEY), serde_json::json!(output.height));
    insert_confidence_stats(&mut metadata, &output.result.lines);
    #[cfg(auto_rotate)]
    if let Some(orientation) = output.orientation {
        metadata.additional.insert(
            Cow::Borrowed(ORIENTATION_DEGREES_KEY),
            serde_json::json!(orientation.degrees),
        );
        metadata.additional.insert(
            Cow::Borrowed(ORIENTATION_CONFIDENCE_KEY),
            serde_json::json!(orientation.confidence),
        );
    }
    #[cfg(auto_rotate)]
    if output.auto_rotated {
        metadata
            .additional
            .insert(Cow::Borrowed(AUTO_ROTATED_KEY), serde_json::Value::Bool(true));
    }
    metadata
}

/// `object.get("detection")` is how `sceptre::DetectionConfig::detect_orientation` reaches
/// this backend: any caller can already flip it on today with
/// `backend_options: {"detection": {"detect_orientation": true}}`. Nothing here defaults it
/// to `true`, and that is a deliberate decision (#662), not an oversight to "finish wiring".
///
/// `detect_orientation` is sceptre's own opt-in whole-page rotation guesser: it probes CRAFT
/// at 0/90/180/270 degrees on a reduced canvas, rotates internally for the winning angle, and
/// unrotates the output quads back onto the frame of the `Image` this backend passed in
/// (`sceptre::engine::sceptre_engine`, `detect::orientation::unrotate_corners`) before
/// `TextLine`s are ever returned here. So enabling it introduces no coordinate-frame bug for
/// this crate's `line_to_element` / `build_internal_document` / table geometry — that risk was
/// checked and ruled out, not assumed away.
///
/// The reason to leave it off is that xberg already has two better-targeted fixes for exactly
/// the defect it targets, and stacking a third, weaker one on top adds cost without adding
/// coverage:
///   - For PDF-sourced pages, `extractors::pdf::ocr::upright_raster_for_backend` (#643) rotates
///     the raster to upright using the page's own `/Rotate` value — a known fact, not a guess —
///     whenever this backend's `page_orientation_handling()` reports `RequiresUpright`, and
///     `undo_upright_raster_correction` maps the returned geometry back deterministically. Zero
///     false-positive risk, because there is nothing to infer.
///   - For raw images with no `/Rotate` to consult, `crate::doc_orientation` (a PP-LCNet
///     classifier) already runs ahead of this backend, cross-backend, behind `config.auto_rotate`
///     (`decode_and_rotate` below), with its own coordinate-frame correction in
///     `extractors::pdf::ocr::undo_auto_rotate_document_bboxes`.
///
/// That leaves only images with unknown rotation and `auto_rotate` left off as a case
/// `detect_orientation` could add coverage for — and sceptre's own ADR 0038 keeps the flag
/// `false` by default even after fixing its false-positive rate to 0/23 on its labeled corpus,
/// because it costs four extra CRAFT forward passes on every page (rotated or not) and the
/// validation set is 23 images. Defaulting it on in xberg would pay that unconditional cost
/// for every Sceptre page to cover a narrower gap than it looks like, and inherit a heuristic
/// xberg has not independently validated. Revisit if `crate::doc_orientation` is ever removed,
/// or if a corpus-backed measurement on xberg's own fixtures justifies the cost.
fn parse_sceptre_options(config: &OcrConfig) -> Result<sceptre::OcrConfig> {
    let Some(options) = config.backend_options.as_ref() else {
        return Ok(sceptre::OcrConfig::default());
    };
    let object = options
        .as_object()
        .ok_or_else(|| ocr_error("Invalid `backend_options` configuration: expected an object"))?;
    let mut parsed = sceptre::OcrConfig::default();
    if let Some(value) = object.get("detection") {
        parsed.detection = parse_sceptre_section("detection", value)?;
    }
    if let Some(value) = object.get("recognition") {
        parsed.recognition = parse_sceptre_section("recognition", value)?;
    }
    if let Some(value) = object.get("concurrency") {
        parsed.concurrency = parse_sceptre_section("concurrency", value)?;
    }
    if let Some(value) = object.get("model") {
        parsed.model = parse_sceptre_section("model", value)?;
    }
    Ok(parsed)
}

fn has_explicit_model_backend(config: &OcrConfig) -> bool {
    config
        .backend_options
        .as_ref()
        .and_then(serde_json::Value::as_object)
        .and_then(|options| options.get("model"))
        .and_then(serde_json::Value::as_object)
        .is_some_and(|model| model.contains_key("backend"))
}

fn validate_inference_backend(backend: Backend) -> Result<()> {
    match backend {
        Backend::Ort if cfg!(feature = "sceptre-ocr-ort") => Ok(()),
        Backend::Tract if cfg!(feature = "sceptre-ocr-tract") => Ok(()),
        Backend::Candle if cfg!(feature = "sceptre-ocr-candle") => Ok(()),
        Backend::Ort => Err(ocr_error(
            "Sceptre ORT inference is unavailable in this build; enable `sceptre-ocr-ort`",
        )),
        Backend::Tract => Err(ocr_error(
            "Sceptre tract inference is unavailable in this build; enable `sceptre-ocr-tract`",
        )),
        Backend::Candle => Err(ocr_error(
            "Sceptre candle inference is unavailable in this build; enable `sceptre-ocr-candle`",
        )),
    }
}

fn parse_sceptre_section<T: serde::de::DeserializeOwned>(name: &str, value: &serde_json::Value) -> Result<T> {
    serde_json::from_value(value.clone()).map_err(|error| {
        ocr_error(format!(
            "Invalid Sceptre `backend_options.{name}` configuration: {error}"
        ))
    })
}

fn reader_cache_key(config: &sceptre::OcrConfig) -> Result<String> {
    serde_json::to_string(config)
        .map_err(|error| ocr_error(format!("Failed to serialize effective Sceptre configuration: {error}")))
}

#[cfg(auto_rotate)]
fn orientation_detector_cache_key(
    cache_dir: &Path,
    acceleration: Option<&crate::core::config::AccelerationConfig>,
) -> Result<String> {
    serde_json::to_string(&(cache_dir, acceleration)).map_err(|error| {
        ocr_error(format!(
            "Failed to serialize Sceptre orientation configuration: {error}"
        ))
    })
}

fn effective_languages(config: &OcrConfig) -> Vec<String> {
    let languages: Vec<String> = config
        .language
        .iter()
        .map(|language| language.trim().to_ascii_lowercase())
        .filter(|language| !language.is_empty())
        .collect();
    if languages.is_empty() {
        vec![DEFAULT_LANGUAGE.to_string()]
    } else {
        languages
    }
}

fn resolve_language_group(languages: &[String]) -> Result<Language> {
    let mut selected = Language::English;
    for language in languages {
        let group = language_group(language).ok_or_else(|| {
            ocr_error(format!(
                concat!(
                    "Sceptre does not support OCR language `{}`; choose an English, Latin, ",
                    "simplified Chinese, Japanese, Korean, Telugu, Kannada, or Cyrillic language"
                ),
                language
            ))
        })?;
        if group == Language::English {
            continue;
        }
        if selected != Language::English && selected != group {
            return Err(ocr_error(format!(
                concat!(
                    "Sceptre cannot combine language groups `{}` and `{}` in one reader; ",
                    "use languages from one script family (English may be combined with any one family)"
                ),
                language_group_name(selected),
                language_group_name(group),
            )));
        }
        selected = group;
    }
    Ok(selected)
}

fn validate_acceleration(config: &OcrConfig) -> Result<()> {
    let Some(acceleration) = config.acceleration.as_ref() else {
        return Ok(());
    };
    if acceleration.provider == ExecutionProviderType::Cpu {
        return Ok(());
    }
    Err(ocr_error(format!(
        concat!(
            "Sceptre in Xberg 1.1 is CPU-only; acceleration provider `{:?}` is not supported. ",
            "Set acceleration.provider to `cpu` or omit acceleration"
        ),
        acceleration.provider
    )))
}

fn validate_auto_rotate_support(_config: &OcrConfig) -> Result<()> {
    #[cfg(not(auto_rotate))]
    if _config.auto_rotate {
        return Err(ocr_error(
            "Sceptre auto-rotation is unavailable in this build; enable the `auto-rotate` or `auto-rotate-tract` feature",
        ));
    }
    Ok(())
}

fn map_sceptre_error(error: sceptre::OcrError) -> XbergError {
    XbergError::Ocr {
        message: "Sceptre OCR operation failed".to_string(),
        source: Some(Box::new(error)),
    }
}

fn ocr_error(message: impl Into<String>) -> XbergError {
    XbergError::Ocr {
        message: message.into(),
        source: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sceptre::{Point, Quad};

    fn line(text: &str, confidence: f32) -> TextLine {
        TextLine {
            quad: Quad {
                points: [
                    Point::new(10.4, 20.2),
                    Point::new(110.6, 20.0),
                    Point::new(111.0, 42.8),
                    Point::new(10.0, 43.1),
                ],
            },
            text: text.to_string(),
            confidence,
        }
    }

    #[test]
    fn should_map_iso_codes_to_sceptre_groups() {
        assert_eq!(language_group("eng"), Some(Language::English));
        assert_eq!(language_group("de"), Some(Language::Latin));
        assert_eq!(language_group("zh_Hans"), Some(Language::ChineseSimplified));
        assert_eq!(language_group("jpn"), Some(Language::Japanese));
        assert_eq!(language_group("jpn_vert"), Some(Language::Japanese));
        assert_eq!(language_group("kor"), Some(Language::Korean));
        assert_eq!(language_group("tel"), Some(Language::Telugu));
        assert_eq!(language_group("kan"), Some(Language::Kannada));
        assert_eq!(language_group("ukr"), Some(Language::Cyrillic));
        assert_eq!(language_group("ara"), None);
        assert_eq!(language_group("cat"), None);
        assert_eq!(language_group("kaz"), None);
    }

    #[test]
    fn shared_validation_should_accept_every_sceptre_language_alias() {
        for language in supported_language_aliases() {
            assert!(
                crate::core::config_validation::validate_language_code(language).is_ok(),
                "Sceptre language alias {language} should pass shared validation"
            );
        }
    }

    #[test]
    fn should_allow_english_with_one_non_english_group() {
        let languages = vec!["eng".to_string(), "deu".to_string(), "fra".to_string()];
        assert_eq!(
            resolve_language_group(&languages).expect("compatible languages"),
            Language::Latin
        );
    }

    #[test]
    fn should_normalize_easyocr_script_tokens() {
        assert_eq!(language_group("CH_SIM"), Some(Language::ChineseSimplified));
        assert_eq!(language_group("rs_latin"), Some(Language::Latin));
        assert_eq!(language_group("rs-cyrillic"), Some(Language::Cyrillic));
    }

    #[test]
    fn should_reject_incompatible_language_groups() {
        let languages = vec!["jpn".to_string(), "kor".to_string()];
        let error = resolve_language_group(&languages).expect_err("groups must be incompatible");
        assert!(error.to_string().contains("cannot combine language groups"));
    }

    #[test]
    fn should_parse_direct_sceptre_options_and_override_languages() {
        let config = OcrConfig {
            language: vec!["rus".to_string()],
            backend_options: Some(serde_json::json!({
                "recognition": { "batch_size": 4 },
                "concurrency": { "max_threads": 2 }
            })),
            ..Default::default()
        };
        let (effective, _) = SceptreOcrBackend::effective_config(&config).expect("valid config");
        assert_eq!(effective.recognition.batch_size, 4);
        assert_eq!(effective.concurrency.max_threads, Some(2));
        assert_eq!(effective.model.languages, vec![Language::Cyrillic]);
    }

    #[test]
    fn sceptre_rejects_an_empty_explicit_model_cache_root() {
        let config = OcrConfig {
            backend: "sceptre".to_string(),
            backend_options: Some(serde_json::json!({"model": {"cache_dir": ""}})),
            ..OcrConfig::default()
        };

        let error = SceptreOcrBackend::effective_config(&config)
            .expect_err("an empty explicit model cache root must be rejected");

        assert_eq!(
            error.to_string(),
            "Validation error: OCR model cache_dir must not be empty"
        );
    }

    #[test]
    fn sceptre_preserves_an_explicit_model_cache_root() {
        let config = OcrConfig {
            backend: "sceptre".to_string(),
            backend_options: Some(serde_json::json!({"model": {"cache_dir": "/request/cache"}})),
            ..OcrConfig::default()
        };

        let (effective, _) = SceptreOcrBackend::effective_config(&config).expect("explicit cache root should resolve");

        assert_eq!(effective.model.cache_dir, Some("/request/cache".into()));
    }

    #[test]
    fn sceptre_reader_cache_key_distinguishes_model_cache_roots() {
        let mut first = sceptre::OcrConfig::default();
        first.model.cache_dir = Some("/first/cache".into());
        let mut second = first.clone();
        second.model.cache_dir = Some("/second/cache".into());

        assert_ne!(
            reader_cache_key(&first).expect("first config should serialize"),
            reader_cache_key(&second).expect("second config should serialize")
        );
    }

    #[cfg(auto_rotate)]
    #[test]
    fn sceptre_orientation_detector_cache_distinguishes_root_and_acceleration() {
        let backend = SceptreOcrBackend::new().expect("backend should initialize");
        let first_config = OcrConfig {
            backend_options: Some(serde_json::json!({"model": {"cache_dir": "/first/cache"}})),
            ..OcrConfig::default()
        };
        let second_config = OcrConfig {
            backend_options: Some(serde_json::json!({"model": {"cache_dir": "/second/cache"}})),
            ..OcrConfig::default()
        };
        let (first_config, _) =
            SceptreOcrBackend::effective_config(&first_config).expect("first config should resolve");
        let (second_config, _) =
            SceptreOcrBackend::effective_config(&second_config).expect("second config should resolve");
        let first = backend
            .orientation_detector_for_config(
                &first_config,
                Some(crate::core::config::AccelerationConfig {
                    provider: ExecutionProviderType::Cpu,
                    device_id: 0,
                }),
            )
            .expect("first detector should resolve");
        let repeated = backend
            .orientation_detector_for_config(
                &first_config,
                Some(crate::core::config::AccelerationConfig {
                    provider: ExecutionProviderType::Cpu,
                    device_id: 0,
                }),
            )
            .expect("repeated detector should resolve");
        let other_root = backend
            .orientation_detector_for_config(
                &second_config,
                Some(crate::core::config::AccelerationConfig {
                    provider: ExecutionProviderType::Cpu,
                    device_id: 0,
                }),
            )
            .expect("detector for another root should resolve");
        let other_acceleration = backend
            .orientation_detector_for_config(
                &first_config,
                Some(crate::core::config::AccelerationConfig {
                    provider: ExecutionProviderType::Cuda,
                    device_id: 1,
                }),
            )
            .expect("detector for another acceleration config should resolve");

        assert!(Arc::ptr_eq(&first, &repeated));
        assert!(!Arc::ptr_eq(&first, &other_root));
        assert!(!Arc::ptr_eq(&first, &other_acceleration));
    }

    #[cfg(auto_rotate)]
    #[test]
    fn sceptre_orientation_detector_cache_is_bounded_when_roots_change() {
        let backend = SceptreOcrBackend::new().expect("backend should initialize");

        for index in 0..=ORIENTATION_DETECTOR_CACHE_CAPACITY {
            backend
                .orientation_detector_cell(
                    std::path::Path::new(&format!("/cache/{index}")),
                    Some(crate::core::config::AccelerationConfig {
                        provider: ExecutionProviderType::Cpu,
                        device_id: 0,
                    }),
                )
                .expect("detector should resolve");
        }

        assert_eq!(
            backend
                .orientation_detectors
                .lock()
                .expect("detector cache lock should remain available")
                .entries
                .len(),
            ORIENTATION_DETECTOR_CACHE_CAPACITY
        );
    }

    #[cfg(auto_rotate)]
    #[test]
    fn sceptre_orientation_detector_cache_rejects_a_fifth_live_root() {
        let backend = SceptreOcrBackend::new().expect("backend should initialize");
        let mut active = Vec::new();
        for index in 0..ORIENTATION_DETECTOR_CACHE_CAPACITY {
            active.push(
                backend
                    .orientation_detector_cell(
                        std::path::Path::new(&format!("/active/{index}")),
                        Some(crate::core::config::AccelerationConfig {
                            provider: ExecutionProviderType::Cpu,
                            device_id: 0,
                        }),
                    )
                    .expect("active detector should resolve"),
            );
        }

        let repeated = backend
            .orientation_detector_cell(
                std::path::Path::new("/active/0"),
                Some(crate::core::config::AccelerationConfig {
                    provider: ExecutionProviderType::Cpu,
                    device_id: 0,
                }),
            )
            .expect("an existing active detector should be reused");
        let error = match backend.orientation_detector_cell(
            std::path::Path::new("/active/overflow"),
            Some(crate::core::config::AccelerationConfig {
                provider: ExecutionProviderType::Cpu,
                device_id: 0,
            }),
        ) {
            Ok(_) => panic!("a fifth live detector must exceed bounded admission"),
            Err(error) => error,
        };

        assert!(Arc::ptr_eq(&active[0], &repeated));
        assert_eq!(
            error.to_string(),
            "OCR error: Sceptre orientation detector cache is at its bounded capacity of 4 active configurations"
        );
        assert_eq!(
            backend
                .orientation_detectors
                .lock()
                .expect("detector cache lock should remain available")
                .entries
                .len(),
            ORIENTATION_DETECTOR_CACHE_CAPACITY
        );
    }

    #[cfg(auto_rotate)]
    #[test]
    fn sceptre_disabled_auto_rotation_does_not_cache_a_detector() {
        let backend = SceptreOcrBackend::new().expect("backend should initialize");
        let request = OcrConfig {
            auto_rotate: false,
            backend_options: Some(serde_json::json!({"model": {"cache_dir": "/unused/cache"}})),
            ..OcrConfig::default()
        };
        let (effective, _) = SceptreOcrBackend::effective_config(&request).expect("config should resolve");

        let rotation = backend
            .rotation_context(&request, &effective)
            .expect("rotation context should resolve");

        assert!(rotation.detector.is_none());
        assert_eq!(
            backend
                .orientation_detectors
                .lock()
                .expect("detector cache lock should remain available")
                .entries
                .len(),
            0
        );
    }

    #[test]
    fn should_propagate_xberg_thread_budget_when_unspecified() {
        let (effective, _) = SceptreOcrBackend::effective_config(&OcrConfig::default()).expect("valid config");
        assert_eq!(
            effective.concurrency.max_threads,
            Some(crate::core::config::concurrency::active_thread_budget())
        );
    }

    #[test]
    fn should_clamp_sceptre_threads_to_xberg_budget() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({ "concurrency": { "max_threads": usize::MAX } })),
            ..Default::default()
        };
        let (effective, _) = SceptreOcrBackend::effective_config(&config).expect("valid config");
        assert_eq!(
            effective.concurrency.max_threads,
            Some(crate::core::config::concurrency::active_thread_budget())
        );
    }

    #[cfg(all(feature = "sceptre-ocr-tract", not(feature = "sceptre-ocr-ort")))]
    #[test]
    fn tract_only_build_should_select_tract_by_default() {
        let (effective, _) = SceptreOcrBackend::effective_config(&OcrConfig::default()).expect("valid config");
        assert_eq!(effective.model.backend, Backend::Tract);
    }

    #[cfg(not(feature = "sceptre-ocr-tract"))]
    #[test]
    fn ort_only_build_should_reject_explicit_tract() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({ "model": { "backend": "tract" } })),
            ..Default::default()
        };
        let error = SceptreOcrBackend::effective_config(&config).expect_err("tract must require its feature");
        assert!(error.to_string().contains("sceptre-ocr-tract"));
    }

    // Candle is never the compile-time default (unlike tract in a tract-only build):
    // selecting it always requires an explicit `backend_options.model.backend = "candle"`,
    // so there is no "candle_only_build_should_select_candle_by_default" counterpart here.
    #[cfg(not(feature = "sceptre-ocr-candle"))]
    #[test]
    fn build_without_candle_should_reject_explicit_candle() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({ "model": { "backend": "candle" } })),
            ..Default::default()
        };
        let error = SceptreOcrBackend::effective_config(&config).expect_err("candle must require its feature");
        assert!(error.to_string().contains("sceptre-ocr-candle"));
    }

    #[cfg(feature = "sceptre-ocr-candle")]
    #[test]
    fn should_accept_explicit_candle_backend_when_compiled_in() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({ "model": { "backend": "candle" } })),
            ..Default::default()
        };
        let (effective, _) =
            SceptreOcrBackend::effective_config(&config).expect("candle backend must be accepted explicitly");
        assert_eq!(effective.model.backend, Backend::Candle);
    }

    #[test]
    fn should_ignore_options_owned_by_other_backends() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({ "custom_backend_option": true })),
            ..Default::default()
        };
        let parsed = parse_sceptre_options(&config).expect("unrelated options must be ignored");
        assert_eq!(
            parsed.recognition.batch_size,
            sceptre::OcrConfig::default().recognition.batch_size
        );
    }

    #[test]
    fn should_reject_unknown_sceptre_section_option() {
        let config = OcrConfig {
            backend_options: Some(serde_json::json!({ "recognition": { "typo": true } })),
            ..Default::default()
        };
        let error = parse_sceptre_options(&config).expect_err("unknown Sceptre fields must fail");
        assert!(error.to_string().contains("backend_options.recognition"));
    }

    #[test]
    fn should_reject_non_cpu_acceleration() {
        let config = OcrConfig {
            acceleration: Some(crate::core::config::AccelerationConfig {
                provider: ExecutionProviderType::CoreMl,
                ..Default::default()
            }),
            ..Default::default()
        };

        let error = SceptreOcrBackend::effective_config(&config).expect_err("CoreML must be rejected");
        assert!(error.to_string().contains("CPU-only"));
    }

    #[cfg(not(auto_rotate))]
    #[test]
    fn should_reject_auto_rotate_without_capability() {
        let config = OcrConfig {
            auto_rotate: true,
            ..Default::default()
        };

        let error = SceptreOcrBackend::effective_config(&config).expect_err("auto-rotation must require its feature");
        assert!(error.to_string().contains("auto-rotation is unavailable"));
    }

    #[test]
    fn should_build_line_geometry_and_clamp_confidence() {
        let element = line_to_element(&line("hello", 1.4));
        assert_eq!(element.text, "hello");
        assert_eq!(element.level, OcrElementLevel::Line);
        assert_eq!(element.confidence.recognition, 1.0);
        assert_eq!(
            element.geometry,
            OcrBoundingGeometry::Quadrilateral {
                points: [(10, 20), (111, 20), (111, 43), (10, 43)]
                    .into_iter()
                    .map(Into::into)
                    .collect()
            }
        );
    }

    #[test]
    fn should_join_lines_and_build_layout_document() {
        let output = BlockingOutput {
            result: OcrResult {
                lines: vec![line("first", 0.9), line("second", 0.8)],
            },
            width: 640,
            height: 480,
            #[cfg(auto_rotate)]
            orientation: None,
            #[cfg(auto_rotate)]
            auto_rotated: false,
        };
        let result = build_document(output, vec!["eng".to_string()], &OcrConfig::default());
        assert_eq!(result.content, "first\nsecond");
        let internal = result.ocr_internal_document.expect("internal OCR document");
        assert_eq!(internal.elements.len(), 2);
        let first = &internal.elements[0];
        assert_eq!(
            first.ocr_confidence.as_ref().expect("confidence").recognition,
            0.9_f32 as f64
        );
        assert_eq!(
            first.bbox,
            Some(BoundingBox {
                x0: 10.0,
                y0: 20.0,
                x1: 111.0,
                y1: 43.0,
            })
        );
        assert!(result.ocr_elements.is_none());
    }

    #[test]
    fn should_filter_requested_line_elements_by_confidence() {
        let config = OcrConfig {
            element_config: Some(crate::types::OcrElementConfig {
                include_elements: true,
                min_level: OcrElementLevel::Line,
                min_confidence: 0.85,
                build_hierarchy: false,
            }),
            ..Default::default()
        };
        let elements = vec![line_to_element(&line("keep", 0.9)), line_to_element(&line("drop", 0.8))];
        let selected = select_output_elements(&elements, &config).expect("one selected line");
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].text, "keep");
    }

    /// Regression test for issue #186: Sceptre must publish confidence
    /// statistics under the same keys Tesseract uses, so quality gating that
    /// reads `mean_text_conf` / `median_word_conf` / `p10_word_conf` /
    /// `word_count` / `low_conf_word_count` works against Sceptre output too.
    #[test]
    fn should_publish_confidence_statistics_in_metadata() {
        let output = BlockingOutput {
            result: OcrResult {
                // percentages: 90, 80, 40, 100 -> mean 77, sorted [40,80,90,100] ~keep
                lines: vec![line("a", 0.9), line("b", 0.8), line("c", 0.4), line("d", 1.0)],
            },
            width: 100,
            height: 100,
            #[cfg(auto_rotate)]
            orientation: None,
            #[cfg(auto_rotate)]
            auto_rotated: false,
        };

        let metadata = build_metadata(&["eng".to_string()], &output);

        assert_eq!(metadata.additional.get("mean_text_conf"), Some(&serde_json::json!(77)));
        assert_eq!(
            metadata.additional.get("median_word_conf"),
            Some(&serde_json::json!(85))
        );
        assert_eq!(metadata.additional.get("p10_word_conf"), Some(&serde_json::json!(40)));
        assert_eq!(metadata.additional.get("word_count"), Some(&serde_json::json!(4)));
        assert_eq!(
            metadata.additional.get("low_conf_word_count"),
            Some(&serde_json::json!(1))
        );
    }

    /// With no recognized lines there is nothing to report; the confidence keys
    /// must be absent rather than fabricated zeros (matching Tesseract's
    /// behavior of only inserting `mean_text_conf` when `>= 0`).
    #[test]
    fn should_omit_confidence_statistics_when_no_lines_recognized() {
        let output = BlockingOutput {
            result: OcrResult { lines: vec![] },
            width: 100,
            height: 100,
            #[cfg(auto_rotate)]
            orientation: None,
            #[cfg(auto_rotate)]
            auto_rotated: false,
        };

        let metadata = build_metadata(&["eng".to_string()], &output);

        assert!(metadata.additional.get("mean_text_conf").is_none());
        assert!(metadata.additional.get("word_count").is_none());
    }

    /// Regression test for issue #186: `psm` must not carry Tesseract's PSM-3
    /// default, since Sceptre has no Page Segmentation Mode concept.
    #[test]
    fn should_not_report_tesseract_psm_for_sceptre_metadata() {
        let output = BlockingOutput {
            result: OcrResult {
                lines: vec![line("hello", 0.9)],
            },
            width: 100,
            height: 100,
            #[cfg(auto_rotate)]
            orientation: None,
            #[cfg(auto_rotate)]
            auto_rotated: false,
        };

        let metadata = build_metadata(&["eng".to_string()], &output);
        let Some(FormatMetadata::Ocr(ocr_metadata)) = metadata.format else {
            panic!("expected FormatMetadata::Ocr");
        };
        assert_eq!(
            ocr_metadata.psm, 0,
            "Sceptre has no PSM; must not claim Tesseract's PSM 3"
        );
    }

    #[test]
    fn cache_key_should_change_with_reader_configuration() {
        let first = sceptre::OcrConfig::default();
        let mut second = first.clone();
        second.recognition.batch_size = 8;
        assert_ne!(
            reader_cache_key(&first).expect("key"),
            reader_cache_key(&second).expect("key")
        );
    }

    #[tokio::test]
    async fn reader_cache_should_reuse_cell_for_same_key() {
        let backend = SceptreOcrBackend::new().expect("backend should construct");
        let first = backend.reader_cell("english").await.expect("cache lookup");
        let second = backend.reader_cell("english").await.expect("cache lookup");

        assert!(Arc::ptr_eq(&first, &second));
        let cache = backend.readers.lock().expect("cache lock");
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.recency, VecDeque::from(["english".to_string()]));
    }

    #[tokio::test]
    async fn reader_cache_should_evict_least_recently_used_inactive_cell() {
        let backend = SceptreOcrBackend::new().expect("backend should construct");
        for index in 0..READER_CACHE_CAPACITY {
            drop(
                backend
                    .reader_cell(&format!("reader-{index}"))
                    .await
                    .expect("cache lookup"),
            );
        }

        drop(backend.reader_cell("reader-0").await.expect("cache lookup"));
        drop(backend.reader_cell("replacement").await.expect("cache lookup"));

        let cache = backend.readers.lock().expect("cache lock");
        assert_eq!(cache.entries.len(), READER_CACHE_CAPACITY);
        assert!(cache.entries.contains_key("reader-0"));
        assert!(!cache.entries.contains_key("reader-1"));
        assert!(cache.entries.contains_key("replacement"));
        assert_eq!(
            cache.recency,
            VecDeque::from([
                "reader-2".to_string(),
                "reader-3".to_string(),
                "reader-0".to_string(),
                "replacement".to_string(),
            ])
        );
    }

    #[tokio::test]
    async fn reader_cache_should_bound_distinct_overflow_admission() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        const OVERFLOW_REQUESTS: usize = 8;

        let backend = Arc::new(SceptreOcrBackend::new().expect("backend should construct"));
        let mut cached = Vec::with_capacity(READER_CACHE_CAPACITY);
        for index in 0..READER_CACHE_CAPACITY {
            cached.push(
                backend
                    .reader_cell(&format!("reader-{index}"))
                    .await
                    .expect("cache lookup"),
            );
        }

        let first_overflow = backend.reader_cell("overflow-0").await.expect("overflow admission");
        let same_overflow = backend.reader_cell("overflow-0").await.expect("overflow reuse");
        assert!(Arc::ptr_eq(&first_overflow, &same_overflow));

        let active_overflow = Arc::new(AtomicUsize::new(0));
        let maximum_overflow = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::with_capacity(OVERFLOW_REQUESTS);
        for index in 1..=OVERFLOW_REQUESTS {
            let backend = Arc::clone(&backend);
            let active_overflow = Arc::clone(&active_overflow);
            let maximum_overflow = Arc::clone(&maximum_overflow);
            tasks.push(tokio::spawn(async move {
                let cell = backend
                    .reader_cell(&format!("overflow-{index}"))
                    .await
                    .expect("overflow admission");
                let active = active_overflow.fetch_add(1, Ordering::SeqCst) + 1;
                maximum_overflow.fetch_max(active, Ordering::SeqCst);
                tokio::task::yield_now().await;
                active_overflow.fetch_sub(1, Ordering::SeqCst);
                drop(cell);
            }));
        }

        tokio::task::yield_now().await;
        assert_eq!(maximum_overflow.load(Ordering::SeqCst), 0);
        drop(same_overflow);
        drop(first_overflow);
        for task in tasks {
            task.await.expect("overflow task");
        }

        assert_eq!(maximum_overflow.load(Ordering::SeqCst), READER_OVERFLOW_CAPACITY);
        assert_eq!(backend.overflow_slots.available_permits(), READER_OVERFLOW_CAPACITY);
        let cache = backend.readers.lock().expect("cache lock");
        assert_eq!(cache.entries.len(), READER_CACHE_CAPACITY);
        assert!(cache.overflow.len() <= READER_OVERFLOW_CAPACITY);
        drop(cache);
        drop(cached);
    }

    #[tokio::test]
    async fn should_reject_invalid_image_without_loading_models() {
        let backend = SceptreOcrBackend::new().expect("backend should construct");
        let error = backend
            .process_image(b"not an image", &OcrConfig::default())
            .await
            .expect_err("invalid image data must fail");

        assert_eq!(error.to_string(), "OCR error: Sceptre OCR operation failed");
        let XbergError::Ocr { source, .. } = error else {
            panic!("invalid Sceptre image data must return an OCR error");
        };
        assert!(
            source.is_some(),
            "the Sceptre decode error must remain in the source chain"
        );
    }

    #[test]
    fn should_reject_oversized_declared_dimensions_before_sceptre_decode() {
        let bytes = crate::extraction::image_decode::bmp_with_declared_dimensions(6000, 6000);

        let error = decode_sceptre_image(&bytes)
            .expect_err("Sceptre decode must apply the default security budget before model work");

        let XbergError::Ocr {
            source: Some(source), ..
        } = error
        else {
            panic!("Sceptre budget rejection must retain the validation error as its source");
        };
        assert!(source.to_string().contains("6000x6000"));
        assert!(source.to_string().contains("security_limits.max_content_size"));
    }

    #[test]
    fn plugin_version_matches_sceptre_crate() {
        let backend = SceptreOcrBackend::new().expect("backend should construct");
        assert_eq!(backend.version(), sceptre::VERSION);
    }

    /// Axis-aligned `TextLine` at the given extent, for block-grouping tests.
    fn word_line(text: &str, x0: f32, x1: f32, y0: f32, y1: f32, confidence: f32) -> TextLine {
        TextLine {
            quad: Quad {
                points: [
                    Point::new(x0, y0),
                    Point::new(x1, y0),
                    Point::new(x1, y1),
                    Point::new(x0, y1),
                ],
            },
            text: text.to_string(),
            confidence,
        }
    }

    /// Regression for the "no block/paragraph grouping" WP-D defect (#668):
    /// `build_internal_document` never set `attributes`, so
    /// `pdf::structure::adapters::hocr_block_id` always returned `None` and
    /// every sceptre line became its own paragraph. This mirrors the
    /// geometric grouping already applied to PaddleOCR
    /// (`ocr::conversion::assign_line_block_ids`, #631) via the intentionally
    /// duplicated `assign_sceptre_block_ids` (see its doc comment for why it
    /// is not a shared call).
    ///
    /// Against unfixed code (`internal.attributes` left `None`, as it was
    /// before this change), `block_id(0)` and `block_id(1)` both return `None`
    /// and the `.expect(...)` calls below panic.
    #[test]
    fn should_assign_shared_block_id_to_close_lines_and_a_new_id_after_a_gap() {
        let elements = vec![
            line_to_element(&word_line("First line", 10.0, 200.0, 100.0, 120.0, 0.9)),
            line_to_element(&word_line("Second line", 10.0, 200.0, 124.0, 144.0, 0.9)),
            line_to_element(&word_line("Far below", 10.0, 200.0, 400.0, 420.0, 0.9)),
        ];

        let document = build_internal_document(&elements);

        let block_id = |index: usize| {
            document.elements[index]
                .attributes
                .as_ref()
                .and_then(|attributes| attributes.get(HOCR_BLOCK_ID_ATTRIBUTE))
                .cloned()
        };

        let first = block_id(0).expect("first line must carry a block id");
        let second = block_id(1).expect("second line must carry a block id");
        let third = block_id(2).expect("third line must carry a block id");

        assert_eq!(first, second, "vertically adjacent lines must share a block id");
        assert_ne!(second, third, "a large vertical gap must start a new block");
    }

    /// Regression for a structural defect in `lines_share_block`, found by feeding it
    /// Sceptre's own detected geometry (not a Tesseract proxy) for
    /// `test_documents/pdf_scanned/ordinance_2197_scanned.pdf`: Sceptre's `width_ths`
    /// detection parameter under-merges this line, so one physical line surfaces as 9
    /// separate `TextLine` entries ("In" / "accordance" / "with" / "Section" / "2-133,"
    /// / "the" / "PD" / "must" / "be"), reproduced here with their real detected
    /// coordinates.
    ///
    /// Each fragment's vertical gap against its predecessor is already 0 (their y-ranges
    /// overlap), so the vertical-gap check alone never rejected them. The bug was the
    /// horizontal overlap/left-edge fallback that follows it: consecutive word fragments
    /// don't x-overlap and their left edges march rightward across the line, so
    /// `overlap_ratio` stays 0 and `left_edge_offset` (~47-260px) blows past the ~16-23px
    /// threshold derived from the line's own ~32-36px height -- producing one spurious
    /// block per word instead of one block for the whole line.
    ///
    /// Against unfixed code (no y-center short-circuit in `lines_share_block`), each
    /// fragment after the first starts a new block, so `block_id(1)` (`"accordance"`)
    /// differs from `block_id(0)` (`"In"`) and the first `assert_eq!` below fails.
    #[test]
    fn should_merge_word_fragments_of_one_sceptre_line_sharing_a_ycenter() {
        let elements = vec![
            line_to_element(&word_line("In", 718.0, 752.0, 906.0, 938.0, 0.9)),
            line_to_element(&word_line("accordance", 765.0, 923.0, 905.0, 941.0, 0.9)),
            line_to_element(&word_line("with", 936.0, 1002.0, 908.0, 940.0, 0.9)),
            line_to_element(&word_line("Section", 1015.0, 1123.0, 905.0, 941.0, 0.9)),
            line_to_element(&word_line("2-133,", 1135.0, 1223.0, 905.0, 941.0, 0.9)),
            line_to_element(&word_line("the", 1244.0, 1292.0, 908.0, 940.0, 0.9)),
            line_to_element(&word_line("PD", 1306.0, 1354.0, 908.0, 940.0, 0.9)),
            line_to_element(&word_line("must", 1368.0, 1444.0, 910.0, 942.0, 0.9)),
            line_to_element(&word_line("be", 1458.0, 1496.0, 908.0, 940.0, 0.9)),
            // A genuinely new paragraph, far below: must still start a new block.
            line_to_element(&word_line("2_", 285.0, 305.0, 1059.0, 1087.0, 0.9)),
        ];

        let document = build_internal_document(&elements);

        let block_id = |index: usize| {
            document.elements[index]
                .attributes
                .as_ref()
                .and_then(|attributes| attributes.get(HOCR_BLOCK_ID_ATTRIBUTE))
                .cloned()
                .expect("every line must carry a block id")
        };

        let line_block_ids: Vec<String> = (0..9).map(block_id).collect();
        for (index, id) in line_block_ids.iter().enumerate().skip(1) {
            assert_eq!(
                *id, line_block_ids[0],
                "word fragment {index} must share the same-line block id as the first fragment"
            );
        }

        assert_ne!(
            block_id(9),
            line_block_ids[0],
            "a distant, unrelated line must still start a new block"
        );
    }

    /// Sceptre counterpart of
    /// `ocr::conversion::assign_line_block_ids_splits_indent_marked_paragraphs_with_uniform_leading`.
    /// The PaddleOCR grouper gained an indent check after a 16-page scanned
    /// document produced exactly 16 paragraphs; `assign_sceptre_block_ids` was
    /// copied from it before that check existed and never received it, so this
    /// same page collapses into a single block under sceptre.
    ///
    /// Paragraph breaks here are marked ONLY by first-line indentation (left
    /// margin 200px, indent 300px): every line sits 16px below its predecessor,
    /// well inside the 0.6x-line-height gap threshold, and every pair overlaps
    /// horizontally. Against unfixed code all eight entries come back as
    /// `"sceptre-block-1"`.
    #[test]
    fn should_split_indent_marked_paragraphs_that_share_uniform_leading() {
        let elements = vec![
            // Paragraph 1: indented first line, two flush continuation lines.
            line_to_element(&word_line("Para one first", 300.0, 2200.0, 200.0, 234.0, 0.9)),
            line_to_element(&word_line("continues here", 200.0, 2200.0, 250.0, 284.0, 0.9)),
            line_to_element(&word_line("and ends", 200.0, 1000.0, 300.0, 334.0, 0.9)),
            // Paragraph 2: indented first line, same 16px gap as within paragraph 1.
            line_to_element(&word_line("Para two first", 300.0, 2200.0, 350.0, 384.0, 0.9)),
            line_to_element(&word_line("continues here", 200.0, 2200.0, 400.0, 434.0, 0.9)),
            line_to_element(&word_line("and ends", 200.0, 700.0, 450.0, 484.0, 0.9)),
            // Paragraph 3: indented first line, same 16px gap again.
            line_to_element(&word_line("Para three first", 300.0, 2200.0, 500.0, 534.0, 0.9)),
            line_to_element(&word_line("continues here", 200.0, 2200.0, 550.0, 584.0, 0.9)),
        ];

        let block_ids = assign_sceptre_block_ids(&elements);

        assert_eq!(
            block_ids,
            vec![
                "sceptre-block-1",
                "sceptre-block-1",
                "sceptre-block-1",
                "sceptre-block-2",
                "sceptre-block-2",
                "sceptre-block-2",
                "sceptre-block-3",
                "sceptre-block-3",
            ],
            "indented first lines must start a new block even though the vertical gap never changes"
        );
    }

    /// Companion control to the indent-split test: OCR boxes are never
    /// pixel-perfect, so continuation lines flush with the block's established
    /// left margin must not be split apart by sub-pixel left-edge jitter.
    #[test]
    fn should_not_split_flush_continuation_lines_on_left_edge_jitter() {
        let elements = vec![
            line_to_element(&word_line("Indented first", 300.0, 2200.0, 200.0, 234.0, 0.9)),
            line_to_element(&word_line("flush second", 201.0, 2200.0, 250.0, 284.0, 0.9)),
            line_to_element(&word_line("flush third", 199.0, 2200.0, 300.0, 334.0, 0.9)),
        ];

        let block_ids = assign_sceptre_block_ids(&elements);

        assert_eq!(
            block_ids,
            vec!["sceptre-block-1", "sceptre-block-1", "sceptre-block-1"],
            "sub-pixel left-edge jitter on flush continuation lines must not start a new block"
        );
    }
}
