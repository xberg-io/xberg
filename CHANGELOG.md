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

### Changed

- **`OcrExtractionResult` now derives `Default`.** Downstream bindings and callers can
  construct and extend it without spelling out every field.

### Fixed

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
