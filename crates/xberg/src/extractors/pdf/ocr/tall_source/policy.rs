//! Which configurations may use source-image strips, and the warning for a tall page that did not.

use super::{MAX_TILE_SIDE, MIN_STANDARD_SOURCE_DPI, read_geometry};
use crate::core::config::{ExtractionConfig, OcrConfig, OutputFormat, PdfBackend, PdfConfig};
use crate::ocr_metadata_keys::{OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY};
use crate::plugins::OcrBackend;
use crate::types::{ExtractedDocument, ProcessingWarning};
use std::borrow::Cow;
use xberg_native_pdf::{PdfDocument, document::PageRotation};

/// [`unsupported_reason`] including the languages the resolved backend actually offers.
pub(in super::super) fn supports_for(config: &ExtractionConfig, ocr: &OcrConfig, backend: &dyn OcrBackend) -> bool {
    unsupported_reason_for(config, ocr, backend).is_none()
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
    unsupported_ocr_options(ocr)
}

/// [`unsupported_reason`] including the languages the resolved backend actually offers.
pub(in super::super) fn unsupported_reason_for(
    config: &ExtractionConfig,
    ocr: &OcrConfig,
    backend: &dyn OcrBackend,
) -> Option<&'static str> {
    unsupported_reason(config, ocr).or_else(|| unsupported_language(ocr, backend))
}

/// The picture route has no page geometry or PDF options; only OCR-facing choices decide it.
pub(in super::super) fn standalone_unsupported_reason(
    config: &ExtractionConfig,
    ocr: &OcrConfig,
    backend: &dyn OcrBackend,
) -> Option<&'static str> {
    if !matches!(config.output_format, OutputFormat::Plain | OutputFormat::Markdown) {
        return Some("the output format is not plain text or Markdown");
    }
    if config.result_format != crate::types::ResultFormat::Unified || config.include_document_structure {
        return Some("per-element results or document structure were requested");
    }
    if config.content_filter.is_some() {
        return Some("content-filter options are set");
    }
    unsupported_ocr_options(ocr).or_else(|| unsupported_language(ocr, backend))
}

fn unsupported_ocr_options(ocr: &OcrConfig) -> Option<&'static str> {
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
    if ocr.has_tuning_backend_options() {
        return Some("custom OCR backend options are set");
    }
    None
}

fn unsupported_language(ocr: &OcrConfig, backend: &dyn OcrBackend) -> Option<&'static str> {
    ocr.effective_tesseract_language()
        .iter()
        .any(|language| !backend.supports_language_for(ocr, language))
        .then_some("the OCR backend does not support every requested language")
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
            extract_vector_geometry: false,
            plan_regions: None,
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
    InsufficientReadEvidence,
    Uncertified,
    FourBitGrayFlate,
    BilevelBelowDensity(f64),
    IndexedRgb,
    Rejected,
}

/// Describes the retained low-density read when strip recovery was declined or ineligible.
pub(in super::super) fn note_unrecovered(
    original: &mut ExtractedDocument,
    ocr: &OcrConfig,
    (document, page): (&PdfDocument, usize),
    raster: (u32, u32),
    why: Unrecovered,
) {
    let (_, page_height) = super::super::rendering::page_dimensions_pt(document, page);
    let dpi = |height| f64::from(height) * 72.0 / f64::from(page_height.max(1.0));
    let render_dpi = dpi(raster.1);
    let density_floor = MIN_STANDARD_SOURCE_DPI * 0.98;
    let tall_unrotated = f64::from(page_height) > f64::from(MAX_TILE_SIDE) * 72.0 / MIN_STANDARD_SOURCE_DPI
        && matches!(
            document.get_page_rotation_status(page),
            Ok(PageRotation::Absent | PageRotation::Valid(0))
        );
    let reduced_dpi = tall_unrotated
        .then(|| reduced_ocr_height(original, raster))
        .flatten()
        .map(dpi)
        .filter(|&density| density < density_floor);
    if reduced_dpi.is_none() && render_dpi >= density_floor {
        return;
    }
    let density_reason;
    let reason: &str = match why {
        Unrecovered::Config(reason) => reason,
        Unrecovered::TablesOrFormulas => "the downscaled read already contains tables or formulas",
        Unrecovered::InsufficientReadEvidence => "the OCR read did not supply enough evidence to compare the strips",
        Unrecovered::FourBitGrayFlate => {
            "the painted scan uses 4-bit DeviceGray with Flate, which source-image strips do not support"
        }
        Unrecovered::BilevelBelowDensity(dpi) => {
            let dpi = dpi.floor();
            density_reason = format!(
                "the {dpi} DPI one-bit grayscale Flate scan is below the \
                 {MIN_STANDARD_SOURCE_DPI} DPI strip-recovery minimum"
            );
            &density_reason
        }
        Unrecovered::IndexedRgb => "the general strip route declined the 8-bit Indexed RGB scan",
        Unrecovered::Uncertified => {
            "the page is not one scan image in a supported encoding and density within the memory limit, \
             or a strip did not read as its source lines"
        }
        Unrecovered::Rejected => "the strips were read but were not better than the downscaled read",
    };
    let empty_read =
        original.content.trim().is_empty() || super::super::word_count_of(&original.metadata.additional) == Some(0);
    if !matches!(why, Unrecovered::TablesOrFormulas) && !empty_read {
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
    let density = match reduced_dpi {
        Some(ocr_dpi) => format!("a {render_dpi:.0} DPI render that OCR preprocessing reduced to {ocr_dpi:.0} DPI"),
        None => format!("a render downscaled to {render_dpi:.0} DPI"),
    };
    crate::core::diagnostics::push_warning(
        &mut original.processing_warnings,
        "ocr",
        format!(
            "Page {} was read from {density} and its source-image strips were not used: {reason}. \
             The text may be incomplete.",
            page + 1
        ),
    );
}

fn reduced_ocr_height(original: &ExtractedDocument, raster: (u32, u32)) -> Option<u32> {
    let resize = original.metadata.image_preprocessing.as_ref()?;
    let metadata = &original.metadata.additional;
    let dimension = |key| u32::try_from(metadata.get(key)?.as_u64()?).ok();
    let processed = (
        dimension(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY)?,
        dimension(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY)?,
    );
    // Require the selected read's dimensions and rotation, not just an intermediate resize. ~keep
    (resize.scale_factor < 1.0
        && !resize.skipped_resize
        && (1..=raster.0).contains(&processed.0)
        && (1..raster.1).contains(&processed.1)
        && read_geometry::select_read_width(original, raster) == Some(processed.0)
        && read_geometry::scale_only(raster, processed, Some(resize)))
    .then_some(processed.1)
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
            (1275, raster_height),
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

    fn clamped_pdf_read() -> ExtractedDocument {
        let mut original = ExtractedDocument::default();
        original.metadata.additional.insert("mean_text_conf".into(), 1.into());
        original
            .metadata
            .additional
            .insert(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(), 435.into());
        original
            .metadata
            .additional
            .insert(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(), 4096.into());
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
        original
    }

    #[test]
    fn reports_an_ocr_preprocessing_clamp_on_a_full_density_pdf_render() {
        let document = PdfDocument::from_bytes(TALL_PAGE.to_vec()).unwrap();
        let mut original = clamped_pdf_read();
        let ocr = OcrConfig::default();
        let source = (&document, 0);
        let why = || Unrecovered::Config("the output format is not plain text or Markdown");
        note_unrecovered(&mut original, &ocr, source, (1275, 12_000), why());
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
        let check_abstention = |mut read, raster| {
            note_unrecovered(&mut read, &ocr, source, raster, why());
            assert!(read.processing_warnings.is_empty());
        };
        check_abstention(original.clone(), (1274, 12_000));
        check_abstention(original.clone(), (1275, 11_999));

        let mut different_read = original.clone();
        different_read
            .metadata
            .additional
            .insert(OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(), 4000.into());
        check_abstention(different_read, (1275, 12_000));

        let mut different_width = original.clone();
        different_width
            .metadata
            .additional
            .insert(OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(), 436.into());
        check_abstention(different_width, (1275, 12_000));

        let mut inconsistent_scale = original.clone();
        inconsistent_scale
            .metadata
            .image_preprocessing
            .as_mut()
            .unwrap()
            .scale_factor = 0.4;
        check_abstention(inconsistent_scale, (1275, 12_000));

        let mut rotated_read = original.clone();
        rotated_read.metadata.additional.insert(
            crate::ocr_metadata_keys::OCR_AUTO_ROTATED_METADATA_KEY.into(),
            true.into(),
        );
        check_abstention(rotated_read, (1275, 12_000));

        let mut pdf = lopdf::Document::load_mem(TALL_PAGE).unwrap();
        let page_id = pdf.get_pages()[&1];
        pdf.get_object_mut(page_id)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Rotate", 180);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        let rotated = PdfDocument::from_bytes(bytes).unwrap();
        assert_eq!(rotated.get_page_rotation_status(0).unwrap(), PageRotation::Valid(180));
        note_unrecovered(&mut original, &ocr, (&rotated, 0), (1275, 12_000), why());
        assert!(original.processing_warnings.is_empty());
    }

    #[test]
    fn reports_the_actual_density_of_a_budgeted_pdf_read() {
        let document = PdfDocument::from_bytes(TALL_PAGE.to_vec()).unwrap();
        for (raster, processed, render_dpi, read_dpi, dimension_clamped) in [
            ((1275_u32, 12_000_u32), (297_u32, 2800_u32), 150, 35, false),
            ((850, 8000), (435, 4096), 100, 51, true),
        ] {
            let mut original = clamped_pdf_read();
            let resize = original.metadata.image_preprocessing.as_mut().unwrap();
            resize.original_dimensions = (raster.0 as usize, raster.1 as usize).into();
            resize.new_dimensions = Some((processed.0 as usize, processed.1 as usize).into());
            resize.scale_factor = f64::from(processed.1) / f64::from(raster.1);
            resize.dimension_clamped = dimension_clamped;
            original.metadata.additional.extend([
                (OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(), processed.0.into()),
                (OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY.into(), processed.1.into()),
            ]);
            note_unrecovered(
                &mut original,
                &OcrConfig::default(),
                (&document, 0),
                raster,
                Unrecovered::TablesOrFormulas,
            );
            let [warning] = original.processing_warnings.as_slice() else {
                panic!("expected one warning, got {:?}", original.processing_warnings);
            };
            assert_eq!(
                warning.message.as_ref(),
                format!(
                    "Page 1 was read from a {render_dpi} DPI render that OCR preprocessing reduced to {read_dpi} DPI \
                     and its source-image strips were not used: the downscaled read already contains tables or formulas. \
                     The text may be incomplete."
                )
            );
        }
    }

    #[tokio::test]
    async fn strip_recovery_early_returns_report_only_the_proven_density_once() {
        use super::super::{RecognitionQuality, try_replace};
        use crate::ocr::tesseract_backend::TesseractBackend;
        use std::sync::Arc;

        let document = PdfDocument::from_bytes(TALL_PAGE.to_vec()).unwrap();
        let backend: Arc<dyn crate::plugins::OcrBackend> = Arc::new(TesseractBackend::new());
        let config = ExtractionConfig::default();
        let ocr = OcrConfig::default();

        let mut empty = clamped_pdf_read();
        empty.metadata.additional.remove("mean_text_conf");
        empty.metadata.additional.insert("word_count".into(), 0.into());
        assert!(RecognitionQuality::from_result(&empty).is_none());

        let mut unknown_width = clamped_pdf_read();
        unknown_width.metadata.format = Some(crate::types::FormatMetadata::Ocr(Default::default()));
        unknown_width.content = "Illegible".into();
        unknown_width.metadata.additional.extend([
            ("word_count".into(), 1.into()),
            ("low_conf_word_count".into(), 1.into()),
            (super::super::DICTIONARY_INVALID.into(), 1.0.into()),
            (OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY.into(), 700.into()),
        ]);
        assert!(RecognitionQuality::from_result(&unknown_width).is_some());
        assert_eq!(read_geometry::select_read_width(&unknown_width, (850, 8000)), None);

        for (mut read, (width, height), density) in [
            (
                empty,
                (1275, 12_000),
                "a 150 DPI render that OCR preprocessing reduced to 51 DPI",
            ),
            (unknown_width, (850, 8000), "a render downscaled to 100 DPI"),
        ] {
            let raster = (0, Arc::new(Vec::new()), width, height);
            assert!(
                !try_replace(&mut read, &backend, &config, &ocr, (&document, 0), &raster)
                    .await
                    .unwrap()
            );
            let [warning] = read.processing_warnings.as_slice() else {
                panic!("expected one warning, got {:?}", read.processing_warnings);
            };
            assert_eq!(
                warning.message.as_ref(),
                format!(
                    "Page 1 was read from {density} and its source-image strips were not used: \
                     the OCR read did not supply enough evidence to compare the strips. The text may be incomplete."
                )
            );
        }
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
        // A language is refused only when the resolved backend does not offer it. ~keep
        let german = OcrConfig {
            language: vec!["deu".into()],
            ..ocr.clone()
        };
        assert_eq!(allowed(&ExtractionConfig::default(), &german), None);
        let mut tesseract_language = ocr.clone();
        tesseract_language.tesseract_config = Some(crate::types::TesseractConfig {
            language: vec!["deu".into()],
            ..Default::default()
        });
        assert_eq!(allowed(&ExtractionConfig::default(), &tesseract_language), None);
        let mut preprocessed = ocr.clone();
        preprocessed.tesseract_config = Some(crate::types::TesseractConfig {
            preprocessing: Some(Default::default()),
            ..Default::default()
        });
        assert!(allowed(&ExtractionConfig::default(), &preprocessed).is_some());
    }

    struct Languages(&'static [&'static str]);

    impl crate::plugins::Plugin for Languages {
        fn name(&self) -> &str {
            "languages"
        }

        fn version(&self) -> String {
            "0".to_owned()
        }
    }

    #[async_trait::async_trait]
    impl crate::plugins::OcrBackend for Languages {
        async fn process_image(&self, _: &[u8], _: &OcrConfig) -> crate::Result<ExtractedDocument> {
            Ok(ExtractedDocument::default())
        }

        fn supports_language(&self, language: &str) -> bool {
            self.0.contains(&language)
        }

        fn backend_type(&self) -> crate::plugins::OcrBackendType {
            crate::plugins::OcrBackendType::Custom
        }
    }

    #[test]
    fn a_language_is_admitted_only_when_the_backend_offers_it() {
        let german = OcrConfig {
            language: vec!["deu".into()],
            ..Default::default()
        };
        let english_only = Languages(&["eng"]);
        let bilingual = Languages(&["eng", "deu"]);
        assert_eq!(
            unsupported_reason_for(&ExtractionConfig::default(), &german, &english_only),
            Some("the OCR backend does not support every requested language")
        );
        assert_eq!(
            unsupported_reason_for(&ExtractionConfig::default(), &german, &bilingual),
            None
        );
        assert!(supports_for(&ExtractionConfig::default(), &german, &bilingual));
        assert!(!supports_for(&ExtractionConfig::default(), &german, &english_only));

        let mut preprocessed = german.clone();
        preprocessed.tesseract_config = Some(crate::types::TesseractConfig {
            preprocessing: Some(Default::default()),
            ..Default::default()
        });
        assert_eq!(
            unsupported_reason_for(&ExtractionConfig::default(), &preprocessed, &bilingual),
            Some("OCR preprocessing, element, pipeline or rotation options are set")
        );

        let html = ExtractionConfig {
            output_format: OutputFormat::Html,
            ..Default::default()
        };
        assert_eq!(
            standalone_unsupported_reason(&html, &german, &bilingual),
            Some("the output format is not plain text or Markdown")
        );
        assert_eq!(
            standalone_unsupported_reason(&ExtractionConfig::default(), &german, &english_only),
            Some("the OCR backend does not support every requested language")
        );
        assert_eq!(
            standalone_unsupported_reason(&ExtractionConfig::default(), &german, &bilingual),
            None
        );
    }
}
