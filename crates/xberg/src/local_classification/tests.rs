use std::time::{Duration, Instant};

use ndarray::{ArrayD, IxDyn, arr2};

use super::control::check_control;
use super::decoding::decode_logits;
use super::policy::{model_identity, validate_config, validate_cpu_execution, validate_request};
use super::{LocalClassificationConfig, LocalClassificationLimits, LocalClassificationOptions, LocalNliModel};

pub(super) fn configuration() -> LocalClassificationConfig {
    let manifest = [
        "model.onnx",
        "tokenizer.json",
        "config.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
    ]
    .map(|path| format!("{}  {path}\n", "a".repeat(64)))
    .concat();
    LocalClassificationConfig {
        model: LocalNliModel {
            repository: "owner/nli-model".to_owned(),
            revision: "a".repeat(40),
            model_file: "model.onnx".to_owned(),
            external_files: vec![],
            sha256_manifest: manifest,
            output_name: "logits".to_owned(),
            entailment_index: 2,
            contradiction_index: 0,
        },
        limits: LocalClassificationLimits::default(),
        hypothesis_template: "This text is about {label}.".to_owned(),
        multi_label: false,
        confidence_threshold: 0.0,
        cache_directory: None,
    }
}

fn labels() -> Vec<String> {
    vec!["science".to_owned(), "finance".to_owned()]
}

#[test]
fn complete_immutable_descriptor_and_bounded_request_are_accepted() {
    let config = configuration();
    validate_config(&config).unwrap();
    validate_request("A short scientific article.", &labels(), &config).unwrap();
}

#[test]
fn floating_or_malformed_revisions_are_rejected_before_preparation() {
    for revision in ["main", "v1", "", &"z".repeat(40), &"a".repeat(39)] {
        let mut config = configuration();
        config.model.revision = revision.to_owned();
        assert!(validate_config(&config).is_err(), "revision {revision}");
    }
}

#[test]
fn three_way_nli_indices_must_be_distinct_and_in_range() {
    for (entailment, contradiction) in [(0, 0), (3, 0), (2, 3)] {
        let mut config = configuration();
        config.model.entailment_index = entailment;
        config.model.contradiction_index = contradiction;
        assert!(validate_config(&config).is_err());
    }
}

#[test]
fn every_selected_graph_and_external_asset_requires_a_manifest_pin() {
    let mut config = configuration();
    config.model.external_files = vec!["weights.data".to_owned()];
    assert!(validate_config(&config).is_err());
    config.model.external_files.clear();
    config.model.model_file = "missing.onnx".to_owned();
    assert!(validate_config(&config).is_err());
}

#[test]
fn tokenizer_and_configuration_assets_cannot_be_silently_unverified() {
    for missing in [
        "tokenizer.json",
        "config.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
    ] {
        let mut config = configuration();
        config.model.sha256_manifest = config
            .model
            .sha256_manifest
            .lines()
            .filter(|line| !line.ends_with(&format!("  {missing}")))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(validate_config(&config).is_err(), "missing {missing}");
    }
}

#[test]
fn hypothesis_templates_require_exactly_one_label_placeholder() {
    for template in ["No label", "{label} and {label}", ""] {
        let mut config = configuration();
        config.hypothesis_template = template.to_owned();
        assert!(validate_config(&config).is_err());
    }
}

#[test]
fn invalid_probability_thresholds_and_limits_fail_before_tokenization() {
    for threshold in [-0.1, 1.1, f32::NAN, f32::INFINITY] {
        let mut config = configuration();
        config.confidence_threshold = threshold;
        assert!(validate_config(&config).is_err());
    }
    let mut config = configuration();
    config.limits.max_tokens = 0;
    assert!(validate_config(&config).is_err());
    config.limits.max_tokens = u32::MAX;
    assert!(validate_config(&config).is_err());
}

#[test]
fn blank_duplicate_and_oversized_inputs_and_labels_are_rejected() {
    let mut config = configuration();
    config.limits.max_input_bytes = 8;
    assert!(validate_request("ninebytes", &labels(), &config).is_err());
    assert!(validate_request(" ", &labels(), &config).is_err());
    assert!(validate_request("text", &[], &config).is_err());
    assert!(validate_request("text", &["science".into(), " science ".into()], &config).is_err());
    config.limits.max_label_bytes = 2;
    assert!(validate_request("text", &["long".into()], &config).is_err());
    assert!(validate_request("text", &[" ".into()], &config).is_err());
}

#[test]
fn label_count_is_bounded_before_pair_encoding() {
    let mut config = configuration();
    config.limits.max_labels = 1;
    assert!(validate_request("text", &labels(), &config).is_err());
}

#[test]
fn caller_limits_cannot_disable_or_raise_the_hard_resource_ceilings() {
    for limits in [
        LocalClassificationLimits {
            max_input_bytes: 0,
            ..Default::default()
        },
        LocalClassificationLimits {
            max_label_bytes: 0,
            ..Default::default()
        },
        LocalClassificationLimits {
            max_labels: 0,
            ..Default::default()
        },
        LocalClassificationLimits {
            max_input_bytes: u32::MAX,
            ..Default::default()
        },
        LocalClassificationLimits {
            max_label_bytes: u32::MAX,
            ..Default::default()
        },
        LocalClassificationLimits {
            max_labels: u32::MAX,
            ..Default::default()
        },
    ] {
        let mut config = configuration();
        config.limits = limits;
        assert!(validate_config(&config).is_err());
    }
}

#[test]
fn explicit_cpu_and_an_empty_or_cpu_only_environment_are_accepted() {
    use crate::core::config::acceleration::ExecutionProviderType;
    validate_cpu_execution(None, &[]).unwrap();
    validate_cpu_execution(Some(&ExecutionProviderType::Cpu), &["CPUExecutionProvider"]).unwrap();
}

#[test]
fn conflicting_overrides_and_inherited_non_cpu_providers_are_rejected() {
    use crate::core::config::acceleration::ExecutionProviderType;
    for provider in [
        ExecutionProviderType::Auto,
        ExecutionProviderType::Cuda,
        ExecutionProviderType::CoreMl,
    ] {
        assert!(validate_cpu_execution(Some(&provider), &[]).is_err());
    }
    assert!(validate_cpu_execution(None, &["CUDAExecutionProvider"]).is_err());
}

#[test]
fn model_cache_identity_separates_revisions_and_artifact_checksums() {
    let original = configuration();
    let mut changed = original.clone();
    changed.model.revision = "b".repeat(40);
    assert_ne!(model_identity(&original), model_identity(&changed));
    changed = original.clone();
    changed.model.sha256_manifest = changed.model.sha256_manifest.replace('a', "b");
    assert_ne!(model_identity(&original), model_identity(&changed));
    assert_eq!(model_identity(&original), model_identity(&original.clone()));
}

#[test]
fn single_label_probabilities_use_stable_softmax_and_return_only_the_best_label() {
    let config = configuration();
    let logits = arr2(&[[0.0, 0.0, 1_000.0], [0.0, 0.0, 999.0]]).into_dyn();
    let result = decode_logits(&labels(), &logits, &config).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].label, "science");
    assert!((result[0].confidence.unwrap() - 0.731_058_6).abs() < 0.000_001);
}

#[test]
fn multi_label_probabilities_compare_entailment_and_contradiction_without_neutral_leakage() {
    let mut config = configuration();
    config.multi_label = true;
    let logits = arr2(&[[0.0, 1_000_000.0, 0.0], [-1_000.0, 0.0, 1_000.0]]).into_dyn();
    let result = decode_logits(&labels(), &logits, &config).unwrap();
    assert_eq!(
        result.iter().map(|label| label.label.as_str()).collect::<Vec<_>>(),
        ["science", "finance"]
    );
    assert_eq!(result[0].confidence, Some(0.5));
    assert_eq!(result[1].confidence, Some(1.0));
}

#[test]
fn probabilities_below_the_caller_threshold_abstain() {
    let mut config = configuration();
    config.multi_label = true;
    config.confidence_threshold = 0.75;
    let logits = arr2(&[[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]).into_dyn();
    assert!(decode_logits(&labels(), &logits, &config).unwrap().is_empty());
}

#[test]
fn logits_must_match_the_label_count_and_three_way_nli_head() {
    for logits in [
        ArrayD::zeros(IxDyn(&[1, 3])),
        ArrayD::zeros(IxDyn(&[2, 4])),
        ArrayD::zeros(IxDyn(&[2, 1, 3])),
    ] {
        assert!(decode_logits(&labels(), &logits, &configuration()).is_err());
    }
}

#[test]
fn nonfinite_logits_cannot_be_reported_as_confidences() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let logits = arr2(&[[value, 0.0, 1.0], [0.0, 0.0, 0.0]]).into_dyn();
        assert!(decode_logits(&labels(), &logits, &configuration()).is_err());
    }
}

#[test]
fn uncancelled_requests_without_a_deadline_are_allowed() {
    check_control(&LocalClassificationOptions::default(), Instant::now()).unwrap();
}

#[test]
fn pre_cancelled_requests_stop_before_permit_wait_or_model_access() {
    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    let options = LocalClassificationOptions {
        cancel_token: Some(token),
        deadline_ms: None,
    };
    assert!(matches!(
        check_control(&options, Instant::now()),
        Err(crate::XbergError::Cancelled)
    ));
}

#[test]
fn expired_requests_stop_before_tokenization_or_native_work() {
    let options = LocalClassificationOptions {
        deadline_ms: Some(1),
        cancel_token: None,
    };
    let started = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
    assert!(matches!(
        check_control(&options, started),
        Err(crate::XbergError::Timeout { .. })
    ));
}

#[test]
fn rejects_oversized_descriptors_before_manifest_or_cache_resolution() {
    let mut config = configuration();
    for field in ["repository", "model", "external", "output", "cache"] {
        let mut case = config.clone();
        match field {
            "repository" => case.model.repository = format!("owner/{}", "x".repeat(8192)),
            "model" => case.model.model_file = "x".repeat(8192),
            "external" => case.model.external_files = vec!["x".repeat(8192)],
            "output" => case.model.output_name = "x".repeat(8192),
            "cache" => case.cache_directory = Some("x".repeat(8192)),
            _ => unreachable!(),
        }
        let error = validate_config(&case).unwrap_err();
        assert!(error.to_string().contains("byte limit"), "{field}: {error}");
    }
    config.model.repository = "owner/nested/model".to_owned();
    assert!(validate_config(&config).is_err());
}
