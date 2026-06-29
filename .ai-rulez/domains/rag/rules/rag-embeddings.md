---
priority: high
---

# RAG Embedding Rules

- `embedTexts(textsJson, configJson)` is the single embedding entry point from NAPI-RS
- Input: JSON array of strings; output: JSON array of `f32` arrays — never pass raw `Vec<f32>` across FFI
- Embedding model is selected by `EmbeddingPreset` in config; default preset must be a pure-Rust option
- ORT-dependent embedding models are gated behind the `embeddings` feature flag — check before loading
- Cache embedding results by `SHA256(text + modelId)` — identical text + model always returns the same vector
- Batch size for embedding calls: default 32 texts per inference call; configurable via `EmbeddingConfig`
- Dimension mismatch between stored vectors and query vector is a hard error — log and return `Err`
- Never run embedding inference on the main async executor thread — use `spawn_blocking`
- Normalize vectors to unit length before storage; cosine similarity is then equivalent to dot product
- If the `embeddings` feature is disabled, `embedTexts` must return a clear error, not a panic
