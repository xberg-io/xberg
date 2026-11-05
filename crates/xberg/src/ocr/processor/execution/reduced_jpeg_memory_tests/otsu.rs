use super::{DIMENSION, MIB, pixel_memory, validate_observation, verify_negative_controls};
use std::process::Command;
use xberg_tesseract::{Pix, TesseractAPI};

const TEST_NAME: &str = "ocr::processor::execution::reduced_jpeg_memory_tests::otsu::default_no_resize_otsu_keeps_live_images_within_40_mib";
const CHILD_ENV: &str = "XBERG_REDUCED_JPEG_OTSU_CHILD";
const BUDGET: usize = 40 * MIB;

fn measure_default_otsu() {
    let rgb = vec![245; usize::from(DIMENSION).pow(2) * 3];
    let bytes = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, DIMENSION, DIMENSION);
    drop(rgb);
    pixel_memory::install();
    let prepared = super::super::prepare_encoded_ocr_image(
        &bytes,
        &super::super::SecurityLimits {
            max_content_size: BUDGET,
            ..Default::default()
        },
        &super::super::TesseractConfig {
            source_dpi: Some(600.0),
            known_full_page_scan: true,
            ..Default::default()
        },
        None,
        false,
    )
    .expect("default policy must preserve the safe no-resize candidate");
    assert_eq!(
        (prepared.width, prepared.height, prepared.source_dpi),
        (2048, 2048, 300)
    );
    assert!(
        prepared
            .image_preprocessing
            .as_ref()
            .expect("DPI metadata")
            .skipped_resize
    );
    let options = prepared.preprocessing.as_ref().expect("actual default policy");
    assert_eq!(
        serde_json::to_value(options).unwrap(),
        serde_json::to_value(crate::types::ImagePreprocessingConfig::default()).unwrap()
    );
    let pix = Pix::from_raw_rgb(&prepared.data, prepared.width, prepared.height).expect("source32");
    let output = super::super::preprocess_pix(pix, options).expect("actual default preprocessing");
    assert_eq!(output.depth(), 1);
    let api = TesseractAPI::new().expect("native API");
    api.init_for_analyse_page().expect("model-free native ingestion");
    api.set_image_2(output.as_ptr()).expect("native preprocessed ingestion");
    drop(api);
    drop(output);
    let observed = pixel_memory::observation();
    let retained = bytes.capacity() + prepared.data.capacity();
    let measured = verify_negative_controls(observed, retained);
    println!(
        "OTSU_PIX_MEMORY_OBSERVATION={}",
        serde_json::json!({
            "encoded_capacity": bytes.capacity(), "rgb_capacity": prepared.data.capacity(),
            "native_peak": observed.peak, "native_allocations": observed.allocations,
            "native_frees": observed.frees, "native_max_active": observed.max_active,
            "measured_lower_bound": measured, "budget": BUDGET, "negative_controls": 2,
        })
    );
    assert!(
        validate_observation(observed, retained, BUDGET).is_ok(),
        "default live image buffers {measured} exceed admitted {BUDGET}"
    );
}

#[test]
fn default_no_resize_otsu_keeps_live_images_within_40_mib() {
    if std::env::var(CHILD_ENV).as_deref() == Ok(TEST_NAME) {
        assert!(std::env::args().any(|arg| arg == "--exact"));
        assert!(std::env::args().any(|arg| arg == "--test-threads=1"));
        measure_default_otsu();
        return;
    }
    let output = Command::new(std::env::current_exe().expect("native executable"))
        .args(["--exact", TEST_NAME, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, TEST_NAME)
        .output()
        .expect("isolated Otsu measurement");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success()
            && stdout.contains("test result: ok. 1 passed; 0 failed; 0 ignored;")
            && stdout.contains("OTSU_PIX_MEMORY_OBSERVATION="),
        "isolated default Otsu failed: {}\n{stdout}\n{stderr}",
        output.status
    );
    print!("{stdout}");
}
