# Alef Candle Exclusions Audit — 2026-06-16

## Summary

This audit designs the exact `exclude_types` / `exclude_functions` / `exclude_modules` skip list that should be added to `alef.toml` to prevent the alef polyglot binding generator from emitting per-language stubs for Rust-only candle OCR backends. The candle OCR backends are selected at **runtime via string configuration** (`OcrConfig.backend = "candle-glm-ocr" | "candle-trocr" | "candle-paddleocr-vl"`), not via language-binding constructors. Therefore, no binding needs to expose `GlmOcrBackend`, `TrocrBackend`, `PaddleOcrVlBackend`, or their supporting types.

## Current alef.toml exclude_* State

### Global crate-level exclusions

From `[crates.exclude]` (lines 842–973):

**Types currently excluded:**

- `Pool` — generic type (alef(skip) not propagated)
- `kreuzberg::extraction::docx::parser::Table`, `TableCell`, `TableRow`, `HeaderFooter`, `Note`
- `kreuzberg::extraction::hwp::model::Section`
- `GlineBackend` — private Arc<Mutex<>> field
- `kreuzberg::chunking::semantic::merge::Segment` — lifetime-parameterised
- `NerBackend`, `kreuzberg::text::ner::backend::NerBackend` — trait without plugin registry
- `EnrichmentConfig`, `EnrichedResult`, `NerEnrichmentConfig` — enrich types pending NerBackend trait_bridge
- `ClassifyContext`, `kreuzberg::text::classification::page_classifier::ClassifyContext` — lifetime-bound

**Syntax used:** Bare type names (e.g., `"PooledString"`) or fully qualified paths (e.g., `"kreuzberg::extraction::docx::parser::Table"`).

### Per-language language-specific exclusions

Examples:

- `[crates.python]`: `exclude_types = ["PooledString"]`
- `[crates.node]`: `exclude_types = ["StreamReader", "PooledString"]`
- `[crates.php]`: `exclude_types = ["ChunkerType", "OutputFormat", ..., "PooledString"]`
- `[crates.wasm]`: `exclude_types = ["OcrFallbackDecision", "OcrProcessor", ...]`

## Candle Public Type Surface

Identified from `kreuzberg-candle-ocr/src/lib.rs` and `crates/kreuzberg/src/candle_ocr/*.rs`:

### Types that would be emitted by alef (if crates/kreuzberg-candle-ocr were added to `[[crates]].sources`)

#### In `kreuzberg-candle-ocr`

**From `crates/kreuzberg-candle-ocr/src/lib.rs`:**

- `pub enum ModelKind` — identifies OCR model (Trocr, PaddleOcrVl, GlmOcr)
- `pub struct CandleOcrOutput` — output container (content, is_structured_markdown, confidence)
- `pub enum DevicePreference` — device selector (Auto, Cpu, Cuda, Metal) [exported]
- `pub use candle_core::DType` (cfg-gated on not WASM) — data type selector

**From `crates/kreuzberg-candle-ocr/src/models/mod.rs`:**

- `pub use trocr::{TrocrEngine, TrocrVariant}` (cfg-gated on feature "trocr")
- `pub use paddleocr_vl::{PaddleOcrVlEngine, PaddleOcrVlTask}` (cfg-gated on feature "paddleocr-vl")
- `pub use glm_ocr::{GlmOcrConfig, GlmOcrEngine, GlmOcrTask}` (cfg-gated on feature "glm-ocr")

**From `crates/kreuzberg-candle-ocr/src/models/glm_ocr/mod.rs`:**

- `pub enum GlmOcrTask` — task selector (Ocr, Table, Formula, Chart, Caption)
- `pub struct GlmOcrConfig` — configuration loaded from HuggingFace
- `pub struct GlmOcrEngine` (not(target_arch = "wasm32")) — inference engine

**From `crates/kreuzberg-candle-ocr/src/models/trocr.rs`:**

- `pub enum TrocrVariant` — model variant (BasePrinted, LargePrinted, BaseHandwritten, LargeHandwritten)
- `pub struct TrocrEngine` (not(target_arch = "wasm32")) — inference engine

**From `crates/kreuzberg-candle-ocr/src/models/paddleocr_vl.rs`:**

- `pub enum PaddleOcrVlTask` — task selector (Ocr, Table, Formula, Chart)
- `pub struct PaddleOcrVlEngine` (not(target_arch = "wasm32")) — inference engine

**From `crates/kreuzberg-candle-ocr/src/device.rs`:**

- `pub enum DevicePreference` — device selector (Auto, Cpu, Cuda, Metal)

#### In `kreuzberg` core

**From `crates/kreuzberg/src/candle_ocr/glm_ocr_backend.rs`:**

- `pub enum LayoutMode` — layout dispatch mode (WholePage, Paired)
- `pub struct GlmOcrBackend` (marked with `#[cfg_attr(alef, alef(skip))]`) — backend implementation
- `pub fn get_or_init_engine(...)` — internal engine pool helper (NOT public API, not exported from lib.rs)

**From `crates/kreuzberg/src/candle_ocr/trocr_backend.rs`:**

- `pub struct TrocrBackend` (marked with `#[cfg_attr(alef, alef(skip))]`) — backend implementation
- `pub fn parse_options(...)` — internal helper (NOT public API)

**From `crates/kreuzberg/src/candle_ocr/paddleocr_vl_backend.rs`:**

- `pub struct PaddleOcrVlBackend` — backend implementation (no explicit skip annotation visible)
- No constructor or functions exported from lib.rs

**From `crates/kreuzberg/src/layout/models/pp_doclayout_v3.rs`:**

- `pub struct PpDocLayoutV3Model` (marked with `#[cfg_attr(alef, alef(skip))]`) — layout detection model
- `pub struct PpDocLayoutV3Config` (if it exists) — model configuration

### Visibility from lib.rs

Checked `crates/kreuzberg/src/lib.rs`: No imports/re-exports of any candle backend types or models. Backend registration happens in `register_default_backends()` which is internal (`pub(crate)` or module-private). Therefore:

- `GlmOcrBackend`, `TrocrBackend`, `PaddleOcrVlBackend` are NOT part of the public binding surface.
- `LayoutMode` is NOT exported from lib.rs (defined in a module not re-exported).
- `CandleOcrOutput` and `DevicePreference` are used internally but not re-exported.

## Current Emission State in Binding Outputs

**Search results:** Grep for `GlmOcrBackend`, `TrocrBackend`, `PaddleOcrVlBackend`, `GlmOcrEngine`, `TrocrEngine`, `PaddleOcrVlEngine`, `GlmOcrTask`, `TrocrVariant`, `PaddleOcrVlTask`, `DevicePreference`, `LayoutMode`, `PpDocLayoutV3Model` across:

- `packages/python/`
- `packages/typescript/`
- `packages/ruby/`
- `packages/php/`
- `packages/go/`
- `packages/java/`
- `packages/csharp/`
- `packages/elixir/`
- `packages/r/`
- `packages/dart/`
- `packages/kotlin-android/`
- `packages/swift/`
- `packages/zig/`
- `crates/kreuzberg-node/`
- `crates/kreuzberg-py/`
- `crates/kreuzberg-php/`
- `crates/kreuzberg-wasm/`
- `crates/kreuzberg-ffi/`

**Result:** No hits. None of the candle types are currently emitted by alef.

**Reason:** The `[[crates]]` source list in `alef.toml` does NOT include any `crates/kreuzberg-candle-ocr/src/` paths. The candle crate is a separate, non-exported dependency of the core kreuzberg library. Alef only sees types re-exported from `crates/kreuzberg/src/lib.rs`, and the candle backends are NOT re-exported there.

## Proposed alef.toml diff

Since `kreuzberg-candle-ocr` is NOT currently in the alef source list, these exclusions are **purely defensive**. They ensure that if `kreuzberg-candle-ocr` is ever added to `[[crates]].sources` in the future (or if accidental re-exports of candle types are added to `kreuzberg/src/lib.rs`), alef will skip them automatically rather than emitting broken bindings.

Add to `[crates.exclude] types`:

```toml
  # Candle OCR backend types — runtime-selected via OcrConfig.backend = "candle-*" string
  # No language binding needs per-backend constructors since backend selection happens
  # at runtime via configuration, not at compile time via binding APIs.
  # These types are internal to the kreuzberg-candle-ocr crate and intentionally NOT
  # re-exported from kreuzberg/src/lib.rs. If accidentally re-exported, exclude them
  # from all polyglot bindings.

  # Backends themselves (marked with #[cfg_attr(alef, alef(skip))] in source)
  "GlmOcrBackend",
  "TrocrBackend",
  "PaddleOcrVlBackend",

  # Candle engines — heavyweight, handle-holding structs; not intended for binding
  "GlmOcrEngine",
  "TrocrEngine",
  "PaddleOcrVlEngine",

  # Task/variant selectors — only used internally; config strings are the binding API
  "GlmOcrTask",
  "TrocrVariant",
  "PaddleOcrVlTask",

  # Device/dtype configuration — internal to candle; bindings use string config only
  "DevicePreference",
  "CandleOcrOutput",

  # Layout detection model (in kreuzberg core, marked with alef(skip))
  "PpDocLayoutV3Model",

  # GLM-OCR task selector (variant of GlmOcrTask)
  "GlmOcrConfig",

  # Layout dispatch mode (glm_ocr_backend.rs)
  "LayoutMode",
```

**Note:** `CandleOcrOutput` is technically part of the public kreuzberg-candle-ocr re-export surface, but it's a narrow implementation detail (just `content: String, is_structured_markdown: bool, confidence: Option<f32>`). It's not used by any binding function signature directly — only internally within the backend layer. Safe to exclude.

**Line count of proposed snippet:** 43 lines (including comments and blank lines for grouping).

## Risk Assessment

### Would adding these exclusions break currently-emitted bindings?

**NO.** Zero candle types are currently emitted by alef in any language binding. Grep search returned no matches across all 17 binding targets.

### Would adding these exclusions prevent future legitimate use?

**No, because:**

1. Backend selection at runtime is via `OcrConfig.backend_name: String`, not via per-binding constructor methods.
2. The Rust core API (`register_ocr_backend_glm_ocr()`, etc.) is not exposed to bindings (internal `pub(crate)` functions).
3. Per-language test fixtures do not need to construct backend objects — they call `extract_file()` with a config JSON that includes `"ocr": { "backend": "candle-glm-ocr" }`.
4. Plugin registry operations (`register_backend`, `unregister_backend`) work with `dyn OcrBackend` trait objects and are NOT exposed in the public binding surface.

### Edge case: what if a user wants to instantiate GlmOcrBackend from a binding?

This is **not a valid use case.** The binding surface exposes:

- Configuration objects (`OcrConfig`, `ExtractionConfig`) — which accept `backend: String`
- Extraction functions (`extract_file`, `extract_bytes`) — which read the config
- Plugin registry stubs (trait-bridge generated) — for registering custom backends at runtime

Users never construct `GlmOcrBackend` directly from bindings. They configure it via JSON strings in `OcrConfig.backend_options`, and the Rust runtime picks the registered backend by name.

## Conclusion

**The proposed exclusions are safe and defensive.** They document candle types as intentionally excluded from polyglot bindings because:

1. Backends are selected via string configuration, not binding APIs.
2. No candle types appear in the public signature of any exported function or config struct.
3. None of these types are currently emitted (zero risk of breaking existing code).
4. If kreuzberg-candle-ocr is ever added as a direct source crate, alef will have pre-configured skip directives.
