#![cfg(all(feature = "pdf", feature = "ocr"))]

mod helpers;

use std::sync::Arc;

use async_trait::async_trait;
use lopdf::{Document, Object, Stream, dictionary};
use xberg::ExtractedDocument;
use xberg::core::config::{ExtractionConfig, OcrConfig};
use xberg::extractors::security::SecurityLimits;
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};

const BACKEND: &str = "budget-diagnostic-test";
const EMPTY_WARNING: &str = "Page 1 yielded no usable OCR text after retries; the raster budget reduced rendering \
    from 150 to 25 DPI. Increase security_limits.max_content_size or split/crop the page and retry. \
    A blank page can also produce no text.";

struct FixedBackend {
    name: &'static str,
    text: &'static str,
    embedded_only: bool,
}

impl Plugin for FixedBackend {
    fn name(&self) -> &str {
        self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_owned()
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for FixedBackend {
    async fn process_image(&self, bytes: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        let mut result = ExtractedDocument::default();
        let embedded = bytes.get(16..20).is_some_and(|width| width == 30u32.to_be_bytes());
        if !self.embedded_only || embedded {
            result.content = self.text.to_owned();
        }
        Ok(result)
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

fn blank_pdf(embedded: bool) -> Vec<u8> {
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let mut resources = dictionary! {};
    if embedded {
        let raster = pdf.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Image", "Width" => 30, "Height" => 40,
                "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
            },
            vec![160u8; 30 * 40],
        ));
        resources.set("XObject", dictionary! { "Scan" => raster });
    }
    let content = if embedded {
        b"q 720 0 0 720 0 0 cm /Scan Do Q\n".to_vec()
    } else {
        Vec::new()
    };
    let stream = pdf.add_object(Stream::new(dictionary! {}, content));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => stream, "Resources" => resources,
        "MediaBox" => vec![0.into(), 0.into(), 720.into(), 720.into()],
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1,
        }),
    );
    let root = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

fn extract(text: &'static str, mixed: bool, budget: usize) -> ExtractedDocument {
    extract_case(text, mixed, budget, false, false)
}

fn extract_case(text: &'static str, mixed: bool, budget: usize, pipeline: bool, embedded: bool) -> ExtractedDocument {
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: BACKEND.to_owned(),
            pipeline: pipeline.then(|| {
                serde_json::from_value(serde_json::json!({
                    "stages": [
                        { "backend": BACKEND, "priority": 100 },
                        { "backend": "budget-diagnostic-second", "priority": 50 }
                    ]
                }))
                .expect("valid two-stage pipeline with production defaults")
            }),
            ..Default::default()
        }),
        force_ocr: !mixed,
        force_ocr_pages: mixed.then(|| vec![1]),
        security_limits: Some(SecurityLimits {
            max_content_size: budget,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };
    register_ocr_backend(Arc::new(FixedBackend {
        name: BACKEND,
        text: if pipeline { "" } else { text },
        embedded_only: embedded,
    }))
    .unwrap();
    register_ocr_backend(Arc::new(FixedBackend {
        name: "budget-diagnostic-second",
        text,
        embedded_only: false,
    }))
    .unwrap();
    let result = helpers::extract_bytes_document_blocking(&blank_pdf(embedded), "application/pdf", &config);
    unregister_ocr_backend(BACKEND).unwrap();
    unregister_ocr_backend("budget-diagnostic-second").unwrap();
    result.unwrap()
}

#[test]
#[serial_test::serial]
fn budget_clamped_empty_full_and_mixed_pages_report_exact_dpi() {
    for mixed in [false, true] {
        for pipeline in [false, true] {
            let result = extract_case("", mixed, 1024 * 1024, pipeline, false);
            let warnings: Vec<_> = result
                .processing_warnings
                .iter()
                .filter(|warning| warning.source == "ocr_raster_budget")
                .map(|warning| warning.message.as_ref())
                .collect();
            assert_eq!(warnings, [EMPTY_WARNING], "mixed={mixed}");
        }
    }
}

#[test]
#[serial_test::serial]
fn budget_clamped_readable_pages_and_unclamped_empty_pages_have_no_empty_diagnostic() {
    for mixed in [false, true] {
        for (text, budget) in [
            ("The quarterly report contains useful readable text.", 1024 * 1024),
            ("", 100 * 1024 * 1024),
        ] {
            let result = extract(text, mixed, budget);
            assert_eq!(
                result
                    .processing_warnings
                    .iter()
                    .filter(|warning| warning.source == "ocr_raster_budget")
                    .count(),
                0,
                "mixed={mixed}, text={text}"
            );
        }
    }
}

#[test]
#[serial_test::serial]
fn successful_embedded_retry_and_later_pipeline_stage_remove_empty_diagnostic() {
    const RECOVERED: &str = "The quarterly report contains useful readable text.";
    for mixed in [false, true] {
        for (pipeline, embedded) in [(true, false), (false, true)] {
            let result = extract_case(RECOVERED, mixed, 1024 * 1024, pipeline, embedded);
            assert!(
                result.content.contains(RECOVERED),
                "mixed={mixed}, pipeline={pipeline}: {:?}",
                result.content
            );
            assert_eq!(
                result
                    .processing_warnings
                    .iter()
                    .filter(|warning| warning.source == "ocr_raster_budget")
                    .count(),
                0,
                "mixed={mixed}, pipeline={pipeline}"
            );
        }
    }
}
