use super::*;
use xberg::layout::{BBox, DetectionResult, LayoutClass, LayoutDetection};

#[path = "input_tests.rs"]
mod inputs;

fn identity(detector: Detector, digest: char) -> ModelIdentity {
    ModelIdentity {
        detector,
        sha256: digest.to_string().repeat(64),
        artifact_bytes: 128,
        execution_provider: "cpu".to_string(),
        intra_op_threads: 1,
        inter_op_threads: 1,
        load_ms: 1.0,
    }
}

fn measurement() -> DetectorMeasurement {
    DetectorMeasurement::from_detection(
        DetectionResult {
            page_width: 100,
            page_height: 200,
            detections: vec![
                LayoutDetection {
                    class_name: LayoutClass::Formula,
                    confidence: 0.9,
                    bbox: BBox {
                        x1: 5.0,
                        y1: 10.0,
                        x2: 30.0,
                        y2: 40.0,
                    },
                },
                LayoutDetection {
                    class_name: LayoutClass::Formula,
                    confidence: 0.8,
                    bbox: BBox {
                        x1: 50.0,
                        y1: 60.0,
                        x2: 70.0,
                        y2: 80.0,
                    },
                },
            ],
        },
        2.0,
    )
    .unwrap()
}

fn report() -> ModelBenchmarkReport {
    let document_sha256 = "c".repeat(64);
    let fixture_sha256 = "d".repeat(64);
    ModelBenchmarkReport {
        schema_version: 2,
        quality_assessed: false,
        geometry: None,
        xberg_build_id: "bounded-contract-test".to_string(),
        cohort_sha256: "e".repeat(64),
        expected_documents: BTreeMap::from([(
            "page".to_string(),
            ExpectedDocument {
                pages: 1,
                document_sha256: document_sha256.clone(),
                fixture_sha256: fixture_sha256.clone(),
            },
        )]),
        expected_pages: 1,
        expected_inference_calls: 2,
        actual_inference_calls: 2,
        iterations: 1,
        max_pages: 1,
        dpi: 150,
        confidence_threshold: 0.5,
        apply_heuristics: false,
        models: [identity(Detector::RtDetr, 'a'), identity(Detector::PpDocLayoutV3, 'b')],
        resources: Some(super::resources::valid_test_resources()),
        rows: vec![DetectorPageRow {
            document: "page".to_string(),
            document_sha256,
            fixture_sha256,
            page: 1,
            iteration: 1,
            render_sha256: "f".repeat(64),
            a_first: true,
            measurements: [measurement(), measurement()],
        }],
    }
}

#[test]
fn should_count_real_canonical_detections_and_roundtrip_the_report() {
    let report = report();
    assert_eq!(
        report.rows[0].measurements[0].canonical_class_counts,
        BTreeMap::from([("formula".to_string(), 2)])
    );
    assert_eq!(report.rows[0].measurements[0].detections.len(), 2);
    report.validate().unwrap();
    let encoded = serde_json::to_vec(&report).unwrap();
    let decoded: ModelBenchmarkReport = serde_json::from_slice(&encoded).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded.actual_inference_calls, 2);
}

#[test]
fn should_reject_unknown_and_table_model_names_before_loading_any_model() {
    for name in ["unknown", "tatr", "slanet_auto", "", "RTDETR"] {
        let config = ModelBenchmarkConfig {
            model_a: name.to_string(),
            ..Default::default()
        };
        let error = validate_config(&config).unwrap_err().to_string();
        assert_eq!(error, format!("Configuration error: unknown layout detector: {name}"));
    }
}

#[test]
fn should_reject_identical_named_arms_before_loading_any_model() {
    let config = ModelBenchmarkConfig {
        model_b: "rtdetr".to_string(),
        ..Default::default()
    };
    assert_eq!(
        validate_config(&config).unwrap_err().to_string(),
        "Configuration error: detector arms must be different"
    );
}

#[test]
fn should_reject_identical_artifact_hashes_even_when_names_differ() {
    let mut report = report();
    report.models[1].sha256 = report.models[0].sha256.clone();
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Configuration error: detector arms have identical model identities"
    );
}

#[test]
fn should_reject_an_empty_corpus_instead_of_reporting_a_zero_work_pass() {
    let mut report = report();
    report.expected_documents.clear();
    report.rows.clear();
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: detector benchmark corpus is empty"
    );
}

#[test]
fn should_reject_skipped_inference_even_when_a_detection_row_is_present() {
    let mut report = report();
    report.rows[0].measurements[1].inference_performed = false;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: detector inference was skipped"
    );
}

#[test]
fn should_reject_a_missing_paired_page_row_against_the_declared_count() {
    let mut report = report();
    report.rows.clear();
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: detector row count mismatch: expected 2 calls, observed 0"
    );
}

#[test]
fn should_reject_page_count_mismatches_before_interpreting_rows() {
    let mut report = report();
    report.expected_pages = 2;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: detector page-count mismatch"
    );
}

#[test]
fn should_reject_fabricated_class_counts_and_duplicate_or_unexpected_rows() {
    let mut report = report();
    report.rows[0].measurements[0]
        .canonical_class_counts
        .insert("formula".to_string(), 3);
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: canonical class counts do not match actual detections"
    );
    let mut report = self::report();
    report.rows.push(report.rows[0].clone());
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: duplicate detector page/iteration row"
    );
    let mut report = self::report();
    report.rows[0].document = "not-in-cohort".to_string();
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: unexpected detector document row"
    );
}

#[test]
fn should_reject_nonfinite_off_page_or_invalid_confidence_detections() {
    let mut original = measurement();
    original.detections[0].bbox.x2 = 101.0;
    let mut report = report();
    report.rows[0].measurements[0] = original;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: invalid or off-page detection"
    );
    for confidence in [f32::NAN, -0.1, 1.1] {
        let mut report = self::report();
        report.rows[0].measurements[0].detections[0].confidence = confidence;
        assert_eq!(
            report.validate().unwrap_err().to_string(),
            "Benchmark error: invalid or off-page detection"
        );
    }
}

#[test]
fn should_keep_zero_detections_distinct_from_skipped_inference() {
    let empty = DetectorMeasurement::from_detection(
        DetectionResult {
            page_width: 100,
            page_height: 200,
            detections: Vec::new(),
        },
        1.0,
    )
    .unwrap();
    let mut report = report();
    report.rows[0].measurements = [empty.clone(), empty];
    report.validate().unwrap();
    assert_eq!(report.actual_inference_calls, 2);
    assert_eq!(report.rows[0].measurements[0].detections.len(), 0);
}

fn three_page_pdf() -> Vec<u8> {
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Count 3 /Kids [3 0 R 4 0 R 5 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 200] /Contents 6 0 R >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 200] /Contents 6 0 R >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 200] /Contents 6 0 R >>",
        "<< /Length 0 >>\nstream\n\nendstream",
    ];
    let mut offsets = vec![0];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for offset in &offsets[1..] {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    bytes
}

#[test]
fn should_apply_the_page_limit_to_a_real_pdf_before_model_loading() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("three.pdf");
    let bytes = three_page_pdf();
    std::fs::write(&path, &bytes).unwrap();
    let fixture_path = temp.path().join("fixture.json");
    std::fs::write(&fixture_path, "{}").unwrap();
    let doc = CorpusDocument {
        name: "bounded".to_string(),
        document_path: path,
        file_type: "pdf".to_string(),
        file_size: bytes.len() as u64,
        ground_truth_text: None,
        ground_truth_markdown: None,
        metadata: Default::default(),
        fixture_path,
    };
    let config = ModelBenchmarkConfig {
        max_pages: 2,
        expected_pages: Some(2),
        ..Default::default()
    };
    let plan = plan_documents(std::slice::from_ref(&doc), &config).unwrap();
    assert_eq!(plan["bounded"].pages, 2);
    let mismatch = ModelBenchmarkConfig {
        expected_pages: Some(3),
        ..config
    };
    assert_eq!(
        plan_documents(&[doc], &mismatch).unwrap_err().to_string(),
        "Configuration error: detector page-count mismatch: planned 2 pages"
    );
}

#[test]
fn should_reject_rows_outside_the_page_limit_and_changed_render_identities() {
    let mut report = report();
    report.rows[0].page = 2;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: detector page/iteration exceeds declared limits"
    );
    let mut report = self::report();
    report.iterations = 2;
    report.expected_inference_calls = 4;
    report.actual_inference_calls = 4;
    let mut second = report.rows[0].clone();
    second.iteration = 2;
    second.render_sha256 = "1".repeat(64);
    report.rows.push(second);
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: page render changed between iterations"
    );
}

#[test]
fn should_reject_invalid_configuration_and_quality_claims_in_saved_artifacts() {
    let mut report = report();
    report.quality_assessed = true;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: unsupported detector report contract"
    );
    let mut report = self::report();
    report.confidence_threshold = -0.1;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: invalid detector configuration or input identity"
    );
    let mut report = self::report();
    report.models[1].sha256 = report.models[0].sha256.to_uppercase();
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Configuration error: detector arms have identical model identities"
    );
}

#[test]
fn should_reject_count_overflow_and_different_arm_dimensions() {
    let mut report = report();
    report.expected_documents.get_mut("page").unwrap().pages = usize::MAX;
    report.max_pages = usize::MAX;
    report.expected_pages = usize::MAX;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Configuration error: detector inference count overflow"
    );
    let mut report = self::report();
    report.rows[0].measurements[1].page_width = 99;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: detector arms saw different page dimensions"
    );
}

#[test]
fn should_reject_unfixed_session_budgets_in_saved_detector_artifacts() {
    let mut report = report();
    report.models[0].intra_op_threads = 8;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: invalid detector model identity"
    );
    let mut report = self::report();
    report.models[1].inter_op_threads = 2;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: invalid detector model identity"
    );
}

#[test]
fn should_reject_saved_artifacts_without_paired_resource_evidence() {
    let mut report = report();
    report.resources = None;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: missing paired process resource evidence"
    );
    let mut report = self::report();
    report.schema_version = 1;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: unsupported detector report contract"
    );
}

#[test]
fn should_reject_a_changed_paired_execution_order() {
    let mut report = report();
    report.rows[0].a_first = false;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: detector paired execution order mismatch"
    );
}

#[test]
fn should_require_alternating_execution_order_across_repetitions() {
    let mut report = report();
    report.iterations = 2;
    report.expected_inference_calls = 4;
    report.actual_inference_calls = 4;
    let mut second = report.rows[0].clone();
    second.iteration = 2;
    second.a_first = false;
    report.rows.push(second);
    report.validate().unwrap();
    report.rows[1].a_first = true;
    assert_eq!(
        report.validate().unwrap_err().to_string(),
        "Benchmark error: detector paired execution order mismatch"
    );
}

#[path = "geometry_tests.rs"]
mod geometry_scoring;
