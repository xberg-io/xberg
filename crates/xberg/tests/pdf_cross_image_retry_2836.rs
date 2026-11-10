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

const BACKEND: &str = "jpeg-budget-joint-2836";
const NOISE_CAUSE: &str = "the retry read was rejected as noise";
const LABELS: &str = "A B C D E F G H I J K L M N O P Q R S T U V W X";

struct LabelBackend {
    seen: Arc<Mutex<Vec<(bool, u32, u32)>>>,
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
        let image = image::load_from_memory(bytes).expect("real decodable image, not a patched JPEG header");
        let jpeg = bytes.starts_with(&[0xff, 0xd8]);
        if jpeg && image.width() > 100 {
            let dpi = config
                .backend_options
                .as_ref()
                .and_then(|options| options.get("source_dpi"))
                .and_then(serde_json::Value::as_f64)
                .expect("placement-proven JPEG source DPI");
            assert!((dpi - 150.0).abs() < 0.01, "source DPI {dpi}");
        }
        self.seen.lock().unwrap().push((jpeg, image.width(), image.height()));
        assert!(
            image.to_luma8().pixels().any(|pixel| pixel.0[0] < 40),
            "painted ink remains visible"
        );
        if image.width() == 32 {
            assert!(
                config
                    .backend_options
                    .as_ref()
                    .and_then(|v| v.get("source_dpi"))
                    .is_none(),
                "sheared auxiliary must have no proven DPI"
            );
        }
        let aux = image.width() == 32;
        let mut result = ExtractedDocument::default();
        result.content = if image.width() == 32 {
            String::new()
        } else {
            LABELS.to_owned()
        };
        result.metadata.image_preprocessing = Some(preprocessing(image.width(), image.height(), aux));
        Ok(result)
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

fn preprocessing(width: u32, height: u32, aux: bool) -> xberg::types::ImagePreprocessingMetadata {
    serde_json::from_value(serde_json::json!({
        "original_dimensions": {"width": width, "height": height},
        "original_dpi": {"horizontal": 150.0, "vertical": 150.0},
        "target_dpi": 150,
        "scale_factor": if aux {0.5} else {1.0},
        "resample_method": "none",
        "new_dimensions": if aux {serde_json::json!({"width":16,"height":16})} else {serde_json::Value::Null},
        "final_dpi": if aux {75} else {150},
        "dimension_clamped": aux,
        "auto_adjusted": aux,
        "calculated_dpi": null,
        "skipped_resize": !aux,
        "resize_error": null,
    }))
    .expect("coherent per-image preprocessing record")
}

fn label_jpeg() -> Vec<u8> {
    let (width, height) = (2048u32, 1536u32);
    let mut pixels = image::RgbImage::from_pixel(width, height, image::Rgb([255; 3]));
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
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85)
        .encode(pixels.as_raw(), width, height, image::ExtendedColorType::Rgb8)
        .unwrap();
    let decoded = image::load_from_memory(&jpeg).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (width, height));
    jpeg
}

fn scan_pdf(aux: bool) -> Vec<u8> {
    let (width, height) = (2048u32, 1536u32);
    let jpeg = label_jpeg();
    let mut pdf = Document::with_version("1.5");
    let pages = pdf.new_object_id();
    let raster = pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => width as i64, "Height" => height as i64,
            "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8, "Filter" => "DCTDecode",
        },
        jpeg,
    ));
    let mut aux_jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut aux_jpeg, 85)
        .encode(&[0u8; 32 * 32 * 3], 32, 32, image::ExtendedColorType::Rgb8)
        .unwrap();
    let auxiliary = pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => 32, "Height" => 32,
            "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8, "Filter" => "DCTDecode",
        },
        aux_jpeg,
    ));
    let content = pdf.add_object(Stream::new(
        dictionary! {},
        if aux {
            b"q 983.04 0 0 737.28 0 0 cm /Scan Do Q q 16 0 2 16 20 20 cm /Aux Do Q".to_vec()
        } else {
            b"q 983.04 0 0 737.28 0 0 cm /Scan Do Q".to_vec()
        },
    ));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 983.04.into(), 737.28.into()],
        "Resources" => dictionary! { "XObject" => dictionary! { "Scan" => raster,"Aux"=>auxiliary } }, "Contents" => content,
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
    discard: bool,
    budget_mib: usize,
    aux: bool,
) -> (xberg::Result<ExtractedDocument>, Vec<(bool, u32, u32)>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    register_ocr_backend(Arc::new(LabelBackend { seen: seen.clone() })).unwrap();
    let config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false, "force_ocr_pages": [1], "pages": {"extract_pages": true},
        "security_limits": {"max_content_size": budget_mib * 1024 * 1024},
        "ocr": {"backend": BACKEND, "quality_thresholds": {"discard_suspected_ocr_noise": discard},
            "pipeline": if pipeline { serde_json::json!({"stages": [{"backend": BACKEND}]}) } else { serde_json::Value::Null }},
    })).unwrap();
    let outcome = helpers::extract_bytes_document_blocking(&scan_pdf(aux), "application/pdf", &config);
    unregister_ocr_backend(BACKEND).unwrap();
    let observations = seen.lock().unwrap().clone();
    (outcome, observations)
}

fn assert_joint(pipeline: bool) {
    let (without, seen) = run(pipeline, false, 24, false);
    let doc = without.expect("same proven label image alone must retain a real nonempty read");
    assert!(doc.content.contains(LABELS));
    assert!(seen.contains(&(true, 2048, 1536)));
    let (with, seen) = run(pipeline, false, 24, true);
    assert!(seen.contains(&(true, 2048, 1536)), "{seen:?}");
    assert!(
        seen.contains(&(true, 32, 32)),
        "unproven clamped blank auxiliary actually reached {seen:?}"
    );
    let doc = with.expect("an unrelated unproven clamped image must not veto the proven unclamped label read");
    assert!(doc.content.contains(LABELS), "{:?}", doc.processing_warnings);
    // LABELS is 24 one-letter words, so it trips the noise test at the default thresholds: the
    // read is kept with the warning, not dropped with the retry's noise cause.
    let noise: Vec<_> = doc
        .processing_warnings
        .iter()
        .filter(|warning| warning.message.contains("suspected OCR recognition noise"))
        .collect();
    assert_eq!(noise.len(), 1, "{:?}", doc.processing_warnings);
    assert!(
        noise[0].message.contains("its text was retained"),
        "{}",
        noise[0].message
    );
    let mentions_cause = doc.processing_warnings.iter().any(|w| w.message.contains(NOISE_CAUSE))
        || doc
            .ocr_page_failures
            .iter()
            .flatten()
            .any(|failure| failure.error.contains(NOISE_CAUSE));
    assert!(
        !mentions_cause,
        "{:?} {:?}",
        doc.processing_warnings, doc.ocr_page_failures
    );
}
#[test]
#[serial_test::serial]
fn direct_unrelated_clamp_cannot_reject_unclamped_labels() {
    assert_joint(false);
}
#[test]
#[serial_test::serial]
fn pipeline_unrelated_clamp_cannot_reject_unclamped_labels() {
    assert_joint(true);
}
