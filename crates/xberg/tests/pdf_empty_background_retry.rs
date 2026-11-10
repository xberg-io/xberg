#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

mod helpers;

use async_trait::async_trait;
use lopdf::{Document, Object, Stream, dictionary};
use std::sync::{Arc, Mutex};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig};

const BACKEND: &str = "jpeg-budget-empty-background";
const LABELS: &str = "A B C D E F G H I J K L M N O P Q R S T U V W X";
const SENTENCE: &str = "The river runs past the old mill and into the lake below the town";
const BACKGROUND_NOISE: &str = "Q R S T U V W X Y Z Q R S T U V W X Y Z Q R S T";

#[derive(Clone, Copy)]
enum BackgroundRead {
    Empty,
    Whitespace,
    Failed,
    Text(&'static str),
}

struct LabelBackend {
    seen: Arc<Mutex<Vec<(u32, u32)>>>,
    background_read: BackgroundRead,
    foreground_read: &'static str,
}

impl Plugin for LabelBackend {
    fn name(&self) -> &str {
        BACKEND
    }
    fn version(&self) -> String {
        "1".to_owned()
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for LabelBackend {
    async fn process_image(&self, bytes: &[u8], config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        assert!(
            bytes.starts_with(&[0xff, 0xd8]),
            "the budget retry must read the original JPEG"
        );
        let image = image::load_from_memory(bytes).expect("valid JPEG, not a modified dimension header");
        self.seen.lock().unwrap().push((image.width(), image.height()));
        let background = image.width() == 1024;
        let expected_dpi = if background { 75.0 } else { 150.0 };
        let dpi = config
            .backend_options
            .as_ref()
            .and_then(|v| v.get("source_dpi"))
            .and_then(serde_json::Value::as_f64)
            .expect("each image has its own placement-proven DPI");
        assert!((dpi - expected_dpi).abs() < 0.01, "{dpi} != {expected_dpi}");
        if background {
            verify_white_clamp(&image);
        } else {
            assert!(
                image.to_luma8().pixels().any(|p| p.0[0] < 40),
                "foreground ink survives"
            );
        }
        if background && matches!(self.background_read, BackgroundRead::Failed) {
            return Err(xberg::XbergError::Ocr {
                message: "controlled background failure".into(),
                source: None,
            });
        }
        let content = if !background {
            self.foreground_read
        } else {
            match self.background_read {
                BackgroundRead::Whitespace => " \n\t",
                BackgroundRead::Text(text) => text,
                _ => "",
            }
        };
        let mut result = ExtractedDocument::default();
        result.content = content.to_owned();
        result.metadata.image_preprocessing = Some(preprocessing(image.width(), image.height(), background));
        Ok(result)
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

fn verify_white_clamp(image: &image::DynamicImage) {
    assert_eq!((image.width(), image.height()), (1024, 768));
    let resized = image::imageops::resize(&image.to_rgb8(), 16, 12, image::imageops::FilterType::Nearest);
    assert_eq!(resized.dimensions(), (16, 12));
    assert!(resized.pixels().all(|p| p.0 == [255; 3]));
}

fn preprocessing(width: u32, height: u32, background: bool) -> xberg::types::ImagePreprocessingMetadata {
    let dpi = if background { 75.0 } else { 150.0 };
    serde_json::from_value(serde_json::json!({
        "original_dimensions": {"width": width, "height": height},
        "original_dpi": {"horizontal": dpi, "vertical": dpi}, "target_dpi": 150,
        "scale_factor": if background {16.0 / 1024.0} else {1.0}, "resample_method": "nearest",
        "new_dimensions": if background {serde_json::json!({"width":16,"height":12})}
            else {serde_json::Value::Null},
        "final_dpi": if background {1} else {150}, "dimension_clamped": background,
        "auto_adjusted": background, "calculated_dpi": null, "skipped_resize": !background,
        "resize_error": null
    }))
    .expect("coherent preprocessing from this image")
}

fn jpeg(background: bool) -> Vec<u8> {
    let (width, height) = if background { (1024, 768) } else { (2048, 1536) };
    let mut pixels = image::RgbImage::from_pixel(width, height, image::Rgb([255; 3]));
    if !background {
        for y in (100..1400).step_by(100) {
            for x in (100..1900).step_by(100) {
                for (dx, dy) in (0..40)
                    .flat_map(|dy| (0..25).map(move |dx| (dx, dy)))
                    .filter(|(dx, dy)| *dx < 4 || *dy < 4 || *dy == 20)
                {
                    pixels.put_pixel(x + dx, y + dy, image::Rgb([0; 3]));
                }
            }
        }
    }
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 85)
        .encode(pixels.as_raw(), width, height, image::ExtendedColorType::Rgb8)
        .unwrap();
    assert_eq!(
        image::load_from_memory(&bytes).unwrap().to_rgb8().dimensions(),
        (width, height)
    );
    bytes
}

fn add_image(pdf: &mut Document, background: bool) -> lopdf::ObjectId {
    let (width, height) = if background { (1024, 768) } else { (2048, 1536) };
    pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => width, "Height" => height,
            "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8, "Filter" => "DCTDecode",
        },
        jpeg(background),
    ))
}

fn scan_pdf(background: bool) -> Vec<u8> {
    let mut pdf = Document::with_version("1.5");
    let pages = pdf.new_object_id();
    let foreground = add_image(&mut pdf, false);
    let white_background = add_image(&mut pdf, true);
    // The white image is behind the ink, so it cannot obscure the valid foreground. ~keep
    let paint = if background {
        "q 983.04 0 0 737.28 0 0 cm /Background Do Q q 983.04 0 0 737.28 0 0 cm /Scan Do Q"
    } else {
        "q 983.04 0 0 737.28 0 0 cm /Scan Do Q"
    };
    let content = pdf.add_object(Stream::new(dictionary! {}, paint.as_bytes().to_vec()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 983.04.into(), 737.28.into()],
        "Resources" => dictionary! { "XObject" => dictionary! { "Scan" => foreground, "Background" => white_background } },
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

fn run(
    pipeline: bool,
    background: bool,
    background_read: BackgroundRead,
    foreground_read: &'static str,
    discard_noise: bool,
) -> (xberg::Result<ExtractedDocument>, Vec<(u32, u32)>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    register_ocr_backend(Arc::new(LabelBackend {
        seen: seen.clone(),
        background_read,
        foreground_read,
    }))
    .unwrap();
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false, "force_ocr_pages": [1], "pages": {"extract_pages": true},
        "security_limits": {"max_content_size": 24 * 1024 * 1024},
        "ocr": {"backend": BACKEND, "quality_thresholds": {"discard_suspected_ocr_noise": discard_noise},
            "pipeline": if pipeline {serde_json::json!({"stages": [{"backend": BACKEND}]})}
                else {serde_json::Value::Null}},
    }))
    .unwrap();
    let result = helpers::extract_bytes_document_blocking(&scan_pdf(background), "application/pdf", &config);
    unregister_ocr_backend(BACKEND).unwrap();
    let observations = seen.lock().unwrap().clone();
    (result, observations)
}

fn assert_background_retains_labels(pipeline: bool, background_read: BackgroundRead) {
    let (control, seen) = run(pipeline, false, background_read, LABELS, false);
    assert!(
        control
            .expect("foreground alone retains labels")
            .content
            .contains(LABELS)
    );
    assert_eq!(seen, vec![(2048, 1536)]);
    let (result, seen) = run(pipeline, true, background_read, LABELS, false);
    assert_eq!(
        seen,
        vec![(1024, 768), (2048, 1536)],
        "both original JPEGs reached the backend in paint order"
    );
    let doc = result.expect("a background with no text must not reject the separate foreground read");
    assert!(doc.content.contains(LABELS), "{:?}", doc.processing_warnings);
    let warnings: Vec<_> = doc
        .processing_warnings
        .iter()
        .filter(|w| w.message.contains("suspected OCR recognition noise"))
        .collect();
    assert_eq!(warnings.len(), 1, "{:?}", doc.processing_warnings);
    assert!(warnings[0].message.contains("its text was retained"));
    assert!(
        !doc.processing_warnings
            .iter()
            .any(|w| w.message.contains("the retry read was rejected as noise"))
    );
    assert!(
        doc.ocr_page_failures
            .iter()
            .flatten()
            .all(|f| !f.error.contains("the retry read was rejected as noise"))
    );
}

#[test]
#[serial_test::serial]
fn direct_empty_proven_background_retains_foreground_labels() {
    assert_background_retains_labels(false, BackgroundRead::Empty);
}
#[test]
#[serial_test::serial]
fn pipeline_empty_proven_background_retains_foreground_labels() {
    assert_background_retains_labels(true, BackgroundRead::Empty);
}
#[test]
#[serial_test::serial]
fn direct_whitespace_proven_background_retains_foreground_labels() {
    assert_background_retains_labels(false, BackgroundRead::Whitespace);
}
#[test]
#[serial_test::serial]
fn pipeline_whitespace_proven_background_retains_foreground_labels() {
    assert_background_retains_labels(true, BackgroundRead::Whitespace);
}
#[test]
#[serial_test::serial]
fn direct_failed_proven_background_retains_foreground_labels() {
    assert_background_retains_labels(false, BackgroundRead::Failed);
}
#[test]
#[serial_test::serial]
fn pipeline_failed_proven_background_retains_foreground_labels() {
    assert_background_retains_labels(true, BackgroundRead::Failed);
}

fn assert_one_image_read_is_judged_on_its_own(
    pipeline: bool,
    background_read: BackgroundRead,
    foreground_read: &'static str,
    discard_noise: bool,
) {
    let (result, seen) = run(pipeline, true, background_read, foreground_read, discard_noise);
    assert_eq!(
        seen,
        vec![(1024, 768), (2048, 1536)],
        "both original JPEGs reached the backend"
    );
    let doc = result.expect("one image's read must not reject the other image's text");
    assert!(
        doc.content.contains(SENTENCE),
        "{:?} {:?}",
        doc.content,
        doc.processing_warnings
    );
    assert!(!doc.content.contains(LABELS), "{:?}", doc.content);
    assert!(!doc.content.contains(BACKGROUND_NOISE), "{:?}", doc.content);
}

#[test]
#[serial_test::serial]
fn direct_clamped_legible_background_survives_noisy_foreground() {
    assert_one_image_read_is_judged_on_its_own(false, BackgroundRead::Text(SENTENCE), LABELS, true);
}
#[test]
#[serial_test::serial]
fn pipeline_clamped_legible_background_survives_noisy_foreground() {
    assert_one_image_read_is_judged_on_its_own(true, BackgroundRead::Text(SENTENCE), LABELS, true);
}
#[test]
#[serial_test::serial]
fn direct_clamped_noisy_background_is_rejected_beside_legible_foreground() {
    assert_one_image_read_is_judged_on_its_own(false, BackgroundRead::Text(BACKGROUND_NOISE), SENTENCE, false);
}
#[test]
#[serial_test::serial]
fn pipeline_clamped_noisy_background_is_rejected_beside_legible_foreground() {
    assert_one_image_read_is_judged_on_its_own(true, BackgroundRead::Text(BACKGROUND_NOISE), SENTENCE, false);
}

fn assert_clamped_noisy_read_alone_is_rejected(pipeline: bool) {
    let (result, seen) = run(pipeline, true, BackgroundRead::Text(BACKGROUND_NOISE), "", false);
    assert_eq!(seen, vec![(1024, 768), (2048, 1536)]);
    let error = result.expect_err("the only read of the page is a clamped read that is noise");
    let message = format!("{error:?}");
    assert!(message.contains("the retry read was rejected as noise"), "{message}");
}

#[test]
#[serial_test::serial]
fn direct_clamped_noisy_background_alone_is_still_rejected() {
    assert_clamped_noisy_read_alone_is_rejected(false);
}
#[test]
#[serial_test::serial]
fn pipeline_clamped_noisy_background_alone_is_still_rejected() {
    assert_clamped_noisy_read_alone_is_rejected(true);
}
