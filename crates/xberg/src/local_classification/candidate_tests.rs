use super::LocalTextClassifier;
use super::artifacts::ArtifactInventory;
use super::policy::validate_config;
use super::tests::configuration;
use crate::candidate_models::{
    CANDIDATE_MODELS, CLASSIFICATION_MODELS_REVISION, nli_classifier, nli_minilm2_l6_h768_classifier,
    nli_minilm2_l6_h768_graph,
};
use sha2::{Digest, Sha256};

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
fn the_descriptor_selects_a_pinned_quantized_graph_never_the_fp32_graph() {
    let Some(graph) = nli_minilm2_l6_h768_graph() else {
        return;
    };
    assert_eq!(Some(graph), expected_host_graph());
    let model = nli_minilm2_l6_h768_classifier().unwrap();
    assert_eq!(model.model_file, format!("nli-minilm2-l6-h768/{graph}"));
    assert!(!model.sha256_manifest.contains("nli-minilm2-l6-h768/model.onnx"));
}

/// Stand-in pins for the NLI candidate: the production inventory's layout and graph selection
/// with small files, so the pinned bytes can be staged in a temp cache.
struct StagedNli {
    directory: tempfile::TempDir,
    config: super::LocalClassificationConfig,
    host_graph_sha256: String,
}

impl StagedNli {
    fn path(&self, name: &str) -> std::path::PathBuf {
        self.directory
            .path()
            .join("models--xberg-io--classification-models/snapshots")
            .join(CLASSIFICATION_MODELS_REVISION)
            .join("nli-minilm2-l6-h768")
            .join(name)
    }
}

/// The quantized graph the host architecture must select, independent of the production selector.
fn expected_host_graph() -> Option<&'static str> {
    if cfg!(target_arch = "aarch64") {
        Some("model_qint8_arm64.onnx")
    } else if cfg!(target_arch = "x86_64") {
        Some("model_quint8_avx2.onnx")
    } else {
        None
    }
}

fn stage_nli(host_graph: &str) -> StagedNli {
    let directory = tempfile::tempdir().unwrap();
    let mut tokenizer = super::tokenization_tests::tokenizer();
    tokenizer
        .add_tokens([tokenizers::AddedToken::from("finance", false)])
        .unwrap();
    let metadata = serde_json::json!({"model_type":"bert", "architectures":["BertForSequenceClassification"],
        "id2label":{"0":"contradiction","1":"entailment","2":"neutral"},
        "max_position_embeddings":512, "pad_token_id":1});
    let host_graph_bytes = super::toy_model::token_sum_nli_model_bytes(3);
    let mut files: Vec<(&str, Vec<u8>)> = vec![
        ("tokenizer.json", tokenizer.to_string(false).unwrap().into_bytes()),
        ("config.json", serde_json::to_vec(&metadata).unwrap()),
        (
            "tokenizer_config.json",
            br#"{"model_max_length":512,"pad_token":"[PAD]"}"#.to_vec(),
        ),
        (
            "special_tokens_map.json",
            br#"{"pad_token":"[PAD]","unk_token":"[UNK]"}"#.to_vec(),
        ),
    ];
    for graph in ["model.onnx", "model_qint8_arm64.onnx", "model_quint8_avx2.onnx"] {
        let bytes = if graph == host_graph {
            host_graph_bytes.clone()
        } else {
            format!("not the graph the host selects: {graph}").into_bytes()
        };
        files.push((graph, bytes));
    }
    let mut manifest = String::new();
    let staged = StagedNli {
        directory,
        config: configuration(),
        host_graph_sha256: hex::encode(Sha256::digest(&host_graph_bytes)),
    };
    std::fs::create_dir_all(staged.path("")).unwrap();
    for (name, bytes) in files {
        std::fs::write(staged.path(name), &bytes).unwrap();
        manifest.push_str(&format!(
            "{}  nli-minilm2-l6-h768/{name}\n",
            hex::encode(Sha256::digest(&bytes))
        ));
    }
    let mut probe = CANDIDATE_MODELS[0];
    probe.sha256_manifest = manifest.leak();
    let mut staged = staged;
    staged.config.model = nli_classifier(&probe).unwrap();
    staged.config.hypothesis_template = "topic {label}".to_owned();
    staged.config.cache_directory = Some(staged.directory.path().to_str().unwrap().to_owned());
    staged
}

async fn prepare(staged: &StagedNli) -> crate::Result<super::LocalClassificationModelInfo> {
    LocalTextClassifier::new(staged.config.clone())?
        .prepare(&Default::default())
        .await
}

#[tokio::test]
async fn staged_pins_load_through_the_adapter_with_the_graph_the_host_selects() {
    let Some(graph) = expected_host_graph() else {
        return;
    };
    let staged = stage_nli(graph);
    let info = prepare(&staged).await.unwrap();
    assert_eq!(info.model_file, format!("nli-minilm2-l6-h768/{graph}"));
    assert_eq!(info.model_sha256, staged.host_graph_sha256);
    assert_eq!(info.revision, CLASSIFICATION_MODELS_REVISION);
}

#[tokio::test]
async fn one_corrupted_byte_in_any_staged_file_is_rejected_by_the_adapter() {
    let Some(graph) = expected_host_graph() else {
        return;
    };
    for name in [
        graph,
        "tokenizer.json",
        "config.json",
        "tokenizer_config.json",
        "special_tokens_map.json",
    ] {
        let staged = stage_nli(graph);
        let mut bytes = std::fs::read(staged.path(name)).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        std::fs::write(staged.path(name), &bytes).unwrap();
        let error = prepare(&staged).await.unwrap_err().to_string();
        assert!(
            error.contains("checksum mismatch") && error.contains(name),
            "{name}: {error}"
        );
    }
}

#[tokio::test]
async fn an_emptied_staged_graph_is_rejected_by_the_adapters_size_check() {
    let Some(graph) = expected_host_graph() else {
        return;
    };
    let staged = stage_nli(graph);
    std::fs::write(staged.path(graph), b"").unwrap();
    let error = prepare(&staged).await.unwrap_err().to_string();
    assert!(error.contains("empty or exceeds"), "{error}");
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
