# Reranking

Reranking takes a query and a list of candidate documents, then scores them jointly to reorder by relevance. Unlike vector similarity, which independently embeds the query and documents, reranking models score the `(query, document)` pair together. This yields significantly better ranking quality at the cost of higher latency.

## Bi-encoders vs cross-encoders

Bi-encoders (the embedding models used for vector similarity) encode the query and each document independently, then compare via dot product or cosine similarity. They are fast and embarrassingly parallel — well suited to first-pass retrieval over millions of documents.

Cross-encoders feed `(query, document)` pairs through a transformer that scores them together. The query and document attend to each other across every layer, producing dramatically more accurate relevance scores. The trade-off is computational cost: every candidate document requires a separate forward pass.

## When to use it

Use reranking as the **second pass** in a retrieval pipeline:

1. **Retrieve** a candidate set (e.g. top-100) cheaply via vector similarity or BM25.
2. **Rerank** that set with a cross-encoder.
3. **Pass** the top-k reranked documents into your LLM context.

This pattern preserves the recall of vector search while sharpening the precision of what reaches the model.

## Backend variants

Four backend variants are supported, mirroring the embedding API:

| Variant | Source | Best for |
|---------|--------|----------|
| **Preset** | Bundled ONNX cross-encoders, downloaded from HuggingFace on first use | Production RAG, standard cross-encoder needs |
| **Custom** | Any ONNX cross-encoder from HuggingFace | Tuned models, niche domains |
| **Llm** | Provider-hosted rerankers (Cohere, Jina, Voyage) via `liter-llm` | Managed APIs, no local model |
| **Plugin** | Caller-supplied backend registered via `register_reranker_backend` | sentence-transformers, in-process tuned models |

The Preset and Custom variants require the `reranker` Cargo feature, which depends on ONNX Runtime. The Llm variant requires the `liter-llm` feature. Plugin works on every target including WASM.

## Presets

Four cross-encoder presets are bundled:

| Name | Model | Size | Languages | Max length |
|------|-------|------|-----------|------------|
| `fast` | `Xenova/ms-marco-MiniLM-L-6-v2` | 22M params (quantized) | English | 512 |
| `balanced` | `Xenova/bge-reranker-base` | 278M params | English, Chinese | 512 |
| `quality` | `Xenova/bge-reranker-large` | 560M params | English, Chinese | 512 |
| `multilingual` | `BAAI/bge-reranker-v2-m3` | 568M params | 100+ languages | 8192 |

Pick the smallest preset that meets your quality bar — larger models add latency.

## Related

- [Architecture](architecture.md) — Where reranking sits in the broader retrieval flow.
- [Reranking guide](../guides/reranking.md) — Code examples per language.
- [Plugin system](plugin-system.md) — Registering a custom reranker backend.
