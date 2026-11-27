use super::*;

#[test]
fn default_framework_capabilities_fail_closed() {
    let capabilities = FrameworkCapabilities::default();
    assert_eq!(capabilities.timing_regime, TimingRegime::Unknown);
    assert_eq!(
        capabilities.resource_measurement_scope,
        ResourceMeasurementScope::Unknown
    );
}

#[test]
fn legacy_paddle_pipeline_parses_supported_spellings() {
    for value in ["paddle-ocr", "paddle_ocr", "paddleocr"] {
        assert_eq!(value.parse::<XbergPipeline>(), Ok(XbergPipeline::PaddleOcr));
    }
}

#[test]
fn layout_v3_pipeline_round_trips_through_str_and_serde() {
    for value in ["layout-v3", "layout_v3"] {
        assert_eq!(value.parse::<XbergPipeline>(), Ok(XbergPipeline::LayoutV3));
    }
    assert_eq!(XbergPipeline::LayoutV3.as_str(), "layout-v3");
    assert_eq!(
        serde_json::to_string(&XbergPipeline::LayoutV3).unwrap(),
        "\"layout-v3\""
    );
    assert_eq!(
        serde_json::from_str::<XbergPipeline>("\"layout-v3\"").unwrap(),
        XbergPipeline::LayoutV3
    );
}

#[test]
fn sceptre_pipeline_identities_are_explicit_and_alias_defaults_to_ort() {
    assert_eq!("sceptre".parse(), Ok(XbergPipeline::SceptreOrt));
    for pipeline in [
        XbergPipeline::SceptreOrt,
        XbergPipeline::SceptreOrtLayout,
        XbergPipeline::SceptreOrtAutoRotate,
        XbergPipeline::SceptreTract,
    ] {
        assert_eq!(pipeline.as_str().parse(), Ok(pipeline));
        assert!(pipeline.as_str().contains("ort") || pipeline.as_str().contains("tract"));
    }
}

#[test]
fn pdf_backend_parses_case_and_separator_insensitively() {
    for value in ["native", "Native", "NATIVE"] {
        assert_eq!(value.parse::<XbergPdfBackend>(), Ok(XbergPdfBackend::Native));
    }
    for value in ["pdfium", "Pdfium", "PDFIUM"] {
        assert_eq!(value.parse::<XbergPdfBackend>(), Ok(XbergPdfBackend::Pdfium));
    }
}

#[test]
fn pdf_backend_rejects_unknown_value_and_lists_valid_ones() {
    let error = "pdf_oxide".parse::<XbergPdfBackend>().unwrap_err();
    assert_eq!(error, "unknown Xberg PDF backend 'pdf-oxide'. Valid: native, pdfium");
}

#[test]
fn stage_timings_round_trips_with_ort_field_present() {
    let timings = StageTimings {
        process_init_ms: 12.5,
        first_parse_ms: 1150.25,
        ort_session_and_inference_ms: Some(1150.25),
    };

    let json = serde_json::to_string(&timings).expect("serialize StageTimings");
    let parsed: StageTimings = serde_json::from_str(&json).expect("deserialize StageTimings");

    assert_eq!(parsed.process_init_ms, 12.5);
    assert_eq!(parsed.first_parse_ms, 1150.25);
    assert_eq!(parsed.ort_session_and_inference_ms, Some(1150.25));
}

#[test]
fn stage_timings_omits_ort_field_when_absent() {
    let timings = StageTimings {
        process_init_ms: 8.0,
        first_parse_ms: 235.0,
        ort_session_and_inference_ms: None,
    };

    let json = serde_json::to_string(&timings).expect("serialize StageTimings");

    assert!(
        !json.contains("ort_session_and_inference_ms"),
        "expected ort_session_and_inference_ms to be skipped when None, got: {json}"
    );
}

#[test]
fn stage_timings_parses_from_cli_json_shape() {
    // Mirrors the exact JSON shape emitted by `xberg-cli`'s `output::StageTimings` when
    // XBERG_EMIT_STAGE_TIMING is set and layout/OCR is active. ~keep
    let raw = r#"{
        "process_init_ms": 4.2,
        "first_parse_ms": 1171.0,
        "ort_session_and_inference_ms": 1171.0
    }"#;

    let parsed: StageTimings = serde_json::from_str(raw).expect("parse CLI stage_timings JSON");

    assert_eq!(parsed.process_init_ms, 4.2);
    assert_eq!(parsed.first_parse_ms, 1171.0);
    assert_eq!(parsed.ort_session_and_inference_ms, Some(1171.0));
}

fn minimal_result(success: bool, error_kind: ErrorKind, duration_ms: u64, throughput_bps: f64) -> BenchmarkResult {
    BenchmarkResult {
        framework: "fw".to_string(),
        output_format: OutputFormat::Markdown,
        file_path: PathBuf::from("test.pdf"),
        file_size: 1024,
        success,
        error_message: if success { None } else { Some("failed".to_string()) },
        error_kind,
        duration: Duration::from_millis(duration_ms),
        extraction_duration: None,
        subprocess_overhead: None,
        metrics: PerformanceMetrics {
            baseline_memory_bytes: 0,
            peak_memory_bytes: 1_000_000,
            peak_memory_delta_bytes: 1_000_000,
            avg_cpu_percent: 50.0,
            cpu_seconds: 1.0,
            throughput_bytes_per_sec: throughput_bps,
            p50_memory_bytes: 1_000_000,
            p95_memory_bytes: 1_000_000,
            p99_memory_bytes: 1_000_000,
            ..Default::default()
        },
        quality: None,
        iterations: vec![],
        statistics: None,
        cold_start_duration: None,
        file_extension: "pdf".to_string(),
        framework_capabilities: FrameworkCapabilities::default(),
        pdf_metadata: None,
        ocr_status: OcrStatus::NotUsed,
        extracted_text: None,
        system_load: None,
        fixture_family: None,
    }
}

#[test]
fn error_kind_zero_overlap_round_trips_through_json() {
    let json = serde_json::to_string(&ErrorKind::ZeroOverlap).expect("serialize ErrorKind::ZeroOverlap");
    assert_eq!(json, "\"zero_overlap\"");

    let parsed: ErrorKind = serde_json::from_str(&json).expect("deserialize ErrorKind::ZeroOverlap");
    assert_eq!(parsed, ErrorKind::ZeroOverlap);
}

/// The Defect A fix: a zero-overlap result is not a quality success, but it is still
/// timing-eligible, so it must survive into the performance-sample set alongside genuine
/// successes. A genuinely-empty result (`ErrorKind::EmptyContent`) is not timing-eligible and
/// stays excluded, proving the two `success == false` cases are treated differently on
/// purpose rather than both being swept up by a broad `!success` check.
#[test]
fn successful_performance_samples_includes_zero_overlap_but_not_empty_content() {
    let success = minimal_result(true, ErrorKind::None, 100, 1_000_000.0);
    let zero_overlap = minimal_result(false, ErrorKind::ZeroOverlap, 200, 2_000_000.0);
    let empty_content = minimal_result(false, ErrorKind::EmptyContent, 300, 3_000_000.0);

    let results = [success, zero_overlap, empty_content];
    let samples = successful_performance_samples(results.iter());

    assert_eq!(
        samples.len(),
        2,
        "expected the success and the zero-overlap result, got {samples:?}"
    );
    assert!(samples.iter().any(|r| r.error_kind == ErrorKind::None));
    assert!(samples.iter().any(|r| r.error_kind == ErrorKind::ZeroOverlap));
    assert!(!samples.iter().any(|r| r.error_kind == ErrorKind::EmptyContent));
}

#[test]
fn is_timing_eligible_matches_success_or_zero_overlap() {
    assert!(is_timing_eligible(&minimal_result(true, ErrorKind::None, 100, 1.0)));
    assert!(is_timing_eligible(&minimal_result(
        false,
        ErrorKind::ZeroOverlap,
        100,
        1.0
    )));
    assert!(!is_timing_eligible(&minimal_result(
        false,
        ErrorKind::EmptyContent,
        100,
        1.0
    )));
    assert!(!is_timing_eligible(&minimal_result(
        false,
        ErrorKind::FrameworkError,
        100,
        1.0
    )));
    assert!(!is_timing_eligible(&minimal_result(
        false,
        ErrorKind::HarnessError,
        100,
        1.0
    )));
    assert!(!is_timing_eligible(&minimal_result(
        false,
        ErrorKind::Timeout,
        100,
        1.0
    )));
    assert!(!is_timing_eligible(&minimal_result(
        false,
        ErrorKind::ConfigSetupError,
        100,
        1.0
    )));
}
