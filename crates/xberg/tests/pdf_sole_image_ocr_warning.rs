#![cfg(all(feature = "pdf", feature = "ocr"))]

use lopdf::{Document, Object, Stream, dictionary};
use xberg::core::config::ExtractInput;
use xberg::types::ProcessingWarning;
use xberg::{ExtractedDocument, ExtractionConfig, extract};

const WARNING: &str = "Page 1 rendered blank and a painted image could not be decoded for OCR.";

const PAINT_IMAGE: &[u8] = b"q 288 0 0 80 0 0 cm /Im0 Do Q";

fn synthetic_scan(commands: &[u8]) -> Vec<u8> {
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let image = pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => 864, "Height" => 240,
            "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8, "Filter" => "FlateDecode",
        },
        b"This is not zlib compressed data".to_vec(),
    ));
    let content = pdf.add_object(Stream::new(dictionary! {}, commands.to_vec()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Integer(288), Object::Integer(80)],
        "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image } },
        "Contents" => content,
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

async fn public_pdf_result(commands: &[u8], force_ocr: bool) -> ExtractedDocument {
    let config = ExtractionConfig {
        use_cache: false,
        force_ocr,
        ..Default::default()
    };
    extract_pdf(synthetic_scan(commands), &config).await
}

async fn extract_pdf(bytes: Vec<u8>, config: &ExtractionConfig) -> ExtractedDocument {
    let input = ExtractInput::from_bytes(bytes, "application/pdf", None);
    let mut result = extract(input, config)
        .await
        .expect("extract synthetic PDF with real OCR");
    assert_eq!(result.results.len(), 1);
    result.results.remove(0)
}

fn assert_image_warning(document: &ExtractedDocument) {
    assert_eq!(document.content.trim(), "");
    let actual: Vec<_> = document
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "ocr")
        .map(|warning| warning.message.as_ref())
        .collect();
    assert_eq!(actual, [WARNING]);
}

#[tokio::test]
async fn default_pdf_ocr_warns_when_the_only_painted_image_cannot_decode() {
    assert_image_warning(&public_pdf_result(PAINT_IMAGE, false).await);
}

#[tokio::test]
async fn forced_pdf_ocr_warns_when_the_only_painted_image_cannot_decode() {
    assert_image_warning(&public_pdf_result(PAINT_IMAGE, true).await);
}

#[tokio::test]
async fn unused_broken_image_does_not_warn_on_a_blank_pdf_page() {
    let result = public_pdf_result(b"", false).await;
    assert_eq!(result.content.trim(), "");
    assert_no_image_decode_warning(&result.processing_warnings);
}

fn assert_no_image_decode_warning(warnings: &[ProcessingWarning]) {
    assert!(
        warnings.iter().all(|warning| warning.message != WARNING),
        "unexpected painted-image decode warning on blank PDF: {warnings:?}"
    );
}

#[test]
#[should_panic(expected = "unexpected painted-image decode warning on blank PDF")]
fn blank_page_warning_oracle_rejects_the_painted_image_message() {
    assert_no_image_decode_warning(&[ProcessingWarning {
        source: "ocr".into(),
        message: WARNING.into(),
    }]);
}

#[tokio::test]
async fn unresolved_image_operator_does_not_overstate_which_image_failed() {
    let commands = b"q 288 0 0 80 0 0 cm /Missing Do /Im0 Do Q";
    assert_image_warning(&public_pdf_result(commands, true).await);
}

#[cfg(paddle_ocr)]
mod automatic_paddle {
    use super::*;
    use xberg::core::config::{OcrConfig, OcrPipelineConfig};

    fn assert_no_pipeline_failure(result: &ExtractedDocument) {
        assert!(
            result
                .processing_warnings
                .iter()
                .all(|warning| warning.source != "ocr_pipeline"),
            "uniform raster must not trigger the automatic Paddle stage: {:?}",
            result.processing_warnings
        );
        assert!(result.ocr_page_failures.as_ref().is_none_or(Vec::is_empty));
    }

    #[tokio::test]
    async fn blank_pdf_does_not_try_automatic_paddle_fallback() {
        let result = public_pdf_result(b"", false).await;
        assert!(result.content.trim().is_empty());
        assert_no_image_decode_warning(&result.processing_warnings);
        assert_no_pipeline_failure(&result);
    }

    #[tokio::test]
    async fn failed_painted_image_keeps_its_warning_without_trying_automatic_paddle() {
        let result = public_pdf_result(PAINT_IMAGE, true).await;
        assert_image_warning(&result);
        assert_no_pipeline_failure(&result);
    }

    fn faint_printed_pdf() -> Vec<u8> {
        let commands = b"q 0.92 g BT /F1 1 Tf 14 50 Td (Faint printed text on the scanned page) Tj ET Q";
        let mut pdf = Document::load_mem(&synthetic_scan(commands)).expect("generated PDF");
        let page = pdf.get_pages()[&1];
        let font = pdf.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
        });
        pdf.get_object_mut(page)
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .get_mut(b"Resources")
            .unwrap()
            .as_dict_mut()
            .unwrap()
            .set("Font", dictionary! { "F1" => font });
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();
        bytes
    }

    fn pipeline(backends: &[&str]) -> OcrPipelineConfig {
        let stages = backends
            .iter()
            .enumerate()
            .map(|(index, backend)| serde_json::json!({"backend": backend, "priority": 100 - index}))
            .collect::<Vec<_>>();
        serde_json::from_value(serde_json::json!({"stages": stages})).expect("valid test pipeline")
    }

    fn assert_paddle_attempted(result: &ExtractedDocument) {
        assert!(
            result.processing_warnings.iter().any(|warning| {
                warning.source == "ocr_pipeline"
                    && warning
                        .message
                        .starts_with("OCR fallback backend 'paddleocr' failed and was skipped:")
                    && warning.message.contains("offline mode")
            }),
            "an attempted Paddle stage must report its missing offline model: {:?}",
            result.processing_warnings
        );
    }

    #[tokio::test]
    #[ignore = "requires an isolated empty Paddle model cache in offline mode"]
    async fn faint_text_with_empty_tesseract_output_still_tries_automatic_paddle() {
        let bytes = faint_printed_pdf();
        let rendered = xberg::render_pdf_page_to_png(&bytes, 0, Some(300), None).expect("actual faint raster");
        let luma = image::load_from_memory(&rendered).expect("rendered PNG").to_luma8();
        let minimum = *luma.as_raw().iter().min().expect("nonempty raster");
        let maximum = *luma.as_raw().iter().max().expect("nonempty raster");
        assert!(minimum > 128 && minimum < maximum, "faint ink survives rendering");
        let config = ExtractionConfig {
            use_cache: false,
            force_ocr: true,
            ..Default::default()
        };
        let tesseract_only = ExtractionConfig {
            ocr: Some(OcrConfig {
                pipeline: Some(pipeline(&["tesseract"])),
                ..Default::default()
            }),
            ..config.clone()
        };
        let primary = extract_pdf(bytes.clone(), &tesseract_only).await;
        assert!(primary.content.trim().is_empty(), "Tesseract misses this faint text");
        assert_paddle_attempted(&extract_pdf(bytes, &config).await);
    }

    #[tokio::test]
    #[ignore = "requires an isolated empty Paddle model cache in offline mode"]
    async fn explicitly_configured_paddle_stage_can_still_run_on_blank_pdf() {
        let config = ExtractionConfig {
            use_cache: false,
            force_ocr: true,
            ocr: Some(OcrConfig {
                pipeline: Some(pipeline(&["tesseract", "paddleocr"])),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_paddle_attempted(&extract_pdf(synthetic_scan(b""), &config).await);
    }

    #[tokio::test]
    #[ignore = "requires an isolated empty Paddle model cache in offline mode"]
    async fn blank_raster_with_usable_embedded_image_still_tries_paddle() {
        let mut pdf = Document::load_mem(&synthetic_scan(b"q -10 -10 1 1 re W n 288 0 0 80 0 0 cm /Im0 Do Q")).unwrap();
        let page = pdf.get_object(pdf.get_pages()[&1]).unwrap().as_dict().unwrap();
        let image = page
            .get(b"Resources")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"XObject")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"Im0")
            .unwrap()
            .as_reference()
            .unwrap();
        let stream = pdf.get_object_mut(image).unwrap().as_stream_mut().unwrap();
        stream.dict.remove(b"Filter");
        stream.dict.set("Width", 32);
        stream.dict.set("Height", 24);
        let mut pixels = vec![255; 32 * 24];
        pixels[384..392].fill(242);
        stream.set_content(pixels);
        let mut bytes = Vec::new();
        pdf.save_to(&mut bytes).unwrap();

        let rendered = xberg::render_pdf_page_to_png(&bytes, 0, Some(300), None).expect("clipped page raster");
        let raster = image::load_from_memory(&rendered).expect("clipped PNG").to_rgb8();
        assert!(raster.as_raw().iter().all(|&sample| sample == 255));
        let native = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).expect("native PDF");
        let handles = native.page_image_handles(0).expect("painted image handles");
        assert_eq!(handles.len(), 1);
        let alternate = handles[0].decode().expect("usable alternate source");
        assert_eq!((alternate.width(), alternate.height()), (32, 24));

        let config = ExtractionConfig {
            use_cache: false,
            force_ocr: true,
            ..Default::default()
        };
        let result = extract_pdf(bytes, &config).await;
        assert!(result.content.trim().is_empty());
        assert_no_image_decode_warning(&result.processing_warnings);
        assert_paddle_attempted(&result);
    }

    #[tokio::test]
    #[ignore = "temporarily replaces a process-global backend; run this test alone"]
    async fn blank_pdf_still_calls_a_registered_paddle_replacement() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};

        const TEXT: &str = "The caller configured this replacement backend to describe blank pages.";
        struct Replacement(Arc<AtomicUsize>);
        impl Plugin for Replacement {
            fn name(&self) -> &str {
                "paddleocr"
            }
            fn version(&self) -> String {
                "1.0.0".into()
            }
            fn initialize(&self) -> xberg::Result<()> {
                Ok(())
            }
            fn shutdown(&self) -> xberg::Result<()> {
                Ok(())
            }
        }
        #[async_trait::async_trait]
        impl OcrBackend for Replacement {
            async fn process_image(&self, _: &[u8], _: &OcrConfig) -> xberg::Result<ExtractedDocument> {
                self.0.fetch_add(1, Ordering::SeqCst);
                let mut result = ExtractedDocument::default();
                result.content = TEXT.to_owned();
                Ok(result)
            }
            fn supports_language(&self, _: &str) -> bool {
                true
            }
            fn backend_type(&self) -> OcrBackendType {
                OcrBackendType::PaddleOCR
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        register_ocr_backend(Arc::new(Replacement(calls.clone()))).expect("register the caller's Paddle replacement");
        let result = public_pdf_result(b"", false).await;
        unregister_ocr_backend("paddleocr").expect("remove the replacement");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(result.content.trim(), TEXT);
    }
}
