use super::{DIMENSION, MIB, pixel_memory};
use crate::types::ImagePreprocessingConfig;
use std::process::Command;

const CHILD_ENV: &str = "XBERG_REDUCED_JPEG_UNSUPPORTED_CHILD";
const TEST_NAME: &str = concat!(
    "ocr::processor::execution::reduced_jpeg_memory_tests::unsupported::",
    "unqualified_reduced_policies_refuse_before_native_allocation"
);

fn policies() -> [ImagePreprocessingConfig; 3] {
    [
        ImagePreprocessingConfig {
            denoise: true,
            ..Default::default()
        },
        ImagePreprocessingConfig {
            binarization_method: "sauvola".into(),
            ..Default::default()
        },
        ImagePreprocessingConfig {
            normalize_shaded_rows: true,
            ..Default::default()
        },
    ]
}

fn prepare(
    bytes: &[u8],
    source_dpi: f64,
    policy: ImagePreprocessingConfig,
) -> crate::Result<super::super::PreparedOcrImage> {
    super::super::prepare_encoded_ocr_image(
        bytes,
        &super::super::SecurityLimits {
            max_content_size: 40 * MIB,
            ..Default::default()
        },
        &super::super::TesseractConfig {
            source_dpi: Some(source_dpi),
            preprocessing: Some(policy),
            ..Default::default()
        },
        None,
        false,
    )
}

fn refusal_controls() {
    let rgb = vec![180; usize::from(DIMENSION).pow(2) * 3];
    let bytes = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, DIMENSION, DIMENSION);
    drop(rgb);
    pixel_memory::install();
    for policy in policies() {
        let error = prepare(&bytes, 600.0, policy)
            .err()
            .expect("unqualified reduced native policy must fail closed");
        let message = error.to_string();
        assert!(message.contains("preprocessing policy"), "{message}");
        assert!(message.contains("max_content_size"), "{message}");
        assert_eq!(pixel_memory::observation().allocations, 0);
    }
    println!("UNQUALIFIED_REDUCED_REFUSALS=3 NATIVE_PIX_ALLOCATIONS=0");
}

#[test]
fn unqualified_reduced_policies_refuse_before_native_allocation() {
    if std::env::var(CHILD_ENV).as_deref() == Ok(TEST_NAME) {
        assert!(std::env::args().any(|arg| arg == "--exact"));
        assert!(std::env::args().any(|arg| arg == "--test-threads=1"));
        refusal_controls();
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST_NAME, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, TEST_NAME)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success()
            && stdout.contains("1 passed; 0 failed; 0 ignored;")
            && stdout.contains("UNQUALIFIED_REDUCED_REFUSALS=3 NATIVE_PIX_ALLOCATIONS=0"),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{stdout}");
}

#[test]
fn ordinary_full_resolution_keeps_optional_preprocessing_policy() {
    let rgb = vec![180; 32 * 32 * 3];
    let bytes = crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&rgb, 32, 32);
    for policy in policies() {
        let expected = serde_json::to_value(&policy).unwrap();
        let prepared = prepare(&bytes, 300.0, policy).expect("full-resolution legacy policy remains available");
        assert_eq!((prepared.width, prepared.height, prepared.source_dpi), (32, 32, 300));
        assert_eq!(serde_json::to_value(prepared.preprocessing.unwrap()).unwrap(), expected);
    }
}
