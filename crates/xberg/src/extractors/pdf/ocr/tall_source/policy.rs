//! Which configurations may use source-image strips, and the warning for a tall page that did not.

use super::MAX_TILE_SIDE;
use super::MIN_STANDARD_SOURCE_DPI;
use crate::core::config::{ExtractionConfig, OcrConfig, OutputFormat};
use crate::types::{ExtractedDocument, ProcessingWarning};
use std::borrow::Cow;
use xberg_native_pdf::PdfDocument;

pub(in super::super) fn supports(config: &ExtractionConfig, ocr: &OcrConfig) -> bool {
    unsupported_reason(config, ocr).is_none()
}

/// Names the first configuration choice that keeps strip recovery from running.
pub(in super::super) fn unsupported_reason(config: &ExtractionConfig, ocr: &OcrConfig) -> Option<&'static str> {
    #[cfg(feature = "layout-detection")]
    if config.layout.is_some() {
        return Some("layout detection is enabled");
    }
    if !matches!(config.output_format, OutputFormat::Plain | OutputFormat::Markdown) {
        return Some("the output format is not plain text or Markdown");
    }
    if config.result_format != crate::types::ResultFormat::Unified || config.include_document_structure {
        return Some("per-element results or document structure were requested");
    }
    if config.images.is_some() || config.pdf_options.is_some() || config.content_filter.is_some() {
        return Some("image, PDF or content-filter options are set");
    }
    if ocr
        .tesseract_config
        .as_ref()
        .is_some_and(|tesseract| tesseract.preprocessing.is_some())
        || ocr.element_config.is_some()
        || ocr.output_format.is_some()
        || ocr.pipeline.is_some()
        || ocr.auto_rotate
    {
        return Some("OCR preprocessing, element, pipeline or rotation options are set");
    }
    if ocr.language != ["eng"] {
        return Some("the OCR language is not English only");
    }
    if !ocr.backend_options.as_ref().is_none_or(|options| {
        options
            .as_object()
            .is_some_and(|fields| fields.keys().all(|key| key == "use_cache"))
    }) {
        return Some("custom OCR backend options are set");
    }
    None
}

/// Tells the caller that a page too tall for the OCR raster kept its downscaled read.
pub(in super::super) fn note_unrecovered(
    original: &mut ExtractedDocument,
    ocr: &OcrConfig,
    (document, page): (&PdfDocument, usize),
    reason: &str,
) {
    let thresholds = ocr.quality_thresholds.clone().unwrap_or_default();
    let (_, page_height) = super::super::rendering::page_dimensions_pt(document, page);
    let noise = super::super::scoring::ocr_recognition_noise_decision(
        &original.content,
        &thresholds,
        crate::plugins::ConfidenceSemantics::Legibility { scale_max: 100.0 },
        super::super::mean_text_conf_of(&original.metadata.additional),
    );
    if page_height <= MAX_TILE_SIDE as f32 * 72.0 / MIN_STANDARD_SOURCE_DPI as f32
        || !(noise.low_confidence || noise.fragmented_noise)
    {
        return;
    }
    original.processing_warnings.push(ProcessingWarning {
        source: Cow::Borrowed("ocr"),
        message: Cow::Owned(format!(
            "Page {} is too tall for one OCR raster and its source-image strips were not used: {reason}. \
             The text is from the downscaled page and may be incomplete.",
            page + 1
        )),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_runs_for_plain_and_markdown_and_names_what_blocks_it() {
        let ocr = OcrConfig {
            language: vec!["eng".into()],
            ..Default::default()
        };
        let allowed = |config: &ExtractionConfig, ocr: &OcrConfig| unsupported_reason(config, ocr);
        for format in [OutputFormat::Plain, OutputFormat::Markdown] {
            let config = ExtractionConfig {
                output_format: format,
                ..Default::default()
            };
            assert_eq!(allowed(&config, &ocr), None);
            let mut page_mode = ocr.clone();
            page_mode.tesseract_config = Some(Default::default());
            assert_eq!(allowed(&config, &page_mode), None);
        }
        let html = ExtractionConfig {
            output_format: OutputFormat::Html,
            ..Default::default()
        };
        assert!(allowed(&html, &ocr).is_some_and(|reason| reason.contains("output format")));
        let structured = ExtractionConfig {
            include_document_structure: true,
            ..Default::default()
        };
        assert!(allowed(&structured, &ocr).is_some_and(|reason| reason.contains("document structure")));
        let german = OcrConfig {
            language: vec!["deu".into()],
            ..ocr.clone()
        };
        assert!(allowed(&ExtractionConfig::default(), &german).is_some_and(|reason| reason.contains("language")));
        let mut preprocessed = ocr.clone();
        preprocessed.tesseract_config = Some(crate::types::TesseractConfig {
            preprocessing: Some(Default::default()),
            ..Default::default()
        });
        assert!(allowed(&ExtractionConfig::default(), &preprocessed).is_some());
    }
}
