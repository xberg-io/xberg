#[path = "common/refusals.rs"]
mod cases;
mod common;
use cases::{plan_cases, report_cases};
use nli_qualification::{FrozenPlan, RunReport};

#[test]
fn refuses_each_invalid_plan_for_its_intended_contract_reason() {
    let cases = plan_cases();
    assert_eq!(cases.len(), 103, "plan refusal inventory changed");
    for case in cases {
        let plan: FrozenPlan = serde_json::from_value(case.input).unwrap();
        let error = plan.validate().expect_err(&case.name).to_string();
        assert!(
            error.contains(case.error),
            "{}: expected {}, received {error}",
            case.name,
            case.error
        );
    }
}

#[test]
fn refuses_each_tampered_model_identity_and_report_field() {
    let plan = common::plan();
    plan.validate().unwrap();
    let cases = report_cases();
    assert_eq!(cases.len(), 31, "report refusal inventory changed");
    for case in cases {
        let error = serde_json::from_value::<RunReport>(case.input)
            .map_err(|error| error.to_string())
            .and_then(|report| report.validate(&plan).map_err(|error| error.to_string()))
            .expect_err(&case.name);
        assert!(
            error.contains(case.error),
            "{}: expected {}, received {error}",
            case.name,
            case.error
        );
    }
    for elapsed in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN, -1.0] {
        let mut report = common::report(&plan);
        report.rows[0].elapsed_ms = elapsed;
        let error = report.validate(&plan).unwrap_err().to_string();
        assert!(
            error.contains("positive finite measured duration"),
            "{elapsed}: {error}"
        );
    }
}

#[cfg(feature = "runtime")]
#[test]
fn portable_admission_agrees_with_sdk_constructor_on_edge_configurations() {
    use xberg::local_classification::LocalTextClassifier;
    let mut compared = 0;
    let mut refused_names = std::collections::BTreeSet::new();
    for case in cases::plan_cases().into_iter().filter(|case| {
        (case.name.starts_with("/candidates/0/variants/fp32/config/")
            && case.error != "NLI request exceeds variant input/label limits")
            || case.name.starts_with("artifact path ")
            || case.name == "repository byte ceiling"
    }) {
        let plan: FrozenPlan = serde_json::from_value(case.input).unwrap();
        let portable = plan.validate_portable();
        let sdk = LocalTextClassifier::new(plan.candidates[0].variants["fp32"].config.clone());
        assert!(portable.is_err(), "portable admitted {}", case.name);
        assert!(sdk.is_err(), "SDK admitted {}", case.name);
        compared += 1;
        refused_names.insert(case.name);
    }
    assert!(
        compared >= 40,
        "SDK refusal comparison examined only {compared} configs"
    );
    for name in [
        "artifact path COM¹.onnx",
        "artifact path LPT².onnx",
        "artifact path COM³",
    ] {
        assert!(refused_names.contains(name), "SDK agreement omitted {name}");
    }
    for path in ["model.onnx", "COM0.onnx", "LPT10.onnx", "models/ordinary.onnx"] {
        let mut plan = common::plan();
        let model = &mut plan.candidates[0].variants.get_mut("fp32").unwrap().config.model;
        model.sha256_manifest = model.sha256_manifest.replace("model.onnx", path);
        model.model_file = path.into();
        plan.validate_portable().unwrap();
        LocalTextClassifier::new(plan.candidates[0].variants["fp32"].config.clone())
            .unwrap_or_else(|error| panic!("SDK refused valid {path}: {error}"));
    }
}

#[test]
fn rejects_prediction_values_counts_and_duplicates_for_their_scoring_mode() {
    let cases = scoring_cases();
    for (multi_label, threshold, labels, expected) in cases {
        let mut plan = common::plan();
        let config = &mut plan.candidates[0].variants.get_mut("fp32").unwrap().config;
        config.multi_label = multi_label;
        config.confidence_threshold = threshold;
        plan.validate().unwrap();
        let mut report = common::report(&plan);
        report.rows[0].labels = serde_json::from_value(labels).unwrap();
        let error = report.validate(&plan).unwrap_err().to_string();
        assert!(error.contains(expected), "expected {expected}, received {error}");
    }
}

fn scoring_cases() -> Vec<(bool, f32, serde_json::Value, &'static str)> {
    use serde_json::json;
    let science = json!({"label": "science", "confidence": 0.9});
    let finance = json!({"label": "finance", "confidence": 0.9});
    let score = |confidence| json!([{"label": "science", "confidence": confidence}]);
    vec![
        (
            false,
            0.75,
            json!([science, finance]),
            "invalid single-label prediction count",
        ),
        (true, 0.75, json!([science, science]), "invalid prediction"),
        (true, 0.75, score(json!(1.1)), "invalid prediction"),
        (true, 0.75, score(json!(-0.1)), "invalid prediction"),
        (true, 0.75, score(json!(null)), "invalid prediction"),
        (true, 0.0, json!([science]), "zero-threshold multi-label cardinality"),
        (false, 0.75, score(json!(0.4)), "single-label maximum probability"),
    ]
}
