//! Live validation for self-hosted 2026-gen embedding presets.
//!
//! Downloads the preset's ONNX model from the `xberg-io/*` mirror and runs real
//! inference, asserting the output dimensionality and that embeddings are
//! non-degenerate and distinct across different inputs. Opt out on offline dev
//! with `XBERG_SKIP_LIVE_HF=1`.

#![cfg(feature = "embeddings")]

use xberg::core::config::{EmbeddingConfig, EmbeddingModelType};

fn should_skip() -> bool {
    std::env::var("XBERG_SKIP_LIVE_HF").is_ok()
}

fn embed_preset(name: &str) -> Vec<Vec<f32>> {
    let config = EmbeddingConfig {
        model: EmbeddingModelType::Preset { name: name.to_string() },
        normalize: true,
        batch_size: 2,
        ..Default::default()
    };
    xberg::embed_texts(
        vec![
            "The quick brown fox jumps over the lazy dog.".to_string(),
            "A treatise on the migratory patterns of arctic terns.".to_string(),
        ],
        &config,
    )
    .unwrap_or_else(|e| panic!("embed with preset {name}: {e}"))
}

fn assert_valid(out: &[Vec<f32>], dims: usize, name: &str) {
    assert_eq!(out.len(), 2, "{name}: one vector per input");
    assert_eq!(out[0].len(), dims, "{name}: expected {dims}-dim vectors");
    assert!(
        out[0].iter().all(|v| v.is_finite()),
        "{name}: all components must be finite"
    );
    assert!(out[0].iter().any(|&v| v != 0.0), "{name}: vector must not be all-zero");
    assert_ne!(out[0], out[1], "{name}: distinct inputs must yield distinct vectors");
}

#[test]
fn gte_modernbert_base_preset_embeds_768_dim() {
    if should_skip() {
        eprintln!("XBERG_SKIP_LIVE_HF=1, skipping");
        return;
    }
    let preset = xberg::get_embedding_preset("gte-modernbert-base").expect("preset must exist");
    assert_eq!(preset.model_repo, "xberg-io/embedding-models");
    assert_eq!(preset.model_file, "gte-modernbert-base/model.onnx");

    let out = embed_preset("gte-modernbert-base");
    assert_valid(&out, 768, "gte-modernbert-base");
}

#[test]
fn arctic_embed_m_v2_preset_embeds_768_dim_with_external_data() {
    if should_skip() {
        eprintln!("XBERG_SKIP_LIVE_HF=1, skipping");
        return;
    }
    let preset = xberg::get_embedding_preset("arctic-embed-m-v2.0").expect("preset must exist");
    assert_eq!(preset.model_repo, "xberg-io/embedding-models");
    assert_eq!(preset.model_file, "arctic-embed-m-v2.0/model.onnx");
    // Large fp32 export: weights live in an external-data sibling that must be
    // downloaded alongside the graph or ORT fails to build the session.
    assert_eq!(preset.additional_files, vec!["arctic-embed-m-v2.0/model.onnx.data".to_string()]);
    // Asymmetric model: queries get a "query: " prefix (applied by the RAG query path).
    assert_eq!(preset.query_prefix.as_deref(), Some("query: "));

    let out = embed_preset("arctic-embed-m-v2.0");
    assert_valid(&out, 768, "arctic-embed-m-v2.0");
}

#[test]
fn qwen3_embedding_0_6b_preset_embeds_1024_dim_last_token() {
    if should_skip() {
        eprintln!("XBERG_SKIP_LIVE_HF=1, skipping");
        return;
    }
    let preset = xberg::get_embedding_preset("qwen3-embedding-0.6b").expect("preset must exist");
    assert_eq!(preset.model_repo, "xberg-io/embedding-models");
    assert_eq!(preset.model_file, "qwen3-embedding-0.6b/model.onnx");
    assert_eq!(preset.pooling, "last");
    assert_eq!(
        preset.additional_files,
        vec!["qwen3-embedding-0.6b/model.onnx.data".to_string()]
    );

    let out = embed_preset("qwen3-embedding-0.6b");
    assert_valid(&out, 1024, "qwen3-embedding-0.6b");
}
