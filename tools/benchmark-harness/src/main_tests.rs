//! Tests for `main.rs`'s CLI dispatch and the `run` command's coverage-gate accounting.

use super::tracing_filter;
use crate::cli::{Cli, Commands};
use crate::commands::cohort_contract::cohort_contract_summary;
use crate::commands::parse_pipeline_names;
use crate::commands::pipeline_benchmark::parse_sort_metric;
use crate::commands::run::{
    XBERG_RUN_PIPELINES, compute_framework_coverage, normalize_run_frameworks, parse_model_provenance,
    parse_pdf_backends, selected_frameworks_use_tesseract, should_register_xberg_pipeline,
    validate_framework_result_cardinality, write_run_artifacts_and_check_gate,
};
use benchmark_harness::provenance::{
    CorpusProvenance, FrameworkProvenance, RepositoryProvenance, RunProvenance, TimingProvenance,
};
use benchmark_harness::types::{ErrorKind, FrameworkCapabilities, OcrStatus, PerformanceMetrics};
use benchmark_harness::{BenchmarkMode, BenchmarkResult, OutputFormat, XbergPdfBackend};
use clap::Parser;

#[test]
fn tracing_filter_keeps_status_events_visible_by_default() {
    assert_eq!(tracing_filter(None, false).to_string(), "benchmark_harness=info");
}

#[test]
fn tracing_filter_enables_debug_events_for_benchmark_debug() {
    assert_eq!(tracing_filter(None, true).to_string(), "benchmark_harness=debug");
}

#[test]
fn tracing_filter_honors_explicit_rust_log() {
    assert_eq!(
        tracing_filter(Some("benchmark_harness=trace"), false).to_string(),
        "benchmark_harness=trace"
    );
}

#[test]
fn batch_mode_normalizes_unsuffixed_xberg_aliases() {
    let names = normalize_run_frameworks(
        &[
            "xberg-markdown-baseline".to_string(),
            "xberg-markdown-layout".to_string(),
            "liteparse".to_string(),
        ],
        true,
    );
    assert_eq!(
        names,
        [
            "xberg-markdown-baseline-batch",
            "xberg-markdown-layout-batch",
            "liteparse"
        ]
    );
}

#[test]
fn batch_mode_preserves_and_deduplicates_canonical_names() {
    let names = normalize_run_frameworks(
        &[
            "xberg-markdown-baseline-batch".to_string(),
            "xberg-markdown-baseline".to_string(),
        ],
        true,
    );
    assert_eq!(names, ["xberg-markdown-baseline-batch"]);
}

#[test]
fn batch_mode_normalizes_warm_in_process_lane_name() {
    let names = normalize_run_frameworks(&["xberg-markdown-layout-steady-state".to_string()], true);

    assert_eq!(names, ["xberg-markdown-layout-steady-state-batch"]);
}

#[test]
fn single_mode_preserves_xberg_names() {
    let names = normalize_run_frameworks(&["xberg-markdown-baseline".to_string()], false);
    assert_eq!(names, ["xberg-markdown-baseline"]);
}

#[test]
fn tesseract_preflight_tracks_effective_framework_selection() {
    assert!(selected_frameworks_use_tesseract(&[]));
    assert!(selected_frameworks_use_tesseract(&[
        "xberg-markdown-layout-batch".to_string()
    ]));
    assert!(!selected_frameworks_use_tesseract(&[
        "xberg-markdown-paddle-ocr".to_string()
    ]));
    assert!(!selected_frameworks_use_tesseract(&[
        "xberg-markdown-baseline-paddle-batch".to_string()
    ]));
    assert!(!selected_frameworks_use_tesseract(&[
        "xberg-markdown-layout-paddle".to_string()
    ]));
    assert!(!selected_frameworks_use_tesseract(&["docling".to_string()]));
}

#[test]
fn run_registration_requires_explicit_legacy_paddle_selection() {
    assert!(XBERG_RUN_PIPELINES.contains(&benchmark_harness::XbergPipeline::PaddleOcr));
    assert_eq!(
        XBERG_RUN_PIPELINES
            .iter()
            .filter(|pipeline| should_register_xberg_pipeline(**pipeline, false))
            .count(),
        7
    );
    assert!(should_register_xberg_pipeline(
        benchmark_harness::XbergPipeline::PaddleOcr,
        true
    ));
    assert!(!should_register_xberg_pipeline(
        benchmark_harness::XbergPipeline::SceptreTract,
        false
    ));
}

#[test]
fn run_registration_requires_explicit_layout_v3_selection() {
    assert!(XBERG_RUN_PIPELINES.contains(&benchmark_harness::XbergPipeline::LayoutV3));
    assert!(!should_register_xberg_pipeline(
        benchmark_harness::XbergPipeline::LayoutV3,
        false
    ));
    assert!(should_register_xberg_pipeline(
        benchmark_harness::XbergPipeline::LayoutV3,
        true
    ));
}

#[test]
fn run_cli_accepts_exact_cohort_and_model_identity() {
    let cli = Cli::try_parse_from([
        "benchmark-harness",
        "run",
        "--fixtures",
        "fixtures",
        "--cohort",
        "cohorts/fast.json",
        "--batch-size",
        "4",
        "--xberg-max-threads",
        "8",
        "--model-id",
        "docling=ds4sd/docling-models@main#abc123",
    ])
    .unwrap();

    assert!(matches!(
        cli.command,
        Commands::Run(crate::commands::run::RunArgs {
            batch_size: Some(4),
            xberg_max_threads: Some(8),
            ..
        })
    ));
    assert!(parse_model_provenance(&["invalid".to_string()]).is_err());
}

#[test]
fn run_cli_accepts_pdf_backends_flag() {
    let cli = Cli::try_parse_from([
        "benchmark-harness",
        "run",
        "--fixtures",
        "fixtures",
        "--pdf-backends",
        "native,pdfium",
    ])
    .unwrap();

    let Commands::Run(crate::commands::run::RunArgs { pdf_backends, .. }) = cli.command else {
        panic!("expected run command");
    };
    assert_eq!(pdf_backends, ["native", "pdfium"]);
    assert_eq!(
        parse_pdf_backends(&pdf_backends).unwrap(),
        [XbergPdfBackend::Native, XbergPdfBackend::Pdfium]
    );
}

#[test]
fn parse_pdf_backends_rejects_unknown_and_duplicate_values() {
    assert!(parse_pdf_backends(&["pdf_oxide".to_string()]).is_err());
    assert!(parse_pdf_backends(&["native".to_string(), "native".to_string()]).is_err());
    assert_eq!(parse_pdf_backends(&[]).unwrap(), Vec::<XbergPdfBackend>::new());
}

#[test]
fn run_cli_rejects_success_rate_outside_unit_interval() {
    for value in ["-0.1", "1.1", "NaN"] {
        assert!(
            Cli::try_parse_from(["benchmark-harness", "run", "--min-success-rate", value]).is_err(),
            "{value} must be rejected before benchmark execution"
        );
    }
}

#[test]
fn compare_rejects_unknown_pipeline_names() {
    let error = parse_pipeline_names(&["baseline".to_string(), "typo".to_string()], "compare --pipelines").unwrap_err();

    assert_eq!(
        error.to_string(),
        "Configuration error: unknown pipeline 'typo' supplied to compare --pipelines"
    );
}

#[test]
fn compare_accepts_category_alongside_name_filter() {
    let cli = Cli::try_parse_from([
        "benchmark-harness",
        "compare",
        "--fixtures",
        "fixtures",
        "--category",
        "image-ocr-realgt",
        "--filter",
        "ndl",
    ])
    .unwrap();

    let Commands::Compare(crate::commands::compare::CompareArgs { category, filter, .. }) = cli.command else {
        panic!("expected compare command");
    };
    assert_eq!(category.as_deref(), Some("image-ocr-realgt"));
    assert_eq!(filter.as_deref(), Some("ndl"));
}

#[test]
fn pipeline_benchmark_rejects_unknown_pipeline_names() {
    let error = parse_pipeline_names(
        &["layout".to_string(), "unknown".to_string()],
        "pipeline-benchmark --paths",
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Configuration error: unknown pipeline 'unknown' supplied to pipeline-benchmark --paths"
    );
}

#[test]
fn pipeline_benchmark_accepts_exact_cohort_and_rejects_ambiguous_filters() {
    let cli = Cli::try_parse_from([
        "benchmark-harness",
        "pipeline-benchmark",
        "--fixtures",
        "fixtures",
        "--cohort",
        "cohorts/ocr-images-fast.json",
    ])
    .unwrap();
    assert!(matches!(
        cli.command,
        Commands::PipelineBenchmark(crate::commands::pipeline_benchmark::PipelineBenchmarkArgs { cohort: Some(_), .. })
    ));

    for conflicting_filter in ["--doc", "--group"] {
        assert!(
            Cli::try_parse_from([
                "benchmark-harness",
                "pipeline-benchmark",
                "--fixtures",
                "fixtures",
                "--cohort",
                "cohort.json",
                conflicting_filter,
                "sample",
            ])
            .is_err()
        );
    }
}

#[test]
fn pipeline_benchmark_rejects_unknown_sort_metric() {
    let error = parse_sort_metric("fastest").unwrap_err();

    assert_eq!(
        error.to_string(),
        "Configuration error: unknown sort metric 'fastest': expected one of: sf1, tf1, time"
    );
}

#[test]
fn cohort_contract_summary_includes_exact_matrix_cells() {
    let summary = cohort_contract_summary(benchmark_harness::bench_matrix::Cohort::Native);
    let matrix = summary["matrix"].as_array().expect("matrix array");

    // 8 xberg (baseline/layout x md/plain x single/batch) + 4 xberg pdfium + 4 docling
    // + 4 liteparse + markitdown + unstructured + tika + pymupdf4llm + 1 optional mineru.
    // The pdfium cells were added to `native_matrix` without updating these totals, and the
    // benchmark CI job filters to `cohort::tests::`, so nothing ran this. ~keep
    assert_eq!(matrix.len(), 25);
    assert_eq!(summary["expected_matrix_keys"], 24);
    assert_eq!(summary["optional_matrix_keys"], 1);
    assert_eq!(
        matrix[0]["artifact"],
        "benchmarks-rust-baseline-markdown-single-file-native-pdf-fast-b8"
    );
    assert_eq!(matrix[0]["mode"], "single-file");
    assert_eq!(matrix[0]["optional"], false);
}

#[test]
fn coverage_is_checked_per_framework_at_the_inclusive_boundary() {
    let coverage = compute_framework_coverage([
        ("healthy", true, ErrorKind::None),
        ("healthy", false, ErrorKind::FrameworkError),
        ("broken", false, ErrorKind::FrameworkError),
    ]);
    // "broken" is below the 0.5 threshold and is reported instead of aborting the whole run.
    let broken = coverage
        .iter()
        .find(|framework| framework.framework == "broken")
        .unwrap();
    assert_eq!(broken.success_rate, 0.0);
    assert_eq!(broken.successful, 0);
    assert_eq!(broken.framework_failures, 1);
    assert!(broken.below_gate(0.5));

    let boundary = compute_framework_coverage([
        ("healthy", true, ErrorKind::None),
        ("healthy", false, ErrorKind::Timeout),
    ]);
    assert_eq!(boundary[0].success_rate, 0.5);
    assert!(
        !boundary[0].below_gate(0.5),
        "0.5 rate meets an inclusive 0.5 threshold"
    );
}

#[test]
fn coverage_counts_zero_overlap_as_a_framework_fault_failure() {
    // ZeroOverlap is the runner's reclassification of non-empty-but-garbage output; it must
    // count against the framework exactly like FrameworkError/EmptyContent/Timeout, not like
    // an infrastructure failure.
    let coverage = compute_framework_coverage([
        ("framework", true, ErrorKind::None),
        ("framework", false, ErrorKind::ZeroOverlap),
    ]);
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0].successful, 1);
    assert_eq!(coverage[0].framework_failures, 1);
    assert_eq!(coverage[0].infrastructure_failures, 0);
    assert_eq!(coverage[0].success_rate, 0.5);
}

#[test]
fn coverage_excludes_infrastructure_failures_from_the_rate() {
    let coverage = compute_framework_coverage([
        ("framework", true, ErrorKind::None),
        ("framework", false, ErrorKind::HarnessError),
        ("framework", false, ErrorKind::ConfigSetupError),
    ]);
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0].successful, 1);
    assert_eq!(coverage[0].framework_failures, 0);
    assert_eq!(coverage[0].infrastructure_failures, 2);
    assert_eq!(coverage[0].success_rate, 1.0);
}

#[test]
fn coverage_records_but_does_not_abort_on_only_infrastructure_failures() {
    let coverage = compute_framework_coverage([("framework", false, ErrorKind::HarnessError)]);
    assert_eq!(coverage.len(), 1);
    assert_eq!(coverage[0].successful, 0);
    assert_eq!(coverage[0].framework_failures, 0);
    assert_eq!(coverage[0].infrastructure_failures, 1);
    assert_eq!(coverage[0].accountable(), 0);
    assert_eq!(coverage[0].success_rate, 0.0);
    // Zero accountable results always fails the gate, even at a 0.0 threshold, since the
    // framework never demonstrated a single success.
    assert!(coverage[0].below_gate(0.0));
}

/// Minimal `RunProvenance` for exercising `write_run_artifacts_and_check_gate` without a
/// full benchmark run.
fn sample_provenance() -> RunProvenance {
    RunProvenance {
        schema_version: 3,
        harness_version: "test".to_string(),
        repository: RepositoryProvenance {
            commit: Some("0".repeat(40)),
            dirty: Some(false),
        },
        corpus: CorpusProvenance {
            cohort: None,
            cohort_manifest_blake3: None,
            ordered_fixtures: vec![],
        },
        frameworks: vec![FrameworkProvenance {
            timing_regime: benchmark_harness::types::TimingRegime::ColdProcess,
            resource_measurement_scope: benchmark_harness::types::ResourceMeasurementScope::IsolatedProcess,
            name: "healthy".to_string(),
            version: "1.0.0".to_string(),
            executable: None,
            models: vec![],
            batch_capability: None,
            requested_workers: None,
            effective_workers: None,
            configured_thread_budget: None,
            worker_semantics: "test".to_string(),
            effective_warmup_iterations: 0,
            eligible_documents: 1,
            batch_partitions: None,
            ocr_language_policy: Default::default(),
        }],
        timing: TimingProvenance {
            mode: BenchmarkMode::SingleFile,
            warmup_iterations: 0,
            benchmark_iterations: 1,
            timeout_ms: 1_000,
            output_format: OutputFormat::Markdown,
        },
        fixed_batch_size: None,
        coverage: None,
    }
}

/// A minimal, valid `BenchmarkResult` row for a given framework/outcome.
fn sample_result(framework: &str, success: bool, error_kind: ErrorKind) -> BenchmarkResult {
    BenchmarkResult {
        framework: framework.to_string(),
        output_format: OutputFormat::Markdown,
        file_path: std::path::PathBuf::from(format!("/tmp/{framework}.pdf")),
        file_size: 1,
        success,
        error_message: (!success).then(|| "synthetic failure".to_string()),
        error_kind,
        duration: std::time::Duration::from_millis(1),
        extraction_duration: None,
        subprocess_overhead: None,
        metrics: PerformanceMetrics {
            baseline_memory_bytes: 0,
            peak_memory_bytes: 0,
            peak_memory_delta_bytes: 0,
            avg_cpu_percent: 0.0,
            cpu_seconds: 0.0,
            throughput_bytes_per_sec: 0.0,
            p50_memory_bytes: 0,
            p95_memory_bytes: 0,
            p99_memory_bytes: 0,
        },
        quality: None,
        iterations: vec![],
        statistics: None,
        cold_start_duration: None,
        file_extension: "pdf".to_string(),
        framework_capabilities: FrameworkCapabilities::default(),
        pdf_metadata: None,
        ocr_status: OcrStatus::Unknown,
        extracted_text: None,
        system_load: None,
    }
}

#[test]
fn gate_failure_still_writes_every_framework_and_reports_failure() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().to_path_buf();

    let results = vec![
        sample_result("healthy", true, ErrorKind::None),
        sample_result("broken", false, ErrorKind::FrameworkError),
    ];

    let outcome = write_run_artifacts_and_check_gate(&results, sample_provenance(), &output, 1.0);
    assert!(outcome.is_err(), "the gate must still fail the command");
    assert_eq!(
        outcome.unwrap_err().to_string(),
        "Benchmark error: 1 of 2 framework(s) fell below the required minimum success rate 1.000: broken"
    );

    let results_path = output.join("results.json");
    assert!(
        results_path.exists(),
        "results.json must be written despite the gate failure"
    );
    let written: Vec<BenchmarkResult> = serde_json::from_str(&std::fs::read_to_string(&results_path).unwrap())
        .expect("results.json must contain valid BenchmarkResult rows");
    let mut frameworks: Vec<&str> = written.iter().map(|result| result.framework.as_str()).collect();
    frameworks.sort_unstable();
    assert_eq!(
        frameworks,
        ["broken", "healthy"],
        "both frameworks' rows must survive the gate failure"
    );

    assert!(output.join("by-extension.json").exists());
    assert!(output.join("provenance.json").exists());
}

#[test]
fn gate_failure_records_threshold_and_per_framework_coverage_in_provenance() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().to_path_buf();

    let results = vec![
        sample_result("healthy", true, ErrorKind::None),
        sample_result("broken", true, ErrorKind::None),
        sample_result("broken", false, ErrorKind::FrameworkError),
    ];

    let outcome = write_run_artifacts_and_check_gate(&results, sample_provenance(), &output, 0.75);
    assert!(outcome.is_err());

    let provenance: RunProvenance =
        serde_json::from_str(&std::fs::read_to_string(output.join("provenance.json")).unwrap())
            .expect("provenance.json must deserialize");
    let gate = provenance.coverage.expect("coverage gate must be recorded");

    assert_eq!(gate.min_success_rate, 0.75);
    assert!(!gate.passed);
    assert_eq!(gate.failing_frameworks, vec!["broken".to_string()]);

    let mut frameworks = gate.frameworks;
    frameworks.sort_by(|a, b| a.framework.cmp(&b.framework));
    assert_eq!(frameworks.len(), 2);
    assert_eq!(frameworks[0].framework, "broken");
    assert_eq!(frameworks[0].successful, 1);
    assert_eq!(frameworks[0].framework_failures, 1);
    assert_eq!(frameworks[0].infrastructure_failures, 0);
    assert_eq!(frameworks[0].success_rate, 0.5);
    assert_eq!(frameworks[1].framework, "healthy");
    assert_eq!(frameworks[1].successful, 1);
    assert_eq!(frameworks[1].framework_failures, 0);
    assert_eq!(frameworks[1].infrastructure_failures, 0);
    assert_eq!(frameworks[1].success_rate, 1.0);
}

#[test]
fn gate_pass_writes_artifacts_and_reports_success() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().to_path_buf();

    let results = vec![sample_result("healthy", true, ErrorKind::None)];

    write_run_artifacts_and_check_gate(&results, sample_provenance(), &output, 1.0)
        .expect("a fully successful run must pass the gate");

    let provenance: RunProvenance =
        serde_json::from_str(&std::fs::read_to_string(output.join("provenance.json")).unwrap()).unwrap();
    let gate = provenance
        .coverage
        .expect("coverage gate must be recorded even on success");
    assert!(gate.passed);
    assert!(gate.failing_frameworks.is_empty());
}

#[test]
fn cardinality_rejects_missing_framework_results() {
    let error = validate_framework_result_cardinality(
        ["xberg-markdown-baseline", "xberg-markdown-baseline"],
        [("xberg-markdown-baseline", 3)],
    )
    .unwrap_err();

    assert_eq!(
        error,
        "xberg-markdown-baseline produced 2 result(s), expected 3 eligible document result(s)"
    );
}

#[test]
fn cardinality_rejects_unexpected_framework_results() {
    let error = validate_framework_result_cardinality(["unexpected"], [("expected", 1)]).unwrap_err();

    assert_eq!(
        error,
        "unexpected produced results without an eligible provenance entry"
    );
}

#[test]
fn run_help_documents_cohort_path_precedence() {
    let Err(error) = Cli::try_parse_from(["benchmark-harness", "run", "--help"]) else {
        panic!("--help should exit through clap");
    };
    let help = error.to_string();

    assert!(help.contains("Absolute paths are used directly"));
    assert!(help.contains("existing relative paths use the current directory"));
    assert!(help.contains("others use the fixture directory"));
}
