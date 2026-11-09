use super::artifacts::ArtifactInventory;
use super::policy::validate_config;
use super::tests::configuration;
use crate::candidate_models::{CLASSIFICATION_MODELS_REVISION, nli_minilm2_l6_h768_classifier};

#[test]
fn staged_nli_candidate_satisfies_the_adapters_own_artifact_validation() {
    let mut config = configuration();
    config.model = nli_minilm2_l6_h768_classifier().unwrap();
    assert_eq!(config.model.repository, "xberg-io/classification-models");
    assert_eq!(config.model.revision, CLASSIFICATION_MODELS_REVISION);
    validate_config(&config).unwrap();
    ArtifactInventory::new(&config.model).unwrap();
}

#[test]
fn staged_nli_candidate_with_an_omitted_or_floating_pin_is_rejected() {
    let candidate = nli_minilm2_l6_h768_classifier().unwrap();

    let mut omitted = configuration();
    omitted.model = candidate.clone();
    omitted.model.sha256_manifest = candidate
        .sha256_manifest
        .lines()
        .filter(|line| !line.ends_with("/tokenizer.json"))
        .map(|line| format!("{line}\n"))
        .collect();
    assert!(validate_config(&omitted).is_err());

    let mut floating = configuration();
    floating.model = candidate;
    floating.model.revision = "main".to_owned();
    assert!(validate_config(&floating).is_err());
}
