//! Integration tests for the benchmark artifact/aggregate release-contract validator.
//!
//! Ported from `tools/benchmark-harness/tests/test_validate_benchmark_artifacts.py`, which
//! exercised `scripts/ci/benchmarks/validate-benchmark-artifacts.py`. Real cohort manifests and
//! fixture descriptors are copied from the repository (so a pinned-digest drift is caught), but
//! referenced documents are synthetic — the validator never reads document content, only its
//! BLAKE3 digest and byte length, which the synthetic bytes provide consistently.

use benchmark_harness::aggregate::{FileTypeAggregation, NewConsolidatedResults, RankedFramework};
use benchmark_harness::bench_matrix::{Cohort, CohortContract};
use benchmark_harness::consolidate::RunProvenanceRecord;
use benchmark_harness::provenance::{
    CorpusProvenance, FixtureProvenance, FrameworkProvenance, RepositoryProvenance, RunProvenance, TimingProvenance,
};
use benchmark_harness::types::{
    BenchmarkResult, ErrorKind, FrameworkCapabilities, IterationResult, OcrStatus, OutputFormat, PerformanceMetrics,
    QualityMetrics,
};
use benchmark_harness::validate_artifacts::{ValidateArtifactsArgs, validate};
use benchmark_harness::{Error, write_json, write_run_provenance};

use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

const SOURCE_SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const RUN_ID: &str = "42";
const ITERATIONS: usize = 3;

mod aggregate_tests;
mod results_tests;

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn blake3_hex(path: &Path) -> String {
    blake3::hash(&std::fs::read(path).expect("read file to hash"))
        .to_hex()
        .to_string()
}

fn zero_metrics() -> PerformanceMetrics {
    PerformanceMetrics {
        baseline_memory_bytes: 0,
        peak_memory_bytes: 0,
        peak_memory_delta_bytes: 0,
        avg_cpu_percent: 0.0,
        cpu_seconds: 0.0,
        throughput_bytes_per_sec: 0.0,
        p50_memory_bytes: 0,
        p95_memory_bytes: 0,
        p99_memory_bytes: 0,
        ..Default::default()
    }
}

/// A cohort's fixture tree, materialized under a temp root exactly like the release CI layout:
/// real cohort manifest + fixture descriptor bytes (so pinned digests stay honest), synthetic
/// document bytes (the validator never reads document content).
struct FixtureTree {
    _root: TempDir,
    cohort_manifest: PathBuf,
    fixtures_root: PathBuf,
    manifest_blake3: String,
    fixture_provenance: Vec<FixtureProvenance>,
}

fn copy_ground_truth_files(source_descriptor: &Path, descriptor: &Path, value: &serde_json::Value) {
    let source_parent = source_descriptor.parent().expect("source descriptor has a parent");
    let destination_parent = descriptor.parent().expect("descriptor has a parent");
    for field in ["text_file", "markdown_file"] {
        let Some(relative_path) = value["ground_truth"][field].as_str() else {
            continue;
        };
        let destination = destination_parent.join(relative_path);
        std::fs::create_dir_all(destination.parent().expect("ground truth has a parent"))
            .expect("create ground truth dir");
        std::fs::copy(source_parent.join(relative_path), destination).expect("copy ground truth");
    }
}

fn materialize_fixture_tree(cohort: Cohort, contract: &CohortContract) -> FixtureTree {
    let root = tempfile::tempdir().expect("tempdir");
    let cohort_manifest = root.path().join("cohort.json");
    let harness_root = root.path().join("tools/benchmark-harness");
    let fixtures_root = harness_root.join("fixtures");
    std::fs::create_dir_all(&fixtures_root).expect("create fixtures root");
    // ~keep Materialize the repository anchor used by production fixture validation so these
    // release-layout tests exercise their intended contract instead of a standalone-tree boundary.
    std::fs::copy(repo_path("Cargo.toml"), harness_root.join("Cargo.toml")).expect("copy harness manifest");

    let cohort_slug = match cohort {
        Cohort::Native => "native-pdf-fast-b8",
        Cohort::Ocr => "ocr-pdf-fast-b4",
        Cohort::Office => "native-office-fast",
        Cohort::Markup => "native-markup-fast",
        Cohort::Ebook => "native-ebook-fast",
        Cohort::Email => "native-email-fast",
        Cohort::Data => "native-data-fast",
        Cohort::Images => "ocr-images-fast",
        Cohort::NativeStructured => "native-pdf-structured-v1",
        Cohort::OcrStructured => "ocr-pdf-structured-v1",
        Cohort::ImagesStructured => "ocr-images-structured-v2",
    };
    std::fs::copy(repo_path(&format!("cohorts/{cohort_slug}.json")), &cohort_manifest).expect("copy cohort manifest");
    let manifest_blake3 = blake3_hex(&cohort_manifest);

    let mut fixture_provenance = Vec::with_capacity(contract.fixtures.len());
    for (fixture, document_stem) in contract.fixtures.iter().zip(contract.document_stems.iter()) {
        let descriptor_path = fixtures_root.join(fixture);
        let source_descriptor_path = repo_path(&format!("fixtures/{fixture}"));
        std::fs::create_dir_all(descriptor_path.parent().expect("descriptor has a parent"))
            .expect("create descriptor dir");
        std::fs::copy(&source_descriptor_path, &descriptor_path).expect("copy fixture descriptor");

        let descriptor: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&descriptor_path).expect("read descriptor"))
                .expect("parse descriptor");
        // ~keep Production validates descriptor-owned ground truth before artifact contracts.
        // Copy the checked-in bytes so this synthetic repository exercises the same boundary.
        copy_ground_truth_files(&source_descriptor_path, &descriptor_path, &descriptor);
        let document_field = descriptor["document"]
            .as_str()
            .expect("descriptor has a document field");
        let document_path = descriptor_path
            .parent()
            .expect("descriptor has a parent")
            .join(document_field);
        std::fs::create_dir_all(document_path.parent().expect("document has a parent")).expect("create document dir");
        std::fs::write(
            &document_path,
            format!("temporary benchmark document: {document_stem}\n"),
        )
        .expect("write document");

        fixture_provenance.push(FixtureProvenance {
            fixture: (*fixture).to_string(),
            fixture_blake3: blake3_hex(&descriptor_path),
            document_blake3: blake3_hex(&document_path),
            document_bytes: std::fs::metadata(&document_path).expect("stat document").len(),
        });
    }

    FixtureTree {
        _root: root,
        cohort_manifest,
        fixtures_root,
        manifest_blake3,
        fixture_provenance,
    }
}

/// The framework name the runner actually writes into artifacts. Xberg batch cells carry a
/// `-batch` suffix (see `adapters::xberg` and `normalize_run_frameworks`); competitors and
/// single-file cells use the bare name. Mirroring this here keeps the fixtures faithful to real
/// artifacts so the validator's suffix handling is exercised.
fn runtime_framework_name(entry: &benchmark_harness::bench_matrix::MatrixEntry) -> String {
    use benchmark_harness::bench_matrix::ExecutionMode;

    if matches!(entry.mode, ExecutionMode::Batch) && entry.framework.starts_with("xberg-") {
        format!("{}-batch", entry.framework)
    } else {
        entry.framework.clone()
    }
}

fn aggregate_framework_name(entry: &benchmark_harness::bench_matrix::MatrixEntry) -> String {
    let mut framework = runtime_framework_name(entry);
    if matches!(entry.mode, benchmark_harness::bench_matrix::ExecutionMode::Batch) && !framework.ends_with("-batch") {
        framework.push_str("-batch");
    }
    framework
}

fn supports_extension(framework: &str, extension: &str) -> bool {
    if framework.starts_with("xberg-") {
        return true;
    }
    let supported: &[&str] = match framework {
        "liteparse" => &[
            "pdf", "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "svg", "doc", "docx", "docm", "dot",
            "dotm", "dotx", "odt", "ott", "rtf", "pages", "ppt", "pptx", "pptm", "pot", "potm", "potx", "odp", "otp",
            "key", "xls", "xlsx", "xlsm", "xlsb", "ods", "ots", "csv", "tsv", "numbers",
        ],
        "pymupdf4llm" => &["pdf", "epub", "fb2", "png", "jpg", "jpeg", "bmp", "tiff", "tif"],
        "docling" => &[
            "pdf", "docx", "pptx", "xlsx", "html", "md", "csv", "png", "jpg", "jpeg", "tiff", "tif", "bmp",
        ],
        "tika" => &[
            "pdf", "docx", "doc", "pptx", "ppt", "xlsx", "odt", "rtf", "epub", "html", "md", "csv", "tsv", "json",
            "yaml", "eml", "msg", "tex", "rst", "org", "png", "jpg", "jpeg", "tiff", "tif",
        ],
        "markitdown" => &[
            "pdf", "docx", "pptx", "xlsx", "html", "md", "csv", "json", "epub", "msg", "png", "jpg", "jpeg", "bmp",
            "tiff", "tif",
        ],
        "unstructured" => &[
            "pdf", "docx", "doc", "pptx", "ppt", "xlsx", "odt", "rtf", "epub", "html", "md", "rst", "org", "csv",
            "tsv", "eml", "msg", "png", "jpg", "jpeg", "tiff", "tif", "bmp",
        ],
        "mineru" => &["pdf", "png", "jpg", "jpeg", "bmp", "tiff", "tif"],
        _ => &[],
    };
    supported.contains(&extension)
}

fn supports_fixture_language(framework: &str, fixture: &str) -> bool {
    let descriptor: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo_path(&format!("fixtures/{fixture}"))).expect("read fixture descriptor"),
    )
    .expect("parse fixture descriptor");
    let language = descriptor["metadata"]["ocr_language"].as_str();
    benchmark_harness::adapter::declared_ocr_language_policy(framework).supports(language)
}

#[test]
fn ocr_pdf_contract_eligibility_matches_framework_language_capabilities() {
    let contract = Cohort::Ocr.contract();
    let eligible = |framework: &str| {
        contract
            .document_extensions
            .iter()
            .zip(contract.fixtures.iter())
            .filter(|(extension, fixture)| {
                supports_extension(framework, extension) && supports_fixture_language(framework, fixture)
            })
            .count()
    };

    assert_eq!(
        eligible("liteparse"),
        3,
        "default-language LiteParse must exclude German"
    );
    assert_eq!(eligible("docling"), 4, "Docling accepts the per-batch German selection");
    assert_eq!(eligible("xberg-markdown-sceptre-ort"), 4);
}

fn build_provenance(
    entry: &benchmark_harness::bench_matrix::MatrixEntry,
    contract: &CohortContract,
    manifest_blake3: &str,
    fixture_provenance: &[FixtureProvenance],
) -> RunProvenance {
    use benchmark_harness::bench_matrix::ExecutionMode;

    let batch = matches!(entry.mode, ExecutionMode::Batch);
    let policy = benchmark_harness::adapter::declared_ocr_language_policy(&entry.framework);
    let eligible_languages: Vec<Option<String>> = contract
        .document_extensions
        .iter()
        .zip(contract.fixtures.iter())
        .filter(|(extension, fixture)| {
            supports_extension(&entry.framework, extension) && supports_fixture_language(&entry.framework, fixture)
        })
        .map(|(_, fixture)| {
            let descriptor: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(repo_path(&format!("fixtures/{fixture}"))).expect("read fixture descriptor"),
            )
            .expect("parse fixture descriptor");
            descriptor["metadata"]["ocr_language"].as_str().map(str::to_string)
        })
        .collect();
    let eligible_documents = eligible_languages.len();
    RunProvenance {
        schema_version: 3,
        harness_version: "test".to_string(),
        repository: RepositoryProvenance {
            commit: Some(SOURCE_SHA.to_string()),
            dirty: Some(false),
        },
        corpus: CorpusProvenance {
            cohort: Some(contract.manifest_name.to_string()),
            cohort_manifest_blake3: Some(manifest_blake3.to_string()),
            ordered_fixtures: fixture_provenance.to_vec(),
        },
        frameworks: vec![FrameworkProvenance {
            name: runtime_framework_name(entry),
            timing_regime: benchmark_harness::types::TimingRegime::ColdProcess,
            resource_measurement_scope: benchmark_harness::types::ResourceMeasurementScope::IsolatedProcess,
            version: "0.0.0".to_string(),
            executable: None,
            models: Vec::new(),
            batch_capability: None,
            requested_workers: None,
            effective_workers: None,
            configured_thread_budget: None,
            worker_semantics: "test".to_string(),
            effective_warmup_iterations: 0,
            eligible_documents,
            batch_partitions: batch.then(|| policy.batch_partition_count(&eligible_languages, contract.batch_size)),
            ocr_language_policy: policy,
        }],
        timing: TimingProvenance {
            mode: entry.mode.benchmark_mode(),
            warmup_iterations: 0,
            benchmark_iterations: ITERATIONS,
            timeout_ms: 0,
            output_format: entry.output_format,
        },
        fixed_batch_size: batch.then_some(contract.batch_size),
        coverage: None,
    }
}

fn build_results(
    entry: &benchmark_harness::bench_matrix::MatrixEntry,
    contract: &CohortContract,
    cohort: Cohort,
    fixtures_root: &Path,
) -> Vec<BenchmarkResult> {
    contract
        .document_stems
        .iter()
        .zip(contract.document_extensions.iter())
        .zip(contract.fixtures.iter())
        .filter(|((_, extension), fixture)| {
            supports_extension(&entry.framework, extension) && supports_fixture_language(&entry.framework, fixture)
        })
        .map(|((stem, extension), fixture)| {
            let descriptor: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(fixtures_root.join(fixture)).expect("read fixture descriptor"),
            )
            .expect("parse fixture descriptor");
            let has_quality_ground_truth = descriptor["ground_truth"]["text_file"].is_string()
                || descriptor["ground_truth"]["markdown_file"].is_string();
            let has_structural_ground_truth = descriptor["ground_truth"]["markdown_file"].is_string();
            BenchmarkResult {
                framework: runtime_framework_name(entry),
                output_format: entry.output_format,
                file_path: PathBuf::from(format!("/workspace/test_documents/{stem}.{extension}")),
                file_size: 1,
                success: true,
                error_message: None,
                error_kind: ErrorKind::None,
                duration: Duration::from_millis(1),
                extraction_duration: None,
                subprocess_overhead: None,
                metrics: zero_metrics(),
                quality: has_quality_ground_truth.then(|| QualityMetrics {
                    f1_score_text: 0.9,
                    f1_score_numeric: 0.8,
                    f1_score_layout: (entry.output_format == OutputFormat::Markdown && has_structural_ground_truth)
                        .then_some(0.7),
                    quality_score: 0.85,
                    missing_tokens: vec![],
                    extra_tokens: vec![],
                    correct: false,
                    reading_order_score: None,
                }),
                iterations: (0..ITERATIONS)
                    .map(|index| IterationResult {
                        // mirror that here so the fixture matches real artifacts.
                        iteration: index + 1,
                        success: Some(true),
                        error_kind: Some(ErrorKind::None),
                        duration: Duration::from_millis(1),
                        extraction_duration: None,
                        subprocess_overhead: None,
                        metrics: zero_metrics(),
                        batch_sample_id: None,
                        batch_performance_sample: None,
                    })
                    .collect(),
                statistics: None,
                cold_start_duration: None,
                file_extension: (*extension).to_string(),
                framework_capabilities: FrameworkCapabilities {
                    timing_regime: benchmark_harness::types::TimingRegime::ColdProcess,
                    resource_measurement_scope: benchmark_harness::types::ResourceMeasurementScope::IsolatedProcess,
                    ..Default::default()
                },
                pdf_metadata: None,
                ocr_status: if cohort.expects_ocr() {
                    OcrStatus::Used
                } else {
                    OcrStatus::NotUsed
                },
                extracted_text: None,
                system_load: None,
            }
        })
        .collect()
}

/// A fully materialized, contract-conformant artifact tree ready to validate.
struct ArtifactScenario {
    _tree: FixtureTree,
    args: ValidateArtifactsArgs,
    contract: CohortContract,
}

fn artifact_scenario(cohort: Cohort) -> ArtifactScenario {
    let contract = cohort.contract();
    let tree = materialize_fixture_tree(cohort, &contract);
    let artifacts_dir = tree.cohort_manifest.parent().expect("root").join("artifacts");
    std::fs::create_dir_all(&artifacts_dir).expect("create artifacts dir");

    for entry in &contract.matrix {
        let run_dir = artifacts_dir.join(format!("{}-{RUN_ID}", entry.artifact)).join("run");
        let provenance = build_provenance(entry, &contract, &tree.manifest_blake3, &tree.fixture_provenance);
        write_run_provenance(&provenance, &run_dir.join("provenance.json")).expect("write provenance");
        let results = build_results(entry, &contract, cohort, &tree.fixtures_root);
        write_json(&results, &run_dir.join("results.json")).expect("write results");
    }

    let args = ValidateArtifactsArgs {
        cohort,
        aggregated_file: None,
        artifacts_dir: Some(artifacts_dir),
        cohort_manifest: Some(tree.cohort_manifest.clone()),
        fixtures_root: Some(tree.fixtures_root.clone()),
        source_sha: Some(SOURCE_SHA.to_string()),
        run_id: Some(RUN_ID.to_string()),
        iterations: ITERATIONS,
    };

    ArtifactScenario {
        _tree: tree,
        args,
        contract,
    }
}

fn provenance_path(scenario: &ArtifactScenario, matrix_index: usize) -> PathBuf {
    let entry = &scenario.contract.matrix[matrix_index];
    scenario
        .args
        .artifacts_dir
        .as_ref()
        .unwrap()
        .join(format!("{}-{RUN_ID}", entry.artifact))
        .join("run/provenance.json")
}

fn results_path(scenario: &ArtifactScenario, matrix_index: usize) -> PathBuf {
    let entry = &scenario.contract.matrix[matrix_index];
    scenario
        .args
        .artifacts_dir
        .as_ref()
        .unwrap()
        .join(format!("{}-{RUN_ID}", entry.artifact))
        .join("run/results.json")
}

fn tamper_json(path: &Path, mutate: impl FnOnce(&mut serde_json::Value)) {
    let mut value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    mutate(&mut value);
    std::fs::write(path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
}

fn assert_err_contains<T: std::fmt::Debug>(result: Result<T, Error>, needle: &str) {
    let error = result.expect_err("expected validation failure");
    let message = error.to_string();
    assert!(
        message.contains(needle),
        "expected error containing {needle:?}, got: {message}"
    );
}

fn required_count(contract: &CohortContract) -> usize {
    contract.matrix.iter().filter(|entry| !entry.optional).count()
}

/// Aggregate key of a required (non-optional) matrix entry. Validation skips optional groups, so a
/// tamper test must target a required group to observe the rejection deterministically — picking an
/// arbitrary group via HashMap iteration order is flaky, as it may land on a skipped optional group.
fn required_group_key(contract: &CohortContract) -> String {
    contract
        .matrix
        .iter()
        .find(|entry| !entry.optional)
        .expect("cohort has a required entry")
        .aggregate_key()
}

fn optional_artifact_dir(scenario: &ArtifactScenario) -> PathBuf {
    let entry = scenario
        .contract
        .matrix
        .iter()
        .find(|entry| entry.optional)
        .expect("cohort has an optional (best-effort) entry");
    scenario
        .args
        .artifacts_dir
        .as_ref()
        .unwrap()
        .join(format!("{}-{RUN_ID}", entry.artifact))
}

fn optional_matrix_index(contract: &CohortContract) -> usize {
    contract
        .matrix
        .iter()
        .position(|entry| entry.optional)
        .expect("cohort has an optional (best-effort) entry")
}

/// Index of a batch-mode xberg cell in the native matrix — the case that carries the runner's
/// `-batch` framework-name suffix.
fn batch_xberg_index(contract: &CohortContract) -> usize {
    use benchmark_harness::bench_matrix::ExecutionMode;
    contract
        .matrix
        .iter()
        .position(|entry| entry.framework.starts_with("xberg-") && matches!(entry.mode, ExecutionMode::Batch))
        .expect("native matrix has a batch xberg cell")
}

fn build_aggregate_with(
    contract: &CohortContract,
    cohort: Cohort,
    include_entry: impl Fn(&benchmark_harness::bench_matrix::MatrixEntry) -> bool,
    mutate_results: impl FnOnce(&mut Vec<BenchmarkResult>),
) -> NewConsolidatedResults {
    let fixture_provenance: Vec<FixtureProvenance> = contract
        .fixtures
        .iter()
        .map(|fixture| FixtureProvenance {
            fixture: (*fixture).to_string(),
            fixture_blake3: "b".repeat(64),
            document_blake3: "c".repeat(64),
            document_bytes: 1,
        })
        .collect();
    let run_provenance: Vec<RunProvenanceRecord> = contract
        .matrix
        .iter()
        .filter(|entry| include_entry(entry))
        .map(|entry| RunProvenanceRecord {
            source_dir: entry.artifact.clone(),
            provenance: Some(build_provenance(
                entry,
                contract,
                contract.manifest_blake3,
                &fixture_provenance,
            )),
            missing_reason: None,
        })
        .collect();
    // ~keep Consolidation tags every framework loaded from a batch artifact directory. This
    // builder bypasses that filesystem loader, so mirror the tag before deriving aggregate keys.
    let results: Vec<BenchmarkResult> = contract
        .matrix
        .iter()
        .filter(|entry| include_entry(entry))
        .flat_map(|entry| {
            let mut entry_results = build_results(entry, contract, cohort, &repo_path("fixtures"));
            let framework = aggregate_framework_name(entry);
            for result in &mut entry_results {
                result.framework.clone_from(&framework);
            }
            entry_results
        })
        .collect();
    let mut results = results;
    mutate_results(&mut results);
    let mut aggregate = benchmark_harness::aggregate_new_format(&results);
    aggregate.run_provenance = run_provenance;
    benchmark_harness::aggregate::apply_pinned_cohort_comparison(&mut aggregate).expect("apply cohort comparison");
    aggregate
}

fn build_aggregate(contract: &CohortContract, cohort: Cohort) -> NewConsolidatedResults {
    build_aggregate_with(contract, cohort, |_| true, |_| {})
}

fn write_aggregate(aggregate: &NewConsolidatedResults) -> (TempDir, PathBuf) {
    let root = tempfile::tempdir().expect("tempdir");
    let path = root.path().join("aggregated.json");
    std::fs::write(&path, serde_json::to_string_pretty(aggregate).unwrap()).unwrap();
    (root, path)
}

fn write_json_value(value: &serde_json::Value) -> (TempDir, PathBuf) {
    let root = tempfile::tempdir().expect("tempdir");
    let path = root.path().join("aggregated.json");
    std::fs::write(&path, serde_json::to_string_pretty(value).unwrap()).unwrap();
    (root, path)
}

fn aggregate_args(cohort: Cohort, aggregated_file: PathBuf) -> ValidateArtifactsArgs {
    ValidateArtifactsArgs {
        cohort,
        aggregated_file: Some(aggregated_file),
        artifacts_dir: None,
        cohort_manifest: None,
        fixtures_root: None,
        source_sha: None,
        run_id: None,
        iterations: ITERATIONS,
    }
}
