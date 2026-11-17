use super::super::document::{LineBox, processed_ocr_layout_dimensions};
use super::super::native_word_coverage::{is_iterator_word, native_box, ocr_word_in_page_points, upright_media_box};
use crate::core::config::{ExtractionConfig, ImageExtractionConfig, OcrConfig, OutputFormat};
use crate::extractors::pdf::unmapped_glyphs::{LOST_GLYPH_MARKER, is_lost_glyph};
use crate::plugins::OcrBackend;
use crate::types::ExtractedDocument;
use std::sync::Arc;
use xberg_native_pdf::PdfDocument;

// At 120 DPI, 10 pt text has a roughly 17 pixel em before OCR. ~keep
const READ_DPI: i32 = 120;
const MIN_WORDS: usize = 100;
const MAX_GLYPH_LOSS: usize = 16;
const MIN_WORDS_PER_LOST_GLYPH: usize = 20;
const MIN_FONT_SIZE_PT: f32 = 10.0;
const MAX_FONT_SIZE_RATIO: f32 = 1.5;
const MAX_READ_ELEMENTS: usize = 4096;
// Four source pixels at 120 DPI leave room for the marked glyph itself. ~keep
const MIN_MARKED_WIDTH_PT: f32 = 2.4;

#[derive(Clone)]
pub(crate) struct MarkedWords {
    media: xberg_native_pdf::geometry::Rect,
    words: Vec<(String, LineBox)>,
}

pub(super) fn eligible_pages(
    config: &ExtractionConfig,
    pdf: &PdfDocument,
    layers: Option<&ahash::AHashMap<u32, String>>,
    backend: Option<&Arc<dyn OcrBackend>>,
) -> ahash::AHashMap<usize, MarkedWords> {
    let (Some(layers), Some(backend), Some(ocr)) = (layers, backend, config.ocr.as_ref()) else {
        return Default::default();
    };
    if config.images.is_some()
        || margin_filtering_active(config)
        || config.output_format != OutputFormat::Plain
        || config.include_document_structure
        || config.result_format == crate::types::ResultFormat::ElementBased
        || ocr
            .tesseract_config
            .as_ref()
            .is_some_and(|settings| settings.preprocessing.is_some())
        || ocr.effective_tesseract_language().as_slice() != ["eng"]
        || !crate::plugins::registry::get_ocr_backend_registry()
            .read()
            .is_builtin_tesseract(backend)
    {
        return Default::default();
    }
    layers
        .iter()
        .filter(|&(page, layer)| {
            super::super::document::native_preservation_is_automatic(config, *page) && sparse_loss(layer)
        })
        .filter_map(|(&page, layer)| {
            let index = page.checked_sub(1)? as usize;
            marked_words(pdf, index, layer).map(|words| (index, words))
        })
        .collect()
}

#[cfg(paddle_ocr)]
pub(super) fn implicit_pipeline_pages(
    config: &ExtractionConfig,
    ocr: &OcrConfig,
    pipeline: Option<&crate::core::config::OcrPipelineConfig>,
    pdf: &PdfDocument,
    layers: Option<&ahash::AHashMap<u32, String>>,
    allowed: bool,
) -> ahash::AHashMap<usize, MarkedWords> {
    if !allowed
        || config.force_ocr
        || config.force_ocr_pages.is_some()
        || ocr.auto_rotate.is_some()
        || !matches!(config.ocr_strategy, crate::core::config::OcrStrategy::Auto)
        || !super::uses_default_ocr_pipeline(ocr)
        || !pipeline.is_some_and(|pipeline| {
            matches!(pipeline.stages.as_slice(), [first, second]
                if first.backend == "tesseract" && second.backend == "paddleocr")
        })
    {
        return Default::default();
    }
    #[cfg(all(test, feature = "bundle-tessdata-eng"))]
    if config
        .security_limits
        .as_ref()
        .is_some_and(|limits| limits.max_archive_size == IMPLICIT_NORMAL_CASE)
    {
        return Default::default();
    }
    crate::plugins::ensure_ocr_backends_initialized();
    let backend = crate::plugins::registry::get_ocr_backend_registry()
        .read()
        .get(&ocr.backend)
        .ok();
    eligible_pages(config, pdf, layers, backend.as_ref())
}

#[cfg(paddle_ocr)]
struct PipelineTrial {
    pdf: Arc<PdfDocument>,
    index: usize,
    config: ExtractionConfig,
    pipeline: crate::core::config::OcrPipelineConfig,
    hints: super::PageOcrHints,
    words: MarkedWords,
}

#[cfg(paddle_ocr)]
type Dpi = crate::pdf::render::OcrRasterDpi;

#[cfg(paddle_ocr)]
type Warnings = Vec<crate::types::ProcessingWarning>;

#[cfg(paddle_ocr)]
pub(super) struct AcceptedTrial {
    pub output: Box<super::OcrPipelineOutput>,
    pub dimensions: (u32, u32),
    pub dpi: Dpi,
}

#[cfg(paddle_ocr)]
pub(super) async fn run_batch(
    pdf: &Arc<PdfDocument>,
    config: &ExtractionConfig,
    pipeline: Option<&crate::core::config::OcrPipelineConfig>,
    indices: &[usize],
    eligible: &ahash::AHashMap<usize, MarkedWords>,
    hints: impl Fn(usize) -> super::PageOcrHints,
) -> crate::Result<std::collections::BTreeMap<usize, AcceptedTrial>> {
    let mut accepted = std::collections::BTreeMap::new();
    let Some(pipeline) = pipeline.filter(|_| !eligible.is_empty()) else {
        return Ok(accepted);
    };
    let mut tasks = tokio::task::JoinSet::new();
    for &index in indices {
        let Some(words) = eligible.get(&index) else {
            continue;
        };
        let trial = PipelineTrial {
            pdf: Arc::clone(pdf),
            index,
            config: config.clone(),
            pipeline: pipeline.clone(),
            hints: hints(index),
            words: words.clone(),
        };
        let task = async move { (index, trial.run().await) };
        #[cfg(test)]
        let task = crate::plugins::registry::inherit_test_ocr_backend(task);
        tasks.spawn(crate::core::extractor::cacheability::inherit(task));
    }
    let mut abandon_at = None;
    loop {
        match super::next_page_task(&mut tasks, config.cancel_token.as_ref(), &mut abandon_at).await {
            super::NextPageTask::Finished(Ok((index, result))) => {
                if let Some(result) = result? {
                    accepted.insert(index, result);
                }
            }
            super::NextPageTask::Finished(Err(error)) => {
                tracing::debug!(panicked = error.is_panic(), "Sparse glyph OCR task did not finish");
            }
            super::NextPageTask::Cancelled => return Err(crate::XbergError::Cancelled),
            super::NextPageTask::Drained | super::NextPageTask::Abandoned => return Ok(accepted),
        }
    }
}

#[cfg(paddle_ocr)]
pub(super) fn declined() -> super::FailedOcrRun {
    crate::XbergError::Ocr {
        message: "Sparse glyph OCR trial did not recover reliable text".to_owned(),
        source: None,
    }
    .into()
}

#[cfg(paddle_ocr)]
impl PipelineTrial {
    async fn render(&self) -> crate::Result<(image::DynamicImage, Dpi, Warnings)> {
        self.check_stop()?;
        let pdf = Arc::clone(&self.pdf);
        let limits = self.config.security_limits.clone().unwrap_or_default();
        let idx = self.index;
        let (((_, image), dpi), warnings) = super::super::super::spawn_pdf_blocking(move || {
            crate::pdf::render::capture_render_warnings(|| {
                super::super::rendering::render_one_selected_page(&pdf, 0, idx, &limits, Some(&images_config()), None)
            })
        })
        .await?;
        self.check_stop()?;
        Ok((image, dpi, warnings))
    }

    fn check_stop(&self) -> crate::Result<()> {
        super::super::super::ensure_pdf_not_cancelled(&self.config)?;
        if self
            .config
            .cancel_token
            .as_ref()
            .is_some_and(|token| token.time_limit_reached())
        {
            return Err(declined().into());
        }
        Ok(())
    }

    async fn read(&self, image: &image::DynamicImage) -> super::OcrRunResult<super::OcrPipelineOutput> {
        let mut hints = self.hints.clone();
        hints.sparse_glyph_trial = Some(self.words.clone());
        hints.source_dpi = super::rendered_page_source_dpi(&self.pdf, self.index, image.width());
        hints.number_source = None;
        hints.probe_reads = None;
        let (_, height_pt) = super::page_dimensions_pt(&self.pdf, self.index);
        let points_per_pixel = (image.height() > 0).then(|| height_pt / image.height() as f32);
        let detached = super::DetachedOcrPage {
            document: &self.pdf,
            stored_scan: None,
            orientation_retry: None,
        };
        Box::pin(super::run_ocr_pipeline_for_page(
            None,
            Some(std::slice::from_ref(image)),
            #[cfg(feature = "layout-detection")]
            None,
            &self.config,
            &self.pipeline,
            None,
            0,
            true,
            points_per_pixel,
            Some(hints),
            self.index,
            Some(detached),
        ))
        .await
    }

    async fn run(self) -> crate::Result<Option<AcceptedTrial>> {
        if let Ok((image, dpi, warnings)) = self.render().await
            // Failed and warning-bearing trials are retried by the ordinary renderer, so the
            // result only reports warnings from the selected normal render in batch order.
            && warnings.is_empty()
            && super::rendered_page_source_dpi(&self.pdf, self.index, image.width())
                .is_some_and(|source| (source - f64::from(READ_DPI)).abs() < 1.0)
        {
            let dimensions = (image.width(), image.height());
            match self.read(&image).await {
                Ok(result)
                    if result
                        .9
                        .get(&((self.index + 1) as u32))
                        .is_some_and(|metadata| matches_trial_raster(metadata, dimensions)) =>
                {
                    super::super::super::ensure_pdf_not_cancelled(&self.config)?;
                    return Ok(Some(AcceptedTrial {
                        output: Box::new(result),
                        dimensions,
                        dpi,
                    }));
                }
                Err(error) if matches!(error.error, crate::XbergError::Cancelled) => {
                    return Err(crate::XbergError::Cancelled);
                }
                Ok(_) | Err(_) => tracing::debug!(page = self.index + 1, "Retrying sparse glyph OCR at normal density"),
            }
        }
        super::super::super::ensure_pdf_not_cancelled(&self.config)?;
        Ok(None)
    }
}

fn margin_filtering_active(config: &ExtractionConfig) -> bool {
    let margins = crate::pdf::native::text::PageMarginFractions::from_extraction_config(Some(config));
    margins.top != 0.0 || margins.bottom != 0.0
}

fn sparse_loss(layer: &str) -> bool {
    let lost = layer.matches(LOST_GLYPH_MARKER).take(MAX_GLYPH_LOSS + 1).count();
    lost > 0
        && lost <= MAX_GLYPH_LOSS
        && !layer
            .chars()
            .any(|character| character != LOST_GLYPH_MARKER && is_lost_glyph(character))
        && layer.split_whitespace().count() >= MIN_WORDS.max(lost * MIN_WORDS_PER_LOST_GLYPH)
}

fn marked_words(pdf: &PdfDocument, index: usize, layer: &str) -> Option<MarkedWords> {
    let media = upright_media_box(pdf, index)?;
    let page = pdf.get_page(index).ok()?;
    let resources = pdf.get_page_resources(index).ok()?;
    if page.as_dict()?.contains_key("Annots")
        || ["XObject", "Pattern", "Shading"]
            .iter()
            .any(|key| resources.as_dict().is_none_or(|dict| dict.contains_key(*key)))
    {
        return None;
    }
    let lines = pdf.extract_text_lines(index).ok()?;
    let words = lines.iter().flat_map(|line| &line.words);
    let sizes = words.clone().flat_map(|word| &word.chars).map(|glyph| glyph.font_size);
    let min = sizes.clone().fold(f32::INFINITY, f32::min);
    let max = sizes.fold(f32::NEG_INFINITY, f32::max);
    if max > min * MAX_FONT_SIZE_RATIO {
        return None;
    }
    if !words
        .clone()
        .all(|word| !word.chars.is_empty() && word.chars.iter().all(readable_glyph_geometry))
    {
        return None;
    }
    let words: Vec<_> = words
        .filter(|word| word.text.contains(LOST_GLYPH_MARKER))
        .map(|word| (word.text.clone(), native_box(word.bbox)))
        .collect();
    let observed: usize = words
        .iter()
        .map(|(word, _)| word.matches(LOST_GLYPH_MARKER).count())
        .sum();
    (observed > 0 && observed == layer.matches(LOST_GLYPH_MARKER).count()).then_some(MarkedWords { media, words })
}

fn readable_glyph_geometry(glyph: &xberg_native_pdf::layout::TextChar) -> bool {
    glyph.font_size >= MIN_FONT_SIZE_PT
        && glyph.rotation_degrees.abs() < 1.0
        && (glyph.char != LOST_GLYPH_MARKER || glyph.bbox.width >= MIN_MARKED_WIDTH_PT)
        && !["thin", "hairline", "light", "condensed", "narrow"]
            .iter()
            .any(|weight| glyph.font_name.to_ascii_lowercase().contains(weight))
}

impl MarkedWords {
    pub(super) fn accounted_for(&self, read: &ExtractedDocument) -> bool {
        let (Some(raster), Some(elements)) = (
            processed_ocr_layout_dimensions(&read.metadata),
            read.ocr_elements.as_ref(),
        ) else {
            return false;
        };
        if elements.len() > MAX_READ_ELEMENTS {
            return false;
        }
        self.words.iter().all(|(native, bounds)| {
            let mut matches = elements.iter().filter(|word| {
                word.level == crate::types::OcrElementLevel::Word
                    && is_iterator_word(word)
                    && ocr_word_in_page_points(word, raster, self.media).is_some_and(|placed| bounds.owns(&placed))
                    && matches_marked_word(native, &word.text)
            });
            matches.next().is_some() && matches.next().is_none()
        })
    }
}

pub(super) fn rejected_by_page_quality(
    read: &ExtractedDocument,
    thresholds: &crate::core::config::OcrQualityThresholds,
    semantics: crate::plugins::ConfidenceSemantics,
) -> bool {
    if read.content.trim().is_empty() {
        return true;
    }
    if !thresholds.discard_suspected_ocr_noise {
        return false;
    }
    use super::super::scoring::{is_dictionary_invalid_noise, mean_text_conf_of, ocr_recognition_noise_decision};
    let metadata = &read.metadata.additional;
    let dictionary_ratio = metadata
        .get(crate::ocr_metadata_keys::OCR_TESSERACT_DICT_INVALID_WORD_RATIO_METADATA_KEY)
        .and_then(serde_json::Value::as_f64);
    ocr_recognition_noise_decision(&read.content, thresholds, semantics, mean_text_conf_of(metadata)).suspected()
        || is_dictionary_invalid_noise(dictionary_ratio, thresholds)
}

#[cfg(paddle_ocr)]
pub(super) fn accepted_pipeline_read(
    words: &MarkedWords,
    read: &ExtractedDocument,
    dimensions: (u32, u32),
    thresholds: &crate::core::config::OcrQualityThresholds,
    semantics: crate::plugins::ConfidenceSemantics,
) -> bool {
    super::super::document::resolved_ocr_correction_degrees(&read.metadata).unwrap_or(0) == 0
        && read
            .metadata
            .image_preprocessing
            .as_ref()
            .is_some_and(|metadata| matches_trial_raster(metadata, dimensions))
        && words.accounted_for(read)
        && !rejected_by_page_quality(read, thresholds, semantics)
}

#[cfg(paddle_ocr)]
fn matches_trial_raster(metadata: &crate::types::ImagePreprocessingMetadata, dimensions: (u32, u32)) -> bool {
    metadata.target_dpi == READ_DPI
        && (metadata.original_dimensions.width, metadata.original_dimensions.height)
            == (dimensions.0 as usize, dimensions.1 as usize)
        && metadata.new_dimensions.is_none()
}

fn matches_marked_word(native: &str, recognized: &str) -> bool {
    let trim = |character: char| !character.is_alphanumeric() && character != LOST_GLYPH_MARKER;
    let native = native.trim_matches(trim).to_lowercase();
    let recognized = recognized.trim_matches(trim).to_lowercase();
    native.chars().count() == recognized.chars().count()
        && native
            .chars()
            .zip(recognized.chars())
            .all(|(expected, actual)| expected == actual || (expected == LOST_GLYPH_MARKER && actual.is_alphanumeric()))
}

pub(super) async fn retry(
    pdf: Arc<PdfDocument>,
    index: usize,
    backend: &Arc<dyn OcrBackend>,
    (original, mut effective): (&OcrConfig, OcrConfig),
    limits: crate::extractors::security::SecurityLimits,
    cancel: Option<&crate::cancellation::CancellationToken>,
) -> crate::Result<(ExtractedDocument, u32, u32, crate::pdf::render::OcrRasterDpi)> {
    if cancel.is_some_and(|token| token.stop_requested()) {
        return Err(crate::XbergError::Cancelled);
    }
    let render_doc = Arc::clone(&pdf);
    let (((_, bytes, width, height), raster_dpi), warnings) = super::super::super::spawn_pdf_blocking(move || {
        crate::pdf::render::capture_render_warnings(|| {
            super::super::rendering::render_one_full_pdf_ocr_page_with_dpi(&render_doc, &[], index, &limits, None, None)
        })
    })
    .await?;
    if cancel.is_some_and(|token| token.stop_requested()) {
        return Err(crate::XbergError::Cancelled);
    }
    let dpi = super::rendered_page_source_dpi(&pdf, index, width);
    if let Some(dpi) = dpi.and_then(serde_json::Number::from_f64)
        && let Some(options) = effective
            .backend_options
            .as_mut()
            .and_then(serde_json::Value::as_object_mut)
    {
        options.insert(crate::core::config::ocr::SOURCE_DPI_BACKEND_OPTION.into(), dpi.into());
    }
    let (original, effective) =
        crate::extractors::image::reserve_retained_ocr_raster(backend, (original.clone(), effective), bytes.len())?;
    let result = crate::extractors::image::process_whole_image_ocr_with_sparse_retry(
        backend,
        bytes,
        &original,
        &effective,
        crate::extractors::image::SparseRetryPolicy {
            whole_page_raster: false,
            allow_standard_retry: false,
        },
        cancel,
    )
    .await;
    #[cfg(all(test, feature = "bundle-tessdata-eng"))]
    let result = test_attempt(result, &original, width, height, None);
    let mut result = result?;
    crate::core::diagnostics::dedup_extend_warnings(&mut result.processing_warnings, warnings);
    Ok((result, width, height, raster_dpi))
}

pub(super) fn images_config() -> ImageExtractionConfig {
    ImageExtractionConfig {
        target_dpi: READ_DPI,
        auto_adjust_dpi: false,
        ..Default::default()
    }
}

pub(super) fn ocr_config(mut config: OcrConfig, eligible: bool) -> OcrConfig {
    if eligible {
        config
            .tesseract_config
            .get_or_insert_with(Default::default)
            .preprocessing = Some(crate::types::ImagePreprocessingConfig {
            target_dpi: READ_DPI,
            deskew: false,
            binarization_method: "none".into(),
            ..Default::default()
        });
    }
    config
}

#[cfg(test)]
mod tests {
    use super::matches_marked_word;

    #[test]
    fn a_marked_word_requires_a_letter_or_digit_at_each_missing_position() {
        assert!(matches_marked_word("�uiet", "quiet"));
        assert!(matches_marked_word("�ue�e!", "queue!"));
        assert!(!matches_marked_word("�uiet", "uiet"));
        assert!(!matches_marked_word("�ue�e", "queueing"));
        assert!(!matches_marked_word("�uiet", "?uiet"));
        assert!(!matches_marked_word("�uiet", "quite"));
    }
}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
type Attempt = (bool, u32, u32, Option<i32>);

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
const QUALITY_CASE: usize = 0x3150_5101;
#[cfg(all(test, feature = "bundle-tessdata-eng"))]
const MISSING_WORD_CASE: usize = 0x3150_5102;
#[cfg(all(test, feature = "bundle-tessdata-eng"))]
const MARGIN_CASE: usize = 0x3150_5103;
#[cfg(all(test, feature = "bundle-tessdata-eng"))]
const NORMAL_CONTROL_CASE: usize = 0x3150_5104;
#[cfg(all(test, feature = "bundle-tessdata-eng"))]
const IMPLICIT_MISSING_WORD_CASE: usize = 0x3238_5101;
#[cfg(all(test, feature = "bundle-tessdata-eng", paddle_ocr))]
const IMPLICIT_NORMAL_CASE: usize = 0x3238_5102;
#[cfg(all(test, feature = "bundle-tessdata-eng"))]
const IMPLICIT_ROTATED_CASE: usize = 0x3238_5103;
#[cfg(all(test, feature = "bundle-tessdata-eng"))]
const IMPLICIT_PANIC_CASE: usize = 0x3238_5104;
#[cfg(all(test, feature = "bundle-tessdata-eng"))]
const IMPLICIT_OVERRIDE_CASE: usize = 0x3238_5105;

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
pub(super) fn test_uses_single_backend(config: &OcrConfig) -> bool {
    test_case(config).is_some_and(|case| !implicit_pipeline_case(case))
        || config
            .security_limits
            .as_ref()
            .is_some_and(|limits| limits.max_archive_size == NORMAL_CONTROL_CASE)
}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
static ATTEMPTS: std::sync::Mutex<std::collections::BTreeMap<usize, Vec<Attempt>>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
fn test_case(config: &OcrConfig) -> Option<usize> {
    config
        .security_limits
        .as_ref()
        .map(|limits| limits.max_archive_size)
        .filter(|case| matches!(*case, QUALITY_CASE | MISSING_WORD_CASE | MARGIN_CASE) || implicit_pipeline_case(*case))
}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
fn implicit_pipeline_case(case: usize) -> bool {
    cfg!(paddle_ocr)
        && matches!(
            case,
            IMPLICIT_MISSING_WORD_CASE | IMPLICIT_ROTATED_CASE | IMPLICIT_PANIC_CASE | IMPLICIT_OVERRIDE_CASE
        )
}

#[cfg(all(test, feature = "bundle-tessdata-eng", paddle_ocr))]
pub(super) fn test_implicit_attempt(
    result: ExtractedDocument,
    config: &OcrConfig,
    width: u32,
    height: u32,
    words: Option<&MarkedWords>,
) -> ExtractedDocument {
    if let Some(case) = test_case(config).filter(|case| implicit_pipeline_case(*case)) {
        let mut result = test_attempt(Ok(result), config, width, height, words).unwrap();
        if let Some(words) = words {
            control_pipeline_attempt(case, words, &mut result, (width, height), config);
        }
        return result;
    }
    result
}

#[cfg(all(test, feature = "bundle-tessdata-eng", paddle_ocr))]
static ROTATION_PROOF: std::sync::Mutex<Option<(bool, bool, bool)>> = std::sync::Mutex::new(None);

#[cfg(all(test, feature = "bundle-tessdata-eng", paddle_ocr))]
fn control_pipeline_attempt(
    case: usize,
    words: &MarkedWords,
    result: &mut ExtractedDocument,
    dimensions: (u32, u32),
    config: &OcrConfig,
) {
    if case == IMPLICIT_PANIC_CASE {
        panic!("intentional panic after the recorded sparse trial");
    }
    if case != IMPLICIT_ROTATED_CASE {
        return;
    }
    use crate::ocr_metadata_keys::{
        OCR_AUTO_ROTATED_METADATA_KEY, OCR_BACKEND_ROTATION_UNDONE_METADATA_KEY, OCR_ORIENTATION_DEGREES_METADATA_KEY,
    };
    let thresholds = config.effective_thresholds();
    let semantics = crate::plugins::ConfidenceSemantics::None;
    let accepted_before_rotation = accepted_pipeline_read(words, result, dimensions, &thresholds, semantics);
    result
        .metadata
        .additional
        .remove(OCR_BACKEND_ROTATION_UNDONE_METADATA_KEY);
    result
        .metadata
        .additional
        .insert(OCR_AUTO_ROTATED_METADATA_KEY.into(), serde_json::json!(true));
    result
        .metadata
        .additional
        .insert(OCR_ORIENTATION_DEGREES_METADATA_KEY.into(), serde_json::json!(180));
    let accepted_after_rotation = accepted_pipeline_read(words, result, dimensions, &thresholds, semantics);
    *ROTATION_PROOF.lock().unwrap() = Some((
        accepted_before_rotation,
        words.accounted_for(result),
        accepted_after_rotation,
    ));
}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
pub(super) fn test_attempt(
    mut result: crate::Result<ExtractedDocument>,
    config: &OcrConfig,
    width: u32,
    height: u32,
    marked_words: Option<&MarkedWords>,
) -> crate::Result<ExtractedDocument> {
    if let Some(case) = test_case(config) {
        let read = result.as_mut().expect("sparse glyph test OCR read failed");
        let retry = marked_words.is_none() && case != MARGIN_CASE;
        if let Some(words) = marked_words {
            assert!(
                words.accounted_for(read),
                "unmodified read must account for the marked words"
            );
            if matches!(case, MISSING_WORD_CASE | IMPLICIT_MISSING_WORD_CASE) {
                remove_marked_word(read, words);
                assert!(!words.accounted_for(read));
            }
        }
        let target = read
            .metadata
            .image_preprocessing
            .as_ref()
            .map(|metadata| metadata.target_dpi);
        ATTEMPTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(case)
            .or_default()
            .push((retry, width, height, target));
        // Exercise each trigger independently of Tesseract model confidence drift. ~keep
        if !implicit_pipeline_case(case) {
            let confidence = if case == QUALITY_CASE && !retry { 0 } else { 100 };
            read.metadata
                .additional
                .insert("mean_text_conf".into(), serde_json::json!(confidence));
        }
    }
    result
}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
fn remove_marked_word(read: &mut ExtractedDocument, words: &MarkedWords) {
    let raster = processed_ocr_layout_dimensions(&read.metadata).unwrap();
    let (native, bounds) = &words.words[0];
    let elements = read.ocr_elements.as_mut().unwrap();
    let index = elements
        .iter()
        .position(|word| {
            word.level == crate::types::OcrElementLevel::Word
                && is_iterator_word(word)
                && ocr_word_in_page_points(word, raster, words.media).is_some_and(|placed| bounds.owns(&placed))
                && matches_marked_word(native, &word.text)
        })
        .expect("iterator word at the first marked position");
    elements.remove(index);
}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
const SPARSE_GLYPH_TEST_STACK_BYTES: usize = 32 * 1024 * 1024;

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
#[test]
fn page_quality_rejection_retries_sparse_glyphs_with_normal_raster_and_preprocessing() {
    super::super::run_on_ocr_test_stack(SPARSE_GLYPH_TEST_STACK_BYTES, || {
        assert_sparse_glyph_attempts(QUALITY_CASE)
    });
}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
#[test]
fn missing_marked_word_retries_sparse_glyphs_with_normal_raster_and_preprocessing() {
    super::super::run_on_ocr_test_stack(SPARSE_GLYPH_TEST_STACK_BYTES, || {
        assert_sparse_glyph_attempts(MISSING_WORD_CASE)
    });
}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
#[test]
fn active_pdf_margin_uses_one_normal_density_read() {
    super::super::run_on_ocr_test_stack(SPARSE_GLYPH_TEST_STACK_BYTES, || {
        assert_sparse_glyph_attempts(MARGIN_CASE)
    });
}

#[cfg(all(test, feature = "bundle-tessdata-eng", paddle_ocr))]
#[test]
fn implicit_pipeline_retries_missing_marked_words_at_normal_density() {
    super::super::run_on_ocr_test_stack(SPARSE_GLYPH_TEST_STACK_BYTES, || {
        assert_sparse_glyph_attempts(IMPLICIT_MISSING_WORD_CASE)
    });
}

#[cfg(all(test, feature = "bundle-tessdata-eng", paddle_ocr))]
#[test]
fn implicit_pipeline_preserves_normal_render_on_rotated_or_failed_trials() {
    for case in [IMPLICIT_ROTATED_CASE, IMPLICIT_PANIC_CASE, IMPLICIT_OVERRIDE_CASE] {
        super::super::run_on_ocr_test_stack(SPARSE_GLYPH_TEST_STACK_BYTES, move || {
            assert_sparse_glyph_attempts(case)
        });
    }
    assert_eq!(
        ROTATION_PROOF.lock().unwrap().take(),
        Some((true, true, false)),
        "an untransformed read must qualify, and adding same-size 180-degree correction must be the deciding change"
    );
}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
async fn assert_sparse_glyph_attempts(case: usize) {
    const FIXTURE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pdf/separation-overprint-2817/"
    );
    let pdf = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pdf/separation-overprint-2817/folio-black.pdf"
    ));
    let expected = std::fs::read_to_string(format!("{FIXTURE}sepop.ocr.txt")).unwrap();
    let expected = expected
        .split_whitespace()
        .skip(3)
        .chain(["Page", "12"])
        .collect::<Vec<_>>();
    let models = tempfile::tempdir().unwrap();
    std::fs::write(
        models.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().unwrap(),
    )
    .unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        security_limits: Some(crate::extractors::security::SecurityLimits {
            max_archive_size: case,
            ..Default::default()
        }),
        pages: Some(crate::core::config::PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        pdf_options: (case == MARGIN_CASE).then(|| crate::core::config::PdfConfig {
            top_margin_fraction: Some(0.01),
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            tessdata_path: Some(models.path().to_path_buf()),
            auto_rotate: (case == IMPLICIT_OVERRIDE_CASE).then_some(false),
            quality_thresholds: Some(crate::core::config::OcrQualityThresholds {
                discard_suspected_ocr_noise: case != MISSING_WORD_CASE && !implicit_pipeline_case(case),
                min_ocr_mean_confidence: 50.0,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = crate::core::extractor::extract_bytes(pdf, "application/pdf", &config)
        .await
        .unwrap();
    let attempts = ATTEMPTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&case);
    let attempts = attempts.unwrap_or_else(|| {
        panic!(
            "expected an OCR attempt for case {case:#x}, got {:?} with warnings {:?}",
            result.extraction_method, result.processing_warnings
        )
    });
    let normal = normal_density_control(pdf, &config).await;
    assert_retry_equivalent(&result, &normal, &attempts, case, expected);
}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
fn assert_retry_equivalent(
    result: &ExtractedDocument,
    normal: &ExtractedDocument,
    attempts: &[Attempt],
    case: usize,
    expected: Vec<&str>,
) {
    let normal_page = &normal.pages.as_ref().unwrap()[0];
    let normal_target = normal_page
        .image_preprocessing
        .as_ref()
        .map(|metadata| metadata.target_dpi);
    if case == MARGIN_CASE {
        assert_eq!(attempts, [(false, 1275, 1650, normal_target)]);
    } else if case == IMPLICIT_OVERRIDE_CASE {
        assert_eq!(attempts, [(true, 1275, 1650, normal_target)]);
    } else {
        assert_eq!(
            attempts,
            [(false, 1020, 1320, Some(120)), (true, 1275, 1650, normal_target)]
        );
    }
    let [page] = result.pages.as_deref().unwrap() else {
        panic!("expected one page");
    };
    for content in [&page.content, &result.content] {
        assert_eq!(content.split_whitespace().collect::<Vec<_>>(), expected);
    }
    assert_eq!(
        serde_json::to_value(&page.image_preprocessing).unwrap(),
        serde_json::to_value(&normal_page.image_preprocessing).unwrap(),
    );
    if implicit_pipeline_case(case) {
        assert_eq!(
            serde_json::to_value(&page.ocr_confidence).unwrap(),
            serde_json::to_value(&normal_page.ocr_confidence).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&result.processing_warnings).unwrap(),
            serde_json::to_value(&normal.processing_warnings).unwrap()
        );
    } else {
        assert_eq!(page.ocr_confidence.as_ref().unwrap().score, Some(1.0));
    }
    assert!(
        result
            .processing_warnings
            .iter()
            .all(|warning| !warning.message.contains("suspected OCR recognition noise"))
    );
}

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
async fn normal_density_control(pdf: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let mut normal_config = config.clone();
    #[cfg(paddle_ocr)]
    if config
        .security_limits
        .as_ref()
        .is_some_and(|limits| implicit_pipeline_case(limits.max_archive_size))
    {
        normal_config.security_limits.as_mut().unwrap().max_archive_size = IMPLICIT_NORMAL_CASE;
        return crate::core::extractor::extract_bytes(pdf, "application/pdf", &normal_config)
            .await
            .unwrap();
    }
    normal_config.security_limits.as_mut().unwrap().max_archive_size = NORMAL_CONTROL_CASE;
    normal_config.images = Some(ImageExtractionConfig {
        target_dpi: crate::image::dpi::DEFAULT_PDF_RENDER_DPI,
        auto_adjust_dpi: false,
        extract_images: false,
        ..Default::default()
    });
    crate::core::extractor::extract_bytes(pdf, "application/pdf", &normal_config)
        .await
        .unwrap()
}
