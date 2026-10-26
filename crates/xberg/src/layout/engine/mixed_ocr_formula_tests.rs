//! Formula recognition on the pages the mixed OCR route replaces, with and without the markdown
//! layout inputs.
//!
//! Lives under `engine` because the stub model is installed in the private engine pool, which only
//! this module can construct an engine for; the OCR backend and the formula recognizer are stubs.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};
use crate::core::config::layout::{FormulaModel, LayoutStrategy};
use crate::core::config::{ExtractionConfig, LayoutDetectionConfig, OcrConfig, OcrStrategy, OutputFormat};
use crate::layout::{BBox, LayoutClass, LayoutDetection};
use crate::plugins::{InternalDocumentExtractor, OcrBackend, OcrBackendType, Plugin};
use crate::types::ExtractionMethod;
use crate::types::internal::{ElementKind, InternalDocument};

// Both thresholds key a stub engine apart from any other pooled engine, so no other test can
// lease one; the second keys the stub that reports no region.
const STUB_CONFIDENCE_THRESHOLD: f32 = 0.41;
const NO_REGION_CONFIDENCE_THRESHOLD: f32 = 0.43;
const STUB_LATEX: &str = "\\stub{x^2}";
const OCR_BACKEND: &str = "mixed-ocr-formula-stub";
const OCR_TEXT: &str = "Automatic OCR replacement text for the scanned second page of this fixture.";
const REJECTED_OCR_BACKEND: &str = "mixed-ocr-formula-rejected-stub";

static RECOGNITIONS: AtomicUsize = AtomicUsize::new(0);

/// Reports the dark pixels of a page as one formula region, so the region follows the page
/// content through any change of raster frame; or reports nothing.
struct FormulaRegionModel {
    regions: bool,
}

impl LayoutModel for FormulaRegionModel {
    fn detect(&mut self, img: &RgbImage) -> Result<Vec<LayoutDetection>, LayoutError> {
        if !self.regions {
            return Ok(Vec::new());
        }
        let dark = img
            .enumerate_pixels()
            .filter(|(_, _, pixel)| pixel.0.iter().map(|&channel| u32::from(channel)).sum::<u32>() < 384)
            .map(|(x, y, _)| (x as f32, y as f32));
        let Some(bbox) = dark.fold(None, |bbox: Option<BBox>, (x, y)| {
            Some(bbox.map_or(
                BBox {
                    x1: x,
                    y1: y,
                    x2: x + 1.0,
                    y2: y + 1.0,
                },
                |bbox| BBox {
                    x1: bbox.x1.min(x),
                    y1: bbox.y1.min(y),
                    x2: bbox.x2.max(x + 1.0),
                    y2: bbox.y2.max(y + 1.0),
                },
            ))
        }) else {
            return Ok(Vec::new());
        };
        Ok(vec![LayoutDetection {
            class_name: LayoutClass::Formula,
            confidence: 0.9,
            bbox,
        }])
    }

    fn detect_with_threshold(&mut self, img: &RgbImage, _threshold: f32) -> Result<Vec<LayoutDetection>, LayoutError> {
        self.detect(img)
    }

    fn name(&self) -> &str {
        "formula-region-stub"
    }
}

fn layout_config(formula_model: Option<FormulaModel>, confidence_threshold: f32) -> LayoutDetectionConfig {
    LayoutDetectionConfig {
        strategy: LayoutStrategy::Always,
        confidence_threshold: Some(confidence_threshold),
        acceleration: Some(AccelerationConfig {
            provider: ExecutionProviderType::Cpu,
            device_id: 0,
        }),
        formula_model,
        ..Default::default()
    }
}

/// Pools a stub engine matching `layout`, so every layout pass leases it instead of loading RT-DETR.
fn install_stub_engine(layout: &LayoutDetectionConfig) {
    let regions = layout.confidence_threshold != Some(NO_REGION_CONFIDENCE_THRESHOLD);
    let engine = LayoutEngine {
        model: Box::new(FormulaRegionModel { regions }),
        config: crate::layout::config_from_extraction(layout),
        thread_budget: crate::core::config::concurrency::resolve_thread_budget(None).max(1),
    };
    let lease = crate::layout::CACHED_ENGINE
        .take_or_create_matching(|_| false, || Ok::<_, LayoutError>(engine))
        .expect("the stub engine is pooled");
    drop(lease);
}

fn stub_latex(_crop: &RgbImage) -> Option<String> {
    RECOGNITIONS.fetch_add(1, Ordering::SeqCst);
    Some(STUB_LATEX.to_string())
}

/// Installs the stub recognizer and the stub OCR backends for one test, and removes them on drop.
/// The stub layout engines stay pooled.
struct Stubs;

impl Stubs {
    fn install() -> Self {
        // Bound first, so a failed registration below still clears whatever was installed.
        let stubs = Self;
        RECOGNITIONS.store(0, Ordering::SeqCst);
        *crate::formula_recognition::STUB_RECOGNIZER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(stub_latex);
        for (name, text) in [(OCR_BACKEND, OCR_TEXT), (REJECTED_OCR_BACKEND, "a")] {
            crate::plugins::register_ocr_backend(Arc::new(StubOcrBackend { name, text }))
                .expect("register the stub OCR backend");
        }
        stubs
    }
}

impl Drop for Stubs {
    fn drop(&mut self) {
        *crate::formula_recognition::STUB_RECOGNIZER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        for name in [OCR_BACKEND, REJECTED_OCR_BACKEND] {
            let _ = crate::plugins::unregister_ocr_backend(name);
        }
    }
}

struct StubOcrBackend {
    name: &'static str,
    text: &'static str,
}

impl Plugin for StubOcrBackend {
    fn name(&self) -> &str {
        self.name
    }

    fn version(&self) -> String {
        "1.0.0".to_string()
    }

    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for StubOcrBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supports_language(&self, _lang: &str) -> bool {
        true
    }

    async fn process_image(
        &self,
        _image_bytes: &[u8],
        _config: &OcrConfig,
    ) -> crate::Result<crate::types::ExtractedDocument> {
        Ok(crate::types::ExtractedDocument {
            content: self.text.to_string(),
            mime_type: std::borrow::Cow::Borrowed("text/plain"),
            ..Default::default()
        })
    }
}

/// Page 1 is native prose; page 2 is a full-page scan with no text layer, dark in its top-left
/// quarter only, turned by `rotate`.
fn native_then_scanned_pdf(rotate: i64) -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let mut prose = String::from("BT /F1 10 Tf 72 720 Td 14 TL\n");
    for line in 0..45 {
        prose.push_str(&format!(
            "(Line {line} of steady single column prose with enough characters to pass the floor.) Tj T*\n"
        ));
    }
    prose.push_str("ET");
    let prose_id = document.add_object(Stream::new(dictionary! {}, prose.into_bytes()));
    let native_page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => prose_id,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });
    let image_id = document.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 2,
            "Height" => 2,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 8,
        },
        vec![0, 255, 255, 255],
    ));
    let scan_id = document.add_object(Stream::new(dictionary! {}, b"q 612 0 0 792 0 0 cm /Scan Do Q".to_vec()));
    let scanned_page_id = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => scan_id,
        "Resources" => dictionary! { "XObject" => dictionary! { "Scan" => image_id } },
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Rotate" => rotate,
    });
    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![native_page_id.into(), scanned_page_id.into()],
            "Count" => 2,
        }),
    );
    let catalog_id = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("fixture PDF must serialize");
    bytes
}

#[derive(Clone, Copy, Debug)]
enum Route {
    Forced,
    Scanned,
    GateFallback,
}

fn config(route: Route, layout: LayoutDetectionConfig, ocr_backend: &str) -> ExtractionConfig {
    let mut config = ExtractionConfig {
        use_cache: false,
        layout: Some(layout),
        ocr: Some(OcrConfig {
            backend: ocr_backend.to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    };
    match route {
        Route::Forced => config.force_ocr_pages = Some(vec![2]),
        Route::Scanned => {
            // Only scan detection can select a page: no quality gate, no near-empty fallback.
            config.ocr_strategy = OcrStrategy::ScannedPages { min_confidence: 0.7 };
            config.ocr_scanned_page_quality_gate = Some(false);
            config.ocr_near_empty_fallback = Some(false);
        }
        Route::GateFallback => {}
    }
    config
}

async fn extract(pdf: &[u8], config: &ExtractionConfig) -> InternalDocument {
    crate::extractors::PdfExtractor::new()
        .extract_content(pdf, "application/pdf", config)
        .await
        .expect("extraction must succeed")
}

fn extraction_method(doc: &InternalDocument) -> Option<ExtractionMethod> {
    crate::extraction::derive::derive_extraction_result(doc.clone(), false, OutputFormat::Plain).extraction_method
}

/// Pages carrying the stub's LaTeX, as a side-channel formula or a formula element.
fn recognised_pages(doc: &InternalDocument) -> BTreeSet<u32> {
    let formulas = doc
        .formulas
        .iter()
        .filter(|formula| formula.latex == STUB_LATEX)
        .filter_map(|formula| formula.page);
    let elements = doc
        .elements
        .iter()
        .filter(|element| matches!(element.kind, ElementKind::Formula) && element.text == STUB_LATEX)
        .filter_map(|element| element.page);
    formulas.chain(elements).collect()
}

/// The stub formula on page 2 must cover the scan's dark top-left quarter in PDF points.
fn assert_formula_on_dark_quarter(doc: &InternalDocument) {
    let bbox = doc
        .formulas
        .iter()
        .find(|formula| formula.page == Some(2) && formula.latex == STUB_LATEX)
        .and_then(|formula| formula.bbox)
        .expect("page 2 carries the recognised formula with a bbox");
    let expected = [0.0, 396.0, 306.0, 792.0];
    let actual = [bbox.x0, bbox.y0, bbox.x1, bbox.y1];
    assert!(
        actual.iter().zip(expected).all(|(a, e)| (a - e).abs() < 12.0),
        "formula bbox {actual:?} must sit on the dark quarter {expected:?}"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn mixed_ocr_routes_recognise_formulas_on_replaced_pages_without_markdown_layout() {
    let _stubs = Stubs::install();
    let layout = layout_config(Some(FormulaModel::LatexOcr), STUB_CONFIDENCE_THRESHOLD);
    install_stub_engine(&layout);
    let pdf = native_then_scanned_pdf(0);

    for route in [Route::Forced, Route::Scanned, Route::GateFallback] {
        RECOGNITIONS.store(0, Ordering::SeqCst);
        let doc = extract(&pdf, &config(route, layout.clone(), OCR_BACKEND)).await;

        assert_eq!(
            extraction_method(&doc),
            Some(ExtractionMethod::Mixed),
            "{route:?} must take the mixed route"
        );
        assert_eq!(
            recognised_pages(&doc),
            BTreeSet::from([2]),
            "{route:?}: only the OCR'd page gets LaTeX"
        );
        assert_eq!(
            RECOGNITIONS.load(Ordering::SeqCst),
            1,
            "{route:?}: one region, one recognition"
        );
        assert_formula_on_dark_quarter(&doc);
        if let Route::Scanned = route {
            let Some(crate::types::FormatMetadata::Pdf(pdf_metadata)) = &doc.metadata.format else {
                panic!("a PDF carries PDF metadata");
            };
            assert_eq!(
                pdf_metadata.scanned_pages,
                Some(vec![2]),
                "scan detection selects page 2"
            );
        }
    }
}

#[tokio::test]
#[serial_test::serial]
async fn mixed_ocr_formula_recognition_runs_once_per_region_with_markdown_layout() {
    let _stubs = Stubs::install();
    let layout = layout_config(Some(FormulaModel::LatexOcr), STUB_CONFIDENCE_THRESHOLD);
    install_stub_engine(&layout);
    let mut config = config(Route::Forced, layout, OCR_BACKEND);
    config.use_layout_for_markdown = true;

    let doc = extract(&native_then_scanned_pdf(0), &config).await;

    assert_eq!(recognised_pages(&doc), BTreeSet::from([1, 2]));
    assert_eq!(
        RECOGNITIONS.load(Ordering::SeqCst),
        2,
        "the markdown layout inputs cover both pages, so the mixed route's must not be read again"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn mixed_ocr_formula_recognition_survives_dropped_markdown_layout_on_rotated_page() {
    let _stubs = Stubs::install();
    let layout = layout_config(Some(FormulaModel::LatexOcr), STUB_CONFIDENCE_THRESHOLD);
    install_stub_engine(&layout);
    let mut config = config(Route::Forced, layout, OCR_BACKEND);
    config.use_layout_for_markdown = true;

    let doc = extract(&native_then_scanned_pdf(90), &config).await;

    assert_eq!(recognised_pages(&doc), BTreeSet::from([2]));
    assert_eq!(RECOGNITIONS.load(Ordering::SeqCst), 1);
    assert_formula_on_dark_quarter(&doc);
}

#[tokio::test]
#[serial_test::serial]
async fn mixed_ocr_formula_recognition_skips_pages_ocr_did_not_replace() {
    let _stubs = Stubs::install();
    let layout = layout_config(Some(FormulaModel::LatexOcr), STUB_CONFIDENCE_THRESHOLD);
    install_stub_engine(&layout);
    let pdf = native_then_scanned_pdf(0);

    let rejected = extract(&pdf, &config(Route::Forced, layout.clone(), REJECTED_OCR_BACKEND)).await;
    assert_eq!(extraction_method(&rejected), Some(ExtractionMethod::Native));
    assert!(recognised_pages(&rejected).is_empty());

    let mut no_pages = config(Route::Forced, layout, OCR_BACKEND);
    no_pages.force_ocr_pages = Some(vec![99]);
    let native = extract(&pdf, &no_pages).await;
    assert!(recognised_pages(&native).is_empty());

    let no_regions = layout_config(Some(FormulaModel::LatexOcr), NO_REGION_CONFIDENCE_THRESHOLD);
    install_stub_engine(&no_regions);
    let without_regions = extract(&pdf, &config(Route::Forced, no_regions, OCR_BACKEND)).await;
    assert_eq!(extraction_method(&without_regions), Some(ExtractionMethod::Mixed));
    assert!(recognised_pages(&without_regions).is_empty());
    assert_eq!(RECOGNITIONS.load(Ordering::SeqCst), 0);
}

#[tokio::test]
#[serial_test::serial]
async fn mixed_ocr_without_formula_model_recognises_nothing() {
    let _stubs = Stubs::install();
    let layout = layout_config(None, STUB_CONFIDENCE_THRESHOLD);
    install_stub_engine(&layout);

    let doc = extract(&native_then_scanned_pdf(0), &config(Route::Forced, layout, OCR_BACKEND)).await;

    assert_eq!(extraction_method(&doc), Some(ExtractionMethod::Mixed));
    assert!(recognised_pages(&doc).is_empty());
    assert_eq!(RECOGNITIONS.load(Ordering::SeqCst), 0);
}
