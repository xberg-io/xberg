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
pub(super) struct MarkedWords {
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
static ATTEMPTS: std::sync::Mutex<std::collections::BTreeMap<usize, Vec<Attempt>>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

#[cfg(all(test, feature = "bundle-tessdata-eng"))]
pub(super) fn test_attempt(
    mut result: crate::Result<ExtractedDocument>,
    config: &OcrConfig,
    width: u32,
    height: u32,
    marked_words: Option<&MarkedWords>,
) -> crate::Result<ExtractedDocument> {
    let case = config.security_limits.as_ref().map(|limits| limits.max_archive_size);
    if let Some(case @ (QUALITY_CASE | MISSING_WORD_CASE | MARGIN_CASE)) = case
        && let Ok(read) = &mut result
    {
        let retry = marked_words.is_none() && case != MARGIN_CASE;
        if let Some(words) = marked_words {
            assert!(
                words.accounted_for(read),
                "unmodified read must account for the marked words"
            );
            if case == MISSING_WORD_CASE {
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
            .unwrap()
            .entry(case)
            .or_default()
            .push((retry, width, height, target));
        // Exercise each trigger independently of Tesseract model confidence drift. ~keep
        let confidence = if case == QUALITY_CASE && !retry { 0 } else { 100 };
        read.metadata
            .additional
            .insert("mean_text_conf".into(), serde_json::json!(confidence));
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
            quality_thresholds: Some(crate::core::config::OcrQualityThresholds {
                discard_suspected_ocr_noise: case != MISSING_WORD_CASE,
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
        .unwrap()
        .remove(&case)
        .expect("observed at least one OCR attempt");
    let normal = normal_density_control(pdf, &config).await;
    let normal_page = &normal.pages.as_ref().unwrap()[0];
    let normal_target = normal_page
        .image_preprocessing
        .as_ref()
        .map(|metadata| metadata.target_dpi);
    if case == MARGIN_CASE {
        assert_eq!(attempts, [(false, 1275, 1650, normal_target)]);
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
    assert_eq!(page.ocr_confidence.as_ref().unwrap().score, Some(1.0));
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
    normal_config.security_limits.as_mut().unwrap().max_archive_size = QUALITY_CASE + 16;
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
