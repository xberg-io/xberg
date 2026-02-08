# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [4.2.15] - 2026-02-08

### Added

#### Agent Skill for AI Coding Assistants

- **Agent Skill for document extraction**: Added `skills/kreuzberg/SKILL.md` following the [Agent Skills](https://agentskills.io) open standard, with comprehensive instructions for Python, Node.js, Rust, and CLI usage. Includes 8 detailed reference files covering API signatures, configuration, supported formats, plugins, and all language bindings. Works with Claude Code, Codex, Gemini CLI, Cursor, VS Code, Amp, Goose, Roo Code, and any compatible tool.

#### MIME Type Mappings
- Added `.docbook` (`application/docbook+xml`) and `.jats` (`application/x-jats+xml`) file extension mappings.

### Added

#### OCR
- **PaddleOCR backend via ONNX Runtime**: Added a new OCR backend (`kreuzberg-paddle-ocr`) using PaddlePaddle's PP-OCRv4 models converted to ONNX format, run via ONNX Runtime. Supports 6 languages (English, Chinese, Japanese, Korean, German, French) with automatic model downloading and caching. Provides superior CJK recognition compared to Tesseract.
- **Unified OCR element output architecture**: Extraction results now include structured `OcrElement` data with bounding geometry (rectangles and quadrilaterals), per-element confidence scores, rotation information, and hierarchical levels (word, line, block, page). Available from both PaddleOCR and Tesseract backends.
- **PaddleOCR support in all bindings**: PaddleOCR is available across Python, Rust, TypeScript/Node.js, Go, Java, PHP, Ruby, C#, and Elixir bindings via the `paddle-ocr` feature flag.
- **PaddleOCR CLI support**: The `kreuzberg-cli` binary supports `--ocr-backend paddle-ocr` for PaddleOCR extraction.
- **Shared ORT discovery**: Added `ort_discovery` module for finding ONNX Runtime shared libraries across platforms, shared between PaddleOCR and future ONNX-based backends.
- **PaddleOCR model setup GitHub Action**: Added `.github/actions/setup-paddle-ocr-models/` action for CI pipelines to download and cache PaddleOCR model files.

#### CI
- **PaddleOCR CI integration**: Added PaddleOCR to the CI/publish pipelines with dedicated test jobs and model caching.

#### musl Linux Support
- **Re-enabled musl targets**: Added `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` targets for CLI binaries, Python wheels (musllinux), and Node.js native bindings. Resolves glibc 2.38+ requirement for prebuilt CLI binaries on older distros like Ubuntu 22.04 (#364).
- **musl CI workflows**: Added dedicated `ci-musl.yaml` workflow for CLI musl build validation with Alpine container smoke tests, and musllinux Python wheel builds to `ci-python.yaml`.
- **PDFium musl awareness**: Build script now downloads musl-specific PDFium binaries and uses correct C++ standard library (`libc++`) for musl targets.

### Removed

#### Node.js Bindings
- **Guten OCR references**: Removed all references to the unused Guten OCR backend. Renamed `KREUZBERG_DEBUG_GUTEN` env var to `KREUZBERG_DEBUG_OCR`.

#### PHP Bindings
- **Guten OCR backend option**: Removed `'guten'` from the documented backend choices in `OcrConfig`.

### Fixed

#### ODT List and Section Extraction
- Fixed ODT extractor not handling `text:list` and `text:section` elements. Documents containing bulleted/numbered lists or sections returned empty content.

#### UTF-16 EML Parsing
- Fixed EML files encoded in UTF-16 (LE/BE, with or without BOM) returning empty content. Detects UTF-16 encoding via BOM markers and heuristic byte-pattern analysis, transcoding to UTF-8 before parsing.

#### Email Attachment Metadata Serialization
- Fixed email extraction inserting a comma-joined string `"attachments"` into the `additional` metadata HashMap, which via `#[serde(flatten)]` overwrote the structured `EmailMetadata.attachments` array. This caused deserialization failures in Go, C#, and other typed bindings when processing emails with attachments.

#### WASM Office Document Support (DOCX, PPTX, ODT)
- DOCX, PPTX, and ODT extractors were gated on `#[cfg(all(feature = "tokio-runtime", feature = "office"))]` but `wasm-target` does not enable `tokio-runtime`. Changed cfg gates to `#[cfg(feature = "office")]` with conditional `spawn_blocking` only when `tokio-runtime` is available. Office documents now extract correctly in WASM builds.

#### WASM PDF Support in Non-Browser Runtimes
- PDFium initialization was guarded by `isBrowser()`, preventing PDF extraction in Node.js, Bun, and Deno. Removed the browser-only restriction so PDFium auto-initializes in all WASM runtimes.

#### Elixir PageBoundary JSON Serialization
- Added missing `@derive Jason.Encoder` to `PageBoundary`, `PageInfo`, and `PageStructure` structs in the Elixir bindings. Without this, encoding page structure metadata to JSON would fail with a protocol error.

#### Pre-built CLI Binary Missing MCP Command
- Pre-built standalone CLI binaries were built without the `mcp` feature flag, causing the `kreuzberg mcp` command to be unavailable. The build script now enables all features (`--features all`) to match the Python, Node, and Homebrew builds. Fixes #369.

#### PDF Error Handling Regression
- Reverted incorrect change from v4.2.14 that silently returned empty results for corrupted/malformed PDFs instead of propagating errors. Corrupted PDFs now correctly return `PdfError::InvalidPdf` and password-protected PDFs return `PdfError::PasswordRequired` as expected.

#### PaddleOCR Model URLs
- Fixed incorrect detection model filename in Docker and CI action (`en_PP-OCRv4_det_infer.onnx` → `ch_PP-OCRv4_det_infer.onnx`).

#### Python Bindings
- Fixed `OcrConfig` constructor silently ignoring `paddle_ocr_config` and `element_config` keyword arguments.

### Changed

#### ONNX Runtime
- Bumped ONNX Runtime from 1.23.2 to 1.24.1 across CI, Docker images, and documentation. Minimum supported ORT version is 1.23+.

#### API Parity
- Added `security_limits` field to all 9 language bindings (TypeScript, Go, Python, Ruby, PHP, Java, C#, WASM, Elixir) for API parity with Rust core `ExtractionConfig`.

---

## [4.2.14] - 2026-02-07

### Fixed

#### Excel File-Path Extraction
- Fixed `.xla` (legacy add-in) and `.xlsb` (binary spreadsheet) graceful fallback only applied to byte-based extraction; file-path-based extraction still propagated parse errors.

#### PDF Test Flakiness
- Fixed flaky PDF tests caused by concurrent pdfium access during parallel test execution. Added `#[serial]` to all pdfium-using tests to prevent global state conflicts.

#### Benchmark Fixtures
- Replaced auto-generated fixture discovery (`generate.rs`) with curated, validated fixture set.
- Added comprehensive fixture validation test suite (8 tests: JSON parsing, document existence, file sizes, ground truth, duplicate detection, format coverage).
- Removed 5 duplicate fixture entries pointing to the same test documents.
- Swapped encrypted EPUB fixture (`epub2_no_cover.epub` with IDPF font encryption) for clean `features.epub`.
- Fixed 272 stale `file_size` declarations in fixture JSON files to match actual files on disk.
- Fixed `validate_ground_truth.py` only checking root-level fixtures; now uses `rglob` for recursive validation.

### Removed
- Removed `generate.rs` auto-generation system from benchmark harness (caused recurring breakage from malformed vendored files).

---

## [4.2.13] - 2026-02-07

### Added

#### WASM Office Format Support
- Added office document extraction to the WASM target: DOCX, PPTX, RTF, reStructuredText, Org-mode, FictionBook, Typst, BibTeX, and Markdown are now available in the browser/WASM build.
- Added WASM integration tests for all new office formats (`office_extraction.rs`).
- Added e2e fixture definitions for RTF, RST, Org, FB2, Typst, BibTeX, and Markdown formats.
- Regenerated e2e test suites across all language bindings to include new office format fixtures.

#### Citation Extraction
- Added structured citation extraction for RIS (`.ris`), PubMed/MEDLINE (`.nbib`), and EndNote XML (`.enw`) formats via `biblib` crate with rich metadata including authors, DOI, year, keywords, and abstract.
- Added `CitationExtractor` with priority 60 for `application/x-research-info-systems`, `application/x-pubmed`, and `application/x-endnote+xml` MIME types.

#### JPEG 2000 OCR Support
- Added full JPEG 2000 image decoding for OCR via `hayro-jpeg2000` (pure Rust, memory-safe decoder). JP2 container and J2K codestream images are now decoded to RGB pixels for Tesseract OCR processing.
- Added pure Rust JP2 metadata parsing (dimensions, format detection) without external dependencies.

#### JBIG2 Image Support
- Added JBIG2 bi-level image decoding for OCR via `hayro-jbig2` (pure Rust, memory-safe decoder). JBIG2 is commonly used in scanned PDF documents.
- Added `image/x-jbig2` MIME type with `.jbig2` and `.jb2` file extension mappings.

#### Gzip Archive Extraction
- Added `GzipExtractor` for extracting text content from gzip-compressed files (`.gz`) via `flate2`, with decompression size limits to prevent gzip bomb attacks.

#### Extractor Registration
- Registered `JatsExtractor` and `DocbookExtractor` in the default extractor registry (extractors existed but were never registered).

#### MIME Type & Extension Mappings
- Added missing MIME types to `SUPPORTED_MIME_TYPES`: `text/x-fictionbook`, `application/x-fictionbook`, `text/x-bibtex`, `text/docbook`, `application/x-pubmed`.
- Added MIME type aliases for broader compatibility: `text/djot`, `text/jats`, `application/x-epub+zip`, `application/vnd.epub+zip`, `text/rtf`, `text/prs.fallenstein.rst`, `text/x-tex`, `text/org`, `application/x-org`, `application/xhtml+xml`, `text/x-typst`, `image/jpg`.
- Added missing file extension mappings: `.fb2`, `.opml`, `.dbk`, `.j2k`, `.j2c`, `.ris`, `.nbib`, `.enw`, `.typ`, `.djot`.

#### Security
- Wired `SecurityLimits` into the archive extraction pipeline: ZIP, TAR, 7z, and GZIP extractors now enforce configurable limits for max archive size, file count, compression ratio, and content size.
- Added `security_limits` field to `ExtractionConfig` for user-configurable archive security thresholds.
- ZIP archives are now validated with `ZipBombValidator` before extraction.
- Replaced hardcoded 256 MB gzip decompression limit with configurable `max_archive_size` (default 500 MB).

### Fixed

#### WASM Build
- Fixed `zstd-sys` build failure for `wasm32-unknown-unknown` by disabling default features on the `zip` crate and using `deflate-flate2` (pure Rust) instead of `zstd` (C code incompatible with WASM).
- Fixed `tokio`/`mio` compilation failure on WASM by removing `tokio-runtime` from the `office` feature (only needed for LibreOffice subprocess conversion, not in-memory parsers).
- Gated LibreOffice conversion paths (`libreoffice.rs`, legacy DOC/PPT handlers) behind `not(target_arch = "wasm32")` to prevent WASM builds from pulling in tokio filesystem and process APIs.

#### MIME Type Detection
- Fixed `.typ` files not recognized as Typst format; added `.typ` as an alias for `application/x-typst`.
- Fixed `.djot` files not recognized; added `.djot` extension mapping to `text/x-djot`.
- Fixed `application/gzip` rejected by MIME validation; added to `SUPPORTED_MIME_TYPES`.
- Fixed case-sensitive MIME type validation rejecting valid types with different casing (e.g., `macroEnabled` vs `macroenabled`); added RFC 2045 case-insensitive fallback.
- Synced `SUPPORTED_MIME_TYPES` with extractor registry to prevent valid formats being rejected before reaching their extractor.

#### Image Extraction
- Fixed JPEG 2000 images (`.jp2`) not handled by ImageExtractor; added `image/jp2`, `image/jpx`, `image/jpm`, and `image/mj2` to supported types.

#### Extraction
- Fixed YAML files rejected with "Unsupported format: application/yaml"; now accepts all four YAML MIME type variants including the standard `application/yaml` (RFC 9512).

#### CLI
- Fixed `.yml` config files rejected by `--config` flag; now accepts both `.yml` and `.yaml`.

#### .tgz Archive Extraction
- Fixed `.tgz` files parsed as raw TAR instead of gzip-compressed TAR. The MIME mapping now correctly routes `.tgz` to the GzipExtractor, which detects inner TAR archives via ustar magic bytes and delegates to TAR extraction.

#### Excel Exotic Formats
- Fixed `.xlam` (Excel add-in), `.xla` (legacy add-in), and `.xlsb` (binary spreadsheet) files causing extraction errors when they lack standard workbook data. These formats now gracefully return an empty workbook instead of propagating parse errors.

#### PDF Error Handling
- Fixed password-protected and malformed PDFs causing extraction errors. The PDF extractor now gracefully returns an empty `ExtractionResult` instead of propagating `PdfError::PasswordRequired` and `PdfError::InvalidPdf`.

#### Benchmark Harness
- Fixed framework initialization check running before external adapters (Tika, pdfplumber, etc.) were registered, causing false "failed to initialize" errors.
- Fixed missing `composer install` step in PHP benchmark CI job.
- Fixed C# benchmark wrapper using wrong MIME type casing for macro-enabled Office formats and incorrect djot MIME type.
- Fixed WASM benchmark wrapper missing MIME mappings for several supported formats.
- Added error counts (`framework_errors`, `harness_errors`) and error detail breakdown to benchmark aggregation output.
- Added distinct `ErrorKind::Timeout` tracking in benchmark results, propagated through aggregation and per-extension stats.
- Removed 12 malformed/password-protected/broken fixture files from benchmark corpus.

---

## [4.2.12] - 2026-02-06

### Fixed

#### DOCX Extraction
- Fixed DOCX list items missing whitespace between text runs, causing words to merge together. (#359)

---

## [4.2.11] - 2026-02-06

### Fixed

#### Python Bindings
- Fixed CLI binary missing from all platform wheels in the publish workflow. (#349)

### Fixed

#### OCR Heuristic

- **Pass actual page count to OCR fallback evaluator**: `evaluate_native_text_for_ocr` was called with `None` for page count, defaulting to 1. This inflated per-page averages for multi-page documents, causing scanned PDFs to skip OCR.
- **Per-page OCR evaluation for mixed-content PDFs**: Added `evaluate_per_page_ocr` which evaluates each page independently using page boundaries. If any single page triggers OCR fallback, the entire document is OCR'd. Previously, good pages masked scanned pages in the aggregate evaluation.

---

## [4.2.10] - 2026-02-05

### Fixed

#### MIME Type Detection
- Fixed DOCX/XLSX/PPTX files incorrectly detected as `application/zip` when using bytes-based MIME detection. (#350)

#### Java Bindings
- Fixed format-specific metadata (e.g., `sheet_count`, `sheet_names`) missing from `getMetadataMap()`.
- Fixed `ClassCastException` when deserializing nested generic collections in model classes. (#355)

#### Python Bindings
- Fixed Windows CLI binary still missing from wheel due to wrong filename in CI copy step. (#349)

---

## [4.2.9] - 2026-02-03

### Fixed

#### MCP Server
- Fixed "Cannot start a runtime from within a runtime" panic when using MCP server in Docker.
- Removed unused `async` parameter from MCP tools.

#### Python Bindings
- Fixed "embedded binary not found" error on Windows due to missing `.exe` extension handling. (#349)

#### OCR Heuristic
- Fixed OCR fallback evaluator receiving `None` for page count, causing scanned PDFs to incorrectly skip OCR.
- Added per-page OCR evaluation so that mixed-content PDFs with some scanned pages are properly OCR'd.

---

## [4.2.8] - 2026-02-02

### Fixed

#### Python Bindings
- Fixed `ChunkingConfig` serialization outputting wrong field names (`max_characters`/`overlap` instead of `max_chars`/`max_overlap`).

#### Java Bindings
- Fixed ARM64 SIGBUS crash in `kreuzberg_get_error_details` by returning a heap-allocated pointer instead of struct-by-value.

#### Ruby Bindings
- Fixed `rb_sys` missing as runtime dependency, causing `LoadError` during native extension compilation.

#### FFI
- Added `kreuzberg_free_error_details()` to properly free heap-allocated `CErrorDetails` structs.

---

## [4.2.7] - 2026-02-01

### Added

#### API
- Added OpenAPI schema for `/extract` endpoint with full type documentation.
- Added unified `ChunkingConfig` with canonical field names and serde aliases for backwards compatibility.

#### OCR
- Added `KREUZBERG_OCR_LANGUAGE="all"` support to auto-detect and use all installed Tesseract languages. (#344)

### Fixed

#### Ruby Bindings
- Fixed `Cow<'static, str>` type conversions in Magnus bindings.
- Fixed missing `bytes` workspace dependency in vendor Cargo.toml.

#### Python Bindings
- Fixed runtime `ExtractedImage` import; defined as Python-level runtime types instead of importing from compiled Rust bindings.

#### C# Bindings
- Fixed `Attributes` deserialization on ARM64 to handle both array-of-arrays and object JSON formats.

#### Java Bindings
- Fixed test timeouts causing CI hangs by adding `@Timeout(60)` to concurrency and async tests.

#### Elixir Bindings
- Overhauled all struct types to match Rust source: fixed `Metadata`, `Table`, `Image`, `Chunk`, `Page`, `ExtractionResult` field names and types.
- Added new struct modules matching Rust types: `ChunkMetadata`, `Keyword`, `PageHierarchy`, `DjotContent`, `PageStructure`, `ErrorMetadata`, `ImagePreprocessingMetadata`, and more.

#### TypeScript Bindings
- Overhauled type definitions to match NAPI-RS Rust source; fixed `ChunkingConfig`, `ExtractionResult`, `ExtractionConfig`, and `FormattedBlock` fields.

#### PHP Bindings
- Overhauled type definitions to match Rust source; fixed `Keyword`, `Metadata`, `ExtractionResult`, and `FormattedBlock` fields.

#### Ruby Bindings
- Overhauled RBS type stubs to match Ruby source and Rust Magnus bindings.

#### Python Bindings
- Overhauled `_internal_bindings.pyi` type stubs to match Rust source; fixed `Chunk`, `PptxMetadata`, `PdfMetadata`, `HtmlMetadata`, and optionality on multiple fields.
- Removed duplicate `types.py` containing 43 conflicting type definitions.

#### Java Bindings
- Overhauled type definitions to match Rust source; fixed `Metadata`, `PptxMetadata`, `PageInfo`, `ImageMetadata`, `LinkMetadata`, and added missing enums and types.

#### C# Bindings
- Overhauled type definitions to match Rust source; fixed `Metadata`, `PptxMetadata`, `PageBoundary`, `ImageMetadata`, and added missing types.
- Fixed keyword deserialization to discriminate between simple string keywords and extracted keyword objects.

#### Go Bindings
- Overhauled type definitions to match Rust source; fixed `Metadata`, `PptxMetadata`, `ImageMetadata`, `PageBoundary`, `PageInfo`, and added missing enums and types.

### Changed

- Bumped `html-to-markdown-rs` from 2.24.1 to 2.24.3.

### Performance

- Converted static string fields to `Cow<'static, str>` to eliminate heap allocations for string literals.
- Reduced allocations in RST parser, fictionbook extractor, and email extractor.
- Replaced `HashMap` with `Vec` for small metadata maps and `AHashMap` for hot-path maps.
- Switched `Metadata.additional` keys to `Cow<'static, str>` for interning.
- Replaced `Vec<u8>` with `bytes::Bytes` for `ExtractedImage.data`, enabling zero-copy cloning.

---

## [4.2.6] - 2026-01-31

### Fixed

#### Python Bindings
- Fixed missing `output_format`/`result_format` fields on `ExtractionResult`.
- Fixed missing `elements` and `djot_content` fields on `ExtractionResult`.
- Fixed chunks returned as dicts instead of objects; created proper `PyChunk` class with attribute access.

---

## [4.2.5] - 2026-01-30

### Fixed

#### Python Bindings
- Fixed missing `OutputFormat`/`ResultFormat` exports causing `ImportError`.
- Fixed `.pyi` stub alignment for `ExtractionResult`, `Element`, and related types.
- Fixed Python 3.10 compatibility for `StrEnum` (native `StrEnum` is 3.11+).

#### PHP Bindings
- Fixed config alignment with Rust core for `ImageExtractionConfig`, `PdfConfig`, `ImagePreprocessingConfig`, and `ExtractionConfig`.
- Removed phantom parameters not present in Rust core.

#### TypeScript/Node Bindings
- Fixed missing `elements` field; added `JsElement`, `JsElementMetadata`, `JsBoundingBox` to NAPI-RS bindings.

#### C# Bindings
- Fixed enum serialization using `JsonStringEnumMemberName` for .NET 9+.

#### Elixir Bindings
- Fixed test failures and cleaned up warnings on Windows.

#### Node Bindings
- Added Bun runtime support.

### Changed

#### All Bindings
- Achieved `PageContent` field parity across all language bindings.

---

## [4.2.4] - 2026-01-29

### Fixed

#### TypeScript/Node Bindings
- Fixed missing `elements` field; added `Element`, `ElementType`, `BoundingBox`, and `ElementMetadata` types.

#### Rust Core
- Fixed `KeywordConfig` deserialization failing on partial configs by adding `#[serde(default)]`.

#### C# Bindings
- Fixed `Element` serialization for `element_based` result format deserialization.

#### Elixir Bindings
- Derived `Jason.Encoder` for `ExtractionConfig` struct.

---

## [4.2.3] - 2026-01-28

### Fixed

#### API
- Fixed JSON array rejection; `/embed`, `/chunk`, and other endpoints now properly reject arrays in request bodies with 400 status.

#### CLI
- Fixed `--format json` to serialize the complete `ExtractionResult` including chunks, embeddings, images, pages, and elements.

#### MCP
- Fixed MCP tool responses to return full JSON-serialized `ExtractionResult`, matching API and CLI output.

#### Elixir Bindings
- Added `ExtractionConfig.new/0` and `new/1` constructors.
- Changed `text` field to `content` on `Chunk` for API parity with Rust core.

#### C# Bindings
- Fixed file-not-found errors to throw `KreuzbergIOException` instead of `KreuzbergValidationException`.

#### WASM / Cloudflare Workers
- Fixed `initWasm()` failing in Cloudflare Workers and Vercel Edge with "Invalid URL string" error; added `initWasm({ wasmModule })` option for explicit WASM module injection.

#### Go Bindings
- Removed references to deprecated `WithEmbedding()` API and `Chunking.Embedding` field.

#### Java Bindings
- Removed non-canonical `embedding` and `imagePreprocessing` top-level fields from `ExtractionConfig`.

#### MCP
- Fixed boolean merge logic bug causing configuration corruption when using `config` parameter.

---

## [4.2.2] - 2026-01-28

### Changed

#### PHP Bindings
- Removed 5 non-canonical fields from `ExtractionConfig` and fixed defaults; all 16 fields now match Rust canonical source.

#### Go Bindings
- Removed non-canonical `Success`, `Visible`, and `ContentType` fields from result types.

#### Ruby Bindings
- Fixed `enable_quality_processing` default from `false` to `true` to match Rust.

#### Java Bindings
- Fixed `enableQualityProcessing` default from `false` to `true` to match Rust.

#### TypeScript Bindings
- Removed non-existent type exports (`EmbeddingConfig`, `HierarchyConfig`, etc.) from index.ts.

### Fixed

#### Elixir Bindings
- Fixed `force_build: true` causing production installs to fail; now only builds from source in development. ([#333](https://github.com/kreuzberg-dev/kreuzberg/issues/333))

#### Docker Images
- Fixed "OCR backend 'tesseract' not registered" error by adding dynamic tessdata discovery for multiple tesseract versions.
- Fixed "Failed to initialize embedding model" error by adding persistent Hugging Face model cache directory.

#### API
- Fixed JSON error responses to return proper JSON `ErrorResponse` instead of plain text.
- Added validation constraints for chunking config and embed texts array.
- Added validation that `overlap` must be less than `max_characters`.
- `EmbeddingConfig.model` now defaults to "balanced" preset when not specified.

#### Rust Core
- Fixed XLSX out-of-memory with Excel Solver files that declare extreme cell dimensions. ([#331](https://github.com/kreuzberg-dev/kreuzberg/issues/331))

---

## [4.2.1] - 2026-01-27

### Fixed

#### Rust Core
- Fixed PPTX image page numbers being reversed due to unsorted slide paths. ([#329](https://github.com/kreuzberg-dev/kreuzberg/issues/329))
- Added comprehensive error logging for silent plugin failures. ([#328](https://github.com/kreuzberg-dev/kreuzberg/issues/328))
- Extended `VALID_OUTPUT_FORMATS` to include all valid aliases (`plain`, `text`, `markdown`, `md`, `djot`, `html`).
- Fixed `validate_file_exists()` to return `Io` error instead of `Validation` error for file-not-found.

#### Go Bindings
- Added `OutputFormatText` and `OutputFormatMd` format constant aliases.

#### Elixir Bindings
- Added `text` and `md` aliases to `validate_output_format`.

#### Ruby Bindings
- Fixed `extract` and `detect` methods to accept both positional and keyword arguments.
- Renamed `image_extraction` to `images` (canonical name) with backward-compatible alias.

#### PHP Bindings
- Renamed fields to canonical names (`images`, `pages`, `pdfOptions`, `postprocessor`, `tokenReduction`).
- Added missing `postprocessor` and `tokenReduction` fields.

#### Java Bindings
- Added `getImages()` and `images()` builder methods as aliases for `getImageExtraction()`.

#### WASM Bindings
- Added `outputFormat`, `resultFormat`, and `htmlOptions` to `ExtractionConfig` interface.

### Documentation

- Added Kubernetes deployment guide with health check configuration and troubleshooting. ([#328](https://github.com/kreuzberg-dev/kreuzberg/issues/328))

---

## [4.2.0] - 2026-01-26

### Added

#### MCP Interface
- Full `config` parameter support on all MCP tools, enabling complete configuration pass-through from AI agents.

#### CLI
- Added `--output-format` flag (canonical replacement for `--content-format`).
- Added `--result-format` flag for controlling result structure (unified, element_based).
- Added `--config-json` flag for inline JSON configuration.
- Added `--config-json-base64` flag for base64-encoded JSON configuration.

#### API - All Language Bindings
- Added `outputFormat` / `output_format` field (Plain, Markdown, Djot, HTML) to all bindings.
- Added `resultFormat` / `result_format` field (Unified, ElementBased) to all bindings.

#### Go Bindings
- Added `OutputFormat` and `ResultFormat` types with `WithOutputFormat()` and `WithResultFormat()` functional options.

#### Java Bindings
- Added `outputFormat` and `resultFormat` to Builder pattern.

#### PHP Bindings
- Added 6 missing configuration fields: `useCache`, `enableQualityProcessing`, `forceOcr`, `maxConcurrentExtractions`, `resultFormat`, `outputFormat`.

### Changed

#### Configuration Precedence
- CLI flag > inline JSON > config file > defaults.

#### MCP Schema Evolution
- `enable_ocr` and `force_ocr` now under `config` object instead of top-level parameters.

### Fixed

#### Ruby Bindings
- Fixed batch chunking operations.

#### MCP
- Fixed boolean merge logic bug in nested config objects.

### BREAKING CHANGES

**MCP Interface Only (AI-only, no user impact)**

- Removed `enable_ocr` and `force_ocr` top-level parameters from MCP tools; use `config.ocr.enable_ocr` and `config.force_ocr` instead.
- MCP tools now require `config` object parameter; old names accepted in v4.2 with deprecation warnings.

### Deprecated

#### CLI (backward compatible)
- `--content-format` flag deprecated in favor of `--output-format`.

#### Environment Variables (backward compatible)
- `KREUZBERG_CONTENT_FORMAT` deprecated in favor of `KREUZBERG_OUTPUT_FORMAT`.

---

## [4.1.2] - 2026-01-25

### Added

#### Ruby Bindings
- Added Ruby 4.0 support (tested with Ruby 4.0.1).

### Fixed

#### Ruby Bindings
- Fixed gem native extension build failure due to incorrect Cargo.toml path rewriting.

#### Go Bindings
- Fixed Windows timeout caused by FFI mutex deadlock; now uses lazy initialization via `sync.Once`.

---

## [4.1.1] - 2026-01-23

### Fixed

#### PPTX/PPSX Extraction
- Fixed PPTX extraction failing on shapes without text (e.g., image placeholders). (#321)
- Added PPSX (PowerPoint Show) file support.
- Added PPTM (PowerPoint Macro-Enabled) file support.

---

## [4.1.0] - 2026-01-21

### Added

#### API
- Added `POST /chunk` endpoint for text chunking with configurable `max_characters`, `overlap`, and `trim`.

#### Core
- Added Djot markup format support (`.djot`) with full parser, structured representation, and YAML frontmatter extraction.
- Added content output format configuration (`ContentFormat` enum: Plain, Markdown, Djot, HTML) with CLI `--content-format` flag.
- Added Djot output format support for HTML and OCR conversions.
- Added element-based output format (`ResultFormat::ElementBased`) providing Unstructured.io-compatible semantic element extraction.

#### Language Bindings
- All bindings (Python, TypeScript, Ruby, PHP, Go, Java, C#, Elixir, WASM) updated with content format and result format configuration, plus `Element`, `ElementType`, `ElementMetadata`, `BoundingBox`, and `DjotContent` types.

### Changed

- Split 22 large monolithic files into 110+ focused modules for improved maintainability; no breaking changes to public APIs.

### Fixed

#### Python
- Fixed missing type exports (`Element`, `ElementMetadata`, `ElementType`, `BoundingBox`, `HtmlImageMetadata`) in `kreuzberg.types.__all__`.

#### Elixir
- Fixed `FunctionClauseError` when extracting DOCX files with keywords metadata. ([#309](https://github.com/kreuzberg-dev/kreuzberg/issues/309))

---

## [4.0.8] - 2026-01-17

### Changed

#### Docker
- Migrated from Docker Hub to GitHub Container Registry (`ghcr.io/kreuzberg-dev/kreuzberg`).

### Fixed

#### C#
- Fixed `HtmlConversionOptions` serializing as `null` instead of `{}` when empty, causing Rust FFI errors.

#### Python
- Fixed missing `_internal_bindings.pyi` type stub file in Python wheels. ([#298](https://github.com/kreuzberg-dev/kreuzberg/issues/298))

#### Homebrew
- Fixed bottle checksum mismatches by computing checksums from actual uploaded release files.

---

## [4.0.6] - 2026-01-14

### Fixed

#### Elixir
- Fixed checksum file generation for precompiled NIFs during Hex.pm publishing.

#### PHP
- Fixed runtime panic from unregistered `ChunkMetadata` and `Keyword` classes in ext-php-rs.

---

## [4.0.5] - 2026-01-14

### Added

#### Go Module
- Added automated FFI library installer that downloads the correct platform-specific library from GitHub releases. ([#281](https://github.com/kreuzberg-dev/kreuzberg/issues/281))

### Fixed

#### Elixir
- Fixed precompiled NIF checksums missing from Hex package.

---

## [4.0.4] - 2026-01-13

### Fixed

#### Docker
- Fixed `MissingDependencyError` when extracting legacy MS Office formats in Docker; added LibreOffice symlinks and missing runtime dependencies. ([#288](https://github.com/kreuzberg-dev/kreuzberg/issues/288))

---

## [4.0.3] - 2026-01-12

### Added

#### HTML Configuration Support
- Full `html_options` configuration now available from config files and all language bindings. ([#282](https://github.com/kreuzberg-dev/kreuzberg/issues/282))

### Fixed

#### Go Module
- Fixed header include path so `go get` users no longer get compilation errors about missing headers. ([#280](https://github.com/kreuzberg-dev/kreuzberg/issues/280))

#### C# SDK
- Fixed `JsonException` when using keyword extraction; keywords now properly deserialized as `ExtractedKeyword` objects. ([#285](https://github.com/kreuzberg-dev/kreuzberg/issues/285))

#### Distribution
- Made Homebrew tap repository public to enable `brew install kreuzberg-dev/tap/kreuzberg`. ([#283](https://github.com/kreuzberg-dev/kreuzberg/issues/283))

---

## [4.0.2] - 2026-01-12

### Fixed

#### Go Module
- Fixed Go module tag format so `go get` works correctly. ([#264](https://github.com/kreuzberg-dev/kreuzberg/issues/264))

#### Elixir
- Fixed macOS native library extension (`.dylib` instead of `.so`).

---

## [4.0.1] - 2026-01-11

### Fixed

#### Elixir
- Fixed NIF binaries not uploaded to GitHub releases, breaking `rustler_precompiled`. ([#279](https://github.com/kreuzberg-dev/kreuzberg/issues/279))

#### Python
- Fixed `kreuzberg-tesseract` missing from PyPI source distributions, causing builds from source to fail. ([#277](https://github.com/kreuzberg-dev/kreuzberg/issues/277))

#### Homebrew
- Fixed bottle publishing workflow to publish releases from draft state.

#### Ruby
- Updated RBS type definitions to match keyword argument signatures.

#### WASM
- Fixed Svelte 5 variable naming and removed call to non-existent `detectMimeType()` API.

---

## [4.0.0] - 2026-01-10

### Highlights

First stable release of Kreuzberg v4, a complete rewrite with a Rust core and polyglot bindings for Python, TypeScript, Ruby, PHP, Java, Go, C#, Elixir, and WebAssembly.

### Added

#### FFI & Language Bindings
- Python FFI error handling via `get_last_error_code()` and `get_last_panic_context()`.
- PHP custom extractor support with metadata and tables flowing through to results.
- Dynamic Tesseract language discovery from installation.

### Removed

#### Legacy Support
- Completely removed v3 legacy Python package and infrastructure. V3 users should migrate to v4 using the [migration guide](https://docs.kreuzberg.dev/migration/v3-to-v4/).

---

## [4.0.0-rc.29] - 2026-01-08

### Added

#### Documentation
- Added comprehensive platform support documentation to all READMEs.

---

## [4.0.0-rc.28] - 2026-01-07

### Added

#### API Server
- Added `POST /embed` endpoint for generating embeddings from text. ([#266](https://github.com/Anthropic/kreuzberg/issues/266))
- Added `ServerConfig` type for file-based server configuration (TOML/YAML/JSON) with environment variable overrides.

#### Observability
- Added OpenTelemetry tracing instrumentation to all API endpoints.

### Fixed

#### API Server & CLI
- Fixed CLI to properly use ServerConfig from config files (CORS origins, upload size limits).

#### Configuration Examples
- Fixed YAKE/RAKE parameter examples to match actual source code.
- Changed default host from `0.0.0.0` to `127.0.0.1` for safer defaults.

#### PHP
- Fixed `extract_tables` config flag to properly filter table results.

---

## [4.0.0-rc.27] - 2026-01-04

### Fixed

- Fixed WASM npm package initialization failure caused by incorrect import paths in minified output.

---

## [4.0.0-rc.26] - 2026-01-03

### Fixed

- Fixed Node.js macOS ARM64 builds missing from publish workflow.
- Fixed WASM npm package missing WASM binaries.
- Fixed Elixir hex.pm publishing with correct public configuration.
- Fixed Homebrew bottle upload pattern.

---

## [4.0.0-rc.25] - 2026-01-03

### Fixed

- Added comprehensive chunking config validation to Go binding (negative values, excessive sizes, overlap constraints).
- Fixed Java FFI to use `Arena.global()` for thread-safe C string reads.

### Changed

- Updated C# target framework to .NET 10.0.

---

## [4.0.0-rc.24] - 2026-01-01

### Fixed

- Fixed Go Windows CGO directives to bypass pkg-config.
- Fixed Ruby Windows build with proper platform handling and embeddings feature.
- Fixed Node Windows tests with proper symlink resolution.
- Fixed Homebrew formula bottle naming and source sha256 fetching.

---

## [4.0.0-rc.23] - 2026-01-01

### Added

#### Java
- Added `EmbeddingConfig` class with builder pattern for embedding generation.

#### C#
- Added `EmbeddingConfig` sealed class as type-safe replacement for Dictionary-based configuration.

#### Node.js (NAPI-RS)
- Added Worker Thread Pool APIs: `createWorkerPool`, `extractFileInWorker`, `batchExtractFilesInWorker`, `closeWorkerPool`.

### Fixed

- Fixed page markers to include page 1 (previously only inserted for page > 1).
- Fixed Go concurrency crashes (segfaults/SIGTRAP) by adding mutex for thread-safe FFI calls.

---

## [4.0.0-rc.22] - 2025-12-27

### Added

- PHP bindings with comprehensive FFI bindings and E2E test suite.
- Root `composer.json` for Packagist publishing.
- HTML metadata extraction: headers, links, images, structured data (JSON-LD, Microdata, RDFa), `language`, `text_direction`, `meta_tags`.

### Fixed

- Fixed C# target framework from net10.0 (preview) to net8.0 LTS.
- Fixed Ruby vendor script missing workspace dependency inlining for `lzma-rust2` and `parking_lot`.

### Changed

- **BREAKING: HTML metadata structure** - Replaced YAML frontmatter parsing with single-pass metadata extraction. See `docs/migration/v4.0-html-metadata.md`.

---

## [4.0.0-rc.21] - 2025-12-26

### Fixed

- Fixed PDF initialization race conditions causing segfaults and concurrency errors across all language bindings.
- Fixed EPUB metadata extraction with incorrect field mapping (created_at mapped to subject).

### Added

- CLI test app for validating kreuzberg-cli published to crates.io.

---

## [4.0.0-rc.20] - 2025-12-25

### Added

- Font configuration API with configurable font provider, custom directory support, and automatic path expansion.

---

## [4.0.0-rc.19] - 2025-12-24

### Added

- Homebrew bottle support for faster macOS installation.
- Environment variable configuration for API size limits (`KREUZBERG_MAX_REQUEST_BODY_BYTES`, `KREUZBERG_MAX_MULTIPART_FIELD_BYTES`).
- Config file caching for TOML/YAML/JSON loading.

### Fixed

- Fixed large file uploads rejected above 2MB; now configurable up to 100MB. (#248)
- Fixed browser package Vite compatibility with missing `pdfium.js`. (#249)
- Fixed Node.js missing binaries in Docker and pnpm monorepo environments. (#241)
- Fixed Ruby gem native extension build with proper linker path resolution.
- Fixed font provider thread safety race condition.
- Added custom font path validation with symlink resolution and canonicalization.

### Changed

- **BREAKING**: Custom font provider now enabled by default.
- Default API size limit increased to 100MB.
- TypeScript serialization replaced MessagePack + Base64 with direct JSON.

### Performance

- 15-25% overall execution improvement, 30-45% memory reduction.
- Memory pool improvements (35-50% reduction).

### Removed

- Removed deprecated backward compatibility: TypeScript `KREUZBERG_LEGACY_SERIALIZATION`, Go legacy error codes, Ruby `Ocr = OCR` alias, Rust `Metadata.date` field, Cargo legacy feature aliases.

### Security

- Custom font directories validated with canonicalization and symlink resolution.

---

## [4.0.0-rc.18] - 2025-12-23

### Fixed

- Fixed Ruby gem missing kreuzberg-ffi crate in vendored dependencies.
- Fixed Ruby gem macOS linker errors.
- Fixed Python wheels macOS ImportError from hardcoded dylib paths.

---

## [4.0.0-rc.17] - 2025-12-22

### Added

- Docker ARM64 support with multi-architecture images.

### Fixed

- Fixed Python wheels macOS ImportError.
- Fixed Ruby gems macOS linker errors.
- Fixed TypeScript plugin registration TypeError with JavaScript-style plugins.

### Performance

- Improved Go ConfigMerge performance with native field copying.

---

## [4.0.0-rc.16] - 2025-12-21

### Added

- Batch processing APIs with 4-6x throughput improvement for high-volume extraction.

### Fixed

- Fixed Python IDE support; type stub files now included in wheel distributions.
- Fixed Go Windows linking with duplicate linker flags.
- Fixed Ruby gem compilation with missing link search paths.
- Fixed Ruby gem publishing with artifact corruption.

### Performance

- 2-3x batch throughput gains with FFI streaming.
- C# JSON serialization with source generation.

---

## [4.0.0-rc.15] - 2025-12-20

### Fixed

- Fixed Node.js Windows x64 platform packages not publishing to npm.

---

## [4.0.0-rc.14] - 2025-12-20

### Fixed

- Fixed LibreOffice in Docker (updated to version 25.8.4).
- Fixed Python IDE type hints; type stub files now included in wheels.
- Fixed Ruby gem compilation with Rust crate vendoring.
- Fixed Python `ExtractionResult` missing `pages` field in IDE autocomplete.

---

## [4.0.0-rc.13] - 2025-12-19

### Fixed

- Fixed PDF `bundled` feature flag (corrected to `bundled-pdfium`).
- Fixed Go Windows linking with missing system libraries.
- Fixed Ruby gem packaging with missing TOML dependency.
- Fixed WASM distribution with compiled binaries for npm publishing.

---

## [4.0.0-rc.12] - 2025-12-19

### Fixed

- Fixed Python wheels PDFium bundling with correct feature flag.
- Fixed C# MSBuild target for native assets.
- Fixed Ruby bindings `unsafe` keyword for Rust 2024 edition.
- Fixed Docker ONNX Runtime package name for Debian Trixie.

---

## [4.0.0-rc.11] - 2025-12-18

### Fixed

- Fixed PDFium bundling now correctly included in all language bindings.
- Fixed C# native libraries build target for platform-specific copies.
- Fixed Ruby gem publishing with double-compression validation errors.
- Fixed Go Windows linking with duplicate CGO linker flags.
- Added WASM PDF extraction support for browser and Node.js.

---

## [4.0.0-rc.10] - 2025-12-16

### Breaking Changes

- PDFium feature names changed: `pdf-static` -> `static-pdfium`, `pdf-bundled` -> `bundled-pdfium`, `pdf-system` -> `system-pdfium`.
- Default PDFium linking changed to `bundled-pdfium`.
- Go module path moved to `github.com/kreuzberg-dev/kreuzberg/packages/go/v4`.

### Fixed

- Fixed Windows CLI to include bundled PDFium runtime.
- Added Go `ExtractFileWithContext()` and batch variants.
- Replaced TypeScript `any` types with proper definitions.

---

## [4.0.0-rc.9] - 2025-12-15

### Added

- `PDFIUM_STATIC_LIB_PATH` environment variable for custom static PDFium paths in Docker builds.

### Fixed

- Fixed Python wheels to include typing metadata (`.pyi` stubs).
- Fixed Java Maven packages to bundle platform-specific native libraries.
- Fixed Node npm platform packages to contain compiled `.node` binaries.
- Fixed WASM Node.js runtime crash with `self is not defined`.
- Fixed PDFium static linking to correctly search for `libpdfium.a`.

---

## [4.0.0-rc.8] - 2025-12-14

### Added

- MCP HTTP Stream transport with SSE support.

### Fixed

- Fixed Go CGO library path configuration for Linux and macOS.
- Fixed Python wheels manylinux compatibility.
- Fixed Ruby gems to remove embedding model cache from distribution.
- Fixed Maven Central publishing to use modern Sonatype Central API.

---

## [4.0.0-rc.7] - 2025-12-12

### Added

- Configurable PDFium linking: `pdf-static`, `pdf-bundled`, `pdf-system` Cargo features.
- WebAssembly bindings with full TypeScript API for browser, Cloudflare Workers, and Deno.
- RTF extractor improvements with structured table extraction and metadata support.
- Page tracking redesign with byte-accurate page boundaries and per-page metadata.

### Changed

- **BREAKING**: `ChunkMetadata` fields renamed: `char_start` -> `byte_start`, `char_end` -> `byte_end`. (#226)

### Fixed

- Fixed Ruby gem corruption from embedding model cache in distribution.
- Fixed Java FFM SIGSEGV from struct alignment on macOS ARM64.
- Fixed C# variable shadowing compilation errors.

---

## [4.0.0-rc.6] - 2025-12-10

### Added

- `core` feature for lightweight FFI build without ONNX Runtime.

### Fixed

- Fixed ODT table extraction with duplicate content.
- Fixed ODT metadata extraction to match Office Open XML capabilities.
- Fixed Go Windows MinGW builds by disabling embeddings feature.
- Fixed Ruby rb-sys conflict by removing vendoring.
- Fixed Python text extraction missing `format_type` metadata field.

---

## [4.0.0-rc.5] - 2025-12-01

### Breaking Changes

- Removed all Pandoc dependencies; native Rust extractors now handle all 12 previously Pandoc-supported formats (LaTeX, EPUB, BibTeX, Typst, Jupyter, FictionBook, DocBook, JATS, OPML, Org-mode, reStructuredText, RTF).

### Fixed

- Fixed macOS CLI binary missing libpdfium.dylib at runtime.
- Fixed Windows Go builds with GNU toolchain detection.
- Fixed Ruby Bundler 4.0 gem installation failures.

---

## [4.0.0-rc.4] - 2025-12-01

### Fixed

- Fixed crates.io and Maven Central publishing authentication.
- Fixed ONNX Runtime mutex errors and deadlocks.

---

## [4.0.0-rc.3] - 2025-12-01

### Fixed

- Fixed NuGet publishing authentication.
- Fixed CLI binary packages to include libpdfium shared library.

---

## [4.0.0-rc.2] - 2025-11-30

### Breaking Changes

- TypeScript/Node.js package renamed from `kreuzberg` to `@kreuzberg/node`.

### Added

- C#/.NET bindings using .NET 9+ FFM API.
- MkDocs documentation site with multi-language examples and API reference.

### Fixed

- Fixed Tesseract OCR API call ordering.
- Fixed Go Windows CGO MinGW linking.
- Fixed embeddings model cache lock poisoning recovery.

---

## [4.0.0-rc.1] - 2025-11-23

### Major Release - Complete Rewrite

Complete architectural rewrite from Python-only to Rust-core with polyglot bindings.

### Architecture

- Rust core with all extraction logic for performance.
- Polyglot bindings: Python (PyO3), TypeScript/Node.js (NAPI-RS), Ruby (Magnus), Java (FFM API), Go (CGO).
- 10-50x performance improvements with streaming parsers for multi-GB files.

### Added

- Plugin system: PostProcessor, Validator, Custom OCR, Custom Document Extractors.
- Language detection with automatic multi-language support.
- RAG and embeddings with 4 presets (fast/balanced/quality/multilingual).
- Image extraction from PDFs and PowerPoint with metadata.
- Stopwords system for 64 languages.
- Comprehensive format-specific metadata for PDF, Office, Email, Images, XML, HTML.
- MCP server for Claude integration.
- Docker support with multi-variant images and OCR backends.

### Changed

- Async-first API; sync variants have `_sync` suffix.
- Strongly-typed config and metadata.
- New API: `extract()` -> `extract_file()`, added `extract_bytes()`, `batch_extract_files()`.

### Removed

- Pure-Python API, Pandoc dependency, GMFT, spaCy entity extraction, KeyBERT, document classification.

### Breaking Changes

- Python 3.10+, Node.js 18+, Rust 1.75+ required.
- Binary wheels only.
- TypeScript/Node.js package renamed to `@kreuzberg/node`.
- `char_start`/`char_end` -> `byte_start`/`byte_end`.

See [Migration Guide](https://docs.kreuzberg.dev/migration/v3-to-v4/) for details.

---

## [3.22.0] - 2025-11-27

### Fixed
- Fixed EasyOCR import error handling.
- Hardened HTML regexes for script/style stripping.

---

## [3.21.0] - 2025-11-05

### Added

- Complete Rust core library with document extraction pipeline, plugin system, PDF/Office/HTML/XML extraction, OCR subsystem, image processing, text processing, cache, embeddings, MCP server, and CLI.
- Language bindings: Python (PyO3), TypeScript (NAPI-RS), Ruby (Magnus), Java (FFM API), Go (CGO), C# (FFI).
- REST API server and MCP server for Claude integration.

### Changed

- Architecture restructured around Rust core with thin language-specific wrappers.
- Build system upgraded to Rust Edition 2024 with Cargo workspace.

### Removed

- Old v3 codebase superseded by v4.

### Security

- All dependencies audited, sandboxed subprocess execution, input validation, memory safety via Rust.

### Performance

- Streaming PDF extraction, zero-copy patterns, SIMD optimizations, ONNX Runtime for embeddings, async-first design.

---

## [3.20.2] - 2025-10-11

### Fixed

- Fixed missing optional dependency errors in GMFT extractor.

---

## [3.20.1] - 2025-10-11

### Changed

- Optimized sdist size by excluding unnecessary files.

---

## [3.20.0] - 2025-10-11

### Added

- Python 3.14 support.

### Changed

- Migrated HTML extractor to html-to-markdown v2.

---

## [3.19.1] - 2025-09-30

### Fixed

- Fixed Windows Tesseract 5.5.0 HOCR output compatibility.
- Fixed TypedDict configs with type narrowing and cast.

---

## [3.19.0] - 2025-09-29

### Added

- Context-aware exception handling with critical system error policy.

### Fixed

- Aligned sync/async OCR pipelines and fixed Tesseract PSM enum handling.
- Removed magic library dependency.
- Added Windows-safe fallbacks for CLI progress.
- Fixed ValidationError handling in batch processing.

---

## [3.18.0] - 2025-09-27

### Added

- API server configuration via environment variables.
- Auto-download missing spaCy models for entity extraction.
- Regression tests for German image PDF extraction. (#149)

### Changed

- Updated html-to-markdown to latest version.

### Fixed

- Fixed HOCR parsing issues.

---

## [3.17.0] - 2025-09-17

### Added

- Token reduction for text optimization with streaming support.

### Fixed

- Fixed excessive markdown escaping in OCR output. (#133)

---

## [3.16.0] - 2025-09-16

### Added

- Enhanced JSON extraction with schema analysis and custom field detection.

### Fixed

- Fixed `HTMLToMarkdownConfig` not exported in public API.
- Fixed EasyOCR module-level variable issues.
- Fixed Windows-specific path issues.

---

## [3.15.0] - 2025-09-14

### Added

- Comprehensive image extraction support.
- Polars DataFrame and PIL Image serialization for API responses.

### Fixed

- Fixed TypeError with unhashable dict in API config merging.

---

## [3.14.0] - 2025-09-13

### Added

- DPI configuration system for OCR processing.

### Changed

- Enhanced API with 1GB upload limit and comprehensive OpenAPI documentation.
- Completed pandas to polars migration.

---

## [3.13.0] - 2025-09-04

### Added

- Runtime configuration API with query parameters and header support.
- OCR caching system for EasyOCR and PaddleOCR backends.

### Changed

- Replaced pandas with polars for table extraction.

### Fixed

- Fixed Tesseract TSV output format and table extraction.
- Fixed UTF-8 encoding handling across document processing.
- Fixed HTML-to-Markdown configuration externalization.
- Fixed regression in PDF extraction and XLS file handling.

---

## [3.12.0] - 2025-08-30

### Added

- Multilingual OCR support in Docker images with flexible backend selection.

### Changed

- Simplified Docker images to base and core variants.

### Fixed

- Fixed naming conflict in CLI config command.

---

## [3.11.1] - 2025-08-13

### Fixed

- Fixed EasyOCR device-related parameters passed to readtext() calls.
- Optimized numpy import to only load inside `process_image_sync` for faster startup.

---

## [3.11.0] - 2025-08-01

### Changed

- Implemented Python 3.10+ syntax optimizations.

### Fixed

- Fixed image extractor async delegation.
- Fixed timezone assertion in spreadsheet metadata.
- Fixed ExceptionGroup import for Python 3.10+ compatibility.
- Fixed `_parse_date_string` bug.

---

## [3.10.0] - 2025-07-29

### Added

- PDF password support through new crypto extra feature.

---

## [3.9.0] - 2025-07-17

### Added

- Initial release of v3.9.0 series.

---

## [3.8.0] - 2025-07-16

### Added

- Foundation for v3.8.0 release.

---

## [3.7.0] - 2025-07-11

### Added

- MCP server for AI integration enabling Claude integration with document extraction.

### Fixed

- Fixed chunk parameters to prevent overlap validation errors.
- Fixed HTML test compatibility with html-to-markdown v1.6.0.

---

## [3.6.0] - 2025-07-04

### Added

- Language detection integrated into extraction pipeline.

### Fixed

- Completed entity extraction migration from gliner to spaCy.

### Changed

- spaCy now used for entity extraction replacing gliner.

---

## [3.5.0] - 2025-07-04

### Added

- Language detection with configurable backends.
- Full synchronous support for PaddleOCR and EasyOCR backends.

### Fixed

- Fixed chunking default configuration.
- Fixed PaddleOCR sync implementation for v3.x API.

### Changed

- Python 3.10+ now required (3.9 support dropped).

---

## [3.4.0] - 2025-07-03

### Added

- API support with Litestar framework for web-based document extraction.
- EasyOCR and GMFT Docker build variants.

### Fixed

- Fixed race condition in GMFT caching.
- Fixed race condition in Tesseract caching.

---

## [3.3.0] - 2025-06-23

### Added

- Isolated process wrapper for GMFT table extraction.
- CLI support with Click framework.
- Pure synchronous extractors without anyio dependencies.
- Document-level caching with per-file locks and parallel batch processing.
- Thread lock for pypdfium2 to prevent macOS segfaults.

### Fixed

- Fixed Windows-specific multiprocessing and utils failures.
- Fixed file existence validation in extraction functions.

### Changed

- Replaced msgspec JSON with msgpack for 5x faster cache serialization.

---

## [3.2.0] - 2025-06-23

### Added

- GPU acceleration support for OCR and ML operations.

### Fixed

- Fixed EasyOCR byte string issues.
- Fixed Pandoc version issues.
- Added multiple language support to EasyOCR.

---

## [3.1.0] - 2025-03-28

### Added

- GMFT (Give Me Formatted Tables) support for vision-based table extraction.

### Changed

- Image extraction now non-optional in results.

---

## [3.0.0] - 2025-03-23

### Added

- Chunking functionality for document segmentation.
- Extractor registry for managing format-specific extractors.
- Hooks system for pre/post-processing.
- OCR backend abstraction with EasyOCR and PaddleOCR support.
- Multiple language support in OCR backends.

### Fixed

- Fixed Windows error message handling.
- Fixed PaddleOCR integration issues.

### Changed

- Refactored structure for improved organization.
- OCR integration with configurable backends.

---

## See Also

- [Configuration Reference](reference/configuration.md) - Detailed configuration options
- [Migration Guide](migration/v3-to-v4.md) - v3 to v4 migration instructions
- [Format Support](reference/formats.md) - Supported file formats
- [Extraction Guide](guides/extraction.md) - Extraction examples

[4.2.10]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.10
[4.2.9]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.9
[4.2.8]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.8
[4.2.7]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.7
[4.2.6]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.6
[4.2.5]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.5
[4.2.4]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.4
[4.2.3]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.3
[4.2.2]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.2
[4.2.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.1
[4.2.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.0
[4.1.2]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.1.2
[4.1.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.1.1
[4.1.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.1.0
[4.0.8]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.8
[4.0.6]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.6
[4.0.5]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.5
[4.0.4]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.4
[4.0.3]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.3
[4.0.2]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.2
[4.0.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.1
[4.0.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0
[4.0.0-rc.29]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.29
[4.0.0-rc.28]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.28
[4.0.0-rc.27]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.27
[4.0.0-rc.26]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.26
[4.0.0-rc.25]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.25
[4.0.0-rc.24]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.24
[4.0.0-rc.23]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.23
[4.0.0-rc.22]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.22
[4.0.0-rc.21]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.21
[4.0.0-rc.20]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.20
[4.0.0-rc.19]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.19
[4.0.0-rc.18]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.18
[4.0.0-rc.17]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.17
[4.0.0-rc.16]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.16
[4.0.0-rc.15]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.15
[4.0.0-rc.14]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.14
[4.0.0-rc.13]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.13
[4.0.0-rc.12]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.12
[4.0.0-rc.11]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.11
[4.0.0-rc.10]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.10
[4.0.0-rc.9]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.9
[4.0.0-rc.8]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.8
[4.0.0-rc.7]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.7
[4.0.0-rc.6]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.6
[4.0.0-rc.5]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.5
[4.0.0-rc.4]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.4
[4.0.0-rc.3]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.3
[4.0.0-rc.2]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.2
[4.0.0-rc.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.22.0
[3.22.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.22.0
[3.21.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.21.0
[3.20.2]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.20.2
[3.20.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.20.1
[3.20.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.20.0
[3.19.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.19.1
[3.19.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.19.0
[3.18.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.18.0
[3.17.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.17.0
[3.16.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.16.0
[3.15.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.15.0
[3.14.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.14.0
[3.13.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.13.0
[3.12.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.12.0
[3.11.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.11.1
[3.11.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.11.0
[3.10.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.10.0
[3.9.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.9.0
[3.8.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.8.0
[3.7.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.7.0
[3.6.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.6.0
[3.5.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.5.0
[3.4.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.4.0
[3.3.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.3.0
[3.2.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.2.0
[3.1.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.1.0
[3.0.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v3.0.0
