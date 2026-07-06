# xberg Privacy API v1 — API Design Specification

**Date:** 2026-06-29
**Status:** Design — approved by product owner
**Author:** Claude (AI Design

## 1. Overview

### 1.1 Goal
Design a world-class, privacy-first document intelligence API that feels like one cohesive intelligence platform—not a feature buffet. The API is built on top of the existing xberg core, extending it with production-grade PII detection, anonymization with rehydration, and a unified search layer.

### 1.2 Core Philosophy: "Process Any Document → Get Intelligence"
Instead of exposing a long list of endpoints (`/extract`, `/transcribe`, `/ocr`, `/ner`), the API provides a single universal endpoint: **`POST /v1/process`**.
Users describe *what* they want (extract, redact, classify), and the pipeline auto-routes. This mirrors the design philosophy of world-class APIs like Stripe (unified resource model), OpenAI (single `/v1/chat/completions`), and Twilio (declarative messaging).

### 1.3 Key Principles
1. **Outcome-oriented, not feature-oriented:** Users describe the intelligence they need, not the algorithm.
2. **Async-first by default:** Large documents return a `task_id` immediately. Poll `GET /v1/tasks/{id}` for status.
3. **PII as a first-class citizen:** Redaction is not an afterthought. Every document can be redacted with configurable strategy.
4. **Composable pipelines:** Operations chain naturally (extract → redact → chunk → embed → store).
5. **Consistent API surface:** Every endpoint follows the same request/response pattern.

---

## 2. Architecture

### 2.1 High-Level Flow

```
User Request → POST /v1/process
    │
    ├─── Input Normalization (file, URL, text, base64)
    │
    ├─── Auto-Routing (MIME detection, format classification)
    │       ├─── PDF/Office/Images → Extraction Engine
    │       ├─── Audio/Video → Transcription (Whisper)
    │       └─── Text/JSON → Direct Pipeline
    │
    ├─── Pipeline Orchestration (user-defined `operations`)
    │       ├─── Extract
    │       ├─── Redact (PII anonymization)
    │       ├─── NER (Named Entity Recognition)
    │       ├─── Classify
    │       ├─── Chunk
    │       └─── Embed
    │
    └─── Response Assembly (structured, consistent envelope)
```

### 2.2 Deployment Modes

- **Managed Cloud (SMB):** `xberg-api` binary + load balancer. API keys via `X-API-Key` header.
- **Self-Hosted (Enterprise):** Same binary, licensed per-seat or per-CPU. Customer manages infrastructure.
- **Hybrid:** Managed cloud for development, self-hosted for production.

---

## 3. API Endpoints

### 3.1 Universal Document Processing

#### `POST /v1/process`
Process any document (PDF, DOCX, MP3, MP4, TXT, etc.) and return structured intelligence based on the requested operations.

**Request — Multipart Form:**
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `file` | binary | Yes* | The document to process |
| `url` | string | Yes* | URL of the document (instead of file) |
| `text` | string | Yes* | Raw text to process (instead of file) |
| `operations` | JSON | Yes | Declarative operations pipeline |

*One of `file`, `url`, or `text` is required.

**Request Body (JSON `operations` field):**
```json
{
  "extract": {
    "output_format": "markdown",
    "include_images": false,
    "ocr": { "backend": "tesseract", "languages": ["eng"] }
  },
  "transcribe": {
    "model": "tiny",
    "language": "en"
  },
  "redact": {
    "mode": "mask",
    "strategy": "server_encrypted",
    "categories": ["person", "organization", "email", "ssn", "phone"],
    "rehydrate": true
  },
  "ner": {
    "entities": ["person", "organization", "location", "date"],
    "threshold": 0.5,
    "model": "gliner2-multi-v1"
  },
  "classify": {
    "labels": ["legal", "medical", "financial", "technical"],
    "threshold": 0.3
  },
  "chunk": {
    "strategy": "semantic",
    "max_chars": 1000,
    "overlap": 200
  },
  "embed": {
    "model": "bge-base",
    "store_in_collection": "contracts"
  }
}
```

**Response (200 OK):**
```json
{
  "task_id": "t_abc123",
  "status": "completed",
  "document": {
    "id": "doc_abc123",
    "source": {
      "mime": "application/pdf",
      "size": 5242880,
      "original_filename": "contract.pdf"
    },
    "processing": {
      "mime_detected": "application/pdf",
      "format": "pdf",
      "pages": 5,
      "ocr_used": false,
      "transcribed": false
    },
    "extracted": {
      "text": "Snowflake Inc. signed a 5-year contract...",
      "markdown": "## Contract\n\n**Party A**: Snowflake Inc.\n**Party B**: Acme Corp...",
      "structured": {
        "tables": [...],
        "headings": [...],
        "metadata": { "author": "Legal Dept", "title": "Master Service Agreement" }
      }
    },
    "redacted": {
      "text": "[ORG] signed a 5-year contract...",
      "mode": "mask",
      "rehydration_key": "reh_e4f8a1b2c3d4...",
      "findings": [
        { "category": "organization", "original": "Snowflake Inc.", "replaced_with": "[ORG]", "confidence": 0.98 }
      ],
      "expires_at": "2026-07-06T12:00:00Z"
    },
    "entities": [
      { "type": "organization", "text": "Snowflake Inc.", "start": 0, "end": 14, "confidence": 0.98 },
      { "type": "organization", "text": "Acme Corp", "start": 45, "end": 54, "confidence": 0.95 },
      { "type": "date", "text": "January 1, 2025", "start": 78, "end": 91, "confidence": 0.99 }
    ],
    "classification": {
      "legal": 0.92,
      "financial": 0.83,
      "medical": 0.02,
      "technical": 0.15
    },
    "chunks": {
      "count": 12,
      "strategy": "semantic",
      "average_size": 850
    },
    "embeddings": {
      "model": "bge-base",
      "dimensions": 768,
      "stored_in_collection": "contracts"
    }
  }
}
```

**Response (202 Accepted — for large documents):**
```json
{
  "task_id": "t_def456",
  "status": "pending",
  "poll_url": "/v1/tasks/t_def456"
}
```

---

### 3.2 Task Management (Async Operations)

#### `GET /v1/tasks/{id}`
Get the status/results of an async operation.

**Response (200 OK):**
```json
{
  "task_id": "t_def456",
  "status": "running",
  "created_at": "2026-06-29T12:00:00Z",
  "updated_at": "2026-06-29T12:01:30Z",
  "progress": {
    "stage": "extracting",
    "percentage": 45
  },
  "result": null,
  "error": null
}
```

---

### 3.3 Search & RAG

#### `POST /v1/search`
Search across processed documents or RAG collections.

**Request:**
```json
{
  "query": "termination clauses with 30-day notice in SaaS contracts",
  "collections": ["contracts", "legal-templates"],
  "mode": "hybrid",
  "top_k": 10,
  "rerank": true,
  "filters": {
    "mime_type": "application/pdf",
    "classification.legal": { "gt": 0.8 }
  }
}
```

**Response:**
```json
{
  "results": [
    {
      "chunk_id": "c_xyz789",
      "chunk_content": "Either party may terminate...",
      "score": 0.9432,
      "score_breakdown": {
        "vector": 0.92,
        "full_text": 0.88,
        "rrf": 0.9432
      },
      "document": {
        "id": "doc_abc123",
        "title": "Master Service Agreement - Snowflake Inc.",
        "source_uri": "https://s3.../contract.pdf"
      }
    }
  ],
  "total_results": 42,
  "query_time_ms": 45
}
```

---

### 3.4 Standalone NLP Operations

#### `POST /v1/ner`
Named entity recognition on raw text.

**Request:**
```json
{
  "text": "Apple Inc. signed a deal with Microsoft...",
  "entities": ["person", "organization", "location"],
  "threshold": 0.5,
  "model": "gliner2-multi-v1"
}
```

**Response:**
```json
{
  "entities": [
    { "type": "organization", "text": "Apple Inc.", "start": 0, "end": 10, "confidence": 0.98 },
    { "type": "organization", "text": "Microsoft", "start": 33, "end": 42, "confidence": 0.95 }
  ],
  "model_used": "gliner2-multi-v1",
  "processing_time_ms": 120
}
```

#### `POST /v1/classify`
Text classification using the GLiNER2 `[L]` head.

**Request:**
```json
{
  "text": "The weather is terrible today",
  "labels": ["positive", "negative", "neutral"],
  "model": "gliner2-multi-v1"
}
```

**Response:**
```json
{
  "classifications": [
    { "label": "negative", "score": 0.92 },
    { "label": "neutral", "score": 0.06 },
    { "label": "positive", "score": 0.02 }
  ],
  "model_used": "gliner2-multi-v1",
  "processing_time_ms": 45
}
```

---

### 3.5 PII Rehydration

#### `POST /v1/documents/{id}/rehydrate`
Restore original PII from a redacted document. Requires authentication and audit logging.

**Request:**
```json
{
  "rehydration_key": "reh_e4f8a1b2c3d4...",
  "passphrase": "user_provided_secret",
  "scope": "all"
}
```

**Response:**
```json
{
  "document_id": "doc_abc123",
  "rehydrated_text": "Snowflake Inc. signed a 5-year contract...",
  "restorations": [
    { "category": "organization", "original": "Snowflake Inc.", "position": [0, 14] }
  ],
  "audit_log_id": "audit_789xyz"
}
```

---

### 3.6 RAG Collections

#### `POST /v1/collections`
Create a new RAG collection.

**Request:**
```json
{
  "name": "contracts",
  "embedding_model": "bge-base",
  "distance_metric": "cosine",
  "index_method": "hnsw"
}
```

#### `POST /v1/collections/{id}/ingest`
Ingest documents into a collection.

**Request:**
```json
{
  "document_ids": ["doc_abc123", "doc_def456"],
  "operations": {
    "chunk": { "strategy": "semantic", "max_chars": 1000 },
    "embed": true
  }
}
```

#### `POST /v1/collections/{id}/query`
Query a collection (same interface as `/v1/search` with collection autofilled).

---

## 4. Data Models

### 4.1 Document
The `document` object is the central resource, returned by `/v1/process`.

```typescript
interface Document {
  id: string;
  source: {
    mime: string;
    size: number;
    original_filename?: string;
  };
  processing: {
    mime_detected: string;
    format: string;
    pages?: number;
    duration_ms?: number;
    ocr_used: boolean;
    transcribed: boolean;
  };
  extracted?: {
    text: string;
    markdown?: string;
    structured?: {
      tables: any[];
      headings: any[];
      metadata: Record<string, any>;
    };
  };
  redacted?: {
    text: string;
    mode: "mask" | "token_replace" | "hash";
    rehydration_key?: string;
    findings: RedactionFinding[];
    expires_at?: string;
  };
  entities?: NEREntity[];
  classification?: Record<string, number>;
  chunks?: {
    count: number;
    strategy: string;
    average_size: number;
  };
  embeddings?: {
    model: string;
    dimensions: number;
    stored_in_collection?: string;
  };
}
```

### 4.2 RedactionFinding
```typescript
interface RedactionFinding {
  category: string;
  original: string;
  replaced_with: string;
  confidence: number;
  position: [number, number]; // start, end in text
}
```

### 4.3 NEREntity
```typescript
interface NEREntity {
  type: string;
  text: string;
  start: number;
  end: number;
  confidence: number;
}
```

---

## 5. PII Rehydration Strategies

### 5.1 Server-Encrypted (Default for SMBs)
- Encrypted rehydration map stored server-side.
- AES-256-GCM encryption with scrypt key derivation.
- Key managed by xberg (automatic rotation).
- Simplest UX: zero key management for customers.

### 5.2 Customer-Provided Key (Enterprise)
- Customer provides encryption key at upload time.
- xberg never sees the key.
- Rehydration requires the same key.
- Best for strict compliance requirements.

### 5.3 Audit-Logged (Regulated Industries)
- All rehydration requests logged with full audit trail.
- Requires explicit approval from data owner.
- Automatic expiry of stored originals (configurable, default 90 days).
- Suitable for legal, healthcare, finance.

---

## 6. Error Handling

All errors follow a consistent structure:

```json
{
  "error": {
    "type": "ValidationError",
    "message": "Invalid file format: .xyz is not supported",
    "status_code": 400,
    "request_id": "req_abc123",
    "documentation_url": "https://docs.xberg.io/errors/validation-error"
  }
}
```

| Status | Error Type | Description |
|--------|-----------|-------------|
| 400 | `ValidationError` | Bad request (invalid JSON, missing fields) |
| 413 | `PayloadTooLarge` | File exceeds size limit |
| 422 | `UnsupportedFormat` | File format not supported |
| 429 | `RateLimitExceeded` | Too many requests |
| 500 | `InternalServerError` | Server error |

---

## 7. Technology Stack

| Layer | Technology |
|-------|-----------|
| API Server | Axum (already in xberg) |
| NER/Classification | `xberg-gliner` upgraded with `anno::gliner2_fastino` (8-session ONNX, IoBinding) |
| OCR | Tesseract (default), PaddleOCR, Candle |
| Transcription | Whisper (ONNX Runtime) |
| Embeddings | BGE presets (ONNX) |
| Reranking | Cross-encoder (ONNX) |
| Vector Store | sqlite-vec (default), graphqlite (optional) |
| Full-Text Search | SQLite FTS5 |
| Async Jobs | In-memory JobStore (already in xberg) |
| Serialization | JSON (default), TOON (optional) |

---

## 8. Implementation Phases

### Phase 1: GLiNER2 Upgrade for xberg-gliner
1. Port `anno::gliner2_fastino` (11 files, 8-session ONNX pipeline) into `xberg-gliner`.
2. Replace the basic `xberg-gliner` (8 small files) with the production-grade implementation.
3. Add IoBinding support (1.5-3× faster on CPU, required for GPU).
4. Wire structure extraction (`TaskSchema`) for document understanding.

### Phase 2: PII Pipeline as First-Class Feature
1. Integrate PII detection from `anno::pii` (11 GDPR categories).
2. Add PII redaction to `/v1/process` pipeline.
3. Implement rehydration with all three strategies (server-encrypted, customer-key, audit-logged).
4. Add `/v1/documents/{id}/rehydrate` endpoint.

### Phase 3: Unified `/v1/process` Endpoint
1. Extend the existing `POST /extract` into `POST /v1/process`.
2. Add `operations` declarative pipeline.
3. Implement auto-routing (PDF → extraction, MP3 → transcription).
4. Add support for all operations: extract, transcribe, redact, ner, classify, chunk, embed.

### Phase 4: Search & RAG
1. Add `/v1/search` endpoint (vector + full-text + hybrid + graph).
2. Add `/v1/collections` CRUD endpoints.
3. Integrate reranking into search pipeline.
4. Add filters and faceted search.

### Phase 5: Enterprise Features
1. Multi-tenancy (collection-level isolation).
2. Webhooks for async jobs.
3. Detailed audit logging.
4. API key management (out of scope for core, implement in gateway layer).

---

## 9. Open Questions

1. **Feature flags:** Which features should be gated behind `api` feature flag vs. always available?
2. **Async storage:** Currently uses in-memory `JobStore`. Should we add Redis/SQLite persistence for async jobs in production?
3. **Rate limiting:** Should rate limiting be in the core API or a gateway layer?
4. **Multi-tenancy:** Is collection-level isolation sufficient, or do we need full tenant isolation?
5. **Webhooks:** Should the core API support webhooks, or should that be handled by a separate service?

---

## 10. References

- **Existing xberg API:** `crates/xberg/src/api/` (handlers.rs, types.rs, router.rs, openapi.rs)
- **Existing features:** OCR, NER, redaction, chunking, embeddings, reranking, transcription
- **GLiNER2 implementation (anno):** `crates/anno/src/backends/gliner2_fastino/`
- **PII implementation (anno):** `crates/anno/src/pii.rs`
- **RAG implementation (xberg):** `crates/xberg-rag/src/`
