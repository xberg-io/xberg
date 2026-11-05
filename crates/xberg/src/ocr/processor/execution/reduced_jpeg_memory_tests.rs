mod adaptive;
mod contrast;
#[cfg(feature = "pdf")]
mod encoded_pipeline;
mod monotonicity;
mod otsu;
mod unsupported;

#[allow(
    unsafe_code,
    reason = "Isolated native test measures real Leptonica pixel allocations through its C hook"
)]
mod pixel_memory;

use super::{SecurityLimits, TesseractConfig, prepare_encoded_ocr_image};
use pixel_memory::Observation;
use sha2::{Digest, Sha256};
use std::process::Command;
use xberg_tesseract::TesseractAPI;

const DIMENSION: u16 = 4096;
const MIB: usize = 1024 * 1024;
const CHILD_ENV: &str = "XBERG_REDUCED_JPEG_MEMORY_CHILD";

fn isolated(name: &str, budget: usize) {
    if std::env::var(CHILD_ENV).as_deref() == Ok(name) {
        let args: Vec<_> = std::env::args().collect();
        assert!(args.iter().any(|arg| arg == "--exact"));
        assert!(args.iter().any(|arg| arg == "--test-threads=1"));
        measure_raw_ingestion(budget);
        return;
    }
    let test = format!("ocr::processor::execution::reduced_jpeg_memory_tests::{name}");
    let output = Command::new(std::env::current_exe().expect("locate native test executable"))
        .args(["--exact", &test, "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, name)
        .output()
        .expect("run isolated pixel-memory test");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success()
            && stdout.lines().any(|line| line == "running 1 test")
            && stdout
                .lines()
                .any(|line| line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;")),
        "isolated image-memory test failed: {}\n{stdout}\n{stderr}",
        output.status,
    );
    print!("{stdout}");
    assert!(
        stdout.contains("PIX_MEMORY_OBSERVATION="),
        "native measurement did not fire"
    );
}

fn synthetic_baseline() -> Vec<u8> {
    let original = vec![0; usize::from(DIMENSION).pow(2) * 3];
    crate::extraction::image::jpeg_fixture::baseline_rgb_jpeg(&original, DIMENSION, DIMENSION)
}

fn validate_observation(observed: Observation, retained: usize, budget: usize) -> Result<usize, &'static str> {
    if observed.allocations == 0 || observed.peak == 0 || observed.max_active < 2 {
        return Err("missing concurrent native pixel allocations");
    }
    if observed.live != 0 || observed.allocations != observed.frees {
        return Err("native pixel allocations were not released");
    }
    let measured = retained
        .checked_add(observed.peak)
        .ok_or("image-memory accounting overflow")?;
    if measured > budget {
        return Err("live image buffers exceed budget");
    }
    Ok(measured)
}

fn verify_negative_controls(observed: Observation, retained: usize) -> usize {
    let measured = validate_observation(observed, retained, usize::MAX).expect("nonzero valid native observation");
    assert_eq!(
        validate_observation(
            Observation {
                allocations: 0,
                ..observed
            },
            retained,
            usize::MAX
        ),
        Err("missing concurrent native pixel allocations"),
    );
    assert_eq!(
        validate_observation(observed, retained, measured - 1),
        Err("live image buffers exceed budget"),
    );
    measured
}

fn measure_raw_ingestion(budget: usize) {
    let bytes = synthetic_baseline();
    let input_sha = Sha256::digest(&bytes);
    assert!(
        bytes.capacity() < 2 * MIB,
        "fixture encoded capacity must leave measured positive-control headroom"
    );
    assert!(
        usize::from(DIMENSION).pow(2) * 3 > budget,
        "fixture must require reduced decoding"
    );
    pixel_memory::install();
    let prepared = prepare_encoded_ocr_image(
        &bytes,
        &SecurityLimits {
            max_content_size: budget,
            ..Default::default()
        },
        &TesseractConfig::default(),
        None,
        false,
    )
    .expect("actual production preparation must recover a reduced raster");
    assert!(prepared.width > 0 && prepared.width < u32::from(DIMENSION));
    assert_eq!(prepared.width, prepared.height);
    assert_eq!(
        prepared.data.len(),
        prepared.width as usize * prepared.height as usize * 3
    );
    assert!(
        !prepared.apply_pix_preprocessing,
        "dark fixture must exercise raw image ingestion"
    );
    let retained = bytes
        .capacity()
        .checked_add(prepared.data.capacity())
        .expect("retained buffer accounting");
    let observed = ingest(&prepared);
    assert_eq!(
        Sha256::digest(&bytes),
        input_sha,
        "encoded source must remain unchanged"
    );
    let measured = verify_negative_controls(observed, retained);
    println!(
        "PIX_MEMORY_OBSERVATION={}",
        serde_json::json!({
            "source_sha256": hex::encode(input_sha), "encoded_capacity": bytes.capacity(),
            "rgb_capacity": prepared.data.capacity(), "width": prepared.width, "height": prepared.height,
            "native_allocations": observed.allocations, "native_frees": observed.frees,
            "native_max_active": observed.max_active, "native_peak": observed.peak,
            "native_live_after_drop": observed.live, "measured_lower_bound": measured, "budget": budget,
            "negative_controls": 2,
        })
    );
    assert!(
        validate_observation(observed, retained, budget).is_ok(),
        "actual live image-buffer lower bound {measured} exceeds admitted budget {budget}"
    );
}

fn ingest(prepared: &super::PreparedOcrImage) -> Observation {
    let api = TesseractAPI::new().expect("create actual native API");
    api.init_for_analyse_page()
        .expect("initialize without traineddata or recognition");
    api.set_image(
        &prepared.data,
        prepared.width as i32,
        prepared.height as i32,
        3,
        (prepared.width * 3) as i32,
    )
    .expect("actual native raw-image ingestion");
    drop(api);
    pixel_memory::observation()
}

#[test]
fn should_keep_raw_native_ingestion_within_40_mib() {
    isolated("should_keep_raw_native_ingestion_within_40_mib", 40 * MIB);
}

#[test]
fn should_accept_the_same_synthetic_source_with_46_mib() {
    isolated("should_accept_the_same_synthetic_source_with_46_mib", 46 * MIB);
}
