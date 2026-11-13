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
    let mut result = crate::extractors::image::process_whole_image_ocr_with_sparse_retry(
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
    .await?;
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
