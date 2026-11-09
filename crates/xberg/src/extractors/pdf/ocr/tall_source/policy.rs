//! Which configurations may use source-image strips, and the warning for a tall page that did not.

use super::MIN_STANDARD_SOURCE_DPI;
use crate::core::config::{ExtractionConfig, OcrConfig, OutputFormat};
use crate::types::{ExtractedDocument, ProcessingWarning};
use std::borrow::Cow;
use xberg_native_pdf::PdfDocument;

pub(in super::super) fn supports(config: &ExtractionConfig, ocr: &OcrConfig) -> bool {
    #[cfg(feature = "layout-detection")]
    if config.layout.is_some() {
        return false;
    }
    unsupported_reason(config, ocr).is_none()
}

/// Names the first configuration choice that keeps strip recovery from running. Callers
/// that report it have already excluded layout detection and stored scans.
pub(in super::super) fn unsupported_reason(config: &ExtractionConfig, ocr: &OcrConfig) -> Option<&'static str> {
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
        || ocr.auto_rotate == Some(true)
    {
        return Some("OCR preprocessing, element, pipeline or rotation options are set");
    }
    if ocr.effective_tesseract_language().as_slice() != ["eng"] {
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

/// Strips are joined as plain paragraphs, so a Markdown caller must be told no structure was rebuilt.
pub(in super::super) fn note_flat_structure(result: &mut ExtractedDocument, config: &ExtractionConfig, page: usize) {
    if config.output_format == OutputFormat::Markdown {
        result.processing_warnings.push(ProcessingWarning {
            source: Cow::Borrowed("ocr"),
            message: Cow::Owned(format!(
                "Page {} was recovered from source-image strips as plain text; Markdown structure such as headings \
                 and paragraph breaks was not rebuilt.",
                page + 1
            )),
        });
    }
}

/// Why a tall page kept the read made from its downscaled render.
pub(in super::super) enum Unrecovered {
    Config(&'static str),
    TablesOrFormulas,
    Uncertified,
    Rejected,
}

/// Tells the caller that a page read from a render shrunk below the standard scan density kept
/// that read. `raster_height` is the pixel height of the render the page was read from.
pub(in super::super) fn note_unrecovered(
    original: &mut ExtractedDocument,
    ocr: &OcrConfig,
    (document, page): (&PdfDocument, usize),
    raster_height: u32,
    why: Unrecovered,
) {
    let (_, page_height) = super::super::rendering::page_dimensions_pt(document, page);
    let render_dpi = f64::from(raster_height) * 72.0 / f64::from(page_height.max(1.0));
    if render_dpi >= MIN_STANDARD_SOURCE_DPI * 0.98 {
        return;
    }
    let reason = match why {
        Unrecovered::Config(reason) => reason,
        Unrecovered::TablesOrFormulas => "the downscaled read already contains tables or formulas",
        Unrecovered::Uncertified => {
            "the page is not one scan image in a supported encoding and density within the memory limit, \
             or a strip did not read as its source lines"
        }
        Unrecovered::Rejected => "the strips were read but were not better than the downscaled read",
    };
    if matches!(
        why,
        Unrecovered::Config(_) | Unrecovered::Uncertified | Unrecovered::Rejected
    ) {
        let thresholds = ocr.quality_thresholds.clone().unwrap_or_default();
        let noise = super::super::scoring::ocr_recognition_noise_decision(
            &original.content,
            &thresholds,
            crate::plugins::ConfidenceSemantics::Legibility { scale_max: 100.0 },
            super::super::mean_text_conf_of(&original.metadata.additional),
        );
        if !(noise.low_confidence || noise.fragmented_noise) {
            return;
        }
    }
    original.processing_warnings.push(ProcessingWarning {
        source: Cow::Borrowed("ocr"),
        message: Cow::Owned(format!(
            "Page {} was read from a render downscaled to {render_dpi:.0} DPI and its source-image strips \
             were not used: {reason}. The text may be incomplete.",
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
        let mut tesseract_language = ocr.clone();
        tesseract_language.tesseract_config = Some(crate::types::TesseractConfig {
            language: vec!["deu".into()],
            ..Default::default()
        });
        assert!(
            allowed(&ExtractionConfig::default(), &tesseract_language)
                .is_some_and(|reason| reason.contains("language"))
        );
        let mut preprocessed = ocr.clone();
        preprocessed.tesseract_config = Some(crate::types::TesseractConfig {
            preprocessing: Some(Default::default()),
            ..Default::default()
        });
        assert!(allowed(&ExtractionConfig::default(), &preprocessed).is_some());
    }
}
