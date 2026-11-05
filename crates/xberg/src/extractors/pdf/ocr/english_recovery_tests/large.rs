const MIB: usize = 1024 * 1024;
use super::super::{DetachedOcrPage, PageOcrHints, run_ocr_pipeline_for_page};
use crate::core::config::ExtractionConfig;

#[allow(
    unsafe_code,
    reason = "Isolated tests detect original-size Rust image allocations through System"
)]
#[path = "large/allocation_sizes.rs"]
mod allocation_sizes;
use super::{fixture, model};
use crate::ocr::processor::test_pixel_memory as pixel_memory;
#[path = "large/source.rs"]
mod source;

const CHILD: &str = "XBERG_LARGE_JPEG_ENGLISH_CHILD";

fn config(mib: usize, model: &std::path::Path) -> ExtractionConfig {
    let mut config: ExtractionConfig = serde_json::from_value(serde_json::json!({
        "use_cache":false, "force_ocr":true, "enable_quality_processing":false,
        "ocr": {"backend":"tesseract", "language":["eng"],
            "backend_options":{"source_dpi":600,"use_cache":false},
            "tesseract_config":{"psm":6,"preprocessing":{
                "target_dpi":300,"binarization_method":"none","contrast_enhance":true,
                "auto_rotate":false,"deskew":false,"denoise":false,"invert_colors":false,
                "normalize_shaded_rows":false
            }},
            "pipeline":{"stages":[{"backend":"tesseract","priority":1}],
                "quality_thresholds":{"pipeline_min_quality":0.0}}
        }
    }))
    .unwrap();
    let limits = crate::extractors::security::SecurityLimits {
        max_content_size: mib * MIB,
        ..Default::default()
    };
    config.security_limits = Some(limits.clone());
    let ocr = config.ocr.as_mut().unwrap();
    ocr.security_limits = Some(limits);
    ocr.tessdata_path = Some(model.to_path_buf());
    config
}

async fn extract(route: &str, source: &source::Source, config: &ExtractionConfig) -> crate::Result<String> {
    if route == "detached" {
        let images = [source.preview.clone()];
        let pipeline = config.ocr.as_ref().unwrap().pipeline.as_ref().unwrap();
        let result = run_ocr_pipeline_for_page(
            None,
            Some(&images),
            #[cfg(feature = "layout-detection")]
            None,
            config,
            pipeline,
            None,
            0,
            false,
            Some(72.0 / 150.0),
            Some(PageOcrHints {
                source_dpi: Some(150.0),
                known_full_page_scan: true,
                ..Default::default()
            }),
            0,
            Some(DetachedOcrPage {
                document: &source.document,
                #[cfg(all(paddle_ocr, feature = "ocr", not(target_arch = "wasm32")))]
                stored_scan: None,
                #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
                orientation_retry: None,
            }),
        )
        .await?;
        assert_eq!(result.5.len(), 1);
        return Ok(result.0);
    }
    let (bytes, mime) = if route == "pdf" {
        (&source.pdf, "application/pdf")
    } else {
        (&source.jpeg, "image/jpeg")
    };
    let result = crate::extract(crate::ExtractInput::from_bytes(bytes.clone(), mime, None), config).await?;
    if !result.errors.is_empty() {
        return Err(crate::XbergError::validation(format!(
            "actual public errors: {:?}",
            result.errors
        )));
    }
    assert_eq!(result.results.len(), 1);
    Ok(result.results.into_iter().next().unwrap().content)
}

async fn reject_truncated_pipeline_payload(source: &source::Source, model: &std::path::Path) {
    let scan = source
        .jpeg
        .windows(2)
        .position(|pair| pair == [0xff, 0xda])
        .expect("baseline scan marker");
    let truncated = source.jpeg[..scan].to_vec();
    let config = config(512, model);
    let metadata = crate::extraction::image::extract_image_metadata_for_processing(
        &truncated,
        config.security_limits.as_ref().unwrap(),
        |width, height| {
            crate::extractors::pdf::ocr::estimate_png_encode_page_peak_bytes(width, height)
                .is_ok_and(|peak| peak <= (512 * MIB) as u64)
        },
    )
    .expect("header admission remains separate from actual payload validation");
    assert_eq!(
        (metadata.width, metadata.height),
        (u32::from(source::WIDTH), u32::from(source::HEIGHT))
    );
    let error = crate::extract(crate::ExtractInput::from_bytes(truncated, "image/jpeg", None), &config)
        .await
        .expect_err("actual pipeline must reject truncated admitted JPEG");
    assert!(error.to_string().contains("JPEG"), "{error}");
    println!("ADMITTED_TRUNCATED_JPEG_REJECTED={error}");
}

fn report(
    route: &str,
    mib: usize,
    source: &source::Source,
    result: &crate::Result<String>,
    counters: (usize, usize, usize),
    observed: pixel_memory::Observation,
) {
    let (full_allocations, max_rust_allocation, half_allocations) = counters;
    let has_words = result.as_ref().is_ok_and(|text| fixture::has_words(text));
    println!(
        "EXACT_LARGE_ENGLISH={}",
        serde_json::json!({
            "route":route,"mib":mib,"width":source::WIDTH,"height":source::HEIGHT,
            "jpeg_sha256":fixture::sha(&source.jpeg),"pdf_sha256":fixture::sha(&source.pdf),
            "encoded_bytes":source.jpeg.len(),"text":result.as_ref().ok(),
            "error":result.as_ref().err().map(ToString::to_string),"has_words":has_words,
            "full_raster_allocations":full_allocations,"max_rust_allocation":max_rust_allocation,
            "half_raster_allocations":half_allocations,
            "native_peak":observed.peak,"native_allocations":observed.allocations,"native_frees":observed.frees,
            "native_max_active":observed.max_active,
        })
    );
}

async fn qualify(route: &str) {
    pixel_memory::install();
    let model = model::verified_model_directory();
    let source = source::create();
    if route == "image" {
        reject_truncated_pipeline_payload(&source, &model).await;
    }
    assert_eq!(source::ORIGINAL_RGB, 190_423_203);
    allocation_sizes::verify_detector(source::ORIGINAL_RGB);
    let mut succeeded = false;
    for mib in [40, 64, 512] {
        let config = config(mib, &model);
        allocation_sizes::start(source::ORIGINAL_RGB, source::HALF_RGB);
        let result = extract(route, &source, &config).await;
        crate::plugins::clear_ocr_backends().unwrap();
        let (full_allocations, max_rust_allocation, half_allocations) = allocation_sizes::finish();
        let observed = pixel_memory::observation();
        assert_eq!(observed.live, 0, "native image ownership released");
        assert_eq!(observed.allocations, observed.frees);
        let has_words = result.as_ref().is_ok_and(|text| fixture::has_words(text));
        report(
            route,
            mib,
            &source,
            &result,
            (full_allocations, max_rust_allocation, half_allocations),
            observed,
        );
        if route != "pdf" || mib < 512 {
            assert_eq!(
                full_allocations, 0,
                "must not allocate original RGB/RGBA: {route}/{mib}"
            );
        }
        assert!(
            !succeeded || has_words,
            "higher limit must retain successful recognition"
        );
        succeeded |= has_words;
        if mib == 512 {
            if route == "image" {
                assert!(half_allocations > 0, "actual safe half raster must fire detector");
            }
            fixture::assert_words(result.as_ref().expect("512MiB real public route"));
        }
    }
    model.verify();
}

async fn isolated(route: &str, selector: &str) {
    if std::env::var(CHILD).as_deref() == Ok(route) {
        qualify(route).await;
        return;
    }
    let selector = selector.strip_prefix("xberg::").unwrap_or(selector);
    let cache = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", selector, "--ignored", "--nocapture", "--test-threads=1"])
        .env(CHILD, route)
        .env("XBERG_CACHE_DIR", cache.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success()
            && stdout.contains("1 passed; 0 failed; 0 ignored;")
            && stdout.matches("EXACT_LARGE_ENGLISH=").count() == 3,
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    print!("{stdout}");
}

#[tokio::test]
#[ignore = "large model-backed OCR; covered by rust:test:jpeg-recovery"]
async fn large_english_image() {
    isolated("image", concat!(module_path!(), "::large_english_image")).await;
}

#[tokio::test]
#[ignore = "large model-backed OCR; covered by rust:test:jpeg-recovery"]
async fn large_english_detached() {
    isolated("detached", concat!(module_path!(), "::large_english_detached")).await;
}

#[path = "large/pdf_route_tests.rs"]
mod pdf_route_tests;

#[path = "large/purity.rs"]
mod purity;
