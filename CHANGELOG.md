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

- **Bring your own tokenizer for token-budgeted chunking.** Register a `TokenizerBackend`
  plugin (`register_tokenizer_backend`) — from Rust or any language binding — and reference
  it by name from `ChunkSizing::Tokenizer { model }`. The registry is checked before the
  HuggingFace path, so chunks are sized with the exact tokenizer the consumer's embedder
  uses (llama.cpp/GGUF vocabularies, SentencePiece models, custom vocabs). Existing
  HuggingFace-id configs behave unchanged.
- **Configurable embedding truncation length.** `EmbeddingConfig.max_sequence_length` sets how
  many tokens a chunk keeps before the tokenizer truncates it (default 512, always capped at the
  model's own `model_max_length`). Point it at a long-context model's window — e.g. 8192 for
  Jina/Nomic — so long chunks embed in full instead of only their first 512 tokens. It also
  participates in the embedding-engine cache key, so two configs that differ only in truncation
  length don't share a tokenizer.

### Changed

- **`OcrExtractionResult` now derives `Default`.** Downstream bindings and callers can
  construct and extend it without spelling out every field.

### Fixed

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
- **Vertical-text (tategaki) PDF pages return their text again.** pdf_oxide's reading-order
  sort panicked on pages whose vertical-mode spans sit closer together than the median span
  width — scanned pages with vertical OCR layers, typeset tategaki books. The panic guard kept
  extraction alive, but the affected page came back as a per-page error with its text lost.
  pdf_oxide 0.3.73 fixes the sort, so those pages now extract normally.
- **Redaction now scrubs every text-bearing field.** The redaction pass rewrote the main content
  and a handful of fields but left table cells, page content, form-field values, image captions,
  URIs, metadata, and structured output carrying the original text — while still reporting success.
  All of these are now redacted before the result is returned.
- **Encrypted PDFs honor the configured passwords.** `PdfConfig.passwords` had no effect, so a
  password-protected PDF came back as an empty success even with the right password supplied. Each
  configured password is now tried, and a still-locked document returns an error instead of empty text.
- **Merged table cells keep their column.** A cell following a horizontal merge (or under a vertical
  merge) shifted left into the spanning column in HTML, DOCX, and the document-structure grid,
  misaligning every following row against its headers. Cells now keep their true column position.
- **Text encoding is detected per document, not assumed UTF-8.** Latin-1 plain text and CSV no longer
  turn accented characters into replacement characters, XML honors its `encoding=` declaration, and a
  UTF-8 BOM is stripped from the first CSV header instead of being glued onto the field name.
- **Files are routed by content, not just their extension.** A misnamed file (e.g. a DOCX named
  `report.pdf`) is now detected from its bytes and sent to the correct extractor.
- **Token reduction applies to Markdown and HTML output.** The reduction was computed and then
  discarded for non-plain output formats; it now takes effect for the formatted content too.
- **Non-UTF-8 text inside archives is recovered.** Text members of zip/tar/7z archives whose bytes
  weren't valid UTF-8 were silently dropped; they are now decoded with the same detection used elsewhere.
- **OCR failures surface instead of returning empty text.** A failed or empty OCR pass no longer masks
  itself as a clean empty result, and an empty OCR result no longer wipes a page's native text; a
  `ProcessingWarning` is attached so callers can tell the page fell back.
- **Dense unruled tables are no longer dropped by the density guard.** A real reference table with many
  short-valued rows and few columns was rejected on row count alone; it is kept when its cells are
  short values, while columned prose is still rejected.
- **Language detection honors `min_confidence` and orders results deterministically.** The confidence
  threshold was silently capped, and equal-frequency languages came back in a nondeterministic order.
- **Config changes that alter output no longer serve a stale cached result.** The source name and OCR
  tessdata now participate in the cache key.
- **CSV `NaN`/`inf`/`infinity` are treated as text, not numbers**, so they no longer flip header and
  column-type detection.
- **Table diffs report shape changes.** A table whose row/column shape changed produced an
  information-free empty diff instead of showing the old table removed and the new one added.

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
