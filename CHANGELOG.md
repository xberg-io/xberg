# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Xberg is the next iteration of [Kreuzberg](https://github.com/kreuzberg-dev/kreuzberg-v4-lts).
The changelog starts fresh at `1.0.0-rc.1`. For the Kreuzberg v1–v4 history, see the
[Kreuzberg v4 LTS changelog](https://github.com/kreuzberg-dev/kreuzberg-v4-lts/blob/main/CHANGELOG.md).

---

## [Unreleased]

### Added

- **EU/GDPR structured PII detection.** `ingest_folder`'s new opt-in
  `eu_patterns` flag scans for checksum-validated EU national IDs (FR INSEE,
  ES DNI/NIE, IT Codice Fiscale, PL PESEL, NL BSN, BE Registre National),
  FR SIRET/SIREN, EU VAT numbers, EU license plates, and GDPR Art. 9
  special-category keywords (health, biometric, genetic, political,
  religious, union, criminal, sexual orientation, ethnic origin), plus a
  k-anonymity risk report via `buildPiiReport()`. Default is unchanged
  (`eu_patterns: false`) for existing callers.
- **Durable rehydration-map storage.** `POST /v1/process` (with
  `operations.redact.rehydrate=true`) and `POST
  /v1/documents/{id}/rehydrate` now persist encrypted PII rehydration maps
  through a new `xberg-doc-store` crate. The default backend is unchanged
  (in-memory, 24h TTL, lost on restart); setting `XBERG_REHYDRATION_DB_PATH`
  and building with the `doc-store-sqlite` feature switches to a durable,
  WAL-mode SQLite backend that survives process restarts. No wire-format
  change to either endpoint.

### Changed

- **`OcrExtractionResult` now derives `Default`.** Downstream bindings and callers can
  construct and extend it without spelling out every field.
- **MCP server backend migrated to the shared WASM engine.** The MCP server's
  document, RAG, and PII tool groups (`extract_*`, `detect_pii`, `redact_document`,
  `rehydrate_*`, `ingest_*`, `query_corpus`, collection/document/stats/reports)
  now run on `@xberg-io/xberg-wasm` (via the `xberg-wasm-runtime` layer) instead
  of the native NAPI bindings (`@xberg-io/xberg`, `xberg-rag-node`). Tool names and
  input schemas are unchanged — no breaking change to the MCP protocol surface —
  so connected agents keep working. OCR is provided through the runtime layer's
  injected PaddleOCR (`ppu-paddle-ocr` over ONNX Runtime) with an in-binary
  Tesseract fallback.
- **`create_collection` default `embedding_dim` is now 384 (was 768).** The
  runtime's embedder is fixed at 384 dimensions (all-MiniLM); the previous 768
  default (a bge-base preset assumption) no longer matches, and a mismatched
  collection dimension fails at ingest time. Collections used with
  `ingest_document`/`query_corpus` must be 384-dim. The `embedding_dim` field is
  still accepted for explicit overrides.
- **`query_corpus` on the WASM store is vector-only for now.** The runtime's
  in-memory vector store services `vector` retrieval (and coerces `hybrid` to it);
  `full_text`/`graph` modes and result reranking are not yet available through the
  WASM store and return a clear error / are skipped rather than silently degrading.
  `filter`, `include_content`, and `include_document` are honored.
- **Some MCP tool groups remain on the native path.** `extract_entities` /
  `structured_extract` (LLM-backed NER and LLM structured extraction),
  `transcribe_audio` (Whisper ONNX transcription), and `scrape_url` (headless-browser
  crawling) have no equivalent in the current WASM engine and continue to use the
  native bindings; they require the native binding to be present at call time.

### Fixed

- **MCP PII rehydration now decrypts in-WASM.** `rehydrate_document` decrypts the
  AES-256-GCM rehydration-map container inside the WASM engine (byte-compatible
  with the maps written by the previous TypeScript path — `XPII\x01 | salt(16) |
  iv(12) | tag(16) | ciphertext`, scrypt N=2^14), removing the native crypto
  dependency from that path.
- **RAG retrieval `PrimaryScore` is serializable again.** The `PrimaryScore` enum's
  `Vector`/`FullText` variants were newtype-over-scalar under an internally-tagged
  `#[serde(tag = "kind")]` representation, which serde cannot (de)serialize — every
  cross-boundary retrieval (`query_corpus` over the WASM store) failed with
  `invalid type: map, expected f32`. They are now struct variants (`{ score }`),
  so results round-trip through `serde_wasm_bindgen` and `serde_json`.
- **PDF/OCR worker-stack overflow.** The deep per-page OCR extraction futures are now
  boxed (`Box::pin`) so their large state lives on the heap instead of inflating the
  worker-thread stack frame. Together with the stack the binding runtimes provision for
  the async path, this stops scanned / image-only PDFs from aborting the process with a
  stack overflow (SIGBUS) during OCR.
- **Tesseract image OCR no longer fails on an empty language list.** `OcrConfig { language: [] }`
  joined to an empty Tesseract language string, which the native backend tried to load as a
  language pack named `""` — surfacing as the confusing `Failed to download language pack ''`.
  An empty language now defaults to English consistently across every OCR backend, matching the
  documented `OcrConfig` default. PaddleOCR results also report English in their metadata instead
  of an empty language when none is configured.
- **WASM Tesseract backend builds again.** It still treated the OCR `language` config as a single
  string after it became a list, so the WebAssembly build stopped compiling. It now uses the
  primary language (the in-memory WASI Tesseract handles one language at a time, like the PaddleOCR
  and VLM backends) and warns when more than one is requested.

### Internal

- **MCP server shares one WASM engine with the browser UI.** The server constructs a
  single `XbergEngine` at startup from the runtime layer's injection descriptor
  (`{ embedder, store, ner?, ocr? }`), so extraction, embeddings, NER, OCR, PII
  redaction, and vector retrieval run through the same `.wasm` and JS runtime code
  paths as the browser build instead of separate native bindings.

---

## [1.0.0-rc.1] - 2026-06-26

Initial Xberg release candidate. Xberg continues the Kreuzberg document-intelligence
engine under a new name with a reset v1 version line. This is a full rebrand with no
back-compat aliases; the published `kreuzberg` packages remain frozen on the v4 LTS line.

### Changed

- **Rebranded Kreuzberg → Xberg.**
  - **Rust:** crate `kreuzberg` → `xberg` (and every `kreuzberg-*` workspace crate →
    `xberg-*`); the `kreuzberg::` namespace → `xberg::`; `KreuzbergError` → `XbergError`.
  - **CLI:** binary `kreuzberg` → `xberg`; config discovery `kreuzberg.{toml,yaml,json}` →
    `xberg.{toml,yaml,json}`; all `KREUZBERG_*` environment variables → `XBERG_*`; cache
    directory `.kreuzberg/` → `.xberg/`.
  - **FFI:** symbol prefix `kreuzberg_*` → `xberg_*`; header `kreuzberg.h` → `xberg.h`; lib
    `kreuzberg_ffi` → `xberg_ffi`.
  - **Package coordinates:** PyPI `xberg`, npm `@xberg-io/*`, RubyGems/Hex/pub.dev `xberg`,
    Maven `io.xberg`, NuGet `Xberg`, Packagist `xberg-io/xberg`, Homebrew `xberg`.
  - **Go:** module `github.com/xberg-io/xberg` with no `/vN` suffix (v1); the binding lives at
    `packages/go/`.
  - **Docs:** documentation now at `docs.xberg.io`.
- **ner-onnx:** vendored the stripped span-mode GLiNER runtime as `xberg-gliner`, replaced the
  ORP pipeline wrapper with direct `ort` session management, and moved runtime model downloads
  to the `xberg-io/gliner-models` artifact repository. The public `ner-onnx` feature and NER
  config shape are unchanged.

[1.0.0-rc.1]: https://github.com/xberg-io/xberg/releases/tag/v1.0.0-rc.1
