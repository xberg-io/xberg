//! Explicit model-backed fixture qualification, without native allocation hooks. ~keep
#[path = "english_recovery_tests/fixture.rs"]
mod fixture;
#[path = "english_recovery_tests/observer.rs"]
mod observer;

use super::{PageOcrHints, run_ocr_pipeline_for_page};
use crate::core::config::{ExtractionConfig, OcrConfig, OcrPipelineConfig};
use crate::extractors::security::SecurityLimits;
use fixture::{assert_words, sha};
use std::path::PathBuf;

const CHILD: &str = "XBERG_2339_ENGLISH_CHILD";
const MODEL_ENV: &str = "XBERG_2339_TESSDATA_DIR";
const MODEL_SHA: &str = "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2";

fn verified_model_directory() -> PathBuf {
    let directory = PathBuf::from(std::env::var_os(MODEL_ENV).expect("explicit verified local tessdata required"));
    let bytes = std::fs::read(directory.join("eng.traineddata")).expect("no model download or missing-model skip");
    assert_eq!(bytes.len(), 4_113_088);
    assert_eq!(sha(&bytes), MODEL_SHA);
    directory
}

fn config(cap: usize, directory: &std::path::Path) -> (ExtractionConfig, OcrPipelineConfig) {
    let pipeline: OcrPipelineConfig = serde_json::from_value(serde_json::json!({
        "stages": [{"backend":"tesseract", "priority":1}]
    }))
    .unwrap();
    let mut ocr: OcrConfig = serde_json::from_value(serde_json::json!({
        "backend":"tesseract", "language":["eng"],
        "backend_options":{"source_dpi":600,"use_cache":false},
        "tesseract_config":{"psm":6,"preprocessing":{
            "target_dpi":300,"auto_rotate":false,"deskew":false,"denoise":false,
            "contrast_enhance":true,"binarization_method":"none","invert_colors":false,
            "normalize_shaded_rows":false
        }}
    }))
    .unwrap();
    ocr.tessdata_path = Some(directory.to_path_buf());
    ocr.pipeline = Some(pipeline.clone());
    ocr.security_limits = Some(SecurityLimits {
        max_content_size: cap,
        ..Default::default()
    });
    (
        ExtractionConfig {
            use_cache: false,
            force_ocr: true,
            security_limits: ocr.security_limits.clone(),
            ocr: Some(ocr),
            ..Default::default()
        },
        pipeline,
    )
}

async fn qualify(detached: bool) {
    let model = verified_model_directory();
    let fixture = fixture::fixture();
    println!("REAL_ENGLISH_FIXTURE {}", fixture.receipt);
    let records = observer::install();
    for mib in [40, 64] {
        let cap = mib * 1024 * 1024;
        let (config, pipeline) = config(cap, &model);
        records.lock().unwrap().clear();
        let text = if detached {
            let images = [fixture::detached_image(&fixture.doc)];
            let points_per_pixel = 96.0 / images[0].height() as f32;
            let hints = PageOcrHints {
                source_dpi: Some(150.0),
                known_full_page_scan: true,
                single_block_pages: None,
                native_page_layers: None,
            };
            let result = run_ocr_pipeline_for_page(
                None,
                Some(&images),
                #[cfg(feature = "layout-detection")]
                None,
                &config,
                &pipeline,
                None,
                0,
                false,
                Some(points_per_pixel),
                Some(hints),
                0,
                Some(super::DetachedOcrPage {
                    document: &fixture.doc,
                    #[cfg(all(auto_rotate, feature = "ocr", not(target_arch = "wasm32")))]
                    orientation_retry: None,
                }),
            )
            .await
            .expect("actual detached production pipeline, no supplied text");
            assert_eq!(result.5.len(), 1, "one original page, no cross-page recovery");
            assert_words(&result.5[0]);
            result.0
        } else {
            let result = crate::extract(
                crate::ExtractInput::from_bytes(fixture.pdf.clone(), "application/pdf", None),
                &config,
            )
            .await
            .expect("actual public PDF extraction");
            assert!(
                result.errors.is_empty(),
                "public extraction errors: {:?}",
                result.errors
            );
            assert_eq!(result.results.len(), 1);
            result.results.into_iter().next().unwrap().content
        };
        assert_words(&text);
        observer::assert_policy(&records, cap, &model);
        println!(
            "REAL_ENGLISH_OBSERVATION {}",
            serde_json::json!({"detached":detached,"cap":cap,"model_sha256":MODEL_SHA,
                "fixture":fixture.receipt,"text":text,"calls":*records.lock().unwrap()})
        );
    }
    assert_eq!(verified_model_directory(), model);
    assert_eq!(sha(&fixture.pdf), fixture.receipt["image_pdf_sha256"].as_str().unwrap());
}

async fn isolated(detached: bool, name: &str) {
    if std::env::var(CHILD).as_deref() == Ok(name) {
        qualify(detached).await;
        return;
    }
    let selector = format!("extractors::pdf::ocr::pipeline::english_recovery_tests::{name}");
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &selector, "--ignored", "--nocapture", "--test-threads=1"])
        .env(CHILD, name)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    print!("{stdout}");
    eprint!("{stderr}");
    assert!(
        output.status.success(),
        "actual English child failed: {stdout}\n{stderr}"
    );
    assert!(stdout.contains("running 1 test"));
    assert!(stdout.contains("1 passed; 0 failed; 0 ignored"));
    assert_eq!(stdout.matches("REAL_ENGLISH_OBSERVATION ").count(), 2);
}

#[tokio::test]
#[ignore = "manual real OCR; requires explicit verified XBERG_2339_TESSDATA_DIR"]
async fn small_english_image_only_pdf_returns_real_text_at_both_caps() {
    isolated(false, "small_english_image_only_pdf_returns_real_text_at_both_caps").await;
}

#[tokio::test]
#[ignore = "manual real OCR; requires explicit verified XBERG_2339_TESSDATA_DIR"]
async fn small_english_detached_page_returns_real_text_at_both_caps() {
    isolated(true, "small_english_detached_page_returns_real_text_at_both_caps").await;
}
