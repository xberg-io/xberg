---
title: "Changelog"
---

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## Archived releases

[Versions 1.0-1.2](/changelog-1.0-1.2/).

<details>
<summary>Links to archived releases</summary>

### [1.2.9](/changelog-1.0-1.2/#129---2026-09-24) - 2026-09-24

### [1.2.8](/changelog-1.0-1.2/#128---2026-09-23) - 2026-09-23

### [1.2.7](/changelog-1.0-1.2/#127---2026-09-22) - 2026-09-22

### [1.2.6](/changelog-1.0-1.2/#126---2026-09-20) - 2026-09-20

### [1.2.5](/changelog-1.0-1.2/#125---2026-09-18) - 2026-09-18

### [1.2.4](/changelog-1.0-1.2/#124---2026-09-18) - 2026-09-18

### [1.2.3](/changelog-1.0-1.2/#123---2026-09-16) - 2026-09-16

### [1.2.2](/changelog-1.0-1.2/#122---2026-09-15) - 2026-09-15

### [1.2.1](/changelog-1.0-1.2/#121---2026-09-14) - 2026-09-14

### [1.2.0](/changelog-1.0-1.2/#120---2026-09-13) - 2026-09-13

### [1.1.5](/changelog-1.0-1.2/#115---2026-09-10) - 2026-09-10

### [1.1.4](/changelog-1.0-1.2/#114---2026-09-09) - 2026-09-09

### [1.1.3](/changelog-1.0-1.2/#113---2026-09-08) - 2026-09-08

### [1.1.2](/changelog-1.0-1.2/#112---2026-09-07) - 2026-09-07

### [1.1.1](/changelog-1.0-1.2/#111---2026-09-07) - 2026-09-07

### [1.1.0](/changelog-1.0-1.2/#110---2026-09-06) - 2026-09-06

### [1.0.14](/changelog-1.0-1.2/#1014---2026-08-04) - 2026-08-04

### [1.0.13](/changelog-1.0-1.2/#1013---2026-08-04) - 2026-08-04

### [1.0.12](/changelog-1.0-1.2/#1012---2026-08-03) - 2026-08-03

### [1.0.11](/changelog-1.0-1.2/#1011---2026-08-03) - 2026-08-03

### [1.0.10](/changelog-1.0-1.2/#1010---2026-08-02) - 2026-08-02

### [1.0.9](/changelog-1.0-1.2/#109---2026-08-02) - 2026-08-02

### [1.0.7](/changelog-1.0-1.2/#107---2026-07-31) - 2026-07-31

### [1.0.6](/changelog-1.0-1.2/#106---2026-07-31) - 2026-07-31

### [1.0.5](/changelog-1.0-1.2/#105---2026-07-30) - 2026-07-30

### [1.0.4](/changelog-1.0-1.2/#104---2026-07-30) - 2026-07-30

### [1.0.3](/changelog-1.0-1.2/#103---2026-07-29) - 2026-07-29

### [1.0.2](/changelog-1.0-1.2/#102---2026-07-28) - 2026-07-28

### [1.0.1](/changelog-1.0-1.2/#101---2026-07-28) - 2026-07-28

### [1.0.0](/changelog-1.0-1.2/#100---2026-07-27) - 2026-07-27

</details>

## [Unreleased]

### Added

- **models:** the experimental candidate inventory (`crates/xberg/src/candidate_models/inventory.json`) now records every staged artifact's owned repository, immutable revision, SHA-256, size and SPDX license, and selects a per-task subset (NLI-MiniLM2 zero-shot; SmolVLM-256M/500M and Florence-2-base captioning) so a consumer never downloads another task's graphs. Downloads are checked by size and SHA-256, and a wrong or floating revision fails before the bytes are read. Every entry stays experimental and unqualified; none is a default or a supported preset. (#2336)
- **pdf:** `pdf_options.extract_vector_geometry` (default `false`) returns each PDF page's vector geometry on `pages[].geometry`: every painted path's subpaths, segments, bounding box, fill colour, stroke colour/width/dash, and Optional Content layer, in page coordinates with the content-stream and Form XObject transforms applied. The coordinate frame is the same one text bounds use, so a label can be matched to the shape it annotates. Off by default because a dense drawing sheet can hold more than ten thousand paths. (#2966)
- **ocr:** `OrtSessionOptions` (`memory_pattern`, `cpu_arena`) now applies to PaddleOCR detection, classification and recognition ONNX Runtime sessions (`max_threads` does not affect PaddleOCR), and `set_ort_session_options` / `ort_session_options` are exposed to the language bindings, so variable-size documents can run with a bounded memory footprint. PaddleOCR engines are pooled per option set. (#3284)- **api:** `XBERG_API_TOKEN` enables bearer-token authentication on every REST route except `GET /health`, and on the MCP HTTP transport (`xberg mcp --transport http`). The token must be ASCII. The Helm chart takes the token from a Secret (`xberg.apiTokenSecret`).

### Changed

- **api:** Breaking. The REST server and the MCP HTTP transport now refuse to start on a non-loopback host (the Docker images bind `0.0.0.0`) unless `XBERG_API_TOKEN` is set or `XBERG_ALLOW_UNAUTHENTICATED=true` opts out (only for a trusted network). CORS is denied unless `XBERG_CORS_ORIGINS` lists origins (`*` allows all); the library `serve*` functions now read `XBERG_CORS_ORIGINS` and `XBERG_JOB_TIMEOUT_SECS` from the environment as the CLI does. `ServerConfig.cors_allows_all()` and `is_origin_allowed()` (exposed in the bindings) change meaning: an empty `cors_origins` list now denies instead of allowing every origin, and only `*` allows all. Also, `DELETE /cache/clear` and `POST /cache/warm` now count against the concurrency limit. Migration: set `XBERG_API_TOKEN` and send `Authorization: Bearer <token>`, or set `XBERG_ALLOW_UNAUTHENTICATED=true` behind a trusted proxy; add browser origins to `XBERG_CORS_ORIGINS`. (#3165)
- **Breaking (libheif):** In the Rust-only `xberg-libheif` crate, `ImageHandle`, `Track`, `RegionItem` and the `Region` geometry types carry the input lifetime (`ImageHandle<'a>`, `Region<'a>`) and share ownership of the context's native state and input, so they stay valid after the `HeifContext` is dropped but cannot outlive a borrowed input; a live region also keeps the context from loading another image, since libheif resolves its transformed getters against that context's image properties. `Reader` now requires `Send` and is called under a lock, since libheif may read from worker threads. `HeifContext` is no longer `Send`. `read_file`, `read_reader` and `read_bytes` return a usage error while handles derived from the context are alive, and methods that take an `ImageHandle` reject one from another context. The `image` integration buffers its input in memory (up to 512 MiB). The removed internal `ImageHandle::context()` wrapped the shared native context without carrying its source. (#3229)
- **Breaking (tesseract):** `xberg-tesseract` iterators are tied to the engine and its current recognition result. `PageIterator`, `ResultIterator`, `ChoiceIterator` and `MutableIterator` gain an `<'api>` lifetime and cannot outlive the `TesseractAPI` that produced them. Their `new` constructors and public `handle` fields are removed, and `TesseractAPI::handle` is `pub(crate)`. `ResultIterator::choices()` replaces raw `ChoiceIterator` construction, and `get_mutable_iterator` returns `MutableIterator`. An iterator call made after Init, `set_image`, `recognize`, `clear`, layout analysis or another call that replaces the result fails with the new `TesseractError::StaleIterator`. `TesseractError` is now `#[non_exhaustive]`. (#3228)
- **Breaking (tesseract):** `Pix::grayscale_bytes` returns `InvalidParameterError` for an image that is not 8 bpp or has a colormap, instead of narrowing raw pixel values. (#3228)
- **pdf/ocr:** Embedded-image OCR reads identical image bytes once and shares the result with every occurrence, so a native PDF that paints one image on every page no longer pays an OCR read per page. (#3260)

### Fixed

- **layout:** `serve` and `extract` read layout models from the Xberg cache (`XBERG_CACHE_DIR/layout`, the platform cache directory by default) before the Hugging Face cache, so the models the Docker image warms at build time are used instead of downloaded again; a warmed model now resolves with `HF_HUB_OFFLINE=1` and no Hub request. `cache warm` defaults to the same root as the runtime instead of `.xberg` in the current directory, and `xberg::cache::resolve_cache_base_dir()` exposes the resolved root. (#3320)
- **ocr:** A recognized dot-leader table row is no longer printed beside the row the detected table already carries. The row claim now compares rows without the leader artifacts a recognizer adds between a label and its values, including a leader that picked up a stray dash, while letters, digits, signs and currency still have to match in order. (#2503)- **tesseract:** The text getters on `TesseractAPI` (UTF-8, hOCR, ALTO, TSV, box, LSTM box, word-str box, UNLV) free the native text when UTF-8 conversion fails. (#3228)

## [1.4.2] - 2026-11-19

### Added

- **pdf:** Warning when a tall scanned page was read at reduced resolution and source-image strip recovery did not run or did not improve it. (#2502)
- **ocr:** `XBERG_TESSDATA_SYSTEM_PATHS=0` (or `false`) stops tessdata resolution from searching the system Tesseract directories.

### Changed

- **cli:** The wrapped Linux musl tarball uses mimalloc with immediate purging and process-wide transparent huge pages disabled by default, including for native libraries and inherited by child processes. The Docker CLI image and static binstall binaries keep their existing allocator. (#3171)
  Set `MIMALLOC_PURGE_DELAY` or `MIMALLOC_ALLOW_THP` before launching to override either setting.
- **docker:** The CLI image uses stable Alpine packages and runs as UID/GID 10001. (#3168)
- **Breaking (ocr):** All-page PDF OCR failures retain the typed backend source and use Rust `XbergError::Ocr`. The raw FFI code changes from 1008 (Plugin) to 1002 (OCR). Python callers should catch `OcrError` where they previously caught `PluginError`, except for a custom Python OCR backend, whose own exception is raised instead (below). REST and batch retain their previous plugin classification. (#3175)
- **Breaking (python):** An exception raised by any custom Python OCR backend callback reaches the caller as the original exception object, including `initialize()` in `register_ocr_backend`, `shutdown()` in `unregister_ocr_backend` and `clear_ocr_backends`, and the `AttributeError` for a missing `process_image*` method. Extraction previously raised `OtherError`, or `OcrError` when every PDF page failed, and the registry functions raised `RuntimeError`. `except xberg.XbergError` no longer catches these failures unless the backend raises an `XbergError` subclass. In `extract_batch` a failed image OCR callback is reported as `ocr` (1002) instead of `other` (1017); an all-page PDF failure keeps `plugin` (1008). A callback raising `xberg.CancelledError` is an ordinary page failure, not a typed cancellation: PDF OCR continues with the remaining pages. Other bindings are unchanged. (#3175)
- **docx:** Parsing and document assembly run on Xberg’s shared native Tokio blocking pool and check for cancellation between parsing stages. (#3171)
- **excel:** Single and batch workbook reads use the shared native blocking pool. Timeouts abort queued reads while in-progress Calamine reads finish before releasing their resources. (#3171)

### Security

- **rendering:** HTML, Markdown and Djot output drop links and image sources whose scheme is not `http`, `https`, `mailto`, `tel` or `ftp` (relative, scheme-relative and fragment links are kept, as are `data:image/` image sources), so a `javascript:` or `vbscript:` link in a source document no longer becomes clickable script; the link text is kept.
- **install:** The install script verifies CLI release archives against SHA-256 digests from GitHub release metadata before extraction. (#3168)
- **ocr:** Runtime Tesseract language-pack downloads are verified against pinned SHA-256 digests and use bounded timeouts. (#3168)

### Fixed

- **chunking:** `ChunkSizing::Tokenizer` with a registered tokenizer backend whose `count_tokens` includes special tokens keeps every chunk within `max_characters` tokens; chunks were over by up to the special-token count. (#3291)
- **pdf:** Native extraction keeps two separately outlined ruled tables with a blank gap between them apart and in page order in Plain and Markdown, unless they share both width and column count. (#3286)
- **ocr:** A process no longer crashes at exit when a page abandoned by the extraction time limit releases its Tesseract engine after `exit()` has begun. Recognitions still in flight at exit are stopped and get up to 2 seconds to return. (#3290)
- **pdf:** CCITT-compressed scans with `/Rows 0` use the image height, so their pages render and can be read by OCR. (#3253)
- **ocr:** PaddleOCR-VL and DeepSeek OCR reject images over their decoding budget before model loading.
  DeepSeek also budgets local crop resizing, so images accepted before may exceed the limit. (#3168)
- **ocr:** TrOCR checks decode, resize and tensor buffers before allocating pixels. Previously accepted images can exceed the
  configured budget. The standalone API exposes typed image errors and accepts decoded rasters through `process_decoded_image`.
  The dimensions helper reads the header without validating pixels. (#3168)
- **ocr:** GLM-OCR honors the configured image budget in both modes. Its standalone byte APIs use a 100 MiB decode and RGB-conversion budget. GLM decode failures now return `CandleOcrError::Image` in place of `InferenceFailed`. Use `GlmOcrEngine::process_rgb_image_with_task` for decoded RGB input. (#3168)
- **pdf:** Native extraction handles scanner PDFs that add spaces or tabs between `stream` and the line ending, preserving the declared stream bytes. (#3252)
- **pdf:** Native structured extraction keeps verified footnotes as `Footnote` content and omits them from Plain and Markdown bodies unless `include_footnotes` is set. (#2331)
- **runtime:** Dynamically loaded ONNX Runtime libraries are discovered without changing the process environment. Sceptre OCR reports the native loader error if a discovered library cannot be loaded. (#3172)
- **pdf:** Explicit zero page margins preserve the same OCR text recovery as omitted margins. (#3172)
- **ocr:** In Plain output, scanned PDFs rejoin witnessed words split across eligible lines and retain lexical hyphens; Markdown already joined line-end hyphens. (#3170)
- **pdf:** Structured warnings stay with the document that produced them, including when fonts are shared across documents. (#3172)
- **pdf:** A running head whose page number is set off by spaces no longer makes a two-column page interleave its columns.
- **pdf:** Tick labels under a row of plot panels are no longer emitted as a Markdown table.
- **rtf:** No space is inserted where an inline group closes, so `12{\i 34}56` extracts as `123456`. (#3176)
- **ocr:** Typed backend cancellation stops PDF OCR before further page batches or fallback and returns `XbergError::Cancelled`. (#3175)
- **pdf:** Bold column labels under a single merged table title are kept as headers in HTML and Markdown. (#2332)
- **pdf:** Layout-assisted TATR recognizes source dividers and preserves source-supported row and column spans. (#2332)
- **ocr:** Default sparse-text OCR keeps a certified standalone table with one unreadable value as a table. (#2503)
- **ocr:** Tesseract PSM 6 preserves verified table row breaks in unified HTML for single-page PDF scans. (#2503)
- **pdf:** Recover sparse missing glyphs with a 120 DPI trial on eligible Tesseract pages in the default OCR pipeline. (Closes #3238)

## [1.4.1] - 2026-11-16

### Added

- **ocr:** Warning when resizing a standalone tall image may leave text incomplete. (#2502)

### Security

- **api, mcp:** REST and MCP requests are rejected when their config names a server file, directory or model source (`html_output.css_file`, `tree_sitter.cache_dir`, `ocr.tessdata_path`, model cache directories, custom model ids, `concurrency`); `/extract` merges the request config over the server config, and a request can only lower security limits, archive depth, timeouts and URL caps.
- **api:** At most `XBERG_MAX_CONCURRENT_REQUESTS` `/extract-async` jobs run at once, separately from synchronous requests; further jobs stay `pending`.
- **api, mcp:** Remote REST or MCP-over-HTTP configs that add or change an LLM stage are rejected unless `XBERG_ALLOW_CALLER_LLM_ENV_CREDENTIALS=1` is set, so callers cannot use the server's LLM credentials.
- **mcp:** Over HTTP, `extract` and `extract_batch` reject local paths and `file://` URIs and `detect_mime_type` is refused, unless `XBERG_MCP_ALLOW_LOCAL_URI_INPUTS=1` is set.
- **mcp:** Over HTTP, tool errors, per-input errors and `cache_stats`/`cache_clear`/`cache_warm` no longer reveal server filesystem paths; `cache_warm` no longer names model cache paths on either transport.
- **api:** Error responses, batch per-input errors and `/cache/stats`, `/cache/clear`, `/cache/warm` no longer reveal server filesystem paths.
- **hwp:** Compressed HWP body-text sections are inflated under `security_limits.max_content_size` each and `max_archive_size` in total.
- **archive:** `max_archive_size` caps decompressed bytes across all nesting levels of an archive and `max_content_size` caps the combined output of its children; what does not fit is skipped with a warning.

### Fixed

- **ocr:** OCR progress is reported once per page, even when a fallback backend or retry reads it again. (#3174)
- Output is stable between runs: heading levels, list indentation, OCR font names and `metadata.additional` key order no longer depend on hash iteration order.
- **rtf:** `\deleted` tracked-change text is dropped from extracted content. (#3164)
- **onnx:** Concurrent ONNX inference no longer shares a session mutably (undefined behavior). (#3166)
- **fictionbook:** Security-limit violations inside paragraphs are reported instead of silently accepted.
- **pdf:** Better paragraph reading order in scanned multi-column PDFs. (#2331)
- **pdf:** Adjacent tables with aligned columns stay separate. (#2332)
- **pdf:** Verified decimal column headers are preserved and decimal records stay in table bodies in HTML. (#2332)
- **ocr:** False short words and repeated texture reads are removed from noisy scanned PDFs in Plain and Markdown output. (#2943)
- **ocr:** Tall scanned PDFs: verified panel dimensions, headings, multiline headings, split words and lines that collapsed to punctuation are recovered. (#2502)
- **tables:** Tables are recovered on forced-OCR PDF pages that need a rotation retry. (#2332)
- **tables:** Verified column headers above repeated numeric groups in PDF tables are preserved. (#2332)
- **ocr:** Tesseract table output for standalone images and embedded PDF images no longer duplicates rows and keeps rows, row breaks and dot leaders intact (PSM 3, 6 and 11). (#2503)
- **ocr:** Opt-in recovery for PaddleOCR ruled JPEG table cells (English PP-OCRv6 Small, ONNX Runtime), off by default. (#2503)

## [1.4.0] - 2026-11-13

### Added

- **math:** Explicit math text annotations (`AnnotationKind::math(display)`, `math_display()` in Rust; `Custom` annotation `xberg.math` with value `inline` or `display` in other bindings) render as LaTeX in Markdown, HTML and Djot.
- **ocr:** Corroborated Khmer numbers are recovered on eligible English-primary PDF pages when `khm.traineddata` is installed. (#2408)
- **layout:** Experimental `LayoutDetectionConfig.model` (`--layout-model`) selects `rt_detr` (default) or `pp_doc_layout_v3`; PP-DocLayout-V3 requires ONNX Runtime and is an error on tract or WASM builds. (#2284)
- **ocr:** Oversized PDF scans are read from their embedded JPEG at the original resolution when a page render exceeds the image budget. (#2530)
- **classification:** Experimental Rust API for CPU ONNX NLI classification. (#2335)
- **layout:** Rust `Engine::extract_with_layout_region_text` returns source-proven text for selected native PDF layout regions in reading order. (#2357)
- **batch:** Per-input `use_layout_for_markdown` override for REST `/extract`, MCP `extract_batch` and the CLI `--file-configs` map. (#2367)
- **structured:** Opt-in partial-response recovery with batch-local validation diagnostics. (#2318)
- **structured:** Caller-selected image size limits and option-aware render caching. (#2319)
- **layout:** `extract_with_layout_pages` selects PDF pages (1-based) for layout detection. (#2255)
- **pdf:** Rust `extract_page_text_with_script_geometry` retains script baselines.
- **api:** `include_image_data: false` omits raw image bytes from extraction responses (JSON and multipart, sync and async). (#2187)
- **pdf:** `PdfError` converts from Lopdf 0.46 errors as well as Lopdf 0.45.
- **llm:** Rust `xberg::llm::usage::usage_from_error` exposes token usage of failed structured responses. (#2139)

### Changed

- **mcp:** Extraction responses omit empty error and crawl URL lists and a zero redirect count. (#2785)
- **python:** Archived releases are installable from the GitHub package index `https://xberg-io.github.io/python-wheels/simple/`; PyPI keeps all releases of the current minor and the latest stable patch of each older minor.
- **html:** Conversion options add `expand_abbreviations` and `include_blockquote_citations` (default on) and `hidden_content` (default drop). Rust exhaustive `ConversionOptions` literals, Java canonical record constructors and Kotlin positional arguments must account for the new fields.
- **url:** Crawler configuration adds content options; query-aware deduplication is on by default. Rust exhaustive crawler configuration literals need the new fields.
- **pdf:** JPEG 2000 palette images clamp out-of-range color indices instead of failing. (GH#2112)
- **html:** `headers[].text` is plain text; superscript and subscript markup no longer appears.
- **excel:** A sheet over `security_limits.max_table_cells` is returned as an empty placeholder with a warning instead of failing with `SecurityError::TooManyCells`; legacy XLS/XLA workbooks with duplicate sheet names still fail. (#2820)
- **markdown:** Math is written as `$...$` and `$$...$$` instead of `` $`...`$ `` and ```` ```math ```` (kept only for TeX that cannot be delimited); TeX is escaped to stay inside its delimiters and a prose `$` next to math becomes `\$`. (#2331)
- **Breaking (ocr):** `OcrConfig.auto_rotate` is tri-state (`Option<bool>` in Rust, nullable in bindings): `false` never rotates or retries, `true` always rotates, unset keeps the orientation retry. Positional constructors moved the field (14th in Python, PHP and WebAssembly); the C ABI adds `xberg_ocr_config_has_auto_rotate`. (#2496)
- **ocr:** A standalone picture that looks like a table is re-read as one uniform block (Tesseract PSM 6) and that read is adopted; such pictures are OCR'd twice. (#2503, #2432)
- **layout:** The `pp_doclayout_v3` model is PaddlePaddle's official ONNX export, re-hosted at `xberg-io/layout-models`. (#2284)
- **ocr:** The default VLM OCR prompt asks the model to transcribe repeated text in order instead of merging it. (#2132)
- **pdf:** Native PDF text expands ligature code points U+FB00 to U+FB06 into their letters in every output; the native `expand_ligatures` option has no effect and is deprecated. (GH#2322)
- **pdf:** Native text extraction allocates less. (#2429)
- Minimum supported versions raised: Node.js 22, Ruby 3.3, PHP 8.3, Go 1.27, Java 25, Android API 24, Dart 3.13, macOS 15, Rust 1.99.
- **python:** Minimum Python is 3.11; wheels target the `abi3-py311` stable ABI.
- **macos:** Every macOS artifact (wheels, CLI, FFI and Node libraries, Elixir NIF, Swift package `.macOS(.v15)`) targets macOS 15.0, previously 11.0 (Swift 26).
- **go:** 50 DTOs implement `json.Marshaler`; 74 required collection fields encode nil as `[]`/`{}`. Anonymously embedding these types promotes the method and drops the wrapper's extra fields.
- **plugin:** The agent plugin uses the Xberg symbol for its icon and logo.
- **Rust:** Crawlberg configuration types gained upstream fields and `WafBlocked` gained status and body fields; exhaustive literals and matches must be updated.
- **elixir:** Native builds accept compatible newer Brotli decompressor and allocator releases instead of pinning decompressor 6.0.1.
- **ocr:** Statically built Tesseract (`build-tesseract`, `build-tesseract-wasm`) is 5.5.4 and requires CMake 3.22 or newer.

### Security

- **deps:** `rmcp` 3.9.0 fixes unbounded response-body reads (RUSTSEC-2026-0462).
- **pdf:** Whole-document OCR retries share stored JPEG payloads, bounding retained bytes. (#2339)
- **pdf:** OCR retries of embedded scans after a pipeline failure honor image decode limits and cancellation. (#2339)
- **excel:** Calamine 0.38.0 fixes a panic and unbounded allocation on malformed CFB/OLE2 spreadsheets (RUSTSEC-2026-0436).
- **build:** zlib 1.1.31 (RUSTSEC-2026-0418).
- **rendering:** Comrak 0.59 removes quadratic work when rendering user-controlled HTML table cells.
- **excel:** XLSX, XLSB, XLS and ODS enforce spreadsheet limits before allocating worksheet buffers and table grids. (#2296)
- **crawl:** Custom headers stay scoped to the seed host when following subdomains (Crawlberg 1.12.3); API requests cannot override the TLS certificate-verification policy.
- **integrations:** Integration package locks updated for advisories: filelock 4.2.4, setuptools 84.2.1 (GHSA-xmj3-9gf7-h52w), Pillow 13.0.0 and NLTK 3.10.4 (Python 3.11 and later).
- **crypto:** ctutils 0.4.5 restores constant-time masks (RUSTSEC-2026-0385).
- **pdf:** Font lookup preprocessing is bounded and malformed lookup types are rejected; extended glyph IDs no longer paint unrelated glyphs.
- **runtime:** Tokio 1.54.1 fixes a data race and use-after-free in async timers.
- **tls:** rustls-webpki 0.103.16 rejects distrusted emSign intermediates (RUSTSEC-2026-0382); bundled trust stores omit the emSign C1 and C3 roots.
- **excel:** ZIP-backed spreadsheets reject mismatched local headers, duplicate member names and overlapping entries.

### Fixed

- **pdf:** The page failure record and warning agree when native text is restored after forced OCR fails. (#3148)
- **ocr:** Automatic script retries reuse Tesseract engines; OCR caches refresh when the installed model changes. (#3153)
- **pdf:** Pages with a few unmapped glyphs are OCR'd at a lower resolution first. (#3150)
- **extraction:** Exact page byte ranges are preserved when page text cannot be located or is out of order; ambiguous ranges are omitted. (#3149)
- **pdf:** Standard Latin Type 1 fonts render with WinAnsi glyphs when the text map disagrees, so OCR reads the visible text. (Refs #2826, #3147)
- **pdf:** Overprinted process-color text stays visible in rasters used for OCR. (Refs #2817)
- **pdf:** Tables on rotated scanned pages are recovered during forced layout OCR, with bounds in page points. (#3131)
- **pdf:** Packed RGB pixels with indirect predictor settings are preserved in PNG export; predictor-wrapped JPEGs keep their original bytes. (#3115)
- **elixir:** Binary extraction input, including non-UTF-8 bytes, is passed through unchanged.
- **pdf:** Nested equation superscripts and subscripts stay attached to their own bases. (#3111)
- **pdf:** Columns stay separate when one text string spans a wide tracking gap.
- **pdf:** Minus signs from unmapped math-axis bar glyphs in symbolic Type1 fonts are preserved. (#2130)
- **tracing, ocr:** Parallel PDF OCR keeps the caller's span context. (#2131)
- **wordperfect:** Plain output retains footnote and endnote bodies. (#2129)
- **pdf, ocr:** Page boundaries match the final Plain output, including UTF-8 and page separators. (#2128)
- **wasm:** OCR-enabled WebAssembly builds link with Rust 1.99.
- **wasm, url:** URL extraction no longer decompresses a response that Fetch already decoded.
- **docx:** Inline equations stay in their sentences and in the formula inventory. (#2123)
- **docx, pptx:** OMML equations resolve character references in attribute values, render `m:groupChr` braces as `\underbrace`/`\overbrace`, and keep the degree of n-th roots.
- **pdf:** Recognized formulas stay attached to their own regions.
- **macos:** Native libraries keep the Mach-O alignment required by the system loader.
- **pdf:** Markdown honors header, footer and watermark opt-ins for tagged pagination artifacts. (#2120)
- **pdf:** Table numbers split across touching text runs keep their thousands separators without inserted spaces. (#2122)
- **pdf:** Decoded JPEG 2000 images are reported as 8 bits per component. (#2112)
- **ppt:** Tables on legacy `.ppt` slides are extracted as tables.
- **html:** Markdown output passes fenced code blocks through unchanged. (GH#2118)
- **macos:** Extraction no longer hangs in hosts that ignore `SIGCHLD`, including the BEAM. (#2117)
- **pdf:** A `•` inside a table row's text no longer turns the row into list items. (GH#2115)
- **pdf:** Two book pages printed side by side are read left page then right page again (regression in 1.3.9). (GH#2116)
- **php:** The Linux extension no longer crashes at PHP shutdown. (#2113)
- **pdf:** Tables on a scanned page on the layout-detection route are reported in page points. (GH#3078)
- **pdf:** Readable Khmer text drawn over a full-page image is kept when OCR cannot replace it. (#2826, #3016)
- **ocr:** Failed-page warnings are kept when the PDF time limit skips embedded-image OCR retries; partial results are not cached. (GH#3102)
- **pdf:** Complex spot-color pages render within the PNG memory budget, at reduced DPI if needed. (#2870)
- **pdf:** Native text is preserved when a PDF OCR read is discarded. (#3013, #3014, #3129)
- **pdf:** Duplicate text is removed from automatic mixed OCR output while split words, repeated identifiers and OCR-only cells are kept. (#2534, #3105)
- **ocr:** Sideways prose is recovered on automatically selected scan pages in mixed PDFs. (#3050, #3062, #3092, #3093)
- **ocr:** Whole-document Tesseract OCR in Paddle-enabled builds keeps sideways-prose recovery and orientation metadata. (#3007)
- **pdf:** Malformed TrueType character maps no longer hang extraction or rendering; they fall back to the declared encoding. (#3034, #3064)
- **pdf:** Malformed embedded font offsets no longer crash the renderer. (#3019)
- **pdf:** A column cut no longer runs through a sentence whose inline script fills a line gap. (GH#2925)
- **ocr:** When automatic OCR takes a whole page with a partly decodable text layer, low-confidence misreads take the layer's spelling and words the read dropped are restored. (GH#2952, GH#3026, GH#3027, GH#3028)
- **pdf:** Backend OCR documents with non-PDF source formats (such as `hocr`) re-render accepted OCR pages from their elements. (#2939, #3014, #3129)
- **extract:** A PDF extraction that reaches `extraction_timeout_secs` returns the pages already read with a `timeout` warning instead of failing; partial results are not cached. (GH#2958)
- **extract:** A native PDF cut short by the time limit returns promptly without tables, images, form fields, document structure, scan metadata or chunk embeddings, with a `pdf_time_limit` warning. (GH#3029)
- **extract:** `extraction_timeout_secs` also bounds chunk embedding, post-processors and structured extraction; stages it ends are dropped with a `time_limit` warning. (GH#3044)
- **extract:** PDF pages OCR did not reach before the time limit are reported as unread with `is_blank` unset. (#3055, #3056)
- **go:** linux-x86_64 and linux-aarch64 Go release archives ship usable static-link flags. (GH#2988)
- **pdf:** Readable native text on pages with unmapped glyphs is preserved during automatic OCR. (#2517, #2637)
- **pdf:** Pages with knockout groups that exceed the render work limit no longer render twice, and pages within it no longer fall back to rendering without knockout. (#2955, #3002, #3058, #3060, #3098)
- **ocr:** Khmer number recovery remains available for later pages after bounded source declines. (#2905)
- **djot:** Straight quotes, apostrophes and runs of dots or dashes in marked rows are preserved. (#2913)
- **djot:** Block-like text in paragraphs, lists and OCR output is preserved, including markup when nested OCR declares Markdown or Djot. (#2891)
- **doc:** Legacy `.doc` list paragraphs are numbered from the document's own list sequences, including roman, letter and `(1)` styles, across intervening headings. Djot output now writes a decimal source label (`1.`) as an ordered-list marker for DOCX, PDF and image extraction. (#2857)
- **ocr:** Symbol-noise lines are pruned on the direct PDF read of a flagged page, not only on the layout and pipeline routes. (#2921)
- **api:** Async jobs expire 5 minutes after they finish instead of after submission; rejected, crashed and cancelled jobs release their slot, and invalid input gets 400 even when the job store is full. (#2621)
- **ocr:** Numeric-only OCR reads no longer count as retaining native prose.
- **ocr:** Only OCR-produced page text is re-rendered as plain; DOCX pages and native pages of mixed PDFs keep their text, footnotes and equation spacing. (#2839, #2902)
- **docx:** Plain content no longer contains paragraph style notes, Markdown link targets, `[Image: ...]` placeholders for unnamed drawings, or inline Markdown markup in table cells. (GH#2947)
- **pages:** For non-PDF sources (DOCX, PPTX, XLSX), each Plain page's text is its part of `content` and its page range points at that text; XLSX sheet pages start with the sheet name in every format; chunk pages follow. (GH#2946)
- **docx:** The description of a drawn shape is written in its place in `content` in every format. (GH#2946)
- **pdf:** Recognized tables render as plain text on the mixed OCR route. (#2903)
- **pdf:** OCR tables are placed in page points when recognition ran on a resampled page image (scans below 300 dpi). (#2990)
- **djot:** Paragraph text that begins with block syntax is escaped and preserved. (#2856, #2869)
- **pdf:** Nested superscript and subscript stacks stay on their base's line. (#2728)
- **djot:** Dot-leader table rows keep all their text, including underscores, stars, brackets and backslashes. (GH#2867, GH#2972, GH#3009)
- **pdf:** Script stacks keep their markup after page ordering, and large-delimiter pieces are not treated as scripts. (#2904, #2884)
- **pdf:** Knockout groups composite against the initial group backdrop; translucent paints, opacity, blend modes and soft masks render consistently, and a group over its work limit renders without knockout with a warning. (#2855)
- **pdf:** Fractions in exponents or indices are no longer read as script stacks (`x^{-1/2}` no longer reads `x^{-2}`). (#2931)
- **pdf:** A base's subscript is read apart from its exponent and the exponent's index. (#2926, #2987, #3010, #3080, #3110)
- **pdf:** Correctly mapped Greek, Cyrillic or Arabic text on an English page with a lost glyph is kept instead of being sent to OCR. (#2861)
- **ocr:** Fragmented symbol lines are removed from images and PDF pages flagged for OCR noise; `OcrQualityThresholds.prune_symbol_noise_lines` controls this. (#2312, #2887)
- **pdf:** Horizontal scaling (`Tz`) applies to string advances, glyph outlines, fallback glyph boxes, invisible text on separation plates and Type 3 glyphs in page renders. (#2677, #2914, #2915, #2957)
- **ocr:** Builds without Paddle can recover an eligible RGB JPEG scan after a render-budget refusal. (#2849)
- **pdf:** Primary word-boundary mode measures run widths consistently when a text matrix moves back into a run. (#2845)
- **ocr:** A blank read of a clamped background image no longer marks the page's other text as illegible. (#2842)
- **ocr:** Each scanned image on a page gets its own noise verdict. (GH#2853)
- **ocr:** Oversized single-page scans are recovered under an explicit OCR pipeline. (#2907)
- **ocr:** OCR metadata and `image_preprocessing` come only from retained embedded-image reads. (#2918)
- **ocr:** Page byte ranges are kept under `OutputFormat::Plain` when OCR page text renders tables differently; `pages[].content` no longer carries Markdown tables there. (#2825)
- **pdf:** Compound glyph names at printable font codes (`/f_f_i`) extract their letters. (GH#2834)
- **ocr:** A large scanned page's embedded-image retry read follows `discard_suspected_ocr_noise` instead of always being dropped as noise. (#2832)
- **ocr:** Embedded-image recovery is reported only when the read survives the noise check. (#2908)
- **ocr:** A sideways paragraph of short words is read as prose. (#2670)
- **excel:** A workbook over `security_limits.max_table_cells` skips only the sheets that do not fit, with a warning per sheet, instead of failing. (#2820)
- **pdf:** An English page whose `ToUnicode` map sends its letters to another script is checked with OCR instead of being kept as native text. (#2830)
- **pdf:** Underscore-joined `/Differences` glyph names decode to all their letters (Adobe Glyph List rules). (#2689)
- **pdf:** Words are no longer split when a producer sets the text matrix back over a run before a large TJ offset (`benefi ts`). (GH#2322, GH#2690)
- **pdf:** Runs drawn without font info keep every glyph in OCR rasters. (#2709)
- **pdf:** Page numbers painted by a form XObject with a separate resource dictionary render all digits. (#2685)
- **pdf:** A block at the right of a page that sits wholly above a left block is read first. (#2680, #2803)
- **pdf:** OCR no longer splits words at TrueType ligature glyphs (`fi nancial`); Markdown, HTML and Djot output of OCR pages changes accordingly. (GH#2322)
- **pdf:** Pages of outlined prose with eight or more rows, or with a picture, are recovered. (GH#2313)
- **pdf:** Ligature glyphs missing from the ToUnicode map return their letters from the font's GSUB ligatures; several horizontal-scale and TJ seam cases no longer insert spaces inside words. (GH#2664)
- **pdf:** arXiv citations and the words before them are kept in Markdown; only the margin stamp is removed, including stamps split into parts. (#2650)
- **ocr:** A process exiting while a thread is still recognizing no longer crashes or segfaults.
- **pdf:** A space drawn inside a ligature's box no longer splits the word; a ToUnicode value of U+0000 falls back to the `/Differences` glyph name. (GH#2322)
- **ocr:** CCITT preview work for invisible-layer probes is budgeted per page, so every page of a multi-page scan can be recovered. (#2694, #2730)
- **ocr:** Automatic all-pages Tesseract OCR keeps native text when the read duplicates it. (#2768)
- **ocr:** Singleton `[/FlateDecode]` filter arrays are decoded like `/FlateDecode` in PDF stream probes and Indexed scan recovery. (#2798)
- **pdf:** Spot-colour plates keep their ink under separable blend modes, per ISO 32000-1. (#2814)
- **pdf:** A Type0 page's native text is kept after OCR ink loss only when the native layer owns, is covered by, or substantially matches the OCR read. (#2802)
- **pdf:** Separation plates handle `/BM` arrays with indirect elements, malformed soft masks and non-standard blend-mode names consistently with the page renderer. (#2735)
- **pdf:** Ink-certified native table pages are preserved when their OCR structure omits words. (Refs #2534)
- **pdf:** A page keeps its native text, with an `ocr` warning, when the OCR stage for its script fails and a later stage for another script reads none of it; affected multi-page documents are re-read page by page. (#2794, #2810)
- **ocr:** Dot-leader table pictures print each row once, including when the row also appears in the table. (#2783)
- **pdf:** A complete Khmer (or other cluster-script) text layer is kept when a later pipeline stage cannot read the script. (#2799)
- **pdf:** Words stay joined after a ligature glyph drawn as its own run (`diff erent`); `TextSpan` gains a `ligature_glyph` field. (GH#2718)
- **ocr:** Standalone dot-leader table pictures with a garbled first read are re-read as one block and the block read is adopted; each row is its own line in Markdown, HTML and Djot. (#2503, #2804)
- **djot:** Every dot-leader table row that starts with punctuation is escaped so it stays one paragraph. (#2838)
- **ocr:** The native page is kept when OCR differs only by hanging underscores or non-text-block bookkeeping. (Refs #2534)
- **bindings:** The `LayoutDetectionConfig` constructors in Python, PHP and WASM have their released eight-argument order again, with `model` last and optional; Java and Swift gain the legacy eight-argument constructor. (#2764, #2717)
- **pdf:** A lone accent letter above a formula line is no longer joined into it (`bX`). (GH#2719)
- **pdf:** A repeated page stamp whose words appear in different orders is removed as page furniture. (GH#2719)
- **ocr:** A picture keeps its first read when the table re-read finds no table. (#2732)
- **ocr:** Han (`chi_sim`) and Khmer (`khm`) text can be recovered with installed models without changing the configured languages. (Refs #2300)
- **pdf:** Graphics states recovered while visiting a Form XObject apply to later strokes. (#2752)
- **pdf:** Pages with many graphics-state names no longer cost quadratic allocations in native path extraction. (#2744)
- **pdf:** A table is printed once when two overlapping table detections hold the same words. (#2739)
- **ocr:** Sideways scans with two stacked tables and prose are recovered, with table boxes in the source PDF frame. (#2497)
- **pdf:** A painted orphan glyph beside a table is removed whatever the glyph (`%`, `-`, currency, non-Latin digits). (#2616)
- **ocr:** Leader runs no longer exempt a whole glued native token from the glued-cell check. (#2704)
- **ocr:** Grayscale preprocessing without binarization uses less memory. (#2699)
- **pdf:** Native path extraction honors ExtGState dash patterns and zero stroke opacity, preventing false HTML table headers. (#2332)
- **pdf:** Native leaf headers beneath repeated column groups are preserved in HTML and document grids. (#2332)
- **ocr:** Words hidden by a glued native token on a table page keep the OCR read. (#2691)
- **pdf:** Math lines such as `X = Y` or `3.2 ± 0.1` are no longer read as numbered headings. (#2688)
- **ocr:** Tall neutral RGB scans are recovered after preprocessing reduces their read resolution; read-frame diagnostics survive per-page OCR. (#2502)
- **pdf:** A native table overlapping a grid that does not hold its rows is no longer dropped. (#2376)
- **ocr:** Table reads with escaped cell pipes are compared correctly with native text, so the native page is kept. (Refs #2534)
- **ocr:** The native page is kept when automatic OCR reads it as a table of the same words. (Refs #2534)
- **php, elixir:** The macOS PHP extension and Elixir NIF load without Homebrew's libheif; the PHP extension links libheif, libde265 and libaom statically (HEIF output unavailable) and the NIF ships its codec libraries. (#2547)
- **pdf:** A ligature drawn as its own run joins the rest of its word in Plain and Markdown output. (#2322)
- **pdf:** An authored space after a ligature (`staﬀ only`) is kept in Plain and Markdown output. (#2322)
- **pdf:** An over-budget embedded JPEG page image is decoded at a reduced scale instead of refusing the page. (#2682)
- **ocr:** `max_threads` applies to PaddleOCR ONNX Runtime sessions. (#2649)
- **pdf:** Prose behind a full-page grayscale ICC image is recovered by OCR. (#2313)
- **pdf:** Raised exponents and footnote marks stay inside their sentence on native pages. (#2647)
- **pdf:** Reading order no longer moves list items and words out of their sentence on native pages. (GH#2658)
- **pdf:** Printed folios and rotated stamps stay separate from nearby prose, and raised table values stay with their row. (#2647, #2658)
- **pdf:** Unversioned arXiv identifiers in citations are no longer removed as page furniture. (GH#2644)
- **ocr:** Raster-proven dot-leader scan rows are recovered whatever their header, label and value text. (GH#2616)
- **ocr:** Dot-leader table rows with values such as `$1,010` or `3.2%`, or labels with digits, are matched correctly. (GH#2616)
- **ocr:** Sideways prose regions are admitted by word-box geometry, so punctuation, digits and short words no longer block the retry. (GH#2616)
- **ocr:** Explicitly rotated tables on the per-page route get in-page bounds. (#2497)
- **ocr:** An implicitly rotated table is recovered together with a heading or prose above or below it. (#2497)
- **ocr:** Tall-scan recovery covers Indexed scans down to 120 DPI, Markdown output and explicit Tesseract page modes, and warns when strips were not used. (#2502)
- **pdf:** A native page in scripts the configured OCR languages cannot read (marked or unmarked, including through a Tesseract pipeline) is kept instead of OCR'd, with an `ocr` warning naming the model to add; scans in such scripts are still read. (GH#2536, #2536)
- **pdf:** A Tesseract pipeline whose first stage reads the script OCRs the page; pages with many scripts no longer fail with an index error; warnings name the setting the languages came from. (#2738)
- **pdf:** A bullet in a larger font or drawn after its text no longer splits its list item. (#2443)
- **pdf:** Table rows are no longer repeated as prose when a marker or sign splits a cell. (#2443)
- **pdf:** Rules from another section no longer split a ruled band into columns. (#2443)
- **ocr:** A wrapped figure caption is kept as text instead of table cells (uncached Plain extraction with English Tesseract defaults). (#2616)
- **engine:** `Engine::extract`, batch and layout-page extraction run on a 16 MiB stack runtime, so FFI hosts with small thread stacks (such as the JVM) no longer crash.
- **ocr:** Coloured graphics with a dark mean reach OCR with their raw pixels. (GH#2515)
- **ocr:** Mixed-route OCR lines are matched to native text in the correct frame, so native lines are no longer duplicated. (#2313)
- **redaction:** `extract_with_external_redaction` runs on the 16 MiB stack runtime, so FFI hosts no longer crash.
- **ocr:** Certified table rows of a standalone image stay on one line in Markdown and Djot. (#2432)
- **pdf:** A native page's text is returned once when automatic OCR only re-reads its ink. (#2534)
- **pdf:** Certified native text on an automatically selected OCR page of a mixed document is kept. (#2533)
- **pdf:** A one-page native text layer is kept when the OCR read does not recover its lost glyphs. (#2533)
- **pdf:** A verified embedded prose line can be recovered with automatic Tesseract segmentation (caches off). (#2327)
- **pdf:** Missing words behind partial hidden text on verified CCITT Group 4 scans are recovered. (#2425)
- **pdf:** Verified scanned figure captions are kept as prose instead of a false table (uncached Plain extraction). (#2530)
- **pdf:** The heading and table of an upright scanned page are placed in PDF points. (#2497)
- **pdf:** Verified right-column text from tall indexed-color scans is recovered (uncached Plain extraction). (#2426)
- **ocr:** Explicit sideways PDF scans are recovered with cached Tesseract orientation data, including counterclockwise tables, sideways prose and upside-down tables; result caches are bound to orientation model revisions. (#2496)
- **ocr:** Detached two-table PDF reading order is corrected and table bounds are in PDF points. (#2497)
- **ocr:** Source-proven dot-leader scan rows are returned as one table in Auto OCR. (Refs #2503)
- **ocr:** Duplicate dot-leader rows are removed from standalone Tesseract PSM 6 output. (#2503)
- **pdf:** Painted exhibit, fee and company tables are preserved without dropping adjacent content. (#2376)
- **pdf:** Source-ruled numeric cells stay separate in fallback tables, and fully ruled wide tables with merged index cells are detected. (#2376)
- **pdf:** Certified side-by-side panel lines stay in source order in Plain output. (#2537)
- **pdf:** Rendered pages retained during OCR keep only RGB pixels, so upright scans fit the default memory budget.
- **pdf:** Aligned indexed assignments stay on separate display rows with their subscripts. (#2443)
- **pdf:** Raised chart legend markers stay with their labels. (#2443)
- **pdf:** Figure tick labels beside native tables are preserved. (#2443)
- **pdf:** Parallel form legends are kept out of synthetic heading blocks. (#2543)
- **pdf:** Separate figure labels are kept out of empty inferred table row spans. (#2332)
- **pdf:** Readable native text is kept when automatic OCR cannot see unpainted Type0 glyphs. (#2533)
- **pdf:** Mapped Khmer text is kept when automatic English OCR produces only noise. (#2536)
- **pdf:** A visibly painted title is kept when automatic OCR noise is discarded. (#2312)
- **pdf:** Only one copy of native text is returned when OCR reads the same words. (#2534)
- **ocr:** PNG and TIFF retries stay within the image budget when upscaling, and oversized PNG/TIFF rasters with unknown DPI are recovered within it. (#2530)
- **ocr:** Upscaling a JPEG with unknown DPI reports the actual budget limit. (#2531)
- **ocr:** A warning is emitted when a budget-clamped PDF render returns suspected noise. (#2532)
- **paddle:** Reading order is preserved after automatically correcting rotated PDF pages. (#2519)
- **pdf:** Source words in merged table cells and two drawn header bands are retained, with the header rowspan in HTML and the table grid. (#2332)
- **pdf:** Identity CID TrueType glyphs are painted when Unicode decoding loses characters. (#2309)
- **pdf:** Word boundaries are preserved when Type0 fonts lack Unicode mappings, with a warning when native characters are unavailable. (#2328)
- **pdf:** Independently shaded table headers and ruled cell boundaries are retained without extra columns. (#2332)
- **pdf:** Grouped leaf headers in ruled tables are preserved; extracted stroked paths record their dash pattern. (#2332)
- **ocr:** Dark-mean images are preprocessed only with dense glyph evidence, keeping words beside large graphics. (#2515)
- **pdf:** Confirmed readable native lines that automatic OCR misses are retained. (#2439)
- **pdf:** Caller font bindings are restored after a Form XObject that shadows the font name. (#2516)
- **pdf:** Signed decimal powers stay with their native bases. (#2511)
- **pdf:** Stepped-baseline prose panels keep source order and separate lines. (#2323)
- **markdown:** Rendered headings and scripts survive canonical Unicode normalization. (#2512)
- **pdf:** Mixed unembedded text runs render on hosts without fonts, using bundled DejaVu Sans Bold for bold text and orienting CJK fallback glyphs correctly. (#2510)
- **pdf:** Empty OCR reads after the raster budget reduces page DPI are reported with requested and actual DPI. (#2504)
- **pdf:** Touching glyphs whose coordinates round to a tiny overlap are no longer split into words.
- **pdf:** Whole-document OCR recovery admits stored image inputs before making copies, keeps the preferred OCR stage options, and honors cancellation and the smaller image byte cap. (#2339)
- **ocr:** Oversized JPEG scans decode at a reduced IDCT scale (1/2, 1/4, 1/8) that fits the content size limit; unsupported JPEG kinds keep a size-limit error naming the reason. (#2339)
- **pdf:** Text on tall scans is recovered when an OCR strip contains only blank space. (#2501)
- **pdf:** OCR rows are recovered from tall one-bit grayscale, neutral RGB and grayscale JPEG scans. (#2502)
- **pdf:** Upper rows stay first when stacked ruled grids merge. (#2494)
- **redaction:** Terms split by a superscript in HTML headings or by font changes in PDF text are redacted. (#2363)
- **pdf:** OCR and layout-only page rasters are fitted to the raster budget before allocation. (#2491)
- **ocr:** Warnings from failed PDF rasterization no longer leak into later extraction results.
- **pdf:** The space after a raised footnote number at the start of its definition is preserved. (#2443)
- **ocr:** Complete tables on sideways PDF scans, including two tables per page, are recovered with a cached orientation model. (#2314, #2497)
- **pdf:** Raised numeric suffixes stay with their bases in Markdown when glyph edges overlap. (#2489)
- **ocr:** Cover and map labels are kept in the page text (Tesseract and PaddleOCR). (#2431)
- **docker:** Core and full images no longer store the binary twice.
- **pdf:** Long Form text is preserved when the Form invocation is marked as placed PDF content. (#2310)
- **pdf:** Text missing from partial invisible scan layers is recovered in Plain Auto extraction with built-in Tesseract. (#2425, #2506)
- **pdf:** Aligned financial amounts stay with their labels, and financial body rows stay out of Markdown headers. (#2315)
- **pdf:** Characters mapped by multiline ToUnicode ranges are preserved. (#2476)
- **pdf:** Row-painted table values stay with their labels across wide gutters in plain text. (#2343)
- **tokenizers:** Callback errors are preserved when registering a tokenizer backend. (#2374)
- **ocr:** Letters on images of repeated aligned text are preserved. (#2472)
- **ocr:** Oversized PNG and TIFF inputs are downscaled within the workspace budget. (#2618, #2619)
- **ocr:** Tall scans with default PNG predictor parameters are recovered. (#2620)
- **pdf:** Layout rasters render within the configured memory budget, keeping detections in PDF points. (#2332)
- **pdf:** Painted table-cell overhangs and figure ticks beside tables are matched by exact source geometry. (#2616)
- **ocr:** A merged table header is separated from its first record in standalone OCR. (#2432)
- **pdf:** Parallel bold legend rows stay separate regardless of punctuation, case or digits. (#2615)
- **pdf:** Wrapped text in painted record rows stays with adjacent cells. (#2613)
- **pdf:** Multirow painted double-rule grids are recovered. (#2614)
- **ocr:** A certified duplicate glyph is removed from a recovered photo title. (#2312)
- **pdf:** Hidden CCITT prose split across two content streams is recovered. (#2425)
- **ocr:** Certified tall grayscale Flate strips are recovered down to 120 DPI. (#2502)
- **ocr:** Verified embedded PDF prose is recovered with caching enabled. (#2327)
- **pdf:** A missing embedded prose line is recovered on any page of a multi-page PDF, and an unrecovered picture's OCR text follows its own page. (#2327)
- **pdf:** Full-page grayscale raster prose beside visible native stubs, and outlined prose beside sparse native text, are recovered. (#2313)
- **ocr:** Recovered photo titles in PNG extraction are placed in source order and recovered in Markdown and Plain output, including cache hits. (#2611, #2312)
- **ocr:** Rows stay together in standalone sparse OCR page text. (#2432)
- **pdf:** A missing prose line in an embedded image is recovered with explicit PSM 6. (#2327)
- **ocr:** Complete numeric rows in scanned PDF tables are recovered. (#2432)
- **pdf:** Ruled table rows stay on the correct side of a section divider. (#2376)
- **ocr:** Automatic PaddleOCR is skipped on uniform PDF page rasters. (#2430)
- **pdf:** Complete native table headers are exposed as column labels. (#2332)
- **dart:** The generated SDK compiles.
- **pdf:** Low-confidence OCR on rotated scanned pages is retried with a cached orientation model. (#2389)
- **pdf:** Latin text renders on hosts without a matching font using bundled DejaVu Sans. (#2347)
- **structured:** The caller's raster budget applies to PDF pages. (#2352)
- **pdf:** An undecodable painted image is reported with its page when OCR sees a blank page. (#2427)
- **pdf:** Scanned images with a singleton DeviceGray color-space array, and CCITT Group 4 images with array-wrapped filters, render. (#2427)
- **docx:** Hyperlinks split across styled runs return their complete label.
- **redaction:** Source IDs in HTML heading metadata are redacted. (#2363)
- **redaction:** Annotation authors and decoded QR payloads are redacted.
- **redaction:** Literal tag names in annotations and raw source blocks are redacted while generated Markdown formatting is kept.
- **pdf:** Word and line breaks are preserved when layout reorders upright PDF text. (#2373)
- **redaction:** Repeated terms split by Djot inline styling are redacted; unverifiable output is rejected.
- **windows:** PHP, CLI, C FFI and Elixir releases bundle their Microsoft runtime dependencies. (#2370)
- **redaction:** HTML is rebuilt from redacted plain text so terms split by inline styling are covered; unverifiable HTML is rejected. (#2361)
- **pdf:** Prime marks stay beside their operands. (#2443)
- **pdf:** Same-baseline small caps are kept out of script markup. (#2443)
- **redaction:** Redaction reports when a post-processor added, removed or renumbered pages. (#2447)
- **redaction:** Markdown projection edges are validated and emptied inline wrappers are removed on Drop. (#2446)
- **pdf:** Native superscript suffixes stay with their base text across paragraph grouping. (#2275)
- **redaction:** Repeated inline-styled terms are redacted in Markdown and DOCX/PDF output. (#2361)
- **pdf, pptx:** With `pages.insert_page_markers`, every PDF page and PPTX slide gets its own marker, including empty pages and slides that fail to parse. (GH#2422)
- **ocr:** The orientation detector, PaddleOCR manifest and table models use one pinned model revision, so a synced cache works offline; caches holding older table models download them once more. (GH#2417)
- **pdf:** Configured spans follow the ordinary page-bounds policy; raw character extraction keeps off-page characters. (#2379)
- **pdf:** Body text placed inside Form XObjects is kept in spans and characters. (#2310, #2380)
- **pdf:** Escaped PlacedPDF form tags are recognized, in form and page-level decisions, and cached output no longer depends on earlier page reads. (#2378)
- **pdf:** Nested Form text at the recursion limit no longer depends on earlier page reads. (#2377)
- **pdf:** Generated image placeholders are omitted when image extraction is disabled. (#2386)
- **pdf:** Cached Form spans stay tied to the destination page's clipping context. (#2382)
- **docx:** Ordered and bullet item kinds are preserved at each level of a shared numbering definition.
- **python:** Enum stringification is unchanged when targeting Python 3.11.
- **formula:** Formula recognition models honor `XBERG_CACHE_DIR`. (#2409)
- **markdown:** Decimal list starts, gaps and nested list boundaries from the source are preserved. (#2387)
- **redaction:** Formatted edits across paragraphs are preserved while redacting current text. (#2361)
- **tesseract:** Building the Tesseract crate (native and WASM) no longer emits routine progress as Cargo warnings.
- **redaction:** Raw and decoded findings are checked before returning edited formatted text; incomplete coverage is rejected. (#2358, #2363)
- **redaction:** Source tag names stay redactable in Markdown, and legitimate post-processor edits are preserved, falling back to redacted plain text with a warning when generated Markdown cannot be verified. (#2358)
- **redaction:** Generated Markdown formatting (superscript, underline) is preserved while its content is redacted. (#2358)
- **markdown:** Superscript and underline annotations render as `<sup>` and `<u>`. (#2358)
- **pdf:** Content-stream comments can no longer become operators or marked-content tags. (#2399)
- **keywords:** YAKE results keep source token order and merge possessive variants. (#2388)
- **markdown:** Frontmatter stays in metadata and out of the body, elements and chunks, with typed YAML and TOML values. (#2392)
- **llm:** Provider schema names are sanitized without changing caller prompts or JSON Schema references. (#2385)
- **python:** Extraction errors keep their typed exception classes, including on progress-enabled calls. (#2383)
- **layout:** PP-DocLayoutV3 preprocessing and output validation match the reference model. (#2284)
- **ocr:** The standalone Sceptre ORT feature compiles. (#2364)
- **pdf:** Admitted table rows are kept when a section would be too small to retain. (#2332)
- **python:** Windows wheels bundle the required Microsoft C++ runtime. (GH#2368)
- **ocr:** Word, table, formula and OCR boxes stay aligned after backend auto-rotation; new metadata key `ocr_backend_rotation_undone: true` marks pages whose geometry is already upright. (#2356)
- **bindings:** Java, C# and PHP exceptions expose shared family interfaces; Python and Ruby `IoError` is catchable as both `LoadError` and `XbergError`; Kotlin JNI declares its resource dependencies; Swift links OpenDirectory on macOS. (GH#2366)
- **ocr:** Cancellation is honored after sparse OCR retries complete. (#2345)
- **ocr:** Scaled, rotated PDF OCR geometry is mapped through its processed image frame. (#2332)
- **pdf:** Native table header roles survive grid conversion and HTML rendering. (#2332)
- **pdf:** Prose-flanked multiplication dots stay in their sentence. (#2275)
- **pdf:** Detached pages are retried from embedded JPEG/JP2 bytes after raster OCR fails. (#2339)
- **pdf:** A photo page keeps its readable text layer, followed by the OCR text, when OCR reads only noise. (GH#2311)
- **pdf:** Superscripts and formatting stay on their characters after ligature expansion, space absorption and Unicode normalization. (#2275)
- **tables:** OCR and native TATR table headers, spans and cell geometry are retained in image and PDF output. (#2332)
- **pdf:** Each text run keeps its own baseline when cached form objects reuse span IDs.
- **pdf:** Adjacent values such as `5.55` and `16` no longer fuse during leftward reading-order jumps.
- **tables:** Declared headers are preserved across PDF table fragments. (#2332)
- **pdf:** Recognized formulas missing from OCR body elements are rendered, or kept in metadata with a warning. (#2331)
- **rendering:** HTML honors table header metadata and keeps headerless data rows. (#2332)
- **vtt:** Overflowing timestamps are rejected and character references in speaker annotations are decoded. (#2330, #2329)
- **excel:** XLSX text limits apply before cell materialization, including repeated shared strings, and workbook limits apply to file inputs. (#2321)
- **excel:** Missing-worksheet warnings are kept alongside readable sheet content.
- **djot:** Nested table captions stay inside footnotes without duplication.
- **vtt:** WebVTT header metadata is skipped while the first timed cue is kept. (#2320)
- **packaging:** Docker images and native release archives include the canonical licensing notices.
- **mobile:** Default Dart and Swift builds for iOS devices and simulators work again.
- **doc:** Legacy Word superscript and subscript formatting (inherited, direct, piece-level, in tables, footnotes and comments) is preserved. (#2275)
- **pdf:** Superscript and subscript annotations are preserved in detected tables, including rotated text. (#2275)
- **pptx:** Slide headings and table grids are preserved in plain document structures. (#2302)
- **pdf:** Word spaces are recovered when subset fonts omit a space-glyph width. (#2303)
- **pdf:** Repeated running headers are classified and header/footer filters apply in every renderer. (#2304)
- **pdf:** Up to six inferred heading font tiers are retained. (#2305)
- **ocr:** Keyed Tesseract sessions reuse secondary dictionaries. (#2306)
- **office:** Native text is recovered from embedded EMF and WMF pictures. (#2297)
- **ocr:** Repeated Tesseract orientation and script detection no longer crashes. (#2300)
- **pptx:** Numeric chart values use 15 significant digits. (#2298)
- **docx:** Numbered list items carry the label Word displays (`Q4.`, `(b)`, `3.2`) in `list_marker` and Markdown. (#2293)
- **csv:** Table-cell limits apply to padded dimensions. (#2296)
- **pdf:** Prose superscripts and subscripts are preserved with table detection enabled. (#2275)
- **docx:** Equations in table cells are preserved as math. (#2256)
- **html:** YAML metadata blocks survive Markdown normalization. (#2295)
- **llm:** Configured provider options are forwarded to hosted embedding and reranking requests.
- **pdf:** Two-up pages are read one half at a time. (#2227)
- **pdf:** Native regions are kept unless VLM answers retain their content; table-model load failures are reported. (#2259, #2269)
- **djot:** Smart punctuation, table captions, line breaks and image alt text are preserved. (#2254)
- **djot:** Caption footnote references are dropped when an empty table is omitted. (#2248)
- **docx, odt:** Equation placement, limits, function names and surrounding text are preserved. (#2258)
- **docx:** Blank paragraphs and list items beside display equations are omitted. (#2262)
- **email:** HTML head metadata and scripts stay out of message text, and malformed base64 parts are recovered. (#2252, #2253)
- **plugin:** All seven Pi skills advertised by the root package are available. (#2257, #2246)
- **ocr:** Sceptre OCR and document orientation can initialize in either order.
- **djot:** Blocks in footnote definitions stay separated. (#2270)
- **extraction:** An `image_tables` or `pdf_tables` warning is emitted when OCR table recognition cannot load its model. (#2244)
- **jupyter:** Terminal-style output overwrites handle wide characters and combining marks, and 8-bit CSI sequences are removed. (#2245, #2288)
- **djot:** Line breaks in footnote definitions and image alt text are kept. (#2247)
- **html:** Superscript and subscript markers (`10^9^`, `H~2~O`) are kept in Markdown when HTML options are unset. (#2275)
- **structured:** Rust `pages_for_call_with_security_limits` and `ParsedDocument::from_bytes_with_security_limits` accept caller image rasterization limits. (#2283)
- **pdf, docx:** Superscripts and subscripts are preserved in native PDF prose Markdown and in DOCX table cells and note definitions. (#2275)
- **images:** Cell-only OCR tables are preserved without duplicating neighboring tables. (#2226)
- **doc:** Floating and inline PNG and JPEG pictures in Word 97-2003 documents are extracted and OCR'd in source order. (#2273, #2210)
- **html:** Non-UTF-8 pages are decoded with the encoding declared by their byte order mark or `<meta>`. (#2220)
- **markdown:** Link labels and image alt text stay in place in plain output. (#2233, #2235)
- **mathml:** LaTeX output escapes `^` and `~`, drops invisible operators, converts `mmultiscripts`, and writes big-operator limits as `_{...}^{...}`.
- **pdf:** A missing table structure model (TATR or SLANeXT) no longer drops the document's layout structure; a `pdf_tables` warning names it.
- **ocr:** `ocr.backend = "vlm"` works on mixed PDFs with layout detection on.
- **pdf:** VLM regions are cropped from the correct pixel box.
- **pdf:** A VLM region on a page with no text layer gets its own page break.
- **pdf:** Formulas are recognised on scanned pages that Tesseract OCRs page by page.
- **layout:** Markdown layout detection runs on the page's CropBox.
- **pdf:** Formula and VLM region recognition keep working when the auto layout gate skips a page.
- **toml:** Dates and times print as TOML text instead of under a `$__toml_private_datetime` key.
- **images:** Legacy OCR Markdown tables render once when the backend also returns structured tables.
- **latex:** `\ref` and `\cite` relationships attach to the paragraph that holds them.
- **latex:** Accent commands, letter commands and control spaces print as their characters (`Schrödinger`, `ß`).
- **archive:** A ZIP archive with an encrypted member extracts its other members and names the skipped one in a warning.
- **markup:** Markdown, Djot, MDX, Org, reStructuredText, Typst, RTF, DocTags and Jupyter documents starting with a UTF-8 BOM are read correctly.
- **jupyter:** ANSI escape codes are dropped from `execute_result` and `display_data` plain text.
- **excel:** Float cells are rounded to Excel's 15 significant digits.
- **structured:** JSON, JSON Lines, YAML and GeoJSON documents starting with a UTF-8 BOM are parsed.
- **html:** Whitespace-only formatted links keep the word separator.
- **mime:** Standard LaTeX, shell script and Apple iWork MIME aliases are accepted. (#2205, #2208)
- **mime:** `application/octet-stream` with parameters or different casing still triggers format detection. (#2206, #2207)
- **downloads:** Percent-encoded proxy credentials are decoded correctly for model and tessdata downloads.
- **asciidoc:** Cross references, passthroughs, escapes and hard line breaks read as Asciidoctor renders them.
- **doc:** Legacy Word tables keep their cell and row structure.
- **vtt:** Numeric character references, `&quot;` and `&apos;` are decoded, and ruby text is left out of the line.
- **rst:** Internal underscores in Unicode words stay intact.
- **rendering:** Markdown output ends raw HTML and JSX blocks before the next block.
- **djot:** Paragraphs of a list item are joined with a space.
- **docbook:** An admonition's body is its text.
- **build:** The `xml` feature compiles without `svg` or `pdf`.
- **fictionbook:** Note references no longer split their paragraph, and a note's repeated title stays out of its definition.
- **jats:** Inline markup stays inside its word, and table rows outside a row group are read.
- **rst:** Named hyperlink references and `replace::` substitutions read as docutils renders them.
- **typst:** Comments are left out and escapes read as their characters.
- **jupyter:** Stream and error outputs drop ANSI escape codes and apply carriage-return overwrites.
- **rst:** Line blocks keep their line breaks, and `Text::` reads `Text:`.
- **latex:** Wrapped source lines form one paragraph, and `~` reads as a no-break space.
- **orgmode:** Comment lines and `#+BEGIN_COMMENT` blocks are left out.
- **swift:** Package release builds no longer pass conflicting dynamic-lookup linker flags.
- **rendering:** Plain output includes footnotes for LaTeX, Markdown, reST, Org, Typst, Djot, DocBook, FictionBook, EPUB and PPT; styled HTML renders footnote definitions, including for DOCX and ODT.
- **html:** Inline MathML keeps its paragraph placement, bold table text keeps literal asterisks, and links in preformatted content keep their destinations.
- **odt:** `text:s` spaces and text after nested spans, spaces or tabs are kept.
- **rtf:** `\~` and `\_` read as non-breaking space and hyphen, and escaped braces and backslashes stay in their table cell.
- **doc:** Fields spanning paragraphs keep their displayed text without leaking instructions. (#2184)
- **ppt:** Speaker notes are referenced from their own slides in Markdown. (#2186)
- **ios:** Native libraries link with bundled ONNX Runtime's CoreML backend.
- **html:** Whitespace-only formatted paragraphs and headings no longer panic. (#2182)
- **markdown:** A list item's text comes before the code blocks, tables, headings and formulas inside it.
- **markdown:** Paragraphs inside a list item keep their breaks and annotation offsets. (#2181)
- **code:** Graphviz `.dot` files and Graphviz/PlantUML MIME aliases extract as source code. (#2180)
- **ocr:** Tesseract reads display-rotated (`/Rotate`) PDF pages upright regardless of `auto_rotate`. (#2172)
- **pdf:** With layout detection, item numbers are no longer deleted because their digits occur in other table cells. (#2171)
- **pdf:** `extraction_confidence.text_coverage` counts known empty pages whether or not page output is requested. (#2170)
- **pdf:** Ruled table records whose cells wrap to different line counts are no longer split into one row per line. (#2169)
- **pdf:** Item schedules with rows about two line heights apart stay one table. (#2168)
- **pdf:** `metadata.pages.boundaries` index the final Markdown, HTML and Djot content; unverifiable ranges are omitted with a `page_boundaries` warning. (#2166)
- **pdf:** Repeated prices in rotated tables are no longer dropped as duplicates. (#2164)
- **pdf:** Splitting multiline table rows keeps merged columns aligned. (#2165)
- **pdf:** Native PDF page tables reflect the final table selection, order, IDs and columns. (#2167)
- **markdown:** Text in code spans, code blocks and math can no longer become raw HTML, and an empty code span no longer panics.
- **api:** `/health`, `/info` and `/version` no longer wait behind `XBERG_MAX_CONCURRENT_REQUESTS`. (#2161)
- **pdf:** Disabling extraction caching also disables the built-in Tesseract result cache.
- **ocr:** Images without resolution metadata are upscaled according to text size instead of a fixed 4.2x; cached results are invalidated. (#2127)
- **reranker:** The default `balanced` reranker works on macOS again (runs on CPU under `auto` acceleration). (#2159)
- **ner:** The default GLiNER model no longer reports identifiers without an address cue as `email`. (#2133)
- **ocr:** Multi-language Tesseract runs no longer drop lines in languages other than the first. (#2158)
- **rendering:** A footnote reference stays on the block that cites it. (#2156)
- **docx, odt:** Footnote and endnote references are written once and numbered like their definitions in every output format.
- **docx:** Prompts of empty content controls (`w:showingPlcHdr`) are no longer extracted.
- **pdf:** Explicit header and footer inclusion preserves geometric page numbers. (#2150)
- **pdf:** Separate two-column bordered tables beside wider tables are preserved.
- **ocr:** Failed PDF OCR pages report the type chain of the underlying error. (#2149)
- **ocr:** Tesseract C API declarations match native return types, fixing the WASM link failure and a crash in `process_pages`. (#2145)
- **ocr:** Table reconstruction keeps closely spaced columns. (#2140)
- **images:** TIFF `XResolution`/`YResolution` is used as the source DPI. (#2143)
- **ocr:** Ordered text and explicit HTML raw blocks from custom OCR backends are preserved; unsupported nested payloads fail explicitly.
- **ocr:** The standalone image OCR pipeline keeps the tables it returns. (#2144)
- **ocr:** PaddleOCR, layout detection and GLiNER share one ONNX Runtime environment. (#2148)
- **csharp:** macOS native-package builds avoid duplicate loader paths. (#2147)
- **images:** Images without EXIF metadata no longer emit a metadata warning. (#2141)
- **docx:** Text moved with Track Changes appears once, at its new location. (#2151)
- **llm:** Failed structured output keeps model, token and finish-reason metadata in `llm_usage`. (#2139)
- **pptx:** Speaker notes are matched to slides through relationships. (#2138)
- **csv:** `Table::markdown` escapes `|` and line breaks in CSV and TSV cells.
- **docx:** Rows using `w:gridBefore`/`w:gridAfter` keep cells under the right columns. (#2153)
- **swift:** iOS simulator release builds omit macOS extension-linker flags.
- **docx:** Lists in non-Latin numbering formats stay numbered. (#2154)
- **pptx:** Tables, charts and SmartArt keep their place in reading order. (#2155)
- **llm:** Strict structured extraction validates output against the original JSON schema. (#2136)
- **ocr:** Tesseract honors the requested OCR engine mode; unsupported modes report an error. (#2135)
- **pdf:** Consecutive-paragraph cleanup keeps repeated body text at distinct or unknown positions.
- **docx:** `w:cr` breaks the line and `w:ptab` separates words. (#2152)
- **ocr:** A confident OCR read replaces an invisible text layer that shares almost no words with the page picture. (#2425)
- **markdown:** Display math starting with `<` no longer opens an HTML block. (GH#2793)
- **ocr:** OCR lines holding a numeral are no longer filtered out, and page ranges follow `content` through Unicode normalization. (GH#2851, GH#2923)
- **core:** Entity offsets follow `content` through Unicode normalization. (GH#2924)
- **pdf:** Words placed one by one no longer run together. (GH#2675, GH#2875)
- **pdf:** Forced whole-document OCR with a Tesseract preprocessing block renders at the requested DPI above 150. (Refs #2711)
- **pdf:** Source-backed words and captions are preserved when mixed OCR accepts a Plain render. (Refs #3046, #3145)
- **djot:** Headings and titles such as `1. Introduction` are no longer escaped. (GH#2975)
- **markdown:** Math keeps its TeX when read by Markdown readers that apply escapes, such as GitHub. (GH#2897)
- **pdf:** Ordinary transparency groups and soft masks no longer count toward the pixel-work limit, restoring rendering of dense pages. (GH#2953)
- **pdf:** Corroborated native Type0 text is retained when a render loses glyphs. (Refs #2817)
- **ocr:** Real words beside stray glyphs are kept on pages flagged as recognition noise. (#2942, #2943)

## [1.3.10] - 2026-10-13

### Fixed

- **pdf:** A margin-note column beside a justified main column is read as its own block. (GH#2109)
- **pdf:** Prose running across a table's inner gutter is no longer split between columns. (GH#2110)
- **deps:** YAML parsing moved from `serde_yaml_ng` to `yaml_serde` 0.10.7, removing the unmaintained `unsafe-libyaml` (RUSTSEC-2024-0449). (GH#2108)
- **deps:** `ort` is pinned to `=2.0.0-rc.13`; rc.14 broke downstream builds.
- **swift:** iOS libraries build again.
- **elixir:** NIF release builds no longer fail on BEAM-provided `enif_*` symbols.
- **php:** The Linux extension links ONNX Runtime dynamically and ships `libonnxruntime` beside `xberg.so`, so it loads on Debian 12.
- **zig:** Trait-bridge callbacks returning a string no longer crash (registering a post-processor segfaulted).

## [1.3.9] - 2026-10-12

### Added

- **config:** `strict_parse` (CLI `--strict-parse`, env `XBERG_STRICT_PARSE`). By default, unparseable YAML, JSON, TOML or XML, XML/SVG over `security_limits.max_entity_length` and CSV/TSV over `security_limits.max_table_cells` are extracted as plain text with a `parse-fallback` warning; `true` keeps the error.

### Changed

- Updated dependencies, including `tree-sitter-language-pack` 1.22.1 (25 refreshed grammars), `zip` 9.0.2, `biblib` 0.10, `html-escape` 0.3 and `unhwp` 0.19.

### Fixed

- **extraction:** Helm templates and other unparseable YAML, JSON, TOML or XML files no longer fail extraction. (GH#2100)
- **extraction:** CSV files over `max_table_cells` and XML/SVG files over `max_entity_length` are extracted as plain text with a warning. (GH#2101)
- **release:** The Helm chart is published again; it was missing for 1.3.6 to 1.3.8. (GH#2106)
- **pdf:** `ExtractedImage.bounding_box` is correct for images in a Form XObject with a `/Matrix` (regression since 1.2.7). (GH#2103)
- **pdf:** A narrow margin-note column is read before the main text again. (GH#2102)
- **rendering:** Blocks with many inline annotations render in linear time. (GH#2099)
- **pdf:** Two-column pages whose column baselines differ by up to 1 pt are read in column order; label/value rows still read row by row. (GH#2097)
- **pptx, ocr:** Plain output of a PPTX includes the OCR text of slide pictures, also when `images.inject_placeholders` is `false`. (GH#2086)
- **ocr:** Text shaped like a Markdown image reference no longer duplicates a picture's OCR text. (GH#2093)
- **ppt, ocr:** Slide content in `pages` includes the OCR text of pictures in plain output. (GH#2088)
- **pdf, ocr:** A failed whole-PDF OCR fallback reports pages without native text in `ocr_page_failures`. (GH#2083)

## [1.3.8] - 2026-10-11

### Added

- **pdf, ocr:** `PageConfig.keep_native_content` (off by default, requires `extract_pages = true`) keeps the original text layer of OCR-replaced pages in the new `native_content` page field. (GH#2069)
- **ocr:** `XBERG_OCR_MODEL_CACHE_DIR` sets the default cache for PaddleOCR and Sceptre models; `xberg cache seed-classic-ocr` provisions the verified catalog and writes a manifest for offline builds. (GH#2072)
- **go:** Native setup supports static linking and musl assets, auto-detecting musl Linux hosts.

### Changed

- **ocr:** `images.append_ocr_text` defaults to `true`: Markdown and HTML output show a picture's OCR text after its placeholder. (GH#2068)
- **ocr:** When an image file's OCR read yields text or a table, its returned image has no `images[].ocr_result`; the text is in `content`, `tables` and `ocr_elements` only. (GH#2068)

### Fixed

- **extraction, packaging:** Xberg builds with and depends on zip 9.
- **chunking:** Markdown and text splitting, and bounding-box and node-id attachment, scale linearly on multi-megabyte, unbroken or node-dense inputs; chunks are unchanged. (GH#2085)
- **chunking:** Semantic chunks report the exact byte range of their text.
- **chunking:** A chunk is no longer just a stray closing mark, bullet, heading marker or opening quote.
- **chunking:** YAML and JSON section chunks report their source byte range and pages.
- **structured extraction:** Anthropic JSON Schema requests keep `additionalProperties: false`.
- **pdf:** Native table cells keep thousands and decimal separators attached to their digits, including Arabic separators. (GH#2092)
- **hwpx:** Pictures resolve when resource filenames include extensions; ambiguous matches are skipped with a warning. (GH#2091)
- **rtf:** Pictures with nested property groups and `\binN` payloads are extracted correctly, and `\nonshppict` copies are not returned twice. (GH#2090)
- **jupyter:** Notebook pictures contribute their OCR text once to every output format. (GH#2089)
- **html, epub, fictionbook:** Markdown and Djot output keep embedded-image OCR text. (GH#2087)
- **pdf:** A title across a two-column page's gutter no longer moves the column split.
- **api, pdf:** Extraction timeouts stop the underlying PDF work. (GH#2080)
- **pdf:** Borderless key-value headers keep labels before wrapped values; `document::reorder_ltr_key_value_rows` applies the same repair to pre-extracted spans. (GH#2079)
- **doc:** Chinese and Japanese text in Word 97-2003 documents is no longer garbled. (GH#2073)
- **pdf:** Text recognized in a picture on a page with a text layer is kept in `content` and page content; DOCX page content gains picture text likewise. (GH#2068)
- **ocr:** A picture with both an inline placeholder and an image entry gives its OCR text once. (GH#2068)
- **ocr:** An image file is OCR'd once and its text appears once in `content`. (GH#2068)

## [1.3.7] - 2026-10-09

### Added

- **pdf:** `order_page_spans` applies tagged-structure, article-thread and geometric reading order to pre-extracted spans.
- **ocr:** `ExtractedDocument.ocr_page_failures` lists failed PDF OCR pages (`page`, `error`, `recovered`) in every binding; `metadata.additional["ocr_page_failures"]` is no longer set. (GH#2064)

### Changed

- **batch:** In-memory extraction without a cache backend no longer hashes inputs or serializes results for a discarded cache.

### Fixed

- **node:** Extraction works with `AsyncLocalStorage` and async hooks enabled (stack overflow on Node 22 and later).
- **pdf, ocr:** Arabic and Hebrew extraction preserves logical reading order in tagged PDFs and Tesseract output.
- **cli:** Machine-readable `xberg version` output includes the build identifier.
- **doc:** Legacy Word headers, footers, footnotes, comments and text boxes are preserved and respect the layer filters. (GH#2054)
- **php:** Linux packages run on Debian 12 and other glibc 2.36 systems. (GH#2060)
- **ocr:** Columned prose and punctuation fragments are no longer reported as tables.
- **ocr:** Layout extraction keeps scanned-PDF segmentation and preprocessing settings.
- **typst:** Tables wrapped in figures are preserved.
- **latex:** Multiline section titles remain headings.
- **python, pdf:** Python post-processors returning an `ExtractedDocument` keep rendered headings and chunks; internal PDF font-size annotations no longer reach plain text or keywords. (GH#2050)
- **pdf:** JPEG 2000 palettes conflicting with an explicit `/ColorSpace` are rejected consistently on every architecture.
- **zig:** Generated bindings compile with Zig 0.17.

## [1.3.6] - 2026-10-07

### Changed

- **zig:** The Zig binding requires Zig 0.17.0.
- **pdf:** Lossy JPEG 2000 images decode to the same pixels as OpenJPEG (earlier releases decoded slightly darker); rendered pages and OCR input change accordingly.

### Fixed

- **docx:** A footnote and an endnote with the same number are kept apart.
- **pdf:** Mixed-rotation pages keep word boundaries in upright text. (GH#2049)

## [1.3.5] - 2026-10-06

### Added

- **code:** `.baml` files are extracted as source code.

### Changed

- **config:** URL crawling and HTML conversion add options: full-URL crawl patterns, Chromium executable and trusted flags, custom SSRF CIDR deny-lists that take precedence, and an HTML input byte limit.

### Fixed

- **security:** REST and MCP requests can no longer override crawl SSRF limits, proxy and browser launch settings, or document and WARC output paths.
- **api:** The OpenAPI schema describes `KeywordConfig.yake_params`, `KeywordConfig.rake_params` and `TesseractConfig.preprocessing` as a direct `$ref`; wire payloads are unchanged. (GH#2041)
- **excel:** Date, time and duration cells read as the sheet shows them.
- **docx:** The first footnote and endnote of a Word document are kept.
- **pdf:** Malformed classic xref tables no longer resolve the wrong indirect object. (GH#2044)

## [1.3.4] - 2026-10-04

### Fixed

- **ocr:** Spaced scanned invoice rows stay in one table, and a missing quantity is recovered from the original pixels. (GH#2043)

## [1.3.3] - 2026-10-02

### Added

- **cli:** `extract --output <FILE>` and `batch --output <DIRECTORY>` write converted content to files, refusing to overwrite. (GH#2040)
- **html:** `ConversionOptions.inline_data_media` (`InlineDataMedia` enum) keeps, replaces with alt text, or drops inline `data:` media.
- **onnx:** Rust `set_ort_session_options` (`OrtSessionOptions { memory_pattern, cpu_arena, max_threads }`) bounds ONNX Runtime memory and threads for embedding, reranking, sparse-embedding and late-interaction sessions.
- **wasm:** Arrays of generated class values expose `copyForTransfer()`.

### Fixed

- **markdown:** DOCX and ODT tabs render as spaces instead of `&#9;`. (GH#2039)
- **Breaking (csharp, java):** `EntityCategory`, `OutputFormat` and `PiiCategory` are sealed record hierarchies so `Custom(String)` payloads survive JSON; callers construct and match variant records instead of enum constants.
- **docx:** A table, text box or image after a nested list no longer ends up inside the list. (GH#2035)

## [1.3.2] - 2026-10-01

### Added

- **redaction:** External content-inspection findings drive redaction via `RedactionConfig.findings` / `findings_path` and the `extract_with_external_redaction` and `redact_external` operations; `redaction.min_score` filters scored findings, and Presidio, AWS Comprehend, Azure Language and GCP DLP field names are accepted. (GH#1941)
- **cli:** `--redaction-findings <PATH|->` passes external findings to `extract` and `batch`. (GH#1941)
- **captioning:** `captioning.alt_text` (`preserve` default, `combine`, `replace`) controls how generated captions interact with existing alt text. (GH#1998)
- **rendering:** `output_format = "docx"` returns a Word file (base64 in `content`; CLI `--content-format docx`); requires the `office` feature. (GH#1942)
- **rendering:** `output_format = "pdf"` returns a PDF file (base64 in `content`; CLI `--content-format pdf`); requires the `pdf` feature. (GH#1943)
- **tesseract:** `ResultIterator::extract_all_words_with_line_starts` reports which words begin a text line. (GH#1978)
- **ocr:** Typed `paddle_ocr_settings` (`PaddleOcrConfig`) on `OcrConfig` and `OcrPipelineStage` in every binding; it takes precedence over raw `paddle_ocr_config`. (GH#2010)

### Changed

- **Breaking (Rust and generated bindings):** Configuration constructors gain `CaptioningConfig.alt_text`, `paddle_ocr_settings` and the `RedactionConfig` finding fields; Rust exhaustive literals, Java canonical constructors and positional PHP calls must be updated. (GH#1941, GH#1998, GH#2010)
- **ocr:** Tesseract returns line elements before word elements; element positions and `build_hierarchy` ids shift, and layout image OCR text follows source word order. (GH#1978)

### Deprecated

- **ocr:** Raw `paddle_ocr_config` fields and the `paddle-ocr-types` Cargo feature, in favour of `paddle_ocr_settings`; removal planned for 2.0. (GH#2010)

### Fixed

- **python:** Nested configuration fields accept public `xberg.*Config` dataclasses, dicts and JSON strings. (GH#2015)
- **redaction:** Structured table column labels (`tables[].columns`, `pages[].tables[].columns`) are redacted. (GH#1991)
- **cli:** A `redaction` block in a CLI config is applied; the `redaction` feature is in the default and `binstall` builds. (GH#1941)
- **redaction:** A validation error during redaction fails the extraction instead of returning unredacted text. (GH#1941)
- **ocr:** PaddleOCR's selected-page PDF route keeps embedded-image prose beside recovered tables in Markdown. (GH#2021)
- **plugins:** Cached extractions are invalidated when post-processors or validators change. (GH#2032)
- **ocr:** The `aarch64-apple-darwin` CLI release accelerates Candle OCR backends with Metal. (GH#2033)
- **config:** `ExtractionConfig::discover()` also finds `xberg.yaml`, `xberg.yml` and `xberg.json`. (GH#2017)
- **excel:** XLSX sheets with row-only dimensions and ODS indentation whitespace extract correctly; legacy XLS/XLA stream lengths are checked against the container size and `max_archive_size`. (GH#2002, GH#2003, GH#2005)
- **odt:** Text boxes in page-anchored frames are extracted, and encrypted documents return an encryption error. (GH#2006, GH#2004)
- **ocr:** Automatic PaddleOCR fallback no longer enables table detection unless requested. (GH#2018, GH#2030, GH#2031)
- **ocr:** Scanned tables with amount columns split by digit width, junk cells, rule marks, multi-row header bands or wide header labels are kept and their columns rejoined. (GH#1832, GH#2028, GH#2029, GH#2019, GH#2001, GH#1952, GH#1949)
- **ocr:** Folding the first table column no longer panics or drops its header. (GH#2026)
- **ocr:** A blank-rendering PDF page keeps OCR text recovered from embedded images that also hold a table, without repeating the table. (GH#2008, GH#2014)
- **cache:** Structured extraction results produce cache hits; unreadable legacy entries are replaced. (GH#1990)
- **mime:** Large CSS and TOML files with array-like prefixes are no longer detected as JSON. (GH#1982)
- **ocr:** Label-and-value tables no longer disappear; PaddleOCR reports rejected table candidates in `processing_warnings`. (GH#1970)
- **wasm:** Tesseract OCR returns word elements when `element_config.include_elements` is enabled. (GH#1974)
- **ocr:** A tall word box no longer splits a scanned-table row. (GH#1977)
- **transcription:** A Whisper window that cannot be decoded is skipped with a warning instead of aborting the recording. (GH#1944)
- **pdf:** Numeric footnotes stay separate in `document.nodes` and Markdown.
- **ocr:** Numbered list items in standalone images keep their numbers. (GH#1979)
- **pdf:** Alignment instructions beside split description cells are retained. (GH#1958)
- **pdf:** Table-cell text keeps the same word spacing as the page text. (GH#1948)
- **dbf:** dBASE rows keep their values under the right headers. (GH#1968)
- **pdf:** Grouped table header spans are preserved in structured output. (GH#1959)
- **ocr:** Tesseract returns elements when `include_elements` is set without `min_level`. (GH#1978)
- **heuristics:** OCR confidence and quality-score evidence count each word once. (GH#1978)
- **ocr:** A PaddleOCR-detected table no longer repeats its text in the content. (GH#1973)
- **ocr:** Widely spaced table rows no longer lose trailing line items. (GH#1957)
- **ocr:** A scanned page with an unmapped text layer keeps scan segmentation instead of block mode. (GH#1946)

## [1.3.0] - 2026-09-28

### Added

- **ocr:** During automatic OCR fallback, PDF pages with an unusable text-layer character map use Tesseract block segmentation (PSM 6); effective modes are recorded in `metadata.additional.ocr_page_segmentation_modes`. (GH#1896)
- **python:** `extract` and `extract_batch` accept an `on_progress` callback receiving per-page OCR `ProgressEvent`s. (GH#1887)
- **bindings:** `pdf_page_count` is available again in every language binding. (GH#1869)
- **bindings:** The Python, Node, PHP, Ruby and Elixir packages for Linux and macOS Apple Silicon compile in the `pdfium` backend; `libpdfium` is not bundled and is loaded from `PDFIUM_DYNAMIC_LIB_PATH`. (GH#1863)
- **tesseract:** `ResultIterator::extract_word_symbols` returns the text and box of each symbol in accepted words. (GH#1833)
- **ocr:** Config-aware `OcrBackend::supported_languages_for`, `supports_language_for`, `list_ocr_backend_capabilities_for` and `ocr_backend_supports_language_for` honour `OcrConfig.tessdata_path`. (GH#1857)

### Changed

- **Breaking (wasm):** Assigning a class-typed property no longer invalidates the assigned handle; optional class-typed fields are cleared with `clearX()`, and `opts.x = null` now throws. The generated `.d.ts` requires TypeScript 5.1 or newer. Trait-bridge options fields assigned as properties now take effect. (alef #470)
- **Breaking (ocr):** `TesseractConfig::thresholding_method` is an integer (0 Otsu default, 1 LeptonicaOtsu, 2 Sauvola) instead of a `bool`, and now takes effect; config files with `false`/`true` still parse as 0/1. (GH#1784)
- **pdf:** `metadata.pages` is populated when `ocr_near_empty_fallback` or `ocr_scanned_page_quality_gate` is set without an `ocr` block. (GH#1752)

### Fixed

- **pdf:** A borderless table beside a prose column is no longer reconstructed across the page gutter. (GH#1769)
- **ocr:** Multi-word table headers stay in the column they label. (GH#1934)
- **ocr:** Shaded-row normalization no longer loses dark and mid-grey rows or strikes through rows split by a baseline gap. (GH#1837, GH#1918)
- **excel:** Legacy XLS and XLA files are no longer rejected by ZIP-bomb validation. (GH#1938)
- **doc:** Word 6/95 binary documents are extracted; fast-saved Word 6/95 files return an explicit unsupported error. (GH#1939)
- **ocr:** The automatic Tesseract-to-Paddle fallback is decided per PDF page, including on the selected-page route. (GH#1908)
- **ocr:** Automatic OCR fallback keeps valid native pages when other pages are scans. (GH#1927)
- **ocr:** Scanned-page DPI, preprocessing and segmentation settings survive layout detection, `vlm_fallback` and explicit pipelines; standalone image pipelines use the image's DPI. (GH#1926)
- **ocr:** An inset scanned page with many unmapped Type 3 glyphs is still recognised as a scan. (GH#1921)
- **ocr:** One selected PDF page's OCR failure no longer discards the other pages' OCR, with a page-numbered warning. (GH#1925)
- **ocr:** PDF pages rendered for OCR lower their DPI to stay within the content-size limit. (GH#1924)
- **ocr:** `images.include_page_rasters` no longer duplicates the page's OCR text. (GH#1929)
- **ocr:** Right-aligned amount columns of mixed width keep their values apart. (GH#1909)
- **tables:** Comma-grouped amounts are treated as numeric during table cleanup. (GH#1914)
- **ocr:** Numbered row labels no longer cause the whole table to be discarded. (GH#1928)
- **ocr:** A dark scanned PDF page gets the same default preprocessing as lighter scan pages. (GH#1894)
- **ocr:** The embedded-image OCR retry on a scan page uses the page's segmentation mode and preprocessing. (GH#1907)
- **ocr:** OCR of selected pages retries a blank or failed page on its embedded images. (GH#1912)
- **pdf:** Colour-key `/Mask` on `/Indexed` images is applied. (GH#1899)
- **pdf:** `/Indexed` images render on separation plates, including JPEG 2000 coded ones and spot inks used only through a palette. (GH#1898, GH#1916, GH#1915)
- **pdf:** `counts.pages` reports the PDF's real page count. (GH#1888)
- **pdf:** JPEG 2000 images whose `/Width`/`/Height` disagree with the codestream extract at the codestream size, with a warning. (GH#1900)
- **ocr:** Tables on resampled scanned pages, and backend-reported tables, are placed inside the page at the correct position. (GH#1895, GH#1813)
- **ocr:** `layout.table_model = "disabled"` also applies on the force-OCR PDF route. (GH#1813)
- **ocr:** A page whose OCR word boxes cannot be mapped yields its text as prose with a warning instead of an empty table. (GH#1813)
- **ocr:** `numeric_repair` applies to whole-document OCR and to scanned pages' structured output. (GH#1892, GH#1840)
- **ocr:** The Tesseract language probe honours `tessdata_path` and checks the configured languages. (GH#1891, GH#1857)
- **pdf:** An implausible text page that automatic OCR fallback does not route always reports a warning. (GH#1890)
- **ocr:** Right-aligned amount columns are no longer split by digit count. (GH#1886)
- **ocr:** A reconstructed table no longer removes region lines whose words it dropped. (GH#1884)
- **pdf:** JPEG 2000 fixes: bare CMYK codestreams keep their black plane and use `/OutputIntents`; `/Indexed` and lossy palette images decode; alpha channels, `/SMaskInData` and missing `/ColorSpace` are handled. (GH#1883, GH#1885, GH#1903, GH#1922, GH#1902, GH#1839, GH#1850, GH#1781)
- **pdf:** Expanded `/Indexed` images report 8 bits per component. (GH#1901)
- **pdf:** Transparent image pixels show the page beneath, and stencil image masks no longer paint their whole rectangle on separation plates. (GH#1905, GH#1906)
- **pdf:** Colour-key masks apply to 1, 2 and 4-bit images and images with a `/Decode` array. (GH#1904)
- **pdf:** `/Indexed` palettes over DeviceN are read one byte per colorant. (GH#1913)
- **docker:** Images no longer share OCR cache entries across builds of the same version. (GH#1845)
- **ocr:** `ocr_near_empty_fallback` and `ocr_scanned_page_quality_gate` work without an `ocr` block. (GH#1752)
- **pdf:** `PageContent.hierarchy.blocks[].font_size` reports the measured font size instead of 12. (GH#1844)
- **pdf:** Image handles for an `/ImageMask` without `/ColorSpace` report `DeviceGray`. (GH#1825)
- **pdf:** Page-count and page-render entry points reject a wrong PDF password. (GH#1879)
- **api:** The OpenAPI document describes omitted optional references as a direct `$ref` instead of `oneOf[$ref, null]`. (GH#1841)
- **ocr:** A table region whose OCR text was all filtered out no longer publishes an empty table. (GH#1813)
- **pdf:** Multi-page forced-OCR extraction keeps render warnings. (GH#1847, GH#1851)
- **ocr:** An invalid candle `backend_options` or `paddle_ocr_config` fails the extraction up front. (GH#1829)
- **ocr:** Shaded-row edges read as underscores or stray marks no longer glue values into one cell or split row labels. (GH#1833, GH#1858, GH#1834)
- **ocr:** Phantom or label-fragment columns no longer cause a scanned table to be discarded. (GH#1797)
- **ocr:** The embedded-image OCR retry reports the actual cause and no longer overwrites better text. (GH#1826)
- **ocr:** A two-word column header split across two columns no longer shifts every value. (GH#1832)
- **ocr:** PDF table-structure recognition no longer blocks async runtime worker threads. (GH#1812)
- **ocr:** `numeric_repair` no longer rewrites coordinates in hOCR and TSV output. (GH#1836)
- **ocr:** `normalize_shaded_rows` warns when it cannot take effect. (GH#1837)
- **pdf:** Two-column page fixes: full-width tables, multi-run figure legends, sparse tables beside prose, lines before a short band and short second columns no longer break column order. (GH#1801, GH#1808, GH#1800, GH#1806, GH#1809)
- **ocr:** Scanned PDF pages are OCR'd at their own raster density and sparse-text mode, as the same image would be, also with layout detection on. (GH#1786, GH#1828)
- **pdf:** Unmapped Type 3 text keeps one placeholder per painted glyph and can route the page to OCR. (GH#1782)
- **pdf:** Horizontal-rule tables are detected on pages that also have a gridded table. (GH#1799)
- **pdf:** Headings spanning several shaded table columns are kept. (GH#1803)
- **pdf:** Arrow-bullet (`➢`) lists are emitted as lists. (GH#1790)
- **pdf:** Valid multi-page PDFs no longer log a warning per page-tree branch. (GH#1798)
- **pdf:** Fonts with a `/Differences` encoding dictionary no longer log a spurious error on load. (GH#1795)
- **pdf:** `scanned_confidence` scores native text by page coverage. (GH#1779, GH#1793, GH#1752)
- **pdf:** Numeric footnotes stay separate in CLI and binding output. (GH#1771)
- **ocr:** Opt-in `ImagePreprocessingConfig::normalize_shaded_rows` recovers table rows on shaded backgrounds. (GH#1785)
- **ocr:** Opt-in `OcrConfig::numeric_repair` fixes mis-punctuated numeric OCR tokens. (GH#1789)
- **ocr:** `ocr-wasm`-only builds compile with warnings denied. (GH#1830)
- **pdf:** Text in table cells spanning several rows or columns is no longer dropped. (GH#1802)
- **pdf:** Documents with a truncated cross-reference stream no longer return mostly empty pages. (GH#1774)
- **pdf:** Word gaps from fixed-pitch producers are preserved. (GH#1770)
- **pdf:** Page attributes inherit past dangling references. (GH#1775)
- **pdf:** Render warnings report missing glyph ink only when a glyph was not painted. (GH#1794)
- **pdf:** Type 3 glyph advances follow `/FontMatrix`. (GH#1780)
- **ocr:** The OCR result cache honours `use_cache: false` and keys on the recognition model used. (GH#1787)
- **ocr:** PNGs without a resolution tag are no longer assumed to be 72 dpi and resampled. (GH#1788)
- **ocr:** `force_ocr` renders page batches in parallel within `max_threads`. (GH#1796)
- **pdf:** `RunLengthDecode` and `LZWDecode` stop at the output limit instead of allocating past it. (GH#1764)
