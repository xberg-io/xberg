# xberg-mcp-server

MCP server for document intelligence, RAG, and GDPR-compliant PII redaction. Wraps the [xberg](https://github.com/xberg-io/xberg) extraction engine and [xberg-rag](../crates/xberg-rag/) store over NAPI-RS bindings.

## Tools

| Category | Tool | Description |
|---|---|---|
| **Extraction** | `extract_document` | Extract text/tables/metadata from file URI or URL |
| | `extract_batch` | Extract from multiple files in parallel |
| | `list_formats` | List all supported MIME types |
| | `extract_entities` | GLiNER NER or LLM-based named entity extraction |
| | `structured_extract` | LLM JSON-schema structured extraction |
| **Media** | `transcribe_audio` | Whisper ONNX offline audio/video transcription |
| **Web** | `scrape_url` | HTTP/headless web scraping → Markdown |
| **Collections** | `create_collection` | Create a vector store collection |
| | `get_collection` | Get collection spec |
| | `drop_collection` | Delete a collection |
| **Ingest** | `ingest_document` | Chunk, embed, and store pre-extracted text |
| | `ingest_folder` | Extract → detect PII → redact → embed → store |
| **Query** | `query_corpus` | Search with vector / full_text / hybrid / graph mode |
| **Documents** | `upsert_document` | Insert or update a document record |
| | `get_document` | Retrieve document by ID |
| | `delete_documents` | Delete documents by ID list |
| | `delete_by_filter` | Delete documents matching a filter |
| **PII** | `detect_pii` | Pattern-based PII detection (11 categories) |
| | `redact_document` | Redact PII with token_replace / mask / hash |
| **Rehydration** | `rehydrate_tokens` | Restore PII from a token map |
| | `list_tokens` | List redaction tokens in text |
| | `rehydrate_document` | Decrypt a rehydration map file |
| **Reports** | `get_ingestion_summary` | Collection-level PII statistics |
| | `get_document_report` | Per-document PII detail |
| | `explain_reports` | Workflow and GDPR compliance guide |
| **Stats** | `collection_stats` | Document and chunk counts |
| | `list_collections` | List tracked collections |
| | `export_collection` | Export collection metadata |
| | `import_collection` | Import documents from JSON/JSONL file |
| | `update_metadata` | Update document metadata |
| | `get_audit_log` | Audit trail of PII detection operations |
| | `get_extraction_stats` | Extraction performance metrics |
| **Cache** | `rag_cache_warm` | Download ONNX models (BGE-M3, reranker, GLiNER) |
| | `rag_cache_status` | Check model cache status |

## Setup

### Prerequisites

- Node.js ≥ 18
- Built NAPI bindings: `cargo build --release -p xberg-node -p xberg-rag-node`

### Install

```sh
npm install
npm run build
```

### Claude Desktop config

```json
{
  "mcpServers": {
    "xberg": {
      "command": "node",
      "args": ["path/to/mcp-server/dist/index.js"],
      "env": {
        "XBERG_STORE_PATH": "/path/to/store.db",
        "XBERG_CACHE_DIR": "/path/to/model-cache",
        "HF_TOKEN": "hf_xxxxxxxxxxxxxxxxxxxx"
      }
    }
  }
}
```

`HF_TOKEN` is only needed for the **default** ONNX NER path (`extract_entities` with `backend: "onnx"`, or `ingest_folder` with `use_ner: true` and `ner_backend: "onnx"`), which pulls from the private `xberg-io/gliner-models` catalog. The model downloads once (~200 MB) and is then cached.

If you don't have access to that private catalog, point `hf_repo` / `hf_model_file` / `hf_tokenizer_file` (or `ner_hf_repo` / `ner_hf_model_file` / `ner_hf_tokenizer_file` on `ingest_folder`) at any public or private GLiNER ONNX export of your own instead — e.g. [`knowledgator/gliner-pii-base-v1.0`](https://huggingface.co/knowledgator/gliner-pii-base-v1.0):

```jsonc
{
  "backend": "onnx",
  "hf_repo": "knowledgator/gliner-pii-base-v1.0",
  "hf_model_file": "onnx/model_fp16.onnx",
  "hf_tokenizer_file": "tokenizer.json"
}
```

`HF_TOKEN` is still required if `hf_repo` is itself private; it's optional for public repos. Files downloaded from a custom `hf_repo` are not checksum-verified, unlike the pinned catalog.

GLiNER2 models (`fastino/gliner2` lineage) use a different ONNX tensor contract than GLiNER1 — set `hf_architecture: "gliner2"` (or `ner_hf_architecture` on `ingest_folder`) when pointing at one. Most GLiNER2 model cards on HuggingFace ship `safetensors` only with no ONNX export; check the repo's file list for a `.onnx` file before trying this. `lion-ai/gliner2-base-v1-onnx` is a confirmed-working monolithic single-file GLiNER2 ONNX export:

```jsonc
{
  "backend": "onnx",
  "hf_repo": "lion-ai/gliner2-base-v1-onnx",
  "hf_model_file": "model.onnx",
  "hf_tokenizer_file": "tokenizer.json",
  "hf_architecture": "gliner2"
}
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `XBERG_STORE_PATH` | platform cache dir | SQLite database path |
| `XBERG_CACHE_DIR` | platform cache dir | ONNX model cache directory |
| `HF_TOKEN` | — | HuggingFace read token for GLiNER model download ([get one](https://huggingface.co/settings/tokens)) |
| `XBERG_MCP_PORT` | `8080` | HTTP transport port |
| `XBERG_MCP_HOST` | `127.0.0.1` | HTTP transport host |

## GDPR Workflow

```
ingest_folder(source_folder, redacted_folder, rehydration_passphrase)
  ├── Extract text from each file (xberg)
  ├── Detect PII (11 regex categories)
  ├── Redact with [CATEGORY_N] tokens
  ├── Write redacted copy → redacted_folder/*_REDACTED.*
  ├── Write PII report → redacted_folder/*_REPORT.docx
  ├── Encrypt token map → redacted_folder/.rehydration/*.map (AES-256-GCM)
  └── Chunk + embed + store in RAG collection

rehydrate_document(document_id, passphrase)
  → token_map

rehydrate_tokens(redacted_text, token_map)
  → original text
```

Rehydration maps use AES-256-GCM with scrypt key derivation. Omitting `rehydration_passphrase` stores maps as plaintext (development only).

## Development

```sh
npm run dev          # Run with tsx (no build step)
npm test             # Run vitest tests
npm run build        # Compile TypeScript
```

Tests in `tests/` cover PII detection, redaction strategies, and encryption round-trips without requiring native bindings.
