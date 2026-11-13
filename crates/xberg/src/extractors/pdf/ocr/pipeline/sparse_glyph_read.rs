use crate::core::config::{ExtractionConfig, ImageExtractionConfig, OcrConfig, OutputFormat};
use crate::extractors::pdf::unmapped_glyphs::{LOST_GLYPH_MARKER, is_lost_glyph};
use crate::plugins::OcrBackend;
use std::sync::Arc;
use xberg_native_pdf::PdfDocument;

// At 120 DPI, 10 pt text has a roughly 17 pixel em before OCR. ~keep
const READ_DPI: i32 = 120;
const MIN_WORDS: usize = 100;
const MAX_GLYPH_LOSS: usize = 16;
const MIN_WORDS_PER_LOST_GLYPH: usize = 20;
const MIN_FONT_SIZE_PT: f32 = 10.0;

pub(super) fn eligible_pages(
    config: &ExtractionConfig,
    pdf: &PdfDocument,
    layers: Option<&ahash::AHashMap<u32, String>>,
    backend: Option<&Arc<dyn OcrBackend>>,
) -> ahash::AHashSet<usize> {
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
        .filter_map(|(&page, _)| {
            let index = page.checked_sub(1)? as usize;
            ordinary_text_page(pdf, index).then_some(index)
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

fn ordinary_text_page(pdf: &PdfDocument, index: usize) -> bool {
    if !matches!(pdf.get_page_rotation(index), Ok(0)) {
        return false;
    }
    let (Ok(page), Ok(resources)) = (pdf.get_page(index), pdf.get_page_resources(index)) else {
        return false;
    };
    page.as_dict()
        .is_some_and(|dict| !dict.contains_key("Annots") && !dict.contains_key("CropBox"))
        && resources.as_dict().is_some_and(|dict| {
            !dict.contains_key("XObject") && !dict.contains_key("Pattern") && !dict.contains_key("Shading")
        })
        && pdf.extract_text_lines(index).is_ok_and(|lines| {
            !lines.is_empty()
                && lines
                    .iter()
                    .flat_map(|line| &line.words)
                    .all(|word| word.avg_font_size >= MIN_FONT_SIZE_PT)
        })
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
