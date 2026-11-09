use super::artifacts::{
    ArtifactInventory, MAX_ARTIFACT_BYTES, MAX_GRAPH_BYTES, MAX_METADATA_BYTES, MAX_TOKENIZER_BYTES,
};
use super::policy::validate_config;
use super::tests::configuration;
use crate::candidate_models::{
    CANDIDATE_MODELS, CLASSIFICATION_MODELS_REVISION, nli_minilm2_l6_h768_classifier,
    nli_minilm2_l6_h768_classifier_for, nli_minilm2_l6_h768_graph,
};

fn host_candidate() -> Option<super::LocalNliModel> {
    nli_minilm2_l6_h768_graph().map(|_| nli_minilm2_l6_h768_classifier().unwrap())
}

#[test]
fn staged_nli_candidate_satisfies_the_adapters_own_artifact_validation() {
    let Some(model) = host_candidate() else {
        return;
    };
    let mut config = configuration();
    config.model = model;
    assert_eq!(config.model.repository, "xberg-io/classification-models");
    assert_eq!(config.model.revision, CLASSIFICATION_MODELS_REVISION);
    validate_config(&config).unwrap();
    ArtifactInventory::new(&config.model).unwrap();
}

#[test]
fn every_file_the_nli_descriptor_lists_is_within_the_adapters_caps() {
    let nli = &CANDIDATE_MODELS[0];
    let files = nli.files().unwrap();
    let size_of = |path: &str| files.iter().find(|f| f.path == path).unwrap().size_bytes;
    for graph in ["model_qint8_arm64.onnx", "model_quint8_avx2.onnx"] {
        let descriptor = nli_minilm2_l6_h768_classifier_for(graph).unwrap();
        let mut total = 0;
        for line in descriptor.sha256_manifest.lines() {
            let path = line.split_once("  ").unwrap().1;
            let size = size_of(path);
            let cap = match path.rsplit('/').next().unwrap() {
                name if name == graph => MAX_GRAPH_BYTES,
                "tokenizer.json" => MAX_TOKENIZER_BYTES,
                _ => MAX_METADATA_BYTES,
            };
            assert!(size <= cap as u64, "{path} is {size} bytes, over its {cap} byte cap");
            total += size;
        }
        assert!(total <= MAX_ARTIFACT_BYTES as u64, "{graph} totals {total} bytes");
    }
    assert!(
        size_of("nli-minilm2-l6-h768/model.onnx") > MAX_GRAPH_BYTES as u64,
        "the fp32 graph is over the cap, which is why the descriptor never selects it"
    );
}

#[test]
fn staged_nli_candidate_with_an_omitted_or_floating_pin_is_rejected() {
    let Some(candidate) = host_candidate() else {
        return;
    };

    let mut omitted = configuration();
    omitted.model = candidate.clone();
    omitted.model.sha256_manifest = candidate
        .sha256_manifest
        .lines()
        .filter(|line| !line.ends_with("/tokenizer.json"))
        .map(|line| format!("{line}\n"))
        .collect();
    let error = validate_config(&omitted).unwrap_err().to_string();
    assert!(
        error.contains("lacks a pin: tokenizer.json") || error.contains("nli-minilm2-l6-h768/tokenizer.json"),
        "{error}"
    );

    let mut floating = configuration();
    floating.model = candidate;
    floating.model.revision = "main".to_owned();
    let error = validate_config(&floating).unwrap_err().to_string();
    assert!(error.contains("immutable forty-digit hexadecimal revision"), "{error}");
}
