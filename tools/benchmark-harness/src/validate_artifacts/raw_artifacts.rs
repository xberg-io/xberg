//! Raw per-framework artifact mode: validates `provenance.json`/`results.json` pairs
//! downloaded into per-artifact directories against the release contract.
//!
//! Split out of `validate_artifacts.rs` purely to keep that file under the repo's
//! file-length limit; `use super::*` pulls in the shared kernel (types, `require`/
//! `contract_error`, JSON loaders, etc.) defined there.

use super::*;
use crate::adapters::xberg::is_auto_ocr_framework;

/// Everything [`validate_provenance`] checks a `provenance.json` against, besides the record
/// itself and its path -- bundled into one struct purely to keep the function's parameter count
/// down; carries no behavior of its own.
struct ProvenanceExpectations<'a> {
    entry: &'a MatrixEntry,
    contract: &'a CohortContract,
    manifest_blake3: &'a str,
    fixtures: &'a [ExpectedFixture],
    source_sha: &'a str,
    iterations: usize,
    eligible_documents: usize,
    expected_partitions: Option<usize>,
}

fn validate_provenance_repository_and_corpus(
    provenance: &RunProvenance,
    path: &Path,
    expected: &ProvenanceExpectations<'_>,
) -> Result<()> {
    require(
        provenance.schema_version == EXPECTED_PROVENANCE_SCHEMA_VERSION,
        format!("{}: unexpected provenance schema", path.display()),
    )?;
    require(
        provenance.repository.commit.as_deref() == Some(expected.source_sha),
        format!("{}: source SHA mismatch", path.display()),
    )?;
    require(
        provenance.repository.dirty == Some(false),
        format!("{}: benchmark checkout was dirty", path.display()),
    )?;

    require(
        provenance.corpus.cohort.as_deref() == Some(expected.contract.manifest_name),
        format!("{}: cohort name mismatch", path.display()),
    )?;
    require(
        provenance.corpus.cohort_manifest_blake3.as_deref() == Some(expected.manifest_blake3),
        format!("{}: cohort manifest hash mismatch", path.display()),
    )?;

    require(
        provenance.corpus.ordered_fixtures.len() == expected.fixtures.len(),
        format!("{}: fixture count mismatch", path.display()),
    )?;
    for (index, (item, expected_item)) in provenance
        .corpus
        .ordered_fixtures
        .iter()
        .zip(expected.fixtures)
        .enumerate()
    {
        require(
            item.fixture == expected_item.fixture,
            format!("{}: fixture {index} identity/order mismatch", path.display()),
        )?;
        require(
            item.fixture_blake3 == expected_item.fixture_blake3,
            format!("{}: fixture {index} descriptor BLAKE3 mismatch", path.display()),
        )?;
        require(
            item.document_blake3 == expected_item.document_blake3,
            format!("{}: fixture {index} document BLAKE3 mismatch", path.display()),
        )?;
        require(
            item.document_bytes == expected_item.document_bytes,
            format!("{}: fixture {index} document size mismatch", path.display()),
        )?;
    }
    Ok(())
}

fn validate_provenance_timing(
    provenance: &RunProvenance,
    path: &Path,
    expected: &ProvenanceExpectations<'_>,
) -> Result<()> {
    require(
        provenance.timing.mode == expected.entry.mode.benchmark_mode(),
        format!("{}: execution mode mismatch", path.display()),
    )?;
    require(
        provenance.timing.benchmark_iterations == expected.iterations,
        format!("{}: iteration count mismatch", path.display()),
    )?;
    require(
        provenance.timing.output_format == expected.entry.output_format,
        format!("{}: output format mismatch", path.display()),
    )?;

    let expected_batch = matches!(expected.entry.mode, ExecutionMode::Batch).then_some(expected.contract.batch_size);
    require(
        provenance.fixed_batch_size == expected_batch,
        format!("{}: fixed batch size mismatch", path.display()),
    )
}

fn validate_provenance_framework(
    provenance: &RunProvenance,
    path: &Path,
    expected: &ProvenanceExpectations<'_>,
) -> Result<()> {
    require(
        provenance.frameworks.len() == 1,
        format!("{}: expected one framework", path.display()),
    )?;
    let framework = &provenance.frameworks[0];
    // The run side bakes the execution mode into xberg framework names (batch cells emit
    // `xberg-<fmt>-<pipeline>-batch`), matching how `extract_framework_and_mode` recovers the
    // base name during aggregation; `entry.framework` is the mode-independent base. Compare the
    // stripped base — mode itself is validated separately via `timing.mode`/`fixed_batch_size`.
    require(
        extract_framework_and_mode(&framework.name).0 == expected.entry.framework,
        format!("{}: framework mismatch", path.display()),
    )?;
    require(
        framework.eligible_documents == expected.eligible_documents,
        format!("{}: fixture count mismatch", path.display()),
    )?;
    require(
        framework.batch_partitions == expected.expected_partitions,
        format!("{}: batch partition mismatch", path.display()),
    )?;
    require(
        framework.ocr_language_policy == declared_ocr_language_policy(&expected.entry.framework),
        format!("{}: OCR language policy mismatch", path.display()),
    )?;
    require(
        framework.timing_regime == crate::types::TimingRegime::ColdProcess,
        format!("{}: timing regime mismatch", path.display()),
    )?;
    require(
        framework.resource_measurement_scope == crate::types::ResourceMeasurementScope::IsolatedProcess,
        format!("{}: resource measurement scope mismatch", path.display()),
    )
}

fn validate_provenance(provenance: &RunProvenance, path: &Path, expected: &ProvenanceExpectations<'_>) -> Result<()> {
    validate_provenance_repository_and_corpus(provenance, path, expected)?;
    validate_provenance_timing(provenance, path, expected)?;
    validate_provenance_framework(provenance, path, expected)?;
    Ok(())
}

/// Everything [`validate_results`] checks a framework's `results.json` against, besides the
/// records themselves and their path -- bundled into one struct purely to keep the function's
/// (and its helpers') parameter count down; carries no behavior of its own.
struct ResultExpectations<'a> {
    entry: &'a MatrixEntry,
    iterations: usize,
    cohort: Cohort,
    allow_failures: bool,
}

fn validate_result_set_shape(
    results: &[BenchmarkResult],
    path: &Path,
    expected_fixtures: &[&ExpectedFixture],
) -> Result<()> {
    require(
        results.len() == expected_fixtures.len(),
        format!("{}: result fixture count mismatch", path.display()),
    )?;

    let actual_names: Vec<String> = results
        .iter()
        .map(|result| posix_basename(&result.file_path.to_string_lossy()))
        .collect();
    require(
        actual_names
            .iter()
            .map(String::as_str)
            .eq(expected_fixtures.iter().map(|fixture| fixture.document_name.as_str())),
        format!("{}: result fixture order/content mismatch", path.display()),
    )?;
    let unique_names: HashSet<&String> = actual_names.iter().collect();
    require(
        unique_names.len() == actual_names.len(),
        format!("{}: duplicate fixture results", path.display()),
    )
}

fn validate_result_outcome(
    index: usize,
    result: &BenchmarkResult,
    path: &Path,
    expected_fixture: &ExpectedFixture,
    expected: &ResultExpectations<'_>,
) -> Result<()> {
    if result.success {
        require(
            result.error_kind == ErrorKind::None,
            format!("{}: result {index} has an error", path.display()),
        )?;
        require(
            result.error_message.is_none(),
            format!("{}: result {index} has an error message", path.display()),
        )?;
        validate_release_quality_contract(
            result.quality.as_ref(),
            expected.entry.output_format,
            expected_fixture.has_quality_ground_truth,
            expected_fixture.has_structural_ground_truth,
            path,
            &format!("result {index}"),
        )
    } else {
        require(
            expected.allow_failures,
            format!("{}: result {index} failed", path.display()),
        )?;
        require(
            result.error_kind != ErrorKind::None,
            format!("{}: failed result {index} has no error kind", path.display()),
        )?;
        require(
            matches!(
                result.error_kind,
                ErrorKind::FrameworkError
                    | ErrorKind::Timeout
                    | ErrorKind::EmptyContent
                    | ErrorKind::ZeroOverlap
                    | ErrorKind::LayoutFailed
            ),
            format!(
                "{}: optional result {index} has an infrastructure error",
                path.display()
            ),
        )?;
        require(
            result
                .error_message
                .as_deref()
                .is_some_and(|message| !message.is_empty()),
            format!("{}: failed result {index} has no error message", path.display()),
        )
    }
}

/// The OCR status a result must report, or `None` when its status is not contractual.
///
/// It is contractual only for xberg, the subject under test: the OCR cohort must exercise
/// xberg's OCR pipeline (Used) and every other cohort must not (NotUsed). Auto-OCR variants are
/// exempt in the OCR cohort, because xberg's own trigger decides per page whether to OCR and may
/// decline; they must still have succeeded. External competitors self-report OCR usage from
/// their own internals — some are Tesseract-backed yet legitimately skip OCR on a given file
/// (e.g. Tika reading an embedded text layer) — so their reported status is descriptive data we
/// record, not a contract we enforce.
fn required_ocr_status(framework: &str, cohort_expects_ocr: bool) -> Option<OcrStatus> {
    if !framework.starts_with("xberg-") || (cohort_expects_ocr && is_auto_ocr_framework(framework)) {
        return None;
    }
    Some(if cohort_expects_ocr {
        OcrStatus::Used
    } else {
        OcrStatus::NotUsed
    })
}

fn validate_result_ocr_and_iterations(
    index: usize,
    result: &BenchmarkResult,
    path: &Path,
    expected: &ResultExpectations<'_>,
) -> Result<()> {
    if let Some(expected_ocr) = required_ocr_status(&expected.entry.framework, expected.cohort.expects_ocr()) {
        require(
            result.ocr_status == expected_ocr,
            format!(
                "{}: result {index} OCR status mismatch: expected {expected_ocr:?}, got {:?}",
                path.display(),
                result.ocr_status
            ),
        )?;
    }
    require(
        result.iterations.len() == expected.iterations,
        format!("{}: result {index} iteration count mismatch", path.display()),
    )?;
    let sequential = result
        .iterations
        .iter()
        .enumerate()
        .all(|(expected_index, iteration)| iteration.iteration == expected_index + 1);
    require(
        sequential,
        format!("{}: result {index} iteration order/duplicates mismatch", path.display()),
    )
}

fn validate_one_result(
    index: usize,
    result: &BenchmarkResult,
    path: &Path,
    expected_fixture: &ExpectedFixture,
    expected: &ResultExpectations<'_>,
) -> Result<()> {
    crate::output::validate_result(result)
        .map_err(|error| contract_error(format!("{}: result {index}: {error}", path.display())))?;
    require(
        extract_framework_and_mode(&result.framework).0 == expected.entry.framework,
        format!("{}: result {index} framework mismatch", path.display()),
    )?;
    require(
        result.output_format == expected.entry.output_format,
        format!("{}: result {index} format mismatch", path.display()),
    )?;
    require(
        result.framework_capabilities.timing_regime == crate::types::TimingRegime::ColdProcess,
        format!("{}: result {index} timing regime mismatch", path.display()),
    )?;
    require(
        result.framework_capabilities.resource_measurement_scope
            == crate::types::ResourceMeasurementScope::IsolatedProcess,
        format!("{}: result {index} resource measurement scope mismatch", path.display()),
    )?;
    validate_result_outcome(index, result, path, expected_fixture, expected)?;
    validate_result_ocr_and_iterations(index, result, path, expected)
}

fn validate_results(
    results: &[BenchmarkResult],
    path: &Path,
    expected_fixtures: &[&ExpectedFixture],
    expected: &ResultExpectations<'_>,
) -> Result<()> {
    validate_result_set_shape(results, path, expected_fixtures)?;
    for (index, result) in results.iter().enumerate() {
        validate_one_result(index, result, path, expected_fixtures[index], expected)?;
    }
    Ok(())
}

/// [`validate_raw_artifacts`]'s inputs, bundled into one struct purely to keep its (and its
/// helpers') parameter count down; carries no behavior of its own.
pub(super) struct RawArtifactsArgs<'a> {
    pub(super) artifacts_dir: &'a Path,
    pub(super) cohort_manifest: &'a Path,
    pub(super) fixtures_root: &'a Path,
    pub(super) source_sha: &'a str,
    pub(super) run_id: &'a str,
    pub(super) iterations: usize,
    pub(super) cohort: Cohort,
    pub(super) contract: &'a CohortContract,
}

/// Reads `artifacts_dir` and checks its subdirectory names against the required and allowed
/// artifact-name sets, returning the directories keyed by artifact name.
fn validate_artifact_directory_set(
    artifacts_dir: &Path,
    allowed_names: &HashMap<String, &MatrixEntry>,
    required_names: &HashSet<String>,
) -> Result<HashMap<String, PathBuf>> {
    let actual_dirs = read_artifact_dirs(artifacts_dir)?;

    let allowed_key_set: HashSet<&str> = allowed_names.keys().map(String::as_str).collect();
    let actual_key_set: HashSet<&str> = actual_dirs.keys().map(String::as_str).collect();
    let required_key_set: HashSet<&str> = required_names.iter().map(String::as_str).collect();
    let present_required: HashSet<&str> = actual_key_set.intersection(&required_key_set).copied().collect();
    require(
        required_key_set == present_required,
        describe_set_mismatch(
            "artifacts",
            required_key_set.iter().copied(),
            present_required.iter().copied(),
        ),
    )?;
    require(
        actual_key_set.is_subset(&allowed_key_set),
        describe_set_mismatch(
            "allowed artifacts",
            allowed_key_set.iter().copied(),
            actual_key_set.iter().copied(),
        ),
    )?;
    Ok(actual_dirs)
}

/// Context shared by every artifact directory validated within one [`validate_raw_artifacts`]
/// call -- bundled into one struct purely to keep [`validate_one_raw_artifact`]'s parameter count
/// down; carries no behavior of its own.
struct RawArtifactContext<'a> {
    contract: &'a CohortContract,
    fixtures: &'a [ExpectedFixture],
    manifest_blake3: &'a str,
    source_sha: &'a str,
    iterations: usize,
    cohort: Cohort,
}

/// Validates one downloaded artifact directory's `provenance.json` and `results.json` against the
/// release contract. Optional means absence is permitted, never that present bytes are trusted:
/// this runs identically for a required or an optional (best-effort) artifact. ~keep
fn validate_one_raw_artifact(artifact_dir: &Path, entry: &MatrixEntry, context: &RawArtifactContext<'_>) -> Result<()> {
    let results_path = only_file(artifact_dir, "results.json")?;
    let provenance_path = only_file(artifact_dir, "provenance.json")?;
    let ocr_languages: Vec<Option<String>> = context
        .fixtures
        .iter()
        .map(|fixture| fixture.ocr_language.clone())
        .collect();
    let supported_indexes = supported_fixture_indexes(entry, context.contract, &ocr_languages);
    let selected_fixtures: Vec<&ExpectedFixture> = supported_indexes
        .iter()
        .map(|index| &context.fixtures[*index])
        .collect();
    let eligible_languages: Vec<Option<String>> = supported_indexes
        .iter()
        .map(|index| ocr_languages[*index].clone())
        .collect();
    let expected_partitions = matches!(entry.mode, ExecutionMode::Batch).then(|| {
        declared_ocr_language_policy(&entry.framework)
            .batch_partition_count(&eligible_languages, context.contract.batch_size)
    });

    let provenance: RunProvenance = load_typed_json(&provenance_path)?;
    validate_provenance(
        &provenance,
        &provenance_path,
        &ProvenanceExpectations {
            entry,
            contract: context.contract,
            manifest_blake3: context.manifest_blake3,
            fixtures: context.fixtures,
            source_sha: context.source_sha,
            iterations: context.iterations,
            eligible_documents: supported_indexes.len(),
            expected_partitions,
        },
    )?;

    let results: Vec<BenchmarkResult> = load_typed_json(&results_path)?;
    validate_results(
        &results,
        &results_path,
        &selected_fixtures,
        &ResultExpectations {
            entry,
            iterations: context.iterations,
            cohort: context.cohort,
            allow_failures: entry.optional,
        },
    )
}

pub(super) fn validate_raw_artifacts(args: &RawArtifactsArgs<'_>) -> Result<String> {
    let manifest_blake3 = validate_manifest(args.cohort_manifest, args.contract)?;
    let fixtures = expected_fixtures(args.fixtures_root, args.contract)?;
    let allowed_names: HashMap<String, &MatrixEntry> = args
        .contract
        .matrix
        .iter()
        .map(|entry| (format!("{}-{}", entry.artifact, args.run_id), entry))
        .collect();
    let required_names: HashSet<String> = args
        .contract
        .matrix
        .iter()
        .filter(|entry| !entry.optional)
        .map(|entry| format!("{}-{}", entry.artifact, args.run_id))
        .collect();
    let actual_dirs = validate_artifact_directory_set(args.artifacts_dir, &allowed_names, &required_names)?;

    let context = RawArtifactContext {
        contract: args.contract,
        fixtures: &fixtures,
        manifest_blake3: &manifest_blake3,
        source_sha: args.source_sha,
        iterations: args.iterations,
        cohort: args.cohort,
    };
    for (artifact_name, artifact_dir) in &actual_dirs {
        let entry = allowed_names[artifact_name];
        validate_one_raw_artifact(artifact_dir, entry, &context)?;
    }

    Ok(format!(
        "validated {} {} benchmark artifacts",
        actual_dirs.len(),
        args.cohort.as_str()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xberg_must_use_ocr_in_ocr_cohorts_and_not_elsewhere() {
        assert_eq!(
            required_ocr_status("xberg-markdown-baseline", true),
            Some(OcrStatus::Used)
        );
        assert_eq!(
            required_ocr_status("xberg-markdown-baseline-batch", false),
            Some(OcrStatus::NotUsed)
        );
        assert_eq!(required_ocr_status("docling", true), None);
    }

    /// Auto-OCR rows may decline to OCR, so their status is exempt in the OCR cohort only. ~keep
    #[test]
    fn auto_ocr_rows_are_exempt_from_the_used_requirement_only_in_ocr_cohorts() {
        for framework in [
            "xberg-markdown-auto-ocr",
            "xberg-markdown-auto-ocr-batch",
            "xberg-plaintext-auto-ocr-paddle",
            "xberg-markdown-auto-ocr-paddle-batch",
        ] {
            assert_eq!(required_ocr_status(framework, true), None, "{framework}");
            assert_eq!(
                required_ocr_status(framework, false),
                Some(OcrStatus::NotUsed),
                "{framework}"
            );
        }
        assert_eq!(
            required_ocr_status("xberg-markdown-paddle-ocr", true),
            Some(OcrStatus::Used)
        );
    }
}
