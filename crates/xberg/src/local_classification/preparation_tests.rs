use super::tests::configuration;
use super::{LocalClassificationBackend, LocalClassificationConfig, LocalClassificationOptions, LocalTextClassifier};
use sha2::{Digest, Sha256};
use std::sync::Arc;

struct Fixture {
    directory: tempfile::TempDir,
    config: LocalClassificationConfig,
}

impl Fixture {
    fn snapshot(&self) -> std::path::PathBuf {
        self.directory
            .path()
            .join("models--owner--nli-model/snapshots")
            .join(&self.config.model.revision)
    }
}

fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let mut config = configuration();
    config.hypothesis_template = "topic {label}".to_owned();
    config.cache_directory = Some(directory.path().to_str().unwrap().to_owned());
    let mut fixture = Fixture { directory, config };
    let mut tokenizer = super::tokenization_tests::tokenizer();
    tokenizer
        .add_tokens([tokenizers::AddedToken::from("finance", false)])
        .unwrap();
    let metadata = serde_json::json!({"model_type":"bert", "architectures":["BertForSequenceClassification"],
        "id2label":{"0":"contradiction", "1":"neutral", "2":"entailment"},
        "max_position_embeddings":512, "pad_token_id":1});
    let files = [
        ("model.onnx", super::toy_model::token_sum_nli_model_bytes(3)),
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
    std::fs::create_dir_all(fixture.snapshot()).unwrap();
    fixture.config.model.sha256_manifest.clear();
    fixture.config.model.size_manifest.clear();
    for (name, bytes) in files {
        std::fs::write(fixture.snapshot().join(name), &bytes).unwrap();
        fixture
            .config
            .model
            .sha256_manifest
            .push_str(&format!("{}  {name}\n", hex::encode(Sha256::digest(&bytes))));
        fixture
            .config
            .model
            .size_manifest
            .push_str(&format!("{}  {name}\n", bytes.len()));
    }
    fixture
}

fn labels() -> Vec<String> {
    vec!["science".to_owned(), "finance".to_owned()]
}

#[tokio::test]
async fn explicit_preparation_binds_model_identity_and_executes_named_token_pairs() {
    let fixture = fixture();
    let classifier = LocalTextClassifier::new(fixture.config.clone()).unwrap();
    let info = classifier.prepare(&Default::default()).await.unwrap();
    assert_eq!(info.repository, fixture.config.model.repository);
    assert_eq!(info.revision, fixture.config.model.revision);
    assert_eq!(info.selected_backend, LocalClassificationBackend::OnnxCpu);
    assert_eq!(info.effective_backend, LocalClassificationBackend::OnnxCpu);
    assert!(info.experimental);
    assert_eq!(
        info.model_sha256,
        hex::encode(Sha256::digest(
            std::fs::read(fixture.snapshot().join("model.onnx")).unwrap()
        ))
    );
    let result = classifier
        .classify("alpha", &labels(), &Default::default())
        .await
        .unwrap();
    assert_eq!(result.model, info);
    assert_eq!(result.labels.len(), 1);
    assert_eq!(result.labels[0].label, "finance");
    assert!((result.labels[0].confidence.unwrap() - 0.549834).abs() < 0.00001);
}

#[tokio::test]
async fn independent_multi_label_scores_use_each_complete_hypothesis() {
    let mut fixture = fixture();
    fixture.config.multi_label = true;
    let classifier = LocalTextClassifier::new(fixture.config.clone()).unwrap();
    classifier.prepare(&Default::default()).await.unwrap();
    let result = classifier
        .classify("alpha", &labels(), &Default::default())
        .await
        .unwrap();
    assert_eq!(
        result
            .labels
            .iter()
            .map(|label| label.label.as_str())
            .collect::<Vec<_>>(),
        ["science", "finance"]
    );
    for (label, expected) in result.labels.iter().zip([0.9241418, 0.9370266]) {
        assert!((label.confidence.unwrap() - expected).abs() < 0.00001, "{label:?}");
    }
}

#[tokio::test]
async fn prepared_sessions_serve_concurrent_requests_without_artifact_access() {
    let fixture = fixture();
    let classifier = Arc::new(LocalTextClassifier::new(fixture.config.clone()).unwrap());
    classifier.prepare(&Default::default()).await.unwrap();
    std::fs::remove_dir_all(fixture.snapshot()).unwrap();
    let mut workers = Vec::new();
    for _ in 0..4 {
        let classifier = Arc::clone(&classifier);
        workers.push(tokio::spawn(async move {
            classifier.classify("alpha", &labels(), &Default::default()).await
        }));
    }
    for worker in workers {
        let result = worker.await.unwrap().unwrap();
        assert_eq!(result.labels.len(), 1);
        assert_eq!(result.labels[0].label, "finance");
    }
}

#[tokio::test]
async fn preparation_rejects_pinned_fixed_taxonomy_metadata_before_inference() {
    let mut fixture = fixture();
    let path = fixture.snapshot().join("config.json");
    let mut metadata: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    metadata["id2label"] = serde_json::json!({"0":"sports","1":"finance","2":"other"});
    let bytes = serde_json::to_vec(&metadata).unwrap();
    std::fs::write(path, &bytes).unwrap();
    fixture.config.model.sha256_manifest = fixture
        .config
        .model
        .sha256_manifest
        .lines()
        .map(|line| {
            if line.ends_with("  config.json") {
                format!("{}  config.json\n", hex::encode(Sha256::digest(&bytes)))
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    fixture.config.model.size_manifest = fixture
        .config
        .model
        .size_manifest
        .lines()
        .map(|line| {
            if line.ends_with("  config.json") {
                format!("{}  config.json\n", bytes.len())
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    let classifier = LocalTextClassifier::new(fixture.config).unwrap();
    let error = classifier.prepare(&Default::default()).await.unwrap_err();
    assert!(error.to_string().contains("NLI label"), "{error}");
}

#[tokio::test]
async fn preparation_observes_cancellation_before_resolving_a_missing_cache() {
    let classifier = LocalTextClassifier::new(configuration()).unwrap();
    let token = crate::cancellation::CancellationToken::new();
    token.cancel();
    let options = LocalClassificationOptions {
        cancel_token: Some(token),
        deadline_ms: None,
    };
    assert!(matches!(
        classifier.prepare(&options).await,
        Err(crate::XbergError::Cancelled)
    ));
}

#[tokio::test]
async fn preparation_observes_an_expired_deadline_before_resolving_a_missing_cache() {
    let classifier = LocalTextClassifier::new(configuration()).unwrap();
    let options = LocalClassificationOptions {
        cancel_token: None,
        deadline_ms: Some(0),
    };
    assert!(matches!(
        classifier.prepare(&options).await,
        Err(crate::XbergError::Timeout { .. })
    ));
}

#[tokio::test]
async fn prepared_inference_rejects_a_complete_pair_above_the_token_budget() {
    let mut fixture = fixture();
    fixture.config.limits.max_tokens = 6;
    let classifier = LocalTextClassifier::new(fixture.config.clone()).unwrap();
    classifier.prepare(&Default::default()).await.unwrap();
    let error = classifier
        .classify("alpha beta", &labels(), &Default::default())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("token limit"), "{error}");
}

#[tokio::test]
async fn the_actual_graph_observes_premise_tokens_as_well_as_hypothesis_tokens() {
    let mut fixture = fixture();
    fixture.config.multi_label = true;
    let classifier = LocalTextClassifier::new(fixture.config.clone()).unwrap();
    classifier.prepare(&Default::default()).await.unwrap();
    let selected = ["science".to_owned()];
    let alpha = classifier
        .classify("alpha", &selected, &Default::default())
        .await
        .unwrap();
    let beta = classifier
        .classify("beta", &selected, &Default::default())
        .await
        .unwrap();
    assert!((alpha.labels[0].confidence.unwrap() - 0.9241418).abs() < 0.00001);
    assert!((beta.labels[0].confidence.unwrap() - 0.9308616).abs() < 0.00001);
}

fn external_fixture(declare_weights: bool) -> Fixture {
    let mut fixture = fixture();
    let graph = super::toy_model::external_token_sum_nli_model_bytes("weights.data");
    let weights = 0.1f32.to_le_bytes();
    std::fs::write(fixture.snapshot().join("model.onnx"), &graph).unwrap();
    std::fs::write(fixture.snapshot().join("weights.data"), weights).unwrap();
    fixture.config.model.sha256_manifest = fixture
        .config
        .model
        .sha256_manifest
        .lines()
        .map(|line| {
            if line.ends_with("  model.onnx") {
                format!("{}  model.onnx\n", hex::encode(Sha256::digest(&graph)))
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    fixture.config.model.size_manifest = fixture
        .config
        .model
        .size_manifest
        .lines()
        .map(|line| {
            if line.ends_with("  model.onnx") {
                format!("{}  model.onnx\n", graph.len())
            } else {
                format!("{line}\n")
            }
        })
        .collect();
    if declare_weights {
        fixture.config.model.external_files = vec!["weights.data".to_owned()];
        fixture
            .config
            .model
            .sha256_manifest
            .push_str(&format!("{}  weights.data\n", hex::encode(Sha256::digest(weights)),));
        fixture
            .config
            .model
            .size_manifest
            .push_str(&format!("{}  weights.data\n", weights.len()));
    }
    fixture
}

#[tokio::test]
async fn declared_pinned_external_weights_are_a_valid_operational_graph_control() {
    let fixture = external_fixture(true);
    let classifier = LocalTextClassifier::new(fixture.config.clone()).unwrap();
    classifier.prepare(&Default::default()).await.unwrap();
    let result = classifier
        .classify("alpha", &labels(), &Default::default())
        .await
        .unwrap();
    assert_eq!(result.labels[0].label, "finance");
    assert!((result.labels[0].confidence.unwrap() - 0.549834).abs() < 0.00001);
}

#[tokio::test]
async fn undeclared_unpinned_external_weights_cannot_enter_a_prepared_session() {
    let fixture = external_fixture(false);
    assert!(fixture.config.model.external_files.is_empty());
    assert!(!fixture.config.model.sha256_manifest.contains("weights.data"));
    let classifier = LocalTextClassifier::new(fixture.config.clone()).unwrap();
    let error = classifier.prepare(&Default::default()).await.unwrap_err();
    assert!(error.to_string().contains("external"), "{error}");
    let error = classifier
        .classify("alpha", &labels(), &Default::default())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("prepare"), "{error}");
}

#[tokio::test]
async fn preparation_reports_missing_prestaged_cache_without_network_repair() {
    let fixture = fixture();
    std::fs::remove_file(fixture.snapshot().join("tokenizer.json")).unwrap();
    let classifier = LocalTextClassifier::new(fixture.config).unwrap();
    let error = classifier.prepare(&Default::default()).await.unwrap_err();
    assert!(
        error.to_string().contains("missing from the pre-staged cache"),
        "{error}"
    );
    assert!(error.to_string().contains("tokenizer.json"), "{error}");
}

#[tokio::test]
async fn preparation_rejects_oversized_graph_tokenizer_and_external_data_before_parsing() {
    for (asset, bytes) in [
        ("model.onnx", 256 * 1024 * 1024 + 1),
        ("tokenizer.json", 16 * 1024 * 1024 + 1),
        ("weights.data", 256 * 1024 * 1024 + 1),
    ] {
        let fixture = external_fixture(true);
        let path = fixture.snapshot().join(asset);
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .unwrap()
            .set_len(bytes)
            .unwrap();
        let classifier = LocalTextClassifier::new(fixture.config).unwrap();
        let error = classifier.prepare(&Default::default()).await.unwrap_err();
        assert!(
            error.to_string().contains("remaining artifact byte limit"),
            "{asset}: {error}"
        );
        assert!(error.to_string().contains(asset), "{error}");
    }
}

#[tokio::test]
async fn preparation_rejects_a_corrupt_tokenizer_snapshot_before_parsing() {
    let fixture = fixture();
    let path = fixture.snapshot().join("tokenizer.json");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[0] ^= 1;
    std::fs::write(path, bytes).unwrap();
    let classifier = LocalTextClassifier::new(fixture.config).unwrap();
    let error = classifier.prepare(&Default::default()).await.unwrap_err();
    assert!(
        error.to_string().contains("checksum mismatch: tokenizer.json"),
        "{error}"
    );
}

#[test]
fn the_offline_artifact_report_distinguishes_staged_missing_and_mismatched_files() {
    use super::{LocalClassificationArtifactReport, LocalClassificationArtifactState};

    let fixture = fixture();
    let cache = fixture.directory.path().to_str().unwrap();
    let report = LocalClassificationArtifactReport::for_model(&fixture.config.model, Some(cache)).unwrap();
    assert!(report.complete());
    assert_eq!(report.staged(), fixture.config.model.sha256_manifest.lines().count());
    assert_eq!(report.repository, fixture.config.model.repository);
    // A caller-pinned toy graph whose path carries no int8 marker is reported as fp32.
    assert_eq!(report.precision, super::LocalClassificationPrecision::Fp32);
    assert_eq!(report.model_file, "model.onnx");
    let model = report
        .artifacts
        .iter()
        .find(|artifact| artifact.path == "model.onnx")
        .unwrap();
    assert_eq!(model.state, LocalClassificationArtifactState::Staged);
    assert_eq!(
        model.sha256,
        hex::encode(Sha256::digest(
            std::fs::read(fixture.snapshot().join("model.onnx")).unwrap()
        ))
    );

    std::fs::write(fixture.snapshot().join("model.onnx"), b"tampered").unwrap();
    std::fs::remove_file(fixture.snapshot().join("tokenizer.json")).unwrap();
    let report = LocalClassificationArtifactReport::for_model(&fixture.config.model, Some(cache)).unwrap();
    assert!(!report.complete());
    assert_eq!(
        report.staged(),
        fixture.config.model.sha256_manifest.lines().count() - 2
    );
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "model.onnx" && artifact.state == LocalClassificationArtifactState::SizeMismatch
    }));
    assert!(report.artifacts.iter().any(|artifact| {
        artifact.path == "tokenizer.json" && artifact.state == LocalClassificationArtifactState::Missing
    }));
}

#[test]
fn a_candidate_report_from_an_empty_cache_is_offline_and_fully_missing() {
    use super::LocalClassificationArtifactReport;

    let directory = tempfile::tempdir().unwrap();
    let report = LocalClassificationArtifactReport::for_candidate(
        "nli-minilm2-l6-h768",
        super::LocalClassificationPrecision::Quantized,
        Some(directory.path().to_str().unwrap()),
    )
    .unwrap();
    assert_eq!(report.candidate.as_deref(), Some("nli-minilm2-l6-h768"));
    assert_eq!(report.repository, "xberg-io/classification-models");
    assert_eq!(report.revision, crate::candidate_models::CLASSIFICATION_MODELS_REVISION);
    assert!(report.empty());
    assert_eq!(report.staged(), 0);
    assert!(report.total_bytes > 0);
    assert!(report.model_file.contains("onnx"));
    assert!(report.artifacts.iter().all(|artifact| artifact.sha256.len() == 64));
}

#[tokio::test]
async fn an_offline_stage_and_prepare_do_not_fetch_and_a_missing_artifact_fails_explicitly() {
    let fixture = fixture();
    let classifier = LocalTextClassifier::new(fixture.config.clone()).unwrap();
    // The fabricated cache is complete, so a stage resolves every pin without a transfer.
    classifier.stage(&Default::default()).await.unwrap();
    classifier.prepare(&Default::default()).await.unwrap();

    std::fs::remove_file(fixture.snapshot().join("model.onnx")).unwrap();
    let captioner = LocalTextClassifier::new(fixture.config.clone()).unwrap();
    let error = captioner.prepare(&Default::default()).await.unwrap_err().to_string();
    assert!(error.contains("missing from the pre-staged cache"), "{error}");
}

#[tokio::test]
async fn alternating_model_identities_never_reuse_another_models_session() {
    let mut first = fixture();
    let mut second_fixture = fixture();
    second_fixture.config.model.revision = "b".repeat(40);
    // Re-stage the same bytes under the second revision so both caches are complete.
    let second_snapshot = second_fixture
        .directory
        .path()
        .join("models--owner--nli-model/snapshots")
        .join(&second_fixture.config.model.revision);
    std::fs::create_dir_all(&second_snapshot).unwrap();
    for entry in std::fs::read_dir(first.snapshot()).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), second_snapshot.join(entry.file_name())).unwrap();
    }
    let first_cache = first.directory.path().to_str().unwrap().to_owned();
    let second_cache = second_fixture.directory.path().to_str().unwrap().to_owned();
    first.config.cache_directory = Some(first_cache);
    second_fixture.config.cache_directory = Some(second_cache);

    let a = LocalTextClassifier::new(first.config.clone()).unwrap();
    let b = LocalTextClassifier::new(second_fixture.config.clone()).unwrap();
    let first_info = a.prepare(&Default::default()).await.unwrap();
    let second_info = b.prepare(&Default::default()).await.unwrap();
    assert_ne!(
        super::policy::model_identity(&first.config),
        super::policy::model_identity(&second_fixture.config),
        "the model identity binds the pinned revision"
    );
    assert_eq!(first_info.revision, first.config.model.revision);
    assert_eq!(second_info.revision, second_fixture.config.model.revision);
    let first_result = a.classify("alpha", &labels(), &Default::default()).await.unwrap();
    assert_eq!(first_result.model.revision, first.config.model.revision);
    let second_result = b.classify("alpha", &labels(), &Default::default()).await.unwrap();
    assert_eq!(second_result.model.revision, second_fixture.config.model.revision);
}
