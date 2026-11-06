use super::*;
use crate::core::config::{
    OcrConfig,
    layout::{LayoutDetectionConfig, TableModel},
};
use crate::extractors::security::SecurityLimits;
use crate::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const FIRST: &str = "prepared-raster-budget-first";
const SECOND: &str = "prepared-raster-budget-second";
const READABLE: &str = "The quarterly report contains useful readable text.";
const WARNING: &str = "Page 1 yielded no usable OCR text after retries; the raster budget reduced rendering \
    from 150 to 25 DPI. Increase security_limits.max_content_size or split/crop the page and retry. \
    A blank page can also produce no text.";

struct FixedBackend {
    name: &'static str,
    text: &'static str,
    calls: Arc<AtomicUsize>,
}

impl Plugin for FixedBackend {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_owned()
    }
    fn initialize(&self) -> crate::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> crate::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for FixedBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, bytes: &[u8], _: &OcrConfig) -> crate::Result<crate::ExtractedDocument> {
        assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 250);
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(crate::ExtractedDocument {
            content: self.text.to_owned(),
            ..Default::default()
        })
    }
}

struct BackendGuard;
impl Drop for BackendGuard {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend(FIRST);
        let _ = unregister_ocr_backend(SECOND);
    }
}

fn fixture() -> (Vec<u8>, PreparedLayoutInputs, Arc<crate::pdf::render::OcrRasterDpis>) {
    let pdf = crate::pdf::render::build_minimal_pdf_with_mediabox(720.0, 720.0);
    let doc = xberg_native_pdf::PdfDocument::from_bytes(pdf.clone()).unwrap();
    let (rendered, dpi) = crate::pdf::render::render_pdf_ocr_page_with_dpi(&doc, 0, None, &limits()).unwrap();
    assert_eq!((rendered.width, rendered.height), (250, 250));
    let image =
        crate::extraction::image_decode::decode_standard_image_with_security_limits(&rendered.data, &limits()).unwrap();
    let detections = vec![crate::layout::DetectionResult {
        page_width: 250,
        page_height: 250,
        detections: Vec::new(),
    }];
    (
        pdf,
        (vec![image], detections),
        Arc::new(ahash::AHashMap::from([(0, dpi)])),
    )
}

fn limits() -> SecurityLimits {
    SecurityLimits {
        max_content_size: 1024 * 1024,
        ..Default::default()
    }
}

fn config(pipeline: bool) -> ExtractionConfig {
    ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: FIRST.to_owned(),
            pipeline: pipeline.then(|| {
                serde_json::from_value(serde_json::json!({
                    "stages": [{"backend": FIRST, "priority": 100}, {"backend": SECOND, "priority": 50}]
                }))
                .unwrap()
            }),
            ..Default::default()
        }),
        layout: Some(LayoutDetectionConfig {
            table_model: TableModel::Disabled,
            ..Default::default()
        }),
        security_limits: Some(limits()),
        ocr_embedded_images: Some(false),
        use_cache: false,
        ..Default::default()
    }
}

async fn read_full(
    pdf: &[u8],
    prepared: &PreparedLayoutInputs,
    config: &ExtractionConfig,
    raster_dpis: Option<Arc<crate::pdf::render::OcrRasterDpis>>,
) -> (String, Vec<crate::types::ProcessingWarning>) {
    let hints = Some(PageOcrHints {
        raster_dpis,
        ..Default::default()
    });
    let (text, doc) = if let Some(pipeline) = config.ocr.as_ref().unwrap().pipeline.as_ref() {
        let output = run_ocr_pipeline(
            Some(pdf),
            Some(&prepared.0),
            Some(&prepared.1),
            config,
            pipeline,
            None,
            hints,
        )
        .await
        .unwrap();
        (output.0, output.3)
    } else {
        let output =
            extract_with_ocr_with_page_hints(Some(pdf), Some(&prepared.0), Some(&prepared.1), config, None, hints)
                .await
                .unwrap();
        (output.0, output.4)
    };
    (text, doc.map(|doc| doc.processing_warnings).unwrap_or_default())
}

async fn read_mixed(
    selected: &[u32],
    pdf: &[u8],
    prepared: &PreparedLayoutInputs,
    config: &ExtractionConfig,
    raster_dpis: Option<Arc<crate::pdf::render::OcrRasterDpis>>,
) -> (String, Vec<crate::types::ProcessingWarning>) {
    let boundaries: Vec<_> = (0..prepared.0.len())
        .map(|page| crate::types::PageBoundary {
            byte_start: 0,
            byte_end: 0,
            page_number: (page + 1) as u32,
        })
        .collect();
    let output = extract_mixed_ocr_native_with_layout_inputs(
        "",
        &boundaries,
        MixedOcrPageSelection {
            ocr: selected,
            single_block: &[],
            preserve_native: &[],
            native_layers: None,
            hidden_layer_backend: None,
            #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
            single_native_type0_page: None,
            raster_dpis,
        },
        pdf,
        config,
        AllPagesFailedPolicy::PreserveNative,
        MixedLayoutInputs::Prepared(Some(prepared)),
    )
    .await
    .unwrap()
    .output;
    (output.0, output.8)
}

async fn check_case(mixed: bool, pipeline: bool, known: bool, text: &'static str) {
    let calls = Arc::new(AtomicUsize::new(0));
    let _guard = BackendGuard;
    for (name, text) in [(FIRST, if pipeline { "" } else { text }), (SECOND, text)] {
        register_ocr_backend(Arc::new(FixedBackend {
            name,
            text,
            calls: calls.clone(),
        }))
        .unwrap();
    }
    let (pdf, prepared, dpis) = fixture();
    let dpis = known.then_some(dpis);
    let config = config(pipeline);
    let (result, warnings) = if mixed {
        read_mixed(&[1], &pdf, &prepared, &config, dpis).await
    } else {
        read_full(&pdf, &prepared, &config, dpis).await
    };
    assert_eq!(calls.load(Ordering::SeqCst), if pipeline { 2 } else { 1 });
    assert_eq!(result.trim(), text, "mixed={mixed}, pipeline={pipeline}, known={known}");
    let warnings: Vec<_> = warnings
        .iter()
        .filter(|warning| warning.source == "ocr_raster_budget")
        .map(|warning| warning.message.as_ref())
        .collect();
    let expected = if known && text.is_empty() {
        vec![WARNING]
    } else {
        Vec::new()
    };
    assert_eq!(
        warnings, expected,
        "mixed={mixed}, pipeline={pipeline}, known={known}, text={text}"
    );
}

async fn check_route(mixed: bool) {
    let mut cases_checked = 0;
    for pipeline in [false, true] {
        for known in [false, true] {
            for text in ["", READABLE] {
                check_case(mixed, pipeline, known, text).await;
                cases_checked += 1;
            }
        }
    }
    assert_eq!(cases_checked, 8);
}

#[tokio::test]
#[serial_test::serial]
async fn prepared_layout_budget_full_uses_final_chosen_text_and_known_provenance() {
    check_route(false).await;
}

#[tokio::test]
#[serial_test::serial]
async fn prepared_layout_budget_mixed_uses_final_chosen_text_and_known_provenance() {
    check_route(true).await;
}

fn two_page_fixture() -> (Vec<u8>, PreparedLayoutInputs, Arc<crate::pdf::render::OcrRasterDpis>) {
    let (bytes, mut prepared, _) = fixture();
    let mut pdf = lopdf::Document::load_mem(&bytes).unwrap();
    let page_id = pdf.get_pages()[&1];
    let page = pdf.get_object(page_id).unwrap().clone();
    let parent = page.as_dict().unwrap().get(b"Parent").unwrap().as_reference().unwrap();
    let second = pdf.add_object(page);
    let pages = pdf.get_object_mut(parent).unwrap().as_dict_mut().unwrap();
    pages
        .get_mut(b"Kids")
        .unwrap()
        .as_array_mut()
        .unwrap()
        .push(second.into());
    pages.set("Count", 2);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    let doc = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).unwrap();
    let (rendered, dpi) = crate::pdf::render::render_pdf_ocr_page_with_dpi(&doc, 1, None, &limits()).unwrap();
    let image =
        crate::extraction::image_decode::decode_standard_image_with_security_limits(&rendered.data, &limits()).unwrap();
    prepared.0.push(image);
    prepared.1.push(prepared.1[0].clone());
    (bytes, prepared, Arc::new(ahash::AHashMap::from([(1, dpi)])))
}

#[tokio::test]
#[serial_test::serial]
async fn prepared_layout_budget_warning_stays_on_its_physical_page() {
    let calls = Arc::new(AtomicUsize::new(0));
    let _guard = BackendGuard;
    register_ocr_backend(Arc::new(FixedBackend {
        name: FIRST,
        text: "",
        calls: calls.clone(),
    }))
    .unwrap();
    for mixed in [false, true] {
        calls.store(0, Ordering::SeqCst);
        let (pdf, prepared, dpis) = two_page_fixture();
        let config = config(false);
        let (text, warnings) = if mixed {
            read_mixed(&[2], &pdf, &prepared, &config, Some(dpis)).await
        } else {
            read_full(&pdf, &prepared, &config, Some(dpis)).await
        };
        assert_eq!(calls.load(Ordering::SeqCst), if mixed { 1 } else { 2 });
        assert_eq!(text.trim(), "");
        let warnings: Vec<_> = warnings
            .iter()
            .filter(|warning| warning.source == "ocr_raster_budget")
            .map(|warning| warning.message.as_ref())
            .collect();
        assert_eq!(warnings, [WARNING.replacen("Page 1", "Page 2", 1)], "mixed={mixed}");
    }
}
