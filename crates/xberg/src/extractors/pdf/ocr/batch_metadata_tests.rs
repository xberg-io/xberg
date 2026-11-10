use std::sync::{Arc, Mutex};

use image::{DynamicImage, Rgb, RgbImage};

use super::pipeline::extract_with_ocr_for_page;
use super::rendering::{open_pdf_for_full_ocr, page_dimensions_pt};
use crate::core::config::{ExtractionConfig, OcrConfig, OutputFormat};
use crate::ocr_metadata_keys::{
    OCR_AUTO_ROTATED_METADATA_KEY, OCR_BACKEND_ROTATION_UNDONE_METADATA_KEY, OCR_ORIENTATION_DEGREES_METADATA_KEY,
    OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY, OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
};
use crate::plugins::{OcrBackend, OcrBackendType, Plugin};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::ocr_elements::{OcrBoundingGeometry, OcrConfidence, OcrElement, OcrElementLevel};
use crate::types::{BoundingBox, ExtractedDocument};

const BACKEND: &str = "pdf-2356-batch-metadata-frame";
const PROSE: &str = "The source document contains readable ordinary words and preserves their original geometry.";

struct RotatingBackend {
    frames: Mutex<Vec<(u32, u32)>>,
}

impl Plugin for RotatingBackend {
    fn name(&self) -> &str {
        BACKEND
    }
}

#[async_trait::async_trait]
impl OcrBackend for RotatingBackend {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }

    fn supports_language(&self, _: &str) -> bool {
        true
    }

    async fn process_image(&self, bytes: &[u8], config: &OcrConfig) -> crate::Result<ExtractedDocument> {
        assert_eq!(config.backend, BACKEND);
        let raster = image::load_from_memory(bytes).expect("actual batch PNG must decode");
        assert!(matches!((raster.width(), raster.height()), (300, 200) | (420, 240)));
        assert_eq!(raster.to_rgb8().get_pixel(60, 35).0, [0, 0, 255]);
        self.frames.lock().unwrap().push((raster.width(), raster.height()));
        let processed = raster.rotate90();
        assert_eq!(
            (processed.width(), processed.height()),
            (raster.height(), raster.width())
        );
        assert_eq!(processed.to_rgb8().get_pixel(raster.height() - 36, 60).0, [0, 0, 255]);
        Ok(rotated_result(&processed, raster.width()))
    }
}

fn rotated_result(processed: &DynamicImage, source_width: u32) -> ExtractedDocument {
    let left = processed.width() - 50;
    let mut document = InternalDocument::new("pdf");
    let mut word = InternalElement::text(
        ElementKind::OcrText {
            level: OcrElementLevel::Word,
        },
        PROSE,
        0,
    );
    word.bbox = Some(BoundingBox {
        x0: f64::from(left),
        y0: 40.0,
        x1: f64::from(left + 30),
        y1: 100.0,
    });
    document.push_element(word);
    let mut result = crate::extraction::derive::derive_extraction_result(document.clone(), true, OutputFormat::Plain);
    result.ocr_internal_document = Some(document);
    result.ocr_elements = Some(vec![OcrElement {
        text: "source".into(),
        level: OcrElementLevel::Word,
        geometry: OcrBoundingGeometry::Rectangle {
            left,
            top: 40,
            width: 30,
            height: 60,
        },
        confidence: OcrConfidence {
            recognition: 0.99,
            detection: None,
        },
        ..Default::default()
    }]);
    for (key, value) in [
        (
            OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY,
            serde_json::json!(processed.width()),
        ),
        (
            OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY,
            serde_json::json!(processed.height()),
        ),
        (OCR_AUTO_ROTATED_METADATA_KEY, serde_json::json!(true)),
        (OCR_ORIENTATION_DEGREES_METADATA_KEY, serde_json::json!(270)),
        ("representative_source_width", serde_json::json!(source_width)),
    ] {
        result.metadata.additional.insert(key.into(), value);
    }
    result
}

fn source_pdf(frames: &[(u32, u32)]) -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages = document.new_object_id();
    let mut kids: Vec<Object> = Vec::new();
    for &(width, height) in frames {
        let stream = document.add_object(Stream::new(dictionary! {}, Vec::new()));
        let page = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages, "Rotate" => 0,
            "MediaBox" => vec![0.into(), 0.into(), i64::from(width).into(), i64::from(height).into()],
            "Resources" => dictionary! {}, "Contents" => stream,
        });
        kids.push(page.into());
    }
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => kids, "Count" => frames.len() as i64,
        }),
    );
    let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    document.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

fn source_raster(width: u32, height: u32) -> DynamicImage {
    let mut raster = RgbImage::from_pixel(width, height, Rgb([255, 255, 255]));
    for y in 20..50 {
        for x in 40..100 {
            raster.put_pixel(x, y, Rgb([0, 0, 255]));
        }
    }
    DynamicImage::ImageRgb8(raster)
}

struct BackendGuard;

impl Drop for BackendGuard {
    fn drop(&mut self) {
        let _ = crate::plugins::unregister_ocr_backend(BACKEND);
    }
}

fn config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        enable_quality_processing: false,
        ocr: Some(OcrConfig {
            backend: BACKEND.into(),
            auto_rotate: Some(true),
            element_config: Some(crate::types::OcrElementConfig {
                include_elements: true,
                min_level: OcrElementLevel::Word,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn exercise(frames: &[(u32, u32)]) {
    let bytes = source_pdf(frames);
    let (source, count, rotations) = open_pdf_for_full_ocr(&bytes).expect("valid two-page source PDF");
    assert_eq!(count, 2);
    assert_eq!(rotations, [0, 0], "no caller upright rotation");
    for (index, &(width, height)) in frames.iter().enumerate() {
        assert_eq!(page_dimensions_pt(&source, index), (width as f32, height as f32));
    }
    let images: Vec<_> = frames
        .iter()
        .map(|&(width, height)| source_raster(width, height))
        .collect();
    let backend = Arc::new(RotatingBackend {
        frames: Mutex::new(Vec::new()),
    });
    crate::plugins::register_ocr_backend(backend.clone()).unwrap();
    let _guard = BackendGuard;
    let result = run_batch(&bytes, &images).await;
    let mut observed = backend.frames.lock().unwrap().clone();
    observed.sort_unstable();
    let mut expected = frames.to_vec();
    expected.sort_unstable();
    assert_eq!(
        observed, expected,
        "exactly two actual producer calls in the source frames"
    );
    assert_metadata(&result, frames[0]);
}

async fn run_batch(bytes: &[u8], images: &[DynamicImage]) -> ExtractedDocument {
    let run = Box::pin(extract_with_ocr_for_page(
        Some(bytes),
        Some(images),
        #[cfg(feature = "layout-detection")]
        None,
        &config(),
        None,
        0,
        true,
        None,
        None,
        0,
        None,
    ))
    .await
    .expect("actual page-batch production path");
    let (text, _, _, elements, document, _, page_texts, ..) = run;
    assert_actual_words(&text, &elements, &page_texts);
    let document = document.expect("real batch returned a structured document");
    assert!(!document.elements.is_empty());
    let result = crate::extraction::derive::derive_extraction_result(document, true, OutputFormat::Plain);
    assert!(
        result
            .document
            .as_ref()
            .is_some_and(|document| !document.nodes.is_empty())
    );
    result
}

fn assert_actual_words(text: &str, elements: &[OcrElement], page_texts: &[String]) {
    assert_eq!(page_texts.len(), 2);
    assert!(page_texts.iter().all(|text| text.contains("source")) && text.contains("source"));
    assert_eq!(
        elements.len(),
        2,
        "two public word geometries before metadata assertions"
    );
    for element in elements {
        assert_eq!(element.text, "source");
        assert!(matches!(
            element.geometry,
            OcrBoundingGeometry::Rectangle {
                left: 40,
                top: 20,
                width: 60,
                height: 30
            }
        ));
    }
}

fn assert_metadata(result: &ExtractedDocument, first: (u32, u32)) {
    let metadata = &result.metadata.additional;
    assert_eq!(metadata["representative_source_width"], serde_json::json!(first.0));
    assert_eq!(metadata[OCR_AUTO_ROTATED_METADATA_KEY], serde_json::json!(true));
    assert_eq!(metadata[OCR_ORIENTATION_DEGREES_METADATA_KEY], serde_json::json!(270));
    assert!(
        metadata[OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY]
            .as_u64()
            .is_some_and(|value| value > 0)
    );
    assert!(
        metadata[OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY]
            .as_u64()
            .is_some_and(|value| value > 0)
    );
    assert_eq!(
        [
            metadata[OCR_PROCESSED_IMAGE_WIDTH_METADATA_KEY].as_u64(),
            metadata[OCR_PROCESSED_IMAGE_HEIGHT_METADATA_KEY].as_u64()
        ],
        [Some(u64::from(first.0)), Some(u64::from(first.1))],
        "published metadata must describe the normalized representative frame, not the old backend frame"
    );
    assert_eq!(
        metadata.get(OCR_BACKEND_ROTATION_UNDONE_METADATA_KEY),
        Some(&serde_json::json!(true))
    );
}

#[tokio::test]
#[serial_test::serial]
async fn should_publish_normalized_first_page_batch_metadata_2356() {
    exercise(&[(300, 200), (420, 240)]).await;
}

#[tokio::test]
#[serial_test::serial]
async fn should_keep_first_page_metadata_when_batch_page_sizes_are_reversed_2356() {
    exercise(&[(420, 240), (300, 200)]).await;
}
