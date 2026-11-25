//! Aggregate mode: validates one consolidated `aggregated.json` (as produced by the
//! `consolidate` subcommand) against the release contract.
//!
//! Split out of `validate_artifacts.rs` purely to keep that file under the repo's
//! file-length limit; `use super::*` pulls in the shared kernel (types, `require`/
//! `contract_error`, JSON loaders, etc.) defined there.

use super::*;

mod paired;

/// Validate one file-type bucket's failure accounting and report its sample count. Required cells
/// must have no errors; optional cells may contain only framework-accountable failures. Cohorts
/// can span several file types, so the caller also checks each bucket against the cohort's exact
/// per-extension fixture counts. ~keep
fn validate_bucket(bucket: &PerformancePercentiles, key: &str, allow_failures: bool) -> Result<usize> {
    // `zero_overlap` is a framework fault like the other three: `CountsBuilder::record` folds it
    // into `framework_fault_total` and `accountable_sample_count` includes it, but this sum
    // omitted it, so any bucket holding one failed the invariant against a `total_sample_count`
    // that counts every result. ~keep
    let accountable_failures =
        bucket.framework_errors + bucket.timeouts + bucket.empty_content + bucket.zero_overlap + bucket.layout_failures;
    let infrastructure_failures = bucket.harness_errors + bucket.config_setup_errors;
    let failures = accountable_failures + infrastructure_failures;
    require(
        bucket.successful_sample_count + failures == bucket.total_sample_count,
        format!("{key}: success/error counts do not match total_sample_count"),
    )?;
    require(
        allow_failures || failures == 0,
        format!("{key}: required aggregate contains failures"),
    )?;
    require(
        infrastructure_failures == 0,
        format!("{key}: aggregate contains infrastructure failures"),
    )?;
    Ok(bucket.total_sample_count)
}

fn identity_string(framework: &str, output_format: OutputFormat, mode: &str, fixture_id: &str) -> String {
    // (framework, output_format, mode, fixture_id) already uniquely identifies a row within a cohort.
    // OCR usage is intentionally excluded: it is contractual only for xberg (enforced per-result and
    // per-bucket elsewhere) and merely descriptive for competitors, whose reported side we do not gate.
    format!("{framework}:{output_format}:{mode}:{fixture_id}")
}

fn logical_framework(framework: &str) -> &str {
    if framework.starts_with("xberg-") {
        "xberg"
    } else {
        framework
    }
}

fn validate_format_support(
    aggregate: &NewConsolidatedResults,
    present_entries: &[&MatrixEntry],
    contract: &CohortContract,
    path: &Path,
) -> Result<()> {
    let mut expected_file_types: Vec<String> = contract
        .document_extensions
        .iter()
        .map(|extension| (*extension).to_string())
        .collect();
    expected_file_types.sort();
    expected_file_types.dedup();
    require(
        aggregate.format_support.file_types == expected_file_types,
        format!("{}: format support file types mismatch", path.display()),
    )?;

    let frameworks: std::collections::BTreeSet<&str> = present_entries
        .iter()
        .map(|entry| logical_framework(&entry.framework))
        .collect();
    let mut expected_unsupported = std::collections::BTreeMap::new();
    for framework in frameworks {
        if framework == "xberg" {
            continue;
        }
        let supported = crate::adapters::external::declared_supported_formats(framework);
        let missing: Vec<String> = expected_file_types
            .iter()
            .filter(|extension| !supported.iter().any(|item| item == *extension))
            .cloned()
            .collect();
        if !missing.is_empty() {
            expected_unsupported.insert(framework.to_string(), missing);
        }
    }
    require(
        aggregate.format_support.unsupported == expected_unsupported,
        format!("{}: format support unsupported pairs mismatch", path.display()),
    )
}

/// Context shared by every `run_provenance` record validated within one
/// [`validate_aggregate_provenance`] call -- bundled into one struct purely to keep
/// [`validate_one_aggregate_provenance_record`]'s parameter count down; carries no behavior of
/// its own.
struct AggregateProvenanceContext<'a> {
    expected_entries: &'a HashMap<String, &'a MatrixEntry>,
    contract: &'a CohortContract,
    iterations: usize,
    fixture_expectations: &'a [FixtureQualityExpectation],
    path: &'a Path,
}

/// Checks one record's schema version, checkout cleanliness, and source commit, and records the
/// commit into the running `source_sha` cross-record invariant (every record in one aggregate
/// must share the same source commit). Returns the commit.
fn validate_record_commit<'a>(
    provenance: &'a RunProvenance,
    path: &Path,
    source_sha: &mut Option<&'a str>,
) -> Result<&'a str> {
    require(
        provenance.schema_version == EXPECTED_PROVENANCE_SCHEMA_VERSION,
        format!("{}: aggregate provenance schema mismatch", path.display()),
    )?;
    require(
        provenance.repository.dirty == Some(false),
        format!("{}: aggregate provenance records a dirty checkout", path.display()),
    )?;
    let commit = provenance
        .repository
        .commit
        .as_deref()
        .ok_or_else(|| contract_error(format!("{}: aggregate provenance has no source commit", path.display())))?;
    require(
        commit.len() == 40 && commit.bytes().all(|byte| byte.is_ascii_hexdigit()),
        format!("{}: aggregate provenance has an invalid source commit", path.display()),
    )?;
    require(
        source_sha.is_none_or(|expected| expected == commit),
        format!("{}: aggregate provenance mixes source commits", path.display()),
    )?;
    source_sha.get_or_insert(commit);
    Ok(commit)
}

/// Checks one record's cohort identity and fixture-list signature, and records the signature into
/// the running `fixture_signature` cross-record invariant (every record in one aggregate must
/// share the same fixture identity).
fn validate_record_fixture_signature<'a>(
    provenance: &'a RunProvenance,
    contract: &CohortContract,
    path: &Path,
    fixture_signature: &mut Option<Vec<(&'a str, &'a str, &'a str, u64)>>,
) -> Result<()> {
    require(
        provenance.corpus.cohort.as_deref() == Some(contract.manifest_name)
            && provenance.corpus.cohort_manifest_blake3.as_deref() == Some(contract.manifest_blake3),
        format!("{}: aggregate provenance cohort mismatch", path.display()),
    )?;
    let signature: Vec<(&str, &str, &str, u64)> = provenance
        .corpus
        .ordered_fixtures
        .iter()
        .map(|fixture| {
            (
                fixture.fixture.as_str(),
                fixture.fixture_blake3.as_str(),
                fixture.document_blake3.as_str(),
                fixture.document_bytes,
            )
        })
        .collect();
    require(
        signature.len() == contract.fixtures.len()
            && signature
                .iter()
                .zip(contract.fixtures)
                .all(|((fixture, _, _, _), expected)| fixture == expected)
            && fixture_signature.as_ref().is_none_or(|expected| expected == &signature),
        format!("{}: aggregate provenance fixture identity mismatch", path.display()),
    )?;
    fixture_signature.get_or_insert(signature);
    Ok(())
}

/// Resolves which aggregate cell (and matrix entry) a record's single framework belongs to.
fn resolve_record_cell<'ctx>(
    provenance: &RunProvenance,
    context: &AggregateProvenanceContext<'ctx>,
) -> Result<(String, &'ctx MatrixEntry)> {
    let path = context.path;
    require(
        provenance.frameworks.len() == 1,
        format!("{}: aggregate provenance must contain one framework", path.display()),
    )?;
    let framework = &provenance.frameworks[0];
    let mode = match provenance.timing.mode {
        crate::config::BenchmarkMode::SingleFile => "single",
        crate::config::BenchmarkMode::Batch => "batch",
    };
    let cell = format!(
        "{}:{}:{}",
        extract_framework_and_mode(&framework.name).0,
        provenance.timing.output_format,
        mode
    );
    let entry = *context.expected_entries.get(&cell).ok_or_else(|| {
        contract_error(format!(
            "{}: unexpected aggregate provenance cell {cell}",
            path.display()
        ))
    })?;
    Ok((cell, entry))
}

/// Checks a record's timing/eligibility/batch-partition/OCR-policy settings against its resolved
/// matrix entry.
fn validate_record_settings(
    provenance: &RunProvenance,
    entry: &MatrixEntry,
    cell: &str,
    context: &AggregateProvenanceContext<'_>,
) -> Result<()> {
    let contract = context.contract;
    let ocr_languages: Vec<Option<String>> = context
        .fixture_expectations
        .iter()
        .map(|fixture| fixture.ocr_language.clone())
        .collect();
    let supported_indexes = supported_fixture_indexes(entry, contract, &ocr_languages);
    let expected_eligible = supported_indexes.len();
    let eligible_languages: Vec<Option<String>> = supported_indexes
        .iter()
        .map(|index| ocr_languages[*index].clone())
        .collect();
    let is_batch = entry.mode == ExecutionMode::Batch;
    let framework = &provenance.frameworks[0];
    require(
        provenance.timing.mode == entry.mode.benchmark_mode()
            && provenance.timing.benchmark_iterations == context.iterations
            && provenance.fixed_batch_size == is_batch.then_some(contract.batch_size)
            && framework.eligible_documents == expected_eligible
            && framework.batch_partitions
                == is_batch.then(|| {
                    declared_ocr_language_policy(&entry.framework)
                        .batch_partition_count(&eligible_languages, contract.batch_size)
                })
            && framework.ocr_language_policy == declared_ocr_language_policy(&entry.framework)
            && framework.timing_regime == crate::types::TimingRegime::ColdProcess,
        format!(
            "{}: aggregate provenance settings mismatch for {cell}",
            context.path.display()
        ),
    )?;
    require(
        framework.resource_measurement_scope == crate::types::ResourceMeasurementScope::IsolatedProcess,
        format!(
            "{}: aggregate provenance resource measurement scope mismatch for {cell}",
            context.path.display()
        ),
    )
}

/// Validates one `run_provenance` record and returns its aggregate cell key on success. Also
/// checks it against, and records it into, the running `source_sha`/`fixture_signature`
/// cross-record invariants (every record in one aggregate must share the same source commit and
/// fixture identity).
fn validate_one_aggregate_provenance_record<'a>(
    record: &'a crate::consolidate::RunProvenanceRecord,
    context: &AggregateProvenanceContext<'_>,
    source_sha: &mut Option<&'a str>,
    fixture_signature: &mut Option<Vec<(&'a str, &'a str, &'a str, u64)>>,
) -> Result<String> {
    let path = context.path;
    require(
        record.missing_reason.is_none(),
        format!("{}: aggregate provenance contains a missing reason", path.display()),
    )?;
    let provenance = record.provenance.as_ref().ok_or_else(|| {
        contract_error(format!(
            "{}: aggregate provenance missing for {}",
            path.display(),
            record.source_dir
        ))
    })?;
    validate_record_commit(provenance, path, source_sha)?;
    validate_record_fixture_signature(provenance, context.contract, path, fixture_signature)?;
    let (cell, entry) = resolve_record_cell(provenance, context)?;
    validate_record_settings(provenance, entry, &cell, context)?;
    Ok(cell)
}

fn validate_aggregate_provenance(
    aggregate: &NewConsolidatedResults,
    present_entries: &[&MatrixEntry],
    contract: &CohortContract,
    iterations: usize,
    fixture_expectations: &[FixtureQualityExpectation],
    path: &Path,
) -> Result<()> {
    require(
        aggregate.run_provenance.len() == present_entries.len(),
        format!("{}: aggregate provenance count mismatch", path.display()),
    )?;
    let expected_entries: HashMap<String, &MatrixEntry> = present_entries
        .iter()
        .map(|entry| {
            (
                format!(
                    "{}:{}:{}",
                    entry.framework,
                    entry.output_format,
                    entry.mode.aggregate_slug()
                ),
                *entry,
            )
        })
        .collect();
    let expected_cells: HashSet<String> = expected_entries.keys().cloned().collect();
    let context = AggregateProvenanceContext {
        expected_entries: &expected_entries,
        contract,
        iterations,
        fixture_expectations,
        path,
    };

    let mut actual_cells = HashSet::new();
    let mut source_sha: Option<&str> = None;
    let mut fixture_signature: Option<Vec<(&str, &str, &str, u64)>> = None;
    for record in &aggregate.run_provenance {
        let cell = validate_one_aggregate_provenance_record(record, &context, &mut source_sha, &mut fixture_signature)?;
        actual_cells.insert(cell);
    }
    require(
        actual_cells == expected_cells,
        describe_set_mismatch(
            "aggregate provenance cells",
            expected_cells.iter().map(String::as_str),
            actual_cells.iter().map(String::as_str),
        ),
    )
}

fn error_kind_from_row(row: &PerFixtureRow, path: &Path) -> Result<ErrorKind> {
    match row.error_kind.as_deref() {
        None if row.success => Ok(ErrorKind::None),
        Some("FrameworkError") => Ok(ErrorKind::FrameworkError),
        Some("EmptyContent") => Ok(ErrorKind::EmptyContent),
        Some("ZeroOverlap") => Ok(ErrorKind::ZeroOverlap),
        Some("LayoutFailed") => Ok(ErrorKind::LayoutFailed),
        Some("Timeout") => Ok(ErrorKind::Timeout),
        Some("HarnessError") => Ok(ErrorKind::HarnessError),
        Some("ConfigSetupError") => Ok(ErrorKind::ConfigSetupError),
        kind => Err(contract_error(format!(
            "{}: invalid fixture-row error kind {kind:?}",
            path.display()
        ))),
    }
}

fn duration_from_millis(value: f64, field: &str, path: &Path) -> Result<Duration> {
    require(
        value.is_finite() && value >= 0.0,
        format!("{}: invalid fixture-row {field}", path.display()),
    )?;
    Ok(Duration::from_secs_f64(value / 1_000.0))
}

fn performance_metrics_from_row(row: &PerFixtureRow, path: &Path) -> Result<PerformanceMetrics> {
    let peak_memory_bytes = row.peak_memory_mb * 1_000_000.0;
    require(
        peak_memory_bytes.is_finite() && peak_memory_bytes >= 0.0 && peak_memory_bytes <= u64::MAX as f64,
        format!("{}: invalid fixture-row peak memory", path.display()),
    )?;
    Ok(PerformanceMetrics {
        baseline_memory_bytes: row.baseline_memory_bytes,
        peak_memory_bytes: peak_memory_bytes.round() as u64,
        memory_method: row.memory_method,
        self_reported_peak_memory_bytes: row.self_reported_peak_memory_bytes,
        peak_memory_delta_bytes: row.peak_memory_delta_bytes,
        avg_cpu_percent: row.avg_cpu_percent,
        cpu_seconds: row.cpu_seconds,
        throughput_bytes_per_sec: row.throughput_bytes_per_sec,
        p50_memory_bytes: row.p50_memory_bytes,
        p95_memory_bytes: row.p95_memory_bytes,
        p99_memory_bytes: row.p99_memory_bytes,
    })
}

fn validate_quality_projection(row: &PerFixtureRow, path: &Path) -> Result<()> {
    if let Some(quality) = &row.quality {
        // This strict format rule is intentionally scoped to the current aggregated release
        // schema. Generic `results.json` loading/writing remains backward-compatible with older
        // captures that populated layout F1 for plaintext rows without a schema version. ~keep
        require(
            row.output_format != OutputFormat::Plaintext || quality.f1_score_layout.is_none(),
            format!(
                "{}: plaintext fixture-row quality must not contain f1_score_layout",
                path.display()
            ),
        )?;
        for (name, value) in [
            ("f1_score_text", quality.f1_score_text),
            ("f1_score_numeric", quality.f1_score_numeric),
            ("quality_score", quality.quality_score),
        ] {
            require(
                value.is_finite() && (0.0..=1.0).contains(&value),
                format!(
                    "{}: fixture-row {name} must be a finite value in [0, 1]",
                    path.display()
                ),
            )?;
        }
        if let Some(value) = quality.f1_score_layout {
            require(
                value.is_finite() && (0.0..=1.0).contains(&value),
                format!(
                    "{}: fixture-row f1_score_layout must be a finite value in [0, 1]",
                    path.display()
                ),
            )?;
        }
    }
    let quality_projection = row.quality.as_ref().map(|quality| {
        (
            Some(quality.f1_score_text),
            quality.f1_score_layout,
            Some(quality.f1_score_numeric),
            Some(quality.quality_score),
            Some(quality.correct),
        )
    });
    require(
        quality_projection.unwrap_or((None, None, None, None, None))
            == (
                row.f1_text,
                row.f1_layout,
                row.f1_numeric,
                row.quality_score,
                row.correct,
            ),
        format!("{}: fixture-row quality projection mismatch", path.display()),
    )
}

fn benchmark_result_from_row(row: &PerFixtureRow, path: &Path) -> Result<BenchmarkResult> {
    validate_quality_projection(row, path)?;
    let framework = match row.execution_mode.as_str() {
        "batch" => format!("{}-batch", row.framework),
        _ => row.framework.clone(),
    };
    let ocr_status = match row.ocr {
        Some(true) => OcrStatus::Used,
        Some(false) => OcrStatus::NotUsed,
        None => OcrStatus::Unknown,
    };
    let extraction_duration = row
        .extraction_duration_ms
        .map(|value| duration_from_millis(value, "extraction duration", path))
        .transpose()?;
    let subprocess_overhead = row
        .subprocess_overhead_ms
        .map(|value| duration_from_millis(value, "subprocess overhead", path))
        .transpose()?;
    let cold_start_duration = row
        .cold_start_duration_ms
        .map(|value| duration_from_millis(value, "cold start duration", path))
        .transpose()?;
    Ok(BenchmarkResult {
        framework,
        output_format: row.output_format,
        file_path: PathBuf::from(format!("{}.{}", row.fixture_id, row.file_type)),
        file_size: row.file_size,
        success: row.success,
        error_message: row.error_message.clone(),
        error_kind: error_kind_from_row(row, path)?,
        duration: duration_from_millis(row.duration_ms, "duration", path)?,
        extraction_duration,
        subprocess_overhead,
        metrics: performance_metrics_from_row(row, path)?,
        quality: row.quality.clone(),
        iterations: row.iterations.clone(),
        statistics: row.statistics.clone(),
        cold_start_duration,
        file_extension: row.file_type.clone(),
        framework_capabilities: row.framework_capabilities.clone(),
        pdf_metadata: row.pdf_metadata.clone(),
        ocr_status,
        extracted_text: None,
        system_load: row.system_load,
    })
}

fn require_serialized_equal<T: Serialize>(actual: &T, expected: &T, label: &str, path: &Path) -> Result<()> {
    let actual = serde_json::to_value(actual)
        .map_err(|error| contract_error(format!("{}: serialize {label}: {error}", path.display())))?;
    let expected = serde_json::to_value(expected)
        .map_err(|error| contract_error(format!("{}: serialize expected {label}: {error}", path.display())))?;
    require(actual == expected, format!("{}: {label} mismatch", path.display()))
}

fn validate_derived_aggregate_fields(
    aggregate: &NewConsolidatedResults,
    rows: &[&PerFixtureRow],
    cohort: Cohort,
    path: &Path,
) -> Result<()> {
    let results: Vec<BenchmarkResult> = rows
        .iter()
        .map(|row| benchmark_result_from_row(row, path))
        .collect::<Result<_>>()?;
    let rebuilt = crate::aggregate::aggregate_new_format(&results);
    require_serialized_equal(
        &aggregate.by_framework_mode,
        &rebuilt.by_framework_mode,
        "aggregate metrics",
        path,
    )?;
    require_serialized_equal(&aggregate.disk_sizes, &rebuilt.disk_sizes, "disk sizes", path)?;
    require_serialized_equal(
        &aggregate.format_support,
        &rebuilt.format_support,
        "format support",
        path,
    )?;
    require_serialized_equal(
        &aggregate.failure_summary,
        &rebuilt.failure_summary,
        "failure summary",
        path,
    )?;
    require_serialized_equal(&aggregate.layout_impact, &rebuilt.layout_impact, "layout impact", path)?;
    require_serialized_equal(&aggregate.head_to_head, &rebuilt.head_to_head, "head-to-head", path)?;
    let expected_comparison = comparison_for_cohort(&rebuilt.by_framework_mode, cohort);
    require_serialized_equal(&aggregate.comparison, &expected_comparison, "comparison rankings", path)?;
    require(
        aggregate.metadata.total_results == rebuilt.metadata.total_results
            && aggregate.metadata.framework_count == rebuilt.metadata.framework_count
            && aggregate.metadata.file_type_count == rebuilt.metadata.file_type_count
            && aggregate.metadata.shared_corpus_markdown == rebuilt.metadata.shared_corpus_markdown
            && aggregate.metadata.shared_corpus_plaintext == rebuilt.metadata.shared_corpus_plaintext
            && chrono::DateTime::parse_from_rfc3339(&aggregate.metadata.timestamp).is_ok()
            && aggregate.metadata.disk_size_conflicts == rebuilt.metadata.disk_size_conflicts,
        format!("{}: consolidation metadata mismatch", path.display()),
    )
}

/// Shared, read-only context for the per-group and per-row helpers below -- bundled into one
/// struct purely to keep their parameter counts down; carries no behavior of its own.
struct AggregateValidationContext<'a> {
    contract: &'a CohortContract,
    quality_expectations: &'a [FixtureQualityExpectation],
    expects_ocr: bool,
    path: &'a Path,
}

/// (allowed entries by aggregate key, entries actually present, actual aggregate keys)
type AggregateKeyCoverage<'a> = (HashMap<String, &'a MatrixEntry>, Vec<&'a MatrixEntry>, HashSet<&'a str>);

/// Computes the aggregate's allowed/present entries and checks the required/allowed key-set
/// invariants against `aggregate.by_framework_mode`'s actual keys.
fn validate_aggregate_key_coverage<'a>(
    aggregate: &'a NewConsolidatedResults,
    contract: &'a CohortContract,
) -> Result<AggregateKeyCoverage<'a>> {
    let required_entries: Vec<&MatrixEntry> = contract.matrix.iter().filter(|entry| !entry.optional).collect();
    let allowed_entries: HashMap<String, &MatrixEntry> = contract
        .matrix
        .iter()
        .map(|entry| (entry.aggregate_key(), entry))
        .collect();

    let expected_keys: HashSet<String> = required_entries.iter().map(|entry| entry.aggregate_key()).collect();
    let allowed_keys: HashSet<&str> = allowed_entries.keys().map(String::as_str).collect();
    let actual_keys: HashSet<&str> = aggregate.by_framework_mode.keys().map(String::as_str).collect();
    let required_keys: HashSet<&str> = expected_keys.iter().map(String::as_str).collect();
    let present_required: HashSet<&str> = actual_keys.intersection(&required_keys).copied().collect();
    require(
        required_keys == present_required,
        describe_set_mismatch(
            "aggregate keys",
            required_keys.iter().copied(),
            present_required.iter().copied(),
        ),
    )?;
    require(
        actual_keys.is_subset(&allowed_keys),
        describe_set_mismatch(
            "allowed aggregate keys",
            allowed_keys.iter().copied(),
            actual_keys.iter().copied(),
        ),
    )?;

    let present_entries: Vec<&MatrixEntry> = actual_keys.iter().map(|key| allowed_entries[*key]).collect();
    Ok((allowed_entries, present_entries, actual_keys))
}

/// xberg (the subject under test) has a uniform OCR expectation, so its fixtures must all sit on
/// the cohort's expected OCR side with the opposite side empty, and its per-extension sample
/// cardinality must match the contract exactly.
fn validate_xberg_aggregate_group(
    key: &str,
    group: &crate::aggregate::FrameworkModeAggregation,
    entry: &MatrixEntry,
    context: &AggregateValidationContext<'_>,
) -> Result<()> {
    let path = context.path;
    let contract = context.contract;
    let mut actual_ext_counts: HashMap<&str, usize> = HashMap::new();
    for (file_type, file_group) in &group.by_file_type {
        let (present, absent) = if context.expects_ocr {
            (&file_group.with_ocr, &file_group.no_ocr)
        } else {
            (&file_group.no_ocr, &file_group.with_ocr)
        };
        require(
            absent.is_none(),
            format!(
                "{}: group {key} file type {file_type} has wrong OCR bucket",
                path.display()
            ),
        )?;
        let bucket = present.as_ref().ok_or_else(|| {
            contract_error(format!(
                "{}: group {key} file type {file_type} missing OCR bucket",
                path.display()
            ))
        })?;
        actual_ext_counts.insert(file_type.as_str(), validate_bucket(bucket, key, entry.optional)?);
    }
    let ocr_languages: Vec<Option<String>> = context
        .quality_expectations
        .iter()
        .map(|fixture| fixture.ocr_language.clone())
        .collect();
    let supported_indexes = supported_fixture_indexes(entry, contract, &ocr_languages);
    let mut entry_ext_counts: HashMap<&str, usize> = HashMap::new();
    for index in supported_indexes {
        *entry_ext_counts.entry(contract.document_extensions[index]).or_insert(0) += 1;
    }
    let expected_set: HashSet<&str> = entry_ext_counts.keys().copied().collect();
    let actual_set: HashSet<&str> = actual_ext_counts.keys().copied().collect();
    require(
        expected_set == actual_set,
        format!(
            "{}: group {key} {}",
            path.display(),
            describe_set_mismatch(
                "file-type buckets",
                expected_set.iter().copied(),
                actual_set.iter().copied()
            )
        ),
    )?;
    for (extension, expected_count) in &entry_ext_counts {
        let actual = actual_ext_counts.get(extension).copied().unwrap_or(0);
        require(
            actual == *expected_count,
            format!(
                "{}: group {key} file type {extension} covers {actual} samples, expected {expected_count}",
                path.display()
            ),
        )?;
    }
    Ok(())
}

/// A present best-effort group must retain exact supported-format cardinality and valid failure
/// accounting; only its absence and well-categorized extraction failures are optional. Competitors
/// self-report OCR usage from their own internals -- a single framework can straddle both sides
/// across a cohort, and a failed extraction reports Unknown (in neither OCR bucket) -- so we only
/// integrity-check whatever buckets they populated; their fixture coverage is enforced exactly by
/// the per-fixture-row identity set, which is OCR-status agnostic. ~keep
fn validate_aggregate_groups(
    aggregate: &NewConsolidatedResults,
    allowed_entries: &HashMap<String, &MatrixEntry>,
    context: &AggregateValidationContext<'_>,
) -> Result<()> {
    for (key, group) in &aggregate.by_framework_mode {
        let entry = allowed_entries[key];
        require(
            group.timing_regime == crate::types::TimingRegime::ColdProcess,
            format!("{}: group {key} timing regime mismatch", context.path.display()),
        )?;
        require(
            !group.by_file_type.is_empty(),
            format!("{}: group {key} has no file-type metrics", context.path.display()),
        )?;
        if entry.framework.starts_with("xberg-") {
            validate_xberg_aggregate_group(key, group, entry, context)?;
        } else {
            for file_group in group.by_file_type.values() {
                for side in [&file_group.no_ocr, &file_group.with_ocr].into_iter().flatten() {
                    validate_bucket(side, key, entry.optional)?;
                }
            }
        }
    }
    Ok(())
}

/// The exact set of `framework:format:mode:fixture` identities the release contract expects to
/// see as aggregate fixture rows, given which entries are actually present.
fn expected_aggregate_row_identities(
    present_entries: &[&MatrixEntry],
    contract: &CohortContract,
    quality_expectations: &[FixtureQualityExpectation],
) -> HashSet<String> {
    let ocr_languages: Vec<Option<String>> = quality_expectations
        .iter()
        .map(|fixture| fixture.ocr_language.clone())
        .collect();
    present_entries
        .iter()
        .flat_map(|entry| {
            supported_fixture_indexes(entry, contract, &ocr_languages)
                .into_iter()
                .map(move |index| {
                    identity_string(
                        &entry.framework,
                        entry.output_format,
                        entry.mode.aggregate_slug(),
                        contract.document_stems[index],
                    )
                })
        })
        .collect()
}

/// Validates one aggregate fixture row's success/failure shape and, when successful, its quality
/// contract.
fn validate_one_aggregate_row(
    row: &PerFixtureRow,
    allowed_entries: &HashMap<String, &MatrixEntry>,
    context: &AggregateValidationContext<'_>,
) -> Result<()> {
    let path = context.path;
    let contract = context.contract;
    let key = crate::aggregate::make_aggregate_key(&row.framework, row.output_format, &row.execution_mode);
    let entry = allowed_entries
        .get(&key)
        .ok_or_else(|| contract_error(format!("{}: row has no matching aggregate key", path.display())))?;
    require(
        row.timing_regime == crate::types::TimingRegime::ColdProcess
            && row.framework_capabilities.timing_regime == row.timing_regime,
        format!("{}: fixture row timing regime mismatch", path.display()),
    )?;
    require(
        row.framework_capabilities.resource_measurement_scope
            == crate::types::ResourceMeasurementScope::IsolatedProcess,
        format!("{}: fixture row resource measurement scope mismatch", path.display()),
    )?;
    let fixture_index = contract
        .document_stems
        .iter()
        .position(|stem| *stem == row.fixture_id)
        .ok_or_else(|| contract_error(format!("{}: row has unknown fixture identity", path.display())))?;
    let quality_expectation = &context.quality_expectations[fixture_index];
    if row.success {
        require(
            row.error_kind.is_none() && row.error_message.is_none(),
            format!("{}: successful fixture row contains an error", path.display()),
        )?;
        validate_release_quality_contract(
            row.quality.as_ref(),
            row.output_format,
            quality_expectation.has_quality_ground_truth,
            quality_expectation.has_structural_ground_truth,
            path,
            &format!("fixture row {}", row.fixture_id),
        )
    } else {
        require(
            entry.optional,
            format!("{}: failed required fixture row", path.display()),
        )?;
        require(
            row.error_kind.as_deref().is_some_and(|kind| {
                matches!(
                    kind,
                    "FrameworkError" | "Timeout" | "EmptyContent" | "ZeroOverlap" | "LayoutFailed"
                )
            }),
            format!(
                "{}: failed fixture rows: optional row has no accountable error kind",
                path.display()
            ),
        )?;
        require(
            row.error_message.as_deref().is_some_and(|message| !message.is_empty()),
            format!(
                "{}: failed fixture rows: optional row has no error message",
                path.display()
            ),
        )
    }
}

/// Checks the aggregate's fixture rows against the exact set the release contract expects
/// (count and identity), then validates each row individually.
fn validate_aggregate_rows(
    rows: &[&PerFixtureRow],
    present_entries: &[&MatrixEntry],
    allowed_entries: &HashMap<String, &MatrixEntry>,
    context: &AggregateValidationContext<'_>,
) -> Result<()> {
    let expected_identities =
        expected_aggregate_row_identities(present_entries, context.contract, context.quality_expectations);
    require(
        rows.len() == expected_identities.len(),
        format!(
            "{}: expected {} fixture rows",
            context.path.display(),
            expected_identities.len()
        ),
    )?;

    let identities: HashSet<String> = rows
        .iter()
        .map(|row| identity_string(&row.framework, row.output_format, &row.execution_mode, &row.fixture_id))
        .collect();
    require(
        identities == expected_identities,
        describe_set_mismatch(
            "aggregate fixture rows",
            expected_identities.iter().map(String::as_str),
            identities.iter().map(String::as_str),
        ),
    )?;

    for row in rows {
        validate_one_aggregate_row(row, allowed_entries, context)?;
    }
    Ok(())
}

/// The group counters and per-fixture rows are two published views of the same failures. Check
/// them against each other so a well-typed but internally inconsistent aggregate cannot pass. ~keep
fn validate_aggregate_group_row_reconciliation(
    aggregate: &NewConsolidatedResults,
    rows: &[&PerFixtureRow],
) -> Result<()> {
    for (key, group) in &aggregate.by_framework_mode {
        for (file_type, file_group) in &group.by_file_type {
            // Reconcile each populated OCR side against exactly the rows that fed it (no_ocr ← NotUsed,
            // with_ocr ← Used). Splitting by side lets a competitor legitimately straddle both without
            // double-counting, while xberg's single expected side reconciles as before.
            for (side_is_with_ocr, bucket) in [(false, &file_group.no_ocr), (true, &file_group.with_ocr)] {
                let Some(bucket) = bucket.as_ref() else { continue };
                let bucket_rows: Vec<&&PerFixtureRow> = rows
                    .iter()
                    .filter(|row| {
                        crate::aggregate::make_aggregate_key(&row.framework, row.output_format, &row.execution_mode)
                            == *key
                            && row.file_type == *file_type
                            && row.ocr == Some(side_is_with_ocr)
                    })
                    .collect();
                let count_kind = |kind: &str| {
                    bucket_rows
                        .iter()
                        .filter(|row| !row.success && row.error_kind.as_deref() == Some(kind))
                        .count()
                };
                require(
                    bucket.successful_sample_count == bucket_rows.iter().filter(|row| row.success).count()
                        && bucket.framework_errors == count_kind("FrameworkError")
                        && bucket.harness_errors == count_kind("HarnessError")
                        && bucket.config_setup_errors == count_kind("ConfigSetupError")
                        && bucket.timeouts == count_kind("Timeout")
                        && bucket.empty_content == count_kind("EmptyContent")
                        && bucket.zero_overlap == count_kind("ZeroOverlap")
                        && bucket.layout_failures == count_kind("LayoutFailed"),
                    format!("{key}: group failure counts do not match fixture rows for {file_type}"),
                )?;
            }
        }
    }
    Ok(())
}

pub(super) fn validate_aggregate(
    path: &Path,
    cohort: Cohort,
    contract: &CohortContract,
    iterations: usize,
    quality_expectations: &[FixtureQualityExpectation],
) -> Result<String> {
    let aggregate: NewConsolidatedResults = load_typed_json(path)?;
    require(
        aggregate.schema_version == SCHEMA_VERSION,
        format!("{}: unexpected schema", path.display()),
    )?;
    paired::validate_paired_sections_present(&load_json_value(path)?, path)?;
    paired::validate_layout_impact_coverage(&aggregate, path)?;
    paired::validate_layout_impact_classification(&aggregate, path)?;
    paired::validate_paired_delta_medians(&aggregate, path)?;
    paired::validate_head_to_head_lists(&aggregate, path)?;

    let (allowed_entries, present_entries, actual_keys) = validate_aggregate_key_coverage(&aggregate, contract)?;
    validate_format_support(&aggregate, &present_entries, contract, path)?;
    validate_aggregate_provenance(
        &aggregate,
        &present_entries,
        contract,
        iterations,
        quality_expectations,
        path,
    )?;

    let context = AggregateValidationContext {
        contract,
        quality_expectations,
        expects_ocr: cohort.expects_ocr(),
        path,
    };
    validate_aggregate_groups(&aggregate, &allowed_entries, &context)?;

    let rows: Vec<&PerFixtureRow> = aggregate.per_fixture_results.iter().collect();
    validate_aggregate_rows(&rows, &present_entries, &allowed_entries, &context)?;
    validate_aggregate_group_row_reconciliation(&aggregate, &rows)?;
    validate_derived_aggregate_fields(&aggregate, &rows, cohort, path)?;

    Ok(format!(
        "validated {} {} aggregate keys and {} fixture rows",
        actual_keys.len(),
        cohort.as_str(),
        rows.len()
    ))
}
