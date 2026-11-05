use super::{DIMENSION, MIB, pixel_memory, validate_observation, verify_negative_controls};
use crate::types::ImagePreprocessingConfig;
use std::process::Command;
use xberg_tesseract::Pix;

const CHILD_ENV: &str = "XBERG_REDUCED_JPEG_ADAPTIVE_CHILD";

fn config() -> super::super::TesseractConfig {
    super::super::TesseractConfig {
        source_dpi: Some(600.0),
        preprocessing: Some(ImagePreprocessingConfig {
            target_dpi: 300,
            binarization_method: "adaptive".into(),
            contrast_enhance: false,
            deskew: false,
            denoise: false,
            auto_rotate: false,
            invert_colors: false,
            normalize_shaded_rows: false,
        }),
        ..Default::default()
    }
}

fn measure(budget: usize, refusal: bool) {
    let rgb = vec![180; usize::from(DIMENSION).pow(2) * 3];
    let bytes = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, DIMENSION, DIMENSION);
    drop(rgb);
    pixel_memory::install();
    let result = super::super::prepare_encoded_ocr_image(
        &bytes,
        &super::super::SecurityLimits {
            max_content_size: budget,
            ..Default::default()
        },
        &config(),
        None,
        false,
    );
    if let Err(error) = result {
        assert!(refusal, "affordable adaptive target must succeed: {error}");
        assert!(error.to_string().contains("max_content_size"));
        assert!(error.to_string().contains("target_dpi"));
        assert_eq!(pixel_memory::observation().allocations, 0);
        println!("ADAPTIVE_NATIVE_REFUSAL=0");
        return;
    }
    measure_native(bytes.capacity(), &result.unwrap(), budget);
    assert!(!refusal, "unaffordable target must be refused before native allocation");
}

fn measure_native(encoded_capacity: usize, prepared: &super::super::PreparedOcrImage, budget: usize) {
    assert_eq!(
        (prepared.width, prepared.height, prepared.source_dpi),
        (2048, 2048, 300)
    );
    assert!(prepared.image_preprocessing.as_ref().unwrap().skipped_resize);
    let pix = Pix::from_raw_rgb(&prepared.data, prepared.width, prepared.height).unwrap();
    let output = super::super::preprocess_pix(pix, prepared.preprocessing.as_ref().unwrap()).unwrap();
    assert_eq!(output.depth(), 1);
    drop(output);
    let observed = pixel_memory::observation();
    let retained = encoded_capacity + prepared.data.capacity();
    let measured = verify_negative_controls(observed, retained);
    println!(
        "ADAPTIVE_NATIVE_OBSERVATION={}",
        serde_json::json!({
            "native_peak": observed.peak, "native_allocations": observed.allocations,
            "native_frees": observed.frees, "measured_lower_bound": measured,
            "budget": budget, "negative_controls": 2,
        })
    );
    assert!(
        validate_observation(observed, retained, budget).is_ok(),
        "admitted adaptive live image buffers {measured} exceed {budget}"
    );
}

fn isolated(name: &str, budget: usize, refusal: bool) {
    let name = name.strip_prefix("xberg::").unwrap_or(name);
    if std::env::var(CHILD_ENV).as_deref() == Ok(name) {
        assert!(std::env::args().any(|arg| arg == "--exact"));
        assert!(std::env::args().any(|arg| arg == "--test-threads=1"));
        measure(budget, refusal);
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, name)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success()
            && stdout.contains("1 passed; 0 failed; 0 ignored;")
            && stdout.contains(if refusal {
                "ADAPTIVE_NATIVE_REFUSAL=0"
            } else {
                "ADAPTIVE_NATIVE_OBSERVATION="
            }),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{stdout}");
}

#[test]
fn adaptive_refuses_unaffordable_target_before_native_allocation() {
    isolated(
        concat!(
            module_path!(),
            "::adaptive_refuses_unaffordable_target_before_native_allocation"
        ),
        28 * MIB,
        true,
    );
}

#[test]
fn adaptive_preserves_affordable_reduced_target_with_measured_workspace() {
    isolated(
        concat!(
            module_path!(),
            "::adaptive_preserves_affordable_reduced_target_with_measured_workspace"
        ),
        40 * MIB,
        false,
    );
}
