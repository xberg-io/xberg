//! Regression test for hardcoded 512-token embedding truncation (issue #1223).
//!
//! Before the fix, `get_or_init_engine` always configured the tokenizer with a
//! 512-token truncation limit, so a chunk longer than 512 tokens embedded only
//! its 512-token prefix while the full chunk text was stored. `EmbeddingConfig`
//! now carries `max_sequence_length: Option<usize>`, threaded into the tokenizer
//! (still capped at the model's own `model_max_length`).
//!
//! This test needs a real long-context ONNX model (Jina v2 small, 8192-token
//! context, 512-dim) and ONNX Runtime, so it is `#[ignore]`d by default. Run it
//! explicitly with a downloadable model + working ORT:
//!
//!     cargo test -p xberg --features "embeddings" \
//!         --test embedding_max_sequence_length -- --ignored --nocapture
//!
//! With the 512-hardcode still in place both configs would produce identical
//! vectors; with the fix, raising `max_sequence_length` past 512 lets the tail of
//! a long chunk contribute, so the vectors diverge.

#![cfg(feature = "embeddings")]

use xberg::core::config::{EmbeddingConfig, EmbeddingModelType};

/// Jina v2 small: 8192-token context (model_max_length), 512-dim, mean pooling,
/// ONNX at `onnx/model.onnx` — matches the Custom-model default path.
const MODEL_ID: &str = "Xenova/jina-embeddings-v2-small-en";
const DIMS: usize = 512;

fn custom_config(max_seq: Option<usize>) -> EmbeddingConfig {
    EmbeddingConfig {
        model: EmbeddingModelType::Custom {
            model_id: MODEL_ID.to_string(),
            dimensions: DIMS,
        },
        normalize: true,
        batch_size: 4,
        max_sequence_length: max_seq,
        ..Default::default()
    }
}

/// Build a body of roughly `word_count` whitespace-separated distinct tokens.
fn long_body(word_count: usize) -> String {
    (0..word_count)
        .map(|i| format!("alpha{i}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (na * nb)
}

#[test]
#[ignore = "downloads a ~130MB ONNX model and requires ONNX Runtime; run with --ignored"]
fn test_raising_max_sequence_length_lets_the_tail_contribute() {
    // A shared >512-token prefix, plus a distinctive tail that only survives when
    // truncation is raised above 512 tokens.
    let prefix = long_body(560); // ~560 tokens, already past the 512 default
    let tail = long_body(400).replace("alpha", "omega"); // distinct vocabulary in the tail
    let full_text = format!("{prefix} {tail}");

    let embed = |cfg: &EmbeddingConfig, text: &str| -> Vec<f32> {
        let out = xberg::embed_texts(vec![text.to_string()], cfg)
            .expect("embedding should succeed (model download + ORT required)");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), DIMS, "unexpected embedding dimension");
        out[0].clone()
    };

    let cfg_512 = custom_config(Some(512));
    let cfg_1024 = custom_config(Some(1024));

    // (1) Prefix-only invariant at 512: appending a tail beyond token 512 must NOT
    //     change the embedding when truncation stays at 512.
    let prefix_only_512 = embed(&cfg_512, &prefix);
    let full_512 = embed(&cfg_512, &full_text);
    let sim_prefix_vs_full_at_512 = cosine(&prefix_only_512, &full_512);
    eprintln!("cosine(prefix, full) @512 = {sim_prefix_vs_full_at_512:.6}");
    assert!(
        sim_prefix_vs_full_at_512 > 0.999,
        "at max_sequence_length=512 the tail past token 512 must be truncated, so the full \
         text should embed like its prefix (cosine {sim_prefix_vs_full_at_512:.6})"
    );

    // (2) Raising to 1024 lets the tail contribute: the full-text embedding must
    //     diverge from the 512-truncated one.
    let full_1024 = embed(&cfg_1024, &full_text);
    let sim_512_vs_1024 = cosine(&full_512, &full_1024);
    eprintln!("cosine(full@512, full@1024) = {sim_512_vs_1024:.6}");
    assert!(
        sim_512_vs_1024 < 0.999,
        "raising max_sequence_length to 1024 must let the tail change the embedding \
         (cosine {sim_512_vs_1024:.6} should be < 0.999)"
    );
}
