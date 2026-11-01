use super::{DIMENSION, MIB, ingest, pixel_memory, synthetic_baseline, validate_observation, verify_negative_controls};
use sha2::{Digest, Sha256};
use std::process::Command;

const BUDGET: usize = 64 * MIB;
const CHILD_ENV: &str = "XBERG_REDUCED_JPEG_MONOTONIC_CHILD";
const TEST_NAME: &str = "ocr::processor::execution::reduced_jpeg_memory_tests::monotonicity::should_keep_the_finest_safe_raster_when_full_rgb_fits_64_mib";
const SOURCE_SHA: &str = "85bc1b1a55af6794f58da05b32b540301a731f99d48825de6811401d8fcabec3";

fn measure_larger_budget() {
    let bytes = synthetic_baseline();
    let source_sha = Sha256::digest(&bytes);
    assert_eq!(
        hex::encode(source_sha),
        SOURCE_SHA,
        "use the identical source from the qualified40/46MiB controls"
    );
    let original_pixels = usize::from(DIMENSION).pow(2);
    assert!(
        original_pixels * 3 + bytes.capacity() < BUDGET,
        "full RGB must cross the legacy fitting shortcut"
    );
    // The pinned raw SetImage owns RGB plus two32-bpp Pix buffers; these are
    // fixture-validity lower bounds, not the production admission formula. ~keep
    assert!(original_pixels * (3 + 4 + 4) + bytes.capacity() > BUDGET);
    let half = u32::from(DIMENSION) / 2;
    assert!((half as usize).pow(2) * (3 + 4 + 4) + bytes.capacity() < BUDGET);
    pixel_memory::install();
    let prepared = super::super::prepare_encoded_ocr_image(
        &bytes,
        &super::super::SecurityLimits {
            max_content_size: BUDGET,
            ..Default::default()
        },
        &super::super::TesseractConfig::default(),
        None,
        false,
    )
    .expect("larger64MiB budget must recover the finest phase-safe raster from this valid same-source JPEG");
    assert_eq!(
        (prepared.width, prepared.height),
        (half, half),
        "larger budget must preserve the finest safe2048-square result, rather than unsafe full raster or needless coarsening"
    );
    assert!(!prepared.apply_pix_preprocessing);
    assert_eq!(prepared.data.len(), (half as usize).pow(2) * 3);
    let rgb_sha = Sha256::digest(&prepared.data);
    let retained = bytes
        .capacity()
        .checked_add(prepared.data.capacity())
        .expect("retained actual caller capacities");
    let observed = ingest(&prepared);
    assert_eq!(
        Sha256::digest(&prepared.data),
        rgb_sha,
        "RGB remains live across native ingestion"
    );
    assert_eq!(
        Sha256::digest(&bytes),
        source_sha,
        "encoded bytes remain live and unchanged"
    );
    let measured = verify_negative_controls(observed, retained);
    println!(
        "MONOTONIC_PIX_MEMORY_OBSERVATION={}",
        serde_json::json!({
            "source_sha256": SOURCE_SHA, "width": prepared.width, "height": prepared.height,
            "encoded_capacity": bytes.capacity(), "rgb_capacity": prepared.data.capacity(),
            "native_allocations": observed.allocations, "native_frees": observed.frees,
            "native_peak": observed.peak, "native_max_active": observed.max_active,
            "native_live_after_drop": observed.live, "measured_lower_bound": measured,
            "budget": BUDGET, "negative_controls": 2,
        })
    );
    assert!(
        validate_observation(observed, retained, BUDGET).is_ok(),
        "larger64MiB budget must bound actual recovered native image buffers: {measured}"
    );
}

#[test]
fn should_keep_the_finest_safe_raster_when_full_rgb_fits_64_mib() {
    if std::env::var(CHILD_ENV).as_deref() == Ok(TEST_NAME) {
        let args: Vec<_> = std::env::args().collect();
        assert!(args.iter().any(|arg| arg == "--exact"));
        assert!(args.iter().any(|arg| arg == "--test-threads=1"));
        measure_larger_budget();
        return;
    }
    let output = Command::new(std::env::current_exe().expect("native test executable"))
        .args(["--exact", TEST_NAME, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, TEST_NAME)
        .output()
        .expect("run isolated actual larger-budget recovery");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success()
            && stdout.lines().any(|line| line == "running 1 test")
            && stdout
                .lines()
                .any(|line| line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;")),
        "actual larger-budget recovery failed: {}\n{stdout}\n{stderr}",
        output.status
    );
    print!("{stdout}");
    assert!(
        stdout.contains("MONOTONIC_PIX_MEMORY_OBSERVATION="),
        "native lower-bound measurement did not run"
    );
}
