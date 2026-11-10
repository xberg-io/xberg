# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## Archived releases

[Versions 1.0-1.2](CHANGELOG-1.0-1.2.md).

<details>
<summary>Links to archived releases</summary>

### [1.2.9](CHANGELOG-1.0-1.2.md#129---2026-09-24) - 2026-09-24

### [1.2.8](CHANGELOG-1.0-1.2.md#128---2026-09-23) - 2026-09-23

### [1.2.7](CHANGELOG-1.0-1.2.md#127---2026-09-22) - 2026-09-22

### [1.2.6](CHANGELOG-1.0-1.2.md#126---2026-09-20) - 2026-09-20

### [1.2.5](CHANGELOG-1.0-1.2.md#125---2026-09-18) - 2026-09-18

### [1.2.4](CHANGELOG-1.0-1.2.md#124---2026-09-18) - 2026-09-18

### [1.2.3](CHANGELOG-1.0-1.2.md#123---2026-09-16) - 2026-09-16

### [1.2.2](CHANGELOG-1.0-1.2.md#122---2026-09-15) - 2026-09-15

### [1.2.1](CHANGELOG-1.0-1.2.md#121---2026-09-14) - 2026-09-14

### [1.2.0](CHANGELOG-1.0-1.2.md#120---2026-09-13) - 2026-09-13

### [1.1.5](CHANGELOG-1.0-1.2.md#115---2026-09-10) - 2026-09-10

### [1.1.4](CHANGELOG-1.0-1.2.md#114---2026-09-09) - 2026-09-09

### [1.1.3](CHANGELOG-1.0-1.2.md#113---2026-09-08) - 2026-09-08

### [1.1.2](CHANGELOG-1.0-1.2.md#112---2026-09-07) - 2026-09-07

### [1.1.1](CHANGELOG-1.0-1.2.md#111---2026-09-07) - 2026-09-07

### [1.1.0](CHANGELOG-1.0-1.2.md#110---2026-09-06) - 2026-09-06

### [1.0.14](CHANGELOG-1.0-1.2.md#1014---2026-08-04) - 2026-08-04

### [1.0.13](CHANGELOG-1.0-1.2.md#1013---2026-08-04) - 2026-08-04

### [1.0.12](CHANGELOG-1.0-1.2.md#1012---2026-08-03) - 2026-08-03

### [1.0.11](CHANGELOG-1.0-1.2.md#1011---2026-08-03) - 2026-08-03

### [1.0.10](CHANGELOG-1.0-1.2.md#1010---2026-08-02) - 2026-08-02

### [1.0.9](CHANGELOG-1.0-1.2.md#109---2026-08-02) - 2026-08-02

### [1.0.7](CHANGELOG-1.0-1.2.md#107---2026-07-31) - 2026-07-31

### [1.0.6](CHANGELOG-1.0-1.2.md#106---2026-07-31) - 2026-07-31

### [1.0.5](CHANGELOG-1.0-1.2.md#105---2026-07-30) - 2026-07-30

### [1.0.4](CHANGELOG-1.0-1.2.md#104---2026-07-30) - 2026-07-30

### [1.0.3](CHANGELOG-1.0-1.2.md#103---2026-07-29) - 2026-07-29

### [1.0.2](CHANGELOG-1.0-1.2.md#102---2026-07-28) - 2026-07-28

### [1.0.1](CHANGELOG-1.0-1.2.md#101---2026-07-28) - 2026-07-28

### [1.0.0](CHANGELOG-1.0-1.2.md#100---2026-07-27) - 2026-07-27

</details>

## [Unreleased]

### Added

- **(models): Pin experimental candidate classification and captioning ONNX artifacts to owned Hugging Face revisions.** (#2336)
  The crate-private `candidate_models` inventory (feature `onnx-runtime`) lists NLI-MiniLM2, SmolVLM-256M, SmolVLM-500M and Florence-2-base as unmodified, license-preserving copies in `xberg-io/classification-models` and `xberg-io/captioning-models`, each file pinned by immutable commit and SHA-256. Every entry is marked experimental and unqualified: none is a default or a supported preset, and no captioning adapter consumes them yet. The NLI-MiniLM2 cross-encoder declares Apache-2.0, but its base model `nreimers/MiniLMv2-L6-H768-distilled-from-RoBERTa-Large` publishes no license, so the entry's license is recorded as unresolved; its fp32 graph exceeds the local NLI adapter's graph size cap, so only the quantized graphs are selectable.

- **(benchmarks): Freeze experimental NLI accounting plans and validate completed reports.** (#2334)
  The standalone runner calls the existing local classifier with pinned inputs and artifacts; report validation checks accounting consistency and does not qualify a model or prove inference occurred.

- **(layout): Add an experimental `model` selector for layout detection.** (#2284)
  `LayoutDetectionConfig.model` (`--layout-model`) chooses `rt_detr` (default) or `pp_doc_layout_v3`. PP-DocLayout-V3 is ONNX Runtime only (selecting it on a tract or WASM build is an error from `extract` and `extract_batch`, including per-input overrides, not a fallback), uses bounding boxes and classes only, downloads about 131 MB, and its quality relative to RT-DETR is not yet qualified.

- **(ci): Add a macOS PHP archive gate that rejects imports the PIE install does not ship.** (#2547)
  `scripts/ci/verify-php-macos-extension.sh` fails on any non-system import in `xberg.so`, any libheif or codec import, a deployment target above macOS 15.0, and missing or altered license notices; it rejects the published v1.3.10 arm64 archives, which import Homebrew's libheif.
  The release workflow now runs it on every macOS PHP archive.

- **(ocr): Read a scan's embedded JPEG at the resolution its placement proves.** (#2530)
  When the embedded-image retry covers a page-covering image placed by an unsheared quarter-turn matrix with equal horizontal and vertical resolution, the backend receives that resolution instead of estimating it, so a scan the render budget refused is read within `max_content_size`. Unproven placements keep the previous behavior, a read the recognition-noise check rejects keeps the original refusal, and a retry that still recovers nothing reports why each image failed beside the original error.

- **(classification): Add an experimental Rust API for CPU ONNX NLI.** (#2335)
  Prepare pinned model artifacts explicitly; classification preserves model identity, bounded execution and cancellation.

- **(layout): Return source-proven text for selected native PDF layout regions in Rust.** (#2357)
  `Engine::extract_with_layout_region_text` returns region text in page reading order and omits unproved or redacted text.
  An explicitly declared crop box is supported when its resolved geometry exactly matches the media box.
  A native span crossing a detector box withholds that region while exact independent regions remain available.

- **(engine): Override layout-for-Markdown behavior per batch input.** (#2367)
  Explicit options take precedence after file overrides; omitted options inherit the batch configuration.

- **(benchmarks): Export local caption graphs and inspect decoder drift with frozen provenance.** (#2336, #2353)
  Verify the six locked runtime distributions before export, plan freezes and diagnostic runs.
  Preserve checked versions, source notices, native companion metadata and no-cache ONNX artifact hashes.
  Hash-bound first-divergence and sub-block diagnostics retain original prefixes, paired observer controls and failed qualification.

- **(benchmarks): Reproduce olmOCR comparisons with the original offline evaluator.** (#2331)
  Pin all 984 PDFs and 6,090 tests; preserve model activation evidence, failures and complete group results.

- **(benchmarks): Fetch pinned ParseBench and olmOCR inputs as upstream references.**
  The corpus submodule records 1,493 file hashes and source license notices; offline staging preserves raw table oracles.

- **(benchmarks): Reproduce full ParseBench HTML table comparisons with the official evaluator.** (#2332)
  Pin inputs, model artifacts, executable and configuration identities; retain extraction failures and metric exclusions.

- **(structured): Add opt-in partial-response recovery with batch-local validation diagnostics.** (#2318)
  Return usable candidate fields separately from validated output and citation fusion.
- **(structured): Add caller-selected image size limits and option-aware render caching.** (#2319)
  Resize images before PNG encoding while honoring the caller’s security budget.
- **(benchmarks): Score layout detections against hash-bound COCO annotations.**
  With geometry annotations supplied, model comparisons report bounding-box precision, recall, F1 and COCO AP,
  retain annotation sources,
  and reject altered saved results. (#2284)

- **(layout): Select PDF pages with `extract_with_layout_pages`.**
  The additive API uses 1-based page numbers, retains native text on excluded pages and warns about invalid pages. (#2255)

- **(benchmarks): Preserve independent layout annotations when importing evaluation documents.**
  PDF-bound sidecars retain regions and reading order, reject changed inputs, and backfill existing fixtures. (#2284)

- **(pdf): Native Rust consumers can retain script baselines with `extract_page_text_with_script_geometry`.**

- **(api): Set `include_image_data: false` to omit raw image byte arrays from extraction responses.**
  JSON and multipart requests support this option for synchronous and asynchronous extraction; image metadata
  and optional Base64 data remain available, and the default response is unchanged. (#2187)
- **(pdf): `PdfError` accepts errors from Lopdf 0.46 while preserving its Lopdf 0.45 conversion.**
- **(llm): Rust exposes failed structured-response usage through `xberg::llm::usage::usage_from_error`.** (#2139)

### Changed

- **(excel): A workbook over `security_limits.max_table_cells` no longer fails with a security error.** (#2820)
  A workbook whose only sheet, or every sheet, exceeds the limit now returns empty placeholder sheets and a warning per skipped sheet instead of `SecurityError::TooManyCells`. The limit still bounds parsing work and allocation, because a skipped sheet is never materialized, and there is no strict mode to restore the error. CSV, TSV and other table formats are unchanged. A legacy XLS or XLA workbook in which the over-budget sheet carries the same name as another sheet, as the reader decodes the names (including names that differ only by a byte-order mark, NUL characters or code-page decoding), still fails with the security error, because the reader keys sheets by name.
- **(markdown): Markdown output writes math as `$...$` and `$$...$$` instead of code math.** (#2331)
  Display math standing alone is now `$$`, the TeX, `$$` on separate lines (on one line, `$$tex$$`, inside a quote, list item or footnote, or when a line of the TeX would open a Markdown block); other display math is `$$tex$$` and inline math `$tex$`. Readers that know only dollar math (GitHub, Pandoc, Obsidian, MathJax, the olmOCR evaluator) read these, and they did not read the previous `` $`...`$ `` and ```` ```math ```` forms. TeX holding a backtick, or display TeX with a `%` comment that needs those placements, keeps the ```` ```math ```` fence. HTML and Djot output are unchanged.
  The TeX is rewritten to stay inside its delimiters, and some rewrites are lossy: a `$` in it becomes `\$` (which also changes a `$` inside `\text{}`); blank lines are dropped; lines joined onto one line lose their `%` comments; and `<` gains a space before a letter, `/`, `!` or `?` (in display math only where a `>` follows it), so `<img ...>` cannot read as HTML (the space shows inside `\text{}`). In a paragraph or block that holds math, a prose `$` is written `\$`, so a raw-text consumer sees `\$5` for `$5`.

- **Breaking (ocr): `OcrConfig.auto_rotate` is now tri-state, and an explicit `false` no longer allows the orientation retry.** (#2496)
  Previously `auto_rotate: false` still allowed the cached-OSD retry for a plain Tesseract block; now `false` never rotates and never retries, `true` always rotates, and an omitted value (unset) keeps the retry and, when no `osd.traineddata` is cached, recovers sideways and upside-down scans through the ONNX orientation classifier exactly as extraction without an `ocr` block does. The field is `Option<bool>` in Rust and nullable in every binding; `true` and `false` in existing configs parse unchanged. Rust callers constructing `OcrConfig` must write `Some(true)`/`Some(false)`/`None`. Positional constructors moved `auto_rotate` (4th to 14th in Python, PHP and WebAssembly), so pass it by keyword. In the C ABI `xberg_ocr_config_auto_rotate` returns 0 for an unset value, so callers must check `xberg_ocr_config_has_auto_rotate` first.

- **(pdf): Pin the native PDF reduced-JPEG decoder to the version the OCR path measures, and drop the unreachable decoder output cap.** (#2740)
  `xberg-native-pdf` now requires `jpeg-decoder` `=0.3.2`, as `xberg` does. The decoder output cap is gone: the decoder checked it only after allocating its buffers, so it never bounded an allocation, and it always equalled the size already checked after scaling. Rendering is unchanged.

- **(ocr): Re-read a standalone picture that looks like a table as one uniform block and adopt that read.** (#2503, #2432)
  When the sparse-text first pass finds no table but its words form a lattice (at least four rows, three columns separated by wide gaps, emitted row by row), the picture is read again with Tesseract PSM 6, whose dot-leader row recovery keeps each row on one line, and that read replaces the first. It is adopted as a caller-forced `psm = 6` would be, so a token PSM 6 misreads is kept; only a re-read with no table, an empty one, or one with under three quarters of the first read's letters and digits is discarded. On the #2432 set standalone pictures keep 162 of 288 rows whole (108 before; explicit `psm = 6` reaches 167). Every picture the lattice test admits is OCR'd twice, whether or not the re-read is adopted (11 of 24 pictures on that set, adopted on 6); only default Tesseract OCR is affected. An explicit `psm`, vertical languages, any TIFF, a VLM fallback or pipeline, requested document structure, layout detection, a first read that already has a table, and scanned PDF pages keep the first read.

- **(layout): Pin PP-DocLayoutV3 to PaddlePaddle's official ONNX.** (#2284)
  The `pp_doclayout_v3` artifact now comes from `xberg-io/layout-models` revision `a0c2d342` (130,502,049 bytes, sha256 `45bf7175...`), the official PaddlePaddle export re-hosted with attribution. Its outputs keep the adapter's contract (`fetch_name_0` F32[N,7], `fetch_name_1` I32[1], `fetch_name_2` I32[N,200,200]). Any V3 capture set recorded against the previous artifact is bound to a hash the code no longer pins and must be re-recorded.

- **(ocr): Tell the default VLM OCR prompt to transcribe repeated text in order.** (#2132)
  The prompt now forbids merging or skipping a paragraph, sentence or line because it repeats earlier text. On captured `gemini-2.5-flash` replays of three eight-group images, the previous prompt kept all eight groups in 2 of 8 responses and an instruction with the same intent in 4 of 9 (this exact wording was not replayed); the provider can still omit repeats. A custom `vlm_prompt` is unchanged.
- **(pdf): Native PDF text returns the letters of every ligature.** (GH#2322)
  A glyph mapped to a ligature code point (U+FB00 to U+FB06) now yields its letters in every output format, so `office` matches a search. The default output of a native PDF changes: text that held `ﬁ` now holds `fi`. Neither the native `extract_text` nor the default output carries U+FB00 to U+FB06 any more, so a caller that matched the ligature code points must match the letters. The `expand_ligatures` option of the native conversion options has no effect and is deprecated; it will be removed in the next minor release.

- **(pdf): Reduce allocations in native text extraction.** (#2429)
  Reuse ordered spans directly when the text pass does not need character records.

- **(pdf): Reuse each page's words and ruling paths across both ruled-table detectors.** (#2429)
  Strict and bordered detection retain their existing thresholds and precedence.

- **(bindings): Regenerate the bindings with Alef 0.122.2.**
  The Swift `TesseractConfig` DTO ignores unknown keys, while the Rust config keeps rejecting them and still accepts a
  `language` string and the legacy boolean `thresholding_method`. Python option enums subclass `StrEnum`. The Node
  binding's fallible conversions return their results directly, so clippy accepts them.

- **(corpus): Require Python 3.11 for corpus tooling.** (#2444)
  Refresh the pinned tooling and benchmark metadata; document fixtures are unchanged.

- Minimum supported versions raised: Node.js 22, Ruby 3.3, PHP 8.3, Go 1.27, Java 25, Android API 24, Dart 3.13, macOS 15, Rust 1.99.
- **(python): Set the package minimum and native stable ABI target to Python 3.11 (`abi3-py311`).**
- **(macos): macOS 15 is the minimum for every macOS artifact.**
  Wheels, the CLI, the FFI and Node libraries, the Elixir NIF and the Swift package (`.macOS(.v15)`) all target
  macOS 15.0, the oldest release Apple still supports. They previously targeted 11.0, and the Swift package 26.
- **(go): Implement `json.Marshaler` for 50 DTOs.**
  Their 74 required collection fields now encode nil slices/maps as `[]`/`{}`.
  Anonymous embedding can promote these methods and omit a wrapper's extra fields.
  Use a named field or define the wrapper's own `MarshalJSON` to preserve those fields.
- **(bindings): Align generated surfaces and tool pins with Alef 0.115.9.**
  Swift decoders honor field-level serde defaults, and Kotlin Android uses Android Gradle Plugin 9.5.0.

- **(rust): Upgrade Sceptre to 0.10.0 and ONNX Runtime bindings to rc.15.**
  Refresh Crawlberg 1.12.4 and Liter LLM 2.4.4 across workspace and standalone consumer locks.

- **(rust): Require Rust 1.98 and upgrade mail-parser to 1.0.**
  Email extraction uses read-only parser views while retaining attachment limits, nested-message
  payloads and malformed-base64 recovery. The pinned Rust toolchain remains 1.99.1.

- **(rust): Upgrade parking_lot to 0.13 and refresh compatible dependencies.**
  Public registry accessors now return parking_lot 0.13 lock types. ZIP advances to 9.2 and
  HTML conversion, encoding and 7z dependencies receive their latest compatible fixes.

- **(ci): Restrict CI and governance validation to `main`.**
  Skip jobs for other branches and tags, including manual dispatches, and remove pull-request validation triggers.
  Pin benchmark source checkouts to the dispatched main revision.

- **(ci): Run ParseBench and olmOCR harness controls in benchmark script validation.**
  Initialize the pinned corpus metadata without downloading benchmark PDF datasets.
- **(dependencies): Update Liter LLM to 2.4.2 and refresh Rust dependency locks.**
  Elixir native builds now require Brotli decompressor 6.2 or later within the compatible major version.
- **(development): Follow shared toolchain baseline 1.12.0 and agent conventions 1.5.1.**
  Governance checks follow the shared AI-Rulez version policy.
- **(plugin): Use the Xberg symbol for the plugin icon and logo.**
- **(tests): Refresh the published corpus governance and tooling pin.** (#2338)
  Document contents and checksums are unchanged.
- **(ocr): Add regression coverage for caller-selected image budgets in direct and staged OCR.** (#2325)
- **(ci): Run governance, fixture, archive and OCR-cache contract tests, and document manual OCR-cache acceptance.**
- **(layout): Simplify YOLO decoding and remove its function-length lint exception.** (#1567)
- **(development): Consolidate Mise task aliases and align guides with supported commands.**
- **(development): Upgrade binding generation to Alef 0.115.6.**
  Fix Python keyword exports and stubs, Ruby typed payloads, and frozen-file cache refresh.
- **(development): Run the complete benchmark harness with `mise run benchmark:test`.**
  Resource-monitor checks validate recorded timestamps without relying on operating-system sleep precision.
- **(validation): Keep Markdown and request-policy regression checks compatible with upstream metadata.**
  YAML front matter and Crawlberg field declaration order no longer cause unrelated check failures.

- **(Rust): Update Crawlberg configuration types and error matching.**
  Exhaustive Crawlberg configuration literals need the new upstream fields; use `..Default::default()` where appropriate.
  `WafBlocked` matches must account for its added status and body fields. HTML conversion options retain their existing
  exhaustive field set with html-to-markdown 3.19.

- **(elixir): Allow compatible Brotli and allocator updates in native builds.**
  Elixir no longer forces decompressor 6.0.1 when another dependency requires a newer compatible release.
- **(pdf): Native font metrics retain their 16-bit glyph-count API with the updated font parser.**
  `TrueTypeFont::num_glyphs()` caps larger upstream counts at 65,535; width-table work stays bounded to that range.
  Font ascenders and descenders use normalized above/below-baseline signs.
- **(ocr): Statically built Tesseract moves to 5.5.4.**
  `build-tesseract` and `build-tesseract-wasm` builds compile the signed 5.5.4 tag and need CMake 3.22 or newer.
- **(native): Refresh Docker bases and system OCR/image codecs.**
  Docker and CI use Tesseract 5.5.4, Leptonica 1.87.0, libheif 1.23.7, libde265 1.1.3 and libaom 3.15.2.
  Windows builds verified Tesseract source; Linux and macOS provisioning checks selected runtimes and real decoding.

- **(pdf): The OCR structure heuristic's retention gate reads ligature code points through the text repair's ligature table (#2689).**
  The gate carried its own copy of the U+FB00..FB06 mapping; behavior is unchanged.

### Security

- **(deps): Upgrade `rmcp` to 3.9.0 to fix RUSTSEC-2026-0462.**
  Before 3.9.0 the Streamable HTTP clients read response bodies without a size limit; the `mcp` server feature, its lockfiles and the binding lockfiles now resolve the fixed release.

- **(pdf): Share stored JPEG payloads in whole-document OCR retries.**
  Bound the unique retained bytes for plain embedded JPEGs while preserving repeated paints. (#2339)

- **(pdf): Honor OCR image decode limits when retrying embedded scans after pipeline failure.**
  Whole-document retries use the tighter explicit limit and stop when extraction is cancelled. (#2339)

- **(excel): Upgrade Calamine to 0.38.0 for RUSTSEC-2026-0436.**
  Fixes a parser panic and unbounded allocation when reading malformed CFB/OLE2 spreadsheets.

- **(build): Update the zlib build dependency to 1.1.31 for RUSTSEC-2026-0418.**

- **(rendering): Avoid quadratic work when rendering user-controlled HTML table cells.**
  Comrak 0.59 includes the upstream table-cell rendering fix.

- **(excel): Enforce spreadsheet limits before worksheet buffers and table grids are allocated.**
  XLSX, XLSB, XLS and ODS reject oversized ranges, repeated cells and malformed records early,
  including renamed inputs; readable cached values survive unsupported XLSB formulas. (#2296)

- **(crawl): Keep custom headers scoped to the seed host while following subdomains.**
  Crawlberg 1.12.3 sends a custom header to subdomains only through its explicit public-header allowlist.
  Untrusted API requests cannot override the operator's TLS certificate-verification policy.

- **(integrations): File locking uses filelock 4.2.4 on Python 3.11 and later to protect lock-file ownership.**
  Python 3.10 retains the affected 4.1.0 branch; benchmark environments with upstream caps remain affected.
- **(integrations): LlamaIndex and txtai use setuptools 84.2.1 to reject invalid legacy-wheel namespace paths.**
  Resolves GHSA-xmj3-9gf7-h52w without changing supported Python versions.
- **(benchmarks): Bound pypdf parsing work for malformed PDF references and cross-reference tables.**
  MinerU and Unstructured benchmark environments use pypdf 6.21.2 to resolve CVE-2026-115572, CVE-2026-115580
  and CVE-2026-115585.
- **(benchmarks): Update MinerU's Python Brotli decoder to fix an out-of-bounds write.**
  Resolves CVE-2026-111497 (PYSEC-2026-4322) in the optional benchmark environment.
- **(crypto): Restore constant-time masks and 32-bit `u128` comparisons with ctutils 0.4.5.**
  Resolves RUSTSEC-2026-0385.
- **(build): Root workspace installs reject registry metadata naming a different package.**
  pnpm 12.15.0 keeps install-script permissions tied to the requested dependency.
- **(pdf): Bound font lookup preprocessing and reject malformed lookup types.**
  Extended glyph IDs stay intact through rendering and diagnostics, preventing unrelated glyphs from being painted.
  The public 16-bit glyph-count API remains compatible.
- **(integrations): CrewAI and LlamaIndex use Pillow 13.0.0 on Python 3.11 and later for image security fixes.**
  Python 3.10 retains the affected Pillow 12.3.0; other branches with upstream caps also remain affected.
- **(integrations): Development environments use virtualenv 21.14.13 to prevent launcher and Bash prompt expansion.**
  Resolves CVE-2026-113722 and CVE-2026-115586; recreate existing environments to replace generated scripts.
- **(runtime): Async timers avoid a data race and use-after-free by updating Tokio to 1.54.1.**
  Replaces the yanked 1.54.0 release across all Rust dependency graphs.
- **(benchmarks): Compatible Python benchmark groups use Pillow 13.0.0 for its image-decoder security fixes.**
  MinerU's Gradio dependency retains Pillow 12.3.0 under its published upper bound.
- **(llama-index): The standalone node-parser lock uses NLTK 3.10.4 with guarded model-file access.**
- **(tls): Reject distrusted emSign certificate intermediates by updating rustls-webpki to 0.103.16.**
  Resolves RUSTSEC-2026-0382.
  Bundled WebPKI trust stores also omit the emSign C1 and C3 root certificates.
- **(excel): ZIP-backed spreadsheets reject mismatched local headers, duplicate member names, and overlapping entries.**
  Archive count, size, and compression-ratio limits are checked before header validation or payload reads.

### Fixed

- **(ocr): Drop the symbol-noise lines of a page flagged as OCR recognition noise.** (#2312)
  This applies to any page the recognition-noise verdict flags and `discard_suspected_ocr_noise` retains, not only photographs. The new `OcrQualityThresholds.prune_symbol_noise_lines` (default `true`; `false` keeps every line) removes lines in which fewer than half the tokens are word-shaped, only when the page has at least 8 non-blank lines and at least half of them are such lines. A line is kept if it has any digit, a CJK, Kana, Hangul, Thai, Lao, Khmer or Myanmar character, a non-Latin letter, an `=`/`+`/`<`/`>` between two operands (`x = y`, not `oe = ie` or `i - l`), or is an English phrase (`I am.`, `IS IT`, not `ee so`) or contraction (`I'm`); a single-token answer such as `No.`, `Yes` or `Qty`, a non-English short line such as `de la gare`, and `x=y` without spaces are pruned on a flagged page. Pruning runs after the verdict, backend choice and table acceptance, skips pages with tables, adds a warning naming the page and the number of lines removed, and covers standalone images and PDF OCR pages, including a photograph after native text; multi-stage pipeline scoring and the single-page first-pass gate read the text before pruning, and an annotation that crosses a line is dropped with its lines. Title recovery now also runs when `quality_thresholds` differs from the default only in this flag. A fixture photograph keeps 0 of 840 noise characters as an image and 169 of 1056 as a PDF page, where the lines with a digit stay.
- **(ocr): Retry an eligible RGB JPEG scan after a minimal-build render-budget refusal.** (#2849)
  A PDF/OCR build without Paddle can now recover an automatic single-page English Tesseract read from a certified whole-page RGB JPEG, using the existing bounded per-page coordinator. Unsupported composition or configuration keeps the typed refusal; budgets, noise-discard policy and clamp/resolution safeguards are unchanged. Mixed-route retries of one image also retain the producing Tesseract segmentation mode.
- **(pdf): Keep adaptive Primary word-boundary decisions consistent when a text matrix moves back into a run.** (#2845)
  A leading TJ offset can change whether the preceding text looks justified. Run widths now use the same decision as offset processing, so a kern keeps the pen-move width and a word gap restores the drawn width. Glyph advances and the default word-boundary mode are unchanged.

- **(ocr): Keep foreground text when a blank embedded background was clamped.** (#2842)
  During the JPEG-budget retry, an empty or whitespace-only read of a separate proven-resolution image no longer marks the page's other text as illegible. The clamp and resolution safeguards still apply to images that return text.
- **(ocr): Keep page byte ranges when OCR page text and plain content render a table differently.** (#2825)
  Under `OutputFormat::Plain` the per-page OCR text kept tables as markdown and its own block separators, so it never matched the plain content and `page_boundaries` was dropped with a warning. A page whose text cannot be found in the content is now rendered from its own elements with the plain renderer, so page text equals its content range. When the re-render succeeds, `pages[].content` under Plain OCR no longer carries markdown tables; the structured tables stay in `pages[].tables`.
- **(pdf): Honor compound glyph names at printable font codes.** (GH#2834)
  A `/Differences` mapping such as `65 /f_f_i` now extracts `ffi` instead of the base encoding's `A`. Explicit `ToUnicode` mappings keep precedence.
- **(ocr): Keep the text a large scanned page's embedded-image retry reads when the read trips the noise check.** (#2832)
  A one-page JPEG scan whose render exceeded the image working budget was read from its embedded image at the proven resolution, and the read was dropped whenever the fragmented-word test judged it noise, so the page came back blank. The read now follows `discard_suspected_ocr_noise` like any page read, kept with the suspected-noise warning unless the caller asked to discard it. A noisy read is still dropped when the image was clamped to `max_image_dimension`, was shrunk to below 100 dpi, or has no preprocessing record. The recovery warning says text when text was kept.
- **(ocr): Read a sideways paragraph of short words as prose.** (#2670)
  The text-size check counted every word box narrower than tall as a stroke, so a paragraph of two-letter words such as `is of to in on` was refused. Only a single-character box, or a run of vertical-stroke glyphs such as `Il`, now counts as a stroke.
- **(excel): Extract the sheets that fit when a workbook exceeds `security_limits.max_table_cells`.** (#2820)
  A workbook whose sheets together held more cells than the limit failed as a whole and returned no content. XLSX, XLSM, XLTM, XLAM, XLSB, XLS, XLA and ODS workbooks now skip only the sheet that does not fit, keep every other sheet, and record a warning naming the sheet and the limit. The limit and its default are unchanged.
- **(pdf): Read an English page whose `ToUnicode` map sends its letters to another script, instead of keeping that layer.** (#2830)
  A page with a lost glyph whose layer is in a script no OCR language reads, and which no cluster structure vouches for, was kept native on the layer's say-so. Such a page is now read once in the configured languages first, and goes to OCR when the read is confident prose that spells none of the layer's script. This adds one OCR pass for these pages: up to four pages spread across the candidates (first, last, evenly spaced) are read, and the rest of the first 256 candidates only if one of them is contradicted (a spread page beyond the 256 is read in addition); a page that is then sent to OCR is read again by the ordinary route. The probe acts only with a calibrated backend (Tesseract), whose read has a mean confidence of at least 75/100; an uncalibrated backend (Paddle, a VLM) leaves the layer native with a warning that the page could not be checked. Scan pages and documents over the OCR preview size limit are not probed. A page really in that script, whose read is noise, stays native and keeps the "OCR was skipped" warning. A probe that fails leaves the layer native with a warning.
- **(pdf): Read an underscore-joined `/Differences` glyph name as all its letters.** (#2689)
  With no `ToUnicode` map, a glyph named `f_f_i` extracted as `f`, so a word with an `ffi` glyph lost letters. The name now decodes by the Adobe Glyph List algorithm: everything from the first `.` is dropped, each `_` component maps to its letters (`f_f_i` is `ffi`, `f_l` is `fl`, `s_t` is `st`, `uni0066_uni0069` is `fi`), and empty or unknown components add nothing. A name in which no component maps keeps the letter its base encoding gives the code.
- **(pdf): Keep a word whole when a producer sets the text matrix back over it before a large TJ offset.** (GH#2322)
  Some producers set the text matrix to the start of a run they already drew and reach the next glyph with a large leading offset in a TJ array. That offset no longer adds a space where the matrix points, so words such as `benefits` no longer come out as `benefi ts`. When the offset reaches past the end of that run by a word gap, the space goes at the end of the run. Otherwise the gap between the real letters on each side decides the space. A lead that follows such a matrix also ends the run at its glyphs when it is a lead at least as large as the word-boundary gap (GH#2690). Not covered: a space glyph that lands inside a ligature after the matrix is set back still splits the word (`defi ne`), tracked in GH#2322.
- **(pdf): Keep the width of a drawn run when the text matrix moves back into it in Primary word-boundary mode.** (#2690)
  A text matrix set back into an open run, followed by a large leading TJ offset, left the run as wide as the pen move, narrower than its glyphs. When `word_boundary_mode` is `primary` (used by several presets), the run now ends at its drawn width, as in the default mode.
- **(pdf): Keep every glyph of a text run that the renderer draws without font info.** (#2709)
  The renderer read the run's shaped advances, which are in font units, as thousandths of an em, so the glyphs after the first landed far off the page and a raster for OCR lost them. It now converts the advances by the font's units per em.
- **(pdf): Draw every digit of a page number that a form XObject paints.** (#2685)
  A form whose resource dictionary is a separate PDF object never had its fonts or colour spaces loaded for rendering, so the renderer spaced its glyphs by raw font units and drew only the first one. The renderer now loads that dictionary first, and logs a warning with the object number if it cannot read it.
- **(pdf): Read a block at the right of a page before a block at the left that starts below it.** (#2680, #2803)
  The reading order cut a region into two columns at a gap or density valley between its left and right spans, even when the right side sat wholly above the left side, so a letterhead at the right of a page top came after the address that started below it. The cut is now refused when the right side sits wholly above the left side, and the two blocks read top-down. A left block above a right block keeps its column cut: the two orders agree there. The two-column-prose cut follows the same rule when no row gap separates the blocks.
- **(pdf): Stop OCR from splitting words at a ligature such as "fi nancial".** (GH#2322)
  A page that OCR reads is first drawn by the native renderer. A glyph code in an embedded TrueType font
  can decode to several letters, as a ligature does, and the renderer gave each later glyph the width of a
  later glyph. That opened a gap after the ligature, and OCR read it as a space. The renderer now draws each
  run (one show string) that holds such a code glyph by glyph from the font, each at its own width.
  Markdown, HTML and Djot output of OCR pages change too.
- **(pdf): Recover a page of outlined prose with eight or more drawn rows beside a native header and footer.** (GH#2313)
  The outlined-prose check refused a page that painted more than about seven rows of outlined text.
  Such a page kept only its native header and footer. The check now accepts a full page of outlined rows.
  The recovered words still pass through the merge that keeps every native word.
- **(pdf): Recover a page of outlined prose that also paints a picture.** (GH#2313)
  The outlined-prose check refused any page that drew an image or a form object.
  A picture holds no outlined text, so the check now reads past an image. It still refuses a page with a form object.
- **(pdf): Return the letters of a ligature glyph that the ToUnicode map leaves out.** (GH#2664)
  Some embedded TrueType subsets draw "ti", "tt" or "st" as one glyph that has no ToUnicode entry and no glyph name, so its letters were dropped and an invented word such as "frotiny" came out as "fro ny".
  The letters now come from the font's own standard and discretionary GSUB ligatures: the component glyphs of the ligature, each mapped through ToUnicode.
  A ToUnicode entry for the ligature still wins, and a ligature whose rules disagree or whose components have no ToUnicode entry keeps the old output.
  The shaping forms of complex scripts, such as Khmer below-base forms, are not read, because their components are not in reading order.
  Ligature lookups behind an Extension lookup are covered by a test.
  A ligature drawn by its own show operator under a text matrix that scales the glyphs no longer leaves a space after it ("defi ne" for "define"): the run that holds it is now as wide as its glyph.
  A word drawn in two shows under text matrices that differ only in their horizontal scale, the second show starting at or just before the pen of the first, no longer gets a space inside it ("Char lie" for "Charlie"): the run continues under the second matrix, in its unit.
  A line written as several TJ arrays, each placed by a Td with no line move back to the pen of the previous array, no longer gets a space at the seam between two arrays ("bro wn" for "brown") when the font gives its glyph widths.
- **(pdf): Keep arXiv citations and the words before them in Markdown output.** (#2650)
  One rule now decides what the arXiv margin stamp is: the identifier with its version, the category and the date, at the end of a paragraph. The PDF structure pass and the Markdown renderer both use it.
  A reference such as `[11] J. Doe, A short title, arXiv:2107.09275 [hep-ph].`, a versioned citation without a date, and a sentence that ends at a bare identifier are kept whole.
  A stamp drawn in parts that the extractor reads as separate paragraphs (identifier, category, date) is removed when the parts on the page make one whole stamp, so its date no longer shows as a heading. The parts must read identifier, category, date (or the reverse), share one rotation (modulo a full turn), sit on one line in their reading direction, and lie in a margin: within a tenth of the page width (text turned up or down) or height (text read across) from a page edge, measured by the thickness of the text, not the length of its run. A page that reads category, identifier, date keeps its parts. A stamp turned exactly 45 or 225 degrees is measured on the side of its baseline that its reading axis implies, like every other rotation. A page with two split stamps keeps all six parts, because together they do not make one stamp.
  A stamp that ends a paragraph is removed alone; the words before it stay.

- **(ocr): Do not crash when the process exits while a thread is still recognizing.**
  The Tesseract exit cleanup freed every engine, including one an OCR thread still used, so a document that hit the extraction timeout could abort or segfault and lose all its text.
  The cleanup now frees only engines that are parked in the idle pool and skips an engine in use. Each engine has its own id, so a new engine at the address of a freed one is never mistaken for the old engine. A direct user of the `xberg-tesseract` crate who never calls `mark_idle` gets tesseract's leak report on stderr at exit instead of a crash.
- **(pdf): Join a word that a ligature glyph split in two, and return the letters of every ligature.** (GH#2322)
  A space drawn inside a ligature's box (the pen is pulled back by the space width) no longer counts as a word break: the gap between the real letters decides. A space with its own room stays, because the page shows a gap there: a space with a full advance after a ligature stays in native text, so `fi eld` stays split. A ToUnicode value of U+0000 for a ligature now falls back to the `/Differences` glyph name of the font. A glyph mapped to a ligature code point (U+FB00 to U+FB06) now yields its letters, so `office` matches a search. The default output of a native PDF changes: text that held `ﬁ` now holds `fi`. The `expand_ligatures` option of the native conversion options has no effect and is deprecated; it will be removed in the next minor release.
- **(ocr): Budget the CCITT preview of an invisible-layer probe per page.** (#2694)
  Each audited page may spend the per-page work derived from `max_iterations`, and all pages together at most six such budgets (960 million work units at the default, roughly 1-5 s of decode), so a multi-page scan with a junk layer on every page recovers every page while a hostile file still gets bounded decode work. Before, one budget shared by the document let a 300 dpi scan recover only its first page or two.
- **(ocr): Keep native text when an automatic all-pages OCR read duplicates it.** (#2768)
  Automatic plain-text Tesseract OCR now applies the same native-text acceptance when every page needs OCR as when only some pages do. Explicit renderers, pipelines, VLM fallback, custom backends, forced OCR, rich projections, fabricated native mappings and native Khmer-number candidates keep their existing routes. Mixed reads attach accepted OCR tables before building page results, including explicitly forced pages and detected scanned pages, while retaining tables from pages whose native text was kept. All mixed routes, including non-Plain output and partial-page OCR, retain accepted fallback spatial OCR elements and matching coordinate frames when no prebuilt sidecar exists. Accepted pages also retain effective segmentation metadata when plain output skips structural merging.
- **(ocr): Tolerate singleton Flate filter arrays in bounded PDF stream probes and Indexed scan recovery.** (#2798)
  `[/FlateDecode]` uses the same decoding as `/FlateDecode` in shared hidden-layer, source-audit, embedded-gap, visible-raster content and tall-scan content/palette probes, including hidden-layer images with validated predictor dictionaries. Certified Indexed images accept either spelling; the separate gray/JPEG and ICC image certificates retain their scalar-only filters. Tests use synthetic filter-spelling and predictor variants; no unresolved real page from #2502 is fixed by this change.
- **(pdf): Keep spot-colour plate ink under separable blend modes.** (#2814)
  A spot-colour plate lost ink where a `/BM` state painted over an area with no ink, while the page render shows it. The plate now blends the complement of each tint, as ISO 32000-1 11.3.5 requires, and mixes the blend with the source by the backdrop alpha, so a paint of tint 0 counts as a backdrop and untouched paper keeps the source tint. Overlap values change for Multiply, Screen, Darken, Lighten, ColorDodge, ColorBurn and SoftLight.
- **(pdf): Keep a Type0 page's native text after OCR ink loss only when the native layer owns or is covered by the read, or spells at least 25% of the read's distinct words.** (#2802)
  A render that dropped a Type0 glyph kept the page's native layer on its own, even when a white fill covered that layer and a short visible line read differently, on both single-page and multi-page documents. Retention now also needs the native text to own the read ink, its words to be covered by the read, or the native text to spell at least 25% of the read's distinct words of at least `min_meaningful_word_len` characters (4 by default), with at least `min_meaningful_words` shared words (3 by default, never fewer than two) and one of them alphabetic (unspaced scripts such as Han, kana, Hangul, Thai and Khmer are compared by character bigrams, with at least 6 shared), so a short or contradicting read no longer loses to a covered layer.
- **(pdf): Read sidecar transparency through the shared ExtGState parser.** (#2735)
  The sidecar transparency check now follows the shared graphics-state parser, so it agrees with the page and separation renderers on what each key means. A `/BM` array with an indirect element now blends on separation plates, as it already did on the page. A malformed soft mask (no `/G`, a `/G` that is not a reference, or an unresolved reference) no longer forces the composite path. A `/BM` name that is not a standard blend mode, including `/Compatible`, now reads as Normal in the shared parser, so it counts as Normal for both the check and the renderer, as an array of such names already did.
- **(pdf): Preserve ink-certified native table pages when their OCR structure omits words.** (Refs #2534)
- **(pdf): Keep a page's native text when the OCR read of a script it is written in contains none of that script.** (#2794)
  When a Tesseract pipeline's first stage reads a native layer's script (such as `khm` for Khmer) but fails or scores below `pipeline_min_quality`, a later stage for another script (such as `eng`) no longer replaces the native text with its own. The page keeps its native text and an `ocr` warning names the script. A healthy read in the script still replaces the page, and pages the unreadable-script rule did not mark are unchanged. A whole-document read that drops the script on every page of the document is discarded and the native text is returned, with no second OCR pass; a document where only some pages drop it is read again page by page (a second OCR pass), so the marked pages keep their native text (#2810), and a page whose Latin letters outnumber its Khmer keeps the OCR read. When every page drops it, the discarded read's page failures (marked not recovered) and warnings are still reported, while its layout and LLM usage are dropped.
- **(pdf): Keep a marked page's native text when a whole-document OCR read drops its script but other pages are scans.** (#2810)
  When every page failed the quality check and only some were marked, the whole-document read replaced a native Khmer page with Latin output. The pages now go through the per-page route, which keeps the marked page's native text with the `ocr` warning and keeps the OCR text of the scans; if that run errors or cannot read a page that the whole-document read did read, the whole-document read is used and a warning names the pages whose native text it replaced. This path OCRs the document a second time.
- **(ocr): Print each row of a dot-leader table picture once when the block read keeps a line the table already holds.** (#2783)
  The PSM 6 read's hOCR grouped some rows into one multi-line paragraph, and a line is dropped as claimed only on an exact token match with a table row. A leader token or a stray `=` the table cell lacks kept the line, so the row appeared as text and in the table. Standalone dot-leader reads now compare a line with a table row without its leader tokens (dots, `=`, `_`); every other route still matches exactly.
- **(pdf): Keep a complete Khmer text layer when the first OCR stage reads Khmer and a later stage does not.** (#2799)
  A pipeline such as `[khm, eng]` sent a native Khmer page to OCR although the layer mapped every glyph, and the shorter OCR text replaced the complete layer. A page in a script with cluster structure (Khmer, Thai, Lao, Myanmar and the Indic scripts) now keeps its native text when a later stage cannot read the script, with or without lost-glyph markers. The kept-page warning names that stage. The page goes to the stage that reads the script when its signs no longer attach to letters, or when lost-glyph markers are more than half of its glyphs. Pages in Han, Cyrillic, Arabic and other scripts without cluster structure go to OCR as before.
- **(pdf): Keep a word joined after a ligature that the font decodes to letters.** (GH#2718)
  When a PDF draws a ligature glyph as its own run and the font decodes it to letters, as in "di", "ff", "erent", the gap before the next run counted as a word break and the output read "diff erent". The decoder now marks a run that is one glyph decoded to the letters of a Latin ligature (ff, fi, fl, ffi, ffl or st), or a run whose ActualText is those letters, and both gap checks read that mark instead of matching the letters. A glyph that decodes to a short word such as "to" is not marked, and a run of the two ordinary letters "ff" between two words now keeps its spaces. An ActualText replacement drawn with a TJ array now spans the glyphs it replaces, across every array in its scope and including the offsets between its strings, so the gap after it no longer reads as a word break. The mark is the new `ligature_glyph` field of `TextSpan` (runtime metadata, not serialized); code that builds a `TextSpan` by its fields names it.
- **(ocr): Re-read a standalone dot-leader table picture whose first read holds a garbled table.** (#2503)
  At 14 px the sparse read turns each leader into low-confidence lowercase noise inside a table whose label cells are corrupted, so the picture was never re-read. A first-read table whose column rows (four or more) hold such noise is now read again as one block, and the re-read replaces it only if it has a table, at least four column rows carrying a leader, and keeps the first read's confidently read letters and digits. The 14 px picture keeps 11 of 12 rows by default instead of 3; correct first-read tables, table-of-contents pictures, scans and an explicit caller PSM are unchanged.
- **(models): Make the candidate inventory's pins reachable by tests.** (#2771)
  A test now stages stand-in files under the NLI-MiniLM2 pins in a temp cache and loads them through the real local classification adapter, so a corrupted byte, a skipped checksum comparison or a descriptor that selects the fp32 graph fails. The inventory verifies files with the shared download checksum check, and the unread per-file size manifest is removed so the SHA-256 is the only pin.

- **(ocr): Adopt the block re-read of a standalone dot-leader table picture.** (#2503)
  A re-read that finds no table is now also adopted when its word boxes show the column structure and at least four rows carry a dot leader, because the table check drops a leader table as prose while the block read keeps every row on one line. An 11 px plain leader picture keeps 11 of 12 rows by default instead of none; pictures without leaders, scans, part-page pictures and an explicit caller PSM are unchanged.
- **(ocr): Keep each row of an adopted dot-leader table picture on its own line in Markdown.** (#2503)
  The adopted block re-read has no table, so its rows were one paragraph whose line breaks Markdown renders as spaces, welding the table into a single line. The extractor now marks that paragraph and Markdown renders each row as its own paragraph; Plain output and re-reads that find a table are unchanged.
- **(djot): Keep each row of an adopted dot-leader table picture on its own line.** (#2804)
  Since #2773 the extractor marks the paragraph of an adopted block re-read so Markdown and the comrak-based HTML render each row as its own paragraph, but Djot never read the mark and joined every row into one line. Djot now renders each marked row as its own paragraph, escaping a leading block marker, and so does the styled HTML renderer, both through the same helper; Plain output is unchanged.
- **(djot): Escape every row of a dot-leader table picture that starts with punctuation.** (#2838)
  A row that began with `:::`, a code fence, `[^1]:`, `[ref]:`, `^ ` or a lone `{.a}` still changed the Djot structure after #2804: it became a div, a code block, a footnote or a link definition, or the row vanished. Djot now escapes the first character of every row that starts with ASCII punctuation, so each row stays one paragraph. After `{`, `[`, `!` or `:` it also escapes the punctuation that follows, so a row such as `{_!_` or `[]{}` keeps its text. A row that starts with a backslash before punctuation is not escaped again. A `#` that is not a heading marker (`#hashtag`) and numbers such as `1.5` stay plain; a row such as `--- Rule` or `-5` is now escaped too, so `--- Rule` reads as a hyphen and an en dash instead of an em dash. Markdown, HTML and plain output are unchanged.
- **(ocr): Keep the native page when OCR differs from it only by hanging underscores or non-text-block bookkeeping (Refs #2534).**
  An underscore run that Tesseract reads just below a native line (the glyph hangs past the line box) now belongs to that line and must show glyph ink, and the count of words Tesseract placed in non-text blocks is run bookkeeping, as `word_count` is. Intel SDM pages 2383, 2486, 2645, 4420 and 4422 return their native text once instead of followed by an OCR copy.
- **(bindings): Restore the pre-#2702 positional order of the `LayoutDetectionConfig` constructors.** (#2764, #2717)
  The released eight-field order is back in the Python, PHP and WASM constructors, with `model` appended last and optional: `strategy`, `applyHeuristics`, `tableModel`, `tableOverlapPreference`, `enableChartUnderstanding`, `confidenceThreshold`, `formulaModel`, `acceleration`, then `model`. Python takes `model` keyword-only (`model=`); PHP and WASM take it as the trailing positional argument. Java and Swift gain a legacy eight-argument constructor and `init` that default to `RT_DETR` / `.rtDetr`. A call written for the last release no longer raises a type error or, in WASM with `1.0`, silently selects `pp_doc_layout_v3`. The default model is unchanged (`rt_detr`).

- **(pdf): Keep a lone accent letter apart from the formula line under it.** (GH#2719)
  A math font can draw a hat as its own lowercase letter, set just above the first letter of a formula line. If the formula line opens with a single capital letter or a number and then an operator (a hat over `X = V DU`, `Y = A B` or `Z + W`), that letter now stays apart from the formula line and is no longer joined into it as `bX`. A hat over a formula that opens with a lowercase letter still joins it.

- **(pdf): Remove a repeated page stamp whose words join in more than one order.** (GH#2719)
  The repeated-margin-text check now also compares a line by its set of words. A footer that reads "A B" on some pages and "B A" on others counts as one line, and is removed as page furniture from every page after its first.

- **(ocr): Keep the first read of a picture when the table re-read finds no table.** (#2732)
  A standalone picture that the lattice test admits is now read again as one uniform block, and that read is adopted only if it finds a table. An upside-down table page, which the second read does not turn into a table, keeps its first read instead of the second.
- **(pdf): Apply graphics states recovered while visiting a Form XObject.** (#2752)
  Failed graphics-state dictionaries and indirect dash or opacity operands are retried only after object-stream loading publishes recovered objects, so later page strokes use recovered settings without repeatedly decoding a permanently broken stream.

- **(pdf): Resolve an indirect page `/ExtGState` once per resource scope in native path extraction.** (#2744)
  Each distinct `gs` name used to clone the whole dictionary, so thousands of names cost quadratic allocations; the container is now loaded once and borrowed, and returning from a Form XObject restores the page's parsed states instead of discarding them.
- **(pdf): Print a table once when two overlapping table detections hold the same words.** (#2739)
  When two native tables overlapped, the check that keeps both compared only the table with fewer rows, so a copy that regrouped its words into fewer, merged rows counted as new content and the page printed its words twice. The check now keeps a second table only when at least half of its words are words the rest of the page does not already print. A page with no text, such as a scanned page, has no words to count, so the check compares the rows of the two tables instead.

- **(ocr): Recover sideways scans containing two stacked tables and independently proved prose.** (#2497)
  Retain source words once, preserve both grids, and return table boxes in the source PDF frame.
  The detached geometry correction is limited to uncached, single-page Plain extraction with one plain text block above the first table; text between or below the tables keeps the existing fallback.

- **(ocr): Recover a sideways page with two tables when the first read held extra filtered words.** (#2497)
  The implicit orientation retry refused two rotated tables unless the first read's iterator word count equalled its reported count, so dictionary-filtered words left the page on its noise read. It now accepts iterator words that include every retained token (extra filtered words only widen the protection check), with the same exact table, geometry and protected-word proofs.

- **(pdf): Remove a painted orphan glyph beside a table whatever the glyph (#2616).**
  The standalone orphan that a painted table cell already carries was deleted only when it was an ASCII digit, so a "%", "-", currency sign or non-Latin digit stayed as a duplicate paragraph. It is now deleted when it is the single remaining source glyph of a painted overhang and the exact plain source paragraph; two competing glyphs, a multi-character word and blank text are still refused. A symbol candidate beside a digit candidate now counts as competing and is refused, where the symbol used to be ignored.

- **(ocr): Budget the CCITT preview of an invisible-layer probe per page.** (#2694, #2730)
  Each audited page may spend the per-page work derived from `max_iterations`, and all pages together at most six such budgets (960 million work units at the default, roughly 3 to 4 s of decode on a release build), so a multi-page scan with a junk layer on every page recovers its pages up to that ceiling (35 pages of 300 dpi) while a hostile file still gets bounded decode work. Before, one budget shared by the document let a 300 dpi scan recover only its first page or two.

- **(ocr): Limit the glued-cell veto's leader exemption to the gap at the leader (#2704).**
  A native token that held a leader run (`PMC0___PMC1`) was exempt from the veto as a whole, and a read `_____` or `-----` closed a gap even when it overlapped the glued word; now only the gap at the run is exempt, a read leader must lie inside the gap, and `⋯` counts as a leader and `‥` counts toward one.

- **(ocr): Run no-binarization contrast preprocessing on the 8-bit plane for genuinely gray input.** (#2699) Since ca6be8f1d3 the image workspace model charged this path about 17 bytes per pixel, which shrank or refused the sparse-retry fallback's upscale. A raster whose channels are equal in every pixel now becomes an 8-bit Pix and frees its RGB before background normalization and sharpening; the result is pixel-identical to the colour path, and the native phases peak at 7.0 bytes per pixel as measured with Leptonica's allocator (29,360,128 bytes at 2048x2048 against a modelled 29,360,384). Colour input keeps the colour path and its model, a step that fails after the RGB is freed keeps the best Pix already built, and an engine-selected JPEG upscale that does not fit shrinks to the largest size that does. The scale-dependent receipt OCR test was replaced with deterministic tests of the fallback gate (#2699); the lost receipt-header recovery is tracked in #2741.

- **(pdf): Honor graphics-state dash patterns and zero-opacity strokes in native PDF path extraction.** (#2332)
  All path consumers, including line, rectangle and table-grid extraction, receive ExtGState dash patterns and no stroke colour when the stroke opacity (`CA`) is zero or negative; this prevents false HTML headers from those separators.
  Repeated uses of a graphics-state name reuse its parsed settings until the resource scope changes.
  Partial stroke opacity, fill opacity (`ca`), soft masks (`SMask`) and blend modes (`BM`) remain unmodelled in path-based header visibility proof.

- **(pdf): Preserve native PDF leaf headers beneath repeated column groups.** (#2332)
  Grouped tables with repeated leaf labels and a shaded first data row retain those labels as header cells in HTML
  and document grids, preserving body cells, table bounds and spans.

- **(ocr): Keep the words that a glued native token hides on a table page (#2691).**
  The check that lets native text stand in for an OCR read credited a native word when the joined read words contained it, so a token such as `1ThreadOvf_PMC1` counted as agreement for the three words the read separates, and page 4443 of the Intel SDM lost `Ovf_PMC1` to `Ovf_PMC3`; a native word that joins read words a cell apart now keeps the OCR read.

- **(pdf): Stop a math line such as `X = Y` from reading as a numbered heading (#2688).**
  A leading roman numeral or number matched the heading enumerator whatever followed it, so a formula opening with a variable (`X = Y`, `2 + ...`), a decimal (`0.03 ± 0.01`, `1.5 = x`) or a DOI tail (`10.1109/IPIN.2018`) became a heading and split its paragraph. An enumerator now fails to be a heading when a math operator (`= + < > ^ ± ≤ −` and the like) follows it, or when `/` or `*` is followed directly by an operand, as in `12/1996`. So `3.2 ± 0.1`, `4.1 × Scaling` and `1.2/1.3 Results` are no longer headings, while `IV. Results`, `II Methods`, `1/ INTRODUCTION` and `IV - Results` still are.

- **(ocr): Recover tall scans after OCR preprocessing reduces their read resolution.** (#2502)
  Neutral RGB tall scans no longer require unfiltered page-paint streams: raw, Flate and ASCII85+Flate wrappers
  are eligible only after the decoded full-page draw and every RGB sample's neutrality are certified.
  Line geometry scales up with source density while preserving the prior limits below 150 DPI;
  memory, reading-order and recognition-quality checks remain in force.
  Original read-frame diagnostics survive automatic per-page OCR and document rebuilding, with accepted page IDs
  retained in order and the existing single-frame metadata object preserved for compatibility.

- **(pdf): Keep a native table whose box overlaps a grid that does not hold its rows (#2376).**
  Overlap arbitration treated an L-shaped grid and the block beside it as one detection and dropped the smaller table, so rows such as the Claims Trend block of a rate filing vanished from the table output; two native tables now count as duplicates only when most of the smaller one's rows reappear in the larger.

- **(ocr): Compare a table read's escaped cell pipes with their iterator words (Refs #2534).**
  The table renderer writes a `|` inside a cell as `\|`, so a cell word that starts with a bar differed from the Tesseract iterator word and the page never reached the native-ink proof. The escape is now undone before the comparison; the native-agreement share is unchanged. Intel SDM page 4311 returns its 298 native words instead of 874.

- **(ocr): Keep the native page when automatic OCR reads it as a table of the same words (Refs #2534).**
  A read that Tesseract laid out as a table is compared with its own iterator words with the table markup and ruling bars set aside, and it yields to the native text under the same unchanged 0.85 native-agreement share and glyph-ink proof as a plain read. The native page was followed by its whole OCR table (1315 words for 550 on Intel SDM page 815).

- **(php,elixir): Make the macOS PHP extension and Elixir NIF load without Homebrew's libheif.** (#2547)
  The v1.3.10 and earlier macOS releases linked `/opt/homebrew/opt/libheif/lib/libheif.1.dylib` by absolute path and did not ship it, so the whole binding failed to load on a Mac without it.
  PIE installs one file, so the PHP extension now links libheif, libde265 and libaom (decode only, no x265 or other encoder) statically, built from pinned sources at the macOS 15.0 floor; the archive ships their upstream notices, the GPL-3.0 text and a source inventory, the release loads it in PHP and decodes HEIC and AVIF before upload, and HEIF output is unavailable there.
  The Elixir NIF archive ships its four codec libraries as `@loader_path` siblings, staged and load-tested in the release job.
  See `THIRD_PARTY_LICENSES.md` for the licenses and how to relink the LGPL components.

- **(pdf): Join a ligature drawn as its own run to the rest of its word in Plain and Markdown output.** (#2322)
  The rich text assembler split `di` + `ﬀ` + `erent` at the ligature whenever the kerning gap passed 0.15 em; it now applies the same wider ligature-boundary threshold as the native extractor. OCR text keeps its string-level repair of a space after a ligature. The OCR structure heuristic's character-retention gate now counts a ligature as its letters, so a page with a ligature is no longer rejected.

- **(pdf): Keep an authored space after a ligature in Plain and Markdown output.** (#2322)
  A space following a ligature glyph ("staﬀ only") was removed by a string-level repair, welding two words into "staffonly". Word boundaries after a ligature now come only from the extractor's glyph geometry.

- **(pdf): Decode an over-budget JPEG page image at a reduced scale instead of refusing the page.** (#2682)
  A scan whose embedded JPEG rasters and canvas exceed the image working budget (for example a 6623x4678 RGB scan on a large page) is decoded at the finest 1/2, 1/4 or 1/8 IDCT scale that fits, so the page renders and takes the normal OCR route. Each axis of the reduced raster must reach 90% of the canvas pixels the image is painted over, less one pixel (`reduced >= 0.9 * painted - 1`); a 600 px reduced axis accepts a painted extent up to 667 px and refuses 668 px.
  Progressive, CMYK, colour-key-masked and in-data soft-masked JPEGs are still refused as before, as is a frame that disagrees with its image dictionary (the JPEG's own frame size decides the scale). A reduced decode that fails after admission is reported as the same budget refusal rather than rendering the page without the image.
  The decode relies on `jpeg-decoder`'s `rayon` worker (enabled for `xberg-native-pdf` on every non-wasm32 target; wasm32 uses its immediate worker), which converts each coefficient row as it is read; the default thread-per-component worker would queue rows without a bound. The decoder checks its buffer limit only after decoding, so the admission arithmetic is what bounds memory.

- **(ocr): Apply `max_threads` to PaddleOCR ONNX Runtime sessions.** (#2649)
  Sessions now take the installed process thread budget instead of the host default, and keep their intra-op and inter-op limits when an execution provider (including explicit CPU) is configured.

- **(pdf): Recover prose behind grayscale ICC page images (#2313).** A page whose only content is a full-page 8-bit gray image with a bounded display ICC profile now takes the visible-raster OCR route, so the prose in the image is no longer dropped in favour of the near-empty native text.

- **(pdf): Keep raised exponents and footnote marks inside their sentence on native pages (#2647).**
  Row and column ordering of preserved-geometry spans now treats a mark raised by up to 0.35em as sitting on its base line, and a mark after a closing bracket can be attached as a suffix, so `(x + 3y)2` and a footnote mark no longer become paragraphs of their own.

- **(pdf): Stop reading order from moving list items and words out of their sentence on native pages (GH#2658).**
  Ordering on a base line now applies only to a mark at most 0.8 of its base size that abuts the base, and a run of raised glyphs such as `(*)` moves as one or not at all. Bullets, text of nearly the same size as a bullet, and text set apart from a larger icon keep their own line.

- **(pdf): Keep unversioned arXiv identifiers in citations on the first pages (GH#2644).**
  Only a versioned identifier counts as the sidebar stamp, so a short citation ending in `arXiv:2107.09275` is no longer page furniture.

- **(ocr): Recover raster-proven dot-leader scan rows whatever the header, label and value text (GH#2616).**
  The full-scan leader recovery no longer requires an all-letter header and label or digit-only values. Rows are still refused unless every leader is proved as regular dots on the raster, each value lies in its header column and the hOCR words match exactly.

- **(ocr): Match a dot-leader table row by its source tokens and leader geometry, not letters and digits (GH#2616).**
  The hOCR row claim no longer requires an all-letter label and digit-only values; a value such as "$1,010" or "3.2%" or a label with a digit now keeps its one claimed row. The row's words must still equal the table cells and the leader must still sit alone between the label and the first value.

- **(ocr): Admit a sideways prose region by its word boxes, not by word letters (GH#2616).**
  The recognition-only retry no longer requires every word to be two or more letters, so a comma, digit, hyphen, apostrophe or "a" no longer keeps the sparse result. Instead the region needs a median line thickness above a page-relative floor, at most a small share of marks thinner than a third of that median, mostly word-shaped (not stroke-shaped) boxes and mostly tokens carrying a letter, all measured in the upright frame.

- **(ocr): Give an explicitly rotated table in-page bounds on the per-page route.** (#2497)
  An explicit `auto_rotate` read on the per-page route kept its table box in the upscaled OCR frame, so the box was twice as large as the page; it is now scaled to the render frame.

- **(ocr): Recover an implicitly rotated table together with a heading or prose.** (#2497)
  A sideways read of one table is accepted with text above or below it when every outside word is a separate Tesseract word lying wholly above or below the table and their row-by-row order is the returned text.

- **(ocr): Recover more tall scans: Indexed scans at 120 DPI, Markdown output and an explicit Tesseract page mode, and report when strips were not used.** (#2502)
  Indexed sources down to 120 DPI use source-image strips, each of which must match its source ink rows; bilevel, neutral RGB and JPEG scans rendered at 120 DPI did not read back as their source words and keep the 150 DPI floor. A Tesseract page mode such as PSM 3 no longer turns recovery off, and Markdown output recovers the words as plain paragraphs with a warning that no Markdown structure was rebuilt. Strips now join as separate paragraphs. A page read from a render downscaled below 150 DPI carries a warning that names why its strips were not used.

- **(pdf): Keep a marked native page that the configured OCR languages cannot read (GH#2536).**
  A page whose lost-glyph-marked text layer is nearly all in scripts none of the configured languages cover (Khmer under `eng`, Latin names aside) no longer goes to OCR, which read it as other letters and replaced it. A scanned page (pictures cover most of it, or a quarter of it with little text) is still read, the other pages of the document are read, and an `ocr` warning names the kept pages and the model to add.

- **(pdf): Keep any native page the configured OCR languages cannot read, also through a Tesseract pipeline, and warn when a scan is in such a script.** (#2536)
  The kept-page rule now also keeps an unmarked page in Khmer, Thai, Lao, Myanmar or an Indic script when its combining signs attach to letters as the script requires, and applies when `ocr.pipeline` is set: the native layer is kept when the Tesseract stage that runs first cannot read the script, while a PaddleOCR, VLM or custom stage, or a `vlm_fallback`, means no claim and leaves the page to OCR. A scanned page in such a script is still read, and an `ocr` warning now tells the caller which model to add. Glyphs with no Unicode value stay unrecovered without the right model.

- **(pdf): Let a Tesseract pipeline whose first stage reads the script OCR the page, and name the right setting in the kept-page and scan-page warnings.** (#2738)
  A pipeline with a `khm` stage ahead of an `eng` stage no longer keeps the native layer of a Khmer page, so the `khm` stage reads it again. A page with many scripts of a few letters each no longer stops the extraction with an index error. The stage languages now follow `tesseract_config.language` the way the OCR run does. The kept-page warning says that glyphs stay lost only when a kept page has lost-glyph markers, and it names the marked pages only when some kept page is unmarked. Both warnings name the setting the shown languages came from (`ocr.language`, the first stage's `language` or `tesseract_config.language`). When the first stage fails, is unavailable or scores below `pipeline_min_quality`, the next stage runs and may replace the native layer, so a `[khm, eng]` pipeline whose `khm` model is missing can still read the page as Latin letters.

- **(pdf): A bullet set in a larger font or drawn after its text no longer splits its list item.** (#2443)
  The bullet is no longer taken as the base of a script run for the text beside it, and a bullet that follows its body in the content stream is placed ahead of it, so wrapped items keep their lines and line-end hyphens are joined.

- **(pdf): Table rows are no longer repeated as prose when a marker or sign splits a cell.** (#2443)
  A short run inside one table cell, such as a superscript citation or either side of a multiplication sign, is now treated as that cell's text instead of being emitted a second time beside the table.

- **(pdf): Rules that belong to another section no longer split a ruled band into columns.** (#2443)
  A column rule lying wholly beyond the ruled rows is no longer projected across them when other rules run through them, so a list column beside a section divider is not read as a two-column table that swallows the page heading.

- **(ocr): Keep a wrapped figure caption as text instead of splitting it into table cells.**
  A caption that spans several lines and whose cell grid a heuristic invented is now certified by its line geometry alone: the last line is at most half the first line's width, the left and right margins of the middle lines agree within a sixth of the median line height, and a whole word in an intact interior line crosses every proposed column cut. The `Figure N:` label and the closing period are no longer required, and a two-column block of text is still kept as a table (#2616).
  This rewrite still runs only with `use_cache: false`, Plain output and English Tesseract defaults, so a default extraction does not reach it.

- **(engine): Run extraction on the 16 MiB runtime so FFI hosts with small thread stacks do not overflow.**
  `Engine::extract`, the batch entry points and layout-page extraction poll their work as a task on the shared large-stack runtime instead of the caller's thread, so a JVM or other host thread of 1 to 2 MB no longer crashes the process (E2E Java `ContractTest` segfault). The task keeps the caller's tracing subscriber and span. Dropping the call aborts the task at its next await: blocking work already running finishes first.
- **(ocr): Keep the raw pixels of coloured graphics that have a dark mean.**
  Automatic preprocessing for dense dark glyphs on a dark-mean image now applies only when most of the dark ink
  is neutral: black, grey, or slate and brown-black ink with up to 40 levels between the strongest and weakest
  colour channel. Saturated text such as pure blue counts as dark ink. A coloured graphic with a dark mean skips
  the preprocessing and reaches OCR with its raw pixels (GH#2515).

- **(ocr): Place mixed-route OCR lines in the resampled frame when matching them to native text.**
  OCR paragraphs are in the frame preprocessing resampled the render to, but the page-text merge scaled them by the render's size, so every OCR box was twice its true size at a 150 to 300 DPI resample, owned no native line and the native lines were prepended a second time (#2313).

- **(redaction): Run external-redaction extraction on the shared 16 MiB runtime.**
  The poll chain of `extract_with_external_redaction` needs about 2.2 MB, which overflowed the roughly 2 MB host thread of an FFI caller such as the JVM and crashed the process. Dropping the call still cancels the work.

- **(ocr): Keep certified table rows on one line in Markdown and Djot output of a standalone image.** (#2432)
  The sparse-row recovery that already served Plain output now also serves Markdown and Djot; the recognized words and amounts are unchanged.

- **(pdf): Return a native page's text once when automatic OCR only re-reads its ink.** (#2534)
  When every word Tesseract read lies on a native text line, shows glyph ink and sits on a background that only glyphs vary, the native page replaces the read even where the words differ.
  Words over rasters, outlines, rules, partial fills or overpaint keep their OCR alternative.
  The proof also runs on the direct Tesseract route used when PaddleOCR is not compiled in.
  A native word that no glyph paint shows (under a fill, clipped or invisible) or a native layer that the read contradicts at more than 15% of its words keeps the OCR page.

- **(pdf): Keep certified native text on an automatically selected OCR page in a mixed document.** (#2533)
  Pages with unmapped glyphs now reach the same glyph-ink proof as a one-page document, so an OCR read that misses mapped source text does not replace it. Explicitly forced OCR and pages without that proof keep their existing behavior.

- **(pdf): Keep a one-page native text layer whose lost glyphs the automatic OCR read does not recover.** (#2533)
  The ink-ownership proof now accepts a layer with glyphs that have no Unicode value when each sits in a located native word and no kind of lost glyph (by font, size and advance) of three places or more is read as letters or digits at half its places, tolerates read words only on flat blank paper between native words, requires glyph ink in every located lost glyph and native word, and lets Markdown block structure yield to the native page. A read that sees a kind of three or more lost glyphs as text at half its places, or any word over a raster, outline, rule or fill, keeps the OCR page.

- **(pdf): Recover a verified embedded prose line with automatic Tesseract segmentation.** (#2327)
  When both caches are off, the resize retry can add the missing line to Plain page text while preserving the original nested OCR result.

- **(pdf): Recover missing words behind partial hidden text on supported CCITT Group 4 scans.** (#2425)
  Automatic Tesseract OCR checks the raster only when the bilevel image and its complete Group 4 stream can be verified.
  Scans with complete hidden text keep their native text.

- **(pdf): Preserve verified scanned figure captions as prose.** (#2530)
  Uncached Plain extraction can remove a false caption table when stock OCR and the page raster confirm the same text and geometry.

- **(pdf): Place the heading and table from an upright scanned page in PDF points.** (#2497)
  Applies to single-page detached Tesseract OCR with caching off and plain output with document structure requested.
  A verified resize preserves the heading and table content and gives both bounds the same bottom-left page origin.

- **(pdf): Recover verified right-column text from supported tall indexed-color scans.** (#2426)
  Uncached Plain extraction with stock Tesseract can add a complete ordered run while preserving existing text and tables.

- **(ocr): Recover language-only explicit sideways PDF scans with cached Tesseract orientation data.** (#2496)
  Preserve caller rotation and segmentation choices, original word geometry and protected primary content.
  Recover counterclockwise table rows when native line membership proves the source geometry.
  Recover a single sideways prose paragraph when native lines prove the complete text and geometry.
  Recover an upside-down table by reading the half-turned raster once Tesseract reports every text block rotated 180 degrees, returning geometry in the original frame.
  Bind result caches and native handles to observed orientation model revisions, including explicit pipeline stages.

- **(ocr): Correct detached two-table PDF reading order and coordinates.** (#2497)
  Keep the recognized cells and convert their render bounds to PDF points before returning the tables.

- **(ocr): Keep source-proven dot-leader scan rows together in Auto OCR.** (Refs #2503)
  A page-wide scan table whose leader runs are confirmed as regular dots in the recognition raster is returned as one table with every label and value kept and the leader dots left out.

- **(ocr): Remove duplicate dot-leader rows from standalone Tesseract PSM 6 output.** (#2503)
  Require matching text and geometry while preserving numeric cells, default OCR, and PDF output.

- **(pdf): Preserve complete painted exhibit tables when cells contain decorative text fills.** (#2376)
  Verify source cell contents and suppress only an exactly proven residual duplicate digit.

- **(pdf): Recover painted fee and company tables without dropping adjacent content.** (#2376)
  Preserve independent tables and remove duplicate row text only when its original source is proven.

- **(pdf): Keep source-ruled numeric cells separate in fallback tables.** (#2376)
  Repeated painted separators constrain column clustering while preserving each accepted table's ordered words.
  Fractional rules can refine existing cells when the original word bounds clear the full painted stroke.

- **(pdf): Admit fully source-ruled wide tables with merged index cells.** (#2376)
  Certify 16-18 column grids only when every cell, merged span and word is proven by plain painted rules on the page.

- **(pdf): Keep certified side-by-side panel lines in source order in plain output.** (#2537)
  Preserve complete rows and page boundaries in documents that mix panels with ordinary text.

- **(pdf): Keep only RGB pixels in rendered pages retained during OCR.**
  Preserve raster dimensions and backend PNG bytes while allowing upright scans to fit the default memory budget.

- **(pdf): Keep aligned indexed assignments on separate display rows.** (#2443)
  Preserve their subscripts and separate each verified row from surrounding prose.

- **(pdf): Keep raised chart legend markers with their labels.** (#2443)
  Join them only where neighboring source rows prove the association and no layout hints own the page.

- **(pdf): Preserve figure tick labels beside native tables.** (#2443)
  Keep the original label when the adjoining residual digit is already carried by a proven table cell.

- **(pdf): Keep parallel form legends out of synthetic heading blocks.** (#2543)
  Preserve source reading order and ForceBold rendering.

- **(pdf): Keep separate figure labels out of empty inferred table row spans.** (#2332)
  Require an existing source anchor before borrowing words from covered rows.

- **(pdf): Keep readable native text when automatic OCR cannot see unpainted Type0 glyphs.** (#2533)
  Require actual page-local render loss and a plain OCR read that omits most native word occurrences.

- **(pdf): Keep mapped Khmer text when automatic English OCR produces only noise.** (#2536)
  Preserve the native text and record why the direct single-page OCR result was declined.

- **(pdf): Keep a visibly painted title when automatic OCR noise is discarded.** (#2312)
  The direct Tesseract fallback keeps a verified native title and the original discard warning.

- **(pdf): Preserve one copy of native text when OCR reads the same words.** (#2534)
  Require an exact match between native words and the Tesseract iterator, including their positions.

- **(ocr): Keep PNG and TIFF retries within the image budget when upscaling known-DPI rasters.** (#2530)
  Use the finest safe unclamped DPI when the requested upscale exceeds native workspace.

- **(ocr): Recover oversized PNG and TIFF rasters with unknown DPI within the image budget.** (#2530)
  Fit a reduced raster before estimating its DPI when the original cannot fit that phase.
  Fit raw PDF page renders that pass the render budget but exceed native OCR workspace.
  Retry a weak read from a certified full-page JPEG without replacing an existing table.

- **(ocr): Report the actual budget limit when upscaling a JPEG with unknown DPI.** (#2531)
  Check the planned upscale without retrying at a reduced decode that cannot reuse the source DPI estimate.

- **(ocr): Warn when a budget-clamped PDF render returns suspected noise.** (#2532)
  Keep the diagnostic quiet when the selected read retains a table or formula.

- **(paddle): Preserve reading order after automatically correcting rotated PDF pages.** (#2519)
  Compose the page hint with the applied pixel correction so upright labels are not reversed.

- **(pdf): Retain source words in merged table cells and two drawn header bands.** (#2332)
  Preserve the source-backed header rowspan in HTML and the public table grid.

- **(pdf): Paint identity CID TrueType glyphs when Unicode decoding loses characters.** (#2309)
  Preserve embedded outlines despite sparse Unicode font maps, in automatic and forced OCR.

- **(pdf): Preserve word boundaries when Type0 fonts lack Unicode mappings.** (#2328)
  Route unmapped glyphs through OCR decisions, warn when native characters are unavailable, and retain OCR-owned text.

- **(pdf): Retain independently shaded source table headers and ruled cell boundaries.** (#2332)
  Keep nested text backgrounds from creating extra columns while preserving separate table records.

- **(pdf): Preserve source-proven grouped leaf headers in ruled tables.** (#2332)
  Two bold group bands over a bold leaf band stay header rows only when dark solid rules, which may be split into collinear segments, underline both. Path extraction now records the `d` dash pattern on stroked paths, so dashed rules are told apart from solid ones; a dash or transparency set through an ExtGState (`gs`) is not yet seen. A grid whose body rows are ruled in the same colour and weight as the leaf rule keeps no header from this path, and gridline detection in recovered diagrams groups lines by colour as well as by colour and dash style.


- **(ocr): Require dense glyph evidence before preprocessing dark-mean images by default.** (#2515)
  Retain words beside large graphics while preserving the dense-text recovery introduced in #2475.

- **(pdf): Retain confirmed readable native lines that automatic OCR misses.** (#2439)
  Retain uncertified OCR alternatives and their formatting beside restored native rows.
  Preserve original native page order when every OCR occurrence is certified and its final payload is plain.

- **(pdf): Restore caller font bindings before resuming text after a Form.** (#2516)
  Keep caller words when a Form shadows the selected font resource name, on cold and warm reads.
  Resolve a Form's own font binding even when its name and size match the caller's selection.

- **(pdf): Keep complete signed decimal powers with their native bases.** (#2511)
  Verify separately drawn points against actual glyph advances while retaining ambiguous fractions and existing scripts.

- **(pdf): Keep stepped-baseline prose panels in source order and preserve their separate lines.** (#2323)

- **(markdown): Retain rendered headings and scripts after canonical Unicode normalization.** (#2512)
  Equivalent plain-text forms keep their rendering, while meaningful post-processor edits still trigger the stale-rendering safeguard.

- **(pdf): Preserve supported glyphs in mixed unembedded text runs on fontless hosts.** (#2510)
  Use bundled DejaVu Sans Bold for bold text, retain placeholders for unavailable characters, and orient CJK fallback glyphs correctly.

- **(pdf): Report empty OCR reads after the raster budget reduces page DPI.** (#2504)
  Include requested and actual DPI after retries, without warning on useful low-DPI text or recovered tables.

- **(pdf): Keep touching glyphs together when coordinate rounding produces a tiny overlap.**
  Avoid inserting a word boundary while retaining correction for genuine fallback-font overlap.

- **(pdf): Admit stored image inputs before additional whole-document recovery copies.** (#2339)
  Preserve legacy formats; the per-page stored-input allowance does not bound filter-expanded or pixel/PNG outputs.

- **(pdf): Keep the preferred OCR stage policy when retrying original embedded images.** (#2339)
  Preserve language, preprocessing and backend options before page-render hints in whole-document and detached recovery.

- **(pdf): Preserve cancellation and the smaller image byte cap during whole-document OCR recovery.** (#2339)
  Forward the minimum outer/OCR cap without changing other security limits, and discard retry results when cancelled.
  Whole-document and page recovery take their other security limits from the same base.

- **(ocr): Decode oversized JPEG scans at a reduced IDCT scale that fits the content size limit.** (#2339)
  8-bit color and grayscale scans, baseline or progressive, decode at 1/2, 1/4 or 1/8.
  Raw Tesseract ingestion also checks its native image buffers before accepting a fitting full RGB raster.
  The scale is the finest one that fits and needs no DPI resize, so a higher limit never turns a success into an error.
  When no fitting scale meets target_dpi, the error names the limit to raise and the target_dpi to set.
  Lossless, hierarchical, arithmetic-coded, 12-bit and CMYK scans keep the size-limit error, which names the reason.
  Standalone JPEG files that only a reduced decode can take are no longer rejected by the image metadata check.
  With `ocr.pipeline` set, a JPEG whose per-stage PNG encode does not fit decodes at the finest scale whose encode fits.
  Known no-binarization contrast checks its native image buffers and refuses an unaffordable configured target before Pix allocation.

- **(pdf): Recover text on tall scans when an OCR strip contains only blank space.** (#2501)

- **(pdf): Recover OCR rows from tall one-bit grayscale scans.** (#2502)
  Decode bounded packed rows and verify the recovered lines against the source raster bands.
  Recover tall RGB scans with a verified full-page draw and three identical channel values at each pixel.
  Retry certified grayscale JPEG scans in bounded strips after the renderer reports a source-image budget excess.
  Decode checked PNG predictor rows and black-and-white indexed palettes before certifying strips.

- **(pdf): Keep upper rows first when stacked ruled grids merge.** (#2494)

- **(redaction): Cover terms split by a superscript in HTML headings or font changes in PDF text.** (#2363)
  When the verified plain rendering covers the crossing, return it with the term redacted.

- **(pdf): Fit OCR page rasters to the native allocation budget before rendering.** (#2491)
  DPI selection accounts for the actual raster dimensions, PNG workspace and any CMYK or spot planes.

- **(ocr): Keep warnings from failed PDF rasterization out of later extraction results.**

- **(pdf): Enforce the request's raster budget before allocating layout-only page images.** (#2491)
  The native check uses the actual page dimensions and final render DPI, including the 72 DPI floor.

- **(pdf): Preserve the space after a raised footnote number at the start of its definition.** (#2443)

- **(ocr): Recover complete tables on sideways PDF scans with a cached orientation model.** (#2314)
  The implicit Tesseract retry accepts a confident complete table and maps it to the PDF page.

- **(ocr): Recover two complete tables on a sideways scan when all words are accounted for.** (#2497)

- **(pdf): Preserve raised numeric suffixes when native glyph edges slightly overlap.** (#2489)
  Keep negative exponents and squared labels with their bases in Markdown.

- **(ocr): Keep cover and map labels in the page text.** (#2431)
  Restore dictionary-filtered lines when every word matches the rejected candidate at its original position.
  A shared header keeps sparse ledgers with recurring numeric columns eligible.
  The same shape rule applies to Tesseract and PaddleOCR.

- **(docker): Stop storing the binary twice in the core and full images.**
  Remove a `chmod` that changed nothing but copied the whole binary into another layer.

- **(pdf): Preserve long Form text when the caller marks the Form invocation as placed PDF content.** (#2310)

- **(pdf): Recover text missing from partial invisible scan layers.** (#2425, #2506)
  In Plain Auto extraction with built-in Tesseract, check eligible scans independently for uncovered text
  bands and trailing words, within the configured iteration limits. Keep each native page unless OCR preserves
  its text and adds readable content.

- **(pdf): Keep aligned financial amounts with their labels.** (#2315)
  Preserve complete native PDF rows in plain text and keep proven financial body rows out of Markdown headers.

- **(pdf): Preserve characters mapped by multiline ToUnicode ranges.** (#2476)
  Keep destination arrays together so their entries cannot be mistaken for separate character ranges.

- **(pdf): Keep row-painted table values with their labels across wide gutters in plain text.** (#2343)
  Preserve blank value cells and wrapped labels when source order confirms the visual pairs.

- **(tokenizers): Preserve callback errors when registering a tokenizer backend.** (#2374)

- **(ocr): Preserve letters on images of repeated aligned text.** (#2472)
  Automatic preprocessing can recover dark glyphs on a light background when the glyphs lower the image-wide mean.

- **(ocr): Downscale oversized PNG and TIFF inputs within the workspace budget.** (#2618, #2619)
  Preserve explicit preprocessing and use a bounded DPI fallback when the source does not supply one.

- **(ocr): Recover tall scans with default PNG predictor parameters.** (#2620)
  Accept omitted values when the PDF defaults match the source image.

- **(pdf): Render layout rasters within the configured memory budget.** (#2332)
  Lower DPI within the allowed range while keeping detections in the original PDF point frame.

- **(pdf): Certify painted table-cell overhangs by exact source geometry.** (#2616)
  Keep matching values with alternate number formatting in their original cells.

- **(ocr): Separate a merged table header from its first record in standalone OCR.** (#2432)
  Require independent row geometry and preserve recognized text and later cell boundaries.

- **(pdf): Preserve a figure tick beside a table using exact source ownership.** (#2616)
  A decimal comma no longer blocks recovery of the outside figure label.

- **(pdf): Keep parallel bold legend rows separate using their geometry.** (#2615)
  Punctuation, case and digits no longer change the decision for equally aligned entries.

- **(pdf): Keep wrapped text in painted record rows with adjacent cells and contrasting spacers.** (#2613)

- **(pdf): Recover rejected multirow painted double-rule grids.** (#2614)
  Preserve tables the detector already accepts, including their empty columns.

- **(ocr): Remove a certified duplicate glyph from a recovered photo title.** (#2312)
  Both text projections must identify the same standalone source occurrence.

- **(pdf): Recover hidden CCITT prose across two audited content streams.** (#2425)
  The split path requires bounded, independently complete raw paint and text streams.

- **(ocr): Recover certified tall grayscale Flate strips down to 120 DPI.** (#2502)
  Require each recovered strip to match the source ink rows.

- **(ocr): Recover verified embedded PDF prose with caching enabled.** (#2327)
  Preserve the primary cache identity and bypass the cache for the private source-DPI retry.

- **(pdf): Recover a missing embedded prose line on any page of a multi-page PDF.** (#2327)
  The source-DPI retry was limited to single-page documents; it now follows the page that holds the single last-painted picture, checks the native text of that page only, and an unrecovered picture's OCR text is placed after its own page instead of after the document.

- **(pdf): Recover full-page grayscale raster prose beside visible native stubs.** (#2313)
  Bound the source paint audit and keep the native lines in order.

- **(pdf): Recover verified outlined prose beside sparse native text.** (#2313)
  Auto OCR checks bounded vector rows and preserves the original native text.

- **(ocr): Place recovered PNG photo titles in source order.** (#2611)
  Use OCR line geometry to place the title while preserving the original document and page text.

- **(ocr): Recover a verified photo title in Markdown PNG extraction.** (#2312)
  Append the title as a paragraph while preserving the original Markdown and page text.

- **(ocr): Recover an outlined photo title in Plain PNG extraction, including cache hits.** (#2312)
  Add a title supported by two distinct source masks while keeping the original OCR text.

- **(ocr): Keep rows together in standalone sparse OCR page text.** (#2432)
  A single accepted table can supply Plain and Djot page rows when it retains every recognized token occurrence.
  In Plain standalone OCR, recover a rejected grid when its geometry and the original text prove every row.

- **(pdf): Recover a missing prose line in an embedded image with explicit PSM 6.** (#2327)
  Use the proven source density to resize the retry while keeping the original nested OCR result.

- **(ocr): Recover complete numeric rows in scanned PDF tables.** (#2432)
  Require matching word inventory and separate, aligned numeric columns when the first table reconstruction fails.

- **(pdf): Keep ruled table rows on the correct side of a section divider.** (#2376)
  Use exact shared row edges when both the original and corrected sections pass native table admission.

- **(ocr): Skip automatic PaddleOCR on uniform PDF page rasters.** (#2430)
  Faint text and usable embedded images remain eligible for fallback.

- **(pdf): Expose complete native table headers as column labels.** (#2332)
  A single fully labeled header row survives in the structured table when its original grid still matches.

- **(dart): Fix compilation of the generated SDK.**
  Keep the unsigned-value checks aligned with the types emitted by Flutter Rust Bridge.

- **(pdf): Retry low-confidence OCR on rotated scanned pages.** (#2389)
  Implicit Tesseract OCR can use a verified cached orientation model to recover text. Explicit OCR settings still apply.

- **(pdf): Render Latin text on hosts without a matching font.** (#2347)
  Use the bundled DejaVu Sans face for covered Latin glyphs after embedded and system font lookup fails.

- **(tests): Check the specific blank-PDF image decode warning.** (#2427)
  Full-feature tests allow unrelated OCR fallback diagnostics on blank pages.

- **(structured): Apply the caller's raster budget to PDF pages.** (#2352)
  Check the canvas, CMYK and spot-color planes, and PNG encoding allowance before allocating the page raster.

- **(pdf): Report an undecodable painted image when OCR sees a blank page.** (#2427)
  The warning names the affected page when neither decoding nor raw image recovery can supply the image.

- **(pdf): Render scanned images with a singleton DeviceGray color-space array.** (#2427)
  This also covers CCITT Group 4 images that omit Rows and wrap the filter and parameters in arrays.

- **(docx): Return complete hyperlink labels for links split across styled runs.** Redaction can match the full label.
  With redaction disabled, the label shows the complete first occurrence. URI deduplication stays the same.

- **(redaction): Redact source IDs in HTML heading metadata.** (#2363)

- **(redaction): Redact annotation authors and decoded QR payloads.** Cover the structured fields alongside the
  document text while preserving unrelated annotation and QR metadata.

- **(redaction): Distinguish generated Markdown tags from source text.** Preserve genuine inline formatting while
  redacting literal tag names in annotations and raw source blocks.

- **(pdf): Preserve word and line breaks when layout reorders upright PDF text.** (#2373)
  Keep table cells searchable and leave adjacent source glyphs attached.

- **(redaction): Redact repeated terms split by Djot inline styling.** Return verified redacted plain text for
  supported Djot documents and pages with a format warning. Reject opaque output, including raw page-marker blocks.

- **(windows): Bundle the Microsoft runtime dependencies of PHP, CLI, C FFI, and Elixir releases.** (#2370)
  Verify hard and delayed DLL imports and runtime notices in the final archives before upload.

- **(redaction): Rebuild verified HTML from redacted plain text.** (#2361)
  Cover repeated terms split by inline styling while keeping HTML output and reporting the loss of original formatting.
  Reject HTML whose original text cannot be verified or whose captured content changes during post-processing.

- **(pdf): Keep witnessed prime marks beside their operands.** (#2443)
  Preserve literal glyphs when both original neighboring spans survive the PDF layout passes.

- **(pdf): Keep same-baseline small caps out of script markup.** (#2443)
  Preserve genuine scripts and their byte ranges when assembling native PDF text.

- **(redaction): Explain when a post-processor changes the document pages.** (#2447)
  Redaction rejects added, removed, or renumbered pages and reports that the post-processor changed them.

- **(redaction): Validate Markdown projection edges and remove emptied inline wrappers on Drop.** (#2446)
  Reject unverified source gaps, including reference definitions at the document edges.
  Remove delimiters owned by fully redacted inline spans while preserving unrelated markup and original finding offsets.

- **(rust): Restore strict Clippy and no-ORT PDF feature builds.**
  Keep script fixture geometry bit-exact and compile the OCR structure wrapper only where it is used.

- **(pdf): Keep proved native superscript suffixes with their base text across paragraph grouping.** (#2275)
  Preserve original glyph geometry and unrelated paragraphs; overlapping suffixes retain their existing behavior.
  Attach surviving suffix records after semantic refinement to preserve references, list boundaries and existing scripts.
  Keep existing script annotations when a new suffix would exceed the legacy base search window.
  Release discarded hierarchy data before constructing script-preserving paragraphs.
  Leave stacked fractions and incomplete raised cohorts unassociated, preserving their original records and annotations.
  Reject opposing fraction records even when their native semantic or style metadata differs.

- **(redaction): Cover repeated inline-styled terms in Markdown and DOCX/PDF output.** (#2361)
  Verify each rendered occurrence against its own plain backup before retaining formatting or returning a plain fallback.
  Preserve untrusted HTML-derived Markdown obligations through processor edits and keep original finding offsets.
  Keep the projection available with standalone redaction and runtime-free processing.
  Preserve literal handling for inputs without an internal document body.

- **(pdf, pptx): Write one page marker for every page and slide, an empty one included.** (GH#2422)
  With `pages.insert_page_markers` on, a PDF page with no content got no marker in the structured path.
  A PPTX deck got one marker for all its slides.
  Expected change: output for these documents now holds one marker per page or slide.
  A three-slide deck shows markers 1, 2 and 3.
  A page or slide that opens with a list gets its marker on its own line before the list,
  in Markdown and HTML as in the other formats.
  A slide that fails to parse still gets its marker.

- **(tests): Keep cache-depth and DOCX list regressions compatible with strict Clippy checks.**

- **(ocr): Fetch and read the document orientation model at one revision, so a synced cache is complete offline.** (GH#2417)
  The orientation detector, the PaddleOCR model manifest and the table models now share one pinned revision of the model repository.
  A cache that the manifest or the cache warm fills holds every file the engine reads. Caches that hold the table models at the older revision download them once more.

- **(pdf): Apply the ordinary span page-bounds policy to configured spans.** (#2379)
  Keep partial overlaps and the 2-point MediaBox tolerance while preserving custom merging.
  Raw character extraction retains off-page characters and PDF coordinates for inspection.

- **(pdf): Keep body text placed inside Form XObjects in spans and characters.** (#2310, #2380)
  Judge each placed group against the visible page text after clipping, in either paint order.
  Keep the 800-character minimum and half-duplicate threshold; characters use the span verdict.
  Duplicate matching still treats punctuation and spacing differences as equivalent in matching lines.
  Column-painted tables with repeated labels, skewed symbol grids, and mostly duplicated groups can still lose
  unique text. Each extraction view now uses its own page dimensions for figure and wrapper clipping.

- **(pdf): Recognize escaped PlacedPDF form tags and keep cached output independent of earlier page reads.** (#2378)
  Preserve a child's own verdict after its parent's placed scope ends, including inside a placed page scope.

- **(pdf): Keep nested Form text independent of earlier page reads at the recursion limit.** (#2377)

- **(pdf): Omit generated image placeholders when image extraction is disabled.** (#2386)
  Preserve recognized image text and the legacy PDF image-extraction opt-in.

- **(pdf): Keep cached Form spans tied to the destination page clipping context.** (#2382)

- **(tests): Preserve literal list-boundary comments in multiline and indented code during output-parity checks.**

- **(tests): Compare visible list and image text consistently across Markdown and HTML output.**

- **(docx): Preserve ordered and bullet item kinds at each level of a shared numbering definition.**

- **(tests): Check preserved source ordinals as ordered Markdown in image and DOCX list regressions.**

- **(python): Preserve existing enum stringification when targeting Python 3.11.**

- **(tests): Assert public `IoError` for missing files in the CrewAI integration CI checks.**

- **(tests): Borrow fenced-code expectations in Markdown round-trip checks.**

- **(ci): Run Go lint with the locked Go 1.27 toolchain and a compatible linter.**

- **(swift): Build release artifacts for the declared macOS 15 minimum.**

- **(formula): Honor `XBERG_CACHE_DIR` for formula recognition models.** (#2409)
  Preserve explicit model directories and the Hugging Face cache fallback when the override is unset.

- **(tests): Respect optional PDF column metadata in page snapshot checks.**
  Preserve canonical table equality and require all 16 distinct output-mode combinations.

- **(markdown): Preserve source decimal list starts, gaps, and nested list boundaries.** (#2387)
  Keep literal clause labels and necessary default-mode escapes in prose, code, and link destinations.
  Retain structural list separators without changing the unescaped-prose option.
  Restart unlabelled continuations before the CommonMark nine-digit ordinal limit is exceeded.
  Render flat alphabetic clauses as paragraphs when their own source list also contains decimal clauses.

- **(redaction): Preserve current formatted edits across supported paragraphs.** (#2361)
  Keep original paragraph whitespace and surrounding literal formatting while redacting current text.
  Split, escaped and code findings retain complete source coverage and semantic hash/token identities.

- **(ci): Bound PNG decoding in the V3 parity capture.**
  Use audited image decoders with default security limits before validating parity inputs.
  Preserve the decoder and security-budget sources with the selected producer inputs.

- **(tesseract): Keep routine native and WASM build progress out of Cargo warnings.**
  Preserve native link directives and actionable fallback diagnostics for strict downstream builds.

- **(ci): Align Python imports and UTC aliases with the Python 3.11 baseline.**
  Use the native exception-group builtin and canonical import ordering in CI helpers.

- **(ci): Check no-runtime test feature invariants at compile time.**
  Keep the email Clippy gate passing without suppressing its constant-assertion check.

- **(pdf): Recognize escaped PlacedPDF names in page-level text decisions.** (#2378)
  Apply the existing parsed page policy to escaped names; form and cross-page cache handling remain separate.

- **(redaction): Check raw and decoded findings before returning edited formatted text.** (#2358, #2363)
  Preserve semantic token and hash identity, reject incomplete overlap coverage, and bound literal planning work.
  Reject unsupported syntax in witnessed formatted-only edits while retaining trusted generated Markdown behavior.

- **(redaction): Keep source tag names redactable in Markdown and preserve edited output.** (#2358)
  Decline tag-syntax exemptions for raw blocks and rendered annotation values; preserve renderer-owned styles elsewhere.
  Return current formatted-only and same-frame page edits through warned literal fallbacks.
  Retain changed-content precedence when processors edit conflicting plain and formatted views.

- **(redaction): Preserve current text after legitimate post-processor edits.** (#2358)
  Discard unproved generated Markdown with a warning and return current redacted plain text.
  Keep page and result identity guards, and preserve unchanged formatting after later no-op processors.

- **(redaction): Preserve verified generated Markdown formatting while redacting its content.** (#2358)
  Treat superscript and underline syntax separately from source text, including per-page output.
  Use an explicit plain-text fallback when a match crosses generated syntax; keep literal and code text redactable.

- **(markdown): Render document superscript and underline annotations with standard inline HTML.** (#2358)
  Emit `<sup>` and `<u>` while retaining source escaping, nested formatting and table-cell scripts.
  Preserve those styles when DOCX/PDF writers parse the postprocessed Markdown; leave code and malformed tags literal.
  The HTML converter's pre-rendered Markdown and explicit conversion options retain their existing notation.

- **(ci): Keep publish workflow contract imports in canonical order.**
  Preserve the test assertions while satisfying the pinned Python import-order gate.

- **(pdf): Preserve real operators and operands across content-stream comments.** (#2399)
  Skip comment bytes before dispatch and EOF recovery so they cannot become marked-content tags or text operators.

- **(keywords): Preserve source token order and merge possessive variants in YAKE results.** (#2388)
  Stop inventing repeated prefixes around apostrophes while preserving contractions and genuine repeated names.

- **(markdown): Keep frontmatter in metadata and out of rendered body, elements, and chunks.** (#2392)
  Preserve typed YAML and TOML values without emitting Rust debug representations as document text.

- **(llm): Sanitize provider schema names while preserving caller prompts and JSON Schema references.** (#2385)

- **(plugin): Refresh local source and root served-skill pins during bundle generation.**
  Verify both locks before bundle and public-skill checks while preserving remote imports.
  Keep the checks in Xberg's task overrides so toolchain synchronization preserves them.

- **(python): Preserve typed extraction errors, including progress-enabled calls.** (#2383)
  Load canonical Rust error metadata before generating public APIs and reuse typed conversion in progress adapters.
  Mark test setup overrides for strict Python type checking across supported interpreter versions.

- **(publish): Authenticate NuGet publishing with short-lived trusted-publisher credentials for `naaman-xberg`.**

- **(layout): Align PP-DocLayoutV3 cubic inputs, singleton dispatch and output validation.** (#2284)
  Use fixed-support cubic resizing and ordered single-image forwards while validating paired rank-bearing rows and masks.
  Capture tasks require one executed test and two forwards with complete named tensor artifacts and matching sizes and hashes.
  Bind preserved producer sources to compiled bytes before constructing the model.
  Decode captured PNGs from hashed bytes and reject input changes during a forward.
  Restore Clippy-clean V3 and ORT test builds without changing inference or capture behavior.
  Public detection ordering and model defaults stay unchanged; model-quality qualification remains pending.

- **(ci): Build Windows Python wheels with the reviewed Visual Studio 2022 runtime.** (#2368)
  Pin smoke and release wheel jobs to the runner that provides the licensed CRT redistribution source.

- **(ocr): Compile the standalone Sceptre ORT feature with its existing discovery module.** (#2364)
  Check the isolated feature closure in CI so other engines cannot mask the required module gate.

- **(dependencies): Require released Sceptre 0.10.1 across Rust dependency graphs.** (#2364)
  Refresh workspace, Ruby, Elixir and Rust E2E locks while retaining the current ONNX Runtime pins.
  Rust E2E and Elixir resolve Windows-sys edges from 0.59.0 to 0.61.2; Ruby resolves them from 0.61.2 to 0.59.0.
  Ruby's phonemizer dependency resolves image 0.24.9 in place of 0.25.10.

- **(pdf): Preserve admitted table rows when a section would be too small to retain.** (#2332)
  Keep the original table and metadata together instead of dropping a partition; valid multi-section tables still split.

- **(ci): Run frozen caption contracts with Python 3.13.18.** (GH#2353)
  Select the locked CI version leg while preserving the repository Python baseline.

- **(python): Bundle the required Microsoft C++ runtime in Windows wheels.** (GH#2368)
  Validate transitive and delayed PE imports without relying on the build host's installed runtime.
  Missing or unsupported Visual Studio installations name the required 2022 edition before resolving runtime paths.

- **(ocr): Invert backend auto-rotation before PDF upright correction in each geometry frame.** (#2356)
  Keep word, table, formula and public OCR boxes aligned with the final processed-raster dimensions.
  Publish normalized representative-page metadata while retaining its original orientation provenance.
  The new metadata key `ocr_backend_rotation_undone: true` marks a page whose returned geometry is already upright.

- **(bindings): Preserve shared exception families and declare Kotlin JNI resource dependencies.** (GH#2366)
  Java, C#, and PHP exceptions expose shared family interfaces; Python and Ruby `IoError` remains catchable
  as both `LoadError` and `XbergError`. Adopt Alef 0.115.14 across local development and CI, link Swift
  OpenDirectory on macOS, and refresh the generated Maven and Gradle tooling defaults.

- **(e2e): Keep the generated Elixir mock-server helper current with Alef 0.115.9.**

- **(swift): The Swift package requires macOS 15.**
  The macOS library builds for the declared floor instead of 11.0; the bundled ONNX Runtime needs macOS 13.4.

- **(tests): Check unsupported ZIP member compression when the lazy reader reads.**
  Retain healthy-member extraction and warning assertions with ZIP 9.2's deferred decoder initialization.

- **(dependencies): Replace the Candle fork with registry 0.11.1.** (#2333)
  Workspace, Elixir, Ruby and Rust E2E locks use the released ZIP 9 dependency chain.

- **(ocr): Honor cancellation after sparse OCR retries complete.** (#2345)
  Return cancellation before selecting retry text or retaining primary text after a retry error.

- **(benchmarks): Bind ParseBench runs to their Python harness sources.** (#2332)
  Reject changed or unbound runner code before extraction; preserve older plans and results for read-only audits.

- **(ci): Validate benchmark checkouts against the immutable workflow revision.** (#2344)
  Keep shell contract tests aligned with main-only benchmark dispatch and reject the former branch override.

- **(ocr): Declare the image table stitching fixture's known header metadata.** (#2354)
  Keep its header-retention assertion aligned with the explicit-header contract without promoting body rows.

- **(ci): Keep only the latest main run for each CI workflow.** (#2346)
  Cancel superseded runs through stable workflow-ref groups, including GPU runs and docs deployments, instead of retaining an obsolete-commit backlog.

- **(ocr): Map scaled, rotated PDF OCR geometry through its processed image frame.** (#2332)
  Preserve preprocessing metadata while aligning word, table and formula coordinates with the page raster.
  Keep the backend-free PDF OCR pipeline compilable and cover it in the narrow feature matrix.

- **(pdf): Preserve native table header roles through grid conversion.** (#2332)
  Source row and cell flags survive merged-cell placement and HTML rendering, including tables without spans.

- **(pdf): Keep prose-flanked multiplication dots in their source sentence.** (#2275)
  Inline symbols retain ordinary formatting while leading bullets and footnotes keep their roles.

- **(benchmarks): Distinguish ParseBench renderer warnings from model failures.** (#2332)
  Retain renderer diagnostics while rejecting genuine model and table-recognition fallbacks.

- **(pdf): Retry detached pages from bounded embedded JPEG/JP2 bytes after raster OCR fails.** (#2339)
  Recovery preserves page numbers, cancellation and original source image identities.

- **(ci): Gate formula test helpers with the features required by their callers.**

- **(pdf): A photo page keeps its text layer when OCR reads only noise.**
  When the OCR text of a page gets the recognition-noise warning and the text layer is readable text, the page content holds the layer first and the OCR text after it. A title in the layer is no longer lost. Pages with a clean OCR result, forced OCR and `discard_suspected_ocr_noise` behave as before. (GH#2311)

- **(models): Reject ambiguous or incomplete pinned model inventories before downloading.** (#2336)
  Duplicate paths, unsafe portable filenames, empty supplied manifests and unpinned external weights fail validation.

- **(pdf): Keep superscripts and formatting on their source characters after text repair.** (#2275)
  Annotation ranges follow ligature expansion, absorbed spaces and Unicode normalization.
- **(tables): Retain OCR table headers, spans and cell geometry in image and PDF output.** (#2332)
  PDF table elements and retained cells use the same final page coordinates.
- **(tables): Preserve native TATR headers, merged cells and cell coordinates through PDF assembly.** (#2332)
  HTML retains row and column spans, and stale or ambiguous table geometry is discarded.
- **(pdf): Retain each text run’s baseline when cached form objects reuse source span IDs.**
  Raw script geometry no longer inherits a different run’s baseline after sorting.

- **(pdf): Preserve separate values during leftward reading-order jumps.**
  Adjacent values such as `5.55` and `16` no longer fuse when their baselines differ slightly.

- **(tables): Preserve declared headers across PDF table fragments.**
  HTML and document structure distinguish physical header rows from labels inherited from another fragment,
  while keeping multiline headers and existing table anchors. (#2332)

- **(pdf): Render recognized formulas missing from OCR body elements.**
  Preserve page order, existing list ownership and element references; retain math in metadata with a warning
  when no safe same-page insertion boundary exists. (#2331)

- **(ci): Install governance tools through the locked mise toolchain.**
  Generation and validation now use the reviewed AI-Rulez and Python versions.
- **(rendering): Honor table header metadata in HTML and preserve headerless data rows.** (#2332)
- **(development): Keep generated skill summaries within Claude and Codex context budgets.**
- **(ci): Refresh governance server tracking for ai-rulez 5.9.3 validation.**
- **(vtt): Reject overflowing timestamps and decode character references in speaker annotations.** (#2330, #2329)
- **(bindings): Keep Rust-only structured rendering and recovery APIs outside generated language bindings.**
- **(ci): Keep Android x86_64 checks compiling when the remote compiler-cache backend is unavailable.**
- **(excel): Enforce XLSX text limits before cell materialization and apply workbook limits to file inputs.** (#2321)
  Count repeated shared strings against the complete workbook’s text budget.
- **(excel): Retain missing-worksheet warnings alongside readable sheet content after dependency updates.**
- **(djot): Keep nested table captions inside footnotes without duplicating them.**
- **(build): Retry Windows tool installation until the requested versions are verified.**
- **(development): Restore local Node and Java binding builds used by integration CI.**
  Integration jobs build and stage the checked-out bindings before testing their dependent packages.
- **(vtt): Skip WebVTT header metadata while preserving the first timed cue.** (#2320)
- **(development): Generate the MCP reference skill without shadowing Claude’s built-in `/mcp` command.**
- **(packaging): Include canonical licensing notices in Docker images and custom native release archives.**
  Dart and Swift packaging checks use the checked-out release inputs.

- **(mobile): Restore default Dart and Swift builds for iOS devices and simulators.**
- **(doc): Retain inherited superscript and subscript character styles while honoring direct formatting.** (#2275)
- **(pdf): Preserve superscript and subscript annotations in detected tables, including rotated text.** (#2275)
- **(packaging): Reject failed macOS dependency inspections and support Node vendoring with Bash 3.2.**

- **(pdf): Cache Type 3 font-mapping provenance during text extraction.**
  Repeated glyphs retain their fallback markers without rescanning the font for every character. (#2307)
- **(doc): Preserve direct and piece-level superscript and subscript formatting.**
  Compact and complex Word piece properties retain their logical character positions, including reused source bytes.
  Footnotes and comments keep annotations in Markdown, Djot and HTML through decoding and whitespace normalization. (#2275)

- **(pptx): Preserve slide headings and table grids in plain document structures.**
  Tables retain literal pipe characters and remain available in extraction results. (#2302)
- **(pdf): Recover word spaces when subset fonts omit a space-glyph width.**
  Explicit spacing, fixed-pitch fonts and standard font metrics remain authoritative. (#2303)
- **(pdf): Classify repeated running headers and honor header/footer filters across renderers.**
  Retained furniture keeps its semantic layer without exposing internal rendering markers. (#2304)
- **(pdf): Retain up to six inferred heading font tiers without flattening semantic subsections.**
  Font-tier lookups stay bounded for documents containing many distinct sizes. (#2305)
- **(ocr): Reuse secondary dictionaries within keyed Tesseract sessions.**
  Repeated multilingual calls avoid reloading dictionaries while preserving quality checks and pool eviction. (#2306)


- **(publish): Inspect the actual Node native binary before packing platform packages.**
  File-size failures now stop publication instead of being hidden by the diagnostic output.
- **(office): Recover native text from embedded EMF and WMF pictures without raster OCR failures.**
  Preserve original image bytes, follow selected fonts, and report unsupported font encodings explicitly. (#2297)
- **(ocr): Prevent crashes during repeated Tesseract orientation and script detection.**
  Script-name results remain valid across repeated calls and detector cleanup. (#2300)

- **(pptx): Remove floating-point storage noise from numeric chart values.**
  Cached and literal numbers use Office's 15 significant digits; series names and category labels stay as written. (#2298)

- **(docx): Keep the number Word paints on each numbered list item.**
  Labels such as `Q4.`, `(b)` and `3.2` are computed from `word/numbering.xml` as Word counts them, with start
  values, level and start overrides, level restarts, lists that share a definition, and letter and roman
  formats, and reach `list_marker` and the Markdown. A plain `1.`, `2.` list is written as before. (#2293)

- **(doc): Preserve superscript and subscript formatting inside legacy Word table cells.**
  Plain cell text remains unchanged; Markdown and HTML retain script formatting across cell paragraphs. (#2275)

- **(csv): Enforce table-cell limits against padded dimensions before retaining rows.** (#2296)
- **(pdf): Preserve prose superscripts and subscripts when table detection is enabled.** (#2275)
- **(docx): Preserve equations as math in table cells and table Markdown without changing public structs.** (#2256)
- **(html): Preserve YAML metadata blocks when normalizing Markdown output.** (#2295)
- **(llm): Forward configured provider options to hosted embedding and reranking requests.**
- **(pdf): Read two-up pages one half at a time.** (#2227)
- **(pdf): Preserve native regions unless VLM answers retain their content; report table-model load failures.** (#2259, #2269)
- **(djot): Preserve smart punctuation, table captions, line breaks and image alt text.** (#2254)
- **(djot): Drop caption footnote references when an empty table is omitted.** (#2248)
- **(docx, odt): Preserve equation placement, limits, function names and surrounding text.** (#2258)
- **(docx): Omit blank text paragraphs and list items beside display equations.** (#2262)
- **(build): Standalone XML and HWPX features pass strict Clippy checks.** (#2243)
- **(email): Keep HTML head metadata and scripts out of message text, and recover malformed base64 parts.**
  Body text and attachments decode stray characters without decoding nested messages twice. (#2252, #2253)

- **(plugin): Restore all seven Pi skills advertised by the root package.**
  Resource paths match the packaged skills and prompts, with checks that reject missing or mismatched entries. (#2257, #2246)

- **(ocr): Initialize Sceptre OCR and document orientation in either order.**
  Xberg and Sceptre 0.8.2 share one ONNX Runtime environment, preventing duplicate-environment failures
  during sequential or concurrent extraction.

- **(djot): Keep neighbouring blocks in footnote definitions separated.**
  Paragraphs, headings, list items, definition terms and table captions no longer run together. (#2270)

- **(extraction): Warn when image or PDF OCR table recognition cannot load its model.**
  Results carry one `image_tables` or `pdf_tables` warning when layout detection found table regions.
  Mixed PDFs retain one warning when native and OCR pages encounter the same missing model. (#2244)

- **(doc): Preserve direct superscript and subscript formatting in legacy Word prose Markdown.**
  Paragraphs, headings and lists keep `10^9^` and `H~2~O` with `escape_markdown: false`.
  Supplementary characters remain intact, and footnotes, headers and comments keep their story boundaries. (#2275)

- **(jupyter): Preserve wide characters and combining marks during terminal-style output overwrites.**
  Carriage returns follow display columns, combining marks stay with their base, and 8-bit CSI sequences are removed.
  Grid-path regressions also verify that combining marks do not leak across lines. (#2245, #2288)

- **(djot): Keep line breaks inside footnote definitions and image alt text.**
  Soft breaks become spaces and hard breaks remain newlines without leaking image-alt breaks into footnote text. (#2247)

- **(html): Preserve superscript and subscript markers in Markdown when HTML options are unset.**
  Default conversion keeps `10^9^` and `H~2~O`; explicitly supplied conversion options retain their script symbols. (#2275)

- **(structured): Rust callers can select image rasterization security limits for trusted large photos.**
  `pages_for_call_with_security_limits` and `ParsedDocument::from_bytes_with_security_limits` retain caller budgets
  through image decoding, PNG encoding and per-operation render caches; existing APIs keep their default limits. (#2283)
- **(pdf): Preserve superscript exponents and subscripts in native PDF prose Markdown.**
  Explicit text rise and positioned scripts retain inline annotations; confirmed numeric footnotes remain separated. (#2275)

- **(docx): Preserve superscript and subscript markers in table cells and note definitions.**
  Markdown keeps `10^9^` and `H~2~O` with `escape_markdown: false`, including scripts beside bold, italic
  or linked text. (#2275)
- **(images): Preserve cell-only OCR tables without duplicating neighboring tables.**
  Extracted tables and page content keep the same cells, order, and metadata. (#2226)
- **(doc): Extract floating PNG and JPEG pictures from Word 97–2003 documents.**
  A picture positioned over the text was skipped with a warning, so its text never reached embedded OCR.
  Floating pictures in the main document are now extracted at their anchor, so their OCR text keeps source
  order. A picture with no image data now says so instead of reporting an unsupported format. (#2273)
- **(html): Read a page that is not UTF-8 in the encoding its byte order mark or `<meta>` declares.**
  Every non-UTF-8 page was decoded as UTF-8, so a `<meta charset>` or `<meta http-equiv>` declaration was ignored
  and each non-ASCII character of a windows-1252, Latin-1 or Shift_JIS page became U+FFFD, with a lossy-decode
  warning. A page without a declaration is still decoded and flagged as before.
  Meta declarations respect quoted attributes, the content-type pragma, and the first 1,024 bytes of the page. (#2220)
- **(markdown): Keep bracketed link labels and image alt text in their original sentence position in plain output.**
  Link and image destinations remain available as URI metadata, and embedded image bytes are retained.
  Empty image labels no longer leave a doubled word gap. (#2233, #2235)

- **(mathml): Write `^` and `~` from math text as `\hat{}` and `\sim`.**
  A `^` or `~` in element text reached the LaTeX raw, where TeX reads them as a superscript and a space, so
  `<msup><mi>x</mi><mo>^</mo></msup>` gave `x^{^}`, which does not parse. They are now escaped where math text is
  written, which covers DOCX equations too, since they share that escape.
- **(mathml): Drop invisible operators from the LaTeX.**
  Function application, invisible times, separator and plus (U+2061 to U+2064) were copied into the LaTeX, which TeX
  engines reject. They are now spaces in math and dropped from `mtext` and `ms`, and an `mfenced` that holds one groups
  its children without separators. `mfenced` open and close attributes are trimmed, and private-use characters are
  stripped from direct text as they already were from element text.
- **(mathml): Convert `mmultiscripts`.**
  The element had no conversion, so its scripts ran into the base and a carbon-14 isotope read `C614`. Pre- and
  post-scripts are now written as `{}_{6}^{14}C` and `R_{i}{}^{j}`.
- **(mathml): Write limits on big operators and limit-taking names as `_{...}^{...}`.**
  `munder`, `mover` and `munderover` always stacked, so a sum read `\overset{n}{\underset{i=1}{\sum }}` and `lim` read
  `\underset{n}{lim}`. They now read `\sum_{i=1}^{n}` and `\lim_{n}`, integrals take `\limits`, and an operator with
  `movablelimits="false"` keeps its limits stacked. `munderover` now applies accent scripts as `munder` and `mover`
  already did, an `accent` or `accentunder` of `false` (on the element or its script's `mo`) keeps a script a visible
  script, a `semantics` wrapper no longer hides `movablelimits="false"`, and content MathML names such as `arg` and
  `coth` read `\arg` and `\coth`. An overbar or arrow accent no longer renders its base twice, which made nested
  accents cost 2^depth.
- **(pdf): A missing table structure model no longer drops the document's structure.**
  When layout detection found a table and the configured table model (TATR or SLANeXT) could not be loaded, the
  extraction fell back to plain text, without its layout-detected tables, headings or layout regions, and with no
  warning. The structure is now kept, table regions are read without the model, and a `pdf_tables` processing
  warning names it.
- **(ocr): `ocr.backend = "vlm"` works on mixed PDFs with layout detection on.**
  Every page the mixed route OCR'd failed with "VLM OCR requires vlm_config to be set", because the one-stage pipeline
  built for layout dropped the OCR config's `vlm_config`.
- **(pdf): Crop VLM regions from the detected pixel box.**
  The crop was cut from the layout hint's PDF-point box read as pixels, so at the default 150 dpi the region came out
  at about half its size and shifted down the page: one at the top of a Letter page was cut from below the middle. It
  is now cut from the pixel-space layout detection the hint was converted from.
- **(pdf): Give a VLM region on a page with no text layer its own page break.**
  A region answer for a scanned page with no elements was spliced next to a neighbouring page's content with no
  break between them, so page markers placed it, or the content after it, on the wrong page.
- **(pdf): Recognise formulas on scanned pages that Tesseract OCRs page by page.**
  In builds with PaddleOCR, a default Tesseract config with `layout.formula_model` set took the per-page pipeline
  branch, which returned before formula recognition, so scanned pages produced no LaTeX.
- **(layout): Run markdown layout detection on the page's CropBox only.**
  Pages went to the layout model at their full MediaBox, so bleed, trim and registration marks outside a smaller
  CropBox shrank the page the model saw. Inference now runs on the CropBox and the detections are shifted back into
  the page raster. Rotated pages and the OCR layout pass are unchanged.
- **(pdf): Formula and VLM region recognition keep their rasters when the layout gate skips a page.**
  With the auto layout strategy, a page the gate skipped made the markdown layout rasters unusable for OCR, and they
  were dropped for the whole document, so formula recognition and VLM regions did nothing on the pages that did run
  layout. The native path now keeps them when a formula model or VLM regions are configured; a document with any
  rotated page still drops them.
- **(toml): Print a date or time as its TOML text instead of under a `$__toml_private_datetime` key.**
  The TOML value went through `serde_json::to_value`, which turned every datetime into
  `{"$__toml_private_datetime": "2025-02-05"}`, so the text read `created_date` and then
  `$__toml_private_datetime: 2025-02-05`. It now reads `created_date: 2025-02-05`.
- **(images): Render legacy OCR Markdown tables once when the backend also returns structured tables.**
  Standalone image pipelines preserve the surrounding prose and structured OCR documents.
- **(latex): Attach `\ref` and `\cite` relationships to the paragraph that holds them.**
  Each reference was pushed as an extra paragraph of its own, `[key]`, so the text showed `[sec:a]` and `[knuth]` as
  separate paragraphs ahead of the sentence that cites them. The relationships now start at that sentence's paragraph
  (or display formula), or at the footnote that holds the reference, as the reStructuredText and Org extractors
  attach theirs.
- **(latex): Print accent commands, letter commands and control spaces as the characters they stand for.**
  `Schr\"odinger`, `\'Ecole` and `Garc\'\i a` kept their backslash, with `\'` turned into a curly quote, and `\ss` and
  `\o` were dropped, in paragraphs, headings and list items alike. They now read Schrödinger, École, García, ß and ø,
  a group such as `Schr{\"o}dinger` prints without its braces, and `Mr.\ Smith` reads `Mr. Smith`.
- **(archive): Extract the other members of a ZIP archive that holds an encrypted member.**
  The ZIP reader refuses to open a member that needs a password, and that error failed the whole archive. The
  encrypted member is now listed, skipped and named in the archive warning, like a member that fails its CRC check.
- **(markup): Read Markdown, Djot, MDX, Org, reStructuredText, Typst, RTF, DocTags and Jupyter documents that start with a UTF-8 byte order mark.**
  The mark stayed in front of the first line, so a leading heading came out as a paragraph with its `#`, `*` or `=`
  marker, the mark leaked into the text, and a notebook failed with a JSON parse error. It is now skipped, as the
  plain text, CSV and AsciiDoc extractors already did.
- **(jupyter): Drop ANSI escape codes from the plain text of `execute_result` and `display_data` outputs.**
  A kernel that colours its `text/plain`, such as IJulia for a `DataFrame`, left `\x1b[1m` and `\x1b[0m` in the
  text. These outputs now get the same terminal handling as stream and error outputs, so a carriage return
  overwrites the line as well: a progress bar written to `text/plain` keeps only its last state, and CRLF becomes LF.
- **(url): Standalone Rust binding and end-to-end graphs resolve browser ingestion consistently.**
  Their libc selections match Crawlberg 1.11.1's published iOS constraint.
- **(excel): Round float cells to Excel's 15 significant digits when the result stays finite.**
  A formula result carried the binary noise of its double into the output, so `=0.1+0.2` read `0.30000000000000004`
  and `=100*1.19` read `118.99999999999999` where Excel shows `0.3` and `119`.
  Finite boundary values retain their original representation when rounding would overflow.
- **(structured): Parse JSON, JSON Lines, YAML and GeoJSON documents that start with a UTF-8 byte order mark.**
  `serde_json` and the YAML parser rejected the mark, so these documents fell back to plain text with a
  `parse-fallback` warning. The mark is now skipped before parsing; TOML already accepted it.
- **(doc): Extract inline PNG and JPEG pictures from Word 97–2003 documents.**
  Embedded OCR reads their text in source order, including image-only documents. Unsupported picture locations
  and formats report processing warnings. (#2210)
- **(html): Preserve word separators carried by whitespace-only formatted links.**
  A blank formatted link label no longer joins the words on either side in Markdown output.
- **(mime): Standard LaTeX, shell script and Apple iWork MIME aliases are accepted.**
  Newly supported aliases resolve to their existing canonical formats. Shell aliases extract as plain text
  when `tree-sitter` is disabled; existing aliases keep their spelling and feature requirements. (#2205, #2208)
- **(mime): Generic octet-stream declarations still trigger format detection with parameters or different casing.**
  `application/octet-stream; charset=utf-8` and surrounding spaces no longer prevent detection
  from names or bytes, including direct bytes extraction and the Tower service. (#2206, #2207)
- **(downloads): Percent-encoded proxy credentials are decoded correctly.**
  Model and tessdata downloads preserve literal percent signs and colons in proxy passwords.
  Encode literal percent signs in proxy URLs as `%25`.
- **(asciidoc): Cross references, passthroughs, escapes and hard line breaks read as Asciidoctor renders them.**
  `<<install,the guide>>`, `xref:faq[the FAQ]` and `pass:[...]` stayed as raw macros, `\*not bold*` kept its backslash, `\{name}` was still substituted, and a line ending in a space followed by `+` kept the `+`; each now reads as its text, and `+` ends the line. Macro names written inside reference labels or passthrough text stay literal.
- **(doc): Legacy Word tables retain their cell and row structure.**
  Table cells no longer run into adjacent paragraphs. Configured table-cell limits also bound padded table output,
  and styles inside table cells no longer suppress headings in the surrounding document.
- **(vtt): Numeric character references and `&quot;`/`&apos;` in cue text are decoded, and ruby text is left out of the line.**
  Signed references remain literal, zero-padded references decode correctly, and `<rt>` text outside ruby remains visible.
  Only the six named escapes were decoded, so `It&#39;s` and `&quot;fine&quot;` reached the text as written; references are now decoded in one pass, and other named ones stay as written. The `<rt>` reading printed over a ruby base was glued into the line (`漢kan字ji`); it is now dropped, also when `</ruby>` closes it.
- **(rst): Internal underscores in Unicode words remain intact when resolving named references.**
- **(rendering): Markdown output ends raw HTML and JSX blocks before the next block.**
  A raw block was written with no line break, so an HTML block's lines ran together and the block after it was glued on (`</div># Heading`, `<Badge />- item`), which turned the heading or list into raw text; consecutive raw blocks now form one run, one line each, followed by a blank line. Source blank lines between raw fragments remain intact.
- **(djot): The paragraphs of a list item are separated by a space.**
  A list item's second paragraph was appended to its first with nothing in between, so `two` followed by `continued paragraph` read `twocontinued paragraph`; the paragraphs are now joined with a space, as in the Markdown extractor.
- **(docbook): An admonition's body is the admonition's text.**
  `<note>`, `<warning>`, `<tip>`, `<caution>` and `<important>` pushed an admonition whose text was its kind and then the body as a separate paragraph, so Markdown showed `> [!NOTE]` / `> note` with the body outside the callout; the body is now the admonition's text, as in the AsciiDoc, RST and Markdown extractors.
- **(build): The `xml` feature compiles without `svg` or `pdf`.**
  The flat ODF drawing recovery in the XML extractor calls `extraction::diagram::odf`, but the `diagram` module was only built with `svg` and `xml` together or with `pdf`, so `--no-default-features --features xml` (and `office,xml`) failed with `cannot find diagram in extraction`.
- **(fictionbook): A note reference no longer splits the paragraph it sits in, and a note's title stays out of its definition.**
  The text before each `<a type="note">` was pushed as a paragraph of its own, so `brown fox<a>1</a> jumps.` came out as `brown fox[^1]` and a separate `jumps.`; the paragraph is now whole and its references follow it, as in the Markdown extractor. A note's own `<title>` is left out of the footnote text when it repeats the reference's marker (`1`); other titles, such as a poem's inside the note, stay.
- **(jats): Inline markup stays inside its word, and table rows outside a row group are read.**
  Every text run was joined with a space, so `CO<sub>2</sub>` read `CO 2` and `uptake<xref>1</xref>.` read `uptake 1 .`; text now joins as written, with layout whitespace collapsed. A `<tr>` directly in `<table>` or in `<tfoot>` was skipped, so such a table came out empty apart from its caption.
- **(rst): Named hyperlink references and `replace::` substitutions read as docutils renders them.**
  `` `the docs`_ `` became inline code, `Python_` kept its underscore and `|project|` stayed literal; a reference now reads as its text, linked to the `.. _name: url` target of that name, and a substitution is replaced by its definition wherever that appears.
- **(typst): Comments are left out of the text, and an escape reads as the character it stands for.**
  Strings, raw literals and escaped hash characters retain their code or markup boundaries.
  `// ...` and `/* ... */` comments reached the text (a block comment's `*` even became bold), and `\#`, `\*` or `\u{1F600}` kept their backslash; a comment line also no longer ends the paragraph around it.
- **(jupyter): Stream and error outputs read as the notebook shows them, without ANSI escape codes.**
  IPython colours every traceback line, so `\u001b[0;31mZeroDivisionError\u001b[0m` reached the text (as `[0;31m...[0m` in Markdown), and a progress bar redrawn with `\r` kept every intermediate state run together; escape sequences are now dropped and a carriage return overwrites the line, as in Jupyter.
- **(rst): Line blocks keep their line breaks, and the paragraph before a literal block keeps its colon.**
  Line blocks also retain their footnote and cross-reference relationships.
  `| Line one` / `| Line two` ran together as `| Line one | Line two`, and `Example::` read `Example`; a line block is now one line per `|` line, and `Text::` reads `Text:` while `Text ::` reads `Text`, as in docutils.
- **(latex): Source lines wrapped at a fixed width form one paragraph, and `~` reads as a no-break space.**
  Every source line was its own paragraph, so a wrapped sentence was split apart and a `\footnote{...}` spanning two lines was not parsed; paragraphs now end at a blank line or a block-level line, comments follow LaTeX's rules (but not inside `\verb|...|`, whose contents stay literal), paragraphs stop at `\bye` and `\end{document}`, and `Dr.~Smith` no longer keeps the tilde.
- **(orgmode): Comment lines and `#+BEGIN_COMMENT` blocks are left out of the text.**
  A `# ...` line was joined into the paragraph around it and a comment block's content was emitted, although Org never exports either.
- **(swift): Package release builds avoid conflicting dynamic-lookup linker flags.**
- **(rendering): Plain-text output writes the footnotes of LaTeX, Markdown, reST, Org, Typst, Djot, DocBook, FictionBook, EPUB and PPT documents.**
  Their definitions were left on the body layer, so plain output kept the `[^1]` marker and dropped the note; definitions now always sit on the footnote layer, JSON output appends them after the body as it does for DOCX and ODT, and styled HTML renders footnote-layer definitions, which it previously dropped for DOCX and ODT as well.
- **(html): Inline MathML keeps paragraph placement, and bold table text preserves literal asterisks.**
  Break-only presentation tables are omitted, and links in preformatted content retain their destinations.
- **(odt): Spaces written as `text:s` and text after a nested span, space or tab are no longer dropped.**
  `a<text:s text:c="3"/>b` reads `a   b`, and list items and table cells keep a styled link's or span's full text.
- **(rtf): Non-breaking spaces and hyphens (`\~`, `\_`) read as those characters, and escaped braces and backslashes stay in their table cell.**
  `5\~kg` no longer reads `5~kg`, and `set \{x\} here` in a cell no longer loses its braces to the body text.
- **(doc): Fields spanning paragraph boundaries retain their displayed text without leaking instructions.** (#2184)
- **(ppt): Speaker notes are referenced from their own slides in rendered Markdown.** (#2186)
- **(ios): Native libraries link with bundled ONNX Runtime’s CoreML backend.**
  Device and simulator builds include the matching Apple compiler availability runtime.
- **(html): Extract whitespace-only formatted paragraphs and headings without panicking.**
  Blank lines in Word-exported HTML no longer abort Markdown extraction; surrounding paragraphs are preserved. (#2182)
- **(markdown): Keep the text of a list item before the code blocks, tables, headings and formulas inside it.**
  The item's text was emitted at the end of the item, after any such block it holds, so an install step such as
  `1. Install:` followed by a fenced code block came out with the code first.
- **(markdown): Separate paragraphs inside a list item retain paragraph breaks and annotation offsets.** (#2181)
- **(code): Graphviz `.dot` files and Graphviz/PlantUML MIME aliases now extract as source code.**
  Binary Word templates retain Word extraction. (#2180)
- **(ocr): Tesseract now reads display-rotated PDF pages upright regardless of `auto_rotate`.** A `/Rotate`d page was sent to Tesseract sideways, so a landscape bid table came back as garbage (page confidence 0.35) unless `auto_rotate` was on; it is now rotated upright first and the recognised boxes are mapped back to the page (0.87 on the same page). Cached results for such pages are not reused. (#2172)
- **(pdf): With layout detection, item numbers are no longer deleted because their digits occur inside other table cells.** A table region suppressed any text whose digits appeared anywhere in its concatenated cells, so item numbers such as `0001` vanished from a bid schedule when a table model missed that column. Short runs now have to match a whole cell word; only long runs may span neighbouring cells of one row. (#2171)
- **(pdf): `extraction_confidence.text_coverage` counts a known empty page whether or not page output is requested.** A three-page PDF with an empty middle page scored `1.0` by default and `0.67` with `extract_pages`; both now report `0.67`. (#2170)
- **(pdf): A ruled table record whose cells wrap onto different numbers of lines is no longer split into one row per line.** A subcontractor disclosure entry (name, address, certification, work, amount) came back as five rows; bands where each line fills the same columns as the previous one are still subdivided. (#2169)
- **(pdf): Native extraction keeps an item schedule whose rows sit about two line heights apart as one table.** Rows at that pitch were split into single-row fragments and discarded, so a 25-item bid schedule produced no table although its text was extracted. Consecutive numeric rows with a repeating pitch and aligned outer edges are now rejoined; a detached total, a second table or double-spaced prose still stays separate. (#2168)
- **(pdf): `metadata.pages.boundaries` now index the final Markdown, HTML and Djot content instead of the earlier plain text.** Ranges are re-located in the rendered output; when that cannot be verified the ranges are omitted and a `page_boundaries` processing warning is added rather than returning wrong offsets. (#2166)
- **(pdf): Repeated prices in separate rows of rotated tables are no longer dropped as redrawn duplicates.** Duplicate detection now compares positions in the text's own reading frame. (#2164)
- **(pdf): Splitting multiline table rows preserves merged columns, keeping bidder names aligned with their IDs and prices.** (#2165)
- **(pdf): Native PDF page tables now reflect the final global table selection, order, IDs and columns.** (#2167)
- **(markdown): Text inside code spans, code blocks and math can no longer turn into raw HTML in Markdown output.**
  Math is now written as code math: `` $`...`$ `` inline, and a ```` ```math ```` block for display math standing
  alone. Math holding a backtick or `$`, or in a block with code holding `$`, is written as a plain code span, and
  a `$` in that block's prose as `\$`. An empty code span no longer panics the Markdown writer. HTML output is
  unchanged.
- **(api): `/health`, `/info` and `/version` no longer queue behind the concurrency limit.** `XBERG_MAX_CONCURRENT_REQUESTS` now applies to the extraction routes only, so a saturated but healthy pod keeps passing orchestrator probes. (#2161)
- **(pdf): disabling extraction caching also disables the built-in Tesseract result cache.**
  The opt-out reaches mixed, full-page, pipeline and embedded-image OCR without changing native-text fallback routing.
- **(ocr): An image with no resolution metadata is no longer upscaled by a fixed 4.2x.** The resample assumed 72 DPI,
  which made large text so big that Tesseract's default `eng` model misread it (`Cedar Analytics supplied 17 widgets`
  came back as `1/7`). The factor now follows the size of the text, up to the old one for small text, and an image
  that is not clean text (a photo or receipt) keeps the old behaviour. On 176 synthetic images it gained 27 of 2464
  tokens (numeric tokens 692 to 698) and on 48 rendered pages it was neutral (+3 of 16018). Output for such images
  can change; cached results are invalidated. (#2127)
- **(reranker): The default `balanced` reranker works on macOS again.** With the default acceleration, every call to
  the `balanced` preset (`ettin-reranker-150m`) failed inside CoreML ("Unable to compute the prediction using a neural
  network model"). That model now runs on the CPU when the provider is left on `auto`. An explicit `coreml` request is
  unchanged, and every other preset keeps CoreML. (#2159)
- **(ner): The default GLiNER model no longer reports an identifier as an email address.** `SYN-004` and `INV-2026-0042`
  came back as `email` entities. An `email` entity with no address cue in or near its span (an `@`, a bracketed `[at]`,
  or `at` with `dot`) is now dropped; spans that carry a cue, including fragments of an address, are kept. (#2133)
- **(ocr): Tesseract runs with several languages no longer drop lines written in a language other than the first.**
  With `rus+eng`, an English image came back empty, because the dictionary noise filter checked words against the first
  language's dictionary only. A word is now valid when any configured language accepts it. (#2158)
- **(rendering): A footnote reference stays on the block that cites it.** Markdown, Djot and LaTeX input placed it on
  the previous paragraph, list item or heading, and Djot output wrote it at the start of the next block (`[^1]# Heading`). (#2156)
- **(docx, odt): Footnote and endnote references are written once, numbered like their definitions.** Markdown, HTML
  and Djot output showed each DOCX or ODT reference twice, and the inline marker carried the source note id, so it could
  link a different note than its definition (an endnote 1 next to a footnote 1 linked the footnote). The marker now
  follows the paragraph that cites it, an ODT reference no longer follows the paragraph before it, and plain output
  writes the same `[^N]` markers and labels the matching definitions `[^N]:` for every format.
  A DOCX paragraph that holds only a note reference keeps it, a reference in a table cell is written after the table
  instead of as a bare source-id literal in the cell, and the per-page content numbers markers and definitions the
  same way, with a footnote and an endnote that share a Word id kept apart.
- **(docx): The prompt of an empty content control is no longer extracted as text.** Word fills an unfilled
  field with a prompt such as "Click or tap here to enter text." and marks it `w:showingPlcHdr`; forms and templates no
  longer repeat that prompt in `content`, in the body, in table cells or in text boxes.
- **(pdf): Explicit header and footer inclusion preserves geometric page numbers independently.** (#2150)
- **(pdf): Native extraction preserves separate two-column bordered tables beside wider tables on the same page.**
- **(ocr): Failed PDF OCR pages report the type chain of the underlying error.**
  The page warning, `OcrPageFailure.error`, and the run error name each error source without its message text. (#2149)
- **(ocr): Tesseract C API declarations match the native return types.**
  Fixes the WASM link failure and a crash in `process_pages`; borrowed text is no longer freed. (#2145)
- **(ocr): Table reconstruction keeps columns that sit close together.**
  A repeated column edge less than about one space width from its neighbour is no longer merged into the neighbouring cell. (#2140)
- **(images): TIFF pixel density (`XResolution`, `YResolution`, `ResolutionUnit`) is used as the source DPI instead of assuming 72.** (#2143)
- **(ocr): Preserve ordered text and explicit HTML raw blocks returned by custom OCR backends.**
  Single-backend and pipeline image/PDF handoffs retain flat structured contributions, source-image references
  and PDF box coordinates. Unsupported nested nodes and backend asset/derived-output payloads fail explicitly
  instead of being flattened or discarded.
- **(ocr): The standalone image OCR pipeline keeps the tables it returns.**
  Tables embedded as GFM in a backend's text are rendered once. (#2144)
- **(ocr): PaddleOCR, layout detection, and GLiNER share one ONNX Runtime environment.**
  The second component to initialize no longer fails with "only one environment is allowed per process". (#2148)
- **(csharp): macOS native-package builds avoid duplicate loader paths.**
  Generated native libraries retain their sibling-library search path. (#2147)
- **(images): Images without EXIF metadata no longer report a metadata extraction warning.** (#2141)
- **(docx): Text moved with Track Changes on appears once, at the place it was moved to.**
  The copy Word keeps at the old place (`w:moveFrom`) is no longer extracted as content. (#2151)
- **(llm): Failed structured output retains available model, token, and finish-reason metadata in `llm_usage`.**
  Extracted text and failure warnings remain available; JSON parse errors omit response content. (#2139)
- **(pptx): Speaker notes follow each slide’s relationships instead of matching slide filenames.**
  Sparse or independently numbered notes remain attached to the correct slide. (#2138)
- **(csv): Escape `|` and line breaks in a CSV table's Markdown.**
  `Table::markdown` for CSV and TSV wrote each cell as it was, so a `|` added a column to its row and an embedded
  newline split the row. Cells are now escaped as every other table renders them (`\|`, `<br>`), and the
  `csv` warning about the unescaped characters is gone.
- **(docx): Table rows that start or end inside the table grid keep their cells under the right columns.**
  `w:gridBefore` and `w:gridAfter` were ignored, so such a row's cells moved left. (#2153)
- **(packaging): GNU cross-builds use a pinned linker wrapper compatible with Rust 1.99 warnings.**
- **(swift): iOS simulator release builds omit macOS extension-linker flags.**
- **(docx): Lists numbered in non-Latin formats stay numbered.** A Chinese 一、二、三 (`chineseCounting`),
  an enclosed ①②③, an ordinal or a Cyrillic list was read as a bullet list. (#2154)
- **(swift): Linux library builds use a pinned cross-compiler compatible with strict Rust warning checks.**
- **(pptx): Tables, charts and SmartArt keep their place in a slide's reading order.** Their position, stored in
  `p:xfrm`, was read as the top-left corner, so they came out before the text above them. (#2155)
- **(llm): Strict structured extraction validates provider output against the original JSON schema.**
  Invalid output leaves extracted text intact and reports a safe warning; schema references resolve offline. (#2136)
- **(ocr): Tesseract honors the requested OCR engine mode, including when reusing pooled engines.**
  Modes unsupported by the installed language data report an error instead of silently using automatic mode. (#2135)
- **(pdf): Consecutive-paragraph cleanup preserves repeated body text at distinct or unknown positions.**
  Headings, lists, code, formulas, captions, and differences in capitalization are preserved; rendering duplicates
  are removed only when their text and valid bounding boxes coincide.
- **(docx): A carriage return (`w:cr`) breaks the line and a position tab (`w:ptab`) separates the words.**
  Both were dropped, so the words around them ran together, in text boxes too. Tab stop definitions in a text
  box no longer add a tab before its text. (#2152)

- **(ocr): Let a confident OCR read replace an invisible text layer that shares almost no words with the page picture.** (#2425)
  A layer painted only in render mode 3 over a page-covering scan is replaced when the read repeats at most 15% of its words and passes the confidence check; visible text and layers that match the picture keep their text. The hidden-layer probe now also audits a text stream and a picture stream that are both Flate-compressed, in either order, and decodes a 300 dpi CCITT Group 4 picture with `/Decode [0 1]` and no `/Rows` or `/BlackIs1`. A layer that covers every ink band is now read too, unless the language check judged it plausible prose. Its CCITT preview budget is 16 work units per `max_iterations`.

- **(markdown): Display math no longer opens an HTML block.** (GH#2793)
  A line of display math that starts with `<` (`<script`, `<div`, `<!--` and others) now gets a space after the `<`, as a `<` before a closing `>` already did, so no Markdown reader starts an HTML block there and drops the math.

- **(ocr): Keep every OCR line that holds a numeral, and keep page ranges inside the content after Unicode normalization.** (GH#2851)
  The dictionary filter for noisy OCR lines no longer removes a line that holds a numeral, so its numbers stay in `content` and in the page text. Lines of letters only follow the filter as before. Page byte ranges in `metadata.pages.boundaries` now follow `content` when Unicode normalization shortens it; a range that cannot be mapped is omitted with a `page_boundaries` warning.
- **(pdf): Words that a producer places one by one no longer run together.** (GH#2675)
  A PDF that sets the text matrix before each word, often with a repeated font operator, kept the gap to the next word inside the previous word's width, so the words read as one. A run that a text matrix extends past its last glyph now ends at that glyph, unless a word-gap offset in the next text array marks the gap. Pages with glyph-by-glyph text now measure a word gap against the font size of the word after it, not against the largest size on the line. Subscripts and raised indices stay on their symbol, and a gap next to a CJK character (Han, kana or Hangul), CJK punctuation or a fullwidth form is measured as before.
- **(pdf): A word that a later show inside an `ActualText` scope widens no longer ends before that show.** (GH#2875)
  When a producer sets the text matrix before each word, the extractor records the run's width before the matrix move, so a font operator that follows ends the word at its last glyph. A show that comes after the move, such as a second show inside an `ActualText` scope or a glyph the font gives no text, now ends that record, so the word reaches the pen again and its width covers the later glyphs, as it did before the record existed.
- **(pdf): Preserve source detail for configured Tesseract preprocessing.** Refs #2711 (partial).
  Forced whole-document OCR with the registered built-in Tesseract backend, including the synthesized classical pipeline, renders native page content at a preprocessing target above 150 DPI within the existing budget. Any preprocessing block requests its target, including the default 300 DPI in a denoise-only block. This recovers a reproduced small-text word boundary; explicit image settings and original full-page scan density retain precedence. Without a preprocessing block the default remains 150 DPI. Selected pages, prepared rasters, custom backends, explicit pipelines and VLM fallback keep their source-render policy. The general 150-DPI glyph-shift issue remains open.

## [1.3.11] - 2026-10-16

### Added

- **(math): explicit text annotations preserve LaTeX in Markdown, HTML, and Djot.**
  Rust callers can use `AnnotationKind::math(display)` and `math_display()`; other bindings can use the existing
  `Custom` representation with name `xberg.math` and value `inline` or `display`. Annotation byte ranges include
  the equation's `$...$` or `$$...$$` delimiters; unannotated dollar signs remain prose.

### Fixed

- **(pdf): Native extraction preserves minus signs from unmapped math-axis bar glyphs in explicitly symbolic
  Type1 fonts.** (#2130)
- **(tracing, ocr): Parallel PDF OCR keeps its caller span context across task boundaries.** (#2131)
- **(wordperfect): Plain output retains footnote and endnote bodies.** (#2129)
- **(pdf, ocr): Page boundaries match complete page text in final Plain output, including UTF-8 and page separators.** (#2128)
- **(wasm): OCR-enabled WebAssembly builds link successfully with Rust 1.99.**
- **(wasm, url): URL extraction no longer decompresses a response twice after Fetch has decoded gzip, deflate, or Brotli.**
- **(docx): inline equations stay in their original sentences and remain in the extracted formula inventory.** (#2123)
- **(docx, pptx): an equation character written as a character reference is converted.** OMML attribute values
  were read without unescaping, so `m:chr m:val="&#x23DF;"` kept the reference text and printed it in place of the
  brace. Character and entity references in these values now resolve to their characters.
- **(docx, pptx): an underbrace or overbrace in an equation is no longer dropped.** `m:groupChr` was skipped, so
  the brace and the text inside it were missing from the output. A brace now renders as `\underbrace` or
  `\overbrace` by its position, and any other character is set under or over its body.
- **(docx, pptx): an n-th root in an equation keeps its degree.** A radical without `m:degHide` was read as a
  square root, so a cube root came out as `\sqrt{x}`. The degree is now dropped only when `m:degHide` is on; `off`
  and `false` read as off.
- **(pdf): recognized formulas no longer swap places when two sit on one line.** When a page had as many detected
  formula regions as extracted formulas, they were paired in reading order, and a formula that sat slightly higher
  than its left neighbour took that neighbour's LaTeX. Regions now pair with formulas by bounding-box overlap, and
  fall back to reading order when overlap cannot pair every region. Pages with mismatched counts also pair best
  overlap first, so an earlier region no longer takes a formula a later one overlaps better.
- **(macos): native libraries preserve the Mach-O alignment required by the system loader.**
- **(pdf): Markdown output honors header, footer, and watermark opt-ins for tagged pagination artifacts.**
  Tagged page numbers follow either the header or footer opt-in. (#2120)
- **(pdf): table numbers split across touching text runs retain their thousands separators without inserted spaces.**
  Adjacent financial columns keep their original boundaries. (#2122)
- **(pdf): every JPEG 2000 image reports 8 bits per component.** A JPEG 2000 decode yields 8-bit samples, and the
  image dictionary's `/BitsPerComponent` is ignored for `/JPXDecode` (ISO 32000-1 Table 89). Only palette images
  reported 8; any other JPEG 2000 image with a stale `/BitsPerComponent` reported the dictionary's value. (GH#2112)
- **(ppt): a table on a legacy `.ppt` slide is extracted as a table.** PowerPoint 97-2003 draws a table as a group of
  one text shape per cell, and the cells were read as loose lines in the order they are stored, which can be
  bottom-right first, so a table came out as a run of values cut off from their headers. The grid is now rebuilt
  from each cell's position and added after the slide's other text. A merged cell is written once at its top-left
  position, as for the other formats, and a row or column with no text, such as a narrow spacer column, is left out.
  A slide whose only line sits over a table keeps that line as its title.
- **(html): Markdown output no longer rewrites the contents of fenced code blocks.** Lines inside a ``` or ~~~ fence
  were treated as prose, so a code line followed by `---` or `===` became a heading and the ruler was deleted, and a
  blank line was inserted before every `#` line. Fenced code is now passed through unchanged. (GH#2118)
- **(macos): the first extraction no longer hangs forever when the host ignores `SIGCHLD`.** Reading the machine's
  physical memory spawned a `sysctl -n hw.memsize` child process, and a host that sets `SIGCHLD` to `SIG_IGN`, such
  as the BEAM, has the kernel reap the child so the wait never returned. The value is now read in-process with
  `sysctlbyname`. (GH#2117)
- **(pdf): a `•` inside the text of a table row no longer turns the row into list items without word spaces.** A
  bullet between two letters is not an item marker, and items split off a paragraph keep the spaces between their
  words. (GH#2115)
- **(pdf): two book pages printed side by side on one sheet are read left page, then right page again.** Since 1.3.9
  the column cut was refused when a few short lines of one half sat within 1 pt of a body line of the other, which
  happens by chance when the halves use different leading (6 pt slide bullets beside 9.9 pt body text). Those
  coincidences were taken for a form's label/value rows, and the halves were read line by line. A cut is now refused
  only when the paired rows cover at least half of the lines on its sparser side. (GH#2116)
- **(php): the Linux extension no longer crashes when PHP exits.** The zig-built `xberg.so` registered static
  destructors that outlived PHP's `dlclose` of the extension, so `exit()` jumped into unmapped memory (SIGSEGV).
  The library is now linked `-z nodelete`, and the release gate checks for it. The linux-arm64 build uses thin
  LTO to stay within the runner's memory. (GH#2113)

### Changed

- **(python): distributions have a permanent GitHub package index alongside PyPI.** PyPI retains all releases
  in the current minor and the latest stable patch of each older minor; archived versions remain installable
  from `https://xberg-io.github.io/python-wheels/simple/`.
- **(rust): the minimum supported Rust version is now 1.95**, matching the upgraded crawler dependency.
- **(html): conversion options add `expand_abbreviations`, `include_blockquote_citations`, and `hidden_content`.**
  Abbreviation expansion and blockquote citations default to enabled; hidden content defaults to being dropped.
  Rust callers constructing `ConversionOptions` with exhaustive struct literals must supply these fields or use
  `..Default::default()`.
  Java callers using the canonical record constructor must add the fields or use the builder; Kotlin callers
  passing positional conversion options should migrate to named arguments because the added fields shift positions.
- **(url): crawler configuration gains additional content options and query-aware deduplication defaults to enabled.**
  Rust callers using exhaustive crawler configuration literals may need to add the new fields.
- **(deps): ONNX Runtime moves to `ort` 2.0.0-rc.14, with `sceptre` 0.8.1 and `tree-sitter-language-pack` 1.22.1.**
  `ort` rc.14 builds sessions from a shared process environment, so every session in xberg, `xberg-gliner` and
  `xberg-paddle-ocr` now takes it. `unhwp` 0.22, `zip` 9.0.3 and `tokio` 1.53.3 are also current.
- **(deps): hayro-jpeg2000 0.4.1 is the minimum, and it resolves JPEG 2000 palettes itself.** 0.4.1 clamps a palette
  index that lossy coding pushes outside the palette, so xberg no longer reads the palette of an image without an
  `/Indexed` colour space and looks it up by hand. A project whose lockfile held 0.4.0 now gets the clamp for every
  palette shape, not only the 8-bit grey, sRGB and CMYK palettes xberg resolved itself. A codestream palette whose
  component count differs from an explicit `/ColorSpace` is still rejected. (GH#2112)

## [1.3.10] - 2026-10-13

### Fixed

- **(pdf): a margin-note column beside a justified main column is read as its own block.** A main column set as one
  span per line and wider than 55% of the page (375 pt of 595 pt) had every line discarded as full-width furniture,
  so the two-column repair never saw a column beside the notes, and its bands were cut at every body line. The
  margin label was placed after the first body line and the following lines alternated between the columns. Spans
  that do not cross the split are no longer furniture; the same holds with the notes on the left. (GH#2109)
- **(pdf): prose that runs across a table's inner gutter is no longer split between columns.** A bordered two-column
  table beside a margin note let the table's gutter become a page-level column cut, and the full-width paragraphs
  above and below it were scattered over both groups, so the opening paragraph was read after a lower heading and a
  short last line such as `2017):` came before its own paragraph. A column cut is now refused when two or more
  continuous lines have at least 60 pt of ink on each side of it. (GH#2110)
- **(deps): the unmaintained `unsafe-libyaml` is gone.** YAML parsing moved from `serde_yaml_ng` to `yaml_serde`
  0.10.7, clearing RUSTSEC-2024-0449 without an ignore entry. (GH#2108)
- **(deps): `ort` is pinned to exactly `2.0.0-rc.13`.** The caret requirement let a fresh resolve pick rc.14, whose
  session builder needs an `Environment`, breaking downstream builds.
- **(swift): the iOS libraries build again.** A workflow-wide `MACOSX_DEPLOYMENT_TARGET` made the Tesseract CMake
  build compile iOS objects as macOS, so linking failed; `xberg-tesseract` now ignores the variable for iOS targets.
- **(elixir): the NIF load check accepts BEAM-provided `enif_*` symbols.** Release builds of the NIF no longer fail the
  `dlopen` probe on symbols only the BEAM supplies.
- **(php): the Linux extension links ONNX Runtime dynamically.** The statically linked runtime needed GCC 13 libstdc++
  symbols that the zigbuild toolchain replaces with libc++, so `xberg.so` could not load on Debian 12. `libonnxruntime`
  now ships beside `xberg.so`, and the release gate fails on unversioned undefined C++ symbols.
- **(zig): trait-bridge callbacks no longer crash when they return a string.** The generated vtable thunks declared
  `char **` out-parameters as 16-byte optionals, so registering a post-processor segfaulted. Regenerated with alef
  0.110.6.
- **(ci): lint and e2e jobs are green again.** The ai-rulez generator is built with `GOWORK=off`, governance uses
  ai-rulez 5.1.0, the integration npm lockfiles match vitest 5, the Ruby e2e lockfile matches the gemspec, and the
  bounded-read and `strict_parse` tests follow `zip` 9.0.2 and the plain-text fallback.

## [1.3.9] - 2026-10-12

### Added

- **(config): `strict_parse` controls what happens when a text-like document cannot be parsed in its own format.**
  By default (`false`), a YAML, JSON, TOML or XML file that fails to parse, an XML or SVG file that trips
  `security_limits.max_entity_length`, and a CSV or TSV file that trips `security_limits.max_table_cells` are extracted
  as plain text instead of failing, and the reason is recorded in `processing_warnings` (source `parse-fallback`).
  Plain-text recovery is truncated to `security_limits.max_content_size` with a warning. Set `strict_parse = true`
  (CLI `--strict-parse true`, environment `XBERG_STRICT_PARSE`) to keep the error. The limits are not loosened, and
  limits aimed at hostile input (archive size, nesting, XML depth) still fail the extraction.

### Changed

- Updated dependencies, including `tree-sitter-language-pack` 1.22.1 (25 refreshed grammars), `zip` 9.0.2, `biblib` 0.10,
  `html-escape` 0.3 and `unhwp` 0.19. `ort` stays at 2.0.0-rc.13 because rc.14 requires an `Environment` for every
  session builder.

### Fixed

- **(extraction): a Helm template or other unparseable YAML, JSON, TOML or XML file no longer fails the extraction.**
  The content is returned as plain text with a warning; `strict_parse = true` restores the error. (GH#2100)
- **(extraction): an oversized XML entity or table no longer discards a text file.** A CSV over
  `security_limits.max_table_cells` or an XML or SVG file over `security_limits.max_entity_length` is extracted as plain
  text with a warning naming the limit that tripped; both limits remain adjustable through `security_limits`.
  (GH#2101)
- **(release): the Helm chart is published even when the CLI image build fails.** The chart publish was gated on the
  aggregate result of the image manifest job, so a CLI image leg timing out skipped the chart for 1.3.6 to 1.3.8 while
  the container images shipped. It is now gated on the `xberg` and `xberg:<version>-core` manifests resolving, the only
  images the chart references. (GH#2106)
- **(pdf): image bounding boxes are correct for images drawn inside a Form XObject with a `/Matrix`.** The form
  `/Matrix` was applied after the page transform instead of before it, so the form's page translation was scaled by
  its own matrix and `ExtractedImage.bounding_box` landed in the wrong place (a regression since 1.2.7). (GH#2103)
- **(pdf): a narrow margin-note column is read before the main text again.** A column of notes set on its own leading
  beside the main text (no note line shares a baseline with a body line) gave the two-column repair too few
  gutter votes once a table's grid rows stopped voting, so the page was read line by line across both columns and
  the margin label was welded to the main heading. A page whose only whitespace corridor is a gutter narrower than
  the native XY-cut's 15 pt valley is now split at that corridor. (GH#2102)
- **(rendering): a block with many inline annotations renders in linear time.** Building the inline nodes of one
  paragraph rescanned every later annotation for each annotation, so Markdown or chunked extraction of a block dense
  with emphasis, code spans and links grew quadratically (1 MiB: 29 s, 4 MiB: over 400 s). The scan now stops at the
  end of the annotated span. The output does not change. (GH#2099)
- **(pdf): two-column pages whose column baselines differ by up to 1pt are read in column order again.** The XY-cut
  line grouping measured gaps between spans in `y` order, so a right-column line set slightly higher sorted first and
  merged with the left column's line into one line, which the column split then sent to one side. Spans are now
  ordered by `x` before gaps are measured, and a gap is measured from the line's rightmost edge so far. A column cut
  that would separate short label/value rows whose baselines differ slightly is refused, so key-value rows and table
  rows read row by row. (GH#2097)
- **(pptx, ocr): plain output of a PPTX file has the OCR text of a picture on a slide.** The text is in `content` and
  in the content of the slide one time. Before, OCR read the picture and the text was in no part of the plain output.
  The text is also there when `images.inject_placeholders` is `false` in Markdown, Djot and HTML output. (GH#2086)
- **(ocr): text that has the form of a Markdown image reference does not repeat the OCR text of a picture.** When a
  document had a paragraph such as `![label](label.png)` as text, and a picture whose OCR text is shown without a
  placeholder, the text of the picture was in `content` two times. A PDF file with the default settings is such a
  document. The text is now there one time. (GH#2093)
- **(ppt, ocr): the content of a slide in `pages` has the OCR text of a picture in plain output.** OCR read the picture
  on a PPT slide and the text was in `content` of plain output, but not in the content of the slide. Page content that
  is built from the elements of a page now has the text one time, after the text of the picture, when `content` is the
  plain rendering: in plain output, and in the output of a custom format name that has no renderer. Page content is
  built that way when the extractor stores no pages: a PPT file is such a document, and a PDF file is one when page
  extraction is not set. The page content of Markdown, Djot, HTML, JSON and a custom renderer does not change.
  (GH#2088)
- **(pdf, ocr): a failed OCR fallback for a whole PDF now reports its failed pages in `ocr_page_failures`.** When the
  OCR run of a whole document failed and the native text was kept, the result had a warning and no record. Each page
  that has no native text now has a record, as it does when OCR runs on the scanned pages only. A page that has
  native text has no record. A page that failed in one OCR pipeline stage and got no text from a later stage had no
  record either, and now has one. (GH#2083)

## [1.3.8] - 2026-10-11

### Added

- **(pdf, ocr): a PDF page whose content OCR replaces can keep its text layer.** `PageConfig` has a new
  `keep_native_content` setting, off by default. When enabled, replaced pages retain their original text layer in the
  new `native_content` page field. The setting requires `extract_pages = true`; default output is unchanged. (GH#2069)
- **(ocr): classic OCR models can use a dedicated deployment cache.** `XBERG_OCR_MODEL_CACHE_DIR`
  supplies the default for PaddleOCR and Sceptre without redirecting other model families, while an explicit
  request cache remains authoritative. `xberg cache seed-classic-ocr` provisions the complete verified catalog
  and emits a deterministic machine-readable manifest for offline container builds. (GH#2072)
- **(go): native setup supports static linking and musl assets.** The regenerated Go setup helper accepts
  explicit link and platform selection, auto-detects musl Linux hosts, and consumes the release's static linker
  metadata when static linking is requested.

### Changed

- **(ocr): the OCR text of a picture follows its placeholder in Markdown and HTML output by default.**
  `images.append_ocr_text` now defaults to `true`, and a missing `images` block reads the same way. When OCR runs on
  the embedded pictures of a document, Markdown and HTML output show the recognized text after the placeholder of each
  picture, for every format that has pictures (PDF, DOCX, PPTX, HTML, EPUB and others). Plain and Djot output already
  held the text. Before, Markdown output with the default settings showed the placeholder alone. Set `append_ocr_text`
  to `false` to get the placeholder alone. (GH#2068)
- **(ocr): the returned image of an image file has no `ocr_result` when the file has OCR text.** When the OCR read
  of an image file such as a PNG gives text or a table, and an `images` block returns the file as an image,
  `images[].ocr_result` of that image is now absent. The OCR text of the picture is the content of the document:
  `content`, `tables` and `ocr_elements`. Before, `ocr_result` held a second OCR read of the same picture, made with
  other settings. Text that only the second read found is no longer in `content` or anywhere in the result. The image
  data, its format and its kind do not change. (GH#2068)

### Fixed

- **(extraction, packaging): Xberg builds with zip 9.** Source-archive extraction, ZIP-based document and archive
  extraction, and MIME routing now handle zip 9's fallible entry names. Published dependency constraints select the
  supported zip 9 line instead of relying on the workspace lockfile.
- **(chunking): Markdown splitting scales to multi-megabyte inputs.** Semantic boundary lookups are indexed once
  instead of rescanning and sorting the remaining document for every emitted chunk, eliminating quadratic CPU time
  on large text-like files. Heading-context resolution walks a precomputed parent chain per chunk instead of replaying
  every earlier heading, which removes the remaining quadratic cost on heading-dense Markdown. Page boundaries are
  validated once per document and chunk page ranges, spans and heading page scopes are found by binary search, so
  heavily paginated documents no longer pay a per-chunk scan of every page. Chunk text, offsets, page data and heading
  context are unchanged; a golden-digest test over generated and real documents pins this, and a scaling guard in CI
  fails on super-linear regressions. (GH#2085)
- **(chunking): semantic chunks report the exact byte range of their text.** When a topic group exceeded the size
  limit, the semantic chunker spread the group's byte range evenly across the pieces it split it into, so `byte_start`
  and `byte_end` did not locate the chunk's text and could fall inside a multi-byte character. The offsets are now the
  real span of each piece, which also corrects the page range and heading context derived from `byte_start`.
- **(chunking): text and Markdown splitting no longer slows down quadratically on text with no line breaks.** Sentence
  and word segmentation of the fallback levels scanned the whole remaining document for every chunk, so a single very
  long line cost time proportional to its length squared. They now look at a prefix just large enough to fill a chunk.
  Chunks are unchanged, which a differential run over about 22,000 adversarial inputs (unterminated runs, CJK, emoji,
  CRLF, mixed and trailing terminators, digit-and-period runs) confirms; 16 MB of unbroken text now chunks in under
  half a second. (GH#2085)
- **(chunking): a chunk is no longer just a stray closing mark, list bullet or heading marker.** When a chunk ended
  right before a sentence's `.` or `!`, or a `-`, `*` or `#` was followed by a unit too long to join it, that mark
  became a chunk of its own. Such a chunk is now filled from finer units instead. On the test corpus this removes
  under 3% of chunks (mostly list and heading markers in small chunk sizes) and moves only the boundaries around them.
- **(chunking): opening quotes, typographic bullets and a closing mark after a full chunk stay with their text.** A
  `"` after a line break, or a `•` bullet, followed by a unit too long to join it was left as a chunk of its own, as
  was a closing `)` or `"` that spilled just past a full final chunk. The splitter now treats any non-alphanumeric
  mark as a mark, refills such a chunk from finer units, and ends the previous chunk a word sooner when only marks
  would remain. A leading whitespace run wider than `overlap` is no longer carried into the next chunk as overlap.
- **(chunking): YAML and JSON section chunks report where their section sits in the source.** Every section used to
  carry `byte_start` and `byte_end` of 0, so all sections resolved to the first page and the offsets located nothing.
  Sections now carry the byte range of their key through the end of their value, `first_page`, `last_page` and
  `page_spans` cover the pages that range touches, and the pieces of a section that had to be split share that range.
  A section the scanner cannot place falls back to its nearest enclosing key.
- **(chunking): attaching bounding boxes and node ids to chunks scales linearly on pages that hold many nodes.** Each
  chunk was compared with every body node on its page, which is quadratic when one page holds most of a large
  document. Pages with many nodes now use a text index; the matched nodes and boxes are identical.
- **(structured extraction): Anthropic native JSON Schema requests retain closed-object constraints.** Structured
  extraction no longer removes `additionalProperties: false` before calling Anthropic models through liter-llm.
- **(pdf): native table cells keep numeric separators attached to their digits.** Thousands and decimal separators,
  including Arabic separators, no longer gain a space when adjacent PDF glyph runs are assembled into a cell.
  (GH#2092)
- **(hwpx): pictures resolve when resource filenames include extensions.** HWPX image references now use a unique
  filename-stem fallback when an exact resource key is unavailable, while ambiguous matches are skipped with a
  warning instead of being chosen nondeterministically. (GH#2091)
- **(rtf): pictures with nested property groups and binary payloads are extracted correctly.** Hex payload collection
  no longer consumes nested picture-property bytes as image data, `\binN` payloads are decoded within
  `max_content_size`, and `\nonshppict` fallback copies are not returned twice when separated from their primary copy
  by a line break. (GH#2090)
- **(jupyter): notebook pictures contribute their OCR text to output.** Attachment and code-output images now retain
  exact source positions and image identities, including prefix-overlapping attachment names, so their recognized
  text appears once in plain, Markdown, Djot and HTML output. (GH#2089)
- **(html, epub, fictionbook): Markdown-family output keeps embedded-image OCR text.** Extractors now invalidate
  pre-rendered markup before image OCR mutates the document, so recognized picture text is rendered once rather than
  being omitted from Markdown and Djot output. (GH#2087)
- **(pdf): a title set across a two-column page's gutter no longer moves the column split beside a lone label.** The
  whitespace-corridor search accepted the gap between a column and a lone header label or page number as a gutter, so
  the two columns were read line by line. That search now needs text on at least six rows on each side of a corridor,
  however many spans a label is written in.
- **(api, pdf): extraction timeouts now stop the underlying PDF work.** Async jobs signal their cancellation token
  when the deadline expires, while native and Pdfium page loops cooperatively stop instead of continuing to consume
  CPU after the caller has received a timeout. Native PDF parsing also runs off the async runtime so synchronous API
  deadlines remain responsive on large documents. (GH#2080)
- **(benchmarks): failed batch processes keep their real exit diagnostics.** When a subprocess exits before RSS
  sampling begins, the harness now reports its exit status and stderr instead of replacing the failure with a
  synthetic "not measurable" resource error.
- **(pdf): borderless key-value headers keep labels before wrapped values.** Native PDF extraction now preserves
  row pairing when vertically centered labels sit beside multiline values instead of emitting the first value line
  before its label. Native-PDF consumers can opt into the same repair for pre-extracted spans with
  `document::reorder_ltr_key_value_rows`. (GH#2079)
- **(doc): Chinese and Japanese text in Word 97-2003 documents is no longer garbled.** UTF-16 CJK text is no longer
  decoded a second time as cp1252, which previously produced mojibake and could truncate mixed Latin/CJK documents.
  (GH#2073)
- **(pdf): text recognized in a picture on a page with a text layer is kept.** Text recognized in a picture on a PDF
  page that also has a text layer is now part of `content` and of that page's content, with or without an `images`
  block. Before, OCR ran on the picture and the words were dropped unless an `images` block was set, and never reached
  the page content. The plain page content of a DOCX page gains the words of its picture in the same way; they were
  in `content` and not in the content of the page. A PDF page that is read by page OCR holds the words of its picture
  one time, in the text of the page: the picture does not add them again in any output format, and with an `images`
  block `content` no longer has them twice. With `ocr_text_only`, the placeholder of that picture stays. (GH#2068)
- **(ocr): a picture with an inline placeholder and an image entry gives its OCR text one time.** With
  `append_ocr_text` or `ocr_text_only`, a document whose picture has both an inline `![alt](url)` placeholder and an
  extracted image, such as an HTML page with a data-URI picture, had the recognized text twice. An inline placeholder
  now takes the text of a picture only when no other part of the output shows that picture. (GH#2068)
- **(ocr): an image file is read by OCR one time, and its text is in `content` one time.** With an `images` block
  that extracts images, an image file such as a PNG was read by OCR twice and the recognized text was in `content`
  twice: in plain, Markdown, Djot and HTML output with the default settings and with `ocr_text_only`, and in plain and
  Djot output with `append_ocr_text` set to `false`. When the OCR read of the file gives text or a table, the returned
  image is not sent to OCR again, and the text is in `content` one time in every output format. With `ocr_text_only`,
  the placeholder of the image stays. An image file whose OCR read gives no text and no table is still read by image
  OCR, as before. (GH#2068)

## [1.3.7] - 2026-10-09

### Added

- **(pdf): pre-extracted spans can use canonical page reading order.** `order_page_spans` applies tagged structure,
  article-thread, and geometric ordering without parsing page content a second time, including for layer-filtered text.
- **(ocr): failed PDF OCR pages are reported as a typed list in every binding.** `ExtractedDocument` has a new
  `ocr_page_failures` field. It is absent (`None`) when no page failed. Each `OcrPageFailure` gives the 1-based `page`,
  the backend `error` text, and `recovered`, which is `true` when the page still has content from native text or from
  its embedded images. When an automatic OCR run of selected pages fails as a whole, each selected page has a record.
  The records were a JSON array under `metadata.additional["ocr_page_failures"]`; that key is no longer set. The
  warnings do not change. (GH#2064)

### Changed

- **(benchmarks): cold-process and warm in-process latency are reported as separate regimes.** Opt-in steady-state
  Xberg runs retain model sessions across iterations, publish a separate cold-start probe, and remain excluded from
  cross-framework rankings until competitors expose equivalent persistent-process adapters.
- **(batch): default in-memory extraction avoids no-op cache work.** Engines without an injected cache backend no
  longer hash every byte input or serialize completed results for a cache that discards them, reducing CPU time and
  transient allocations without changing extractor or OCR cache behavior.

### Fixed

- **(node): extraction works with `AsyncLocalStorage` and async hooks enabled.** Node bindings heap-pin extraction
  futures before creating their JavaScript promises, avoiding a synchronous stack overflow on Node 22 and later.
- **(pdf, ocr): Arabic and Hebrew extraction preserve logical reading order.** Tagged native PDFs use trustworthy
  structure order without reversing table cells, and Tesseract output orders mixed-direction lines and tables from
  their local text direction and geometry. Sparse whole-image RTL OCR also retries a single-block segmentation mode
  when it recovers more strong-script tokens without materially reducing confidence, and mixed Arabic/Latin scans
  recover identifiers only when a bounded English crop provides compatible evidence.

- **(benchmarks): benchmark provenance is bound to the executable that actually ran.** Clean-checkout runs now reject
  stale Xberg binaries whose embedded build identifier does not match the repository commit, and machine-readable
  `xberg version` output includes that identifier for auditability.
- **(doc): legacy Word documents preserve headers, footers, footnotes, comments, and text boxes.** Non-body stories
  are now emitted with their document layers and respect the existing header, footer, and footnote filters instead of
  being dropped whenever the document also contains body text. (GH#2054)
- **(php): Linux packages run on Debian 12 and other glibc 2.36 systems.** Release artifacts are built against the
  declared ABI floor, reject newer GLIBC, GLIBCXX, or CXXABI requirements, and are load-tested in Debian 12 before
  publication. (GH#2060)
- **(ocr): columned prose and punctuation fragments are no longer fabricated as tables.** Long, sparse newsletter
  columns remain ordinary text in both layout and non-layout extraction, while punctuation-only scan regions fall
  back to paragraph output without losing recognized content.
- **(ocr): layout extraction preserves scanned-PDF recognition settings.** Scanned PDFs no longer force Tesseract
  into whole-image single-block segmentation when layout assembly has detected regions, while known scans keep their
  sparse-text segmentation and source-resolution preprocessing hints.
- **(typst): tables wrapped in figures are preserved.** A `#table(...)` nested inside a `#figure(...)`, including
  through an `align(...)` wrapper, now reaches structured and rendered output instead of being discarded with the
  figure body.
- **(latex): multiline section titles remain headings.** A section command whose braced title spans physical lines
  now produces one heading with its label anchor instead of flattened paragraph text and command fragments.
- **(python, pdf): Python post-processors preserve formatted output and hide internal PDF heading annotations.**
  Returning an `ExtractedDocument` from a Python post-processor no longer replaces Rust-only document state, so a
  no-op processor keeps rendered headings and chunks. Font-size annotations used for native PDF heading detection are
  also excluded from plain text and keyword extraction. (GH#2050)
- **(pdf): conflicting JPEG 2000 palette color spaces fail consistently across architectures.** Native PDF extraction
  now rejects a codestream palette whose component count conflicts with an explicit `/ColorSpace`, instead of relying
  on architecture-dependent decoder rounding that could accept the same malformed image on ARM and reject it on x86.
- **(zig): generated bindings compile with Zig 0.17.** String duplication now uses the Zig 0.17 sentinel API, and
  plugin vtables cross the C ABI through layout-compatible pointers instead of rejected value bitcasts.

## [1.3.6] - 2026-10-07

### Changed

- **(zig): the Zig binding now requires Zig 0.17.0.** `build.zig.zon` declares
  `.minimum_zig_version = "0.17.0"` and the package is built and tested on Zig 0.17.
- **(pdf): lossy JPEG 2000 images now decode to the same pixels as OpenJPEG.** Since 1.3.5 the JPEG 2000
  decoder applies the standard's reconstruction offset for lossy coefficients (ISO 15444-1 E-6..E-8); earlier
  releases decoded such images slightly darker. Rendered pages and OCR input change accordingly. On scans with grey
  shaded table rows, OCR without `normalize_shaded_rows` can now recover fewer table rows than in 1.3.4; with
  `normalize_shaded_rows` enabled, results shift by only a few values.

### Fixed

- **(docx): a footnote and an endnote with the same number are kept apart.** Word numbers footnotes
  and endnotes independently, and both were keyed `fn<id>`, so in Markdown, HTML and Djot output the
  first endnote replaced the first footnote and both references pointed at it. Endnotes are now
  keyed `en<id>`, as in the ODT and WordPerfect extractors.
- **(pdf): mixed-rotation pages preserve word boundaries in upright text.** Native extraction no longer rewrites an
  entire page through the rotated-run assembler when the upright content spans multiple rows, which previously joined
  adjacent words in reports with rotated footer text. (GH#2049)

## [1.3.5] - 2026-10-06

### Added

- **(code): BAML files are recognized as source code.** The tree-sitter language pack now includes BAML grammar and
  extension support, so `.baml` inputs route through code extraction instead of being rejected as unknown.

### Changed

- **(config): URL crawling and HTML conversion expose the latest safety and browser controls.** Crawl patterns can
  match the full normalized URL, Chromium launches can select an executable and trusted command-line flags, custom
  SSRF CIDR deny-lists take precedence over permissive settings, and HTML conversion can reject input above a
  caller-defined byte limit. These options are available in Rust and generated language bindings.

### Fixed

- **(security): REST and MCP extraction requests preserve operator-owned crawl egress policy.** Untrusted callers can
  no longer override crawl SSRF limits, proxy and browser launch settings, or document and WARC output paths through
  global or per-input configuration; caller-owned crawl depth, authentication, and custom headers remain supported.
- **(api): `KeywordConfig.yake_params`, `KeywordConfig.rake_params` and `TesseractConfig.preprocessing` are described as a direct `$ref` rather than `oneOf[$ref, null]`.** All three are omitted from the payload when unset. GH#1841 annotated the omission-only references in types the OpenAPI document registers, and these two config types are registered by downstream specs that embed them instead. `TesseractConfig.preprocessing` also no longer advertises `"default": null`, which the struct-level `#[serde(default)]` added. Wire payloads and generated bindings do not change. (GH#2041)
- **(excel): date, time and duration cells read as the sheet shows them.** A time-only cell was
  rendered on the date 1899-12-31, a date-only cell gained `00:00:00`, and a `[h]:mm` duration of
  36 hours read as `1900-01-01 12:00:00`. Times of day now render as `HH:MM:SS`, serials with no time
  of day as `YYYY-MM-DD`, other date-times as before, and elapsed-time formats as total hours `H:MM:SS`.
- **(docx): keep the first footnote and endnote of a Word document.** Word numbers the separator
  lines in `footnotes.xml`/`endnotes.xml` -1 and 0 and its first real note 1, and the parser skipped
  id 1 as a separator, so the first footnote and the first endnote were dropped with their markers.
  Separators are now recognised by their `w:type`; LibreOffice's numbering (separators 0 and 1) is
  handled as before.
- **(pdf): malformed classic xref tables no longer cache the wrong indirect object.** When a subsection starts at
  object 1 but includes object 0's free entry, native PDF parsing now recovers each shifted reference from its actual
  object header instead of treating the page tree as the catalog. (GH#2044)

## [1.3.4] - 2026-10-04

### Fixed

- **(ocr): keep spaced invoice rows in one table and recover a missing quantity from the original pixels.**
  A scanned invoice could split into two tables and lose a short quantity after preprocessing. Full-sized aligned
  rows now stay together across a bounded gap, and a missing quantity gets a high-confidence, cell-bounded retry
  against the source image. (GH#2043)

## [1.3.3] - 2026-10-02

### Added

- **(cli): `extract` and `batch` can write converted content directly to files.** Pass
  `--output <FILE>` to `extract`, or `--output <DIRECTORY>` to `batch` to derive one filename per
  input from its stem and content format. Existing destinations and colliding batch names are
  rejected instead of overwritten. (GH#2040)
- **(html): inline `data:` media handling is configurable across Rust and every generated binding.**
  `ConversionOptions.inline_data_media` and the new `InlineDataMedia` enum let callers keep the
  full data URL (the default), emit only alternative text, or drop the element.
- **(onnx): `set_ort_session_options` bounds ONNX Runtime memory for embedding, reranking, sparse-embedding and late-interaction sessions (Rust API only).** `OrtSessionOptions { memory_pattern, cpu_arena, max_threads }` (defaults `true`, `true`, `None`, i.e. unchanged behaviour) can disable ORT's memory-pattern planner and CPU arena, which otherwise retain allocations sized by the largest batch, and set the intra-op thread count. `cpu_arena` affects the CPU execution provider only, and `max_threads` does not change other thread budgets; OpenMP-enabled ORT builds ignore it and must use `OMP_NUM_THREADS`. Changing the options clears only the affected ONNX engine caches so resident engines rebuild with them; static embeddings, layout, table, Whisper, paddle and GLiNER sessions are unaffected.
- **(wasm): arrays of generated class values expose `copyForTransfer()`.** JavaScript callers can
  clone handle-backed elements before passing an array into an owning constructor or setter,
  avoiding invalidating objects they still use. Class-valued getters document that returned
  values are detached copies and must be reassigned after mutation.

### Fixed

- **(markdown): DOCX and ODT tabs render as spaces instead of literal `&#9;` entities.** This keeps
  label-value text and tab-indented lines readable in Markdown output. (GH#2039)
- **(csharp, java): custom string variants preserve their payloads across JSON.** The generated
  `EntityCategory`, `OutputFormat`, and `PiiCategory` APIs now use sealed record hierarchies for
  externally tagged unit and `Custom(String)` variants instead of payload-dropping plain enums.
  This is a breaking generated API correction in the patch release: callers must construct or
  match the generated variant records rather than enum constants.
- **(docx): a table, text box or image after a nested list no longer ends up inside the list.** These blocks closed only the innermost list level and then stopped tracking the rest, so after a list two or more levels deep the block and the content after it rendered under the last list item. They now close every open level, as headings and paragraphs already did. (GH#2035)

## [1.3.2] - 2026-10-01

### Added

- **(redaction): findings from an external content-inspection engine can drive redaction.** `RedactionConfig.findings` and `findings_path` accept inline JSON, JSON Lines, or a file, while the additive `extract_with_external_redaction` and `redact_external` operations accept raw JSON or JSON Lines for request-scoped findings. Each value is redacted at every occurrence in every textual field and reported as `PiiCategory::Custom(label)`; `redaction.min_score` filters scored findings below a validated `[0.0, 1.0]` threshold while retaining unscored findings. Unknown fields are ignored, Presidio, AWS Comprehend, Azure Language, and GCP DLP field names are accepted as aliases, and offsets support UTF-8 bytes, Unicode code points (the default), and UTF-16 code units. An unresolvable finding fails instead of being skipped, configured findings are capped at the lower of `SecurityLimits.max_iterations` and 10,000, and `parse_external_findings_bounded` enforces caller-supplied limits while parsing. (GH#1941)
- **(cli): `--redaction-findings <PATH|->` passes an external engine's findings to `extract` and `batch`.** It takes a JSON array or JSON Lines file, or `-` to read them from stdin, and turns redaction on if the config has not. `-` cannot be combined with `extract --stdin`. (GH#1941)
- **(captioning): generated image captions can preserve, combine with, or replace existing alt text.** Set `captioning.alt_text` to `"preserve"` (the backward-compatible default), `"combine"`, or `"replace"`; `ExtractedImage.caption` continues to hold the generated caption while `description` and rendered DOCX/PPTX image placeholders follow the selected precedence. (GH#1998)
- **(rendering): `output_format = "docx"` returns the document as a Word file.** `content` holds the `.docx` package base64-encoded and `metadata.output_format` reads `"docx"`; the CLI's `--content-format docx` writes the file itself to stdout. The package is built from the Markdown rendering after every post-processor has run, so redaction removes a term from the Word file exactly as it does from Markdown output. Headings, paragraphs, emphasis, links, nested lists, tables and code blocks carry over; images are not embedded, and chunks describe the Markdown the file was built from. Requires the `office` feature. (GH#1942)
- **(rendering): `output_format = "pdf"` returns the document as a PDF file.** `content` holds the PDF base64-encoded and `metadata.output_format` reads `"pdf"`; the CLI's `--content-format pdf` writes the file itself to stdout. The PDF is laid out from the Markdown rendering after every post-processor has run, so redaction removes a term from it exactly as it does from Markdown output. Text stays selectable and extractable: it is set in an embedded subset of DejaVu Sans with a `ToUnicode` map, and code in the standard Courier faces. Headings (also listed in the PDF outline), emphasis, links, nested lists, tables and code blocks carry over; images are not embedded, and characters DejaVu Sans has no glyph for, such as CJK text, draw as boxes but still extract. Requires the `pdf` feature. (GH#1943)
- **(tesseract): `ResultIterator::extract_all_words_with_line_starts` reports which extracted words begin Tesseract text lines.** It returns the existing word extraction outcome plus an aligned vector of flags, moving a failed first word's flag to the next successfully extracted word without changing `WordData` construction. (GH#1978)
- **(ocr): typed PaddleOCR settings are available without breaking the legacy config field.** `OcrConfig.paddle_ocr_settings` and `OcrPipelineStage.paddle_ocr_settings` expose `PaddleOcrConfig` in every binding, while `paddle_ocr_config` remains raw JSON with its existing Rust, C and binding APIs. Typed settings take precedence when both fields are set. Config files, JSON configs, and REST or MCP request bodies use snake_case keys under `paddle_ocr_settings` and reject unknown keys; binding objects use their language's usual field names. The `paddle-ocr-types` Cargo feature remains accepted as a compatibility alias. (GH#2010)

### Changed

- **Breaking (Rust and generated bindings): configuration constructors change with the new fields and corrected defaults.** Exhaustive Rust struct literals must include `CaptioningConfig.alt_text`, typed `paddle_ocr_settings` on `OcrConfig` and `OcrPipelineStage`, and the external-finding fields on `RedactionConfig`, or use `..Default::default()`. Java record canonical constructors gain the corresponding components. PHP constructors move fields with generated defaults after required parameters, and `RedactionConfig` gains a required `findings` array; positional PHP calls must be updated or replaced with named arguments. Builder, default-constructor, and JSON surfaces continue to apply defaults where their binding supports omission. These compatibility breaks are intentionally shipped in the `1.3.2` patch release so the completed fixes remain available together. (GH#1941, GH#1998, GH#2010)
- **(ocr): Tesseract returns line elements next to its word elements.** Each line holds its words' text in reading order, the union of their boxes and their word-count-weighted mean confidence. A caller who requests `min_level: "word"` now gets the lines first, then the words, so element positions and the ids from `build_hierarchy` shift. With `layout` configured, image OCR text is now assembled from these lines, so it reads in the source's word order instead of a per-word position sort. (GH#1978)

### Deprecated

- **(ocr): raw `paddle_ocr_config` fields and the `paddle-ocr-types` Cargo feature are deprecated.** Use typed `paddle_ocr_settings` on `OcrConfig` and `OcrPipelineStage`. The raw JSON fields remain lenient toward unknown extension keys for compatibility, while invalid values for known settings still fail validation. The Cargo feature remains accepted as a no-op alias. Both are planned for removal in 2.0. (GH#2010)

### Fixed

- **(python): nested configuration fields accept the public config classes.** Generated Python config objects such as `OcrPipelineStage` now convert public `xberg.*Config` dataclasses, dictionaries, and JSON strings instead of requiring callers to construct private `xberg._xberg` classes. (GH#2015)
- **(redaction): structured table column labels no longer retain PII after redaction.** Redaction now rewrites `tables[].columns` and `pages[].tables[].columns` alongside table cells and Markdown, so a header copied into the structured column metadata cannot expose a value removed from the document text. (GH#1991)
- **(cli): a `redaction` block in a CLI config is applied.** No CLI build compiled the redaction engine in, so the block parsed, did nothing, and the output came back unredacted. The `redaction` feature is now part of the default and `binstall` builds, and `all` through the default. (GH#1941)
- **(redaction): a validation error raised while redacting now fails the extraction instead of returning the document unredacted.** The pipeline kept a post-processor's validation error as a processing warning and returned the document as extracted, so a redaction run that failed, for example an `Llm` NER backend with no `NerConfig.llm`, handed back the very text it was asked to redact. The redaction processor now reports these as a plugin error, which fails the extraction. (GH#1941)
- **(ocr): PaddleOCR's selected-page PDF route keeps embedded-image prose beside recovered tables in Markdown.** When a page render was blank, the embedded-image retry recovered both prose and a table, but document restructuring rebuilt the page from an empty paragraph list and retained only the table. The retry's non-table text now feeds that restructuring pass, matching plain output without duplicating table text. (GH#2021)
- **(plugins): cached file extractions now reflect the currently registered post-processors and validators.** Registering, removing, or replacing a lifecycle hook invalidates entries produced by the older registry state, so the next extraction runs every current hook exactly once and refreshes the cache. (GH#2032)
- **(ocr): macOS Apple Silicon release binaries now accelerate Candle OCR with Metal.** The downloadable `aarch64-apple-darwin` CLI enables `candle-metal`, so the bundled TrOCR, PaddleOCR-VL, GLM-OCR, and DeepSeek-OCR backends no longer fall back to impractically slow CPU inference on supported Macs. Other release targets keep their existing feature sets. (GH#2033)
- **(config): project-local YAML and JSON configuration files are now auto-discovered.** `ExtractionConfig::discover()` probes `xberg.toml`, `xberg.yaml`, `xberg.yml`, and `xberg.json` in that order in the current directory and each parent before falling back to the user config directory. (GH#2017)
- **(excel): spreadsheets with valid producer quirks no longer lose content or fail parsing, and malformed XLS files cannot abort on a forged stream length.** XLSX worksheets whose dimension names only rows now extract their shared-string cells, legal indentation whitespace between ODS cells is ignored, and legacy XLS/XLA compound streams are checked against the container size and `security_limits.max_archive_size` before parsing. (GH#2002, GH#2003, GH#2005)
- **(odt): text boxes in page-anchored frames are extracted, and encrypted documents report encryption explicitly.** Paragraphs under `draw:frame > draw:text-box` no longer produce an empty success when the frame sits outside normal paragraph flow, and recursive frames, lists, and inline wrappers respect the configured nesting limit. An ODT whose manifest marks `content.xml` as encrypted now returns an encryption error instead of reporting its ciphertext as invalid UTF-8, without assuming whether password- or public-key encryption was used. (GH#2006, GH#2004)
- **(ocr): automatic PaddleOCR fallback no longer enables table detection unless the caller asks for it.** The synthesized Tesseract-to-Paddle pipeline keeps PaddleOCR's safe default instead of inferring Paddle table intent from Tesseract. Explicit legacy or typed PaddleOCR settings remain authoritative, and selecting a Paddle model version or tier does not change table-detection intent. (GH#2018, GH#2030, GH#2031)
- **(ocr): scanned tables keep sparse amount columns split by digit-width drift.** Table post-processing no longer folds an unnamed value track into a label when the matching numeric track is on its right. It now folds row-disjoint values into the numeric neighbour on either side, recognizes multi-row period labels and consecutive plain years as headers, and keeps a title fragment with the title while moving values under the correct year. (GH#1832, GH#2028, GH#2029)
- **(ocr): a scanned table is no longer discarded over one unnamed, half-empty amount column.** Table post-processing rejected the whole table when a column with no header was empty in more than half of its rows and held few characters, but folded the same column into its neighbour when it was nearly empty. A right-aligned amount column that OCR splits by digit width sits between those two bars, and a correct row merge elsewhere in the grid could move it from "folded" to "rejected", so the page fell back to plain text. An unnamed value column beside a value column is now folded whenever that gate would reject the table on it, and the short amounts return to the column they were split from. (GH#2019)
- **(ocr): table post-processing no longer breaks when it folds the first column.** The fold looked for a title column left of column 0. Debug builds panicked on that lookup, and release builds dropped the header of a title and value track that an earlier fold had moved to column 0. (GH#2026)
- **(ocr): a PDF page that renders blank keeps the text OCR recovers from its embedded images when those images also hold a table.** With `force_ocr` and table detection on, such a page is read again from its embedded images. The result kept the recovered table but dropped the recovered text, with Tesseract and with PaddleOCR. The page text is now left out only when the OCR backend reports that the detected tables hold all of it, which also keeps a full-page table from printing twice. The recovered text also no longer repeats the recovered table: for an image with tables, the retry keeps only the lines those tables do not hold, so each table appears once. (GH#2008, GH#2014)
- **(cache): structured extraction results now produce cache hits.** Extraction cache entries use named MessagePack fields so tables and document nodes round-trip correctly. Unreadable legacy entries are reported and safely replaced after re-extraction. (GH#1990)
- **(mime): large CSS and TOML files with array-like prefixes are no longer mistaken for JSON.** Valid JSON arrays and objects larger than the 4 KiB sniffing window remain detected as JSON. (GH#1982)
- **(ocr): label-and-value tables no longer disappear when labels contain most of the text.** A two-column grid whose first column contains recurring labels and whose second column is mostly numeric values now bypasses the prose-oriented dominant-column rejection. PaddleOCR also reports structurally rejected table candidates in `processing_warnings` instead of leaving only a debug trace. (GH#1970)
- **(wasm): Tesseract OCR returns word elements when `element_config.include_elements` is enabled.** The WebAssembly backend now includes each word's bounding box and recognition confidence, applies the configured level and confidence filters, and leaves element extraction disabled unless requested. (GH#1974)
- **(ocr): a tall word box no longer splits one scanned-table row into two.** Row detection now anchors on representative-height words, lets a tall word seed a row only when it overlaps no existing row band, then assigns every word by overlap with those bands. A word whose OCR box grows above or below its neighbours therefore remains in their row regardless of input order, while a standalone tall heading remains its own row. (GH#1977)
- **(transcription): a 30-second window the Whisper decoder cannot process no longer aborts the whole recording.** The greedy loop treated the decoder's 448-token context as a budget for generated tokens alone, so once a repetition loop pushed the prompt plus generated tokens past the position-embedding table, the decoder sliced an empty position range and crashed with an ONNX `Reshape` error (`Input shape:{8,0,64}`). The prompt length is now reserved from the context budget, and a window that still fails is skipped, reported as a `processing_warnings` entry, and the remaining windows are transcribed instead of the file failing. (GH#1944)
- **(pdf): numeric footnotes stay separate in structured paragraphs as well as page text.** A reference such as `comma 3` followed by note `5` no longer becomes `comma 35` in `document.nodes` or Markdown. Inline-script rejoining now uses the same matching-note evidence as plain-text extraction; scripts without that evidence and adjacent mathematical expressions retain their existing joins.
- **(ocr): numbered list items detected in standalone images keep their printed numbers.** Image layout extraction now separates a leading numeric marker from the item text and records it as the source label, so Markdown rendering no longer strips the only copy of the number. (GH#1979)
- **(ocr): a table header label wider than the amounts under it no longer splits its column.** On a scan, a right-aligned label that reads as text, such as a period written as letters and digits, starts further left than every amount and formed a header-only column of its own. The column's long amounts then followed the label's left edge, the short amounts stayed in a headerless column, and the sparse-column check dropped the whole table. A header-only track now folds into the value column whose right edge it shares, whatever its label reads, unless that column has a label of its own in the same header row: two labels side by side stay two columns. With `normalize_shaded_rows` on, which stays opt-in, a scanned table whose recovered header band carries such period labels keeps its table. (GH#2001)
- **(ocr): a scanned table is no longer dropped when OCR reads one junk cell in a split amount column, or when its header band spans several rows.** On a scan, one right-aligned amount column splits into two adjacent columns by digit width. The fold that rejoins them ran only when every token below the first row read as a value, so one misread letter, one rule mark, or the labels of a second header row kept the column split. The leftover column then failed the sparse-column check and the whole table was lost. The fold now reads the header as the band of rows above the first row with a number, and folds a column whose values outnumber its other tokens. A text column never folds, whatever numbers it holds, and two columns that each hold a number in the same row never fold. (GH#1952)
- **(pdf): retain alignment instructions beside split description cells.** Cell detection now searches for a closing boundary on both sides instead of stopping at a neighbouring row separator. (GH#1958)
- **(pdf): table-cell text keeps the same word spacing as the page text.** On a tight face the glyphs on either side of a source space can abut or overlap, so the word merger read the zero-or-negative gap as "one word" and fused the two — turning `corrispettivi superiori al costo` into `corrispettivisuperiori alcosto` and `d’impresa` into `d’im presa` inside table cells while the page text was correct. The word merger now honours the source whitespace as a word boundary, table words carry that source space even when their boxes touch, and the fragment re-glue band was widened so a font that kerns one word apart by more than a normal space still reassembles. (GH#1948)
- **(dbf): every row of a dBASE table now keeps its values under the headers they belong to.** Each record was read into a name-keyed `HashMap` and walked in hash order, which differs from record to record and from run to run, so a table with more than one field came back with every row's cells permuted and `metadata.format.fields` could pair a field name with another field's type. Records are now read in field-declaration order, which also keeps both values when two fields share a name instead of dropping one. (GH#1968)
- **(pdf): preserve grouped table header spans in structured output.** Native span geometry now reaches document nodes, while dense cells and Markdown retain consistent columns. Mixed row/column spans use the shared placement rules, and redaction covers retained native grid text. (GH#1959)
- **(ocr): Tesseract returns OCR elements when `include_elements` is set without a `min_level`.** The default level is `line`, and Tesseract produced only words, so images and PDFs returned no elements at all. (GH#1978)
- **(heuristics): the OCR confidence aggregate and the quality-score evidence floor count each recognized word once.** When a result carries words and the lines that hold them, as PaddleOCR and Tesseract results do at `min_level: "word"`, only the finest level is folded. Previously every word counted twice, so ten words reached the 20-word floor of the quality-score cap. (GH#1978)
- **(ocr): a table that PaddleOCR detects in an image no longer repeats its text in the content.** With `enable_table_detection` on, the content listed each recognised line of the table as its own paragraph and then the same text again as the table rows. PaddleOCR now removes the lines that a detected table carries, with the same rule the Tesseract backend uses. A table that lost words from its region keeps those lines, so no text is lost. A scanned PDF page that is only a table no longer shows the table text a second time on the `force_ocr` route. A prose image that table detection turns into a table now shows its text once, as that table. (GH#1973)
- **(ocr): a table whose rows are spaced widely no longer loses its trailing line items.** Rows spaced beyond the vertical region-gap threshold became one-row regions; with fewer than the six-word table minimum they were dropped, so an invoice kept only its header and first item. A region below the minimum now attaches to a column-aligned neighbour instead of being discarded, while genuinely separate tables — each at least the minimum size — still stay apart. (GH#1957)
- **(ocr): a scanned table whose amount column splits into two tracks is no longer dropped when OCR reads the page's rules as cells.** On a scan, one right-aligned amount column can split into two adjacent columns by digit width. The merge that rejoins them refused whenever a track held a mark read from a rule or a shaded band (`-`, a dash run, `:`, `~`), so the leftover track failed the sparse-column check and the whole table was lost. A cell with no letter or digit now counts as empty when the merge compares the two tracks, such a mark in the header band is no longer a column label, and the merged cell keeps the real amount. A sign, currency sign or bracket in front of an amount still keeps the two tracks apart, so it is never dropped. (GH#1949)
- **(ocr): a scanned page whose text layer has no usable character map keeps the scan's segmentation mode.** Automatic OCR routing gave Tesseract block mode (PSM 6) to every page whose text layer has no usable character map, including a scan that carries such a layer over its image. On a scanned table, block mode loses the table reconstruction, so the values leave their rows. Block mode now applies only to a page without a scan raster: a page whose images cover a quarter of it or more keeps the mode a scan gets. A page of unmapped vector text still gets block mode, and an explicit caller setting still wins. (GH#1946)

## [1.3.0] - 2026-09-28

### Added

- **(ocr): PDF pages with an unusable text-layer character map use Tesseract block segmentation per page.** During automatic OCR fallback, only pages routed by the fabricated-character-map detector use PSM 6 so table row labels and values stay together; ordinary scans and explicit `force_ocr` or `force_ocr_pages` requests retain their existing mode, and an explicit caller setting still wins. Accepted Tesseract pages record their effective mode in `metadata.additional.ocr_page_segmentation_modes`. (GH#1896)
- **(python): `extract` and `extract_batch` accept an optional `on_progress` callback for per-page OCR progress.** Each `ProgressEvent` reports the 1-based page number, total pages, completion sequence, OCR backend, and the zero-based input index for batch extraction. Callbacks run off the extraction worker; a callback exception is logged and ignored so extraction can finish. (GH#1887)
- **(bindings): `pdf_page_count` is available again in every language binding.** It returns the number of pages in a PDF without rendering any of them, takes an optional password for an encrypted file, and raises an error on bytes that are not a PDF. The 1.0.0 notes list it as the cheap way to size a render loop, but a later regeneration dropped it from the bindings, so it existed only in Rust. It now ships in Python, TypeScript (Node and WASM), Ruby, PHP, Go, Java, C#, Elixir, Dart, Kotlin, Swift, Zig and the C FFI. (GH#1869)
- **(bindings): the published Python, Node, PHP, Ruby and Elixir packages compile the `pdfium` PDF backend in.** `PdfBackend::Pdfium` is documented in every one of their generated API references, but selecting it always failed: the Cargo feature that compiles the engine was enabled for no published artifact. These packages still do **not** bundle `libpdfium` -- supply it yourself and point `PDFIUM_DYNAMIC_LIB_PATH` at the directory holding it, as described under "Pdfium PDF backend" in the installation guide. The engine reaches the Linux (x86_64, aarch64) and macOS Apple Silicon artifacts only; the Windows and Intel-macOS packages build from a reduced feature set, and WebAssembly, Android and iOS cannot load a shared library at all. `native` remains the default backend and nothing changes for a caller that does not select `pdfium`. (GH#1863)
- **(tesseract): `ResultIterator::extract_word_symbols` reads the text and box of each symbol in the words that a filter accepts.** It walks the page under one lock, like `extract_all_words`. (GH#1833)
- **(ocr): `OcrBackend::supported_languages_for` and `supports_language_for` report a backend's languages under a specific `OcrConfig`.** `list_ocr_backend_capabilities_for` and `ocr_backend_supports_language_for` are the corresponding config-aware forms of the existing config-less functions. Use these when the caller sets `OcrConfig.tessdata_path`: for Tesseract the language list is a property of the resolved tessdata directory, and the config-less forms always answer for the no-override directory. (GH#1857)

### Changed

- **(wasm): assigning a class-typed property no longer destroys the handle you assigned, and clearing an optional one now needs `clearX()`.** wasm-bindgen lowers a by-value exported-struct argument through `__destroy_into_raw()`, so `options.field = handle` left the caller holding a dead handle: reusing that handle, or assigning it to a second options object, threw `null pointer passed to rust`. Such a setter now takes a borrow and stores a clone, so the handle stays alive. Because `Option<&T>` has no `OptionFromWasmAbi` implementation, an optional field of a generated class type takes the same borrow and wraps it in `Some`, and a generated `clear{Field}()` companion is what unsets it -- **`opts.x = null` now throws rather than clearing the field**, which is the one breaking change for existing callers. The generated `.d.ts` accessor pair for such a field is asymmetric (`get x(): T | undefined; set x(value: T);`) and therefore requires TypeScript 5.1 or newer. Separately, a trait-bridge options field assigned as a property was blanked before the call and so had no effect at all; it is now the fallback when no explicit visitor argument is given. (alef #470)
- **(ocr): `TesseractConfig::thresholding_method` is an integer selecting the binarization algorithm, where it used to be a `bool`.** The field was sent to Tesseract as the string `"true"` or `"false"`, which its integer parameter parser cannot read -- and `SetVariable` reports no error for a value it fails to parse, so the check around that call never fired. The setting therefore did nothing at all, and methods 1 (LeptonicaOtsu) and 2 (Sauvola) were unreachable: on the reporting document Tesseract's own CLI reads 135 of 138 values at method 1 where xberg read 120. Valid values are 0 (Otsu, the default and the previous effective behaviour), 1 and 2, and they are now validated rather than passed through. Configuration files and any caller that supplies a map or dictionary are unaffected -- a legacy `false` still deserializes to 0 and `true` to 1, the method its old documentation described. This is a breaking change only for callers that assign the field in typed code, in Rust or in a generated binding, where `true` becomes `1`. (GH#1784)
- **(pdf): `metadata.pages` is now populated for `ocr_near_empty_fallback` or `ocr_scanned_page_quality_gate` set without an `ocr` block, at parity with what an `ocr` block alone already produced.** The annotation fallback's implicit page-boundary tracking was discarded from `metadata.pages` based on a narrower, re-derived condition than the one that decides whether boundaries are tracked in the first place, so a caller relying on either GH#1752 setting alone still got `metadata.pages == None`. (GH#1752)

### Fixed

- **(pdf): a borderless table beside a prose column is no longer reconstructed across the page gutter.** The heuristic table pass now separates a dominant whitespace corridor before column detection, while rejecting a split that leaves labels on one side and values on the other so financial tables keep their rows. Recurring numeric tracks keep a real table with wrapped row labels from being rejected as flowing prose, and full-width blocks below the columns remain intact. (GH#1769)
- **(ocr): a multi-word table header now stays in the data column it labels.** Header words that had already merged into one cell for column detection were split apart again during assignment, so a trailing word could move into the next header. A merged header is now assigned as one cell when its detected column has data rows, while header-only spans keep the existing fragment repair. (GH#1934)
- **(ocr): shaded-row normalization no longer loses dark and mid-grey table rows.** Overlapping inversion boxes could invert part of a dark row twice, while normalized band edges could be read as `[` or `|` frame glyphs and hide otherwise recognized rows during table reconstruction. Overlapping boxes are now inverted once, and repeated normalization artifacts at a table's outer edges are removed before cell assignment. (GH#1837)
- **(excel): legacy XLS and XLA compound files no longer undergo ZIP-bomb validation as outer ZIP archives.** An embedded ZIP whose central directory could be discovered from the OLE bytes made security accounting seek its local headers against the wrong container and reject the workbook before the XLS parser ran. The bypass requires both a legacy spreadsheet extension and the OLE signature; ZIP-backed spreadsheet formats retain the configured archive limits. (GH#1938)
- **(doc): Word 6/95 binary documents now reach the legacy text extractor.** The parser accepts the `0xA5DC` signature, recognizes legacy `nFib` versions before opening a Word 97 table stream, and reads the legacy `ccpText` field from its actual header offset. Fast-saved Word 6/95 files remain unsupported and now return an explicit remedy instead of being misparsed as Word 97. (GH#1939)
- **(ocr): a shaded table row split by its own baseline gap is no longer struck through by the shaded-row normalisation.** A dark-fill row is found as a run of rows that read mostly dark, and the baseline gap of a line of capitals can read light enough to end that run and start a new one, so one printed row is found as two bands. The padding that grows each band onto the fill's anti-aliased edge then grew both halves back across the gap, and the rows they both covered were inverted twice -- returning them to the fill's own dark value as a rule drawn through the middle of the row's text. On the reference scan that rule crossed a totals row, which lost its label and read its `30,474` as `20,474`, and it disturbed the plain row above enough that its label came back as a stray fragment plus a misread word. Each row is now inverted exactly once. Measured on that scan, ground-truth values landing in the correct cell rose from 102 to 108 of 138 at the segmentation mode the PDF OCR route selects by default, with its light-fill and plain rows both becoming complete; dark and mid-fill rows are unaffected and remain the subject of GH#1837. (GH#1918)
- **(ocr): the automatic Tesseract-to-Paddle fallback is decided independently for each PDF page.** Whole-document OCR previously scored the pages' joined text, so one readable page could hide an unreadable page and prevent Paddle from running; selected-page OCR silently skipped the automatic pipeline and ran Tesseract alone. Both routes now stop at Tesseract for a good page and invoke Paddle only for a page below the configured quality threshold. (GH#1908)
- **(ocr): automatic OCR fallback keeps a valid native page when other pages in the document are scans.** When trustworthy page boundaries were available, the quality gate still evaluated the whole-document average first. A document with one native page and nine empty scanned pages therefore sent all ten pages through OCR, replacing the native page and its tables. The gate now decides each bounded page independently and uses whole-document scoring only when page boundaries are absent or untrustworthy. (GH#1927)
- **(ocr): scanned-page settings now survive detached-image OCR pipeline calls.** PDF pages selected by `force_ocr_pages`, scan detection, or per-page fallback now keep the same source DPI, scan preprocessing signal, and default Tesseract segmentation mode as whole-document OCR when layout detection, `vlm_fallback`, or an explicit pipeline is active. Standalone image pipelines likewise use the image's embedded or inferred DPI and whole-image mode. (GH#1926)
- **(ocr): an inset scanned page with more than 400 unmapped Type 3 glyphs is still recognised as a scan.** The inset-raster check counted every extracted glyph as readable native text, including fallback characters fabricated for a font with no usable Unicode mapping, so a broken OCR sidecar could make the page look like body text beside a figure. The check now counts only mapped glyphs toward its native-text limit; a page with more than 400 genuinely mapped glyphs remains a text page. (GH#1921)
- **(ocr): one selected PDF page's OCR failure no longer discards successful OCR from the other pages.** Render, upright-rotation, backend, and pipeline failures are isolated to their page with a page-numbered warning. An explicit `force_ocr_pages` request still returns an error when every requested page fails; automatic per-page OCR keeps the native result. (GH#1925)
- **(ocr): PDF pages rendered for OCR now lower their DPI before exceeding the configured content-size limit.** The render chooser uses the same live-byte accounting as the later PNG encode check and reports the page and applied cap, so a supported high DPI or a legal-size scan no longer aborts extraction under the default limit. (GH#1924)
- **(ocr): PDF page rasters are no longer OCR'd a second time as embedded images.** Enabling `images.include_page_rasters` now keeps the raster in the result without appending a duplicate copy of the page's OCR text to document content. (GH#1929)
- **(ocr): right-aligned amount columns of mixed width in a scanned table keep their values apart.** When a one-digit amount of one column starts close to where an eight-digit amount of the next column starts, column detection put both into one column. Two amounts of one row then shared a cell, and the table was dropped or showed values in the wrong column. A column that holds two cells of one row is now split by right edge, and a header label no longer stops the value tracks under it from folding into one column. (GH#1909)
- **(tables): an amount column with thousands separators is cleaned up like a column of plain digits.** Table cleanup empties a nil dash and joins `- 5,000` to `-5,000` only in a column it reads as numeric, and it did not read `40,218,965` as a number. Two amount columns of one table could therefore show the same nil dash as empty in one and as `-` in the other. Comma-grouped amounts such as `1,234,567` and `1,234.50` now count as numbers; `1,2` and `12,34` still do not. (GH#1914)
- **(ocr): numbered row labels no longer make table reconstruction discard the whole table.** OCR can split labels such as `Item 1` into a word column and a narrow numeric tail column, making the tail look like an independent field and causing an otherwise complete table to be rejected. A short numeric tail is now folded into its adjacent label only when every row resolves to the same repeated label stem; genuine numeric and amount columns remain separate. (GH#1928)
- **(ocr): a dark scanned PDF page now gets the same default OCR preprocessing as a lighter page in the same file.** With no `tesseract_config` set, the OCR step judged every image by its pixels alone -- mean luminance and near-white pixel fraction -- to decide whether to apply the default resample to 300 dpi, binarisation and deskew. A scanned page with shaded table rows or a grey background failed that test and reached Tesseract at render resolution with no binarisation, while a lighter page in the same document was preprocessed and read its table values correctly. The PDF OCR route already knows which pages are whole-page scans from its own scan-detection check; that signal now reaches image preparation directly, so a known scan page always takes the default preprocessing regardless of how dark it reads. A bare image handed to the standalone image extractor, which has no such signal, is unaffected and still judged by its pixels. (GH#1894)
- **(ocr): the OCR retry on a scanned page's embedded image uses the same Tesseract settings as the page.** When the OCR of a whole PDF document (`force_ocr`, or a document that needs OCR throughout) comes back blank or fails on a page, xberg retries OCR on the page's embedded images, and that retry used the configured OCR settings unchanged. The embedded image of a page that is one full-page scan therefore reached Tesseract without the whole-image page segmentation mode and without the scan signal that gives the page the default preprocessing, although the same page gets both when it renders. The retry now takes both on a scan page. A configured `psm` or `preprocessing` still wins, and a page that is not a scan and the other OCR backends are unchanged. (GH#1907)
- **(ocr): OCR of selected pages retries a blank or failed page on its embedded images.** When the OCR of a page comes back blank or fails, the whole-document OCR route retries OCR on the page's embedded images. The per-page route did not. That route runs for `force_ocr_pages`, for pages that scan detection marks as scanned, and for the per-page OCR fallback. With the default settings it had no retry, and with a `pipeline`, a `vlm_fallback` policy or layout detection the retry found no document to read the images from. So a scanned page that the renderer could not draw came back blank, with no warning. The per-page route now retries in every setting and adds the same warnings as the whole-document route. A failed page that the retry cannot recover still fails the extraction. (GH#1912)
- **(ocr): the `psm` documentation of `TesseractConfig` describes the default for a scanned PDF page.** It said that a rendered PDF page always gets the engine's automatic-layout mode and suggested setting `psm: 11` for table pages. Since GH#1786, a page that is one full-page scan gets the whole-image mode that image OCR uses when no `psm` is set. Only a page that is not a scan keeps the engine default. The Rust docs, every binding's docs and the API reference now say this. (GH#1911)
- **(pdf): a colour-key `/Mask` on an `/Indexed` image is applied again.** A colour-key mask states its ranges in the image's raw index space, but extraction expands an Indexed image's indices to RGB for display and always cleared the flag the renderer's colour-key path used to decide whether masking was still possible, so the mask was silently skipped and pixels the file marked transparent were painted with their palette colour instead. The raw index plane is now kept alongside the palette-expanded RGB, and the colour-key path reads it for an Indexed image rather than giving up. (GH#1899)
- **(pdf): an `/Indexed` image now reaches the separation-plate renderer.** The plate classifier had no case for `/Indexed`, so a palette image resolved to `Unknown` and was skipped before it was even decoded, whatever ink intent its palette's base colour space carried -- a `DeviceCMYK`-, `Separation`- or `DeviceN`-based palette painted no plate at all. An `/Indexed` colour space is now classified by its base, and when that base carries ink intent the palette lookup's own base-space bytes -- not the RGB the renderer produces for on-screen display -- route straight to its plates. (GH#1898)
- **(pdf): `counts.pages` now reports a PDF's real page count instead of the number of pages that happened to produce content.** The count came from page-boundary tracking, which only runs when an explicit `pages` config or one of a handful of OCR settings switches it on, and which in any case only counts pages that yielded text. A scanned PDF extracted with `disable_ocr` therefore reported `0` pages, and a PDF with one native page beside one scanned page reported `1` of its `2`, while the same result's PDF metadata carried the correct count throughout. The format's own page count, read from the page tree independently of any tracking, is now a floor on the reported number. (GH#1888)
- **(pdf): a JPEG 2000 image whose dictionary `/Width` or `/Height` disagrees with its codestream now extracts at the codestream's size.** A `/JPXDecode` image's pixel buffer is always sized to the codestream, but the image reported the dictionary's declared dimensions regardless -- a smaller declared size read a scrambled prefix of the buffer at the wrong stride, and a larger one made the buffer come up short for anything that trusted `width * height * components`, so the image came out silently garbled or dropped, with nothing recorded either way. ISO 32000-1:2008 §7.4.9 treats `/Width` and `/Height` as informative for JPEG 2000, so the codestream's own `SIZ` marker segment is now authoritative for both, and a disagreement between the two now raises a warning. (GH#1900)
- **(ocr): a table on a resampled scanned page is placed inside the page instead of spilling off it.** The mixed-OCR route converted a table's bounding box from raster pixels to PDF points using the size of the page render, but OCR preprocessing resamples a sub-300 dpi render up to `target_dpi` before recognition, so the box comes back in the resampled frame. Dividing by the render's size overstated every table box by exactly that ratio -- a table on a 150 dpi render came out at twice its size, with part of it outside the page. The element boxes on the same route already resolved the processed frame, which is why only tables were affected. (GH#1895)
- **(ocr): `layout.table_model = "disabled"` now turns off table recognition on the force-OCR PDF route too.** The route gated TATR on whether a layout pass had run and nothing else, so the setting was honoured on the native PDF route and for standalone images and silently ignored for every `force_ocr` extraction. The SLANet values are unchanged on this route: they are wired only on the native route, so treating them as "not TATR" here would remove table recognition outright rather than substitute a model. (GH#1813)
- **(ocr): a table a backend reported itself is now placed using the pixel frame its coordinates are in.** The bounding box was rescaled to PDF points using the page render's dimensions while the backend's coordinates are in its own processed-image space, which on a Tesseract page is roughly twice as large in each axis, so the table landed at about half its true position and size. The formula path on the same route already resolved the processed frame; the table path now does the same. (GH#1813)
- **(ocr): a page whose OCR word boxes cannot be mapped into the render raster now yields its text as prose instead of an empty table.** The transform answered three untrustworthy metadata shapes -- no processed dimensions reported, an auto-rotation with no usable orientation, and a raster that is not a scaled copy of the render -- by handing back untransformed boxes. Those sit outside the render-space table bounding box, so the cell-overlap filter discarded every one of them and the region published a fully shaped table holding none of the page's values. Such a page now skips table recognition and records a warning, and its words stay in the paragraph stream. A scale pair whose two axes disagree by more than 1% is rejected under the same tolerance the standalone-image route has always applied. (GH#1813)
- **(ocr): `numeric_repair` now runs on whole-document OCR, not only on per-page OCR.** GH#1840 extended the repair across the mixed OCR/native route's outputs, but the whole-document route -- the one `force_ocr`, the near-empty `Auto` fallback and the OCR gate's whole-document fallback all take -- had no repair at all, so `{numeric_repair: true, force_ocr_pages: [1, 2]}` repaired those pages while `{numeric_repair: true, force_ocr: true}` repaired nothing from the same flag. That route's flat text, structured document and reconstructed tables are now repaired too, under the same opt-in flag and the same markup-renderer exclusion, and ahead of formula recognition so recognised LaTeX is not re-punctuated. The repair's documented limitation is unchanged and now reaches this route as well: a bare four-digit year with no `FY` prefix is grouped as a thousands value. (GH#1892)
- **(ocr): the Tesseract language probe now asks for the languages the caller configured, and its memo tells apart callers that resolve different directories.** GH#1857 threaded `OcrConfig.tessdata_path` into the probe but left it asking for English, and a candidate tessdata directory is accepted only when it holds every language asked for -- so a `tessdata_path` holding just the configured language was rejected, resolution fell through to `TESSDATA_PREFIX`, the xberg cache and the system paths, and `supports_language_for`/`supported_languages_for` denied a language every real job with that config loads. The probe now resolves with the config's effective languages, and only against directories that already hold them, so a capability query still cannot trigger a language-pack download. Its memo is keyed on those languages together with the override, `TESSDATA_PREFIX` and `XBERG_CACHE_DIR` rather than on the override alone, so two callers whose resolution differs in any of them no longer serve each other's answer. `supports_language` without a config now probes for the language it was asked about; `supported_languages` without a config still answers for the English datapath. (GH#1891)
- **(pdf): an implausible text page the automatic OCR fallback will not route now always reports a warning.** The warning's condition was a second, re-derived form of the predicate that decides whether the near-empty `Auto` fallback runs. The two agreed until `ocr_near_empty_fallback` gained an explicit `false` (GH#1752): with an `ocr` block and the fallback switched off, pages flagged as reading like no real language were not routed to OCR *and* the warning saying so was suppressed, so the caller received wrong-mapped text with no diagnostic at all. `ocr_near_empty_fallback: true` with no `ocr` block, on a build with no registered automatic OCR backend, diverged the same way. The warning now calls the routing predicate itself rather than re-deriving it, and its text no longer names a missing `ocr` block as the only possible cause. (GH#1890)
- **(ocr): a right-aligned amount column is no longer split into two or three columns by digit count.** Table column detection grouped every word by its left edge, but an amount column is right-aligned: `5`, `73` and `1,234,567` share a right edge and their left edges differ by far more than `table_column_threshold` at 300 dpi. One column minted several, and what the existing disjoint-numeric-column merge did not fold back came out as a dropped trailing nil-dash column, a sparse row's missing interior cells, or a value in the label cell. Adjacent columns whose every word reads as a value -- a number, or a symbol a cell prints alone such as a nil dash -- now fold into one when their right edges coincide within the same threshold. The threshold itself is unchanged, since widening it merges genuinely narrow neighbouring columns. Folding changes only how many columns the grid has: a folded column keeps the left edge of every track it absorbed and still decides which words belong to it on the nearest of those, so no word moves between columns. (GH#1886)
- **(ocr): a reconstructed table no longer replaces its region's lines when it dropped their words.** On the Tesseract route every hOCR line under a detected table's bounding box was removed from the page document unconditionally, with no check that the table's cells still carried those lines' words. A reconstruction that splits a wrapped label across two rows, glues a value onto a label, or loses the interior cells of a sparse row therefore deleted those words from the page: they were in neither the paragraphs nor the table. A table now claims its region only when its non-empty cells retain at least as many words as the lines it would remove -- the same absolute word-count rule the standalone-image rebuild already applied -- and a table that lost words leaves those lines in place while still being published in `tables`, so the structured form is never lost either. The cost for such a region is that its text appears both as paragraphs and as a table. (GH#1884)
- **(pdf): a bare CMYK JPEG 2000 codestream in an image that names no `/ColorSpace` keeps its black plane.** ISO 32000-1 Table 89 lets a `/JPXDecode` image omit `/ColorSpace`, and such an image is given a placeholder `/DeviceRGB` until the codestream reveals its real component count. That placeholder was handed to the decoder as though the document had declared three components, which agreed with the decoder's own reading of a bare four-component codestream as RGB plus alpha, so the fourth plane was discarded as transparency: CMY was painted as RGB on the render path, and the image was dropped from every plate on the separation path. The component count now comes from the codestream's own `SIZ` header when the dictionary declares nothing. A bare codestream cannot mark a component as alpha -- that requires the JP2 channel-definition box -- so all of its components are now read as colour components; the consequence is that a bare four-component codestream that really was RGBA, in an image declaring no `/ColorSpace`, is read as CMYK and loses its alpha. A PDF carries transparency in `/SMask`, not in the codestream. Images that declare a `/ColorSpace`, and JP2-boxed streams, are unaffected. (GH#1883)
- **(pdf): a JPEG 2000 image with an `/Indexed` colour space now renders and extracts through its palette.** The decoder applied the palette box inside the JPEG 2000 data and failed the whole image when lossy coding pushed an index sample outside that palette, so a scanned page stored this way rendered blank, and image extraction dropped the image. ISO 32000-1 §7.4.9 puts the dictionary's `/ColorSpace` in charge, so the decoder now reads the raw indices, clamps each to the last entry of the palette, and looks it up in the dictionary's `/Indexed` palette, as it already did for other index streams. An `/Indexed` JPEG 2000 image with no palette box of its own was painted as greyscale index values and is now looked up too. An `/Indexed` JPEG 2000 image the decoder did resolve used to extract as CMYK and now extracts as RGB, like every other `/Indexed` image. An opacity channel in the JPEG 2000 data is dropped, as it already was. Such an image extracts at the JPEG 2000 data's own size when the dictionary's `/Width` or `/Height` disagrees, and a colour-key `/Mask` on it is tested against its indices. (GH#1885)
- **(pdf): a lossy palette JPEG 2000 image with no `/Indexed` colour space now decodes.** When the JPEG 2000 data carries its own palette and the dictionary names no `/Indexed` colour space, the decoder resolved that palette with an unclamped lookup, so one index sample that lossy coding pushed past the palette failed the whole image and the image was dropped. xberg now reads the palette from the JPEG 2000 header and resolves the index plane through the same clamped lookup it uses for `/Indexed` images. The image keeps the colour space of the palette, so a CMYK palette extracts as CMYK and paints the process plates. The separation renderer now uses that decoded colour space when the image dictionary omits `/ColorSpace`, instead of classifying the image as unknown and skipping it before decoding. This applies when the dictionary names no `/ColorSpace`, or names a device colour space with the palette's component count. A declared colour space that the palette does not fit still fails as before, and a palette with entries deeper than 8 bits keeps the decoder's own lookup. (GH#1903, GH#1922)
- **(pdf): an expanded `/Indexed` image reports 8 bits per component.** After an `/Indexed` image's indices were expanded to 8-bit colour samples, `bits_per_component` still reported the dictionary's 1, 2 or 4-bit depth, so the field disagreed with the pixel data. It now reports 8, for uncompressed index streams and for `/Indexed` JPEG 2000 images. (GH#1901)
- **(pdf): a transparent pixel of a drawn image shows the page under it.** The renderer drew each image into the page with its opacity stored beside unchanged colour, but the page canvas expects colour already scaled by opacity. A pixel that a soft mask (`/SMask`) or a colour-key `/Mask` made transparent therefore added its own colour to the page under it: a white image with a transparent region painted that region white on any background, and a half-transparent pixel came out too light. xberg now scales each image pixel's colour by its opacity before drawing it, and an image drawn smaller than its own size is resized with that scaled colour. (GH#1905)
- **(pdf): a stencil image mask no longer paints its whole rectangle on a separation plate.** The separation renderer drew an `/ImageMask` stencil with the ink tint as unscaled colour beside the stencil's opacity, but the plate canvas expects colour already scaled by opacity. Each pixel the stencil leaves transparent therefore added the full tint to the plate under it, so the whole rectangle of the mask read as ink. xberg now scales the tint by the stencil opacity, and the plate under an unmarked pixel keeps its value. (GH#1906)
- **(pdf): a JPEG 2000 image with `/SMaskInData` 1 or 2 now renders with its own transparency.** When the image dictionary sets `/SMaskInData` to 1 or 2, the opacity channel inside the JPEG 2000 data is the image's soft mask, but the renderer dropped that channel and drew the image opaque. xberg now keeps the channel and applies it as the soft mask when the image has no `/SMask` entry. An `/SMask` entry still takes precedence, and `/SMaskInData` 0 or absent leaves the channel unused as before. Image extraction is unchanged. (GH#1902)
- **(pdf): a colour-key `/Mask` on a 1, 2 or 4-bit image, or on an image with a `/Decode` array, is now applied.** The ranges of a colour-key `/Mask` are written in the image's own sample values, but the extractor unpacks 1, 2 and 4-bit samples to 8 bits and folds a `/Decode` array into them, so the renderer skipped the mask and painted the pixels the file marks transparent. xberg now keeps the raw samples of such an image beside the unpacked ones and tests the mask against them, for every colour space. A colour-key `/Mask` on a 16-bit image is still skipped. (GH#1904)
- **(pdf): an `/Indexed` palette over a DeviceN colour space is now read one byte per colorant.** The extractor read each palette entry as four bytes whatever the number of colorants, so a palette over a DeviceN space with two, three, five or more colorants mixed neighbouring entries. Each entry now holds one byte per colorant the space names, and such an image paints its colorant plates. (GH#1913)
- **(pdf): a JPEG 2000 `/Indexed` image now paints the separation plates.** The separation renderer looked up the palette only for an uncompressed index stream, so a JPEG 2000 coded `/Indexed` image left every plate empty. Its index plane is now looked up in the palette the same way, at the JPEG 2000 data's own size. (GH#1916)
- **(pdf): a spot ink used only through an `/Indexed` palette now gets its separation plate.** The plate list for a page comes from the spot inks its colour spaces name, and that list never looked inside an `/Indexed` colour space. An image whose palette is over a `/Separation` or `/DeviceN` space therefore got no plate for those inks, and its ink was lost. The base of an `/Indexed` colour space now names its inks like any other space. (GH#1915)
- **(docker): images no longer share OCR cache entries across builds at the same crate version.** The cache key folds in a per-build discriminator derived from `git rev-parse HEAD`, but `.dockerignore` excludes `**/.git/`, so the derivation always failed inside a Docker build and the discriminator fell back to an empty string -- collapsing every image and published artifact built at one version into a single cache key space, including a `force_republish` rebuild at an unchanged version. Every Dockerfile that compiles Rust -- the `core`, `full` and `cli` images, and the eight that build published artifacts (the musl `xberg-cli` and `cargo binstall` binaries, the musl Python wheel, `libxberg_ffi` for the Java/C# natives, the musl and manylinux Elixir NIFs, and the musl and manylinux Node addons) -- now takes `XBERG_BUILD_ID` as a required build argument and fails the build rather than default to empty; the CI and publish workflows resolve it from the commit being built and pass it through. (GH#1845)
- **(ocr): `ocr_near_empty_fallback` and `ocr_scanned_page_quality_gate` now get the per-page byte offsets their routing needs, even without an `ocr` block.** The text pass only tracked page boundaries when `force_ocr_pages` was set or an `ocr` block was present, so a caller who set either of the two settings introduced for GH#1752 alone got no per-page offsets at all: the per-page quality gate silently collapsed to one whole-document verdict, and a single flagged page escalated to whole-document OCR instead of routing just that page. Boundary tracking now also turns on for either setting. (GH#1752)
- **(pdf): per-page hierarchy blocks report the measured font size instead of a hardcoded 12pt.** `PageContent.hierarchy.blocks[].font_size` was set to a literal `12.0` for every heading and paragraph block, even though the PDF structure pipeline already computes each paragraph's real dominant font size -- it was read into a debug log and then discarded. Each block now reports the font size actually measured for its element, falling back to 12pt only when none was recorded. (GH#1844)
- **(pdf): a JPEG 2000 CMYK image that names no `/ColorSpace` now picks up the document's `/OutputIntents` profile.** ISO 32000-1 Table 89 lets a `/JPXDecode` image omit `/ColorSpace`, and such an image is given a placeholder `/DeviceRGB` until the codestream reveals its real component count. The ICC decision was made from that placeholder and never redone, so a four-component codestream was corrected to `/DeviceCMYK` but reached extraction and rendering with no profile, and its colour was converted through the §10.3.5 additive clamp instead of the document's press characterisation. The same image declaring `/ColorSpace /DeviceCMYK` was always handled correctly. (GH#1839)
- **(pdf): an image handle for an `/ImageMask` with no `/ColorSpace` now reports `DeviceGray`, matching the decoded image.** `/ColorSpace` is forbidden on an image mask (§8.9.6.2), and extraction has always defaulted such an image to `/DeviceGray` before decoding it. The cheap, pre-decode image-handle enumeration used a plain `/DeviceRGB` default for any image with no `/ColorSpace`, so a mask's handle disagreed with the image `decode()` later produced from the same XObject. Both the XObject and inline-image handle builders now default a missing `/ColorSpace` the same way extraction does. (GH#1825)
- **(ocr): `numeric_repair` now reaches a scanned PDF page's structured output, not just its flat text.** On the PDF mixed-OCR route the repair was applied to each page's flat OCR string, which is what a Plain-format extraction renders -- but every other renderer rebuilds each OCR'd page from its structured per-page document, and every table on every renderer comes from there, so Markdown, HTML and JSON output and all reconstructed table cells kept the unrepaired numbers. The per-page documents' element text, table cells, header columns and table markdown are now repaired too, under the same opt-in flag and the same markup-renderer exclusion as before. An element carrying a partial-range bold or italic annotation is left unrepaired, because the repair changes byte offsets the annotation is anchored to. (GH#1840)
- **(ocr): the Tesseract language-availability probe now honours a configured `tessdata_path`.** `supports_language` and `supported_languages` always probed the no-override tessdata search chain, even when the caller's `OcrConfig.tessdata_path` names a different directory -- so a backend reported as unable to handle a language a real OCR job, which does honour that override, loads without trouble. The probe now takes the same override a job passes, and its language memo is keyed by it so two configs with different overrides do not serve each other's answer. `supports_language`/`supported_languages` and `list_ocr_backend_capabilities`/`ocr_backend_supports_language` keep their previous no-override behaviour; use the new `_for` forms when a configured `tessdata_path` should be honoured. (GH#1857)
- **(pdf): the page-count and page-render entry points reject a wrong PDF password instead of ignoring it.** `xberg_native_pdf`'s `authenticate` reports a rejected password as `Ok(false)` rather than as an error, and this path mapped only the error and discarded the bool, so every wrong password authenticated silently -- `pdf_page_count` and `render_pdf_page_to_png` would read an encrypted document with any password at all, including from a binding that mangled the argument in transit. The document extraction path always read the bool and is unaffected. Supplying no password is still not an error: a PDF's page tree is structure rather than encrypted data, so the count reads without authenticating. (GH#1879)
- **(api): an optional reference the JSON contract omits is described in the OpenAPI document as a direct `$ref` rather than `oneOf[$ref, null]`.** utoipa types every `Option<T>` as a union with `{"type": "null"}`, but 41 of the document's 45 optional references carry `skip_serializing_if = "Option::is_none"`, so they are absent from the payload rather than null; the union also pushed each field's description onto a branch, where an OpenAPI generator does not read it. Those 41 properties are now direct references carrying their own description, and they stay out of `required`. The four that genuinely serialise `null` -- `ElementMetadata.coordinates`, `DocumentRevision.anchor`, `ImageMetadataType.dimensions` and `ImagePreprocessingMetadata.new_dimensions` -- keep the union, and inner primitive nullability such as `ExtractionConfidence.ocr_aggregate` is unchanged. Wire payloads do not change, and no generated binding's types change, because binding optionality is derived from the Rust `Option<T>` and not from the schema annotation. A client generated from the published spec will see these fields lose their nullable wrapper while remaining optional. (GH#1841)
- **(ocr): a table region whose OCR text was all filtered out no longer publishes an empty table.** Table-structure recognition decided between emitting a table and falling back to the region's raw text on the reconstructed cell grid's *geometry* alone. When the overlap filter that assigns OCR elements to a region discarded every one of them -- its test is against the whole table bounding box, so a region whose word boxes and rasterised crop disagree about coordinates loses all of them -- a geometrically valid grid still produced a fully shaped table of empty strings whose markdown is non-empty, because a header separator alone is not empty. The caller published that table, and because the outcome counted as recognised, the text-preserving fallback added for GH#1622 never ran, so the region's words were absent from the table *and* from the unrecognised-text output. Such a region now emits no table, leaving its words in the paragraph stream. (GH#1813)
- **(pdf): a JPEG 2000 image with an alpha channel extracts in its own colour space instead of being mis-typed or dropped.** The decoder took the codestream's raw component count as the colour-component count, and an alpha channel is a component like any other: an RGBA codestream reported four components and was mapped to DeviceCMYK, and a greyscale-with-alpha codestream reported two and was rejected outright, so the image never reached the page. The codestream's own alpha flag now decides, and the alpha channel is dropped rather than counted -- carrying it through as a soft mask is a separate change. Covers both the JP2 container and the bare codestream form. (GH#1850)
- **(pdf): a multi-page forced-OCR extraction no longer loses every render warning.** The engine's render-warning capture buffer is per-thread, and the `force_ocr` and `force_ocr_pages` routes render each batch across a rayon pool. A batch of one page does not split, so it rendered on the calling thread and its warnings were collected; a batch of two or more was handed to pool workers whose buffers nothing drained. The batch size is the resolved thread count, so on any multi-core host every document of two or more pages silently dropped missing-glyph-ink, blank-image and fallback-font notices, while the same document extracted with one thread kept them. Each page's warnings are now drained on the thread that rendered it and merged back for the caller, matching what the layout-detection route already did, and in page order, so the same input reports the same warnings on every run. Capture is opt-in and unchanged: a caller that never installs the render diagnostics is unaffected. (GH#1847, GH#1851)
- **(ocr): an invalid candle `backend_options` or `paddle_ocr_config` now fails the extraction before any page runs.** These settings were parsed by the backend on each page, so on the automatic OCR route a rejected value became a per-page failure, and the extraction returned the native text with only a warning. Configuration validation now runs each backend's own check, for the configured backend and for every pipeline stage, so the caller gets a validation error like any other invalid setting. This is a behaviour change: the error now comes even for a document that needs no OCR. A page that fails at run time, for example on a security limit, still returns the native text with a warning. Options for a custom plugin backend are still checked only on each page. (GH#1829)
- **(ocr): values in a shaded table row no longer glue into one cell across underscore marks.** Tesseract reads the edge of a shaded row as runs of underscores and fuses them onto the values beside it, or into one word that spans two values. The fused word's box then covered the gap between two columns, so two values landed in one cell and the cell kept the underscores. Before table reconstruction, each word is now cut at a leading or trailing underscore run and at an interior run of two or more, and each piece keeps the part of the box its characters cover. A single underscore inside a word stays. The OCR text outside tables is unchanged. (GH#1833)
- **(ocr): values in a shaded table row no longer glue into one cell across a stray mark.** Tesseract also reads the edge of a shaded row as a word of its own, a tall `=` or a thin dash, in the gap between two values, and the cell merge then joined both values into one cell. Before table reconstruction, a word in a Tesseract table region is now dropped when its height is under a quarter or over 1.8 times the median height of its row and its confidence is under 35. Both tests must hold: a value read at a low confidence at normal height stays, and so does a thin dash read at a moderate confidence, such as the dash between the two dates of a statement period. The OCR text outside tables is unchanged. (GH#1858)
- **(ocr): a word of a multi-word row label no longer starts a table row of its own when shading stretches its box.** Shading can stretch the box Tesseract reports for one word over the next row, so the centre of that box passed the row threshold and the word left the row of its label. Before table reconstruction, the words that Tesseract reads on one text line now take the vertical box of the line's word of typical height, so they share one row. The reported table bounding box uses the same boxes. The native PDF table path does not change. (GH#1834)
- **(ocr): a scanned table is no longer discarded because a misread row minted a phantom column.** Table post-processing already folded away a column whose data cells were *entirely* empty, but a column that was merely almost empty had no such path and instead reached the column-density gate, which rejects the whole table. On a scanned page that is routine: a shaded row read as a few stray glyphs lands at x-positions that mint a header-less column, and on the reported page three stray cells out of twenty-two rows were enough to discard an otherwise well-formed 23-row grid, at every segmentation mode and with or without shaded-row normalisation. Such a column is now folded into its neighbour, carrying its cells' text rather than dropping it. This is a rescue path only: it is reached solely for a column the density gate is about to reject on, and a table containing one returned nothing at all before. A column with its own header is still treated as a legitimately sparse column and left alone. (GH#1797)
- **(ocr): the embedded-image OCR retry stops claiming the rasterizer failed, and stops overwriting better text.** The retry fires both when a page's raster came back blank and when a perfectly-drawn page simply yielded no OCR text, but its warning reported every case as image XObjects "the PDF rasterizer could not draw" -- sending a reader looking for a drawing bug that had not happened. Two of the repository's own tests encoded that wording against fixtures whose pages rasterize correctly. The trigger now reports the raster verdict separately and the warning names the cause that was actually measured. Second and more serious: the retry's text replaced the page's own whenever it was merely non-empty, and the retry fires for a page carrying up to 200 characters of real text, so one character of noise from an embedded thumbnail could silently overwrite a terse but correct transcription. The retry's output is now adopted only when it reads more than the page attempt did. The last-resort pipeline route, which runs after every OCR stage has already failed and has no page raster to inspect, keeps the previous wording rather than infer a signal it never measured. (GH#1826)
- **(ocr): a two-word column header split across two columns no longer shifts every value in a scanned table.** When the gap inside a header like `Year 1` reaches the cell-merge threshold, OCR reports the two words as separate cell tokens and each mints its own column; the right-hand one holds a header fragment and no data. Post-processing already folded such a column away, but only while its data cells were *entirely* empty -- and a single stray glyph from a shaded row defeats that, so the phantom column survived and every value in the table sat one or more places left of the column it belonged to, while the OCR itself had read it correctly. A column whose header is non-empty and which carries text in at most one row in ten is now folded into its neighbour, header and cells together, so `Year` and `1` rejoin as one column. On the reported page this moves an absolute 18 of 138 known values into the correct cell to 98, and 36 to 102 under sparse-text segmentation; the 36 that remain are rows whose *label* OCR mangled, not values in the wrong place. The bar is deliberately far below a legitimately sparse labelled column -- the bank-statement `DEPOSIT` column of GH#1649 carries two rows in nine -- but unlike the header-less case this does restructure a table that was accepted before, since the column-density gate exempts a column with its own label. (GH#1832)
- **(ocr): PDF table-structure recognition no longer runs on the async runtime's worker threads.** Checking a TATR model out of the shared pool can block on a `std::sync::Condvar` -- for pool exhaustion, or behind a competing model load -- and on a miss it performs model-file I/O and ONNX Runtime session construction; the recognition call itself is synchronous CPU work with no await points. Both ran directly on a tokio worker thread, which parks that thread rather than yielding the task, so on a small worker pool several concurrent layout-enabled extractions could occupy every worker in native blocking calls with nothing left to poll the task that would release a model back to the pool. Both now run through `spawn_blocking`, the same boundary the RT-DETR layout engine has always used. Nothing is serialized and no pool capacity changed. Reported as an indefinite stall on two concurrent layout-enabled extractions of a scanned PDF (GH#1812), which remains open until the reporter's reproduction is re-run against a build carrying this change.
- **(ocr): `numeric_repair` no longer rewrites the coordinates in hOCR and TSV output.** `TesseractConfig::output_format` selects which renderer the backend returns, and under `"hocr"` or `"tsv"` the OCR text *is* the markup -- so the repair's thousands-separator rule re-punctuated the bare integers inside every `bbox` and every TSV coordinate column, turning `bbox 1234 567` into `bbox 1,234 567`. At the default 300 dpi a Letter page is 2550x3300 px, so four-digit coordinates are the norm rather than an edge case, and a downstream hOCR or TSV parser then read a truncated coordinate or failed outright. Both the standalone-image and the PDF route now skip the repair for a markup renderer; the recognized text in elements and table cells is repaired as before. The three shapes the repair rewrites that are not amounts are now documented on the field rather than left to be discovered: a bare 4-9 digit integer is grouped whatever it denotes (`Year 2019` becomes `Year 2,019`, and likewise a ZIP code, page number or part number, with only a literal `FY ` prefix exempt), a US-convention decimal with exactly three fraction digits becomes an amount (`0.125` becomes `0,125`), and a lone digit before an already-grouped number is joined (`5 12,000` becomes `512,000`), which is indistinguishable from two adjacent table cells. The option remains off by default. (GH#1836)
- **(ocr): `normalize_shaded_rows` says so when it cannot take effect.** The per-band step operates on the grayscale conversion, but with `binarization_method` set to `"none"` or `"off"` *and* `contrast_enhance` on, the non-binarized contrast path re-reads the original image and discards that result -- so the option did nothing at all, silently. That combination now logs a `WARN` naming the reason, and the field documents the limitation. The processing itself is unchanged on every path. (GH#1837)
- **(pdf): a two-column page whose table spans the full width no longer reads column by column.** The corridor search that rescues a split landing inside a table is scoped to the band around that table, so on a page whose table spans the full width the only corridors in scope are the gaps between the table's own columns -- and the search moved a split that was already the page's gutter into one of them. Every prose line below the table then straddled the new split, and the two columns came out interleaved line by line. A split is now kept when a real population of lines outside the band agrees it is already the gutter, measured as both a count and a rate: on a page where the band is the whole page there is nothing outside it to consult, and a disagreement count of zero there means nothing was examined rather than that everything agreed. (GH#1801)
- **(pdf): a full-width figure legend written as several font runs no longer tears at a two-column XY-cut split.** The column cut assigned each font run to a side by its own left edge, so a legend line broken into fragments at its own run boundaries -- a bold label, a kerned superscript, a `Table 2` prefix -- had its later runs cut off and emitted after the opposite column's body instead of staying with the rest of the line. A contiguous run of full-width lines is now peeled off as its own band before the column cut runs at all, and any full-width line that still reaches the cut is assigned whole, to whichever side holds the majority of its inked width, rather than split at the cut point. An ordinary two-column page's own full-width furniture -- a title, an abstract header, a running header, a footer -- is unaffected: peeling requires at least two such lines in a row, not one. (GH#1808)
- **(pdf): a sparse table beside a prose column no longer redirects a misplaced column split into the table's own cell gap.** When a split lands inside a column, the rescue search that moves it to the nearest empty corridor picked the WIDEST corridor with no further check, and a sparse table's own cell gap is routinely wider than the page's real gutter (a row that fills two of five columns opens a gap between them wider than any gutter). The redirect already rejected exactly this shape -- a hanging-label indent, or a corridor whose flanks do not both read as columns -- but only on its second, narrower search; the first, wider search applied neither check. Both searches now share the same qualification, so the true gutter is chosen even when a table's own gap is the wider candidate. (GH#1800)
- **(pdf): a two-column page's last line before a short band no longer welds to the next column's first line.** The paragraph grouper decided whether to start a new element from font, weight, role, list markers and vertical spacing alone -- nothing tested whether two candidate lines stood on opposite sides of the page's own column gutter. A left column's closing line and the right column's opening line, whose boxes happened to overlap a couple of points in the vertical direction, were merged into one full-width element spanning both columns. Two lines with no horizontal overlap at all, on either side of the page's single column corridor, now always start a new element; a full-width line (a title, a caption, a footer) straddles the corridor by construction and is unaffected. Recognising such a page as two columns in the first place no longer depends on each line being one span: a producer that writes one `Tm`/`TJ` per word used to contribute no prose-line evidence at all, because that test was applied per span rather than per visual line. (GH#1806)
- **(pdf): a two-column page whose second column is short no longer welds the two columns' headings together and reads row by row.** Two independent gates refused the page, and fixing either alone left the other defect in place. The column-gutter detector's three sub-detectors all measure MASS -- what share of the page's spans cluster at each left edge, central density, a whole-page classifier verdict -- and a right column that fills only its top third presents none of it, so the detector returned nothing even though the corridor between the columns is empty from the top of the page to the bottom. The XY-cut's heading-run pre-pass is inert without a gutter, so it folded the right column's bold heading into the left column's chapter heading and emitted one list item spanning both columns. A fourth detector now asks only what a gutter *is*: the page's sole empty x-interval, with a real population of spans on each flank, and narrow enough relative to the content it splits that a label/value form's much wider corridor does not qualify. It declines outright on a page carrying two or more multi-column table rows, so a grid's own recurring cell gaps -- which are empty x-intervals too -- are never read as a column corridor. Separately, the reading-order selector required the two sides' prose-line counts to stand within a ratio of each other, and refused this page at 10 lines against 73. A short second column is an ordinary layout -- a sidebar, a declaration, a signature block beside running text -- and how much text a side holds says nothing about whether the two sides are columns; the vertical-overlap test immediately after it already establishes that they stand beside each other, so the ratio is removed rather than loosened. (GH#1809)
- **(ocr): a scanned PDF page is OCR'd the way the same page is OCR'd as an image.** Under `force_ocr` and `force_ocr_pages`, a page that is one full-page image rendered at 150 dpi and ran Tesseract in automatic layout mode, while the same page as a PNG kept its own resolution and ran in sparse-text mode. On a scanned table the difference was table values: the automatic mode dropped values the sparse mode reads, and rendering a 196 dpi scan at 150 discarded detail that the OCR preprocessor then interpolated back. Such a page now renders at its image's own density, kept between 150 and 300 dpi, and uses the sparse-text segmentation mode when the caller sets no `psm`. A scan counts as such when its raster covers at least 80 percent of the page, or at least a quarter of it while the page's own text layer is no more than a stamp of 400 glyphs, which is how a scanned sheet painted inset with margins looks. Every other page keeps the 150 dpi default and the automatic mode, a configured `images.target_dpi` still decides the render resolution, and an explicit `tesseract_config.psm` is never overridden. `tesseract_config.preprocessing.target_dpi` now says that it resamples the OCR image and does not change the render. (GH#1786)
- **(ocr): a scanned page gets the same OCR render density and segmentation mode with layout detection on as with it off.** GH#1786 made a scanned PDF page render at its own raster density and use the sparse-text segmentation mode, but only fixed the routes that render pages themselves. With layout detection on, OCR runs against the layout pass's own rasters, which still rendered at the 150 dpi default and left the page on the automatic segmentation mode. Both routes now share the same scan-density render decision and the same whole-page-raster check, so a scanned page's OCR defaults no longer depend on whether `--layout` is set. (GH#1828)
- **(pdf): unmapped Type 3 text no longer hides behind a short readable header.** Extraction retains one visible placeholder per painted glyph whose font has no Unicode mapping, while blank `d0`/`d1` CharProcs become whitespace. The fabricated-text gate can now route a page dominated by those glyphs to OCR without counting blank spacing procedures. A synthetic two-header page has 130 painted fallback glyphs and 126 readable non-whitespace characters and routes to OCR; a six-header control with 378 readable characters remains native. (GH#1782)
- **(pdf): a table drawn with horizontal rules only is no longer lost when the same page has a fully gridded table.** The default table detector returned as soon as its grid pass found a table, so the horizontal-rule pass never ran on that page and the later bordered and heuristic passes skip any page that already has a table; the rules-only table came back as paragraphs. The horizontal-rule pass now also runs on the rules no found table covers. Two more cases in that pass are fixed along the way: a booktabs header row alone between the top rule and the midrule joins the body instead of being dropped, and a table ruled under every row forms one table instead of one-row fragments. Across the 379 `test_documents` PDFs only `issue-466-example` changes (its "No vertical lines" table is recovered). (GH#1799)
- **(pdf): a heading that spans several shaded table columns is no longer dropped.** Three things cost grouped headings: repeated inset background edges inside an already-ruled band were treated as real dividers, a cell's right edge was chosen once with no retry when a divider ended at the header row, and a heading spanning two narrower body cells matched neither of them because grouping required an exact full-edge match. Inset backgrounds are now ignored inside a ruled band, the column search retries wider candidates, and a spanning heading is attached to an established body grid by a separate pass over already-formed groups -- the exact-edge rule the sweep-line prune depends on is unchanged. Header-only fragments stay separate so they cannot suppress fallback extraction of an unruled body. (GH#1803)
- **(pdf): an arrow-bullet list is emitted as a list instead of being folded into the paragraph above it.** A producer that marks list items with `➢` (U+27A2) had those items run into the preceding paragraph, and they appeared as neither list items in Markdown nor `ListItem` nodes in the document structure, while the same document's `•` lists were handled correctly. Bullet-glyph membership was enumerated independently at thirteen sites across list detection, list normalisation, paragraph splitting, markdown rendering and the two "this is a list, not a table" guards, and those sets had drifted apart -- `➤` (U+27A4) appeared in exactly one of them and `➢` in none. The list-detection and normalisation sites now share one glyph set -- the union of what they already accepted between them -- so a bullet added once is recognised everywhere rather than at whichever sites a fix happened to touch. Two consequences worth stating: the heading/body classifier previously accepted four of these glyphs and now accepts all eleven, which can only make it harder to read a bullet-prefixed line as a heading; and the guard that stops a list being rebuilt as a table accepts the same set, which can only prevent a table, never create one. The sets that are deliberately narrower are left narrow and now say why, and the equivalent guard in the native-pdf crate keeps its own list because it cannot import this one. (GH#1790)
- **(pdf): a valid multi-page PDF no longer logs a warning for every page-tree branch the walk passes.** Finding page N by walking the page tree used the same error variant for "not found under this branch, keep looking" as for a genuine page-tree fault; a fix for a deep-tree abort turned that ordinary signal into a WARN log, so a flat n-page tree logged up to n(n-1)/2 warnings extracting every page. The walk now returns `None` for an ordinary miss and reserves `Err` for a real fault (a load failure, a cycle, the depth limit, or a malformed node); a genuinely broken branch still warns. Extraction output is unchanged. (GH#1798)
- **(pdf): a font whose `/Encoding` is a `/Differences` dictionary no longer logs "dictionary used where stream expected" on every load.** Two sites sped up CID-CMap and writing-mode detection by calling the stream decoder on the raw `/Encoding` object without checking it was a stream first; a `/Differences` dictionary is a completely normal encoding and was never a stream to begin with. The decoder now runs only when `/Encoding` actually is a stream. (GH#1795)
- **(pdf): scan detection now scores a page's native text by how much of the page it covers, not by whether it is present or how many glyphs it has.** Three related gaps in `scanned_confidence` are fixed by the same change: a full-page scan carrying a header/footer scored identically to a decorative background under a real page of text (both 0.50, GH#1779); a page whose image covered less than 80% of it scored 0.0 regardless of any other evidence, so a real-world corpus's 112 image-only pages (22-76% coverage, 41-92 native glyphs each) were unreachable at any threshold (GH#1793); and a full-page raster with any visible glyphs at all was capped at 0.65, under the 0.70 default, even with a bilevel codec and a scanner producer (GH#1752). A header, footer, or stamp covers on the order of 0.1-0.5% of a page's area regardless of glyph count; a real page of text covers upward of 9%. Text at or under 2% of the page area now counts as "no substantive visible text" for scoring purposes, the same as an absent or invisible text layer, and the image-coverage floor below which scoring is skipped entirely is lowered from 80% to 15% so a partial-coverage scan is no longer zeroed before its text layer is even inspected. A page whose native text does cover a real share of the page -- a decorative background under body text, or body text next to a figure -- is unaffected. (GH#1779, GH#1793, GH#1752)
- **(pdf): numeric footnotes stay separate in CLI and binding output.** A small footnote after a number no longer turns `comma 3` plus note `5` into `comma 35`. The separator requires a matching, smaller note below the reference; numeric scripts without that evidence keep their existing joins. (GH#1771)
- **(ocr): an opt-in preprocessing step recovers table rows on a shaded/filled background.** Whole-page Otsu binarization applies one threshold to the entire page, so a subtotal or total row rendered on a light or dark fill is thresholded away in full. `ImagePreprocessingConfig::normalize_shaded_rows` (default `false`) finds each shaded horizontal band, determines its own polarity, and stretches its contrast to plain dark-on-white before binarization runs, leaving the rest of the page untouched. It defaults to off rather than on: the same per-band step that recovers light- and dark-fill rows measurably regresses a mid-grey fill it does not model, so enabling it is a per-document decision, not a new default. (GH#1785)
- **(ocr): an opt-in repair for numeric OCR tokens that are clearly numeric but mis-punctuated.** Tesseract reliably drops a thousands separator (`1172` for `1,172`), misreads a comma as a decimal point (`7.812` for `7,812`), or splits a number across a rendering gap (`2 2,411` for `22,411`) in financial tables. `OcrConfig::numeric_repair` (default `false`) fixes these three narrow shapes after OCR runs. It defaults to off: the repair assumes the US/UK number convention (comma groups, period decimals) and has no table-column context available to distinguish that from `1.234,56`-style European formatting, so it can misjudge a genuine decimal on a page that uses the other convention. Enable it only for documents known to use the US/UK convention. (GH#1789)
- **(ocr): `ocr-wasm`-only builds compile with warnings denied.** The automatic-backend lookup now
  compiles only when a PDF or native embedded-image OCR consumer is enabled. (GH#1830)
- **(pdf): text in a table cell that spans several rows or columns is no longer dropped.** A detected cell's occupancy was recorded only for the grid slot it starts in, so a word whose centre fell in any other part of the same spanning cell was discarded with no warning; intermediate grid boundaries introduced by neighbouring cells made parts of a cell appear empty. Each span is still assigned exactly once, and text outside any detected cell stays excluded. The spanning cell is still emitted as one row per row-band it covers rather than as a single merged cell -- only the text loss is fixed, not the reported geometry. (GH#1802)
- **(pdf): a document whose cross-reference stream is truncated no longer returns mostly empty pages.** When the final `/XRef` stream fails to decode, the parser rebuilds the table by scanning the file for literal `N G obj` headers. That scan cannot see an object packed inside an `/ObjStm` container, which in an incrementally updated PDF routinely includes every page's font dictionary. The reference then resolved to null -- legitimate per PDF 32000-1 7.3.10 for a deleted object, and therefore silent -- so pages extracted no text while the document reported success. Two reported files returned 86 of 88 and 22 of 24 pages empty; both now recover every page. The object-stream sweep that already existed for the analogous mis-flagged-free case now also runs once when the xref came from reconstruction. A reference still unresolvable after that sweep raises an `XrefRecovery` warning, which is deliberately not raised for an ordinary null resolution. (GH#1774)
- **(pdf): a fixed-pitch producer's word gaps survive extraction.** A PDF that places every glyph in its own cell with one `Tm` and one `Tj`, and marks a word gap as an empty cell carrying no space glyph, extracted as a single welded token (`INVOICENUMBERANDDATE`). The operator path that batches such runs adjusts the buffer's accumulated width and consults no spacing heuristic, so the geometric space test never saw the pair. Since a monospace font's space-glyph advance is its character-cell pitch, a gap of at least half a cell beyond the buffer's own advance can only be a skipped cell. The gate applies only to monospace horizontal runs on this path and fired on none of 229 corpus documents, so proportional-font extraction is unchanged. (GH#1770)
- **(pdf): page attributes now inherit past dangling references.** A missing indirect `/MediaBox`, `/CropBox`,
  `/Resources`, or `/Rotate` entry on a page or intermediate page-tree node no longer masks a valid ancestor
  value. Lazy and bulk page walks agree on the nearest valid ancestor and keep sibling inheritance separate.
  (GH#1775)
- **(pdf): a render warning says glyph ink is missing only when a glyph was not painted.** Every warning the PDF engine logged during a page render reached `processing_warnings` as "could not paint one or more glyphs ... the glyph ink is missing", including font-load notes that drop nothing, such as the Type 3 glyph-name fallback. A warning is now worded by what the engine was doing when it logged it: a dropped glyph or a font that could not be found or loaded for rendering keeps the glyph-ink wording, an unrenderable image keeps its image wording, and any other warning reads "Page N rendering logged a warning and continued" with its own cause. (GH#1794)
- **(pdf): a Type 3 font's glyph advances follow its `/FontMatrix` instead of a hardcoded 1/1000 em.** The page renderer and the text rasteriser both advanced the pen by `width * size / 1000`, which is correct only for the 1000-unit em that Type 1 and TrueType fonts use. A Type 3 font declaring `/FontMatrix [0.01 0 0 -0.01 0 0]` measures its glyphs in 100 units, so every advance was understated about tenfold and the glyphs stacked on nearly one x position. The scale factor was already parsed and stored as `font_matrix_a` and simply never read. On the reporting document, 503 such fonts across 165 pages: a rendered page went from 6,408 dark pixels spanning 75 px to 16,705 spanning 610 px, and because the collapsed raster also defeated OCR the pages returned noise and 0 of 20 sampled values. (GH#1780)
- **(ocr): the OCR result cache honours `use_cache: false` and keys on the recognition model actually used.** Two defects made it serve text that did not belong to the request. A top-level `use_cache: false` bypassed only the whole-extraction cache, leaving `TesseractConfig.use_cache` -- which defaults to true -- caching underneath it, so a caller asking for a fresh result still got a stored one; the flag is now propagated into the `backend_options["use_cache"]` channel the backend already reads. And the cache key covered the configured tessdata *path string* but not the directory it resolved to, so two runs against different `TESSDATA_PREFIX` models collided on one entry and the second silently received the first model's text; the resolved directory is now part of the key. Anyone who has compared OCR configurations or models in one process should discard measurements taken before this change. (GH#1787)
- **(ocr): a PNG carrying no resolution tag is no longer assumed to be 72 dpi and resampled before recognition.** An image with no `pHYs` chunk was treated as 72 dpi, so a genuine 300 dpi scan was upscaled about 1.24x into interpolated pixels before OCR -- meaning identical pixels produced different text depending only on whether the encoder happened to write a density tag. On a dense reported page that cost 58% of the words, 1,018 down to 431. The resolution is now inferred from the pixel dimensions against standard page sizes at plausible scan resolutions; dimensions matching no standard page size still fall back to the historical 72 dpi assumption, unchanged. (GH#1788)
- **(pdf): a JPEG 2000 image with no `/ColorSpace` entry is decoded instead of skipped.** ISO 32000-1 Table 89 allows the entry to be absent for `JPXDecode`, because the colour space is carried inside the codestream -- and the decoder already reads it from there, ignoring the dictionary value. The entry was nevertheless required for every non-mask image, so the page handed OCR a blank region while extraction reported success, and neither `force_ocr` nor `force_ocr_pages` helped. Only `JPXDecode` gets the exemption; a missing `/ColorSpace` is still an error for every other filter, pinned by a FlateDecode control test. (GH#1781)
- **(ocr): a table is no longer discarded because one OCR-split label became its own column.** When recognition split a multi-word row label across two cells, the fragment formed a column of its own that was empty in every other row, and the column-sparsity check then rejected the whole table rather than the spurious column -- on one reported set, only one of four inputs survived. Such a column is now folded back into its left neighbour before the check runs, and only when it has no header of its own and every cell in it is word-like, so a genuinely sparse numeric column is still left to the checks that exist for it. A malformed grid is still rejected. (GH#1797)
- **(ocr): `force_ocr` renders its page batches across the thread budget.** The route that recognises every page rendered them one at a time and only parallelised recognition, so raising `max_threads` did not shorten the render half at all -- 17.5 s against 8.2 s on a 40-page scan at sixteen threads, for byte-identical output. The sibling route that takes an explicit page list was already parallel; both now use the same bounded pool, and remain sequential on wasm32, which has no thread pool. (GH#1796)
- **(pdf): a decoder stops at its output limit instead of allocating past it first.** `RunLengthDecode` and `LZWDecode` built their entire output and only then compared it against the configured maximum, and the compression-ratio guard could never fire for run-length data at all, since that filter's densest encoding is 64:1 against a 100:1 check. A chained `[FlateDecode, RunLengthDecode]` stream could therefore hold about 16 GB transiently. Both now check after each decoded unit, bounding the overshoot to one unit. This is transient allocation only -- accepted output could not exceed the cumulative ratio before this change either. (GH#1764)

