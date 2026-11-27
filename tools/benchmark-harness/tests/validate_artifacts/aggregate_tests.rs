use super::*;

#[test]
fn accepts_exact_native_aggregate_contract() {
    let contract = Cohort::Native.contract();
    let aggregate = build_aggregate(&contract, Cohort::Native);
    let (_root, path) = write_aggregate(&aggregate);
    let present = contract.matrix.len();
    let message = validate(&aggregate_args(Cohort::Native, path)).expect("native aggregate should validate");
    assert_eq!(
        message,
        format!(
            "validated {present} native aggregate keys and {} fixture rows",
            present * contract.fixtures.len()
        )
    );
}

#[test]
fn accepts_exact_ocr_aggregate_contract() {
    let contract = Cohort::Ocr.contract();
    let aggregate = build_aggregate(&contract, Cohort::Ocr);
    let (_root, path) = write_aggregate(&aggregate);
    let present = contract.matrix.len();
    let expected_rows: usize = contract
        .matrix
        .iter()
        .map(|entry| {
            contract
                .document_extensions
                .iter()
                .zip(contract.fixtures.iter())
                .filter(|(extension, fixture)| {
                    supports_extension(&entry.framework, extension)
                        && supports_fixture_language(&entry.framework, fixture)
                })
                .count()
        })
        .sum();
    let message = validate(&aggregate_args(Cohort::Ocr, path)).expect("ocr aggregate should validate");
    assert_eq!(
        message,
        format!("validated {present} ocr aggregate keys and {expected_rows} fixture rows")
    );
}

#[test]
fn rejects_warm_timing_regime_in_cold_release_aggregate() {
    let contract = Cohort::Native.contract();
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    aggregate.by_framework_mode.values_mut().next().unwrap().timing_regime =
        benchmark_harness::types::TimingRegime::WarmInProcess;
    let (_root, path) = write_aggregate(&aggregate);

    assert_err_contains(
        validate(&aggregate_args(Cohort::Native, path)),
        "timing regime mismatch",
    );
}

#[test]
fn rejects_missing_timing_regime_in_release_aggregate() {
    let contract = Cohort::Native.contract();
    let aggregate = build_aggregate(&contract, Cohort::Native);
    let mut value = serde_json::to_value(&aggregate).unwrap();
    let group = value["by_framework_mode"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap()
        .as_object_mut()
        .unwrap();
    group.remove("timing_regime");
    let (_root, path) = write_json_value(&value);

    assert_err_contains(
        validate(&aggregate_args(Cohort::Native, path)),
        "timing regime mismatch",
    );
}

#[test]
fn rejects_non_isolated_resource_scope_in_release_aggregate_provenance_and_rows() {
    for target_provenance in [true, false] {
        let contract = Cohort::Native.contract();
        let mut aggregate = build_aggregate(&contract, Cohort::Native);
        if target_provenance {
            aggregate.run_provenance[0].provenance.as_mut().unwrap().frameworks[0].resource_measurement_scope =
                benchmark_harness::types::ResourceMeasurementScope::HarnessProcessLatencyOnly;
        } else {
            aggregate.per_fixture_results[0]
                .framework_capabilities
                .resource_measurement_scope =
                benchmark_harness::types::ResourceMeasurementScope::HarnessProcessLatencyOnly;
        }
        let (_root, path) = write_aggregate(&aggregate);

        assert_err_contains(
            validate(&aggregate_args(Cohort::Native, path)),
            "resource measurement scope mismatch",
        );
    }
}

#[test]
fn rejects_missing_resource_scope_in_release_aggregate_provenance_and_rows() {
    for target_provenance in [true, false] {
        let contract = Cohort::Native.contract();
        let aggregate = build_aggregate(&contract, Cohort::Native);
        let mut value = serde_json::to_value(&aggregate).unwrap();
        let object = if target_provenance {
            value["run_provenance"][0]["provenance"]["frameworks"][0]
                .as_object_mut()
                .unwrap()
        } else {
            value["per_fixture_results"][0]["framework_capabilities"]
                .as_object_mut()
                .unwrap()
        };
        object.remove("resource_measurement_scope");
        let (_root, path) = write_json_value(&value);

        assert_err_contains(
            validate(&aggregate_args(Cohort::Native, path)),
            "resource measurement scope mismatch",
        );
    }
}

#[test]
fn rejects_aggregate_row_when_nested_quality_is_removed_for_a_ground_truth_fixture() {
    let contract = Cohort::Native.contract();
    let aggregate = build_aggregate(&contract, Cohort::Native);
    let mut value = serde_json::to_value(&aggregate).unwrap();
    let row = value["per_fixture_results"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["success"] == true && row["quality"].is_object())
        .expect("aggregate contains a successful quality row");
    let fixture_id = row["fixture_id"].as_str().unwrap().to_string();
    row["quality"] = serde_json::Value::Null;
    let (_root, path) = write_json_value(&value);

    assert_err_contains(
        validate(&aggregate_args(Cohort::Native, path)),
        &format!("fixture row {fixture_id} quality presence mismatch"),
    );
}

#[test]
fn rejects_aggregate_row_when_tf1_or_sf1_scalar_projection_is_removed() {
    for field in ["f1_text", "f1_layout"] {
        let contract = Cohort::Native.contract();
        let aggregate = build_aggregate(&contract, Cohort::Native);
        let mut value = serde_json::to_value(&aggregate).unwrap();
        let row = value["per_fixture_results"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["success"] == true && row[field].is_number())
            .unwrap_or_else(|| panic!("aggregate contains numeric {field}"));
        row[field] = serde_json::Value::Null;
        let (_root, path) = write_json_value(&value);

        assert_err_contains(
            validate(&aggregate_args(Cohort::Native, path)),
            "fixture-row quality projection mismatch",
        );
    }
}

#[test]
fn accepts_exact_aggregate_contract_for_every_format_cohort() {
    // ~keep This guards supported-subset cardinality and format-support validation across all
    // rendered-page and heterogeneous format families, not only the PDF release cohorts.
    for cohort in Cohort::ALL {
        let contract = cohort.contract();
        let aggregate = build_aggregate(&contract, cohort);
        let (_root, path) = write_aggregate(&aggregate);
        validate(&aggregate_args(cohort, path))
            .unwrap_or_else(|error| panic!("{} aggregate contract failed: {error}", cohort.as_str()));
    }
}

#[test]
fn rejects_fabricated_aggregate_comparison() {
    let contract = Cohort::Native.contract();
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    aggregate.comparison.throughput_ranking.push(RankedFramework {
        framework_mode: "fabricated:markdown:single".to_string(),
        rank: 1,
        value: 999.0,
        relative: 1.0,
        optional: false,
        output_format: OutputFormat::Markdown,
        mode: "single".to_string(),
    });
    let (_root, path) = write_aggregate(&aggregate);

    assert_err_contains(
        validate(&aggregate_args(Cohort::Native, path)),
        "comparison rankings mismatch",
    );
}

#[test]
fn rejects_missing_aggregate_provenance() {
    let contract = Cohort::Native.contract();
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    aggregate.run_provenance.clear();
    let (_root, path) = write_aggregate(&aggregate);

    assert_err_contains(
        validate(&aggregate_args(Cohort::Native, path)),
        "aggregate provenance count mismatch",
    );
}

#[test]
fn rejects_fabricated_aggregate_provenance_settings() {
    let contract = Cohort::Native.contract();
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    let batch = aggregate
        .run_provenance
        .iter_mut()
        .find(|record| {
            record
                .provenance
                .as_ref()
                .is_some_and(|provenance| provenance.fixed_batch_size.is_some())
        })
        .expect("batch provenance");
    batch.provenance.as_mut().unwrap().fixed_batch_size = None;
    let (_root, path) = write_aggregate(&aggregate);

    assert_err_contains(
        validate(&aggregate_args(Cohort::Native, path)),
        "aggregate provenance settings mismatch",
    );
}

#[test]
fn rejects_fabricated_aggregate_metadata_counts() {
    let contract = Cohort::Native.contract();
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    aggregate.metadata.total_results += 1;
    let (_root, path) = write_aggregate(&aggregate);

    assert_err_contains(
        validate(&aggregate_args(Cohort::Native, path)),
        "consolidation metadata mismatch",
    );
}

#[test]
fn rejects_aggregate_metrics_fabricated_independently_of_fixture_rows() {
    let contract = Cohort::Native.contract();
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    let group = aggregate
        .by_framework_mode
        .values_mut()
        .next()
        .expect("aggregate group");
    group.overall_performance.as_mut().unwrap().throughput.p50 = 999.0;
    let (_root, path) = write_aggregate(&aggregate);

    assert_err_contains(
        validate(&aggregate_args(Cohort::Native, path)),
        "aggregate metrics mismatch",
    );
}

#[test]
fn accepts_native_aggregate_when_optional_mineru_absent() {
    // validation must still pass on the required frameworks.
    let contract = Cohort::Native.contract();
    // ~keep Filter before aggregation so every production-derived view consistently excludes the
    // absent best-effort framework, including failure and format-support summaries.
    let aggregate = build_aggregate_with(&contract, Cohort::Native, |entry| !entry.optional, |_| {});

    let (_root, path) = write_aggregate(&aggregate);
    let required = required_count(&contract);
    let message =
        validate(&aggregate_args(Cohort::Native, path)).expect("native aggregate should validate without mineru");
    assert_eq!(
        message,
        format!(
            "validated {required} native aggregate keys and {} fixture rows",
            required * contract.fixtures.len()
        )
    );
}

#[test]
fn should_reject_invalid_optional_aggregate_group_when_present() {
    // ~keep Absence is the only relaxation for an optional aggregate group. A present group is
    // release data and must retain the same file-type and sample-count integrity checks.
    let contract = Cohort::Native.contract();
    let optional_key = contract
        .matrix
        .iter()
        .find(|entry| entry.optional)
        .expect("native cohort has an optional entry")
        .aggregate_key();
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    aggregate
        .by_framework_mode
        .get_mut(&optional_key)
        .expect("aggregate contains optional group")
        .by_file_type
        .clear();
    let (_root, path) = write_aggregate(&aggregate);
    assert_err_contains(
        validate(&aggregate_args(Cohort::Native, path)),
        "has no file-type metrics",
    );
}

#[test]
fn should_reject_failed_optional_aggregate_row_when_present() {
    // ~keep Present optional rows must be validated rather than filtered out; otherwise failed
    // executions can be published even when the optional group itself appears structurally valid.
    let contract = Cohort::Native.contract();
    let optional_framework = contract
        .matrix
        .iter()
        .find(|entry| entry.optional)
        .expect("native cohort has an optional entry")
        .framework
        .clone();
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    aggregate
        .per_fixture_results
        .iter_mut()
        .find(|row| row.framework == optional_framework)
        .expect("aggregate contains optional row")
        .success = false;
    let (_root, path) = write_aggregate(&aggregate);
    assert_err_contains(validate(&aggregate_args(Cohort::Native, path)), "failed fixture rows");
}

#[test]
fn should_accept_well_categorized_optional_aggregate_failure_when_present() {
    // ~keep A best-effort framework may publish partial failures when group accounting and row
    // diagnostics agree, preserving useful measurements without weakening structural validation.
    let contract = Cohort::Native.contract();
    let optional_entry = contract
        .matrix
        .iter()
        .find(|entry| entry.optional)
        .expect("native cohort has an optional entry");
    let optional_runtime_framework = aggregate_framework_name(optional_entry);
    // ~keep Inject the failure before production aggregation so bucket counters, rows, rankings,
    // and failure summaries are derived from one source of truth.
    let aggregate = build_aggregate_with(
        &contract,
        Cohort::Native,
        |_| true,
        |results| {
            let result = results
                .iter_mut()
                .find(|result| result.framework == optional_runtime_framework)
                .expect("aggregate input contains optional result");
            result.success = false;
            result.error_kind = ErrorKind::Timeout;
            result.error_message = Some("timed out after 900 seconds".to_string());
        },
    );
    let failed_row = aggregate
        .per_fixture_results
        .iter()
        .find(|row| row.framework == optional_entry.framework && !row.success)
        .expect("aggregate retains optional failure row");
    assert_eq!(failed_row.error_kind.as_deref(), Some("Timeout"));
    let timeout_count = aggregate.by_framework_mode[&optional_entry.aggregate_key()].by_file_type["pdf"]
        .no_ocr
        .as_ref()
        .expect("native group uses no_ocr")
        .timeouts;
    assert_eq!(timeout_count, 1);
    let (_root, path) = write_aggregate(&aggregate);
    validate(&aggregate_args(Cohort::Native, path)).expect("categorized optional aggregate failure should validate");
}

#[test]
fn should_reject_optional_aggregate_infrastructure_failure_when_present() {
    // ~keep Optional aggregate groups cannot convert runner infrastructure faults into
    // best-effort framework data, even when their total cardinality remains internally consistent.
    let contract = Cohort::Native.contract();
    let optional_entry = contract
        .matrix
        .iter()
        .find(|entry| entry.optional)
        .expect("native cohort has an optional entry");
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    let bucket = aggregate
        .by_framework_mode
        .get_mut(&optional_entry.aggregate_key())
        .expect("aggregate contains optional group")
        .by_file_type
        .get_mut("pdf")
        .expect("native group contains pdf")
        .no_ocr
        .as_mut()
        .expect("native group uses no_ocr");
    bucket.successful_sample_count -= 1;
    bucket.harness_errors = 1;
    let (_root, path) = write_aggregate(&aggregate);
    assert_err_contains(
        validate(&aggregate_args(Cohort::Native, path)),
        "infrastructure failures",
    );
}

#[test]
fn should_reject_optional_aggregate_row_with_infrastructure_error_kind() {
    // ~keep Row diagnostics must agree with the accountable bucket categories; a valid total alone
    // cannot disguise a harness fault as a framework-level best-effort failure.
    let contract = Cohort::Native.contract();
    let optional_entry = contract
        .matrix
        .iter()
        .find(|entry| entry.optional)
        .expect("native cohort has an optional entry");
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    let bucket = aggregate
        .by_framework_mode
        .get_mut(&optional_entry.aggregate_key())
        .expect("aggregate contains optional group")
        .by_file_type
        .get_mut("pdf")
        .expect("native group contains pdf")
        .no_ocr
        .as_mut()
        .expect("native group uses no_ocr");
    bucket.successful_sample_count -= 1;
    bucket.timeouts = 1;
    let row = aggregate
        .per_fixture_results
        .iter_mut()
        .find(|row| row.framework == optional_entry.framework)
        .expect("aggregate contains optional row");
    row.success = false;
    row.error_kind = Some("HarnessError".to_string());
    row.error_message = Some("subprocess protocol failed".to_string());
    let (_root, path) = write_aggregate(&aggregate);
    assert_err_contains(
        validate(&aggregate_args(Cohort::Native, path)),
        "accountable error kind",
    );
}

#[test]
fn should_reject_invalid_optional_framework_format_support_when_present() {
    // ~keep The format-support matrix is a published explanation for absent framework/format
    // pairs. Every present entry, including an optional framework, must reference a cohort format.
    let contract = Cohort::Native.contract();
    let optional_framework = contract
        .matrix
        .iter()
        .find(|entry| entry.optional)
        .expect("native cohort has an optional entry")
        .framework
        .clone();
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    aggregate.format_support.file_types = vec!["pdf".to_string()];
    aggregate
        .format_support
        .unsupported
        .insert(optional_framework, vec!["exe".to_string()]);
    let (_root, path) = write_aggregate(&aggregate);
    assert_err_contains(validate(&aggregate_args(Cohort::Native, path)), "format support");
}

#[test]
fn rejects_unexpected_aggregate_key() {
    let contract = Cohort::Native.contract();
    let aggregate = build_aggregate(&contract, Cohort::Native);
    let mut value = serde_json::to_value(&aggregate).unwrap();
    let map = value["by_framework_mode"].as_object_mut().unwrap();
    let (first_key, first_value) = map
        .iter()
        .next()
        .map(|(key, value)| (key.clone(), value.clone()))
        .unwrap();
    map.remove(&first_key);
    map.insert("surprise:markdown:single".to_string(), first_value);
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("aggregated.json");
    std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    assert_err_contains(validate(&aggregate_args(Cohort::Native, path)), "unexpected");
}

#[test]
fn rejects_native_aggregate_with_only_with_ocr_bucket() {
    let contract = Cohort::Native.contract();
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    for group in aggregate.by_framework_mode.values_mut() {
        let file_group = group.by_file_type.get_mut("pdf").unwrap();
        file_group.with_ocr = file_group.no_ocr.take();
    }
    let (_root, path) = write_aggregate(&aggregate);
    assert!(validate(&aggregate_args(Cohort::Native, path)).is_err());
}

#[test]
fn rejects_ocr_aggregate_with_only_no_ocr_bucket() {
    let contract = Cohort::Ocr.contract();
    let mut aggregate = build_aggregate(&contract, Cohort::Ocr);
    for group in aggregate.by_framework_mode.values_mut() {
        let file_group = group.by_file_type.get_mut("pdf").unwrap();
        file_group.no_ocr = file_group.with_ocr.take();
    }
    let (_root, path) = write_aggregate(&aggregate);
    assert!(validate(&aggregate_args(Cohort::Ocr, path)).is_err());
}

#[test]
fn rejects_aggregate_group_without_file_type_metrics() {
    // A group with no file-type buckets can never account for the cohort's fixtures.
    let contract = Cohort::Native.contract();
    let mut aggregate = build_aggregate(&contract, Cohort::Native);
    let key = required_group_key(&contract);
    let first_group = aggregate.by_framework_mode.get_mut(&key).unwrap();
    first_group.by_file_type.clear();
    let (_root, path) = write_aggregate(&aggregate);
    assert_err_contains(
        validate(&aggregate_args(Cohort::Native, path)),
        "has no file-type metrics",
    );
}

#[test]
fn accepts_aggregate_with_multiple_file_type_buckets() {
    // The office cohort spans 7 extensions (docx×2, doc, pptx, ppt, xlsx, odt, rtf); build_aggregate
    // now emits that real per-extension bucket shape, which must validate.
    let contract = Cohort::Office.contract();
    let aggregate = build_aggregate(&contract, Cohort::Office);
    let (_root, path) = write_aggregate(&aggregate);
    validate(&aggregate_args(Cohort::Office, path)).expect("multi-file-type office aggregate should validate");
}

#[test]
fn rejects_aggregate_with_unexpected_file_type_bucket() {
    // A file-type bucket the cohort's fixtures don't contain must be rejected, even if counts sum.
    let contract = Cohort::Office.contract();
    let mut aggregate = build_aggregate(&contract, Cohort::Office);
    let key = required_group_key(&contract);
    let group = aggregate.by_framework_mode.get_mut(&key).unwrap();
    // Move all `docx` samples (2) into a bogus `pdf` bucket: totals still sum to 8, but pdf is not
    // an office extension and docx is now missing.
    let docx = group.by_file_type.remove("docx").unwrap();
    group.by_file_type.insert(
        "pdf".to_string(),
        FileTypeAggregation {
            file_type: "pdf".to_string(),
            ..docx
        },
    );
    let (_root, path) = write_aggregate(&aggregate);
    assert_err_contains(validate(&aggregate_args(Cohort::Office, path)), "file-type buckets");
}

#[test]
fn rejects_aggregate_with_wrong_file_type_sample_count() {
    // The right extensions but a mis-bucketed count (docx should be 2, not 1) must be rejected —
    // the per-extension check the sum-only check used to miss.
    let contract = Cohort::Office.contract();
    let mut aggregate = build_aggregate(&contract, Cohort::Office);
    let key = required_group_key(&contract);
    let group = aggregate.by_framework_mode.get_mut(&key).unwrap();
    group
        .by_file_type
        .get_mut("docx")
        .unwrap()
        .no_ocr
        .as_mut()
        .unwrap()
        .total_sample_count = 1;
    let (_root, path) = write_aggregate(&aggregate);
    assert_err_contains(
        validate(&aggregate_args(Cohort::Office, path)),
        "success/error counts do not match total_sample_count",
    );
}

#[test]
fn accepts_aggregate_with_a_zero_overlap_sample() {
    // ~keep A zero-overlap result is `success == false` with `ErrorKind::ZeroOverlap`, so it is
    // absent from `successful_sample_count` while still counted in `total_sample_count`
    // (`total_sample_count: results.len()`). The rest of the harness already treats it as a
    // framework fault -- `CountsBuilder::record` folds it into `framework_fault_total` and
    // `accountable_sample_count` includes it -- but `validate_bucket` omitted it from its sum, so
    // any bucket holding one failed the count invariant. That is what rejected the otherwise
    // complete benchmark run 34391134650 on `mineru:markdown:single`. Built through the real
    // aggregation path rather than by editing counts, so the arithmetic under test is the
    // arithmetic production computes.
    let contract = Cohort::Native.contract();
    let optional_framework = contract
        .matrix
        .iter()
        .find(|entry| entry.optional)
        .map(aggregate_framework_name)
        .expect("native cohort has an optional entry");
    let aggregate = build_aggregate_with(
        &contract,
        Cohort::Native,
        |_| true,
        |results| {
            let target = results
                .iter_mut()
                .find(|result| result.framework == optional_framework && result.success)
                .expect("optional framework has a successful result to reclassify");
            target.success = false;
            target.error_kind = ErrorKind::ZeroOverlap;
            target.error_message = Some("extraction produced no overlap with the reference".to_string());
        },
    );
    let (_root, path) = write_aggregate(&aggregate);
    validate(&aggregate_args(Cohort::Native, path))
        .expect("a zero-overlap sample is an accountable framework fault, not an unaccounted sample");
}

#[test]
fn rejects_aggregate_row_when_not_an_object() {
    let contract = Cohort::Native.contract();
    let aggregate = build_aggregate(&contract, Cohort::Native);
    let mut value = serde_json::to_value(&aggregate).unwrap();
    value["per_fixture_results"][0] = serde_json::Value::Null;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("aggregated.json");
    std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    assert!(validate(&aggregate_args(Cohort::Native, path)).is_err());
}

#[test]
fn rejects_aggregate_row_whose_fixture_family_differs_from_the_fixture() {
    let contract = Cohort::Ocr.contract();
    let mut aggregate = build_aggregate(&contract, Cohort::Ocr);
    let row = aggregate
        .per_fixture_results
        .iter_mut()
        .find(|row| row.fixture_id == "ocr_test")
        .expect("the OCR cohort runs ocr_test.pdf");
    assert_eq!(row.fixture_family.as_deref(), Some("ocr_test"));
    row.fixture_family = None;
    let (_root, path) = write_aggregate(&aggregate);

    assert_err_contains(validate(&aggregate_args(Cohort::Ocr, path)), "declared family");
}
