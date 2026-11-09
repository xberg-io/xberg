//! Bounded orientation recovery for low-confidence whole-page PDF OCR.

#[cfg(auto_rotate)]
mod prose;
mod recognition;
#[cfg(auto_rotate)]
mod two_tables;

use std::sync::Arc;

use crate::cancellation::CancellationToken;
use crate::core::config::{ExtractionConfig, OcrConfig, OcrQualityThresholds};
#[cfg(auto_rotate)]
use crate::doc_orientation::detector::tesseract_detector;
use crate::extractors::security::SecurityBudget;
#[cfg(any(auto_rotate, test))]
use crate::extractors::security::SecurityLimits;
use crate::ocr_metadata_keys::OCR_RECOGNITION_PAGE_DOWN_METADATA_KEY;
use crate::plugins::{ConfidenceSemantics, OcrBackend};
use crate::table_core::{HocrWord, detect_rows, find_row_index};
use crate::types::{BoundingBox, ExtractedDocument, OcrElement, OcrElementConfig, OcrElementLevel, Table};

use super::document::processed_ocr_layout_dimensions;
#[cfg(auto_rotate)]
use super::document::resolved_ocr_correction_degrees;
use super::scoring::{OcrRecognitionNoiseDecision, mean_text_conf_of, ocr_recognition_noise_decision, word_count_of};

pub(crate) fn recognition_retry_eligible(config: &OcrConfig) -> bool {
    recognition::eligible(config)
}

pub(super) struct OrientationRetry {
    thresholds: OcrQualityThresholds,
    attempts: parking_lot::Mutex<SecurityBudget>,
    #[cfg(auto_rotate)]
    cached_model_ready: tokio::sync::OnceCell<bool>,
    recognition_config: Option<OcrConfig>,
}

impl OrientationRetry {
    pub(super) fn new(config: &ExtractionConfig, thresholds: OcrQualityThresholds) -> Self {
        Self {
            thresholds,
            attempts: parking_lot::Mutex::new(SecurityBudget::from_config(config)),
            #[cfg(auto_rotate)]
            cached_model_ready: tokio::sync::OnceCell::new(),
            recognition_config: config
                .ocr
                .as_ref()
                .filter(|config| recognition_retry_eligible(config))
                .cloned(),
        }
    }

    pub(super) fn allows(config: &ExtractionConfig) -> bool {
        config
            .ocr
            .as_ref()
            .map_or(cfg!(auto_rotate), recognition_retry_eligible)
    }

    pub(super) fn primary_config(&self, config: &OcrConfig) -> Option<OcrConfig> {
        self.recognition_config
            .as_ref()
            .map(|_| recognition::capture_words(config))
    }

    #[cfg(feature = "pdf")]
    pub(super) async fn read_primary<T>(&self, config: &OcrConfig, read: impl std::future::Future<Output = T>) -> T {
        #[cfg(auto_rotate)]
        if self.recognition_config.is_none() && config.element_config.is_none() {
            use crate::ocr::tesseract_backend::orientation_evidence::{CAPTURE, Capture};
            return CAPTURE.scope(Capture::Primary, read).await;
        }
        let _ = config;
        read.await
    }

    fn noise(&self, result: &ExtractedDocument, confidence: ConfidenceSemantics) -> OcrRecognitionNoiseDecision {
        ocr_recognition_noise_decision(
            &result.content,
            &self.thresholds,
            confidence,
            mean_text_conf_of(&result.metadata.additional),
        )
    }

    #[cfg(auto_rotate)]
    async fn prepare_retry(
        &self,
        cancel: Option<&CancellationToken>,
        probe: impl std::future::Future<Output = bool>,
    ) -> crate::Result<bool> {
        check_cancelled(cancel)?;
        if self.attempts.lock().step().is_err() {
            return Ok(false);
        }
        let ready = self
            .cached_model_ready
            .get_or_init(|| async {
                tokio::task::spawn_blocking(|| tesseract_detector().prepare_cached())
                    .await
                    .unwrap_or(false)
            })
            .await;
        check_cancelled(cancel)?;
        if !ready {
            return Ok(false);
        }
        let has_evidence = probe.await;
        check_cancelled(cancel)?;
        Ok(has_evidence)
    }

    pub(super) async fn after_ocr(
        &self,
        backend: &Arc<dyn OcrBackend>,
        raster: Arc<Vec<u8>>,
        config: &OcrConfig,
        original: crate::Result<ExtractedDocument>,
        cancel: Option<&CancellationToken>,
        allow_prose: bool,
    ) -> crate::Result<ExtractedDocument> {
        let mut result = self
            .after_ocr_unscrubbed(backend, raster, config, original, cancel, allow_prose)
            .await?;
        result
            .metadata
            .additional
            .remove(OCR_RECOGNITION_PAGE_DOWN_METADATA_KEY);
        Ok(result)
    }

    async fn after_ocr_unscrubbed(
        &self,
        backend: &Arc<dyn OcrBackend>,
        raster: Arc<Vec<u8>>,
        config: &OcrConfig,
        original: crate::Result<ExtractedDocument>,
        cancel: Option<&CancellationToken>,
        allow_prose: bool,
    ) -> crate::Result<ExtractedDocument> {
        let original = original?;
        check_cancelled(cancel)?;
        if let Some(requested) = &self.recognition_config {
            let result = self
                .after_recognition(backend, raster, config, original, cancel, allow_prose)
                .await?;
            check_cancelled(cancel)?;
            return Ok(recognition::restore_elements(result, requested));
        }
        #[cfg(auto_rotate)]
        {
            let mut result = self.after_implicit(backend, raster, config, original, cancel).await?;
            check_cancelled(cancel)?;
            if config.element_config.is_none() {
                result.ocr_elements = None;
            }
            Ok(result)
        }
        #[cfg(not(auto_rotate))]
        {
            Ok(original)
        }
    }

    #[cfg(auto_rotate)]
    async fn after_implicit(
        &self,
        backend: &Arc<dyn OcrBackend>,
        raster: Arc<Vec<u8>>,
        config: &OcrConfig,
        original: ExtractedDocument,
        cancel: Option<&CancellationToken>,
    ) -> crate::Result<ExtractedDocument> {
        let confidence = backend.confidence_semantics();
        let empty = original.content.trim().is_empty();
        if !original.tables.is_empty()
            || !original.formulas.is_empty()
            || (!empty && !self.noise(&original, confidence).low_confidence)
        {
            return Ok(original);
        }
        let probe = async {
            if !empty {
                return true;
            }
            let raster = Arc::clone(&raster);
            let limits = config.security_limits.clone().unwrap_or_default();
            tokio::task::spawn_blocking(move || raster_has_glyphs(&raster, &limits))
                .await
                .unwrap_or(false)
        };
        if !self.prepare_retry(cancel, probe).await? {
            return Ok(original);
        }

        let (rotated_config, row_threshold) = implicit_config(config);
        #[cfg(feature = "pdf")]
        let retry = {
            use crate::ocr::tesseract_backend::orientation_evidence::{CAPTURE, Capture};
            CAPTURE
                .scope(
                    Capture::TwoTableCandidate,
                    backend.process_image_owned(raster, &rotated_config),
                )
                .await
        };
        #[cfg(not(feature = "pdf"))]
        let retry = backend.process_image_owned(raster, &rotated_config).await;
        check_cancelled(cancel)?;
        self.select_implicit(retry, original, config, confidence, row_threshold)
    }

    #[cfg(auto_rotate)]
    fn select_implicit(
        &self,
        retry: crate::Result<ExtractedDocument>,
        original: ExtractedDocument,
        config: &OcrConfig,
        confidence: ConfidenceSemantics,
        row_threshold: f64,
    ) -> crate::Result<ExtractedDocument> {
        match retry {
            Ok(mut candidate) if self.accepts_implicit(&candidate, &original, config, confidence, row_threshold) => {
                let evidence = candidate.ocr_elements.take().unwrap_or_default();
                candidate.ocr_elements = config
                    .element_config
                    .as_ref()
                    .map(|options| options.select_elements(&evidence))
                    .filter(|elements| !elements.is_empty());
                Ok(candidate)
            }
            Err(crate::XbergError::Cancelled) => Err(crate::XbergError::Cancelled),
            _ => Ok(original),
        }
    }

    async fn prepare_recognition(
        &self,
        cancel: Option<&CancellationToken>,
        probe: impl std::future::Future<Output = crate::Result<Option<String>>>,
    ) -> crate::Result<Option<String>> {
        check_cancelled(cancel)?;
        if self.attempts.lock().step().is_err() {
            return Ok(None);
        }
        let directory = probe.await?;
        check_cancelled(cancel)?;
        Ok(directory)
    }

    async fn after_recognition(
        &self,
        backend: &Arc<dyn OcrBackend>,
        raster: Arc<Vec<u8>>,
        config: &OcrConfig,
        original: ExtractedDocument,
        cancel: Option<&CancellationToken>,
        allow_prose: bool,
    ) -> crate::Result<ExtractedDocument> {
        let confidence = backend.confidence_semantics();
        if !recognition::plain_primary(&original)
            || !recognition::within_certificate_limits(&original)
            || !self.noise(&original, confidence).low_confidence
        {
            return Ok(original);
        }
        // A page-down primary is the half-turn's target and an upside-down read routinely fails the
        // word checks `accepts` demands (the sideways e2e recovers 1 of 480 tokens if they gate this
        // read), so only the marker decides whether the unsupported primary is replaced. ~keep
        let probe = lookup_osd_directory(config.clone());
        let Some(directory) = self.prepare_recognition(cancel, probe).await? else {
            // Without a cached OSD model the omitted policy takes the classifier route that
            // extraction without an `ocr` block uses. ~keep
            #[cfg(auto_rotate)]
            return self.after_implicit(backend, raster, config, original, cancel).await;
            #[cfg(not(auto_rotate))]
            return Ok(original);
        };
        check_cancelled(cancel)?;
        let mut retry_config = recognition::capture_words(config);
        retry_config.tessdata_path = Some(directory.into());
        retry_config.tesseract_config.get_or_insert_default().psm = Some(1);
        let candidate = requesting_page_down(backend.process_image_owned(Arc::clone(&raster), &retry_config)).await;
        check_cancelled(cancel)?;
        match candidate {
            Ok(mut candidate) => {
                let page_down = candidate
                    .metadata
                    .additional
                    .remove(OCR_RECOGNITION_PAGE_DOWN_METADATA_KEY)
                    .is_some();
                if recognition::accepts_candidate(
                    &original,
                    &candidate,
                    &self.thresholds,
                    confidence,
                    config,
                    allow_prose,
                ) {
                    Ok(recognition::adopt(original, candidate))
                } else if page_down {
                    let witness = recognition::witness_tokens(&candidate);
                    self.after_half_turn(backend, raster, config, original, &witness, cancel)
                        .await
                } else {
                    Ok(original)
                }
            }
            Err(crate::XbergError::Cancelled) => Err(crate::XbergError::Cancelled),
            Err(error) => {
                tracing::warn!(%error, "Optional recognition-orientation OCR failed; retaining primary OCR");
                Ok(original)
            }
        }
    }

    async fn after_half_turn(
        &self,
        backend: &Arc<dyn OcrBackend>,
        raster: Arc<Vec<u8>>,
        config: &OcrConfig,
        original: ExtractedDocument,
        witness: &[String],
        cancel: Option<&CancellationToken>,
    ) -> crate::Result<ExtractedDocument> {
        check_cancelled(cancel)?;
        if self.attempts.lock().step().is_err() {
            return Ok(original);
        }
        let limits = config.security_limits.clone().unwrap_or_default();
        let permit = HALF_TURN_PREPARE
            .acquire()
            .await
            .expect("half-turn semaphore never closes");
        let half_turned = match tokio::task::spawn_blocking(move || half_turn(&raster, &limits)).await {
            Ok(half_turned) => half_turned,
            Err(error) if error.is_cancelled() => return Err(crate::XbergError::Cancelled),
            Err(error) => {
                tracing::warn!(%error, "Optional half-turn raster preparation failed; retaining primary OCR");
                None
            }
        };
        drop(permit);
        check_cancelled(cancel)?;
        let Some(half_turned) = half_turned else {
            return Ok(original);
        };
        // The half-turned page is upright, so it is read the way the primary was: the caller's page
        // segmentation and tessdata, not the recognition read's forced PSM 1 and OSD directory (PSM 1
        // would run orientation detection again). Eligible configs never set a PSM. ~keep
        let candidate = backend
            .process_image_owned(Arc::new(half_turned), &recognition::capture_words(config))
            .await;
        check_cancelled(cancel)?;
        match candidate {
            Ok(candidate) => Ok(recognition::select_half_turned(
                original,
                candidate,
                &self.thresholds,
                backend.confidence_semantics(),
                config,
                witness,
            )),
            Err(crate::XbergError::Cancelled) => Err(crate::XbergError::Cancelled),
            Err(error) => {
                tracing::warn!(%error, "Optional half-turn OCR failed; retaining primary OCR");
                Ok(original)
            }
        }
    }

    #[cfg(auto_rotate)]
    fn accepts_implicit(
        &self,
        result: &ExtractedDocument,
        original: &ExtractedDocument,
        config: &OcrConfig,
        confidence: ConfidenceSemantics,
        row_threshold: f64,
    ) -> bool {
        if result.tables.len() != 2 {
            return self.accepts(result, confidence, row_threshold);
        }
        let default_limits = SecurityLimits::default();
        let limits = config.security_limits.as_ref().unwrap_or(&default_limits);
        two_tables::complete_text(result, original, &self.thresholds, limits, row_threshold)
            .is_some_and(|text| self.recovered_text_passes(result, &text, confidence))
    }

    #[cfg(auto_rotate)]
    fn accepts(&self, result: &ExtractedDocument, confidence: ConfidenceSemantics, row_threshold: f64) -> bool {
        if result.content.trim().is_empty()
            || !result.formulas.is_empty()
            || !resolved_ocr_correction_degrees(&result.metadata).is_some_and(|angle| angle != 0)
        {
            return false;
        }
        let table_text;
        let text = if result.tables.is_empty() {
            &result.content
        } else {
            let Some(text) = complete_table_text(result, row_threshold, table_matches_iterator_rows)
                .or_else(|| prose::complete_text(result, row_threshold))
            else {
                return false;
            };
            table_text = text;
            &table_text
        };
        self.recovered_text_passes(result, text, confidence)
    }

    #[cfg(auto_rotate)]
    fn recovered_text_passes(&self, result: &ExtractedDocument, text: &str, confidence: ConfidenceSemantics) -> bool {
        let mean_confidence = mean_text_conf_of(&result.metadata.additional);
        let noise = ocr_recognition_noise_decision(text, &self.thresholds, confidence, mean_confidence);
        !noise.low_confidence && !noise.fragmented_noise && mean_confidence.is_some()
    }
}

async fn lookup_osd_directory(config: OcrConfig) -> crate::Result<Option<String>> {
    match tokio::task::spawn_blocking(move || recognition::existing_osd_directory(&config)).await {
        Ok(directory) => Ok(directory),
        Err(error) if error.is_cancelled() => Err(crate::XbergError::Cancelled),
        Err(error) => {
            tracing::warn!(%error, "Optional recognition-orientation model lookup failed; retaining primary OCR");
            Ok(None)
        }
    }
}

#[cfg(auto_rotate)]
fn implicit_config(config: &OcrConfig) -> (OcrConfig, f64) {
    let mut rotated_config = config.clone();
    rotated_config.auto_rotate = Some(true);
    rotated_config.element_config = Some(OcrElementConfig {
        include_elements: true,
        min_level: OcrElementLevel::Word,
        ..Default::default()
    });
    let row_threshold = rotated_config.tesseract_config.as_ref().map_or_else(
        || crate::types::TesseractConfig::default().table_row_threshold_ratio,
        |config| config.table_row_threshold_ratio,
    );
    (rotated_config, row_threshold)
}

/// The half-turn raster exists only while this step runs; one page at a time process-wide keeps the
/// decoded and re-encoded copies inside a single `max_content_size` budget. ~keep
static HALF_TURN_PREPARE: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

fn half_turn(raster: &[u8], limits: &crate::extractors::security::SecurityLimits) -> Option<Vec<u8>> {
    let live_bytes = u64::try_from(raster.len()).ok()?;
    let mut rgb = crate::extraction::image_decode::decode_standard_rgb8_with_additional_live_bytes_and_security_limits(
        raster, limits, live_bytes,
    )
    .ok()?;
    image::imageops::rotate180_in_place(&mut rgb);
    encode_png_within(&rgb, limits.max_content_size)
}

fn encode_png_within(rgb: &image::RgbImage, max_bytes: usize) -> Option<Vec<u8>> {
    let mut encoded = Vec::new();
    rgb.write_to(&mut std::io::Cursor::new(&mut encoded), image::ImageFormat::Png)
        .ok()?;
    (encoded.len() <= max_bytes).then_some(encoded)
}

#[cfg(all(feature = "pdf", not(target_arch = "wasm32")))]
async fn requesting_page_down<T>(read: impl std::future::Future<Output = T>) -> T {
    crate::ocr::tesseract_backend::page_down_request::requesting(read).await
}

#[cfg(not(all(feature = "pdf", not(target_arch = "wasm32"))))]
async fn requesting_page_down<T>(read: impl std::future::Future<Output = T>) -> T {
    read.await
}

fn check_cancelled(cancel: Option<&CancellationToken>) -> crate::Result<()> {
    if cancel.is_some_and(CancellationToken::is_cancelled) {
        return Err(crate::XbergError::Cancelled);
    }
    Ok(())
}

#[cfg(auto_rotate)]
const GLYPH_SAMPLE_EDGE: u32 = 512;
// RGB thumbnail, luminance and foreground masks, and spare capacity for the glyph estimator's
// flood-fill stack, component heights and letter bodies. At most 20% of pixels are visited
// by the flood fill, so six usize slots per sample cover all three vectors and reallocations. ~keep
#[cfg(auto_rotate)]
const GLYPH_PROBE_WORKSPACE: usize =
    GLYPH_SAMPLE_EDGE as usize * GLYPH_SAMPLE_EDGE as usize * (5 + 6 * size_of::<usize>());

#[cfg(auto_rotate)]
fn raster_has_glyphs(raster: &[u8], limits: &SecurityLimits) -> bool {
    let Ok(rgb) = crate::extraction::image_decode::decode_standard_rgb8_with_additional_live_bytes_and_security_limits(
        raster,
        limits,
        GLYPH_PROBE_WORKSPACE as u64,
    ) else {
        return false;
    };
    let (width, height) = rgb.dimensions();
    let longest = width.max(height);
    let sample = if longest > GLYPH_SAMPLE_EDGE {
        let shrink = |length| ((u64::from(length) * u64::from(GLYPH_SAMPLE_EDGE)) / u64::from(longest)).max(1) as u32;
        image::imageops::thumbnail(&rgb, shrink(width), shrink(height))
    } else {
        rgb
    };
    crate::image::text_height::median_glyph_height(sample.as_raw(), sample.width() as usize, sample.height() as usize)
        .is_some()
}

fn single_table(result: &ExtractedDocument) -> Option<(&Table, BoundingBox, (u32, u32))> {
    const MIN_ROWS: usize = 3;
    const MIN_COLUMNS: usize = 2;
    let [table] = result.tables.as_slice() else {
        return None;
    };
    let first = table.cells.first()?;
    let (width, height) = processed_ocr_layout_dimensions(&result.metadata)?;
    let bbox = table.bounding_box?;
    if !(0.0 <= bbox.x0
        && bbox.x0 < bbox.x1
        && bbox.x1 <= f64::from(width)
        && 0.0 <= bbox.y0
        && bbox.y0 < bbox.y1
        && bbox.y1 <= f64::from(height))
        || table.cells.len() < MIN_ROWS
        || first.len() < MIN_COLUMNS
        || !table
            .cells
            .iter()
            .all(|row| row.len() == first.len() && row.iter().all(|cell| !cell.trim().is_empty()))
    {
        return None;
    }
    Some((table, bbox, (width, height)))
}

fn complete_table_text(
    result: &ExtractedDocument,
    row_threshold: f64,
    matches: impl Fn(&[Vec<String>], &[OcrElement], BoundingBox, f64) -> bool,
) -> Option<String> {
    let (table, bbox, _) = single_table(result)?;
    let words = table.cells.iter().flatten().flat_map(|cell| cell.split_whitespace());
    if usize::try_from(word_count_of(&result.metadata.additional)?).ok()? != words.clone().count()
        || (table.markdown.trim() != result.content.trim() && !words.clone().eq(result.content.split_whitespace()))
    {
        return None;
    }
    let evidence = result.ocr_elements.as_deref()?;
    if !matches(&table.cells, evidence, bbox, row_threshold) {
        return None;
    }
    Some(words.collect::<Vec<_>>().join(" "))
}

fn iterator_word(element: &OcrElement, bbox: BoundingBox) -> Option<HocrWord> {
    if element.backend_metadata.get("backend")?.as_str()? != "tesseract-iterator" {
        return None;
    }
    let (left, top, width, height) = element.geometry.to_aabb();
    let center_x = f64::from(left) + f64::from(width) / 2.0;
    let center_y = f64::from(top) + f64::from(height) / 2.0;
    (width > 0 && height > 0 && (bbox.x0..=bbox.x1).contains(&center_x) && (bbox.y0..=bbox.y1).contains(&center_y))
        .then(|| HocrWord {
            text: element.text.clone(),
            left,
            top,
            width,
            height,
            confidence: element.confidence.recognition,
        })
}

fn table_matches_iterator_rows(
    cells: &[Vec<String>],
    elements: &[OcrElement],
    bbox: BoundingBox,
    row_threshold: f64,
) -> bool {
    // Independently order the raw words by geometry. Tesseract's iterator may emit entire columns first. ~keep
    let Some(words) = elements
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word)
        .map(|element| iterator_word(element, bbox))
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    table_matches_words(cells, &words, row_threshold)
}

fn table_matches_words(cells: &[Vec<String>], words: &[HocrWord], row_threshold: f64) -> bool {
    let row_positions = detect_rows(words, row_threshold);
    if row_positions.len() != cells.len() {
        return false;
    }
    let mut rows = vec![Vec::new(); row_positions.len()];
    for word in words {
        let Some(row) = find_row_index(&row_positions, word) else {
            return false;
        };
        rows[row].push(word);
    }
    rows.into_iter().zip(cells).all(|(mut raw, cells)| {
        raw.sort_unstable_by_key(|word| word.left);
        raw.windows(2).all(|pair| pair[0].left < pair[1].left)
            && raw
                .iter()
                .flat_map(|word| word.text.split_whitespace())
                .eq(cells.iter().flat_map(|cell| cell.split_whitespace()))
    })
}

#[cfg(auto_rotate)]
pub(super) fn scale_recovered_table_to_raster(result: &mut ExtractedDocument, raster: (u32, u32)) {
    let Some((width, height)) = processed_ocr_layout_dimensions(&result.metadata) else {
        return;
    };
    if !matches!(result.tables.len(), 1 | 2) {
        return;
    }
    let scale_x = f64::from(raster.0) / f64::from(width);
    let scale_y = f64::from(raster.1) / f64::from(height);
    for bbox in result.tables.iter_mut().filter_map(|table| table.bounding_box.as_mut()) {
        bbox.x0 *= scale_x;
        bbox.x1 *= scale_x;
        bbox.y0 *= scale_y;
        bbox.y1 *= scale_y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ocr_metadata_keys::{OCR_AUTO_ROTATED_METADATA_KEY, OCR_ORIENTATION_DEGREES_METADATA_KEY};

    #[tokio::test]
    #[cfg(auto_rotate)]
    async fn cancelled_retry_does_not_probe() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(matches!(
            retry
                .prepare_retry(Some(&cancel), async { panic!("cancelled retry probed") })
                .await,
            Err(crate::XbergError::Cancelled)
        ));
        assert!(retry.cached_model_ready.get().is_none());
    }

    #[tokio::test]
    #[cfg(auto_rotate)]
    async fn exhausted_retry_does_not_probe() {
        let limits = SecurityLimits {
            max_iterations: 0,
            ..Default::default()
        };
        let exhausted = OrientationRetry::new(
            &ExtractionConfig {
                security_limits: Some(limits),
                ..Default::default()
            },
            OcrQualityThresholds::default(),
        );
        assert!(
            !exhausted
                .prepare_retry(None, async { panic!("exhausted retry probed") })
                .await
                .unwrap()
        );
        assert!(exhausted.cached_model_ready.get().is_none());
    }

    #[tokio::test]
    #[cfg(auto_rotate)]
    async fn cold_retry_does_not_probe() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        retry
            .cached_model_ready
            .set(false)
            .expect("model cache starts uninitialized");
        assert!(
            !retry
                .prepare_retry(None, async { panic!("cold retry probed") })
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    #[cfg(auto_rotate)]
    async fn cancellation_during_the_probe_prevents_retry() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        retry
            .cached_model_ready
            .set(true)
            .expect("model cache starts uninitialized");
        let cancel = CancellationToken::new();
        let result = retry
            .prepare_retry(Some(&cancel), async {
                cancel.cancel();
                true
            })
            .await;
        assert!(matches!(result, Err(crate::XbergError::Cancelled)));
    }

    #[tokio::test]
    async fn recognition_cancellation_and_exhaustion_precede_the_cache_probe() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(matches!(
            retry
                .prepare_recognition(Some(&cancel), async { panic!("cancelled OSD probe") })
                .await,
            Err(crate::XbergError::Cancelled)
        ));
        let exhausted = OrientationRetry::new(
            &ExtractionConfig {
                security_limits: Some(SecurityLimits {
                    max_iterations: 0,
                    ..Default::default()
                }),
                ..Default::default()
            },
            OcrQualityThresholds::default(),
        );
        assert!(
            exhausted
                .prepare_recognition(None, async { panic!("exhausted OSD probe") })
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn missing_osd_and_cancellation_after_cache_lookup_prevent_recognition() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        assert!(
            retry
                .prepare_recognition(None, async { Ok(None) })
                .await
                .unwrap()
                .is_none()
        );
        let cancel = CancellationToken::new();
        assert!(matches!(
            retry
                .prepare_recognition(Some(&cancel), async {
                    cancel.cancel();
                    Ok(Some("existing models".into()))
                })
                .await,
            Err(crate::XbergError::Cancelled)
        ));
    }

    #[test]
    #[cfg(auto_rotate)]
    fn empty_read_requires_glyphs_not_blank_or_ruled_paper() {
        let limits = SecurityLimits::default();
        let mut raster = image::RgbImage::from_pixel(120, 120, image::Rgb([255; 3]));
        let has_glyphs = |raster: &image::RgbImage| {
            let mut bytes = std::io::Cursor::new(Vec::new());
            raster
                .write_to(&mut bytes, image::ImageFormat::Png)
                .expect("PNG encodes");
            raster_has_glyphs(bytes.get_ref(), &limits)
        };
        assert!(!has_glyphs(&raster));

        for line in (12..120).step_by(12) {
            for x in 8..112 {
                raster.put_pixel(x, line, image::Rgb([0; 3]));
                raster.put_pixel(x, line + 1, image::Rgb([0; 3]));
            }
        }
        assert!(!has_glyphs(&raster));
        assert!(!has_glyphs(&image::imageops::rotate90(&raster)));

        for left in (12..100).step_by(12) {
            for y in 5..10 {
                for x in left..left + 3 {
                    raster.put_pixel(x, y, image::Rgb([0; 3]));
                }
            }
        }
        assert!(has_glyphs(&raster));
    }

    #[test]
    #[cfg(auto_rotate)]
    fn empty_raster_probe_accounts_for_workspace_beyond_decoded_rgb() {
        let mut raster = image::RgbImage::from_pixel(120, 120, image::Rgb([255; 3]));
        for left in (12..100).step_by(12) {
            for y in 5..10 {
                for x in left..left + 3 {
                    raster.put_pixel(x, y, image::Rgb([0; 3]));
                }
            }
        }
        let mut encoded = std::io::Cursor::new(Vec::new());
        raster
            .write_to(&mut encoded, image::ImageFormat::Png)
            .expect("PNG encodes");
        let bytes = encoded.into_inner();
        let required = bytes.len() + raster.as_raw().len() + GLYPH_PROBE_WORKSPACE;
        let mut limits = SecurityLimits {
            max_content_size: required,
            ..Default::default()
        };
        assert!(raster_has_glyphs(&bytes, &limits));
        limits.max_content_size -= 1;
        assert!(!raster_has_glyphs(&bytes, &limits));
    }

    #[test]
    #[cfg(auto_rotate)]
    fn recovery_requires_proved_rotation_and_rejects_unverified_tables() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let confidence = ConfidenceSemantics::Legibility { scale_max: 100.0 };
        let mut candidate = ExtractedDocument {
            content: "A recovered page of ordinary text".to_string(),
            ..Default::default()
        };
        let metadata = &mut candidate.metadata.additional;
        metadata.insert("mean_text_conf".into(), 94.into());
        metadata.insert(OCR_AUTO_ROTATED_METADATA_KEY.into(), true.into());
        metadata.insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), 0.into());
        assert!(!retry.accepts(&candidate, confidence, 0.5));

        candidate
            .metadata
            .additional
            .insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), 270.into());
        assert!(retry.accepts(&candidate, confidence, 0.5));

        candidate.metadata.additional.insert("mean_text_conf".into(), 67.into());
        assert!(!retry.accepts(&candidate, confidence, 0.5));
        candidate.metadata.additional.insert("mean_text_conf".into(), 94.into());
        candidate.tables.push(crate::types::Table::default());
        assert!(!retry.accepts(&candidate, confidence, 0.5));
    }

    fn table_iterator_evidence() -> Vec<OcrElement> {
        let mut evidence = Vec::new();
        for (row, text) in [
            ["Cash", "100", "31,290"],
            ["Rent", "107", "77,078"],
            ["Fuel", "114", "71,433"],
        ]
        .into_iter()
        .enumerate()
        {
            for (column, token) in text.into_iter().enumerate() {
                evidence.push(OcrElement {
                    text: token.into(),
                    level: OcrElementLevel::Word,
                    geometry: crate::types::OcrBoundingGeometry::Rectangle {
                        left: 30 + 25 * column as u32,
                        top: 50 + 30 * row as u32,
                        width: 10,
                        height: 10,
                    },
                    backend_metadata: std::collections::HashMap::from([(
                        "backend".into(),
                        "tesseract-iterator".into(),
                    )]),
                    ..Default::default()
                });
            }
        }
        evidence.reverse();
        evidence
    }

    pub(super) fn table_candidate() -> ExtractedDocument {
        let mut candidate = ExtractedDocument {
            content: "Cash 100 31,290 Rent 107 77,078 Fuel 114 71,433".into(),
            tables: vec![crate::types::Table {
                cells: vec![
                    vec!["Cash 100".into(), "31,290".into()],
                    vec!["Rent 107".into(), "77,078".into()],
                    vec!["Fuel 114".into(), "71,433".into()],
                ],
                bounding_box: Some(crate::types::BoundingBox {
                    x0: 20.0,
                    y0: 40.0,
                    x1: 180.0,
                    y1: 160.0,
                }),
                ..Default::default()
            }],
            ..Default::default()
        };
        candidate.ocr_elements = Some(table_iterator_evidence());
        let metadata = &mut candidate.metadata.additional;
        metadata.insert(
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(),
            200.into(),
        );
        metadata.insert(
            crate::ocr_metadata_keys::OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(),
            200.into(),
        );
        metadata.insert("word_count".into(), 9.into());
        metadata.insert("mean_text_conf".into(), 94.into());
        metadata.insert(OCR_AUTO_ROTATED_METADATA_KEY.into(), true.into());
        metadata.insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), 270.into());
        candidate
    }

    #[test]
    #[cfg(auto_rotate)]
    fn recovery_requires_exact_ordered_iterator_tokens_for_flat_or_markdown_tables() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let confidence = ConfidenceSemantics::Legibility { scale_max: 100.0 };
        let candidate = table_candidate();
        assert!(retry.accepts(&candidate, confidence, 0.5));

        let mut markdown_table = candidate.clone();
        markdown_table.tables[0].markdown =
            "| Cash 100 | 31,290 |\n| --- | --- |\n| Rent 107 | 77,078 |\n| Fuel 114 | 71,433 |".into();
        markdown_table.content = markdown_table.tables[0].markdown.clone();
        assert!(retry.accepts(&markdown_table, confidence, 0.5));

        let mut substituted = markdown_table.clone();
        substituted.tables[0].cells[0][1] = "31,299".into();
        substituted.tables[0].markdown = substituted.tables[0].markdown.replace("31,290", "31,299");
        substituted.content = substituted.tables[0].markdown.clone();
        assert!(
            !retry.accepts(&substituted, confidence, 0.5),
            "rendered markup cannot vouch for its own cells"
        );

        markdown_table.content.push_str("\nUnclaimed words");
        assert!(!retry.accepts(&markdown_table, confidence, 0.5));

        let mut mismatched_text = candidate.clone();
        mismatched_text.content = "Rent 107 77,078 Cash 100 31,290 Fuel 114 71,433".into();
        assert!(!retry.accepts(&mismatched_text, confidence, 0.5));
    }

    #[test]
    #[cfg(auto_rotate)]
    fn recovery_rejects_sparse_duplicate_and_off_page_tables() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let confidence = ConfidenceSemantics::Legibility { scale_max: 100.0 };
        let mut candidate = table_candidate();
        let mut sparse_table = candidate.clone();
        sparse_table.tables[0].cells[1][1].clear();
        sparse_table.content = "Cash 100 31,290 Rent 107 Fuel 114 71,433".into();
        assert!(!retry.accepts(&sparse_table, confidence, 0.5));

        let mut duplicate = candidate.clone();
        duplicate.tables.push(candidate.tables[0].clone());
        assert!(!retry.accepts(&duplicate, confidence, 0.5));

        let mut off_page = candidate.clone();
        off_page.tables[0].bounding_box.as_mut().unwrap().y1 = 201.0;
        assert!(!retry.accepts(&off_page, confidence, 0.5));

        scale_recovered_table_to_raster(&mut candidate, (100, 50));
        let bbox = candidate.tables[0].bounding_box.unwrap();
        assert_eq!((bbox.x0, bbox.y0, bbox.x1, bbox.y1), (10.0, 10.0, 90.0, 40.0));
    }

    #[test]
    #[cfg(auto_rotate)]
    fn table_retry_keeps_the_original_when_reported_word_count_disagrees() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let confidence = ConfidenceSemantics::Legibility { scale_max: 100.0 };
        let mut candidate = table_candidate();
        assert!(retry.accepts(&candidate, confidence, 0.5));
        candidate.metadata.additional.insert("word_count".into(), 8.into());
        let original = ExtractedDocument {
            content: "primary OCR text".into(),
            ..Default::default()
        };
        let selected = retry
            .select_implicit(Ok(candidate), original, &OcrConfig::default(), confidence, 0.5)
            .unwrap();
        assert_eq!(selected.content, "primary OCR text");
        assert!(selected.tables.is_empty());
    }

    #[test]
    #[cfg(auto_rotate)]
    fn accepted_table_retry_exposes_only_requested_confident_elements() {
        let retry = OrientationRetry::new(&ExtractionConfig::default(), OcrQualityThresholds::default());
        let confidence = ConfidenceSemantics::Legibility { scale_max: 100.0 };
        let mut candidate = table_candidate();
        let elements = candidate.ocr_elements.as_mut().unwrap();
        assert_eq!(elements.len(), 9);
        elements[0].confidence.recognition = 0.95;
        let expected = elements[0].text.clone();
        let config = OcrConfig {
            element_config: Some(OcrElementConfig {
                include_elements: true,
                min_level: OcrElementLevel::Word,
                min_confidence: 0.9,
                ..Default::default()
            }),
            ..Default::default()
        };
        let selected = retry
            .select_implicit(Ok(candidate), ExtractedDocument::default(), &config, confidence, 0.5)
            .unwrap();
        assert_eq!(selected.tables.len(), 1);
        let elements = selected.ocr_elements.unwrap();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].level, OcrElementLevel::Word);
        assert_eq!(elements[0].text, expected);
    }

    struct ScriptedBackend {
        replies: parking_lot::Mutex<std::collections::VecDeque<ExtractedDocument>>,
        rasters: parking_lot::Mutex<Vec<Vec<u8>>>,
        configs: parking_lot::Mutex<Vec<OcrConfig>>,
    }

    impl crate::plugins::Plugin for ScriptedBackend {
        fn name(&self) -> &str {
            "scripted"
        }

        fn version(&self) -> String {
            "1.0.0".into()
        }

        fn initialize(&self) -> crate::Result<()> {
            Ok(())
        }

        fn shutdown(&self) -> crate::Result<()> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl OcrBackend for ScriptedBackend {
        fn backend_type(&self) -> crate::plugins::OcrBackendType {
            crate::plugins::OcrBackendType::Custom
        }

        fn supports_language(&self, _lang: &str) -> bool {
            true
        }

        fn confidence_semantics(&self) -> ConfidenceSemantics {
            ConfidenceSemantics::Legibility { scale_max: 100.0 }
        }

        async fn process_image(&self, image_bytes: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
            self.rasters.lock().push(image_bytes.to_vec());
            self.configs.lock().push(config.clone());
            Ok(self.replies.lock().pop_front().expect("scripted reply"))
        }
    }

    fn marked_recognition_read() -> ExtractedDocument {
        let mut read = table_candidate();
        read.tables.clear();
        read.content = "Cash 100 31,290 Rent 107 77,078 Fuel 114 71,433".into();
        read.metadata
            .additional
            .insert(OCR_RECOGNITION_PAGE_DOWN_METADATA_KEY.into(), true.into());
        read
    }

    fn inverted_primary() -> ExtractedDocument {
        let mut primary = table_candidate();
        primary.tables.clear();
        primary.content = "zzq xkv".into();
        primary.metadata.additional.insert("mean_text_conf".into(), 10.into());
        primary
    }

    fn png_with_one_red_corner() -> Vec<u8> {
        let mut image = image::RgbImage::new(8, 6);
        image.put_pixel(0, 0, image::Rgb([255, 0, 0]));
        let mut encoded = Vec::new();
        image
            .write_to(&mut std::io::Cursor::new(&mut encoded), image::ImageFormat::Png)
            .unwrap();
        encoded
    }

    /// Runs the retry against a backend that returns `replies` in order; reports the result and every raster the
    /// backend was given.
    async fn run_recognition_retry(
        replies: Vec<ExtractedDocument>,
        max_iterations: Option<usize>,
    ) -> (ExtractedDocument, Vec<Vec<u8>>) {
        let (result, rasters, _) = run_recognition_retry_with(inverted_primary(), replies, max_iterations).await;
        (result, rasters)
    }

    async fn run_recognition_retry_with(
        primary: ExtractedDocument,
        replies: Vec<ExtractedDocument>,
        max_iterations: Option<usize>,
    ) -> (ExtractedDocument, Vec<Vec<u8>>, Vec<OcrConfig>) {
        let models = tempfile::tempdir().unwrap();
        for language in ["eng", "osd"] {
            std::fs::write(models.path().join(format!("{language}.traineddata")), b"fixture").unwrap();
        }
        let ocr = OcrConfig {
            tessdata_path: Some(models.path().into()),
            ..Default::default()
        };
        let config = ExtractionConfig {
            ocr: Some(ocr.clone()),
            security_limits: max_iterations.map(|max_iterations| SecurityLimits {
                max_iterations,
                ..Default::default()
            }),
            ..Default::default()
        };
        let retry = OrientationRetry::new(&config, OcrQualityThresholds::default());
        let scripted = Arc::new(ScriptedBackend {
            replies: parking_lot::Mutex::new(replies.into()),
            rasters: Default::default(),
            configs: Default::default(),
        });
        let backend: Arc<dyn OcrBackend> = scripted.clone();
        let result = retry
            .after_ocr(
                &backend,
                Arc::new(png_with_one_red_corner()),
                &ocr,
                Ok(primary),
                None,
                false,
            )
            .await
            .unwrap();
        let rasters = scripted.rasters.lock().clone();
        let configs = scripted.configs.lock().clone();
        (result, rasters, configs)
    }

    #[tokio::test]
    async fn page_down_evidence_rereads_the_half_turned_raster_and_returns_only_the_table() {
        let (result, rasters) = run_recognition_retry(vec![marked_recognition_read(), table_candidate()], None).await;
        assert_eq!(rasters.len(), 2, "one recognition-orientation read, one half-turn read");
        let decoded = |bytes: &[u8]| image::load_from_memory(bytes).unwrap().to_rgb8();
        assert_eq!(decoded(&rasters[0]).get_pixel(0, 0).0, [255, 0, 0]);
        let half_turned = decoded(&rasters[1]);
        assert_eq!(half_turned.get_pixel(7, 5).0, [255, 0, 0]);
        assert_eq!(half_turned.get_pixel(0, 0).0, [0, 0, 0]);
        assert_eq!(result.tables.len(), 1);
        assert!(
            !result
                .metadata
                .additional
                .contains_key(OCR_RECOGNITION_PAGE_DOWN_METADATA_KEY)
        );
    }

    #[tokio::test]
    async fn without_page_down_evidence_the_half_turn_is_never_read() {
        let mut unmarked = marked_recognition_read();
        unmarked
            .metadata
            .additional
            .remove(OCR_RECOGNITION_PAGE_DOWN_METADATA_KEY);
        let (result, rasters) = run_recognition_retry(vec![unmarked, table_candidate()], None).await;
        assert_eq!(rasters.len(), 1);
        assert!(result.tables.is_empty());
        assert_eq!(result.content, "zzq xkv");
    }

    #[tokio::test]
    async fn an_exhausted_attempt_budget_keeps_the_primary_without_a_half_turn_read() {
        let (result, rasters) =
            run_recognition_retry(vec![marked_recognition_read(), table_candidate()], Some(1)).await;
        assert_eq!(rasters.len(), 1);
        assert_eq!(result.content, "zzq xkv");
        assert!(
            !result
                .metadata
                .additional
                .contains_key(OCR_RECOGNITION_PAGE_DOWN_METADATA_KEY)
        );
    }

    #[tokio::test]
    async fn a_rejected_half_turn_keeps_the_primary_and_no_marker() {
        let mut broken = table_candidate();
        broken.tables[0].cells[0][1] = "31,299".into();
        let (result, rasters) = run_recognition_retry(vec![marked_recognition_read(), broken], None).await;
        assert_eq!(rasters.len(), 2);
        assert_eq!(result.content, "zzq xkv");
        assert!(
            !result
                .metadata
                .additional
                .contains_key(OCR_RECOGNITION_PAGE_DOWN_METADATA_KEY)
        );
    }

    #[tokio::test]
    async fn the_half_turn_read_uses_the_callers_segmentation_not_the_recognition_reads_psm() {
        let (result, rasters, configs) = run_recognition_retry_with(
            inverted_primary(),
            vec![marked_recognition_read(), table_candidate()],
            None,
        )
        .await;
        assert_eq!(rasters.len(), 2);
        assert_eq!(result.tables.len(), 1);
        assert_eq!(configs[0].tesseract_config.as_ref().and_then(|tess| tess.psm), Some(1));
        assert_eq!(
            configs[1].tesseract_config.as_ref().and_then(|tess| tess.psm),
            None,
            "an upright page is read with the caller's PSM, not PSM 1 orientation detection"
        );
    }

    #[tokio::test]
    async fn a_primary_that_fails_the_word_checks_is_replaced_only_on_page_down_evidence() {
        let unchecked = || {
            let mut primary = inverted_primary();
            primary.metadata.additional.remove("word_count");
            primary
        };
        let (result, rasters, _) =
            run_recognition_retry_with(unchecked(), vec![marked_recognition_read(), table_candidate()], None).await;
        assert_eq!(rasters.len(), 2);
        assert_eq!(result.tables.len(), 1);
        let mut unmarked = marked_recognition_read();
        unmarked
            .metadata
            .additional
            .remove(OCR_RECOGNITION_PAGE_DOWN_METADATA_KEY);
        let (result, rasters, _) =
            run_recognition_retry_with(unchecked(), vec![unmarked, table_candidate()], None).await;
        assert_eq!(rasters.len(), 1);
        assert_eq!(result.content, "zzq xkv");
    }

    #[tokio::test]
    async fn a_markdown_recognition_read_still_corroborates_the_half_turned_table() {
        let mut read = marked_recognition_read();
        read.content = "| Cash 100 | 31,290 |\n| --- | --- |\n| Rent 107 | 77,078 |\n| Fuel 114 | 71,433 |".into();
        let (result, _, _) = run_recognition_retry_with(inverted_primary(), vec![read, table_candidate()], None).await;
        assert_eq!(result.tables.len(), 1);
    }

    #[test]
    fn half_turn_budgets_the_encoded_input_and_the_png_it_writes() {
        let mut noise = image::RgbImage::new(64, 64);
        for (index, pixel) in noise.pixels_mut().enumerate() {
            let value = u8::try_from(index.wrapping_mul(2_654_435_761) >> 24 & 0xff).unwrap();
            *pixel = image::Rgb([value, value.wrapping_mul(7), value.wrapping_mul(13)]);
        }
        let mut raster = Vec::new();
        noise
            .write_to(&mut std::io::Cursor::new(&mut raster), image::ImageFormat::Png)
            .unwrap();
        let limits_at = |max_content_size| SecurityLimits {
            max_content_size,
            ..Default::default()
        };
        let decode_only = |size: usize| {
            crate::extraction::image_decode::decode_standard_rgb8_with_security_limits(&raster, &limits_at(size))
                .is_ok()
        };
        let (mut low, mut high) = (1usize, 10 * raster.len());
        while low < high {
            let middle = (low + high) / 2;
            if decode_only(middle) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        assert!(half_turn(&raster, &limits_at(low + raster.len())).is_some());
        assert!(
            half_turn(&raster, &limits_at(low)).is_none(),
            "the held input raster counts against the budget"
        );
        assert!(encode_png_within(&noise, usize::MAX).is_some());
        assert!(
            encode_png_within(&noise, 16).is_none(),
            "an encode above the limit is refused"
        );
    }
}
