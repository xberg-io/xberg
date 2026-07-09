//! Live validation for self-hosted SPLADE sparse-embedding presets.
//!
//! Downloads the preset's ONNX model from `xberg-io/sparse-embeddings` and runs
//! real inference, asserting a non-empty, in-vocab, strictly-positive sparse
//! vector. Opt out on offline dev with `XBERG_SKIP_LIVE_HF=1`.

#![cfg(feature = "sparse-embeddings")]

use xberg::core::config::{SparseEmbeddingConfig, SparseEmbeddingModelType};

fn should_skip() -> bool {
    std::env::var("XBERG_SKIP_LIVE_HF").is_ok()
}

#[test]
fn opensearch_v3_distill_sparse_embeds() {
    if should_skip() {
        eprintln!("XBERG_SKIP_LIVE_HF=1, skipping");
        return;
    }
    let preset = xberg::get_sparse_embedding_preset("opensearch-v3-distill").expect("preset must exist");
    assert_eq!(preset.model_repo, "xberg-io/sparse-embeddings");
    assert_eq!(preset.model_file, "opensearch-v3-distill/model.onnx");
    assert_eq!(preset.additional_files, vec!["opensearch-v3-distill/model.onnx.data".to_string()]);

    let config = SparseEmbeddingConfig {
        model: SparseEmbeddingModelType::Preset {
            name: "opensearch-v3-distill".to_string(),
        },
        ..Default::default()
    };
    let out = xberg::embed_sparse(vec!["the quick brown fox jumps over the lazy dog".to_string()], &config)
        .expect("sparse embed must succeed");

    assert_eq!(out.len(), 1, "one sparse vector per input");
    let se = &out[0];
    assert!(!se.indices.is_empty(), "sparse vector must have non-zero terms");
    assert_eq!(se.indices.len(), se.values.len(), "indices and values are parallel");
    assert!(se.indices.iter().all(|&i| i < 30522), "term ids within the 30522 vocab");
    assert!(
        se.values.iter().all(|&v| v > 0.0 && v.is_finite()),
        "SPLADE weights must be strictly positive and finite"
    );
}
