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

- **Scanned PDFs are now detectable, and can be OCR'd without forcing OCR on the whole file.**
  PDF metadata gains `scanned_confidence` (0.0–1.0) and `scanned_pages`, so you can tell whether a
  document is a scan before deciding how to extract it. The new `ocr_strategy` config selects which
  pages get OCR'd: the default `auto` keeps today's behaviour, while
  `scanned_pages { min_confidence }` also OCRs pages that look like scans and leaves the rest on
  native text. On the CLI this is `--ocr-scanned-pages [--scanned-min-confidence 0.7]`.

  This closes a gap that had no workaround. Consumer scanners embed an invisible OCR text layer
  whose words are well-formed but wrong (`Tbe` for `The`, `rn` for `m`, `l2,45O.OO` for `12,450.00`).
  Every quality check kreuzberg applied to a native text layer was lexical, so that text passed and
  OCR never ran; the only escape was `force_ocr`, which degrades clean PDFs. Detection instead asks
  whether the page is a picture of a document — how much of it is raster, whether its text layer is
  hidden or absent, its image codec, and the producer. A born-digital slide with a full-bleed
  background image scores 0.50 and is never OCR'd by default.

  Detection cannot judge whether a scanner's text is *accurate*, only that a scanner produced it, so
  a page carrying a good hidden text layer is OCR'd too.

### Removed

- **`HierarchyConfig.ocr_coverage_threshold`.** The field promised to "trigger OCR if less than 50%
  of page has text" and was documented that way in every language binding, but nothing ever read it —
  setting it did nothing. Its only test had been disabled since the alef migration. `ocr_strategy`
  now provides the behaviour it advertised. Existing config files that still set the key keep loading;
  the key is ignored.

### Fixed

- **macOS wheels and the npm darwin package now target macOS 11, instead of only macOS 15.**
  Wheels were built with a deployment target of 15.0, so pip and uv matched no wheel below
  macOS 15 and silently fell back to compiling the Rust sdist; the npm darwin package vendored
  the same Homebrew libheif closure, compiled for the runner's macOS 15. Both artifacts now
  bundle a libheif decode stack built from source at the 11.0 target — as the Linux wheels
  already do inside the manylinux container — and CI fails if any bundled library misses that
  floor or is sourced from Homebrew.
- **The Intel CLI tarball's ONNX Runtime now runs on macOS 13.4+ and is a single file.** It
  previously vendored Homebrew's unpinned onnxruntime bottle (macOS 14 floor) plus its
  abseil/onnx/protobuf/re2 closure. It now ships Microsoft's official 1.23.2 build, the last
  x86_64 macOS release, which links only system frameworks.
- **XML entities (`&amp;`, `&lt;`, `&gt;`) no longer disappear from extracted text.** quick-xml
  0.38 started delivering entity and character references as separate events instead of inlining
  them in text, and every streaming reader that only matched text events silently dropped them —
  `Falafel & Hummus <combo>` in a DOCX came out as `Falafel  Hummus combo`. Affected formats:
  DOCX (body, tables, headers/footers, footnotes/endnotes, math), DocBook, JATS, and generic
  XML/SVG. Text fragments are now coalesced with their resolved references before any
  whitespace handling, so entities survive with correct spacing (`5>3` stays `5>3`, not `5 > 3`).
- **Markdown, CSV, and other text members inside an archive are no longer flattened to escaped
  prose.** Recursive archive extraction resolved each member's MIME by content sniffing first, but
  markdown/CSV/YAML are all plain UTF-8 and sniff to `text/plain` — so a zipped `.md` reached the
  plain-text extractor, which escaped its headings (`# Title` -> `\# Title`) and dropped its
  structure. Member detection now prefers the file extension whenever the sniff is only the generic
  `text/plain`, matching how the top-level path resolves a file by `detect_mime_type(path)` first,
  so a `.md` member routes to the markdown extractor. Concrete binary formats (PDF/DOCX/images),
  which sniff to a specific type, are unaffected.
- **Model downloads can no longer hang the extraction pipeline on a blocked network.** hf-hub
  builds its ureq agent with no read/connect timeout, so a firewalled or stalled HuggingFace
  connection made the blocking `ApiRepo::get()` block forever — wedging the whole pipeline at 0%
  CPU with no error. Every model-file fetch (embeddings, reranker, transcription, PaddleOCR/layout
  managers, and the Candle TrOCR / GLM-OCR backends) now runs under a wall-clock watchdog: on
  expiry it logs a warning and returns an error so the caller degrades (skips the model-backed
  backend) instead of hanging. Default ceiling 300s; override with
  `XBERG_MODEL_DOWNLOAD_TIMEOUT_SECS`.

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

- **Concurrent model downloads no longer race the Hugging Face cache lock.** When two
  threads needed the same cold model at once — e.g. parallel-page OCR workers, or two
  GPU tests sharing a layout model — hf-hub errored one of them with `Lock acquisition
  failed` instead of waiting. Downloads of the same file now serialize above hf-hub, so
  the first populates the cache and the rest get the warm copy; different files still
  download in parallel.
- **Hunyuan-OCR auto-downloads its weights instead of requiring a local `model_path`.**
  The docs said the model downloads on first use, but the backend errored unless
  `backend_options.model_path` pointed at a pre-staged directory — and the checkpoint's
  Hugging Face repo was pulled upstream, so there was nowhere obvious to stage it from.
  The backend now fetches the weights on first use from Tencent's official ModelScope
  release, verifies every file against a checked-in sha256 manifest, and caches them
  under the xberg cache directory. An explicit `model_path` still takes precedence for
  offline or custom weights.
- **PaddleOCR-VL crashed on non-square pages.** The multimodal rope index built the
  vision-block position tensors with a transposed height/width layout and dropped the
  temporal row, so any image whose patch grid wasn't square failed inference with
  `cannot broadcast [1, N] to [N, M]`. Position ids now follow the reference
  Qwen2-VL layout (t constant per frame, h per grid row, w per grid column).
- **PaddleOCR-VL generation never decoded a token.** The greedy loop read the argmax
  as a rank-1 tensor where the model returns `[1, 1]`, re-fed the full sequence every
  step while the attention KV cache kept appending (duplicating keys), and the
  position-continuation branch for cached decode steps was missing, failing with a
  dtype mismatch. The loop now prefills once and feeds each new token through the
  cached decode path at its absolute position, like the other Candle OCR engines,
  and returns only newly generated tokens so the prompt is never echoed into the
  OCR output.
- **PaddleOCR-VL prefill attended bidirectionally, degenerating generation.** The
  ERNIE decoder ran the multi-token prefill without a causal mask, so every prompt
  token attended to future positions, the KV cache was built from contaminated
  hidden states, and generation collapsed into repeating a single token. The prefill
  now applies the standard additive causal mask, like the other Candle OCR decoders.
- **Hunyuan-OCR failed to load checkpoints whose `config.json` omits
  `tie_word_embeddings`.** Later revisions of the released checkpoint drop the field
  (transformers defaults it to `true`); the config parser now does the same instead
  of rejecting the whole model.
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
