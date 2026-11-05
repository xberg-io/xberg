use super::{MIB, pixel_memory};
use crate::core::config::ExtractionConfig;
use image::ImageEncoder;
use sha2::{Digest, Sha256};
use std::process::Command;

#[path = "../../../../extractors/pdf/ocr/english_recovery_tests/fixture.rs"]
mod fixture;

const CHILD: &str = "XBERG_PIPELINE_PNG_MEMORY_CHILD";
const DIMENSION: u32 = 2000;
const MODEL_SHA: &str = "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2";

fn config(extraction_mib: usize, ocr_mib: usize) -> ExtractionConfig {
    let path = std::path::PathBuf::from(
        std::env::var_os("XBERG_2339_TESSDATA_DIR").expect("explicit verified local model required"),
    );
    let model = std::fs::read(path.join("eng.traineddata")).expect("model must already exist; no downloads");
    assert_eq!(model.len(), 4_113_088);
    assert_eq!(hex::encode(Sha256::digest(&model)), MODEL_SHA);
    let mut config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache": false, "force_ocr": true,
        "ocr": {
            "backend": "tesseract", "language": ["eng"],
            "backend_options": {"source_dpi":300, "use_cache":false},
            "tesseract_config": {"psm":6,"preprocessing":{
                "target_dpi":300,"auto_rotate":false,"deskew":true,"denoise":false,
                "contrast_enhance":false,"binarization_method":"otsu","invert_colors":false,
                "normalize_shaded_rows":false
            }},
            "pipeline": {"stages":[{
                "backend":"tesseract", "priority":1,
                "backend_options":{"source_dpi":300,"use_cache":false}
            }], "quality_thresholds":{"pipeline_min_quality":0.0}}
        }
    }))
    .unwrap();
    config.security_limits = Some(crate::extractors::security::SecurityLimits {
        max_content_size: extraction_mib * MIB,
        ..Default::default()
    });
    let ocr = config.ocr.as_mut().unwrap();
    ocr.security_limits = Some(crate::extractors::security::SecurityLimits {
        max_content_size: ocr_mib * MIB,
        ..Default::default()
    });
    ocr.tessdata_path = Some(path);
    config
}

fn png() -> Vec<u8> {
    let fixture = fixture::fixture();
    assert_eq!(
        fixture::sha(&fixture.pdf),
        fixture.receipt["image_pdf_sha256"].as_str().unwrap()
    );
    let patch = fixture::detached_image(&fixture.doc).to_rgb8();
    let mut page = image::RgbImage::from_pixel(DIMENSION, DIMENSION, image::Rgb([255, 255, 255]));
    image::imageops::replace(&mut page, &patch, 100, 100);
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(page.as_raw(), DIMENSION, DIMENSION, image::ExtendedColorType::Rgb8)
        .unwrap();
    bytes
}

async fn measure(extraction_mib: usize, ocr_mib: usize, affordable: bool) {
    let config = config(extraction_mib, ocr_mib);
    let png = png();
    let encoded_len = png.len();
    {
        let registry = crate::plugins::registry::get_ocr_backend_registry();
        let registry = registry.read();
        let backend = registry.get("tesseract").unwrap();
        assert!(
            registry.is_builtin_tesseract(&backend),
            "actual built-in identity is required"
        );
    }
    let encode_peak = crate::extractors::pdf::ocr::estimate_png_encode_page_peak_bytes(DIMENSION, DIMENSION).unwrap();
    let budget = extraction_mib.min(ocr_mib) * MIB;
    assert!(
        encode_peak < budget as u64,
        "fixture must pass the existing PNG encode gate"
    );
    pixel_memory::install();
    let result = crate::extract(crate::ExtractInput::from_bytes(png, "image/png", None), &config).await;
    let rendered = format!("{result:?}");
    crate::plugins::clear_ocr_backends().unwrap();
    let observed = pixel_memory::observation();
    let retained_rgb = 2 * DIMENSION as usize * DIMENSION as usize * 3;
    let image_bytes = retained_rgb + observed.peak + 4 * encoded_len;
    println!(
        "BUILTIN_PIPELINE_PNG_MEMORY={}",
        serde_json::json!({
            "extraction_mib":extraction_mib, "ocr_mib":ocr_mib, "budget":budget,
            "encode_peak":encode_peak, "encoded_len":encoded_len, "retained_rgb":retained_rgb,
            "native_peak":observed.peak, "native_allocations":observed.allocations,
            "native_frees":observed.frees, "native_max_active":observed.max_active,
            "image_bytes":image_bytes, "affordable":affordable
        })
    );
    if affordable {
        let result = result.expect("affordable actual built-in pipeline");
        assert!(result.errors.is_empty(), "{rendered}");
        assert_eq!(result.results.len(), 1);
        fixture::assert_words(&result.results[0].content);
        assert!(observed.allocations > 0 && observed.peak > 0 && observed.max_active >= 5);
        assert_eq!(observed.live, 0);
        assert_eq!(observed.allocations, observed.frees);
        assert!(
            image_bytes <= budget,
            "live image buffers {image_bytes} exceed {budget}"
        );
    } else {
        assert!(
            retained_rgb + observed.peak <= budget,
            "actual pipeline RGB plus native lower bound {} exceeds {}",
            retained_rgb + observed.peak,
            budget
        );
        assert_eq!(observed.allocations, 0, "refusal must precede native image allocation");
        assert!(rendered.contains("max_content_size"), "{rendered}");
    }
}

async fn isolated(name: &str, extraction_mib: usize, ocr_mib: usize, affordable: bool) {
    if std::env::var(CHILD).as_deref() == Ok(name) {
        measure(extraction_mib, ocr_mib, affordable).await;
        return;
    }
    let selector = format!("ocr::processor::execution::reduced_jpeg_memory_tests::encoded_pipeline::{name}");
    let cache = tempfile::tempdir().expect("owned isolated result cache");
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &selector, "--ignored", "--nocapture", "--test-threads=1"])
        .env(CHILD, name)
        .env("XBERG_CACHE_DIR", cache.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success()
            && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;")
            && stdout.contains("BUILTIN_PIPELINE_PNG_MEMORY="),
        "actual built-in PNG child failed: {}\n{stdout}\n{stderr}",
        output.status
    );
    print!("{stdout}");
}

#[tokio::test]
#[ignore = "real OCR; requires verified local XBERG_2339_TESSDATA_DIR"]
async fn builtin_pipeline_png_respects_extraction_limit() {
    isolated("builtin_pipeline_png_respects_extraction_limit", 40, 64, false).await;
}

#[tokio::test]
#[ignore = "real OCR; requires verified local XBERG_2339_TESSDATA_DIR"]
async fn builtin_pipeline_png_respects_ocr_limit() {
    isolated("builtin_pipeline_png_respects_ocr_limit", 64, 40, false).await;
}

#[tokio::test]
#[ignore = "real OCR; requires verified local XBERG_2339_TESSDATA_DIR"]
async fn builtin_pipeline_png_with_affordable_limits_recognizes_actual_text() {
    isolated(
        "builtin_pipeline_png_with_affordable_limits_recognizes_actual_text",
        64,
        64,
        true,
    )
    .await;
}
