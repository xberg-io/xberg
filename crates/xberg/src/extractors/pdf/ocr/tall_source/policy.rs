//! Which configurations may use source-image strips, and the warning for a tall page that did not.

use super::{MAX_TILE_SIDE, MIN_STANDARD_SOURCE_DPI};
use crate::core::config::{ExtractionConfig, OcrConfig, OutputFormat, PdfBackend, PdfConfig};
use crate::types::{ExtractedDocument, ProcessingWarning};
use std::borrow::Cow;
use xberg_native_pdf::PdfDocument;

pub(in super::super) fn supports(config: &ExtractionConfig, ocr: &OcrConfig) -> bool {
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
    if config.images.is_some()
        || config
            .pdf_options
            .as_ref()
            .is_some_and(|pdf| !annotation_only_options(pdf))
        || config.content_filter.is_some()
    {
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

// Annotation extraction is a separate projection. The source certificate still rejects
// actual page annotations, since their appearance may overlap the scan. ~keep
fn annotation_only_options(pdf: &PdfConfig) -> bool {
    matches!(
        pdf,
        PdfConfig {
            extract_annotations: true,
            extract_images: false,
            extract_tables: true,
            passwords: None,
            extract_metadata: true,
            hierarchy: None,
            top_margin_fraction: None,
            bottom_margin_fraction: None,
            allow_single_column_tables: false,
            ocr_inline_images: false,
            extract_form_fields: true,
            reading_order: false,
            backend: PdfBackend::Native,
        }
    )
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
    IndexedRgb,
    Rejected,
}

/// Reports when a tall page keeps a read below the standard scan density.
pub(in super::super) fn note_unrecovered(
    original: &mut ExtractedDocument,
    ocr: &OcrConfig,
    (document, page): (&PdfDocument, usize),
    raster_height: u32,
    why: Unrecovered,
) {
    let (_, page_height) = super::super::rendering::page_dimensions_pt(document, page);
    let render_dpi = f64::from(raster_height) * 72.0 / f64::from(page_height.max(1.0));
    let density_floor = MIN_STANDARD_SOURCE_DPI * 0.98;
    let clamp_dpi = if render_dpi < density_floor {
        None
    } else {
        if f64::from(page_height) <= f64::from(MAX_TILE_SIDE) * 72.0 / MIN_STANDARD_SOURCE_DPI {
            return;
        }
        let Some(ocr_height) = clamped_ocr_height(original, raster_height) else {
            return;
        };
        let ocr_dpi = f64::from(ocr_height) * 72.0 / f64::from(page_height.max(1.0));
        if ocr_dpi >= density_floor {
            return;
        }
        Some(ocr_dpi)
    };
    let reason = match why {
        Unrecovered::Config(reason) => reason,
        Unrecovered::TablesOrFormulas => "the downscaled read already contains tables or formulas",
        Unrecovered::IndexedRgb => "the general strip route declined the 8-bit Indexed RGB scan",
        Unrecovered::Uncertified => {
            "the page is not one scan image in a supported encoding and density within the memory limit, \
             or a strip did not read as its source lines"
        }
        Unrecovered::Rejected => "the strips were read but were not better than the downscaled read",
    };
    if matches!(
        why,
        Unrecovered::Config(_) | Unrecovered::Uncertified | Unrecovered::IndexedRgb | Unrecovered::Rejected
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
    let density = match clamp_dpi {
        Some(ocr_dpi) => format!("a {render_dpi:.0} DPI render that OCR preprocessing reduced to {ocr_dpi:.0} DPI"),
        None => format!("a render downscaled to {render_dpi:.0} DPI"),
    };
    original.processing_warnings.push(ProcessingWarning {
        source: Cow::Borrowed("ocr"),
        message: Cow::Owned(format!(
            "Page {} was read from {density} and its source-image strips were not used: {reason}. \
             The text may be incomplete.",
            page + 1
        )),
    });
}

fn clamped_ocr_height(original: &ExtractedDocument, raster_height: u32) -> Option<u32> {
    let resize = original.metadata.image_preprocessing.as_ref()?;
    if !resize.dimension_clamped
        || resize.resize_error.is_some()
        || resize.original_dimensions.height != raster_height as usize
    {
        return None;
    }
    let height = u32::try_from(resize.new_dimensions.as_ref()?.height).ok()?;
    (1..raster_height).contains(&height).then_some(height)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TALL_PAGE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/repros/tall-scan-strips/jpeg-tall.pdf"
    ));

    fn downscale_warning(raster_height: u32) -> Vec<String> {
        let document = PdfDocument::from_bytes(TALL_PAGE.to_vec()).unwrap();
        let (_, page_height) = super::super::super::rendering::page_dimensions_pt(&document, 0);
        assert_eq!(page_height, 5760.0, "the fixture page is 12000 px tall at 150 DPI");
        let mut original = ExtractedDocument::default();
        note_unrecovered(
            &mut original,
            &OcrConfig::default(),
            (&document, 0),
            raster_height,
            Unrecovered::TablesOrFormulas,
        );
        original
            .processing_warnings
            .into_iter()
            .map(|warning| warning.message.into_owned())
            .collect()
    }

    #[test]
    fn a_render_at_98_percent_of_the_standard_density_is_not_reported_and_one_pixel_less_is() {
        assert_eq!(downscale_warning(11_760), Vec::<String>::new(), "exactly 147 DPI");
        assert_eq!(downscale_warning(12_000), Vec::<String>::new(), "150 DPI");
        assert_eq!(
            downscale_warning(11_759),
            vec![
                "Page 1 was read from a render downscaled to 147 DPI and its source-image strips were not used: \
                 the downscaled read already contains tables or formulas. The text may be incomplete."
                    .to_string()
            ]
        );
    }

    #[test]
    fn reports_an_ocr_preprocessing_clamp_on_a_full_density_pdf_render() {
        let document = PdfDocument::from_bytes(TALL_PAGE.to_vec()).unwrap();
        let mut original = ExtractedDocument::default();
        original.metadata.additional.insert("mean_text_conf".into(), 1.into());
        original.metadata.image_preprocessing = Some(crate::types::ImagePreprocessingMetadata {
            original_dimensions: (1275, 12_000).into(),
            original_dpi: (150.0, 150.0).into(),
            target_dpi: 300,
            scale_factor: 4096.0 / 12_000.0,
            auto_adjusted: false,
            final_dpi: 51,
            new_dimensions: Some((435, 4096).into()),
            resample_method: "LANCZOS3".into(),
            dimension_clamped: true,
            calculated_dpi: None,
            skipped_resize: false,
            resize_error: None,
        });
        let ocr = OcrConfig::default();
        let source = (&document, 0);
        let why = || Unrecovered::Config("the output format is not plain text or Markdown");
        note_unrecovered(&mut original, &ocr, source, 12_000, why());
        let [warning] = original.processing_warnings.as_slice() else {
            panic!("expected exactly one warning, got {:?}", original.processing_warnings);
        };
        assert_eq!(warning.source, "ocr");
        assert_eq!(
            warning.message,
            "Page 1 was read from a 150 DPI render that OCR preprocessing reduced to 51 DPI and its \
             source-image strips were not used: the output format is not plain text or Markdown. The text may be incomplete."
        );

        original.processing_warnings.clear();
        original
            .metadata
            .image_preprocessing
            .as_mut()
            .unwrap()
            .original_dimensions
            .height = 11_999;
        note_unrecovered(&mut original, &ocr, source, 12_000, why());
        assert!(original.processing_warnings.is_empty(), "unmatched resize metadata");
    }

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
