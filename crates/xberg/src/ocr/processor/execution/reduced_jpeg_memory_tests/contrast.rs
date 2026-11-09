use super::{DIMENSION, MIB, pixel_memory, validate_observation, verify_negative_controls};
use crate::core::config::ImageExtractionConfig;
use crate::types::ImagePreprocessingConfig;
use sha2::{Digest, Sha256};
use std::process::Command;
use xberg_tesseract::Pix;

const CHILD_ENV: &str = "XBERG_REDUCED_JPEG_CONTRAST_CHILD";
const TEST_NAME: &str =
    "ocr::processor::execution::reduced_jpeg_memory_tests::contrast::should_keep_no_resize_contrast_within_96_mib";
const BUDGET: usize = 96 * MIB;

fn synthetic_gray_baseline() -> Vec<u8> {
    let original = vec![180; usize::from(DIMENSION).pow(2) * 3];
    crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&original, DIMENSION, DIMENSION)
}

fn config() -> super::super::TesseractConfig {
    super::super::TesseractConfig {
        source_dpi: Some(600.0),
        preprocessing: Some(ImagePreprocessingConfig {
            target_dpi: 300,
            binarization_method: "none".to_string(),
            contrast_enhance: true,
            deskew: false,
            denoise: false,
            auto_rotate: false,
            invert_colors: false,
            normalize_shaded_rows: false,
        }),
        ..Default::default()
    }
}

fn prepare(bytes: &[u8]) -> super::super::PreparedOcrImage {
    let prepared = super::super::prepare_encoded_ocr_image(
        bytes,
        &super::super::SecurityLimits {
            max_content_size: BUDGET,
            ..Default::default()
        },
        &config(),
        Some(&ImageExtractionConfig {
            auto_adjust_dpi: false,
            ..Default::default()
        }),
        false,
    )
    .expect("actual preparation admits known-density no-resize contrast");
    assert!(prepared.apply_pix_preprocessing);
    let metadata = prepared
        .image_preprocessing
        .as_ref()
        .expect("actual DPI normalization metadata");
    assert!(
        metadata.skipped_resize,
        "contrast test must not measure a resampling branch"
    );
    assert_eq!((metadata.final_dpi, prepared.source_dpi), (300, 300));
    assert!(prepared.width > 0 && prepared.width < u32::from(DIMENSION));
    assert_eq!(prepared.width, prepared.height);
    assert_eq!(
        prepared.data.len(),
        prepared.width as usize * prepared.height as usize * 3
    );
    let preprocessing = prepared
        .preprocessing
        .as_ref()
        .expect("retained actual preprocessing policy");
    assert!(preprocessing.contrast_enhance);
    assert_eq!(preprocessing.binarization_method, "none");
    prepared
}

fn apply_contrast(prepared: &super::super::PreparedOcrImage) -> pixel_memory::Observation {
    assert_eq!(
        pixel_memory::observation().allocations,
        0,
        "no Pix before the actual helper"
    );
    let pix = Pix::from_raw_rgb(&prepared.data, prepared.width, prepared.height).expect("actual source32-bpp Pix");
    assert_eq!(pix.depth(), 32);
    let output = super::super::preprocess_pix(pix, prepared.preprocessing.as_ref().unwrap())
        .expect("actual production contrast helper must succeed");
    assert_eq!(
        (output.width(), output.height(), output.depth()),
        (prepared.width as i32, prepared.height as i32, 8)
    );
    drop(output);
    let observed = pixel_memory::observation();
    assert!(
        observed.max_active >= 3,
        "actual contrast must produce concurrent native intermediates"
    );
    observed
}

fn measure_contrast() {
    let bytes = synthetic_gray_baseline();
    let source_sha = Sha256::digest(&bytes);
    assert!(bytes.capacity() < 2 * MIB);
    assert!(
        super::super::native_image_budget::known_phase_peak(
            u32::from(DIMENSION),
            u32::from(DIMENSION),
            bytes.len(),
            config().preprocessing.as_ref(),
        )
        .expect("addressable contrast phase")
        .expect("known contrast policy")
            > BUDGET as u64,
        "full-resolution contrast must require reduced decoding even though its RGB fits",
    );
    pixel_memory::install();
    let prepared = prepare(&bytes);
    let rgb_sha = Sha256::digest(&prepared.data);
    let retained = bytes
        .capacity()
        .checked_add(prepared.data.capacity())
        .expect("retained caller buffer capacities");
    let observed = apply_contrast(&prepared);
    assert_eq!(
        Sha256::digest(&prepared.data),
        rgb_sha,
        "retained RGB stays live and unchanged across preprocessing"
    );
    assert_eq!(
        Sha256::digest(&bytes),
        source_sha,
        "encoded source stays live and unchanged across preprocessing"
    );
    let measured = verify_negative_controls(observed, retained);
    println!(
        "CONTRAST_PIX_MEMORY_OBSERVATION={}",
        serde_json::json!({
            "source_sha256": hex::encode(source_sha), "rgb_sha256": hex::encode(rgb_sha),
            "encoded_capacity": bytes.capacity(), "rgb_capacity": prepared.data.capacity(),
            "width": prepared.width, "height": prepared.height, "source_dpi": prepared.source_dpi,
            "native_allocations": observed.allocations, "native_frees": observed.frees,
            "native_max_active": observed.max_active, "native_peak": observed.peak,
            "native_live_after_drop": observed.live, "measured_lower_bound": measured,
            "budget": BUDGET, "negative_controls": 2,
        })
    );
    assert!(
        validate_observation(observed, retained, BUDGET).is_ok(),
        "actual no-resize contrast image-buffer lower bound {measured} exceeds admitted budget {BUDGET}"
    );
}

#[test]
fn should_keep_no_resize_contrast_within_96_mib() {
    if std::env::var(CHILD_ENV).as_deref() == Ok(TEST_NAME) {
        let args: Vec<_> = std::env::args().collect();
        assert!(args.iter().any(|arg| arg == "--exact"));
        assert!(args.iter().any(|arg| arg == "--test-threads=1"));
        measure_contrast();
        return;
    }
    let output = Command::new(std::env::current_exe().expect("current native test executable"))
        .args(["--exact", TEST_NAME, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, TEST_NAME)
        .output()
        .expect("run one isolated actual contrast measurement");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success()
            && stdout.lines().any(|line| line == "running 1 test")
            && stdout
                .lines()
                .any(|line| line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;")),
        "isolated contrast failed: {}\n{stdout}\n{stderr}",
        output.status
    );
    print!("{stdout}");
    assert!(
        stdout.contains("CONTRAST_PIX_MEMORY_OBSERVATION="),
        "actual native measurement did not fire"
    );
}

/// One byte below the modelled gray-route peak of the 2048 px candidate: the largest limit that candidate
/// cannot be admitted under, so the reduction falls to 1/4 and cannot reach the 300 DPI target. The
/// fixture is uniform gray, so the gray route's model applies. ~keep
fn unaffordable_gray_limit(bytes: &[u8]) -> usize {
    let candidate = u32::from(DIMENSION) / 2;
    let policy = config().preprocessing;
    let peak =
        super::super::native_image_budget::known_gray_phase_peak(candidate, candidate, bytes.len(), policy.as_ref())
            .expect("addressable gray contrast phase")
            .expect("known contrast policy");
    usize::try_from(peak).expect("peak fits usize") - 1
}

#[test]
fn rejects_unaffordable_contrast_before_native_preprocessing() {
    const NAME: &str = "ocr::processor::execution::reduced_jpeg_memory_tests::contrast::rejects_unaffordable_contrast_before_native_preprocessing";
    if std::env::var(CHILD_ENV).as_deref() == Ok(NAME) {
        let bytes = synthetic_gray_baseline();
        let source_sha = Sha256::digest(&bytes);
        pixel_memory::install();
        let result = super::super::prepare_encoded_ocr_image(
            &bytes,
            &super::super::SecurityLimits {
                max_content_size: unaffordable_gray_limit(&bytes),
                ..Default::default()
            },
            &config(),
            Some(&ImageExtractionConfig {
                auto_adjust_dpi: false,
                ..Default::default()
            }),
            false,
        );
        let error = result.err().expect("unaffordable target must be rejected");
        assert!(
            matches!(error, crate::XbergError::Validation { ref message, .. }
                if message.contains("security_limits.max_content_size") && message.contains("target_dpi")),
            "{error}"
        );
        assert_eq!(
            pixel_memory::observation().allocations,
            0,
            "no native Pix was allocated"
        );
        assert_eq!(Sha256::digest(&bytes), source_sha);
        println!("NATIVE_PREFLIGHT_REFUSAL");
        return;
    }
    let output = Command::new(std::env::current_exe().expect("current native executable"))
        .args(["--exact", NAME, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, NAME)
        .output()
        .expect("run isolated refusal control");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success()
            && stdout.contains("NATIVE_PREFLIGHT_REFUSAL")
            && stdout.contains("1 passed; 0 failed; 0 ignored;"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

const GRAY_CHILD: &str =
    "ocr::processor::execution::reduced_jpeg_memory_tests::contrast::gray_route_peak_stays_within_its_model";

fn measure_gray_route() {
    let bytes = synthetic_gray_baseline();
    pixel_memory::install();
    let prepared = prepare(&bytes);
    assert!(
        crate::ocr::preprocessing::rgb_is_gray(&prepared.data),
        "the fixture is genuinely gray"
    );
    let (width, height) = (prepared.width, prepared.height);
    let mut rgb = prepared.data.clone();
    drop(prepared);
    let policy = config().preprocessing.expect("contrast policy");
    assert_eq!(
        pixel_memory::observation().allocations,
        0,
        "no Pix before the gray route"
    );
    let output = crate::ocr::preprocessing::preprocess_gray_contrast(&mut rgb, width, height, &policy)
        .expect("the production gray route must succeed");
    assert_eq!(
        (output.width(), output.height(), output.depth()),
        (width as i32, height as i32, 8)
    );
    assert_eq!(rgb.capacity(), 0, "the RGB is freed before the native phases");
    drop(output);
    let observed = pixel_memory::observation();
    let model = super::super::native_image_budget::gray_route_native_peak(width, height).expect("addressable");
    println!(
        "GRAY_ROUTE_OBSERVATION={}",
        serde_json::json!({
            "width": width, "height": height, "native_peak": observed.peak, "model_native_peak": model,
            "native_max_active": observed.max_active, "live_after_drop": observed.live,
        })
    );
    assert_eq!(observed.live, 0, "native allocations were released");
    assert!(
        observed.peak as u64 <= model,
        "measured {} exceeds the model {model}",
        observed.peak
    );
    assert!(
        observed.peak as u64 * 10 >= model * 9,
        "the model is not tight: measured {} against {model}",
        observed.peak
    );
}

#[test]
fn gray_route_peak_stays_within_its_model() {
    if std::env::var(CHILD_ENV).as_deref() == Ok(GRAY_CHILD) {
        measure_gray_route();
        return;
    }
    let output = Command::new(std::env::current_exe().expect("current native executable"))
        .args(["--exact", GRAY_CHILD, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, GRAY_CHILD)
        .output()
        .expect("run the isolated gray-route measurement");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("1 passed; 0 failed; 0 ignored;"),
        "isolated gray route failed: {}\n{stdout}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("GRAY_ROUTE_OBSERVATION="),
        "the native measurement did not fire"
    );
    print!("{stdout}");
}
