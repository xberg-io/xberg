# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Changed

- **Vendored yake-rust 1.0.3** into kreuzberg core, removing external dependency
  - Fixes #676: `BacktrackLimitExceeded` panic on large files (10+ MB) by replacing regex-based sentence splitting with memchr-based approach
  - Expanded YAKE stopwords from 34 to 64 languages using kreuzberg's unified stopwords module
  - Optimized with ahash, memchr, and byte-table punctuation lookups
  - Removed 6 transitive dependencies (yake-rust, segtok, fancy-regex, streaming-stats, hashbrown, levenshtein)

---

## [4.8.0] - 2026-04-08

### Added

- **Cross-extractor content filtering configuration** — New `ContentFilterConfig` on `ExtractionConfig` with `include_headers`, `include_footers`, `strip_repeating_text`, and `include_watermarks` flags. Controls header/footer/furniture inclusion across PDF, DOCX, RTF, ODT, HTML, EPUB, and PPT extractors. Typed in all bindings (Python, TypeScript, Ruby, Go, Elixir, PHP, Java, C#, WASM).
- **Local LLM support** via liter-llm 1.2 — use Ollama, LM Studio, vLLM, llama.cpp, LocalAI, or llamafile as VLM OCR, embedding, or structured extraction backends with zero API key configuration
- **LLM-powered document intelligence via liter-llm** — Integrates with 146 LLM providers (including local inference engines) for three new capabilities:
  - **VLM OCR**: Vision language models as OCR backend (OpenAI GPT-4o, Anthropic Claude, Google Gemini, etc.). Superior accuracy for low-quality scans, handwriting, Arabic/Farsi, and complex layouts. Configure via `ocr.backend = "vlm"` with `ocr.vlm_config`.
  - **Structured Extraction**: Extract structured JSON data from documents using a JSON schema constraint. Users provide a schema and optional Jinja2 prompt template; the LLM returns conforming data. Supports strict mode (OpenAI) with automatic schema sanitization for cross-provider compatibility.
  - **VLM Embeddings**: Provider-hosted embedding models (e.g., `openai/text-embedding-3-small`, `mistral/mistral-embed`) as alternative to local ONNX models. Works through existing `/embed` API, `embed_text` MCP tool, and `embed` CLI command.
- **New CLI command**: `kreuzberg extract-structured` for schema-guided LLM extraction
- **New API endpoint**: `POST /extract-structured` with multipart file upload
- **New MCP tool**: `extract_structured` for AI assistant integration
- **Minijinja template engine** for customizable LLM prompts — structured extraction supports `{{ content }}`, `{{ schema }}`, `{{ schema_name }}`, `{{ schema_description }}`; VLM OCR supports `{{ language }}`
- **5 new environment variables**: `KREUZBERG_LLM_MODEL`, `KREUZBERG_LLM_API_KEY`, `KREUZBERG_LLM_BASE_URL`, `KREUZBERG_VLM_OCR_MODEL`, `KREUZBERG_VLM_EMBEDDING_MODEL`
- `LlmConfig` and `StructuredExtractionConfig` types exposed in Python, Node.js, and PHP bindings
- `structured_output` field on `ExtractionResult` across all languages
- `structured_output_json` field in C FFI `CExtractionResult` struct
- `EmbeddingModelType::Llm` variant for provider-hosted embeddings
- VLM OCR registered as plugin backend in OCR registry
- Standalone text embedding API (#599, #614) with `/embed` endpoint, `embed_text` MCP tool, and `embed` CLI command

### Changed

- **License changed from MIT to Elastic License 2.0 (ELv2)** — copyright holder changed to Kreuzberg, Inc. Forked upstream crates (kreuzberg-paddle-ocr, kreuzberg-tesseract, kreuzberg-pdfium-render) retain their original MIT licenses.
- All `ExtractionResult` constructors refactored to use `..Default::default()` for forward compatibility
- Embed CLI command extended with `--provider llm` and `--model` flags
- Embed MCP tool extended with `model` and `api_key` parameters
- Extract CLI overrides extended with `--vlm-model`, `--vlm-api-key`, `--vlm-prompt`
- API returns 501 Not Implemented (instead of 500) when liter-llm feature is disabled
- JSON schema `additionalProperties` automatically stripped for non-OpenAI providers

### Fixed

- FFI error code tests updated for Embedding variant
- Flaky FFI string_intern tests serialized with `serial_test`
- TypeScript `NativeBinding` interface updated with `embedSync`/`embed` declarations
- E2E generator emits minimal `cfg` (no `any()` wrapper for single conditions)
- **PDF: brand names stripped by repeating text detection** — `ContentFilterConfig.strip_repeating_text = false` disables cross-page repeating text removal that incorrectly strips brand names from PowerPoint-exported decks (#667)
- **PPTX: slide order scrambled for decks with 10+ slides** — Fixed lexicographic sort of slide paths (`slide10.xml` before `slide2.xml`) to use numeric ordering (#669)
- **UTF-8 panic in arXiv watermark stripping** — `strip_arxiv_watermark_noise` panics when a multi-byte character spans the 6000-byte search limit. Fixed with `floor_char_boundary` (#663)
- **DOC: garbled text from old Word files** — CP1252 text misread as UTF-16LE when the fCompressed bit is unreliable. Added heuristic to detect and re-decode garbled output (#666)
- **WASM: table extraction returns empty array** — TypeScript validation silently drops tables when `pageNumber` is null. Fixed to default to page 0 (#655)

---

## [4.7.4] - 2026-04-06

### Added

- Re-added `--layout` boolean CLI flag for easy layout detection enablement (use `--layout` to enable with model defaults, `--layout false` to explicitly disable)
- arXiv watermark/sidebar noise filtering for academic PDFs — strips LaTeX sidebar identifiers from extracted text
- Second-tier cross-page repeating text detection — catches conference headers and journal running titles that repeat on >70% of pages but appear outside the margin zone
- Figure/picture text suppression — text inside layout-detected Picture regions is now marked as page furniture and excluded from body output

### Fixed

- **Figure-internal text leaking into body output** — Text from inside figures and diagrams (e.g., diagram labels, axis text) was incorrectly included in the extracted body content, sometimes promoted to headings. The layout detection pipeline now suppresses text paragraphs classified as Picture regions.
- CLI tests now correctly reference `--content-format` instead of deprecated `--output-format`
- **Empty image references in PDF markdown/HTML output** — PDFs with embedded images produced empty `![]()` references in markdown and `<img src="" alt="">` in HTML output. The PDF structure pipeline now extracts actual image pixel data via pdfium and populates document images, producing proper `![](image_N.png)` references.
- **Invalid `extractFromFile` config in documentation** — Demo code in the TypeScript API reference included invalid configuration parameters that caused runtime errors.
- **WASM build failure with `extern "C-unwind"`** — The LLVM WASM backend does not support `cleanupret` instructions generated by `extern "C-unwind"` FFI blocks. Added `ffi_extern!` macro that uses `extern "C-unwind"` on native targets (for C++ exception safety) and `extern "C"` on WASM.
- **Go module tag format** — Go module tags now use the correct `packages/go/v4/vX.Y.Z` format matching the module path in `go.mod`, plus the legacy `packages/go/vX.Y.Z` format for backwards compatibility. Backfilled tags for all stable releases.

### Changed

- CLI documentation updated with all missing extraction override flags (`--layout-table-model`, `--disable-ocr`, `--cache-namespace`, `--cache-ttl-secs`)

---

## [4.7.3] - 2026-04-05

### Fixed

- **Archive extraction SIGBUS crash on macOS ARM64** — ZIP, 7Z, TAR, and GZIP archive extraction crashed with SIGBUS (signal 10) in release builds due to miscompilation of unsafe code in `sevenz-rust2` and `zip` crates under `opt-level=3`. Reduced optimization level to 2 for these crates. This also fixes Elixir, R, Go, and C benchmark crashes when processing archive files.
- **Native-text PDF extraction fails when OCR backend unavailable** (#646) — PDFs with extractable native text hard-failed with `ParsingError: All OCR pipeline backends failed` when no OCR backend (PaddleOCR/Tesseract) was installed, even though pdfium already extracted text successfully. The automatic OCR quality-enhancement pass now gracefully falls back to the native extraction result when OCR backends are unavailable, emitting a warning instead of failing.
- **Elixir Logger pollutes stdout** — Elixir benchmark scripts produced `[debug] Initialized Kreuzberg.Plugin.Registry` on stdout, corrupting JSON output. Logger default handler now configured to write to stderr via `config :logger, :default_handler`.
- **WASM benchmark module resolution** — WASM benchmark script failed to load `@kreuzberg/wasm` through pnpm virtual store due to `import.meta.url` resolution issues in tsx. Changed to direct import from local build path.
- **CI: FFI-dependent tests fail when FFI build skipped** — Go, Elixir, R, C FFI, and CLI test jobs ran and failed when `build-ffi` was skipped by paths-filter. Added `needs.build-ffi.result == 'success'` guard.
- **Rust cannot catch foreign exceptions crash** (#606) — C++ exceptions from Tesseract or Leptonica (e.g. on corrupted images or edge-case inputs) propagated across the FFI boundary unhandled, causing `fatal runtime error: Rust cannot catch foreign exceptions, aborting`. All Tesseract/Leptonica FFI declarations now use `extern "C-unwind"` to allow foreign exceptions to unwind safely, and OCR processing is wrapped with `catch_unwind` to convert them to recoverable errors.

---

## [4.7.2] - 2026-04-04

### Added

- **E2E generator published mode** — `cargo run -p kreuzberg-e2e-generator -- generate --mode published --version <V>` generates standalone test apps against published registry versions (PyPI, npm, Maven, NuGet, crates.io, Hex, RubyGems). All 12 language generators now also produce their project/dependency files (pyproject.toml, package.json, composer.json, etc.).

### Changed

- **Global model cache** (#641) — Models now download to platform-appropriate global cache (`~/.cache/kreuzberg/` on Linux, `~/Library/Caches/kreuzberg/` on macOS, `%LOCALAPPDATA%/kreuzberg/` on Windows) instead of per-directory `.kreuzberg/` folders. Override with `KREUZBERG_CACHE_DIR` env var. Consolidates 7 duplicate cache-dir resolution implementations into a single `cache_dir::resolve_cache_dir()` function.

### Fixed

- **Embedded HTML in PDF text layers** — PDFs with raw HTML in their text layer (`<p>`, `<br />`, `<a href>`) produced escaped garbage (`\<p\>`) in output. Now detected and converted to clean markdown using `html-to-markdown-rs`, the same crate and config used by the HTML extractor. Comrak-generated `<!-- end list -->` comments also stripped from output.
- **Code classification false positives** — Layout model sometimes classified regular prose as Code blocks. Added a prose guard that rejects Code classification for text with sentence punctuation, low syntax density, and many words.
- **PageBreak rendering as `-----` separators** — PageBreak elements in InternalDocument were rendered as ThematicBreak (`-----`) in markdown and `<hr>` in HTML output. This polluted extraction output with separators that don't exist in the source document. PageBreak is now treated as structural metadata — paragraph breaks between elements provide sufficient page separation, matching the pdfium baseline behavior.
- **Leptonica DPI crash** (#606) — Images with resolution 0 DPI caused Leptonica preprocessing (background normalization, unsharp mask, grayscale conversion) to trigger a C++ exception that Rust cannot catch, aborting the process. Now validates and fixes DPI to 72 before preprocessing. Also disabled C++ exception handling on Windows MSVC builds (`/EHsc` removed).
- **Node.js `ExtractionResult.children` missing at runtime** — The `children` field was declared in TypeScript definitions but missing from the runtime NAPI object in the published v4.7.1 binary, causing parity test failures.
- **Layout detection fixture stale `preset` field** — E2E fixture `layout_detection.json` included removed `preset` field, causing Python test failures. Removed from fixture.
- **Node.js `disable_ocr` config not respected** — Setting `disableOcr: true` in the Node.js binding still produced OCR content for images instead of returning empty content.
- **C# `Serialization` class inaccessible** — Generated e2e tests referenced `Serialization` class with insufficient access level in the published NuGet package.
- **Java `PdfAnnotation` missing getters** — `getContent()` and `getPageNumber()` methods were missing from the Java record, causing parity test failures. Added JavaBean-style getters to match `getAnnotationType()` and `getBoundingBox()`.
- **Java `Table` missing getters** — `getCells()`, `getMarkdown()`, and `getPageNumber()` methods were missing from the Java record. Added JavaBean-style getters to match existing `getBoundingBox()`.
- **Go test_app module conflict** — Generated Go test_apps used the same module name as e2e/go, causing workspace conflicts. Published mode now uses a distinct module path.
- **PaddleOCR angle classification crash** (#643) — V2 angle classifier model (`PP-LCNet_x1_0_textline_ori`) expects `[N, 3, 80, 160]` input but preprocessing resized to `[N, 3, 48, 192]` (old mobile cls dimensions). Fixed input dimensions to match the v2 model.
- **Centralized concurrency controls** — Fixed 5 places bypassing `resolve_thread_budget()`: embeddings ONNX session (no thread config at all), image OCR (hardcoded 8 tasks), batch extraction fallback (`num_cpus * 1.5`), doc orientation (`.min(4)` cap), PaddleOCR BaseNet (`inter_threads` set to `num_thread` instead of `1`).
- **Chunk page numbers missing** (#636) — Chunks produced with `first_page: null, last_page: null` when chunking was configured without explicit `pages` config. Three fixes: (1) auto-enable page tracking when chunking is configured, so the PDF extractor always produces per-page boundaries; (2) improved page boundary recomputation with first-line fallback when exact content match fails due to rendering transformations; (3) allow zero-length boundaries for blank pages instead of failing validation.

---

## [4.7.1] - 2026-04-03

### Added

- **Tree-sitter grammar management CLI** — New `kreuzberg tree-sitter` subcommand with `download`, `list`, `cache-dir`, and `clean` sub-commands for managing tree-sitter grammar parsers. Supports downloading by language name, group (`--groups web,systems,scripting`), or all (`--all`). Reads `[tree_sitter]` config from `kreuzberg.toml` with `--from-config`.
- **Tree-sitter grammar management API** — New REST endpoints: `POST /grammars/download`, `GET /grammars/list`, `GET /grammars/cache`, `DELETE /grammars/cache` for programmatic grammar management.
- **Tree-sitter grammar management MCP tools** — New MCP tools: `download_grammars`, `list_grammars`, `grammar_cache_info`, `clean_grammar_cache` for AI assistant-driven grammar management.
- **Tree-sitter config startup initialization** — API and MCP servers auto-download tree-sitter grammars on startup when `[tree_sitter]` config specifies `languages` or `groups`.

### Changed

- **Normalized OCR+layout pipeline** — Tesseract+layout path now follows the same architecture as pdfium+layout: hOCR → PdfParagraph → `apply_layout_overrides` → `assemble_internal_document` → comrak. Replaces the broken custom `apply_layout_to_ocr_document` path that destroyed paragraph structure and reading order.
- **Elixir NIF crash protection** — All extraction and batch NIFs now wrapped with `catch_unwind` to prevent panics in native C libraries (pdfium, tesseract) from crashing the BEAM VM. Panics are caught and returned as `{:error, reason}` tuples with error-level tracing including backtraces.

### Fixed

- **hOCR parser depth tracking** — Fixed paragraph boundary detection in the hOCR parser that used a generic depth counter for `<p>`, `<span>`, and `<div>` tags. Closing tags from inner word spans could prematurely terminate a paragraph, causing content after that point to be silently dropped. Now uses tag-name-specific depth tracking.
- **hOCR multi-page content loss** — Per-page hOCR documents from tesseract always report `ppageno=0` (page=1), but the paragraph conversion filtered by the actual page index, silently dropping all content on pages 2+. Removed the per-page filter since each hOCR document is independently extracted per page.
- **OCR batch parallelization** — OCR page processing was hardcoded to 4 concurrent pages regardless of available CPUs. Now uses `resolve_thread_budget()` (auto-detects CPUs, capped at 8) for significantly faster multi-page document processing.
- **Benchmark workflow** — Removed reference to deleted `kreuzberg-extract` binary target.
- **Ruby OCR backend** — Added missing `ocr_internal_document` field to `ExtractionResult` construction.
- **Keyword extraction tests** — Updated test assertions to use new `extracted_keywords` field instead of deprecated `metadata.additional["keywords"]`.
- **PaddleOCR cache dir test** — Fixed test failure when `KREUZBERG_CACHE_DIR` environment variable is set by CI setup actions.
- **API `pdf_password` handler** — Added `#[cfg(feature = "pdf")]` gate to prevent compile error when `api` feature is enabled without `pdf`.
- **Chunking page boundary regression** (#636): Page boundaries were computed against raw extractor text but `result.content` uses rendered text with different byte lengths. Chunks now recompute boundaries from per-page content, fixing `first_page`/`last_page` being null and the "Page boundary byte_end exceeds text length" validation warning.
- **HF Hub environment variables** (#634): Use `ApiBuilder::from_env()` instead of `ApiBuilder::new()` for Hugging Face model downloads, respecting `HF_HOME` and `HF_ENDPOINT` environment variables. Fixes permission errors on Kubernetes when running as non-root.
- **PDF bridge tracing panic on multibyte characters** (#635): Use `.chars().take()` instead of byte indexing for `text_preview` in PDF structure bridge tracing, preventing panics on multibyte UTF-8 characters (e.g., `•`).
- **Go FFI struct layout** — vendored C header was missing `children_json` field, causing 8-byte offset shift. All FFI fields after `chunks_json` read wrong memory (e.g., `ocr_elements_json` read `mime_type` instead).
- **Java FFI struct layout** — `CExtractionResult` layout was missing `code_intelligence_json` field, causing `success` flag to read from wrong offset. All Java extractions returned `success=false`.
- **PHP `__get` magic method bypass** — six JSON fields (`elements`, `djotContent`, `document`, `ocrElements`, `children`, `uris`) returned raw JSON strings instead of deserialized arrays because `#[php(prop)]` intercepted property access before `__get`.
- **Ruby `disable_ocr` config** — `disable_ocr` keyword was not parsed in Ruby config handler, causing OCR to run even when explicitly disabled.
- **Node.js `ExtractionResult` parity** — `document`, `djotContent`, and `ocrElements` fields were `Option<Value>` which NAPI-RS omitted from JS objects when `None`. Changed to `Value` defaulting to `null`.
- **Node.js `convertChunk` missing `chunkType`** — TypeScript type converter did not forward the `chunk_type` field from NAPI bindings.
- **ODT caption text extraction** — text inside `draw:frame > draw:text-box > text:p` (e.g., image captions) was not extracted. The ODT extractor now recurses into text-box content.
- **OCR InternalDocument propagation** — `run_ocr_pipeline` discarded the structured InternalDocument built by `extract_with_ocr`, causing OCR results to fall back to naive `\n\n` paragraph splitting. Now propagated through the full pipeline.
- **OCR table cells** — OCR-detected tables (via TATR) had empty `cells` vectors, causing comrak to render them as paragraphs instead of proper tables. Now populated from the cell grid, matching the native text path fix.
- **OCR non-layout InternalDocument** — When layout detection is not active, the OCR path now builds an InternalDocument from results instead of returning None. Ensures structured output regardless of layout detection availability.
- **Italian/European PDF ligature corruption** — Extended contextual ligature repair to handle `tt`, `ti`, `tti` ligatures common in Italian fonts. Fixes garbled text like `Dire*ore` → `Direttore`, `ges:one` → `gestione`, `progeM` → `progetti`.
- **OCR layout false heading classification** — Tesseract+layout pipeline was worse than pure tesseract (33% vs 41% SF1) because layout confidence threshold was too low (0.5). Raised to 0.7 for OCR path where font-size validation is unavailable.
- **OCR table rendering** — OCR-detected tables were not linked to InternalDocument elements, causing comrak to skip them entirely. Tables now properly registered via `push_table()` with corresponding `ElementKind::Table` elements.
- **Spurious table detection** — Multi-column prose with short cells (like nougat_008) bypassed the prose row check due to a 30-char minimum row length. Lowered to 15 chars so short-cell prose tables are correctly rejected.
- **PHP enum registration** — PHP enums (ContentLayer, ElementType, etc.) were registered with `.class()` instead of `.enumeration()`, causing empty case lists. Virtual properties on ExtractionResult and ArchiveEntry now declared via builder modifiers for reflection visibility.
- **Go macOS FFI linking** — monorepo dev build (`ffi_dev.go`) was missing `-framework Foundation` in CGO LDFLAGS, causing linker failures on macOS with CoreML-enabled ONNX Runtime.
- **Unified WASM e2e tests** — replaced broken separate Deno/Workers e2e generators with a single vitest-based WASM generator. ORT-dependent features (embeddings, layout, paddle-ocr) gracefully skip.
- **WASM Rayon thread pool panic** — Rayon's `par_iter()` / `into_par_iter()` and `ThreadPoolBuilder::build_global()` panicked in WASM (`RuntimeError: unreachable`) because WASM has no threading support. All Rayon usages now fall back to sequential iteration on `wasm32` target.
- **PHP virtual property reflection** — `ClassBuilder::property()` declarations for `__get`-backed fields (metadata, chunks, document, etc.) shadowed the magic method, returning null. Replaced with getter methods that don't interfere with `__get`. Parity test updated to check both `hasProperty()` and getter methods.

---

## [4.7.0] - 2026-03-30


### Added

- **Semantic chunk labeling** (#600): Chunks now include a `chunk_type` field identifying the semantic nature of the content (e.g., `paragraph`, `heading`, `list_item`, `table_cell`, `code_block`). Supported across all 11 language bindings with updated E2E test parity.
- **Unified InternalDocument architecture**: All extractors now return a canonical `InternalDocument` with typed elements, relationships, images, and tables. Replaces format-specific intermediate representations.
- **Unified rendering layer**: New `new_markdown.rs` renderer produces CommonMark from `InternalDocument`, supporting headings, lists, tables, code blocks, formulas, footnotes, images, and inline annotations (bold, italic, links).
- **PDF structure pipeline**: Full rewrite of PDF extraction using `page.text().all()` for clean text, char-indexed font metadata for heading/bold detection, segment-based paragraph gap detection, and pdfium segment bounding boxes for precise paragraph regions.
- **Image extraction across 8 formats**: Embedded images now extracted as `ExtractedImage` with binary data, format, dimensions, and alt text. Supported for DOCX, PPTX, PDF, EPUB, ODT, HTML (data URIs), RTF (hex-decoded), and Markdown/MDX/Jupyter. Markdown output renders as `![alt](image_N.ext)` with binary data in `ExtractionResult.images`.
- **Recursive OCR on embedded images**: When OCR is configured, extracted images from EPUB, ODT, HTML, and RTF are processed through `process_images_with_ocr()`, producing nested `ExtractionResult` in `ExtractedImage.ocr_result`.
- **PDF watermark artifact filtering**: Uses pdfium's `/Artifact` content marks (PDF tagged content spec) to identify and filter watermark text from output.
- **Vertical table header reconstruction**: Detects and fixes rotated column headers in PDF tables where pdfium extracts characters as spaced single characters in reverse order (e.g., "y t i r o h t u A o N" → "NoAuthority").
- **Position-based page furniture detection**: Cross-page repeating text detection now uses actual page margins (top/bottom 10%) and page heights instead of word-count heuristics.
- **html-to-markdown v3 migration**: Switched to html-to-markdown v3 with unified `convert()` API returning `ConversionResult` (content, metadata, tables, images, document structure in a single call). Uses visitor-based table collection. hOCR module vendored as `table_core`.
- **Markdown ground truth for 336 documents**: Pandoc-generated GT across 10 formats (DOCX, HTML, RTF, PPTX, EPUB, ODT, XLSX, XLS, CSV, DOC) for structural quality benchmarking. All 371 markdown GT files cleaned of HTML remnants (415 tables converted to GFM pipe tables, 28 inline tags fixed).
- **Multi-format benchmark support**: Pipeline benchmark now scores all document formats (not just PDF), shows file type per document, replaces NaN with "—", and reports ground truth loading errors.
- **Comprehensive PDF pipeline tracing**: Trace-level logging across heading lifecycle (layout overrides, demotion passes, furniture detection, render layer) for debugging.
- **Pages API for PDF extraction**: Per-page content now properly wired through the extraction pipeline via `prebuilt_pages` on `InternalDocument`, making `result.pages` available for PDF documents.
- **TOON wire format**: Token-Oriented Object Notation support across CLI (`--format toon`), API (`Accept: application/toon`), MCP (`response_format: "toon"`), and all 11 language bindings (Python, Node.js, WASM, C FFI, PHP, Ruby, Elixir, Go, Java, C#, R). TOON is a token-efficient alternative to JSON for LLM prompts — losslessly convertible to/from JSON but uses ~30-50% fewer tokens. Core functions `serialize_to_toon()` and `serialize_to_json()` exposed as public API.
- **Renderer registry**: Trait-based `Renderer` and `RendererRegistry` for custom output format plugins. Built-in renderers (markdown, HTML, djot, plain) registered at startup. External crates can register custom renderers (e.g., DOCX output) via `register_renderer()`.
- **comrak-based rendering**: Markdown and HTML rendering now uses comrak AST bridge instead of hand-rolled string building. Produces GFM-compliant markdown and semantic HTML5. Paragraph consolidation merges consecutive same-format paragraphs at sentence boundaries (fixes DOCX CV fragmentation where each visual line was a separate `*...*` italic block).
- **Benchmark quality scoring improvements**: Content normalization for HTML blocks in markdown scoring, Image↔Paragraph and Table↔ListItem type compatibility, `correct` field in `QualityMetrics`, HTML detection in ground truth validation.
- **Benchmark harness overhaul**: Per-format SF1/TF1 aggregation, noise detection (10 heuristics for HTML remnants, garbled text, broken tables, page artifacts), diagnostic diff mode (`--diagnose`), JSON output (`--json-output`), ground truth validation subcommand (`validate-gt`). Comprehensive tracing across all extractors and the rendering layer.
- **Markdown ground truth for 23 formats**: 350+ benchmark fixtures across CSV, DOCX, HTML, EPUB, LaTeX, RST, RTF, PPTX, ODT, XLSX, XLS, OPML, ORG, JATS, IPYNB, FictionBook, DocBook, Typst, DOC, PPT, and more. GT generated via pandoc and verified against source documents.
- **OpenWebUI integration**: Kreuzberg serves as a document extraction backend for Open WebUI chat interfaces.
- **URI extraction**: New `Uri` type with `UriKind` classification (Hyperlink, Image, Anchor, Citation, Reference, Email) extracted from 20+ document formats. URIs are always-on, deduplicated by (url, kind) pair, and capped at 100k per document. Available in `ExtractionResult.uris`.
- **Recursive email attachment extraction**: EML/MSG/PST attachments are now recursively extracted as `ArchiveEntry` children using the same pattern as archive extractors. Nested `message/rfc822` parts also extracted as children. Respects `max_archive_depth`.
- **PDF embedded file extraction**: PDF file attachments (portfolios) are now recursively extracted as `ArchiveEntry` children via lopdf. Includes filename sanitization, decompression size limits, and name tree depth guards.
- **PDF bookmark/outline extraction**: Document outlines (bookmarks) extracted as URIs — page destinations as `UriKind::Anchor`, external links as `UriKind::Hyperlink`.
- **DOCX/PPTX embedded object extraction**: OLE objects and embedded files from `word/embeddings/` and `ppt/embeddings/` directories are now recursively extracted as children.
- **PPTX hyperlink extraction**: Hyperlinks from slide XML (`<a:hlinkClick>` in run properties) now resolved via relationship files and extracted as URIs.
- **Image path resolution for markup formats**: When using `extract_file()`, relative image paths in Markdown, MDX, LaTeX, RST, OrgMode, Typst, Djot, and DocBook are resolved from the filesystem and extracted as `ExtractedImage` data. OS-agnostic with path traversal prevention.
- **Unified image OCR pipeline stage**: Image OCR moved from per-extractor calls to a single pipeline stage after derivation. All extracted images (including path-resolved markup images) are now OCR'd uniformly when OCR is configured. Concurrency limited to 8 concurrent tasks.
- **FictionBook image and link extraction**: Base64-encoded `<binary>` images and `<a>` hyperlinks now extracted from FB2 documents.
- **Apple iWork extractor improvements**: Numbers outputs tables instead of paragraphs, Keynote has improved slide structure, Pages has heading detection. All three extract metadata from ZIP plist.
- **`code_intelligence` field on ExtractionResult**: Top-level access to tree-sitter `ProcessResult` with full structure, imports, exports, chunks, symbols, diagnostics, and docstrings. Previously only available inside `FormatMetadata::Code` metadata.
- **`CodeContentMode` config**: Control code extraction content mode -- `chunks` (semantic TSLP chunks, default), `raw` (source as-is), `structure` (headings + docstrings only). Configured via `TreeSitterProcessConfig.content_mode`.
- **TSLP semantic chunking for code**: Code files bypass the text-splitter entirely. TSLP's `CodeChunks` (function/class-aware) map directly to kreuzberg `Chunk`s with semantic types and heading context.
- **Cross-format output parity tests**: 36 tests verifying Markdown, HTML, Djot, and Plain produce equivalent text content. GFM lint validation, bracket escaping checks, structural block comparison.
- **HTML input markdown passthrough**: HTML files extracted as Markdown now use html-to-markdown output directly via `pre_rendered_content`, bypassing the lossy InternalDocument to comrak round-trip.

### Code Intelligence

- **Tree-sitter integration** for 248 programming languages via [tree-sitter-language-pack](https://github.com/kreuzberg-dev/tree-sitter-language-pack)
  - Extract functions, classes, imports, exports, symbols, docstrings, diagnostics
  - Syntax-aware code chunking
  - Language detection from file extension and shebang
  - Dynamic grammar download (native) / 30-language static subset (WASM)
  - New `tree-sitter` and `tree-sitter-wasm` feature flags (included in `full` and `wasm-target`)
  - `TreeSitterConfig` and `TreeSitterProcessConfig` in `ExtractionConfig`
  - Re-exported TSLP types (`ProcessResult`, `StructureItem`, `FileMetrics`, etc.)
  - [TSLP documentation](https://docs.tree-sitter-language-pack.kreuzberg.dev)

### Typed Metadata

- New `FormatMetadata` variants: `Code`, `Csv`, `Bibtex`, `Citation`, `FictionBook`, `Dbf`, `Jats`, `Epub`, `Pst`
- Extended `PptxMetadata` with `image_count` and `table_count`
- Migrated deprecated `metadata.additional` writes to typed fields across all extractors
- Strong types for all new metadata variants across all 11 language bindings

### Breaking Changes

- **Layout detection preset removed**: The `preset` field on `LayoutDetectionConfig` has been removed across all bindings. Layout detection now uses the RT-DETR v2 model unconditionally — no "fast" vs "accurate" distinction. The `--layout-preset` CLI flag is removed. Old configs with `"preset": "..."` are silently ignored for backward compatibility.
- **Table model config typed**: `table_model` on `LayoutDetectionConfig` changed from `Option<String>` to a `TableModel` enum (`tatr`, `slanet_wired`, `slanet_wireless`, `slanet_plus`, `slanet_auto`, `disabled`). Defaults to `tatr`. String values still accepted in JSON/TOML configs.

### Fixed

- **PDF table rendering**: Populate `Table.cells` from TATR/SLANeXT grid so comrak renders proper Table nodes instead of wrapping markdown in a Paragraph. Table SF1 improved from 15.5% to 53.7%.
- **Markdown GFM quality**: Enable `prefer_fenced` for code blocks, un-escape brackets/parens (`\[` to `[`), fix code block language spacing in djot.
- **Semantic HTML output**: Enable `github_pre_lang` and `full_info_string` for code blocks with `class="language-X"`.
- **Djot text normalization**: Shared `normalize_inline_text()` for consistent whitespace handling. MD-to-Djot TF1 now 1.0000.
- **PDF structural extraction quality**: Improved heading detection (font-size-ratio H2/H3 differentiation, section numbering patterns, ALL-CAPS detection, paragraph-to-heading rescue pass), table discrimination (reject multi-column prose misclassified as tables via flow-through detection, row-count/column-count ratio, and table quality validation), list detection (multi-token prefix patterns), image scoring (normalize image block matching), and formula detection (math character density heuristic). Layout SF1 improved from 40.7% to 43.7% across 157 verified PDF fixtures.
- **PDF ground truth verified**: All 157 PDF benchmark fixtures verified using vision (rendered page images vs GT markdown). 7 broken Mistral OCR GTs with hallucinated content replaced with vision-verified markdown.
- **LaTeX extraction**: Convert `\href`, `\emph`, `\textbf`, `\textgreater`, `\verb`, `\sout`, blockquotes, lists, special characters, and typographic ligatures to markdown.
- **XLSX/XLS sheet name headings**: Emit `## SheetName` heading before each sheet's table, matching pandoc convention.
- **OPML outline headings**: All outline nodes now emit headings at appropriate depth, not just parent outlines. Inline HTML in text attributes converted to markdown.
- **IPYNB heading detection**: Markdown cells now detect ATX headings and emit proper heading elements. Code cell outputs (stdout, execute_result) included in extraction.
- **JATS abstract and references**: Abstract section with sub-headings now included. References rendered as numbered list with structured citation formatting.
- **ODT formula extraction**: Embedded MathML formula objects extracted as formula content instead of empty image placeholders. Image alt text and captions now extracted from `draw:frame` elements.
- **PPTX slide titles**: Title placeholders detected via OOXML placeholder type and emitted as H2 headings. Bulleted/numbered lists in slides extracted with proper ListStart/ListEnd wrapping.
- **ORG source blocks**: `#+BEGIN_SRC` blocks converted to fenced code blocks with language annotation. `#+BEGIN_EXAMPLE` blocks converted to unfenced code blocks. Inline code `~text~` converted to backtick spans. Paragraph line wrapping joined.
- **RST heading levels**: Overline+underline document titles assigned H1. Code block language hints preserved from `.. highlight::` and `.. code::` directives. `::` literal block shorthand handled.
- **RTF formatting**: Bold/italic/strikethrough formatting now uses exact byte offsets from a unified text+formatting extraction pass, eliminating bold bleeding across paragraphs. Hidden text (`\v`) suppressed. Hyperlink field parsing fixed. Strikethrough support added. Table row rendering fixed for multi-row tables. Ordered list detection from `\listtext` markers.
- **HTML preprocessing**: Navigation elements, forms, and sidebars now stripped by default. Previously disabled, causing page chrome to appear in extraction output.
- **PDF table detection**: Reject false table detections where >70% of cells contain single-word fragments (justified prose incorrectly classified as multi-column table).
- **DocBook root element handling**: XML fragments without a root element now wrapped automatically, fixing extraction of multi-element DocBook files.
- **FictionBook poem support**: Verse lines (`<v>`), subtitles, text-author, and date elements within poem blocks now extracted. Heading levels aligned with pandoc conventions.
- **PDF image FlateDecode fallback**: When `decode_flate_to_png()` fails for FlateDecode, CCITT, or JBIG2 streams, images are now re-extracted via pdfium's bitmap rendering pipeline, producing valid PNG output instead of unusable raw bytes (#615).
- **Metadata standardization**: Metadata from PPTX, Excel, ODT, RST, OrgMode, Typst, RTF, JATS, DOC, PPT, HTML, Email, BibTeX, and Citation extractors now mapped to standard `Metadata` struct fields (title, authors, dates, keywords, language) instead of only `additional` map.
- **MDX link parity with Markdown**: Links and annotations in headings and list items now extracted (was silently dropped).
- **RST hyperlink extraction**: Inline hyperlinks (`` `text <url>`_ ``) and reference targets now extracted.
- **LaTeX `\url{}` extraction**: `\url{...}` commands now extracted as URIs alongside `\href`.
- **OrgMode image detection**: Added .webp, .bmp, .tiff, .avif to recognized image extensions.
- **BibTeX URI classification**: URL fields now correctly classified as Hyperlink (was Citation). Entry title used as label instead of BibTeX key.
- **JATS title field**: Article title now stored in `metadata.title` (was only in `subject`).
- **PDF bookmark stack safety**: Sibling traversal converted from recursion to iterative loop preventing stack overflow on wide outlines.
- **PDF embedded file security**: Filename sanitization (strip directory components), decompressed size limit (50MB), name tree depth limit (50 levels).

- **Tesseract C++ exception crash** (#606): Fixed fatal runtime error where C++ exceptions from Tesseract unwound through Rust FFI frames, triggering `std::terminate()`. Now compiles Tesseract with `-fno-exceptions` on macOS, Linux, and MinGW. The Tesseract CLI executable target (which uses `try`/`catch`) is patched out of CMakeLists.txt at build time since only the library is needed.

- **ExtractionConfig rejects unknown fields**: `#[serde(deny_unknown_fields)]` added to `ExtractionConfig`. Previously, typos or invalid fields (e.g., `layout_analysis` instead of `layout`) were silently ignored.
- **RTF delimiter space consumption**: Fixed space-in-word bug where font encoding directives (`\loch`, `\hich`, `\dbch`) caused spaces mid-word ("H eading" → "Heading"). Root cause: RTF spec requires consuming trailing delimiter space after control words.
- **PPTX markdown mode**: Derive plain/markdown mode from `output_format` config instead of hardcoding `plain=true`. Tables now render as markdown tables, lists get bullet markers, text elements get newline separation.
- **EPUB test compilation**: Added `InternalDocument::content()` method and fixed `epub_spine_semantics_tests` to use it instead of removed `.content` field.
- **HTML extraction rewrite**: Replaced ~400-line manual HTML tag parser with html-to-markdown v3's `DocumentStructure` mapping. Single-pass conversion eliminates CSS/script content leakage and `[image: X]` placeholder artifacts.
- **Chunking heading context with plain output**: Fixed `heading_context` always returning `None` when using plain text output format. The markdown chunker now receives the original markdown for heading map building even when content is rendered as plain text.
- **WASM build compatibility**: Inlined workspace-inherited fields (`version`, `edition`, `authors`) in kreuzberg-wasm Cargo.toml because wasm-pack 0.14.0 cannot resolve `field.workspace = true` references.
- **Pre-commit hooks**: Fixed rumdl hook config (use `rumdl-fmt` from official repo), wasm build (feature-gate layout config access), kreuzberg-node build (missing `formatted_content` field), broken relative links in READMEs and CHANGELOG.
- **Binding compilation**: Added missing `formatted_content` field to kreuzberg-py and kreuzberg-php binding crates.
- **PDF heading body_size_guard**: Narrowed guard range from `≤ body+0.5` to `body±1.5pt` so headings well below body font size (e.g., 8pt in 12pt body) pass through.
- **RTF table extraction**: Fixed critical bug where table cell content was written to both result string and TableState, causing cells to appear as individual lines instead of proper markdown tables.
- **DOCX merged cells**: Repeat content across gridSpan (horizontal) and vMerge (vertical) spans. Added `source_path` field to `ExtractedImage` for DOCX image relationship paths.
- **DOCX formatting**: Merge adjacent runs with identical formatting to prevent spurious `****` sequences. Strip `<u>` underline HTML tags.
- **Python wheel `__isoc23_strtoll` error on older Linux distributions** (#588): Downgraded the Linux build environment `manylinux` target from `manylinux_2_39` to `manylinux_2_28` for pre-compiled Python wheels to ensure compatibility with systems using glibc versions prior to 2.39 (e.g., Ubuntu 20.04/22.04, Debian 11/12).
- **`clear_ocr_backends` now fully clears the registry**: Calls `shutdown_all()` instead of `reset_to_defaults()`, so the backend list is empty after clearing as expected by the API contract.
- **Go macOS link failure**: Added missing `-framework Foundation` to CGO LDFLAGS. ORT's CoreML provider uses Foundation for NSLog/NSFileManager, causing undefined symbol errors on macOS.
- **Tesseract Windows MinGW build (Elixir/Go/C FFI publish)**: CMake resolved bare `g++` to MSVC `cl.exe` on CI runners with both toolchains. Added `resolve_mingw_compiler()` to find absolute paths from MSYS2 subsystem dirs. Bumped Tesseract cache key to invalidate stale MSVC-compiled artifacts.
- **Windows GNU ORT linking**: `bundled` strategy on Windows GNU now uses dynamic linking with pre-downloaded Microsoft ORT (pyke.io has no static binaries for `x86_64-pc-windows-gnu`). Documented ONNX Runtime DLL requirement for Go, Elixir, and C/C++ on Windows.

### Changed

- **PDF text extraction**: Full rewrite from segment-indexed assembly to `page.text().all()` + char-indexed font metadata. Produces cleaner text with correct word spacing.
- **hOCR table reconstruction vendored**: `HocrWord`, `reconstruct_table`, `table_to_markdown` moved from `html-to-markdown-rs::hocr` to `kreuzberg::table_core` module.
- **CLI format flags**: `--format` (`-f`) now supports `text`, `json`, and `toon` wire formats. `--output-format` renamed to `--content-format` (deprecated alias kept with warning). `OutputFormat` enum gains `Custom(String)` variant for extensible format plugins.
- **html-to-markdown-rs v3.0.0**: Switched from git dependency to crates.io release.
- **License policy**: MPL-2.0 and LGPL-2.1 no longer globally allowed — pinned to specific crate exceptions (cbindgen, option-ext, r-efi). Unicode-DFS-2016 allowed for comrak dependency.

### Removed

- **`max_upload_mb` server config field**: Use `max_multipart_field_bytes` (in bytes) instead. The `KREUZBERG_MAX_UPLOAD_SIZE_MB` environment variable is also removed — use `KREUZBERG_MAX_MULTIPART_FIELD_BYTES`.
- **`metadata.additional` legacy insertions**: Pipeline features (chunking, embeddings, language detection, keywords) no longer insert error/status keys into `metadata.additional`. Errors are available via `processing_warnings`. Keywords are in `extracted_keywords`. Embedding status is derivable from chunk embeddings.
- **`derive_content_string` function**: Replaced by `render_plain()` in the rendering module.

---

## [4.6.3] - 2026-03-27

### Added

- **Tower service layer** (`service` module): Composable `ExtractionService` implementing `tower::Service` with configurable middleware layers (tracing, metrics, timeout, concurrency limit). New `tower-service` feature flag, auto-enabled by `api` and `mcp`. `ExtractionServiceBuilder` provides ergonomic layer composition.
- **Semantic OpenTelemetry conventions** (`telemetry` module): Formal `kreuzberg.*` attribute namespace with 30+ span attributes, metric names, and operation/stage constants. Documented conventions for document extraction, pipeline stages, OCR, and model inference telemetry.
- **Extraction metrics**: 11 OTel metric instruments (counters, histograms, gauge) covering extraction totals, durations, cache hits/misses, pipeline stages, OCR, and concurrent extractions. Feature-gated behind `otel`.
- **InstrumentedExtractor wrapper**: Automatic per-extractor tracing spans and metrics without per-extractor annotations. Injected at registry dispatch when `otel` feature is enabled.

### Improved

- **Deeper instrumentation**: Pipeline post-processing stages (Early/Middle/Late), individual processor execution, OCR operations, and RT-DETR layout model inference now have semantic spans and duration metrics.
- **API and MCP servers use ExtractionService**: Both consumers now route extractions through the Tower service stack, getting unified tracing, metrics, and middleware for free.
- **Unified config merge**: JSON config merge logic deduplicated between CLI and MCP into a shared function.
- **API server hardening**: Added response compression (gzip/brotli/zstd), panic recovery, request-ID correlation, and sensitive header redaction via tower-http middleware.

### Changed

- **Removed per-extractor `#[instrument]` annotations**: 29 manual `#[cfg_attr(feature = "otel", tracing::instrument(...))]` annotations replaced by the automatic `InstrumentedExtractor` wrapper.
- **Span attribute names migrated to `kreuzberg.*` namespace**: `extraction.filename` -> `kreuzberg.document.filename`, `extraction.mime_type` -> `kreuzberg.document.mime_type`, etc.

### Fixed

- **EPUB spine semantics refactor** (#594): Richer OPF package model preserves manifest fallback chains, guide references, and non-linear spine items. Navigation chrome stripped from output. Malformed guide references now produce warnings instead of hard failures. Tested for fallback cycles and empty spines.
- **DOCX image extraction for `<a:blip>` with child elements** (#591): Images with high-quality settings (containing `<a:extLst>` children) were not extracted because only `Event::Empty` was handled. Now also handles `Event::Start` for `<a:blip>`.
- **OCR table extraction returned empty results via pipeline path** (#593): Layout detection was gated behind a `needs_structured` check, skipping it for the default `Plain` output format. Tables from `run_ocr_pipeline` were discarded. Both paths now propagate tables correctly.
- **Missing `chunker_type` field in bindings** (#592): Exposed `chunker_type`, `sizing_cache_dir`, and `prepend_heading_context` fields across Python, TypeScript/WASM, Go, C#, PHP bindings.
- **Full API parity across all 10 bindings**: Added `max_archive_depth` to all bindings. Added missing `acceleration`, `email` to Ruby/R. Added `layout` to PHP. Added 7 missing fields to WASM. Fixed parity script regex for Go slice types.
- **`test_pipeline_with_all_features` assertion without `quality` feature**: `quality_score` assertion now gated behind `#[cfg(feature = "quality")]`.
- **Node Windows publish failure**: Prepare script fallback used bash-specific `mkdir -p` and `echo >` which fail on Windows. Replaced with cross-platform `node -e` fallback.
- **CI Validate path triggers too narrow**: Broadened glob patterns to cover `docs/**`, `biome.json`, `.task/**`, and other lintable paths that prek hooks check.
- **Publish pipeline ORT bundling**: Added configurable `strategy` input (`system`/`bundled`) to `setup-onnx-runtime` action. Set `strategy: bundled` for all publish jobs so `ort-bundled` cargo feature takes effect, producing self-contained binaries.

---

## [4.6.2] - 2026-03-26

### Added

- **PDF page rendering API** (#583): New `render_pdf_page` function and `PdfPageIterator` for rendering individual PDF pages as PNG images. Available across all 11 language bindings with idiomatic patterns (Python context manager, Go Close(), Java AutoCloseable, C# IDisposable, Elixir Stream, etc.). Default 150 DPI, configurable per call.

### Fixed

- **Table recognition coordinate mismatch on scanned PDFs** (#582): Layout detection bboxes (640x640 model space) are now scaled to OCR render resolution before TATR table recognition. Previously, coordinate space mismatch caused zero tables to be found.
- **OCR elements report `page_number: 1` for all pages** (#582): Tesseract resets page numbers per single-page render. Page numbers are now correctly stamped after OCR in the batch loop.
- **Rust E2E tests missing PDF feature**: Added `pdf` feature to the e2e-generator Rust template, fixing 41 `UnsupportedFormat("application/pdf")` failures.
- **HWP styled extraction empty on ARM**: Added `skip_on_platform` support to Python and Java e2e generators, skipping the `hwp_styled` fixture on `aarch64-unknown-linux-gnu`.
- **WASM CI build failure**: Made `kreuzberg-node` prepare script resilient to missing native addon, preventing `ENOENT: dist/cli.js` during pnpm workspace install.
- **Go C header stale at 4.5.0**: Synced header and `DefaultVersion` constant to match current version.
- **Ruby gem missing ONNX Runtime**: Added `ort-bundled` feature to Ruby native Cargo.toml.
- **Elixir doctest failures**: Updated `ExtractionConfig.to_map/1` doctests for `force_ocr_pages` field.
- **WASM benchmark timeout**: Reduced per-extraction timeout from 600s to 120s and job timeout from 6h to 2h.

### Improved

- **`version:sync` now syncs Go C header, DefaultVersion, and Docker compose tags**: Prevents version drift across language bindings.
- **Publish pipeline commits Elixir NIF checksums back to main**: Prevents stale checksums after releases.
- **WASM test app migrated to Deno**: Replaced Node.js/vitest with Deno test runner, fixing `fetch()` unavailability.
- **Docs migrated from MkDocs to Zensical**: 4-5x faster incremental builds.

---

## [4.6.1] - 2026-03-25

### Added

- **Per-file batch extraction timeouts** (#546): New `extraction_timeout_secs` on `ExtractionConfig` (batch-level default) and `timeout_secs` on `FileExtractionConfig` (per-file override). Timeouts apply after semaphore acquisition. New `KreuzbergError::Timeout` variant with `elapsed_ms` and `limit_ms` fields. All binding layers updated.
- **Page-level OCR overrides** (#432): New `force_ocr_pages` option (1-indexed) on both `ExtractionConfig` and `FileExtractionConfig`. Enables selective OCR on specific pages of mixed-quality PDFs while preserving native text on others.
- **PST extraction support** (#502): Extract emails from Microsoft Outlook PST archives via the `outlook-pst` crate. Iterative depth-first folder traversal with depth cap of 50. Feature-gated under `email`.
- **JSONL/NDJSON extraction** (#575): Native `.jsonl`/`.ndjson` extraction via `StructuredExtractor`. Registered as `application/x-ndjson` MIME type.

### Fixed

- **OCR elements now propagated to ExtractionResult** (#566): OCR elements with geometry data are collected during extraction and set on `ExtractionResult.ocr_elements`. Hierarchy transformer emits body-level blocks as `NarrativeText` elements with coordinates. OpenAPI schema registers OCR-related types.
- **OOM crash on multi-page scanned PDFs** (#570): Replaced pre-rendering all PDF pages into memory with batched rendering. Pages are now rendered and OCR'd in bounded batches, capping peak memory to `batch_size * page` instead of `page_count * page`.
- **OCR memory usage reduced 60-78%**: Restructured the OCR batch rendering loop to render-and-encode one page at a time instead of holding all decoded RGB buffers simultaneously. A 98-page scanned PDF dropped from 4.6GB to 1.9GB peak RSS (batch_size=4), and from 3.3GB to 713MB (batch_size=1). Batch size now adapts to available system memory on Linux and macOS.
- **PDF control character encoding artifacts**: PDFs with broken ToUnicode font mappings that produce U+0002 (STX) and other control characters where hyphens should appear now have these replaced with hyphens when between word characters, or stripped otherwise. Fixes garbled output like `re\x02labelling` → `re-labelling`.
- **DocumentStructure missing Heading nodes for PDFs**: `push_heading_group` now inserts a `Heading` child inside each `Group` node (matching DOCX builder behavior). Fallback `add_paragraphs` now detects markdown heading markers and creates heading groups instead of flat paragraphs.
- **Layout detection returns empty tables on scanned PDFs** (#574): Three independent bugs caused `result.tables` to always be `[]` for scanned/image-based PDFs: (1) layout detection was gated behind a `needs_structured` output-format check, silently skipping detection for `Plain` (the default); (2) TATR-recognized tables in the OCR path were inlined as markdown text but never converted to `Table` structs; (3) `run_ocr_with_layout` returned only text, discarding table data. All three paths now propagate tables correctly.
- **Table recognition coordinate mismatch on scanned PDFs** (#582): Layout detection operates at 640×640 pixels but TATR table recognition and layout-hint classification consumed those coordinates verbatim against OCR-rendered images (e.g. 2480×3508 px at 300 DPI). Bounding boxes never overlapped OCR word positions, producing zero recognized tables and incorrect paragraph-class overrides. Bounding boxes are now scaled from layout-model resolution to the actual OCR render resolution before both `recognize_page_tables` and `detection_to_layout_hints` are called.
- **OCR elements report `page_number: 1` for all pages** (#582): The Tesseract backend resets `page_number` to 1 for every single-page render. The page-number is now stamped with the correct 1-indexed page index after collecting each batch page's OCR elements.
- **PDF layout engine panic on malformed input** (#544): Replaced the panicking `.expect()` inside the thread-local `LayoutEngine` initializer in `layout_runner.rs` with proper `Result`-based error propagation. A failure to initialise the layout engine now returns a descriptive error instead of crashing the host process via FFI (Python, Node, etc.).

---

## [4.6.0] - 2026-03-24

### Added

- **Recursive archive extraction**: Archives (ZIP, TAR, 7Z, GZIP) now recursively extract all processable files, each with its own `ExtractionResult` including `DocumentStructure`, annotations, and metadata. New `ArchiveEntry` type with path, mime type, and nested result. Configurable via `max_archive_depth` (default: 3, set to 0 for legacy single-text behavior).
- **YAML/JSON section chunker**: New `ChunkerType::Yaml` variant that splits structured files by keys with full hierarchy paths (e.g., `database > primary > host`). Auto-inferred from extraction metadata — no explicit `chunker_type` needed for YAML/JSON files.
- **Unified DocumentStructure DTO**: Extended the `DocumentStructure` model with 7 new node types (`Slide`, `DefinitionList`, `DefinitionItem`, `Citation`, `Admonition`, `RawBlock`, `MetadataBlock`), 4 new annotation kinds (`Highlight`, `Color`, `FontSize`, `Custom`), and format-specific `attributes` bag on every node.
- **DocumentStructureBuilder**: Ergonomic builder with heading-driven section nesting, container stack (Quote/Admonition/Slide auto-parenting), and annotation helpers. Replaces hand-constructed `DocumentNode` structs across all extractors.
- **Unified rendering module**: `render_to_markdown()` and `render_to_plain()` renderers that walk a `DocumentStructure` tree to produce consistent output with inline annotation rendering, table pipe escaping, and nested list depth support.
- **DocumentStructure support for all extractors**: Every extractor (35 formats) now natively produces a `DocumentStructure` when `include_document_structure` is enabled:
  - Office: DOCX (with TextAnnotation from Run formatting, Formula from OMML), PPTX (Slide containers), ODT, DOC, PPT
  - Markup: HTML (1,100-line tag parser with inline annotations), LaTeX, RST (admonitions, definition lists), OrgMode, Markdown, MDX, Djot, Typst
  - Books: EPUB (chapter structure from spine), FictionBook (inline formatting annotations)
  - Scientific: JATS (article structure), DocBook (section hierarchy)
  - Data: Excel (sheet headings + tables), CSV, DBF, JSON/YAML/TOML, BibTeX (citations), Jupyter (code + markdown cells)
  - Other: Email (metadata headers), RTF, OPML (outline hierarchy), HWP, iWork (Keynote/Numbers/Pages), XML, Image (OCR text)
- **DocBook/JATS inline annotations**: Semantic inline formatting for academic/technical documents — emphasis, bold, code, links, subscript/superscript mapped to `AnnotationKind` variants.
- **Document-level OCR**: `OcrBackend` trait supports `process_document()` for whole-file extraction without per-page rasterization. Up to 30% faster on multi-page documents with better context.

### Changed

- **CSV extraction for embedding quality**: Produces `Row N: Header: Value` format instead of space-separated when a header row is detected. Programmatic `tables` field unchanged.
- **XML extraction for embedding quality**: Indented hierarchical output preserving element tree with attributes inline, blank lines between top-level siblings, and `xmlns:*` filtering.

### Improved

- **Zero-copy file I/O**: Automatic memory-mapping for files >1MB via `memmap2` with SIMD-accelerated UTF-8 validation (`simdutf8`). Measurable speed improvement for large PDFs and archives. WASM falls back to heap allocation.
- **Unified concurrency management**: Centralized thread budget for Rayon, ONNX, and PaddleOCR with configurable `ConcurrencyConfig`. PDF OCR batched in chunks instead of all-at-once, reducing memory footprint on large documents.

### Fixed

- **Incorrect page numbers in element-based output** (#557): When `result_format="element_based"` was used without `PageConfig(extract_pages=True)`, all elements received `page_number=1`. Now auto-enables `extract_pages` when element-based output is requested.
- **Misleading `PageConfig` docstring** (#558): Updated docstring and type stub to show default constructor first and document interaction with `result_format="element_based"`.
- **MSG extraction misses compressed RTF bodies** (#560): Added PR_RTF_COMPRESSED (0x1009) fallback for `.msg` files that store the body only in compressed RTF format. Implements MS-OXRTFCP decompression and RTF-to-plain-text stripping.
- **Indexed colour PDF images returned as raw** (#561): Palette-based PDF images now decode correctly. Extracts the colour palette from the PDF dictionary and applies palette lookup to produce valid PNG output instead of unusable raw bytes.
- **ODT extraction robustness**: Replaced unwraps with safe fallbacks in ODT parsing.

---

## [4.5.4] - 2026-03-23

### Added

- **Document-level OCR optimization**: The `OcrBackend` trait now supports native `process_document()` for efficient whole-file extraction without rasterizing individual PDFs to images when the backend supports it (e.g., Python's EasyOCR backend).

### Changed

- **OCR protocol clarity**: Differentiated `process_file` to `process_image_file` in OCR backend trait for clearer protocol semantics.
- **Python refactoring**: Removed unused loop variable in EasyOCR implementation.
- **Dependency optimization**: Dropped redundant tokio multi-thread feature flag.

### Tests

- **Backend registry robustness**: Hardened backend registry tests with drop guards and comprehensive mock coverage.

### Added

- **PST (Outlook Personal Folders) extraction**: New `PstExtractor` backed by the `outlook-pst` crate. Traverses the full IPM folder hierarchy iteratively, extracts subject, sender, recipients (TO/CC/BCC), body, and date from every message in the archive. Enabled via the existing `email` feature flag. MIME type: `application/vnd.ms-outlook-pst`.

### Fixed

- **PDF image extraction panic on mismatched buffer lengths** (#552): Replaced `assert!` in `pdf/images.rs` with graceful error handling. Malformed PDF images with wrong buffer sizes are now skipped instead of panicking. Regression from v4.5.0.
- **`pdf` feature compilation without `layout-detection`** (#550): `config.layout` reference in `extraction.rs` was not behind a `#[cfg(feature = "layout-detection")]` gate, causing compilation errors when `pdf` was enabled without `layout-detection`.
- **Unused `table_model` variable warning**: Fixed cfg-gating in `pipeline.rs` so `table_model` parameter is properly handled when `layout-detection` feature is disabled.
- **Clippy `too_many_arguments` on `recognize_tables_slanet`**: Added allow attribute for the 8-parameter function in `table_recognition.rs`.
- **Ruby binding missing `table_model` field**: Added `table_model` parsing to `LayoutDetectionConfig` initializer in Ruby native extension.
- **WASM module resolution in Supabase/Deno edge functions** (#551): Added explicit `package.json` exports for `pkg/kreuzberg_wasm.js` and WASM binary. Extended `wasm-loader.ts` with Deno detection and clear error messaging for restricted edge runtimes.
- **`zip` dependency pinned below 7.4**: Avoids let-chain build failures on some stable Rust toolchains (#549).
- **Vendored HWP text extraction**: Replaced external `hwpers` crate with vendored subset (~1,650 lines). Eliminates `zip 2.x` transitive dependency that caused WASM and CI Validate build failures.

### Added

- **`prepend_heading_context` chunking option**: When `true` and `chunker_type` is `Markdown`, prepends the heading hierarchy path (e.g. `# Title > ## Section`) to each chunk's content string. Useful for RAG pipelines where chunks need self-contained structural context. Available across all 10 language bindings, CLI, and WASM. Includes fixture-driven e2e tests and documentation for all languages.

---

## [4.5.3] - 2026-03-22

### Added

- **Apple iWork Format Support**: Native parsing for modern (2013+) `.pages`, `.numbers`, and `.key` files via a new `iwork` feature flag. Uses zero-allocation protobuf text extraction from Snappy-compressed IWA containers.
- **SLANeXT table structure recognition models**: Alternative table structure backends alongside TATR. New `table_model` field on `LayoutDetectionConfig` selects the backend. Options: `"tatr"` (default, 30MB), `"slanet_wired"` (365MB, bordered tables), `"slanet_wireless"` (365MB, borderless tables), `"slanet_plus"` (7.78MB, lightweight), `"slanet_auto"` (classifier-routed, ~737MB). Available across all 12 language bindings and CLI (`--layout-table-model`).
- **PP-LCNet table classifier**: Automatic wired/wireless table detection for SLANeXT auto mode. Uses center-crop preprocessing with BGR channel order matching PaddleOCR convention.
- **CLI `cache warm --all-table-models`**: Opt-in download of SLANeXT model variants (~730MB). Default warm downloads only RT-DETR + TATR.
- **ISO 21111-10 benchmark fixture**: Table-heavy ISO standard document with MinerU ground truth for table extraction benchmarking.

---

## [4.5.2] - 2026-03-21

### Fixed

- **PDF word splitting in extracted text**: Pdfium's text extraction inserted spurious spaces mid-word (e.g. `"s hall a b e active"` instead of `"shall be active"`). Added selective page-level respacing: pages with detected broken word spacing are re-extracted using character-level gap analysis (`font_size × 0.33` threshold). Clean pages use the fast single-call path. Reduces garbled lines from 406 to 0 on the ISO 21111-10 test document with no performance impact.
- **Markdown underscore escaping**: Underscores in extracted text (e.g. `CTC_ARP_01`) were incorrectly escaped as `CTC\_ARP\_01` throughout the markdown output. Underscore escaping has been removed entirely since extracted PDF text contains literal identifiers, not markdown formatting.
- **Page header/footer leakage**: Running headers like `ISO 21111-10:2021(E)` and copyright footers leaked into the document body. Added fuzzy alphanumeric matching to detect repeated header/footer text even when spacing or character extraction varies across pages.
- **R batch function spurious NULL argument**: R wrapper batch functions passed an extra `NULL` positional argument to native Rust functions, causing "unused argument" errors on all batch operations.
- **Elixir Windows ORT DLL staging**: ONNX Runtime DLL was only staged in `target/release/` but not in `priv/native/` where the BEAM VM loads NIFs. OCR/layout/embedding features now work correctly on Windows CI.

### Added

- **General extraction result caching**: All file types (PDF, Office, HTML, archives, etc.) are now cached — not just OCR results. Repeated extractions of the same file with the same config return instantly from cache.
- **Cache namespace isolation**: New `cache_namespace` field on `ExtractionConfig` enables multi-tenant cache isolation on shared filesystems. Available via `--cache-namespace` CLI flag and across all language bindings.
- **Per-request cache TTL**: New `cache_ttl_secs` field on `ExtractionConfig` overrides the global TTL for individual extractions. Set to `0` to skip cache entirely. Available via `--cache-ttl-secs` CLI flag.
- **Cache namespace deletion**: `delete_namespace()` removes all cache entries under a namespace. `get_stats_filtered()` returns per-namespace statistics.
- **Multi-worker cleanup safety**: Cache cleanup no longer triggers excessively when multiple worker pods share the same cache directory.
- **Bundled eng.traineddata**: English OCR works out of the box with zero runtime configuration (~4MB bundled at build time).
- **Tessdata in `cache warm`**: `kreuzberg-cli cache warm` now downloads all tessdata_fast language files (~120 languages) to `KREUZBERG_CACHE_DIR/tessdata/`, giving full Tesseract language support without system packages.
- **Tessdata in `cache manifest`**: `kreuzberg-cli cache manifest` now includes all tessdata files with source URLs, enabling `--sync-cache` to download tessdata alongside models.
- **`KREUZBERG_CACHE_DIR/tessdata` resolution**: `resolve_tessdata_path()` now checks `KREUZBERG_CACHE_DIR/tessdata` and the bundled build path before falling back to system paths. Resolution order: `TESSDATA_PREFIX` env → `KREUZBERG_CACHE_DIR/tessdata` → bundled tessdata → system paths.
- **CLI `embed` command**: Generate vector embeddings from text via `kreuzberg embed --text "..." --preset balanced`. Supports stdin, multiple texts, JSON/text output. Feature-gated on `embeddings`.
- **CLI `chunk` command**: Split text into chunks via `kreuzberg chunk --text "..." --chunk-size 512`. Configurable size, overlap, chunker type, tokenizer model.
- **CLI `completions` command**: Generate shell completions for bash, zsh, fish, powershell via `kreuzberg completions <shell>`.
- **CLI `--log-level` global flag**: Override `RUST_LOG` via `kreuzberg --log-level debug extract doc.pdf`.
- **CLI extraction overrides**: 27 flags exposed via `ExtractionOverrides` struct with `#[command(flatten)]`. New flags: `--layout-preset`, `--layout-confidence`, `--acceleration`, `--extract-pages`, `--page-markers`, `--extract-images`, `--target-dpi`, `--pdf-extract-images`, `--pdf-extract-metadata`, `--token-reduction`, `--include-structure`, `--max-concurrent`, `--max-threads`, `--msg-codepage`, `--ocr-auto-rotate`.
- **CLI colored output**: Text output uses `anstyle` for colored headers, labels, success values, and dim separators. Respects `NO_COLOR` env var.
- **API `POST /detect`**: MIME type detection endpoint via multipart file upload.
- **API `GET /version`**: Version info endpoint.
- **API `GET /cache/manifest`**: Model manifest with checksums and sizes.
- **API `POST /cache/warm`**: Eager model download endpoint with embedding preset support.
- **MCP `get_version` tool**: Query server version from MCP clients.
- **MCP `cache_manifest` tool**: Get model manifest via MCP.
- **MCP `cache_warm` tool**: Pre-download models via MCP.
- **MCP `embed_text` tool**: Generate embeddings via MCP (feature-gated).
- **MCP `chunk_text` tool**: Text chunking via MCP.
- **Pipeline table extraction tracing**: Added zero-cost `tracing::trace!` and `tracing::debug!` logging throughout the layout detection and table extraction pipeline for easier debugging.
- **TATR model availability check**: Layout detection now returns an error if table regions are detected but the TATR model is unavailable, instead of silently falling back to degraded extraction.
- **Publish idempotency checks**: All publish jobs now have re-check steps using `check-registry@v1` before publishing. Added `check-elixir-release` job for GitHub release asset verification.
- **ARM benchmark runners**: Benchmark workflows switched to `runner-medium-arm64` for ARM-native performance testing.
- **Registry check tool**: `python3 scripts/publish/check_all_registries.py <version>` checks all 10+ registries and GitHub release assets locally.

### Changed

- **CLI batch flags**: Batch command now supports all extraction override flags (chunking, layout, acceleration, etc.) via shared `ExtractionOverrides` struct, matching extract command parity.
- **CLI config architecture**: Replaced 13-parameter `apply_extraction_overrides` function with `ExtractionOverrides` struct using `#[command(flatten)]`. Config fields auto-scale as `ExtractionConfig` evolves.
- **MCP tool architecture**: Removed dead `tools/` trait-based duplicates; all tools implemented directly in `server.rs`.

### Improved

- **CLI validation**: OCR backend values validated (tesseract, paddle-ocr, easyocr). Chunk size/overlap bounds checked. DPI range (36-2400) and layout confidence (0.0-1.0) validated. Zero-value `max_concurrent`/`max_threads` rejected. `--chunking-tokenizer` errors when feature disabled.
- **API validation**: Embedding preset names validated in `/embed`. Chunk `max_characters` bounds checked (1-1M) in `/chunk`.
- **MCP validation**: Empty paths rejected in `batch_extract_files`. Chunk `max_characters` bounds checked in `chunk_text`. Embedding preset validated in `embed_text`.
- **Chunk overlap auto-clamping**: When `--chunk-size` is smaller than default overlap, overlap is automatically clamped to `size/4` instead of producing a confusing error.

---

## [4.5.1] - 2026-03-20

## [4.5.1] - 2026-03-20

- **Java FFI `CBatchResult` struct layout mismatch**: The `count` and `results` fields were swapped in the Java Panama FFM layout, causing all batch extraction operations to fail with memory access errors.
- **Go FFI stale C header**: The `CExtractionResult` struct field order in the Go binding's C header did not match the Rust `#[repr(C)]` layout (reordered alphabetically in 4.5.0, added `djot_content_json`). Go read fields at wrong offsets, causing `pages_json` to deserialize `metadata_json` instead.
- **FFI `LayoutDetectionConfig` not feature-gated**: The FFI crate unconditionally imported `LayoutDetectionConfig` and exposed `kreuzberg_config_builder_set_layout`, causing compilation failures on targets without the `layout-detection` feature (e.g., `x86_64-pc-windows-gnu`).
- **Python wheel builds on Linux aarch64**: OpenSSL library path was hardcoded to `x86_64-linux-gnu` in the manylinux build script, failing on aarch64 runners. Now detects architecture via `uname -m`.
- **R batch function signature mismatch**: R wrapper functions were missing the `file_configs` parameter when calling native Rust functions, causing "Expected Scalar, got Language" errors on all batch operations.
- **R package ORT linking**: The R build configuration (`config.R`) did not link against ONNX Runtime when `ORT_LIB_LOCATION` was set, causing `undefined symbol: OrtGetApiBase` at load time.

---

## [4.5.0] - 2026-03-20

### Added

- **ONNX-based document layout detection**: New `layout` config field enables document layout analysis using RT-DETR v2 with 17 element classes. Supports `"fast"` and `"accurate"` presets with auto-downloaded models. Available across all language bindings.
- **SLANet table structure recognition**: Detected Table regions are processed by SLANet-plus for neural HTML structure recovery, producing markdown tables with colspan/rowspan support. Now runs on all pages including structure-tree pages (previously skipped).
- **Layout-enhanced heading detection**: Layout model SectionHeader and Title regions guide heading detection in both structure tree and heuristic extraction. High-confidence hints (>=0.7) can override font-size-based classification.
- **Multi-backend OCR pipeline**: New `OcrPipelineConfig` enables quality-based fallback across OCR backends (e.g., Tesseract then PaddleOCR) with configurable priority, language, and backend-specific settings.
- **OCR quality thresholds**: New `OcrQualityThresholds` config with 16 tunable parameters for OCR output quality assessment and fallback decisions.
- **OCR auto-rotate**: New `OcrConfig.auto_rotate` flag (default: false) for automatic page rotation detection. Handles 0/90/180/270 degree rotations.
- **PaddleOCR v2 model tier system**: New `model_tier` field with `"mobile"` (default, ~21MB, fast) and `"server"` (~172MB, highest accuracy). Both use unified multilingual models (CJK+English in one model). Available across all bindings.
- **`AccelerationConfig` for GPU/execution provider control**: Fine-grained control over ONNX execution providers (CPU, CoreML, CUDA, TensorRT) for layout detection and table recognition. Typed across all bindings.
- **`ConcurrencyConfig` for thread limiting** (#503): New `max_threads` field caps Rayon, ONNX intra-op threads, and batch concurrency to a single limit. Typed across all bindings.
- **`EmailConfig` for MSG fallback codepage** (#505): Configurable fallback codepage for MSG files lacking a codepage property (default: windows-1252). Set e.g. `1251` for Cyrillic. Typed across all bindings.
- **Per-file extraction configuration (`FileExtractionConfig`)**: Per-file config overrides in batch operations. Each file can specify its own OCR, chunking, output format settings. CLI supports `--file-configs`, MCP supports `file_configs` parameter.
- **Opt-in single-column pseudo tables** (#449): New `allow_single_column_tables` on `PdfConfig` (default: false). Allows single-column structured data (glossaries, itemized lists) to be emitted as tables.
- **Experimental: `pdf_oxide` text extraction backend** (`pdf-oxide` feature): Pure Rust PDF text extraction as an alternative to pdfium. Opt-in only, not included in `full` feature set.
- **CLI `cache warm` command**: Eagerly downloads all PaddleOCR and layout detection models. Supports `--all-embeddings` or `--embedding-model <preset>`. Useful for containerized or offline deployments.
- **CLI `cache manifest` command**: Outputs a JSON manifest of all expected model files with SHA256 checksums, sizes, and source URLs for scripted cache verification.
- **ChunkSizing configuration**: `sizing_type`, `sizing_model`, and `sizing_cache_dir` fields exposed in `ChunkingConfig` across all bindings.
- **Chunk heading context**: New `HeadingContext` type in `ChunkMetadata` providing heading level and text.
- **`ModelManifestEntry` type and `manifest()` / `ensure_all_models()` methods**: Public API for querying and eagerly downloading model cache manifests.
- **SF1 structural quality metrics in benchmark CI**: SF1 quality scores now computed alongside TF1, with PDF-specific quality rankings for tracking extraction quality regressions.

### Changed

- **Layout preset default**: Changed from `"fast"` to `"accurate"`. The `Fast` variant has been removed. The `"fast"` string is still accepted for backwards compatibility.
- **PaddleOCR default model tier**: Changed from `"server"` to `"mobile"`. Mobile models provide equivalent quality on standard documents while being 3-5x faster. Server tier remains available via `with_model_tier("server")`.
- **PaddleOCR v2 models**: All models updated to v2 generation (PP-OCRv5 detection, PP-LCNet classification, unified multilingual recognition). V1 models remain available for older versions.
- **Unified multilingual recognition models**: PP-OCRv5 unified server (84MB) and mobile (16.5MB) models replace per-script English and Chinese models. Per-script models retained for 9 other script families.
- **Batch API unification**: `_with_configs` batch functions removed; per-file `FileExtractionConfig` is now an optional parameter on the unified batch functions.
- **Layout pipeline no longer forces heuristic extraction**: Structure tree extraction proceeds normally when layout detection is enabled, preserving text quality.
- **Global ONNX model caching**: Layout detection and SLANet models are cached globally and reused across extractions, avoiding expensive ONNX session recreation in batch scenarios.
- **Vendored text embedding pipeline**: Replaced `fastembed` dependency with vendored engine using ONNX Runtime directly for tighter integration.
- **Embedding `embed()` now takes `&self` instead of `&mut self`**: Enables parallel embedding generation without mutable reference constraints.
- **L2 normalization parallelized**: Embedding batches >= 64 vectors now use multi-threaded normalization.
- **`padding` field in PaddleOcrConfig**: Now exposed across Python, TypeScript, Ruby, and Go bindings (previously Rust-only).
- **Language-agnostic section pattern recognition**: Headings ending with a period are now allowed when they match structural patterns (section symbol, all-caps, numbered sections). Improves heading detection for legal, academic, and multilingual documents.
- **Layout classification guards**: Heading overrides from the layout model now have word count limits, punctuation checks, figure label detection, and body-font-size validation to prevent false heading promotions.
- **Strong typing across bindings**: Replaced weak `Dictionary`/`Map`/`array` types with strongly typed config classes in C#, Java, and PHP. Added missing config types to Python stubs, Node.js, Ruby, Elixir, and PHP.

### Removed

- **`fastembed` dependency**: Replaced by vendored embedding engine using ONNX Runtime directly.
- **`EmbeddingModelType::FastEmbed` variant**: Use `Preset` or `Custom` variants instead.

### Fixed

- **C# FFI struct layout mismatch** (#538): `CExtractionResult` struct layout between Rust and C# was mismatched, causing deserialization failures and overflow exceptions that made the C# library completely broken in 4.4.6.
- **PDF `force_ocr` without explicit OCR config** (#495): `force_ocr=true` was silently ignored when no `ocr` config block was provided. Now unconditionally triggers the OCR pipeline with default settings.
- **PDF image extraction** (#511): Extracted images returned raw compressed data instead of properly decoded image bytes. Now automatically decoded and re-encoded as standard formats (PNG/JPEG).
- **Node.js `extractFileInWorker` mime_type passthrough** (#523): MIME type was silently injected into PDF password config instead of being forwarded to extraction. Now correctly passed through.
- **DOCX parser type inference failure** (#519): The `zip` 8.2.0 dependency introduced type ambiguity in DOCX and XML parsers, causing compilation failures.
- **Python `py.typed` and `.pyi` missing from sdist**: Type stubs and `py.typed` marker now included in both wheel and sdist formats.
- **PDF broken CMap word spacing**: Geometric validation now vetoes false word boundaries in PDFs with broken font CMaps, fixing "co mputer" -> "computer" style errors.
- **PDF structure tree heading trust**: Structure tree heading tags (H1-H6) are now trusted as author-intent metadata. Previously, font-size validation rejected valid headings close to body size.
- **PDF structure tree extraction performance**: Text and style maps now built in a single pass, eliminating multi-second extraction times on complex pages.
- **OCR Picture regions suppressing text**: Layout-detected Picture regions now preserve embedded text as plain paragraphs instead of silently dropping it.
- **Non-transitive sort comparators**: Spatial reading-order sorts now use discrete row buckets instead of tolerance-based grouping, ensuring correct and stable ordering.
- **Page furniture over-stripping**: Added bulk and per-paragraph guards to prevent aggressive furniture stripping from removing legitimate content.
- **`KREUZBERG_CACHE_DIR` not respected by all caches**: Embeddings, OCR result cache, and document extraction cache now honor the environment variable.
- **MSG PT_STRING8 encoding**: MSG files now correctly decode ANSI string properties using the declared Windows code page instead of UTF-8 lossy conversion.
- **SLANet-Plus ONNX model**: Re-exported with shape fix, resolving inference failures that caused all SLANet table extractions to silently fail on macOS CoreML.
- **TATR model panic in batch processing**: Model unavailability in parallel closures caused crashes in FFI callers (Java, C#). Now falls back gracefully to heuristic table extraction.
- **Docker musl builds**: Alpine/musl Docker images now link against the system ONNX Runtime library, fixing build failures. All features work in musl CLI images.
- **FFI batch functions null handling**: C#/Java FFI batch functions now accept NULL for `file_config_jsons` instead of rejecting it.

### Known Issues

- **PHP PIE Windows package temporarily unavailable**: The Windows build for the PHP PIE extension is disabled due to a transitive dependency conflict (`ort-sys` → `lzma-rust2` → `crc` version collision on the `x86_64-pc-windows-gnu` target). Linux and macOS PHP packages are unaffected. Will be resolved when upstream `ort` updates its `lzma-rust2` dependency.
- **WASM: no layout detection, acceleration, or email config**: ONNX Runtime does not support WebAssembly, so layout detection (RT-DETR), hardware acceleration config, and concurrency config are unavailable in the WASM binding. OCR via Tesseract WASM and embeddings are supported.

---

## [4.4.6]

### Added

- **dBASE (.dbf) format support**: Extract table data from dBASE files as markdown tables with field type support.
- **Hangul Word Processor (.hwp/.hwpx) support**: Extract text content from HWP 5.0 documents (standard Korean document format).
- **Office template/macro format variants**: Added support for `.docm`, `.dotx`, `.dotm`, `.dot` (Word), `.potx`, `.potm`, `.pot` (PowerPoint), `.xltx`, `.xlt` (Excel) formats.

### Fixed

- **DOCX image placeholders missing (#484)**: Extracting `.docx` files with `extract_images=True` no longer produced `![](image)` placeholders in the output. The default plain text output path was stripping image references. Image extraction now forces markdown output so placeholders are always included.

### Changed

- **Format count updated to 91+**: Documentation across all READMEs, docs, and package manifests updated to reflect expanded format support (previously 75+).

## [4.4.5]

### Fixed

- **PDF markdown garbles positioned text (#431)**: PDFs with positioned/tabular text (CVs, addresses, data tables) had their line breaks destroyed during paragraph grouping. Added page-level positioned text detection: when fewer than 30% of lines on a page reach the right margin, short lines are split into separate paragraphs to preserve the document's visual structure.
- **Node worker pool password bug**: `extractFileInWorker` was passing the `password` argument as `mime_type` to `extract_file_sync`, meaning passwords were never applied and MIME detection could break. Password is now correctly injected into `config.pdf_options.passwords`.
- **Unused import in kreuzberg-node**: Removed unused `use serde_json::Value` import in `result.rs` that caused clippy warnings.
- **WASM Deno OCR test hang**: OCR tests hung indefinitely on WASM Deno because Tesseract synchronous initialization blocks the single-threaded runtime. OCR fixtures are now skipped for the wasm-deno target.
- **WASM camelCase config deserialization**: JS consumers send camelCase config keys (e.g. `includeDocumentStructure`) but `serde` expects snake_case. Added `camel_to_snake` transform in `parse_config()` so config fields are properly deserialized. Fixes document structure extraction returning empty results via WASM.
- **PHP 8.5 array coercion on macOS**: On PHP 8.5 + macOS, ext-php-rs coerces `#[php_class]` return values to arrays instead of objects. Added `normalizeExtractionResult()` wrapper that transparently converts arrays via `ExtractionResult::fromArray()`.
- **PHP 8.5 support**: Upgraded ext-php-rs to 0.15.6 for PHP 8.5 compatibility.
- **Vendoring scripts missing path deps**: Ruby and R vendoring scripts failed when workspace dependencies use `path` instead of `version`. Added path field handling to `format_dependency()` and kreuzberg-ffi fixup block to the Ruby vendoring script.
- **pdfium-render clippy lints**: Fixed clippy warnings in kreuzberg-pdfium-render crate.

### Added

- **CLI `--pdf-password` flag**: New `--pdf-password` option on `extract` and `batch` commands for encrypted PDF support. Can be specified multiple times.
- **MCP `pdf_password` parameter**: Added `pdf_password` field to `extract_file`, `extract_bytes`, and `batch_extract_files` MCP tool params for better discoverability.
- **API `pdf_password` multipart field**: The HTTP API extract endpoint now accepts a `pdf_password` multipart field for encrypted PDFs.
- **`PdfConfig` Default impl**: Added `Default` implementation for `PdfConfig` to support ergonomic config construction.
- **Binding crate clippy in CI**: Added clippy steps to `ci-node`, `ci-python`, and `ci-wasm` workflows (gated to Linux). Added `node:clippy`, `python:clippy`, and `wasm:clippy` task commands.
- **E2E password-protected PDF fixture**: Added `pdf_password_protected` fixture testing copy-protected PDF extraction across all bindings.

### Changed

- **All binding crates linted in pre-commit**: Removed clippy exclusions for kreuzberg-php, kreuzberg-node, and kreuzberg-wasm from pre-commit config.
- **golangci-lint v2.11.3**: Upgraded from v2.9.0 across Taskfile, CI workflows, and install scripts.

## [4.4.4]

### Fixed

- **CLI test app fixes**: Fixed broken symlinks in CLI test documents, corrected `--format` to `--output-format` flag usage, fixed multipart form field name (`file=` → `files=`) in serve tests, and rewrote MCP test to use JSON-RPC stdin protocol instead of background process detection.
- **Publish idempotency check scripts**: Fixed `check_nuget.sh` and `check-nuget-version.sh` using bash 4+ `${var,,}` syntax incompatible with bash 3.x. Fixed `check_pypi.sh` and `check_packagist.sh` writing to `$GITHUB_OUTPUT` internally instead of stdout (conflicting with workflow-level redirect). Fixed `check-rubygems-version.sh` false negatives for native gems by switching from `gem search` to RubyGems JSON API. Fixed `check-rubygems-version-python.sh` Python operator precedence bug. Fixed `check-maven-version.sh` using unreliable Solr search API instead of direct repo HEAD request. Fixed stderr redirect missing on diagnostic messages in multiple scripts.
- **Node test app version**: Updated Node.js test app to reference v4.4.4 package version.

### Changed

- **CLI install with all features**: CLI test install script now uses `--all-features` flag to enable API server and MCP server subcommands.
- **Publish workflow republish support**: Added `republish` input to publish workflow that deletes and re-creates the tag on current HEAD before publishing, enabling clean retag + full republish.

## [4.4.3]

### Added

- **PDF image placeholder toggle**: New `inject_placeholders` option on `ImageExtractionConfig` (default: `true`). Set to `false` to extract images as data without injecting `![image](...)` references into the markdown content.

### Fixed

- **Token reduction not applied** ([#436](https://github.com/kreuzberg-dev/kreuzberg/issues/436)): Token reduction config was accepted but never executed during extraction. The pipeline now applies `reduce_tokens()` when `token_reduction.mode` is configured.
- **Nested HTML table extraction**: Nested HTML tables now extract correctly with proper cell data and markdown rendering, using the visitor-based table extraction API from html-to-markdown-rs.
- **hOCR plain text output**: hOCR conversion now correctly produces plain text when `OutputFormat::Plain` is requested, instead of silently falling back to Markdown.
- **PDF garbled text for positioned/tabular content** ([#431](https://github.com/kreuzberg-dev/kreuzberg/issues/431)): PDF text extraction now detects X-position gaps between consecutive characters and inserts spaces when the gap exceeds `0.8 × avg_font_size`. Previously, characters placed at specific coordinates without explicit space characters were concatenated without spaces.
- **Chunk page metadata drift with overlap** ([#439](https://github.com/kreuzberg-dev/kreuzberg/issues/439)): Chunk byte offsets are now computed via pointer arithmetic from the source text, fixing cumulative drift that caused chunks to report incorrect page numbers when overlap is enabled.
- **Node.js metadata casing**: Standardized all `Metadata` and `EmailMetadata` fields to `camelCase` (e.g., `pageCount`, `creationDate`, `fromEmail`) in the Node.js/TypeScript bindings. Also corrected pluralization for `authors` and `keywords`.
- **WASM build failure on Windows CI**: CMake try-compile checks on Windows used the host MSVC compiler (`cl.exe`), which rejected GCC/Clang flags like `-Wno-implicit-function-declaration`. Added `CMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY` to both `build_leptonica_wasm` and `build_tesseract_wasm` to skip linking during cross-compilation checks.
- **WASM OCR build panic when `git`/`patch` unavailable**: The tesseract WASM patch (`tesseract.diff`) application panicked when both `git apply` and `patch` commands failed. Added programmatic C++ source fixups as a fallback, applying all necessary changes (CPUID guard, pixa_debug_ unique_ptr conversion, source list trimming) via string replacement when the diff patch cannot be applied.

## [4.4.2]

### Fixed

- **E2E element type assertions**: Fixed element type field name in E2E generator templates for Python, TypeScript, WASM Deno, Elixir, Ruby, PHP, and C#. Each binding uses different casing conventions (Python: dict key `element_type`, TypeScript/Node: `elementType` via NAPI camelCase, Elixir: atom-to-string conversion, C#: JSON serialization for snake_case wire value).
- **Ruby PDF annotation extraction**: Fixed `PdfAnnotation` and `PdfAnnotationBoundingBox` classes not being registered in the autoload list, causing `NameError` when extracting PDF annotations. Also fixed bounding box field name mismatch between Rust output (`x0/y0/x1/y1`) and Ruby struct (`left/top/right/bottom`).
- **Ruby cyclomatic complexity**: Refactored `build_annotation_bbox` in result.rb to extract repeated field lookup pattern, reducing cyclomatic complexity below threshold.
- **WASM OCR blocking event loop**: The `ocrRecognize()` function in the WASM package was running synchronously on the main thread, blocking the Node.js event loop during image decoding and Tesseract OCR processing. This prevented timeouts and other async operations from firing while OCR was in progress. OCR now runs in a worker thread (Node.js `worker_threads` / browser `Web Worker`), keeping the main thread responsive.
- **JPEG 2000 OCR decode failure**: JPEG 2000 images (jp2, jpx, jpm, mj2) and JBIG2 images failed with "The image format could not be determined" during PaddleOCR and WASM OCR because these code paths used the standard `image` crate which doesn't support JPEG 2000. A shared `load_image_for_ocr()` helper now detects JP2/J2K/JBIG2 formats by magic bytes and uses `hayro-jpeg2000`/`hayro-jbig2` decoders across all OCR backends. The `ocr-wasm` feature now includes these decoders (pure Rust, WASM-compatible).
- **WASM PDF empty content**: `initWasm()` fired off PDFium initialization asynchronously without awaiting it, causing a race condition where PDF extraction could start before PDFium was ready, returning empty content. PDFium initialization is now properly awaited during `initWasm()`.

### Added

- **OMML-to-LaTeX math conversion for DOCX**: Mathematical equations in DOCX files (Office Math Markup Language) are now converted to LaTeX notation instead of being rendered as concatenated Unicode text. Supports superscripts, subscripts, fractions (`\frac`), radicals (`\sqrt`), n-ary operators (`\sum`, `\int`), delimiters, function names, accents, equation arrays, limits, bars, border boxes, matrices, and pre-sub-superscripts. Display math uses `$$...$$` and inline math uses `$...$` in markdown output. Plain text output includes raw LaTeX without delimiters.

- **Plain text output paths for all extractors**: When `OutputFormat::Plain` or `OutputFormat::Structured` is requested, DOCX, PPTX, ODT, FB2, DocBook, RTF, and Jupyter extractors now produce clean plain text without markdown syntax (`#`, `**`, `|`, `![](image)`, `-`, etc.). Previously these extractors always emitted markdown regardless of the requested output format.
  - **DOCX**: `Document::to_plain_text()` skips heading prefixes, inline formatting markers, image placeholders, and renders footnotes/endnotes as `id: text` instead of `[^id]: text`.
  - **PPTX**: `ContentBuilder` respects `plain` mode — skips `#` title prefix, image markers, list markers, and uses `Notes:` instead of `### Notes:`.
  - **ODT**: Heading prefixes (`#`), list markers (`-`), and pipe-delimited tables conditionally omitted for plain text.
  - **FB2/FictionBook**: Inline markers (`*`, `**`, `` ` ``, `~~`), heading prefixes, and cite prefixes skipped for plain text.
  - **DocBook**: Section title prefixes, code fences, list markers, blockquote prefixes, bold figure captions, and pipe tables all conditionally omitted.
  - **RTF**: Table output in result string uses tab separation instead of pipe-delimited markdown. Image `![image](...)` markers omitted for plain text.
  - **Jupyter**: Skips `text/markdown` and `text/html` output types in plain mode, preferring `text/plain`.

- **`cells_to_text()` shared utility**: Tab-separated plain text table formatter alongside existing `cells_to_markdown()`. Used by DOCX, PPTX, ODT, RTF, and DocBook extractors for plain text table rendering.

### Changed

- **CLI includes all features**: `kreuzberg-cli` now depends on `kreuzberg` with the `full` feature set instead of a separate `cli` subset. The `cli` feature group has been removed from `kreuzberg`. This ensures the CLI supports all formats including archives (7z, tar, gz, zip).

### Fixed

- **Alpine/musl CLI Docker image**: Fixed "Dynamic loading not supported" error when running `kreuzberg-cli` in Alpine containers. The CLI binary is now dynamically linked against musl libc, enabling runtime library loading for PDF processing.
- **R package Windows installation**: Improved Python detection in configure script for Windows environments (added `py` launcher and `RETICULATE_PYTHON` support). Symlink extraction errors during source package installation are now handled gracefully.
- **PHP 8.5 precompiled extension binaries**: Added PHP 8.5 support alongside existing PHP 8.4 in CI and release workflows.
- **OCR DPI normalization**: The `normalize_image_dpi()` preprocessing logic is now integrated into the OCR pipeline. Images are normalized to the configured target DPI before being passed to Tesseract, and the calculated DPI is set via `set_source_resolution()`. This eliminates the "Estimating resolution as ..." warning and improves OCR accuracy for images with non-standard DPI.
- **HTML metadata extraction with Plain output**: Fixed HTML metadata (headers, links, images, structured data) not being collected when using `OutputFormat::Plain` (the default). The underlying library's plain text fast path skips metadata extraction; kreuzberg now uses Markdown format internally for metadata collection and converts to plain text separately.
- **PPTX text run spacing**: Adjacent text runs within paragraphs are now joined with smart spacing instead of being concatenated directly ("HelloWorld" → "Hello World").
- **CSV Shift-JIS/cp932 encoding detection**: `encoding_rs` is now a non-optional dependency. CSV files with Shift-JIS encoding are correctly decoded instead of producing mojibake. Fallback encoding detection tries common encodings (Shift-JIS, cp932, windows-1252, iso-8859-1, gb18030, big5).
- **EML multipart body extraction**: All text/html body parts are now extracted by iterating over all indices instead of only index 0. Nested `message/rfc822` parts in multipart/digest are recursively extracted.
- **EPUB media tag leakage**: `<video>`, `<audio>`, `<source>`, `<track>`, `<object>`, `<embed>`, `<iframe>` tags no longer leak into extracted text. Added `<br>` → newline and `<hr>` → newline handling.
- **FB2 poem extraction**: Added support for `<poem>`, `<stanza>`, and `<v>` (verse) elements. Previously poetry content was silently dropped.
- **FB2 Unicode sub/superscript**: Characters inside `<sup>` and `<sub>` are converted to Unicode equivalents. Added strikethrough support, horizontal rules for `<empty-line>`, and footnote extraction from notes body.
- **ODT StarMath-to-Unicode conversion**: Mathematical formulas in ODT files are now converted to Unicode equivalents (Greek letters, operators, super/subscripts) instead of raw StarMath syntax.
- **BibTeX output format**: Output now uses `@type{key, field = {value}}` format matching standard BibTeX conventions.
- **LaTeX display math**: `\[...\]` display math environments are converted to `$...$` format.
- **RST directive preservation**: Field lists, directive markers, and `.. code-block::` directives are preserved in extracted text.
- **RTF table cell separators**: Plain mode now uses pipe delimiters for table cells instead of tabs.
- **Typst extraction improvements**: Layout directives stripped, headings output as plain text, tables extracted with column-aware layout, links output as display text only.
- **DOCX field codes refined**: Field instructions (between `begin` and `separate`) are now skipped while field results (between `separate` and `end`) are preserved. Previously all content between field begin/end was dropped, losing visible text like "Figure 1:" and page numbers.
- **DOCX drawing alt text in plain text**: `to_plain_text()` now emits image alt text from `wp:docPr` descriptions instead of silently skipping drawings.
- **DOCX/drawing/table XML entity decoding**: `get_attr()` helpers in `drawing.rs` and `table.rs` now use `quick_xml::escape::unescape()` to correctly decode XML entities like `&#xA;` in attribute values.

---

## [4.4.1]

### Added

- **OCR table inlining into markdown content** (#421): When `output_format = Markdown` and OCR detects tables, the markdown pipe tables are now inlined into `result.content` at their correct vertical positions instead of only appearing in `result.tables`. Adds `OcrTableBoundingBox` to `OcrTable` for spatial positioning. Sets `metadata.output_format = "markdown"` to signal pre-formatted content and skip re-conversion.
- **OCR table bounding boxes**: OCR-detected tables now include bounding box coordinates (pixel-level) computed from TSV word positions, propagated through all bindings as `Table.bounding_box`.
- **OCR table test images**: Added balance sheet and financial table test images from issue #421 for integration testing.

### Fixed

- **OCR test_tsv_row_to_element used wrong Tesseract level**: Test specified `level: 4` (Line) but asserted `Word`. Fixed to `level: 5` (correct Tesseract word level).

- **MSG recipients missing email addresses**: The MSG extractor read `PR_DISPLAY_TO` which contains only display names (e.g. "John Jennings"), losing email addresses entirely. Now reads recipient substorages (`__recip_version1.0_#XXXXXXXX`) with `PR_EMAIL_ADDRESS` and `PR_RECIPIENT_TYPE` to produce full `"Name" <email>` output with correct To/CC/BCC separation.
- **MSG date missing or incorrect**: Date was parsed from `PR_TRANSPORT_MESSAGE_HEADERS` which is absent in many MSG files. Now reads `PR_CLIENT_SUBMIT_TIME` FILETIME directly from the MAPI properties stream, with fallback to transport headers.
- **EML date mangled for non-standard formats**: `mail_parser` parsed ISO 8601 dates (e.g. `2025-07-29T12:42:06.000Z`) into garbled output (`2000-00-20T00:00:00Z`) and replaced invalid dates with `2000-00-00T00:00:00Z`. Now extracts the raw `Date:` header text from the email bytes, preserving the original value.
- **EML/MSG attachments line pollutes text output**: `build_email_text_output()` appended an `Attachments: ...` line that doesn't represent message content. Removed from text output; attachment names remain in metadata.
- **HTML script/style tags leak in email fallback**: The regex-based HTML cleaner for email bodies used `.*?` which doesn't match across newlines, allowing multiline `<script>`/`<style>` content to leak into extracted text. Added `(?s)` flag for dotall matching.
- **SVG CData content leaks JavaScript/CSS**: `Event::CData` handler in the XML extractor didn't check SVG mode, causing `<script>` and `<style>` CDATA blocks to appear in SVG text output.
- **RTF parser leaks metadata noise into text**: The RTF extractor did not skip known destination groups (`fonttbl`, `stylesheet`, `colortbl`, `info`, `themedata`, etc.) or ignorable destinations (`{\*\...}`), causing ~17KB of font tables, color definitions, and internal metadata to appear in extracted text.
- **RTF `\u` control word mishandled**: Control words like `\ul` (underline) and `\uc1` were incorrectly interpreted as Unicode escapes (`\u` + numeric param), producing garbage characters instead of being treated as formatting commands.
- **RTF paragraph breaks collapsed to spaces**: `\par` control words emitted a single space instead of newlines, causing all paragraphs to merge into a single line. Now correctly emits double newlines for paragraph separation.
- **RTF whitespace normalization destroys paragraph structure**: `normalize_whitespace()` treated newlines as whitespace and collapsed them to spaces. Rewritten to preserve newlines while collapsing runs of spaces within lines.

---

## [4.4.0]

### Added

- **R language bindings** — Added kreuzberg R package via extendr with full extraction API (sync/async, batch, bytes), typed error conditions, S3 result class with accessors, config discovery, OCR/chunking configuration, plugin system, and 32 documentation snippets.
- **PHP async extraction**: Non-blocking extraction via `DeferredResult` pattern with Tokio thread pool. Includes `extractFileAsync()`, `extractBytesAsync()`, `batchExtractFilesAsync()`, `batchExtractBytesAsync()` across OOP, procedural, and static APIs. Framework bridges for Amp v3+ (`AmpBridge`) and ReactPHP (`ReactBridge`).
- **WASM native OCR** (`ocr-wasm` feature): Tesseract OCR compiled directly into the WASM binary via `kreuzberg-tesseract`, enabling OCR in all environments (Browser, Node.js, Deno, Bun) without browser-specific APIs. Supports 43 languages with tessdata downloaded from CDN into memory.
- **WASM Node.js/Deno PDFium support**: PDFium initialization now works in Node.js and Deno by loading the WASM module from the filesystem. Configurable via `KREUZBERG_PDFIUM_PATH` environment variable.
- **WASM full-feature build**: OCR, Excel, and archive extraction are now enabled by default in the WASM package. All `wasm-pack build` targets include the `ocr-wasm` feature.
- **WASM Excel extraction** (`excel-wasm` feature): Calamine-based Excel/spreadsheet extraction available in WASM without requiring Tokio runtime.
- **WASM archive extraction**: ZIP, TAR, 7z, and GZIP archive extraction now available in WASM via synchronous extractor implementations.
- **WASM PDF annotations**: PDF annotations (text notes, highlights, links, stamps) are now exposed in the WASM TypeScript API via the `annotations` field on `ExtractionResult`.
- **C FFI distribution**: Official C shared library (`libkreuzberg`) with cbindgen-generated header, cmake packaging (`find_package(kreuzberg)`), pkg-config support, and prebuilt binaries for Linux x86_64/aarch64, macOS arm64, and Windows x86_64. Includes 10 test files, benchmark harness integration, and full API reference documentation.
- **Go FFI bindings**: Go package (`packages/go/v4`) consuming the C FFI shared library with prebuilt binaries published as GitHub release assets for all four platforms.
- **C as 12th e2e test language**: The e2e-generator now produces C test files exercising the FFI API, with 15 passing test cases.
- **R distribution via r-universe**: Switched R package distribution from CRAN to r-universe for faster release cycles and easier native compilation. Includes vendoring script for offline builds.

### Fixed

- **DOCX equations not extracted**: OMML math content (`<m:oMath>`, `<m:r>`, `<m:t>` elements) was completely ignored by the DOCX parser, causing all equation text (e.g. `A=πr²`, quadratic formula) to be silently dropped. Math runs are now extracted as regular text.
- **DOCX line breaks ignored**: `<w:br/>` elements were not handled, causing adjacent text segments to merge (e.g. timestamps concatenated with following text). Line breaks now insert whitespace.
- **PPTX/PPSX table content lost**: Tables were rendered as HTML without whitespace between tags, causing the entire table to tokenize as a single unreadable blob. Tables now render as markdown pipe tables with proper cell separation.
- **PPTX/PPSX/PPTM image markers pollute text**: Image references like `![rId2](slide_1_image_rId2.jpg)` injected spurious numeric tokens into extracted content. Image markers now use a clean `![image]()` format.
- **DOCX image markers pollute text**: Drawing references like `![alt](image_3)` injected spurious numeric tokens. Changed to `![alt](image)`.
- **EPUB double-lossy conversion**: XHTML content was converted through an XHTML→markdown→plain-text pipeline, losing content at each stage (underscores, asterisks, numeric URLs stripped). Replaced with direct `roxmltree` traversal that extracts text content from XHTML elements without intermediate markdown.
- **Excel float formatting drops numeric precision**: `format_cell_to_string()` formatted whole-number floats as `"1.0"` instead of `"1"`, causing numeric token mismatches in quality scoring. Also fixed `DateTime` handling to use `to_ymd_hms_milli()` instead of the unavailable `as_datetime()` API.
- **HTML metadata extraction pollutes content**: When using `convert_html_to_markdown_with_metadata()`, the `extract_metadata` option was left enabled, causing YAML frontmatter to be prepended to the content string even though metadata was already returned as a struct. Set `extract_metadata = false` in the metadata extraction path.
- **Markdown extractor loses tokens through AST reconstruction**: The markdown extractor parsed content into a pulldown-cmark AST then reconstructed text, losing tokens through transformation. Now returns raw text content directly (after frontmatter extraction) while still parsing the AST for table and image extraction.
- **SVG text extraction includes element prefixes**: XML extractor prepended `element_name:` to all text content, adding spurious tokens. SVG extraction now targets only text-bearing elements (`<text>`, `<tspan>`, `<title>`, `<desc>`) without prefixes.
- **XML ground truth uses raw source**: CSV, XML, and IPYNB ground truth files contained raw source markup (delimiters, tags, JSON structure) instead of expected extracted text, causing quality scores near zero. Regenerated all 20 ground truth files.
- **Elixir benchmark UTF-8 locale**: Erlang VM running with `latin1` native encoding corrupted UTF-8 strings from Rust NIFs. Added `ERL_LIBS` path configuration in the benchmark harness.
- **WASM OCR not working** (`enableOcr()` regression): `enableOcr()` registered the OCR backend only in a JS-side registry, but the Rust extraction pipeline uses a separate Rust-side plugin registry. OCR via `extractBytes`/`extractFile` always failed with "OCR backend 'tesseract' not registered". The function now bridges both registries so OCR works end-to-end.
- **WASM tessdata CDN URL returns 404**: The `NativeWasmOcrBackend` tessdata URL pointed to a non-existent path in the `tesseract-wasm` npm package. Updated to use the official `tesseract-ocr/tessdata_fast` GitHub repository.
- **XML UTF-16 parsing fails on files with odd byte count**: The XML extractor rejected valid UTF-16 encoded files that had a trailing odd byte (e.g. `factbook-utf-16.xml`) with "Invalid UTF-16: odd byte count". The decoder now truncates to the nearest even byte boundary, matching the lenient approach already used in email extraction.
- **R bindings crash on strings with embedded NUL bytes**: Extraction results containing NUL (`\0`) characters (e.g. from RTF files) caused the R FFI layer to error with "embedded nul in string" since R strings are C-based. NUL bytes are now stripped before passing strings to R.
- **R bindings `%||%` operator incompatible with R < 4.4**: The R package used the `%||%` null-coalescing operator which is only available in base R >= 4.4, but the package declares `R >= 4.2`. Added a package-local polyfill for backwards compatibility.
- **API returns HTTP 500 for unsupported file formats** (#414): Uploading files with unsupported or undetectable MIME types (e.g. DOCX via `curl -F`) returned HTTP 500 Internal Server Error instead of HTTP 400 Bad Request. The `/extract` endpoint now falls back to extension-based MIME detection from the filename when the client sends `application/octet-stream`, and `UnsupportedFormat` errors are mapped to HTTP 400 with a clear `UnsupportedFormatError` response.
- **PDF markdown extraction missing headings/bold for flat structure trees** (#391): PDFs where the structure tree tags everything as `<P>` (common with Adobe InDesign) now produce proper headings and bold text. The structure tree path previously bypassed font-size-based heading classification entirely. Pages with font size variation but no heading tags are now enriched via K-means font-size clustering. Additionally, bold detection now recognizes fonts with "Bold" in the name (e.g. `MyriadPro-Bold`) even when the PDF doesn't set the font weight descriptor.
- **PaddleOCR backend not found when using `backend="paddleocr"`** (#403): The PaddleOCR backend registered itself as `"paddle-ocr"` but users and documentation use `"paddleocr"`. The OCR backend registry now resolves the `"paddleocr"` alias to the canonical `"paddle-ocr"` name.
- **WASM metadata serialization**: Fixed `#[serde(flatten)]` with internally-tagged enums dropping `format_type` and format-specific metadata fields. Switched from `serde_wasm_bindgen` to `serde_json` + `JSON.parse()` for output serialization.
- **WASM config deserialization**: Fixed camelCase TypeScript config keys (e.g. `outputFormat`, `extractAnnotations`) not being recognized by Rust serde. Config keys are now converted to snake_case before passing to the WASM boundary.
- **WASM PDFium module loading**: Fixed `copy-pkg.js` overwriting the real PDFium Emscripten module with a stub init helper. The build script now locates and copies the actual PDFium ESM module (`pdfium.esm.js` + `pdfium.esm.wasm`) from the Cargo build output, with a Deno compatibility fix for bare `import("module")`.
- **Email header extraction loses display names**: EML and MSG parsers extracted only bare email addresses, discarding sender/recipient display names. From, To, CC, and BCC fields now use `"Display Name" <email@example.com>` format when a display name is available.
- **Email date header normalized to RFC 3339**: The EML parser always converted dates to RFC 3339 format, losing the original date string. Now preserves the raw `Date` header value and only falls back to RFC 3339 normalization when the raw header is unavailable.
- **Docker builds fail due to missing snippet-runner exclusion**: The `sed` command in `Dockerfile.cli`, `Dockerfile.core`, and `Dockerfile.full` did not remove the `snippet-runner` workspace member, causing build failures when the crate directory was not COPY'd into the build context.
- **WASM Deno e2e tests skip OCR fixtures**: Generated Deno test files called `initWasm()` but never called `enableOcr()`, so the Tesseract OCR backend was never registered and all OCR tests silently skipped. The e2e generator now calls `enableOcr()` after `initWasm()` in every generated test file.
- **WASM Deno e2e tests ignore pages config**: The `buildConfig()` helper in generated Deno tests did not map the `pages` extraction config (page markers, page extraction), causing tests with page-related assertions to use defaults. Added `mapPageConfig()` to the test helper template.

### Removed

- **`polars` dependency**: Removed unused `polars` crate and `table_from_arrow_to_markdown` dead code from the `excel` feature. Excel extraction uses `calamine` directly.

---

## [4.3.8]

### Added

- **MDX format support** (`mdx` feature): Extract text from `.mdx` files, stripping JSX/import/export syntax while preserving markdown content, frontmatter, tables, and code fences
- **List supported formats API** (#404): Query all supported file extensions and MIME types via `list_supported_formats()` in Rust, `GET /formats` REST endpoint, `list_formats` MCP tool, or `kreuzberg formats` CLI subcommand

### Fixed

- **PDF ligature corruption in CM/Type1 fonts**: Added contextual ligature repair for PDFs with broken ToUnicode CMaps where pdfium doesn't flag encoding errors. Fixes corrupted text like `di!erent` → `different`, `o"ces` → `offices`, `#nancial` → `financial` in LaTeX-generated PDFs. Uses vowel/consonant heuristic to disambiguate ambiguous ligature mappings. Applied to both structure tree and heuristic extraction paths.
- **PDF dehyphenation across line boundaries**: Added paragraph-level dehyphenation that rejoins words broken across PDF line breaks (e.g. `soft ware` → `software`, `recog nition` → `recognition`). Handles both explicit trailing hyphens (Case 1) and implicit breaks where pdfium strips the hyphen (Case 2, using full-line detection). Applied to both structure tree and heuristic extraction paths.
- **PDF page markers missing in Markdown and OCR output** (#412): Page markers (`insert_page_markers` / `marker_format`) were not inserted when using Markdown output format or OCR extraction since the 4.3.5 pipeline rewrite. Fixed by threading the marker format through the markdown assembly pipeline and OCR page joining. Djot output inherits markers automatically.
- **PDF Djot/HTML output quality parity**: Djot and HTML output formats now use the same high-quality structural extraction pipeline as Markdown (headings, tables, bold/italic, dehyphenation). Previously these formats fell back to plain text split into paragraphs, losing all document structure.
- **PDF sidebar text pollution**: Widened the margin band for sidebar character filtering from 5% to 6.5% of page width, fixing cases where rotated sidebar text (e.g. arXiv identifiers) leaked into extracted content.
- **Node.js PDF config options not passed to native binding**: Fixed `extractAnnotations`, `hierarchy`, `topMarginFraction`, and `bottomMarginFraction` PDF config fields being silently dropped by the TypeScript config normalizer, causing PDF annotation extraction to always return `undefined` in the Node.js binding.

---

## [4.3.7]

### Added

- NFC unicode normalization applied to all extraction outputs, ensuring consistent representation of composed characters across all backends (gated behind `quality` feature)
- Configurable PDF page margin fractions (`top_margin_fraction`, `bottom_margin_fraction`) in `PdfConfig`
- PDF annotation extraction with new `PdfAnnotation` type supporting `Text`, `Highlight`, `Link`, `Stamp`, `Underline`, `StrikeOut`, and `Other` annotation types
- `extract_annotations` configuration option in `PdfConfig`
- `annotations` field on `ExtractionResult` across all language bindings (Rust, Python, TypeScript, Ruby, PHP, Go, Java, C#, Elixir, WASM)

### Fixed

- **PDF markdown extraction quality at parity with docling** (91.0% avg F1 vs docling's 91.4% across 16 test PDFs, while being 10-50x faster): Replaced `PdfiumParagraph::from_objects()` with per-character text extraction using pdfium's `PdfPageText::chars()` API, which correctly handles font matrices, CMap lookups, and text positioning. Adaptive line-break detection uses measured Y-position changes rather than font-size-relative thresholds, fixing PDFs where pdfium reports incorrect unscaled font sizes.
- **PDF markdown extraction no longer drops all content on PDFs with broken font metrics**: Added font-size filter fallback — when the `MIN_FONT_SIZE` filter (4pt) removes all text segments (e.g. PDFs where pdfium reports `font_size=1` due to font matrix scaling), the filter is skipped and unfiltered segments are used instead.
- **PDF margin filter no longer drops all content on edge-case PDFs**: Added margin filter fallback — when margin filtering removes all text segments (e.g. PDFs where pdfium reports baseline_y values outside expected margin bands), the filter is skipped for that page.
- **PDF ligature repair integrated into per-character extraction**: Ligature corruption (`fi`→`!`, `fl`→`#`, `ff`→`"`) is now repaired inline during character iteration rather than as a separate post-processing pass, improving both accuracy and performance.
- **PDF multi-column text extraction** improved: Federal Register-style multi-column PDFs went from 69.9% to 90.7% F1 by using pdfium's text API which naturally handles reading order.
- PDF table detection now requires ≥3 aligned columns, eliminating false positives from two-column text layouts (academic papers, newsletters)
- PDF table post-processing rejects tables with ≤2 columns, >50% long cells, or average cell length >50 chars
- PDF markdown rendering no longer drops content when pdfium returns zero-value baseline coordinates (fixes missing titles/authors in some LaTeX-generated PDFs)
- PaddleOCR backend validation now dynamically checks the plugin registry instead of hardcoding, preventing false "backend not registered" errors when the plugin is available (#403)
- WASM bindings now export `detectMimeFromBytes` and `getExtensionsForMime` MIME utility functions
- Node.js NAPI-RS binding correctly exposes `annotations` field on `ExtractionResult`
- Python output format validation tests updated to reflect `json` as a valid format (alias for `structured`)
- XLSX extraction with `output_format="markdown"` now produces markdown tables instead of plain text (#405)
- MCP tools with no parameters (`cache_stats`, `cache_clear`) now emit valid `inputSchema` with `{"type": "object", "properties": {}}` instead of `{"const": null}`, fixing Claude Code and other MCP clients that validate schema type (#406)
- Python `get_valid_ocr_backends()` now unconditionally includes `paddleocr` in the returned list, matching all other language bindings
- TypeScript E2E test generator now maps `extract_annotations` to `extractAnnotations` in `mapPdfConfig()`, fixing annotation assertion failures
- PHP `PdfConfig` now includes `extractAnnotations`, `topMarginFraction`, and `bottomMarginFraction` fields, restoring parity with the Rust core config

---

## [4.3.6]

### Added

- **Pdfium `PdfParagraph` object-based extraction**: New markdown extraction path using pdfium's `PdfParagraph::from_objects()` for spatial text grouping, replacing raw page-object iteration. Provides accurate per-line baseline positions via `into_lines()` and styled text fragments with bold/italic/monospace detection.
- **Structure tree and content marks API in pdfium-render**: New `ExtractedBlock`, `ContentRole`, and `PdfParagraph` types for tagged PDF semantic extraction. Structure tree headings are validated against font size and word count to prevent broken structure trees from misclassifying body text.
- **Modular markdown pipeline**: Refactored PDF markdown rendering into focused modules — `bridge.rs` (pdfium API bridge), `lines.rs` (baseline grouping), `paragraphs.rs` (paragraph detection), `classify.rs` (heading/code classification), `render.rs` (inline markup), `assembly.rs` (table/image interleaving), `pipeline.rs` (orchestration).
- **Text encoding normalization**: `normalize_text_encoding()` in bridge.rs converts trailing soft hyphens (`\u{00AD}`) to regular hyphens for word-rejoining, strips mid-word soft hyphens, and removes stray C0 control characters from PDF text.
- **Table post-processing validation**: Ported `post_process_table()` from html-to-markdown-rs with 10-stage validation — empty row removal, long cell rejection, data row detection, header extraction, column merging, dimension checks, column sparsity, overall density, content asymmetry, and cell normalization. Eliminates false positive table detections in non-table PDFs.
- **Font quality detection for OCR triggering**: Added `has_unicode_map_error()` to pdfium-render's `PdfPageTextChar`, wrapping `FPDFText_HasUnicodeMapError`. During extraction, characters are sampled per page; if >30% have broken unicode mappings (tofu/garbage), OCR fallback is triggered automatically.
- **Extended list prefix detection**: Paragraph list detection now recognizes en dashes (`–`), em dashes (`—`), single-letter alphabetic prefixes (`a.`, `b)`, `A.`, `B)`), and roman numerals (`i.` through `xii.`).

### Fixed

- **UTF-8 panic in PDF list detection (#398)**: `detect_list_items()` assumed all newlines are 1 byte, causing panics on multi-byte UTF-8 content with CRLF line endings. Fixed with proper CRLF-aware newline advancement and char boundary guards in `process_content()`.
- **PaddleOCR backend not respected in Python bindings (#399)**: `_ensure_ocr_backend_registered()` silently returned without registering for `paddleocr`/`paddle-ocr` backends. These are now correctly skipped like `tesseract`, letting the Rust core handle them.
- **Ruby gem missing `sorbet-runtime` at runtime (#400)**: `sorbet-runtime` was listed as a development dependency in the gemspec but is required at runtime for `T::Struct` types. Promoted to a runtime dependency.
- **E2e generator Ruby rubocop warnings**: The Ruby e2e generator emitted redundant `RSpec/DescribeClass` and `RSpec/ExampleLength` inline disable directives that rubocop autocorrect mangled into invalid syntax. Simplified to only disable `Metrics/BlockLength`.
- **E2e generator TypeScript npm warnings**: Replaced `npx` with `pnpm exec` for running biome in the e2e generator, eliminating spurious warnings from pnpm-specific `.npmrc` settings.
- **Tesseract TSV level mapping off-by-one**: OCR element hierarchy levels were incorrectly mapped — levels are 1=Page, 2=Block, 3=Paragraph, 4=Line, 5=Word. Fixed `parse_tsv_to_elements` to include word-level entries.
- **OCR elements dropped in image OCR path**: `image_ocr.rs` hardcoded `ocr_elements` to `None` instead of passing through the elements parsed from Tesseract TSV output.
- **DOCX extractor panic on multi-byte UTF-8 page boundaries (#401)**: Page break insertion used byte-index slicing on multi-byte UTF-8 content, causing panics. Fixed with char-boundary-safe insertion.
- **Node.js `djot_content` field missing**: `JsExtractionResult` in kreuzberg-node was not mapping the `djot_content` field from Rust results, always returning `undefined`.
- **E2e generator missing `mapPageConfig` and `mapHtmlOptions`**: TypeScript e2e test generator did not map page extraction or HTML formatting options from fixture configs, causing tests with those options to use defaults.
- **Pipeline test race conditions**: Replaced manual `REGISTRY_TEST_GUARD` mutex with `#[serial]` from `serial_test`, fixing flaky failures in `test_pipeline_with_quality_processing`, `test_pipeline_with_all_features`, and `test_postprocessor_runs_before_validator` caused by global registry state pollution between parallel tests.
- **`test_pipeline_with_keyword_extraction` permanently ignored**: Test was marked `#[ignore]` due to test isolation issues. Fixed the underlying problem — `Lazy` static prevented re-registration after `shutdown_all()` — by clearing the processor cache after re-registration.
- **OCR cache deserialization failure**: Added `#[serde(default)]` to `OcrConfidence.detection` field so cached OCR data from before the field was added can still deserialize.
- **CI validate, Rust e2e, Java e2e, and C# e2e failures**: Fixed `ChunkerType` serde casing, populated `djot_content` in pipeline for Djot output format, fixed Java/C# e2e test helper APIs.
- **PDF table detection false positives**: Table detection precision improved from 50% to 100% by applying `post_process_table()` validation to both the pdfium and OCR table detection paths. Non-table PDFs (simple.pdf, fake_memo.pdf, searchable.pdf, google_doc_document.pdf) no longer produce spurious table detections.
- **Baseline tolerance drift in PDF line grouping**: Line grouping tolerance was computed from the minimum font size across all segments in a line, causing it to shrink when subscripts/superscripts were added. Now anchored to the first segment's font size per line.
- **Paragraph gap detection using minimum spacing**: The paragraph break threshold used the minimum inter-line spacing, which was fragile to outlier-tight spacings from superscripts/subscripts. Changed to 25th percentile (Q1) for robustness.

---

## [4.3.5]

### Added

- **PDF markdown output format**: Native PDF text extraction now supports `output_format: Markdown`, producing structured markdown with headings (via font-size clustering), paragraphs, inline bold/italic markup, and list detection — instead of flat text with visual line breaks.
- **Multi-column PDF layout detection**: Histogram-based column gutter detection identifies 2+ column layouts (academic papers, magazines) and processes each column independently, preventing text interleaving across columns.
- **Bold/italic detection via font name fallback**: When PDF font descriptor flags don't indicate bold/italic, the extractor checks font names for "Bold"/"Italic"/"Oblique" substrings and font weight >= 700 as secondary signals.
- **musl/Alpine Linux native builds for Elixir, Java, and C#**: New Docker-based CI jobs build native libraries (`libkreuzberg_rustler.so`, `libkreuzberg_ffi.so`) targeting `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`. Enables instant install on Alpine Linux and musl-based distributions without compiling from source.
- **Pre-compiled platform-specific Ruby gems**: The publish workflow now ships pre-compiled native gems for `x86_64-linux`, `aarch64-linux`, `arm64-darwin`, and `x64-mingw-ucrt`, eliminating the 30+ minute compile-from-source on `gem install kreuzberg`. A fallback source gem is still published for unsupported platforms.
- **`bounding_box: Option<BoundingBox>` field on `Table` struct**: Added spatial positioning data for table extraction, enabling precise table layout reconstruction. Computed from character positions during PDF table detection.
- **`bounding_box: Option<BoundingBox>` field on `ExtractedImage` struct**: Added spatial positioning data for extracted images, enabling image layout reconstruction in document pipelines.
- **Inline table embedding in PDF markdown output**: Tables are inserted at correct vertical position within markdown content instead of being appended at the end. Position determined by bounding box `y0` coordinate.
- **Image placeholder injection in PDF markdown output**: Image references are inserted with OCR text as blockquotes at correct vertical position matching the image's bounding box.
- **`render_document_as_markdown_with_tables()` function**: New public function for table-aware markdown rendering that embeds tables inline at correct positions and injects image placeholders. Used internally by `render_document_as_markdown()`.
- **`inject_image_placeholders()` function**: New post-processing function for markdown that injects `![Image description]()` placeholders and OCR text blockquotes at correct vertical positions in the content.
- **`bounding_box` field in all language bindings**: Added `bounding_box` (optional `BoundingBox`) to `Table` and `ExtractedImage` types across all 10 language bindings: Python, TypeScript (Node/Core/WASM), Ruby, PHP, Go, Java, C#, and Elixir.

### Fixed

- **Pipeline test flakiness**: Disabled post-processing in pipeline tests that don't test post-processing, fixing `test_pipeline_without_chunking` and related tests that failed due to global processor cache poisoning in parallel execution.
- **PHP FFI bridge missing `bounding_box`**: The PHP Rust bridge (`kreuzberg-php`) was not passing `bounding_box` through for `Table` or `ExtractedImage`, causing the field to always be null despite being defined in the PHP user-facing types.

- **PaddleOCR dict index offset causing wrong character recognition (#395)**: `read_keys_from_file()` was missing the CTC blank token (`#`) at index 0 and the space token at the end, causing off-by-one character mapping errors. Now matches the `get_keys()` layout used for embedded models.
- **PaddleOCR angle classifier misfiring on short text (#395)**: Changed `use_angle_cls` default from `true` to `false`. The angle classifier can misfire on short text regions (e.g., 2-3 character table cells), rotating crops incorrectly before recognition. Users can re-enable via `PaddleOcrConfig::with_angle_cls(true)` for rotated documents.
- **PaddleOCR excessive padding including table gridlines (#395)**: Reduced default detection padding from 50px to 10px and made it configurable via `PaddleOcrConfig::with_padding()`. Large padding on small images caused table gridlines to be included in text crops.
- **Ruby CI Bundler gems destroyed by vendoring script**: The `vendor-kreuzberg-core.py` script was deleting the entire `vendor/` directory including `vendor/bundle/` (Bundler's gem installation). Now only cleans crate subdirectories, preserving Bundler state.
- **PDF document loaded twice for markdown rendering**: Eliminated redundant Pdfium initialization and document parsing by rendering markdown speculatively during the first document load, saving 25-40ms per PDF.
- **NaN panics in PDF text clustering and block merging**: Replaced `expect()` calls on `partial_cmp` with `unwrap_or(Ordering::Equal)` across clustering, extraction, and markdown modules to handle corrupt PDF coordinates gracefully.
- **PDF heading detection false positives**: Added distance threshold to font-size centroid matching — decorative elements with extreme font sizes no longer receive heading levels.
- **PDF list item false positives**: Long paragraphs starting with "1." or "-" no longer misclassified as list items (added line count constraint).
- **Silent markdown fallback**: `tracing::warn` messages for markdown rendering failures are no longer gated behind the `otel` feature flag.
- **PDF font-size clustering float imprecision**: Changed exact `dedup()` to tolerance-based dedup (0.05pt) and added NaN/Inf filtering for font sizes from corrupt PDFs.

- **ExtractionResult typed keyword and quality fields**: `ExtractionResult` now includes typed fields `extracted_keywords: Option<Vec<ExtractedKeyword>>` and `quality_score: Option<f64>` instead of untyped `metadata.additional` entries. Keywords now carry algorithm, score, and position information for better keyword analysis.
- **ProcessingWarning type for extraction pipeline**: New `ProcessingWarning { source: String, message: String }` type added to `ExtractionResult.processing_warnings` to explicitly surface non-fatal warnings during document processing (e.g., recoverable decoding issues, missing optional features).
- **Metadata typed fields**: `Metadata` struct now includes typed fields `category`, `tags`, `document_version`, `abstract_text`, and `output_format` for better structured metadata handling across all language bindings.
- **`output_format` always populated**: The `metadata.output_format` field is now set for all output formats (plain, markdown, djot, html, structured), not just structured. Previously only the structured format populated this field.
- **Language binding updates for typed fields**: All language bindings (Python, TypeScript/Node.js, Ruby, PHP, Go, Java, C#, Elixir) updated with corresponding typed properties matching the Rust API (e.g., `extractedKeywords`, `qualityScore` in TypeScript; `extracted_keywords`, `quality_score` in Python/Ruby).

### Fixed

- **PaddleOCR recognition height mismatch (#390)**: Changed `CRNN_DST_HEIGHT` from 32 to 48 pixels to match PP-OCRv4/v5 model input shape `[batch, 3, 48, width]`. The previous value caused ONNX Runtime dimension errors on all platforms.
- **Go binding: `ChunkingConfig` missing `Embedding` field**: Added `Embedding *EmbeddingConfig` to Go's `ChunkingConfig` struct to match the Rust canonical type. Previously, embedding configuration nested inside chunking was silently dropped during JSON round-trip, causing embedding-enabled extractions to run without embeddings.
- **Go binding: `extracted_keywords`, `quality_score`, `processing_warnings` always nil**: The vendored C header (`packages/go/v4/internal/ffi/kreuzberg.h`) was missing the three new `CExtractionResult` fields, and `convertCResult()` never decoded them. Updated the header and added the missing `decodeJSONCString` calls.
- **`extraction_duration_ms` missing from Go, Java, PHP, C# bindings**: The `Metadata.extraction_duration_ms` field was present in Rust, TypeScript, and Elixir but absent from four bindings. Added the field with proper serialization/deserialization to all four.
- **C# `Metadata.Additional` not marked obsolete**: The deprecated `additional` map (superseded by typed fields) was not marked `[Obsolete]` in C#. Added `[Obsolete]` attribute matching the Rust deprecation. Also added `@Deprecated` in Java and `// Deprecated:` doc comment in Go.
- **Ruby RBS type signatures incomplete**: `packages/ruby/sig/kreuzberg.rbs` lacked struct definitions for all T::Struct types (`ExtractedKeyword`, `ProcessingWarning`, `BoundingBox`, `DocumentNode`, etc.) and inner result classes (`Table`, `Chunk`, `OcrElement`, etc.). Rewrote with comprehensive type definitions matching `types.rb` and `result.rb`.
- **Python `.pyi` stub missing `extraction_duration_ms`**: Added `extraction_duration_ms: int | None` to the `Metadata` TypedDict in `_internal_bindings.pyi`.

### Changed

- **PDF table extraction now computes bounding boxes from character positions**: Table bounding box is calculated as the aggregate bounds of all constituent character positions, enabling precise spatial positioning in downstream rendering pipelines.
- **`render_document_as_markdown()` now delegates to `render_document_as_markdown_with_tables()` with empty tables**: The original function is now a thin wrapper for backward compatibility, with all table-aware rendering logic centralized in the new `_with_tables()` variant.

- **PaddleOCR recognition models upgraded to PP-OCRv5**: Upgraded arabic, devanagari, tamil, and telugu recognition models from PP-OCRv3 to PP-OCRv5 for improved accuracy. All 11 script families now use PP-OCRv5 models.
- **PDFium upgraded to chromium/7678**: Upgraded PDFium binary version from 7578 to the latest release (chromium/7678, Feb 2026) across all CI workflows, Docker images, and task configuration. C API is fully backward-compatible with existing bindings.
- **kreuzberg-pdfium-render trimmed to single version**: Removed support for 22 legacy PDFium API versions (5961-7350 + future), deleting ~328k lines of dead code including bindgen files, C headers, and ~4,256 version-conditional compilation blocks. Removed XFA, V8, Skia, and Win32 feature-gated code paths.
- **Workspace dependency consolidation**: Moved `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, `web-sys`, `console_error_panic_hook`, and `log` to workspace-level dependency management, deduplicating versions across `kreuzberg-pdfium-render`, `kreuzberg-wasm`, and `kreuzberg-ffi`.
- **Docker full image: pre-download all PaddleOCR models**: Replaced broken single-language model download with all 11 recognition script families (english, chinese, latin, korean, eslav, thai, greek, arabic, devanagari, tamil, telugu) plus dictionaries. Fixed incorrect HuggingFace URLs and cache paths. Added retry logic with backoff for transient HuggingFace 502 errors.
- **Docker test suite: PaddleOCR verification**: Added `test_paddle_ocr_extraction` to the full variant Docker tests to verify pre-loaded models work end-to-end.
- **E2E tests updated for typed extraction fields**: End-to-end tests now validate typed `extracted_keywords`, `quality_score`, and `processing_warnings` fields instead of reading from `metadata.additional` dictionary.

---

## [4.3.4] - 2026-02-16

### Fixed

- **Node.js keyword extraction fields missing**: The TypeScript `convertResult()` type converter was silently dropping `extractedKeywords`, `qualityScore`, and `processingWarnings` from NAPI results because it only copied explicitly listed fields. Added the missing field conversions. Also renamed the mismatched `keywords` property to `extractedKeywords` in the TypeScript types to match the NAPI binding definition.
- **Windows PHP CI build failure (`crc::Table` not found)**: Downgraded `lzma-rust2` from 0.16.1 to 0.15.7 to avoid pulling `crc` 3.4.0, which removed the `Table` type used by downstream dependencies.
- **CLI installer resolving benchmark tags as latest release**: The `install.sh` script used GitHub's `/releases/latest` API which returned benchmark run releases instead of actual versioned releases. Changed to filter for `v`-prefixed tags. Also marked benchmark releases as prerelease in the workflow so they no longer interfere.

---

## [4.3.3] - 2026-02-14

### Added

#### Centralized Image OCR Processing

- **Shared `process_images_with_ocr` function**: Extracted duplicated OCR processing logic from DOCX and PPTX extractors into `extraction::image_ocr` module, providing a single shared implementation for all document extractors.

#### Jupyter Notebook Image Extraction

- **Base64 image decoding**: Jupyter extractor now decodes embedded base64 image data (PNG, JPEG, GIF, WebP) from notebook cell outputs into `ExtractedImage` structs instead of emitting placeholder text.
- **OCR on notebook images**: Extracted images are processed with OCR when configured, using the centralized `process_images_with_ocr` function.
- **SVG handling**: SVG images in notebook outputs are handled as text content (not sent to raster OCR).

#### Markdown Data URI Image Extraction

- **Data URI image decoding**: Markdown extractor now decodes `data:image/...;base64,...` URIs into `ExtractedImage` structs with proper format detection (PNG, JPEG, GIF, WebP).
- **OCR on embedded images**: Decoded data URI images are processed with OCR when configured.
- **HTTP URLs preserved as text**: Non-data URIs (HTTP/HTTPS) are kept as `[Image: url]` text markers without attempting network access or filesystem traversal.

#### PaddleOCR Multi-Language Support (#388)

- **80+ language support via 11 script families**: PaddleOCR recognition models now cover english, chinese (simplified+traditional+japanese), latin, korean, east slavic (cyrillic), thai, greek, arabic, devanagari, tamil, and telugu script families.
- **Per-family recognition model architecture**: Shared detection/classification models with per-family recognition models and dictionaries, downloaded on demand from HuggingFace (`Kreuzberg/paddleocr-onnx-models`).
- **Engine pool for concurrent multi-language OCR**: Replaced single-engine architecture with a per-family engine pool (`HashMap<String, Arc<Mutex<OcrLite>>>`), enabling concurrent OCR across different languages.
- **Backend-agnostic `--ocr-language` CLI flag**: Works with all OCR backends (tesseract, paddle-ocr, easyocr). Tesseract expects ISO 639-3 codes (eng, fra, deu); PaddleOCR accepts flexible codes (en, ch, french, korean) via `map_language_code()`.
- **SHA256 checksum verification**: All model downloads verified against embedded checksums for integrity.

### Changed

#### PaddleOCR Engine Internals

- **CrnnNet recognition height**: Changed to 32 pixels (later found to be incorrect for PP-OCRv4/v5 models; fixed in next release).
- **Model manager split**: `MODELS` constant replaced with `SHARED_MODELS` (det+cls) and `REC_MODELS` (11 families) with new cache layout `rec/{family}/model.onnx`.
- **Language code mapping expanded**: `map_language_code()` now handles Thai, Greek, East Slavic, and additional Latin-script languages.

#### DOCX Full Extraction Pipeline (#387)

- **DocumentStructure generation**: Builds hierarchical document tree with heading-based sections, paragraphs, lists, tables, images, headers/footers, and footnotes/endnotes when `include_document_structure = true`.
- **Pages field population**: Splits extracted text into per-page `PageContent` entries using detected page break boundaries, with tables and images assigned to correct pages.
- **OCR on embedded images**: Runs secondary OCR on extracted DOCX images when OCR is configured, following the PPTX pattern.
- **Image extraction with page assignment**: Drawing image placeholders in markdown output enable byte-position-based page number assignment for extracted images.
- **Typed metadata fields**: `title`, `subject`, `authors`, `created_by`, `modified_by`, `created_at`, `modified_at`, `language`, and `keywords` are now populated as first-class `Metadata` fields instead of only appearing in the `additional` map.
- **FormatMetadata::Docx**: Structured format metadata with `core_properties`, `app_properties`, and `custom_properties` available via `metadata.format`.
- **Style-based heading detection**: Uses `StyleCatalog` with `outline_level` and inheritance chain walking for accurate heading level resolution, with string-matching fallback.
- **Headers, footers, and footnote references**: Headers/footers included in markdown with `---` separators; `[^N]` inline footnote/endnote references rendered in text.
- **Markdown formatting**: Bold (`**`), italic (`*`), underline (`<u>`), strikethrough (`~~`), and hyperlinks rendered as markdown.
- **Table formatting metadata**: Vertical merge (`v_merge`) handled correctly, `grid_span` for horizontal merging, `is_header` row detection.
- **Drawing image placeholders**: `![alt](image_N)` placeholders in markdown output for embedded images.

#### DOCX Extractor Performance & Code Quality

- **Eliminated 3x code duplication**: Extracted `parse_docx_core()` helper to deduplicate parsing logic across tokio/non-tokio cfg branches.
- **Removed unnecessary clones**: Metadata structs (core/app/custom properties) borrowed then moved instead of cloned; drawings and image relationships only cloned when image extraction is enabled.
- **Optimized Run::to_markdown()**: Single-pass string builder with pre-calculated capacity replaces clone + repeated `format!` calls on the hot path.
- **In-place output trimming**: `to_markdown()` trims in-place instead of allocating a new String via `trim().to_string()`.
- **Removed `into_owned()` on XML text decode**: Uses `Cow` directly from `e.decode()` instead of forcing heap allocation.
- **`write!`/`writeln!` for string building**: Footnote definitions and image placeholders use `write!` to avoid intermediate String allocations.
- **Safe element indexing**: `to_markdown()` uses `.get()` with `else { continue }` instead of direct indexing to prevent potential panics.
- **Deduplicated document structure code**: Header/footer loops and footnote/endnote loops consolidated using iterators.

### Fixed

#### Extraction Quality Improvements

- **LaTeX zero-arg command handling**: Added explicit skip list for 35 zero-argument commands (`\par`, `\noindent`, `\centering`, size commands, etc.). The catch-all handler no longer consumes the next `{...}` group as an argument, preventing silent text loss for unknown zero-arg commands.
- **Structured data `is_text_field` false positives**: Changed from `.contains()` substring matching to exact equality on the leaf field name. Previously, "width" matched because it contains "id"; "valid" matched because it contains "id". Now only exact leaf name matches are considered.
- **XML dead code in `Event::End` handler**: Removed unused variable allocation and discarded comparison (`let _ = popped == name_owned`), replaced with simple `element_stack.pop()`.

### Removed

- **Dead code cleanup**: Removed unused `Document.lists` field, `ListItem` struct, `process_lists()` method, and `HeaderFooter::extract_text()` method.

---

## [4.3.2] - 2026-02-13

### Fixed

#### PHP 8.4 Requirement Update

- **Updated PHP requirement to 8.4+**: All PHP composer.json files, CI workflows, and documentation now require PHP 8.4+ to support PHPUnit 13.0. This fixes CI validation and PHP workflow failures caused by PHPUnit 13.0 requiring PHP 8.4.1+.

#### Elixir Publishing Workflow

- **Fixed macOS ARM64 build timeout**: Increased timeout from 180 to 300 minutes (5 hours) for macOS ARM64 Elixir native library builds. The previous timeout caused incomplete builds and prevented Elixir v4.3.1 from being published to Hex.pm.

---

## [4.3.1] - 2026-02-12

### Fixed

#### Elixir Package Checksums (#383)

- **Fixed checksum mismatch for Elixir 4.3.0 Hex package**: Updated `checksum-Elixir.Kreuzberg.Native.exs` with correct SHA256 checksums for all 8 precompiled NIF binaries (NIF 2.16/2.17 across aarch64-apple-darwin, aarch64-unknown-linux-gnu, x86_64-unknown-linux-gnu, x86_64-pc-windows-gnu). The 4.3.0 release shipped with outdated 4.2.10 checksums, causing installation failures.

#### Dependency Updates

- **Updated all dependencies across 10 language ecosystems**: Rust, Python, Node/TypeScript, Ruby, PHP, Go, Java, C#, Elixir, WASM, and pre-commit hooks all updated to latest compatible versions.
- **Enhanced dependency update tasks**: All language-specific `task update` commands now upgrade to latest major versions (not just respecting version constraints). PHP, Ruby, C#, Elixir, and Python update tasks enhanced with major version upgrade support.

#### WASM Compatibility

- **Fixed WASM build failures**: Added explicit `getrandom 0.3.4` dependency with `wasm_js` feature to `kreuzberg-wasm` crate to ensure transitive dependencies (ahash, lopdf, rand_core) have WebAssembly support enabled.

#### Dependency Pins

- **Pinned lzma-rust2 to 0.15.7**: The 0.16.1 upgrade is incompatible with crc 3.4.0. Keeping 0.15.7 until upstream compatibility is restored.

---

## [4.3.0] - 2026-02-11

### Added

#### Blank Page Detection

- **`is_blank` field on `PageInfo` and `PageContent`**: Pages with fewer than 3 non-whitespace characters and no tables or images are flagged as blank. Detection uses a two-phase approach: text-only analysis during extraction, then refinement after table/image assignment. Available across all 9 language bindings (Python, TypeScript, Ruby, Java, Go, C#, PHP, Elixir, WASM). Closes #378.

#### PaddleOCR Backend

- **PaddleOCR backend via ONNX Runtime**: New OCR backend (`kreuzberg-paddle-ocr`) using PaddlePaddle's PP-OCRv4 models converted to ONNX format, run via ONNX Runtime. Supports 6 languages (English, Chinese, Japanese, Korean, German, French) with automatic model downloading and caching. Provides superior CJK recognition compared to Tesseract.
- **PaddleOCR support in all bindings**: Available across Python, Rust, TypeScript/Node.js, Go, Java, PHP, Ruby, C#, and Elixir bindings via the `paddle-ocr` feature flag.
- **PaddleOCR CLI support**: The `kreuzberg-cli` binary supports `--ocr-backend paddle-ocr` for PaddleOCR extraction.

#### Unified OCR Element Output

- **Structured OCR element data**: Extraction results now include `OcrElement` data with bounding geometry (rectangles and quadrilaterals), per-element confidence scores, rotation information, and hierarchical levels (word, line, block, page). Available from both PaddleOCR and Tesseract backends.

#### Shared ONNX Runtime Discovery

- **`ort_discovery` module**: Finds ONNX Runtime shared libraries across platforms, shared between PaddleOCR and future ONNX-based backends.

#### Document Structure Output

- **`DocumentStructure` support across all bindings**: Added structured document output with `include_document_structure` configuration option across Python, TypeScript/Node.js, Go, Java, PHP, Ruby, C#, Elixir, and WASM bindings.

#### Native DOC/PPT Extraction

- **OLE/CFB-based extraction**: Added native DOC and PPT extraction via OLE/CFB binary parsing. Legacy Office formats no longer require any external tools.

#### musl Linux Support

- **Re-enabled musl targets**: Added `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` targets for CLI binaries, Python wheels (musllinux), and Node.js native bindings. Resolves glibc 2.38+ requirement for prebuilt CLI binaries on older distros like Ubuntu 22.04 (#364).

### Fixed

#### MSG Extraction Hang on Large Attachments (#372)

- Fixed `.msg` (Outlook) extraction hanging indefinitely on files with large attachments. Replaced the `msg_parser` crate with direct OLE/CFB parsing using the `cfb` crate — attachment binary data is now read directly without hex-encoding overhead.
- Added lenient FAT padding for MSG files with truncated sector tables produced by some Outlook versions.

#### Rotated PDF Text Extraction

- Fixed text extraction returning empty content for PDFs with 90° or 270° page rotation. Kreuzberg now strips `/Rotate` entries from page dictionaries before loading, restoring correct text extraction for all rotation angles.

#### CSV and Excel Extraction Quality

- Fixed CSV extraction producing near-zero quality scores (0.024) by outputting proper delimited text instead of debug format.
- Fixed Excel extraction producing low quality scores (0.22) by outputting clean tab/newline-delimited cell text.

#### XML Extraction Quality

- Improved XML text extraction to better handle namespaced elements, CDATA sections, and mixed content, improving quality scores.

#### WASM Table Extraction

- Fixed WASM adapter not recognizing `page_number` field (snake_case) from Rust FFI, causing table data to be silently dropped in Deno and Cloudflare Workers tests.

#### DOCX Formatting Output (#376)

- Fixed DOCX extraction producing plain text instead of formatted markdown. Bold, italic, underline, strikethrough, and hyperlinks are now rendered with proper markdown markers (`**bold**`, `*italic*`, `~~strikethrough~~`, `[text](url)`).
- Fixed heading hierarchy: Title style maps to `#`, Heading1 to `##`, through Heading5+ clamped at `######`.
- Fixed bullet lists (`-`), numbered lists (`1.`), and nested list indentation (2-space per level).
- Fixed tables missing from markdown output. Tables are now interleaved with paragraphs in document order and rendered as markdown pipe tables.
- Fixed table cell formatting being stripped — bold/italic inside table cells is now preserved.
- Added 16 integration tests covering formatting, headings, lists, tables, and document structure.

#### Typst Table Content Extraction

- Fixed Typst `extract_table_content` double-counting opening parenthesis, which caused the table parser to consume all remaining document content after a `#table()` call.

#### PaddleOCR Recognition Model

- Fixed PaddleOCR recognition model (`en_PP-OCRv4_rec_infer.onnx`) failing to load with `ShapeInferenceError` on ONNX Runtime 1.23.x.
- Fixed incorrect detection model filename in Docker and CI action (`en_PP-OCRv4_det_infer.onnx` → `ch_PP-OCRv4_det_infer.onnx`).

#### Python Bindings

- Fixed `OcrConfig` constructor silently ignoring `paddle_ocr_config` and `element_config` keyword arguments.
- Fixed keyword extraction results (and all `metadata.additional` entries from post-processors) being silently dropped in Python bindings. The `ExtractionResult.from_rust()` method now propagates flattened additional metadata fields, matching all other bindings. Closes #379.

#### TypeScript/Node.js Bindings

- Fixed PaddleOCR config (`paddle_ocr_config`) and element config (`element_config`) being silently dropped by the NAPI-RS binding layer.
- Fixed `ocr_elements` missing from extraction result conversion in TypeScript wrapper.

#### Ruby Bindings

- Fixed `kreuzberg-pdfium-render` vendored crate not included in gemspec, causing gem build failures.
- Fixed PaddleOCR config and element config not being parsed in Ruby binding config layer.
- Fixed `ocr_elements` missing from Ruby extraction result conversion.

#### Go Bindings

- Fixed `PdfMetadata` deserialization failing when keyword extraction produces object arrays instead of simple strings. Added lenient `UnmarshalJSON` fallback with field-by-field recovery.

#### C# Bindings

- Fixed keyword extraction data inaccessible in C# — `ExtractedKeywords` was marked `[JsonIgnore]` and excluded from metadata serialization. Added lenient metadata extraction fallback for mixed-type keyword fields.

#### PHP Bindings

- Fixed `document`, `elements`, and `ocrElements` properties inaccessible on `ExtractionResult` — these fields were not exposed through the `__get` handler.
- Fixed `ExtractionConfig::toArray()` not serializing `include_document_structure`, causing document structure extraction to be silently ignored.
- Fixed wrapper function names for document extractor management (`kreuzberg_*_document_extractors` → `kreuzberg_*_extractors`).
- Added missing OCR backend management functions (`kreuzberg_list_ocr_backends`, `kreuzberg_clear_ocr_backends`, `kreuzberg_unregister_ocr_backend`).
- Fixed `page_count` metadata key mismatch between serialization (`pageCount`) and deserialization (`page_count`).

#### Elixir Bindings

- Fixed NIF config parser not forwarding `include_document_structure`, `result_format`, `output_format`, `html_options`, `max_concurrent_extractions`, and `security_limits` options.
- Added missing document extractor management NIFs (`list_document_extractors`, `unregister_document_extractor`, `clear_document_extractors`).

#### CI

- Fixed PHP E2E tests not actually running in CI — the task was configured to run package unit tests instead of E2E tests.

### Changed

#### Build System

- Bumped ONNX Runtime from 1.23.2 to 1.24.1 across CI, Docker images, and documentation.
- Bumped vendored Tesseract from 5.5.1 to 5.5.2.
- Bumped vendored Leptonica from 1.86.0 to 1.87.0.

### Removed

#### LibreOffice Dependency

- **LibreOffice is no longer required**: Legacy .doc and .ppt files are now extracted natively via OLE/CFB parsing. LibreOffice has been removed from Docker images, CI pipelines, and system dependency requirements, reducing the full Docker image size by ~500-800MB. Users on Kreuzberg <4.3 still need LibreOffice for these formats.

#### `msg_parser` Dependency

- Replaced `msg_parser` crate with direct CFB parsing for MSG extraction. Eliminates hex-encoding overhead and reduces dependency count.

#### Guten OCR Backend

- Removed all references to the unused Guten OCR backend from Node.js and PHP bindings. Renamed `KREUZBERG_DEBUG_GUTEN` env var to `KREUZBERG_DEBUG_OCR`.

---

## [4.2.15] - 2026-02-08

### Added

#### Agent Skill for AI Coding Assistants

- **Agent Skill for document extraction**: Added `skills/kreuzberg/SKILL.md` following the [Agent Skills](https://agentskills.io) open standard, with comprehensive instructions for Python, Node.js, Rust, and CLI usage. Includes 8 detailed reference files covering API signatures, configuration, supported formats, plugins, and all language bindings. Works with Claude Code, Codex, Gemini CLI, Cursor, VS Code, Amp, Goose, Roo Code, and any compatible tool.

#### MIME Type Mappings

- Added `.docbook` (`application/docbook+xml`) and `.jats` (`application/x-jats+xml`) file extension mappings.

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

### Changed

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

#### C

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

#### C

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

- [Configuration Reference](docs/reference/configuration.md) - Detailed configuration options
- [Migration Guide](docs/migration/v3-to-v4.md) - v3 to v4 migration instructions
- [Format Support](docs/reference/formats.md) - Supported file formats
- [Extraction Guide](docs/guides/extraction.md) - Extraction examples

[4.7.2]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.7.2
[4.7.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.7.1
[4.7.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.7.0
[4.6.3]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.6.3
[4.6.2]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.6.2
[4.6.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.6.1
[4.6.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.6.0
[4.5.4]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.5.4
[4.5.3]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.5.3
[4.5.2]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.5.2
[4.5.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.5.1
[4.5.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.5.0
[4.4.6]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.4.6
[4.4.5]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.4.5
[4.4.4]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.4.4
[4.4.3]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.4.3
[4.4.2]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.4.2
[4.4.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.4.1
[4.4.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.4.0
[4.3.8]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.3.8
[4.3.7]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.3.7
[4.3.6]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.3.6
[4.3.5]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.3.5
[4.3.4]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.3.4
[4.3.3]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.3.3
[4.3.2]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.3.2
[4.3.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.3.1
[4.3.0]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.3.0
[4.2.15]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.15
[4.2.14]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.14
[4.2.13]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.13
[4.2.12]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.12
[4.2.11]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.2.11
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
[4.0.0-rc.1]: https://github.com/kreuzberg-dev/kreuzberg/releases/tag/v4.0.0-rc.1
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
