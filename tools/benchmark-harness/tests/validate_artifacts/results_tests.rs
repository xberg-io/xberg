use super::*;

#[test]
fn accepts_exact_native_contract() {
    let scenario = artifact_scenario(Cohort::Native);
    let present = scenario.contract.matrix.len();
    let message = validate(&scenario.args).expect("native contract should validate");
    assert_eq!(message, format!("validated {present} native benchmark artifacts"));
}

#[test]
fn accepts_exact_ocr_contract() {
    let scenario = artifact_scenario(Cohort::Ocr);
    let present = scenario.contract.matrix.len();
    let message = validate(&scenario.args).expect("ocr contract should validate");
    assert_eq!(message, format!("validated {present} ocr benchmark artifacts"));
}

#[test]
fn accepts_exact_raw_contract_for_every_format_cohort() {
    // ~keep Every cohort exercises the same optional-present validation path, including family
    // cohorts where competitors intentionally cover only a supported subset of extensions.
    for cohort in Cohort::ALL {
        let scenario = artifact_scenario(cohort);
        validate(&scenario.args).unwrap_or_else(|error| panic!("{} raw contract failed: {error}", cohort.as_str()));
    }
}

#[test]
fn rejects_provenance_schema_2_without_required_measurement_contract() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&provenance_path(&scenario, 0), |value| {
        value["schema_version"] = serde_json::json!(2);
    });

    assert_err_contains(validate(&scenario.args), "unexpected provenance schema");
}

#[test]
fn rejects_every_invalid_raw_quality_metric_with_path_and_index_context() {
    for (field, value) in [
        ("f1_score_text", serde_json::json!(-0.01)),
        ("f1_score_numeric", serde_json::json!(1.01)),
        ("f1_score_layout", serde_json::json!(1.01)),
        ("quality_score", serde_json::json!(2.0)),
    ] {
        let scenario = artifact_scenario(Cohort::Native);
        let path = results_path(&scenario, 0);
        tamper_json(&path, |results| {
            results[0]["quality"][field] = value;
        });

        let error = validate(&scenario.args).expect_err("invalid raw quality must fail");
        let message = error.to_string();
        let context = format!("{}: result 0: Benchmark error: Invalid result state", path.display());
        assert!(
            message.contains(&context) && message.contains(field),
            "expected path/index context and field {field:?}, got: {message}"
        );
    }
}

#[test]
fn rejects_missing_quality_only_when_the_fixture_has_quality_ground_truth() {
    let scenario = artifact_scenario(Cohort::Native);
    let path = results_path(&scenario, 0);
    tamper_json(&path, |results| {
        results[0]["quality"] = serde_json::Value::Null;
    });

    assert_err_contains(
        validate(&scenario.args),
        &format!("{}: result 0 quality presence mismatch", path.display()),
    );
}

#[test]
fn rejects_missing_markdown_sf1_only_when_the_fixture_has_structural_ground_truth() {
    let scenario = artifact_scenario(Cohort::Native);
    let path = results_path(&scenario, 0);
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let result_index = value
        .as_array()
        .unwrap()
        .iter()
        .position(|result| result["quality"]["f1_score_layout"].is_number())
        .expect("native markdown artifact has structural ground truth");
    tamper_json(&path, |results| {
        results[result_index]["quality"]["f1_score_layout"] = serde_json::Value::Null;
    });

    assert_err_contains(
        validate(&scenario.args),
        &format!("{}: result {result_index} SF1 presence mismatch", path.display()),
    );
}

#[test]
fn release_validation_rejects_plaintext_sf1_despite_generic_writer_compatibility() {
    let scenario = artifact_scenario(Cohort::Native);
    let matrix_index = scenario
        .contract
        .matrix
        .iter()
        .position(|entry| !entry.optional && entry.output_format == OutputFormat::Plaintext)
        .expect("native contract has required plaintext entry");
    let path = results_path(&scenario, matrix_index);
    tamper_json(&path, |results| {
        results[0]["quality"]["f1_score_layout"] = serde_json::json!(0.7);
    });

    assert_err_contains(
        validate(&scenario.args),
        &format!("{}: result 0 SF1 presence mismatch", path.display()),
    );
}

#[test]
fn accepts_batch_xberg_framework_with_mode_suffix() {
    // Batch xberg cells write `xberg-<fmt>-<pipeline>-batch`; the validator must accept that
    // suffixed name against the mode-independent matrix entry (mode is checked via timing.mode).
    let scenario = artifact_scenario(Cohort::Native);
    let index = batch_xberg_index(&scenario.contract);
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(provenance_path(&scenario, index)).unwrap()).unwrap();
    assert!(
        value["frameworks"][0]["name"].as_str().unwrap().ends_with("-batch"),
        "fixture must carry the runner's -batch suffix"
    );
    validate(&scenario.args).expect("suffixed batch xberg name must validate");
}

#[test]
fn rejects_batch_xberg_framework_with_wrong_base() {
    // Stripping the mode suffix must not mask a genuinely wrong base framework name.
    let scenario = artifact_scenario(Cohort::Native);
    let index = batch_xberg_index(&scenario.contract);
    tamper_json(&provenance_path(&scenario, index), |value| {
        value["frameworks"][0]["name"] = serde_json::json!("xberg-markdown-bogus-batch");
    });
    assert_err_contains(validate(&scenario.args), "framework mismatch");
}

#[test]
fn rejects_warm_timing_regime_in_cold_release_provenance() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&provenance_path(&scenario, 0), |value| {
        value["frameworks"][0]["timing_regime"] = serde_json::json!("warm_in_process");
    });
    assert_err_contains(validate(&scenario.args), "timing regime mismatch");
}

#[test]
fn rejects_result_timing_regime_that_disagrees_with_cold_release_contract() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&results_path(&scenario, 0), |value| {
        value[0]["framework_capabilities"]["timing_regime"] = serde_json::json!("warm_in_process");
    });
    assert_err_contains(validate(&scenario.args), "result 0 timing regime mismatch");
}

#[test]
fn rejects_missing_timing_regime_in_release_provenance_and_results() {
    for target_provenance in [true, false] {
        let scenario = artifact_scenario(Cohort::Native);
        let path = if target_provenance {
            provenance_path(&scenario, 0)
        } else {
            results_path(&scenario, 0)
        };
        tamper_json(&path, |value| {
            let object = if target_provenance {
                value["frameworks"][0].as_object_mut().unwrap()
            } else {
                value[0]["framework_capabilities"].as_object_mut().unwrap()
            };
            object.remove("timing_regime");
        });

        assert_err_contains(validate(&scenario.args), "timing regime mismatch");
    }
}

#[test]
fn rejects_non_isolated_resource_scope_in_release_provenance_and_results() {
    for target_provenance in [true, false] {
        let scenario = artifact_scenario(Cohort::Native);
        let path = if target_provenance {
            provenance_path(&scenario, 0)
        } else {
            results_path(&scenario, 0)
        };
        tamper_json(&path, |value| {
            let object = if target_provenance {
                &mut value["frameworks"][0]
            } else {
                &mut value[0]["framework_capabilities"]
            };
            object["resource_measurement_scope"] = serde_json::json!("harness_process_latency_only");
        });

        assert_err_contains(validate(&scenario.args), "resource measurement scope mismatch");
    }
}

#[test]
fn rejects_missing_resource_scope_in_release_provenance_and_results() {
    for target_provenance in [true, false] {
        let scenario = artifact_scenario(Cohort::Native);
        let path = if target_provenance {
            provenance_path(&scenario, 0)
        } else {
            results_path(&scenario, 0)
        };
        tamper_json(&path, |value| {
            let object = if target_provenance {
                value["frameworks"][0].as_object_mut().unwrap()
            } else {
                value[0]["framework_capabilities"].as_object_mut().unwrap()
            };
            object.remove("resource_measurement_scope");
        });

        assert_err_contains(validate(&scenario.args), "resource measurement scope mismatch");
    }
}

#[test]
fn accepts_native_contract_when_optional_mineru_absent() {
    // A best-effort framework (MinerU) that never produced an artifact must not fail
    // validation: the baseline still publishes with the required frameworks.
    let scenario = artifact_scenario(Cohort::Native);
    std::fs::remove_dir_all(optional_artifact_dir(&scenario)).expect("remove optional artifact dir");
    let required = required_count(&scenario.contract);
    let message = validate(&scenario.args).expect("native contract should validate without mineru");
    assert_eq!(message, format!("validated {required} native benchmark artifacts"));
}

#[test]
fn should_reject_optional_raw_failure_without_diagnostic_when_present() {
    // ~keep Best-effort failures remain publishable only when their failure category and diagnostic
    // are complete; otherwise consumers cannot distinguish framework behavior from corrupt data.
    let scenario = artifact_scenario(Cohort::Native);
    let results = optional_artifact_dir(&scenario).join("run/results.json");
    tamper_json(&results, |value| {
        value[0]["success"] = serde_json::Value::Bool(false);
        value[0]["error_kind"] = serde_json::Value::String("timeout".to_string());
    });
    assert_err_contains(validate(&scenario.args), "success=false but error_message is None");
}

#[test]
fn should_accept_well_categorized_optional_raw_failure_when_present() {
    // ~keep Optional producer failures are useful comparison data when the artifact remains
    // structurally complete and carries an explicit category and diagnostic.
    let scenario = artifact_scenario(Cohort::Native);
    let results = optional_artifact_dir(&scenario).join("run/results.json");
    tamper_json(&results, |value| {
        value[0]["success"] = serde_json::Value::Bool(false);
        value[0]["error_kind"] = serde_json::Value::String("timeout".to_string());
        value[0]["error_message"] = serde_json::Value::String("timed out after 900 seconds".to_string());
    });
    validate(&scenario.args).expect("well-categorized optional failure should remain publishable");
}

#[test]
fn should_reject_optional_raw_infrastructure_failure_when_present() {
    // ~keep Best-effort applies only to framework-accountable failures; harness/config failures
    // invalidate the benchmark environment and must never flow into release data.
    let scenario = artifact_scenario(Cohort::Native);
    let results = optional_artifact_dir(&scenario).join("run/results.json");
    tamper_json(&results, |value| {
        value[0]["success"] = serde_json::Value::Bool(false);
        value[0]["error_kind"] = serde_json::Value::String("harness_error".to_string());
        value[0]["error_message"] = serde_json::Value::String("subprocess protocol failed".to_string());
    });
    assert_err_contains(validate(&scenario.args), "infrastructure error");
}

#[test]
fn should_reject_optional_raw_artifact_with_wrong_provenance_when_present() {
    // ~keep A present best-effort artifact has the same provenance trust boundary as every
    // required artifact; optionality must never bypass source-commit verification.
    let scenario = artifact_scenario(Cohort::Native);
    let index = optional_matrix_index(&scenario.contract);
    tamper_json(&provenance_path(&scenario, index), |value| {
        value["repository"]["commit"] = serde_json::Value::String("d".repeat(40));
    });
    assert_err_contains(validate(&scenario.args), "source SHA mismatch");
}

#[test]
fn should_reject_optional_raw_artifact_with_missing_fixture_when_present() {
    // ~keep Optional artifacts may be wholly absent, but a present artifact must cover the full
    // cohort; accepting a partial result would silently bias the published comparison.
    let scenario = artifact_scenario(Cohort::Native);
    let index = optional_matrix_index(&scenario.contract);
    tamper_json(&results_path(&scenario, index), |value| {
        value.as_array_mut().unwrap().pop();
    });
    assert_err_contains(validate(&scenario.args), "result fixture count mismatch");
}

#[test]
fn rejects_tampered_manifest_bytes() {
    let scenario = artifact_scenario(Cohort::Native);
    let mut manifest = std::fs::read(scenario.args.cohort_manifest.clone().unwrap()).unwrap();
    manifest.push(b' ');
    std::fs::write(scenario.args.cohort_manifest.as_ref().unwrap(), manifest).unwrap();
    assert_err_contains(validate(&scenario.args), "manifest BLAKE3 mismatch");
}

#[test]
fn rejects_wrong_fixture_digest_for_real_descriptor() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&provenance_path(&scenario, 0), |value| {
        value["corpus"]["ordered_fixtures"][0]["fixture_blake3"] = serde_json::Value::String("0".repeat(64));
    });
    assert_err_contains(validate(&scenario.args), "descriptor BLAKE3 mismatch");
}

#[test]
fn rejects_wrong_document_digest_for_real_document() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&provenance_path(&scenario, 0), |value| {
        value["corpus"]["ordered_fixtures"][0]["document_blake3"] = serde_json::Value::String("0".repeat(64));
    });
    assert_err_contains(validate(&scenario.args), "document BLAKE3 mismatch");
}

#[test]
fn rejects_wrong_document_bytes_for_real_document() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&provenance_path(&scenario, 0), |value| {
        let bytes = value["corpus"]["ordered_fixtures"][0]["document_bytes"]
            .as_u64()
            .unwrap();
        value["corpus"]["ordered_fixtures"][0]["document_bytes"] = serde_json::Value::from(bytes + 1);
    });
    assert_err_contains(validate(&scenario.args), "document size mismatch");
}

#[test]
fn rejects_unexpected_artifact() {
    let scenario = artifact_scenario(Cohort::Native);
    std::fs::create_dir(
        scenario
            .args
            .artifacts_dir
            .as_ref()
            .unwrap()
            .join("benchmarks-surprise-42"),
    )
    .unwrap();
    assert_err_contains(validate(&scenario.args), "unexpected");
}

#[test]
fn rejects_source_sha_mismatch() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&provenance_path(&scenario, 0), |value| {
        value["repository"]["commit"] = serde_json::Value::String("d".repeat(40));
    });
    assert_err_contains(validate(&scenario.args), "source SHA mismatch");
}

fn ocr_scenario_index(scenario: &ArtifactScenario, is_auto_ocr: bool) -> usize {
    scenario
        .contract
        .matrix
        .iter()
        .position(|entry| {
            entry.framework.starts_with("xberg-")
                && entry.framework.contains("-auto-ocr") == is_auto_ocr
                && !entry.framework.contains("paddle")
        })
        .expect("matrix entry")
}

fn set_every_ocr_status(scenario: &ArtifactScenario, index: usize, status: &str) {
    tamper_json(&results_path(scenario, index), |value| {
        for result in value.as_array_mut().unwrap() {
            result["ocr_status"] = serde_json::Value::String(status.to_string());
        }
    });
}

#[test]
fn accepts_auto_ocr_rows_that_declined_to_ocr() {
    let scenario = artifact_scenario(Cohort::Ocr);
    let index = ocr_scenario_index(&scenario, true);
    set_every_ocr_status(&scenario, index, "not_used");
    validate(&scenario.args).expect("auto-ocr rows may report OCR as not used");
}

#[test]
fn rejects_forced_ocr_rows_that_did_not_use_ocr() {
    let scenario = artifact_scenario(Cohort::Ocr);
    let index = ocr_scenario_index(&scenario, false);
    set_every_ocr_status(&scenario, index, "not_used");
    assert_err_contains(validate(&scenario.args), "OCR status mismatch");
}

#[test]
fn rejects_timeout_result() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&results_path(&scenario, 0), |value| {
        value[0]["success"] = serde_json::Value::Bool(false);
        value[0]["error_kind"] = serde_json::Value::String("timeout".to_string());
        value[0]["error_message"] = serde_json::Value::String("timed out".to_string());
    });
    assert_err_contains(validate(&scenario.args), "failed");
}

#[test]
fn rejects_duplicate_fixture_result() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&results_path(&scenario, 0), |value| {
        let first_path = value[0]["file_path"].clone();
        value[1]["file_path"] = first_path;
    });
    assert_err_contains(validate(&scenario.args), "order/content mismatch");
}

#[test]
fn rejects_malformed_provenance() {
    let scenario = artifact_scenario(Cohort::Native);
    std::fs::write(provenance_path(&scenario, 0), "{").unwrap();
    assert_err_contains(validate(&scenario.args), "malformed");
}

#[test]
fn rejects_manifest_fixtures_when_not_an_array() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(scenario.args.cohort_manifest.as_ref().unwrap(), |value| {
        value["fixtures"] = serde_json::json!({});
    });
    assert!(validate(&scenario.args).is_err());
}

#[test]
fn rejects_framework_when_not_an_object() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&provenance_path(&scenario, 0), |value| {
        value["frameworks"] = serde_json::json!([null]);
    });
    assert!(validate(&scenario.args).is_err());
}

#[test]
fn rejects_result_row_when_not_an_object() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&results_path(&scenario, 0), |value| {
        value[0] = serde_json::Value::Null;
    });
    assert!(validate(&scenario.args).is_err());
}

#[test]
fn accepts_sequential_one_based_iterations() {
    // The runner numbers iterations 1-based ([1, 2, 3] for ITERATIONS = 3); the untampered
    // scenario fixture already reflects that, so this simply pins the happy path against
    // regressing back to a 0-based expectation.
    let scenario = artifact_scenario(Cohort::Native);
    validate(&scenario.args).expect("sequential 1-based iterations should validate");
}

#[test]
fn rejects_misordered_iterations() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&results_path(&scenario, 0), |value| {
        let second = value[0]["iterations"][1]["iteration"].clone();
        let third = value[0]["iterations"][2]["iteration"].clone();
        value[0]["iterations"][1]["iteration"] = third;
        value[0]["iterations"][2]["iteration"] = second;
    });
    assert_err_contains(validate(&scenario.args), "iteration order/duplicates mismatch");
}

#[test]
fn rejects_duplicate_iterations() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&results_path(&scenario, 0), |value| {
        let first = value[0]["iterations"][0]["iteration"].clone();
        value[0]["iterations"][1]["iteration"] = first;
    });
    assert_err_contains(validate(&scenario.args), "iteration order/duplicates mismatch");
}

#[test]
fn rejects_iteration_count_mismatch() {
    let scenario = artifact_scenario(Cohort::Native);
    tamper_json(&results_path(&scenario, 0), |value| {
        // the configured ITERATIONS.
        value[0]["iterations"].as_array_mut().unwrap().pop();
    });
    assert_err_contains(validate(&scenario.args), "iteration count mismatch");
}
