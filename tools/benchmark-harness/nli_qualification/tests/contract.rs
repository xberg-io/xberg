mod common;
use common::*;
use nli_qualification::FrozenPlan;
use serde_json::json;

#[test]
fn accepts_exact_cartesian_accounting_including_real_abstentions() {
    let plan = plan();
    plan.validate().unwrap();
    report(&plan).validate(&plan).unwrap();
}

#[test]
fn rejects_empty_work_and_wrong_declared_job_count() {
    for (key, value, expected) in [
        ("documents", json!([]), "documents"),
        ("candidates", json!([]), "candidates"),
        ("arms", json!([]), "arms"),
        ("repeats", json!(0), "repeats"),
        ("expected_inferences", json!(3), "expected_inferences"),
    ] {
        let mut input = plan_value();
        input[key] = value;
        let plan: FrozenPlan = serde_json::from_value(input).unwrap();
        assert!(
            plan.validate().unwrap_err().to_string().contains(expected),
            "{expected}"
        );
    }
}

#[test]
fn rejects_family_source_and_text_leakage_between_splits() {
    for key in ["family", "source_sha256", "text"] {
        let mut input = plan_value();
        input["documents"][1][key] = input["documents"][0][key].clone();
        if key == "text" {
            input["documents"][1]["text_sha256"] = input["documents"][0]["text_sha256"].clone();
        }
        let plan: FrozenPlan = serde_json::from_value(input).unwrap();
        assert!(plan.validate().unwrap_err().to_string().contains("leakage"), "{key}");
    }
}

#[test]
fn rejects_missing_duplicate_unknown_and_skipped_inference_for_the_intended_reason() {
    let plan = plan();
    let mut missing = report(&plan);
    missing.rows.pop();
    assert!(missing.validate(&plan).unwrap_err().to_string().contains("row count"));
    let mut duplicate = report(&plan);
    duplicate.rows[1] = duplicate.rows[0].clone();
    assert!(duplicate.validate(&plan).unwrap_err().to_string().contains("duplicate"));
    let mut unknown = report(&plan);
    unknown.rows[0].document = "unknown".into();
    assert!(unknown.validate(&plan).unwrap_err().to_string().contains("unexpected"));
    let mut skipped = report(&plan);
    skipped.rows[0].inference = "skipped".into();
    assert!(skipped.validate(&plan).unwrap_err().to_string().contains("inference"));
}

#[test]
fn binds_frozen_plan_runtime_and_each_artifact_identity() {
    let plan = plan();
    let mut wrong = report(&plan);
    wrong.plan_sha256 = "0".repeat(64);
    assert!(wrong.validate(&plan).unwrap_err().to_string().contains("plan digest"));
    let mut wrong = report(&plan);
    wrong.runtime.threads = 2;
    assert!(wrong.validate(&plan).unwrap_err().to_string().contains("runtime"));
    let mut wrong = report(&plan);
    wrong.rows[0].model.model_sha256 = "0".repeat(64);
    assert!(
        wrong
            .validate(&plan)
            .unwrap_err()
            .to_string()
            .contains("model identity")
    );
}

#[test]
fn never_accepts_a_qualified_claim_or_invalid_prediction() {
    let plan = plan();
    let mut report = report(&plan);
    report.qualified = true;
    assert!(report.validate(&plan).unwrap_err().to_string().contains("unqualified"));
    report.qualified = false;
    report.rows[0].labels = serde_json::from_value(json!([{"label": "secret", "confidence": 0.9}])).unwrap();
    assert!(report.validate(&plan).unwrap_err().to_string().contains("prediction"));
}

#[test]
fn rejects_corrupt_inputs_missing_companions_and_unknown_fields() {
    let mut input = plan_value();
    input["documents"][0]["text"] = json!("Changed text.");
    let plan: FrozenPlan = serde_json::from_value(input).unwrap();
    assert!(plan.validate().unwrap_err().to_string().contains("text digest"));
    let mut input = plan_value();
    input["candidates"][0]["variants"]["fp32"]["config"]["model"]["sha256_manifest"] =
        json!(format!("{}  model.onnx\n", "a".repeat(64)));
    let plan: FrozenPlan = serde_json::from_value(input).unwrap();
    assert!(plan.validate().unwrap_err().to_string().contains("companion"));
    let mut input = plan_value();
    input["backend"] = json!("pretend");
    assert!(
        serde_json::from_value::<FrozenPlan>(input)
            .unwrap_err()
            .to_string()
            .contains("unknown field")
    );
}

#[test]
fn rejects_nested_duplicate_json_keys_instead_of_accepting_last_value() {
    let input = serde_json::to_string(&plan_value()).unwrap();
    let input = input.replace("\"variants\":{", "\"variants\":{},\"variants\":{");
    assert!(
        nli_qualification::files::parse_json::<FrozenPlan>(input.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("duplicate JSON key: variants")
    );
}

#[test]
fn binds_gold_labels_and_preprocessing_to_the_frozen_digest() {
    let plan = plan();
    let original = report(&plan);
    let mut changed = plan.clone();
    changed.documents[0].gold.clear();
    assert!(
        original
            .validate(&changed)
            .unwrap_err()
            .to_string()
            .contains("plan digest")
    );
    let mut changed = plan.clone();
    changed.candidates[0]
        .variants
        .get_mut("fp32")
        .unwrap()
        .config
        .hypothesis_template = "Category: {label}".into();
    assert!(
        original
            .validate(&changed)
            .unwrap_err()
            .to_string()
            .contains("plan digest")
    );
}

#[test]
fn refuses_sdk_rejected_configuration_and_requests_before_freezing() {
    let cases = [
        ("/limits/max_tokens", json!(0), "limits"),
        ("/limits/max_labels", json!(1), "request"),
        ("/limits/max_input_bytes", json!(1), "request"),
        ("/limits/max_label_bytes", json!(1), "request"),
        ("/model/output_name", json!(""), "descriptor"),
        ("/hypothesis_template", json!(""), "template"),
    ];
    for (path, value, expected) in cases {
        let mut input = plan_value();
        let config = &mut input["candidates"][0]["variants"]["fp32"]["config"];
        if !config.as_object().unwrap().contains_key("limits") {
            config["limits"] =
                serde_json::to_value(xberg::local_classification::LocalClassificationLimits::default()).unwrap();
        }
        *config.pointer_mut(path).unwrap() = value;
        let plan: FrozenPlan = serde_json::from_value(input).unwrap();
        assert!(
            plan.validate().unwrap_err().to_string().contains(expected),
            "SDK must reject {path} for {expected}"
        );
    }
}

#[test]
fn rejects_abstentions_impossible_under_the_frozen_scoring_mode() {
    for (multi_label, threshold) in [(false, 0.0), (false, 0.5), (true, 0.0)] {
        let mut plan = plan();
        let config = &mut plan.candidates[0].variants.get_mut("fp32").unwrap().config;
        config.multi_label = multi_label;
        config.confidence_threshold = threshold;
        assert!(
            report(&plan)
                .validate(&plan)
                .unwrap_err()
                .to_string()
                .contains("cardinality")
        );
    }
}

#[test]
fn accounts_for_two_documents_two_candidates_two_arms_and_two_repeats() {
    let mut plan = plan();
    plan.arms.push("int8".into());
    let variant = plan.candidates[0].variants["fp32"].clone();
    plan.candidates[0].variants.insert("int8".into(), variant);
    let mut candidate = plan.candidates[0].clone();
    candidate.id = "second-nli".into();
    plan.candidates.push(candidate);
    plan.expected_inferences = 16;
    let mut report = report(&plan);
    let prototype = report.rows[0].clone();
    report.rows.clear();
    for document in &plan.documents {
        for candidate in &plan.candidates {
            for arm in &plan.arms {
                for repeat in 1..=plan.repeats {
                    let mut row = prototype.clone();
                    row.document = document.id.clone();
                    row.candidate = candidate.id.clone();
                    row.arm = arm.clone();
                    row.repeat = repeat;
                    report.rows.push(row);
                }
            }
        }
    }
    assert_eq!(report.rows.len(), 16);
    report.validate(&plan).unwrap();
    report.rows[15] = report.rows[0].clone();
    assert!(report.validate(&plan).unwrap_err().to_string().contains("duplicate"));
}

#[test]
fn an_unknown_effective_backend_cannot_deserialize_as_completed_cpu_work() {
    let mut value = serde_json::to_value(report(&plan())).unwrap();
    value["rows"][0]["model"]["effective_backend"] = json!("provider");
    assert!(
        serde_json::from_value::<nli_qualification::RunReport>(value)
            .unwrap_err()
            .to_string()
            .contains("unknown variant")
    );
}
