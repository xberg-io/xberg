# GLM-OCR Wiring Audit — 2026-06-16

Branch: `feat/candle` (worktree: `kzb-candle`, HEAD `e4b56394aa`)

This memo is a read-only audit of every touchpoint between the `candle-glm-ocr`
aggregate feature and the rest of the kreuzberg codebase. Each section records
exact file paths and line numbers. A "Gaps to fix" list is appended at the end.

---

## 1. Cargo Aggregates

### `crates/kreuzberg/Cargo.toml`

`candle-glm-ocr` aggregate defined at **line 157**:

```toml
candle-glm-ocr = ["candle-ocr", "kreuzberg-candle-ocr/glm-ocr", "layout-detection"]
```

Composition: pulls `candle-ocr` (root shared base), the `glm-ocr` sub-feature on
`kreuzberg-candle-ocr`, and `layout-detection` for paired dispatch. Correct.

Exclusion audit:

- `full` (lines 467–493): does **not** include `candle-glm-ocr`. OK.
- `formats` (lines 447–461): does **not** include it. OK.
- `wasm-target` (line 381): `= ["no-ort-target", "excel-wasm", "tree-sitter-wasm", "ocr-wasm"]` — no candle. OK.
- `android-target` (lines 386–396): does not include candle features. OK.
- `windows-target` (lines 405–435): does not include candle features. OK.

### `crates/kreuzberg-candle-ocr/Cargo.toml`

`glm-ocr` sub-feature defined at **line 31**:

```toml
glm-ocr = []
```

Empty feature flag (the sub-crate always ships the GLM-OCR model code; the flag
gates the compiled-in status in the parent). Consistent with trocr and paddleocr-vl
patterns.

**Status:** OK

---

## 2. Registry

File: `crates/kreuzberg/src/plugins/registry/ocr.rs`

The `#[cfg(feature = "candle-glm-ocr")]` block is at **lines 157–168**:

```rust
#[cfg(feature = "candle-glm-ocr")]
{
    use crate::candle_ocr::GlmOcrBackend;
    use crate::candle_ocr::glm_ocr_backend::LayoutMode;
    use kreuzberg_candle_ocr::models::glm_ocr::GlmOcrTask;
    tracing::info!("Initializing GLM-OCR backend");
    let backend = GlmOcrBackend::new(GlmOcrTask::default(), LayoutMode::default());
    self.register(Arc::new(backend)).unwrap_or_else(|e| {
        tracing::warn!("Failed to register GLM-OCR backend: {e}");
    });
    tracing::info!("GLM-OCR backend registered successfully");
}
```

Sits directly after the `candle-paddleocr-vl` block (lines 145–155) and before the
module's closing brace. `GlmOcrTask::default()` and `LayoutMode::default()` are both
called correctly.

Note: the block does **not** carry a `not(target_arch = "wasm32")` guard (unlike the
`liter-llm` block at line 121). This is not a defect because `candle-glm-ocr` is not
present in `wasm-target` or `no-ort-target`, so the cfg gate is unreachable in WASM
builds regardless. Consistent with the `candle-trocr` block (lines 133–143), which
also lacks the wasm guard.

**Status:** OK

---

## 3. Module Export

File: `crates/kreuzberg/src/candle_ocr/mod.rs`

- Module declaration at **line 15**: `#[cfg(all(feature = "candle-glm-ocr", not(target_arch = "wasm32")))]`
- Re-export at **line 26**: same guard.
- `trocr_backend` (line 9) and `paddleocr_vl_backend` (line 12) use only `#[cfg(feature = "...")]`
  without the wasm guard.

The `not(target_arch = "wasm32")` on GLM-OCR but not on the other two creates a
minor inconsistency. However, it is not harmful: `candle-glm-ocr` is never active
on WASM (not in `wasm-target`). If a caller somehow enabled `candle-glm-ocr` on
wasm32 the mod would compile out correctly. The asymmetry could confuse a future
reader but is not a functional bug.

`resolve_device_preference` at **line 51** and `device_preference_from_acceleration`
at **line 79** are both gated `#[cfg(feature = "candle-ocr")]` — the root shared
base, exactly as the audit spec requires. Not narrowed to `any(feature = "candle-trocr",
...)`. OK.

**Status:** OK (minor style asymmetry on wasm guard not actionable)

---

## 4. Backend File

File: `crates/kreuzberg/src/candle_ocr/glm_ocr_backend.rs`

### `LayoutMode` enum (lines 39–47)

```rust
pub enum LayoutMode {
    WholePage,
    #[cfg(feature = "layout-detection")]
    Paired,
}
```

`LayoutMode::default()` (lines 49–63) resolves to `Paired` when `layout-detection`
is active, `WholePage` otherwise. Since `candle-glm-ocr` implies `layout-detection`
via the aggregate, paired mode is always the default when the feature is on. OK.

### `Plugin + OcrBackend` impl (lines 248–362)

- `name()` returns `"candle-glm-ocr"` (line 250). OK.
- `emits_structured_markdown()` returns `true` (lines 357–361). OK.

### `parse_options` (lines 219–245)

Handles `task`, `device`, `layout_mode` keys from `config.backend_options`.
Device selection delegated to `super::resolve_device_preference`. OK.

### Engine pool (lines 65–116)

Keyed by `(DevicePreference, DType)` — task is NOT part of the key (correct, as
documented in lines 7–13). Double-check pattern on write. OK.

### `process_image` / paired path (lines 272–453)

Heavy work in `tokio::task::spawn_blocking`. Paired path:
decode → `PpDocLayoutV3Model::detect` → sort → crop → `process_image_with_task`
per region → `join("\n\n")`. Correct.

### Error source propagation — GAP

`get_or_init_engine` at **lines 90–93**:

```rust
let device = preference.select().map_err(|e| crate::KreuzbergError::Ocr {
    message: format!("Failed to select compute device: {e}"),
    source: None,   // <-- source dropped
})?;
```

`source: None` drops the underlying device-selection error. The engine-init error
at lines 103–106 correctly uses `source: Some(Box::new(e))`. The device-select
path loses the root cause.

**Status:** ⚠️ Gap: `get_or_init_engine` line 92 drops device-select error source (`source: None` should be `source: Some(Box::new(e))`).

---

## 5. CLI Allowlist

File: `crates/kreuzberg-cli/src/commands/overrides.rs`

`VALID_OCR_BACKENDS` const (lines 15–23) includes `"candle-glm-ocr"` at line 22. OK.

`apply_ocr` match arm (line 458):

```rust
Some("candle-glm-ocr") => "candle-glm-ocr",
```

Present. OK.

Default-language arm (line 464):

```rust
"paddle-ocr" | "easyocr" | "candle-paddleocr-vl" | "candle-glm-ocr" => "en".to_string(),
```

`"candle-glm-ocr"` is included. OK.

**Status:** OK

---

## 6. CLI Feature Pass-Through

File: `crates/kreuzberg-cli/Cargo.toml`

Present features (lines 42–48):

```toml
candle-ocr = ["kreuzberg/candle-ocr"]
candle-trocr = ["kreuzberg/candle-trocr"]
candle-paddleocr-vl = ["kreuzberg/candle-paddleocr-vl"]
candle-cuda = ["kreuzberg/candle-cuda"]
candle-metal = ["kreuzberg/candle-metal"]
candle-accelerate = ["kreuzberg/candle-accelerate"]
candle-mkl = ["kreuzberg/candle-mkl"]
```

`candle-glm-ocr = ["kreuzberg/candle-glm-ocr"]` is **absent**. The CLI accepts
`--ocr-backend candle-glm-ocr` at runtime (it's in VALID_OCR_BACKENDS) but the
crate has no feature flag to compile in the backend. A user doing
`cargo build -p kreuzberg-cli --features candle-glm-ocr` would get "Package
`kreuzberg-cli` does not have feature `candle-glm-ocr`" error.

**Status:** ❌ Missing: `candle-glm-ocr = ["kreuzberg/candle-glm-ocr"]` line in `crates/kreuzberg-cli/Cargo.toml` `[features]` section (after line 44, parallel to `candle-trocr` and `candle-paddleocr-vl`).

---

## 7. API/MCP Backend Allowlist

### REST API (`crates/kreuzberg/src/api/`)

The API handlers accept a full `ExtractionConfig` JSON blob via the `config`
multipart field (handlers.rs line 173–184). The config is deserialized directly;
there is no hardcoded backend allowlist. Unknown backends fail at call time with a
`KreuzbergError::Plugin { message: "OCR backend '...' not registered..." }` from the
registry. No change needed for `candle-glm-ocr`.

### MCP Server (`crates/kreuzberg/src/mcp/server.rs`)

The `build_config` function (format.rs line 12) uses `build_config_from_json`, which
is a JSON-level field merge of whatever the client sends. No OCR backend allowlist in
the MCP layer. The only backend references are in tests (line 1291: `"tesseract"`) and
`#[cfg(feature = "paddle-ocr")]` model-download handlers (lines 410, 472–479), neither
of which restricts runtime backend selection.

**Status:** OK (no allowlist in API/MCP; registry acts as gatekeeper)

---

## 8. Backend-Options Propagation

`OcrConfig.backend_options` is defined at:

- `crates/kreuzberg/src/core/config/ocr.rs` line 199 (primary config struct)
- `crates/kreuzberg/src/core/config/ocr.rs` line 340 (pipeline stage struct)

**CLI path**: `apply_ocr` in `overrides.rs` at line 488 hard-writes `backend_options: None`
when `--ocr` is used. There is **no `--ocr-backend-options` CLI flag**. The only way
to reach `GlmOcrBackend::parse_options` with non-None `backend_options` via the CLI
is through `--config-json` / `--config-json-base64` (merge path in
`crates/kreuzberg/src/core/config/merge.rs`) or a discovered `kreuzberg.toml`.

The `apply_ocr` code also resets `backend_options: None` in the secondary override
paths (lines 551 and 1031). This means `--ocr-backend candle-glm-ocr` with
`--config-json '{"ocr":{"backend_options":{"layout_mode":"whole_page"}}}'` would have
the `--ocr` flag's `apply_ocr` overwrite the whole `OcrConfig`, losing the
`backend_options` from the JSON. The JSON merge happens before flag application; the
flag wins and clears `backend_options`.

**Server/MCP path**: `build_config_from_json` (merge.rs lines 38–54) does a
top-level field merge, so `ocr.backend_options` survives if the caller includes it in
the JSON body. No loss here.

**Status:** ⚠️ Gap: CLI `apply_ocr` (overrides.rs line 488) always writes `backend_options: None`. No `--ocr-backend-options` flag exists. `backend_options` (e.g., `layout_mode`, `task`) are unreachable via `--ocr` flag path alone; caller must use `--config-json` to pass them. This is a usability gap for candle backends, not a correctness bug. A `--ocr-backend-options <JSON>` flag with a JSON string value would fix it.

---

## 9. CHANGELOG

File: `CHANGELOG.md` under `## [Unreleased]`

Added entry at **line 38**:

> **ocr**: `candle-glm-ocr` backend exposing zai-org/GLM-OCR through candle. Selectable via `--ocr-backend candle-glm-ocr`. Default layout mode is `paired` (uses PP-DocLayout-V3 for per-region dispatch); set `backend_options.layout_mode = "whole_page"` to disable.

Fixed entries at **lines 25–27**:

> **candle-glm-ocr**: MTP repetition penalty no longer doubles down on already-negative logits.
> **candle-glm-ocr**: Nucleus sampling rejects NaN/inf-tainted probability vectors instead of silently sampling against them.
> **candle-glm-ocr**: Decoder KV cache reset at the start of each generation call to prevent cross-call contamination.

**Status:** OK

---

## 10. Benchmark Harness

### `tools/benchmark-harness/src/types.rs`

`KreuzbergPipeline::CandleGlmOcr` variant present at **line 68**, with `as_str` arm
at **line 80** (`"candle-glm-ocr"`) and `from_str` arm at **line 101**.

### `tools/benchmark-harness/src/comparison.rs`

`Pipeline::CandleGlmOcr` variant at **line 87**.
`build_extraction_config` arm at **lines 417–425** (backend `"candle-glm-ocr"`, language `"en"`).
`all_kreuzberg` includes it at **line 169**.
`from_str` at **line 145**: `"candle-glm-ocr" | "candle_glm_ocr" | "glm-ocr"`.
Test at **line 1340** references `"candle-glm-ocr"`.

### `tools/benchmark-harness/Cargo.toml`

`glm-ocr-bench = ["kreuzberg/candle-glm-ocr"]` at **line 24**. OK.

### `tools/benchmark-harness/src/adapters/kreuzberg.rs`

`KreuzbergPipeline::CandleGlmOcr` match arm at **lines 88–95** pushes
`--ocr-backend candle-glm-ocr` and `--force-ocr true`. OK.

**Status:** OK

---

## 11. Integration Tests

### `crates/kreuzberg-candle-ocr/tests/glm_ocr_integration.rs`

Exists. Gated `#![cfg(feature = "glm-ocr")]`. Smoke test with `#[ignore]` for
network-gated weight download. Includes N-gram repeat detector.

### `crates/kreuzberg-candle-ocr/tests/glm_ocr_paired_pipeline.rs`

Exists. Gated `#![cfg(feature = "glm-ocr")]`. Tests paired pipeline via
`process_image_with_task` directly.

### `crates/kreuzberg/tests/glm_ocr_backend.rs`

Exists. Gated `#![cfg(feature = "candle-glm-ocr")]`. End-to-end test driving
`GlmOcrBackend` through the `OcrBackend` trait.

**Status:** OK

---

## 12. Unit Tests for Correctness Fixes

### `negative_logits_relax_with_penalty` / repetition penalty

`test_apply_repetition_penalty_reduces_both_signs` at
`crates/kreuzberg-candle-ocr/src/models/glm_ocr/mtp.rs` **line 339** verifies:

- Positive logits: divided by penalty (shrink toward 0)
- Negative logits: multiplied by penalty (pushed further negative)

Name does not match spec's `negative_logits_relax_with_penalty` exactly but covers
the same semantic. The spec name appears to be aspirational rather than literal.

**Status:** OK (test exists under a different name)

### `nucleus_sampling_filters_nan`

`test_sample_nucleus_handles_nan` at
`crates/kreuzberg-candle-ocr/src/models/glm_ocr/mtp.rs` **line 361**. Verifies that
NaN in the logit vector does not panic and returns a valid token. Matches the spec.

**Status:** OK (present under `test_sample_nucleus_handles_nan`)

### `detect_structured_markdown`

Present in `crates/kreuzberg-candle-ocr/src/models/glm_ocr/mod.rs` **lines 606–636**
as four separate tests:

- `detect_structured_markdown_recognises_table` (line 607)
- `detect_structured_markdown_recognises_heading` (line 613)
- `detect_structured_markdown_rejects_plain_text` (line 618)
- `detect_structured_markdown_rejects_single_dash` (line 625)
- `detect_structured_markdown_recognises_two_dash_lines` (line 633)

**Status:** OK

### `empty_batch` test in `pp_doclayout_v3.rs`

`crates/kreuzberg/src/layout/models/pp_doclayout_v3.rs` **line 463** contains a
`_doc_empty_batch_contract` dead function documenting the empty-slice contract, but
**no executable test**. The comment at lines 455–462 explicitly states the test is
"omitted here because it cannot be run without a real model file." A compile-time
guard at the `if images.is_empty() { return Ok(Vec::new()); }` path is described
as sufficient.

**Status:** ⚠️ Gap: No `#[test]` for empty-batch contract in `pp_doclayout_v3.rs`.
The `_doc_empty_batch_contract` stub at line 463 is `#[allow(dead_code)]` and never
executes. A unit test can verify the empty-slice guard without a model file by calling
the public `detect` method on a `PpDocLayoutV3Model` that was never loaded — but this
requires a model-free construction path which may not exist.

### Engine pool sharing tests

Neither `crates/kreuzberg/src/candle_ocr/glm_ocr_backend.rs` nor
`crates/kreuzberg/src/candle_ocr/trocr_backend.rs` has an engine pool sharing test.
The `glm_ocr_backend.rs` tests (lines 459–573) cover backend creation, language
support, `parse_options` variants, `initialize`/`shutdown`, and layout helpers but
**not** the `get_or_init_engine` double-check pattern or cache-hit behavior.

**Status:** ⚠️ Gap: No unit test for engine pool sharing / double-init guard in
`glm_ocr_backend.rs` (lines 78–116) or `trocr_backend.rs` (lines 44–74). Both
`ENGINE_POOL` static + read→miss→write→double-check patterns are untested.

---

## Gaps to Fix

| # | File | Line(s) | Action |
|---|------|---------|--------|
| G1 | `crates/kreuzberg-cli/Cargo.toml` | After line 44 | Add `candle-glm-ocr = ["kreuzberg/candle-glm-ocr"]` to `[features]` |
| G2 | `crates/kreuzberg/src/candle_ocr/glm_ocr_backend.rs` | Line 92 | Change `source: None` to `source: Some(Box::new(e))` in `get_or_init_engine` device-select error |
| G3 | `crates/kreuzberg-cli/src/commands/overrides.rs` | Line 488 | Add `--ocr-backend-options` CLI arg (JSON string) and thread it into `backend_options` in `apply_ocr`; the current hard-wired `backend_options: None` makes `layout_mode` and `task` overrides unreachable via `--ocr` flag |
| G4 | `crates/kreuzberg/src/layout/models/pp_doclayout_v3.rs` | Line 463 | Replace `_doc_empty_batch_contract` stub with a real `#[test]` if a model-free construction path exists, or mark as known limitation in test doc |
| G5 | `crates/kreuzberg/src/candle_ocr/glm_ocr_backend.rs` | Lines 78–116 | Add unit test for engine pool: two `get_or_init_engine` calls with same key share the same `Arc`, verifying the double-check path |
| G6 | `crates/kreuzberg/src/candle_ocr/trocr_backend.rs` | Lines 44–74 | Same: add unit test for `ENGINE_POOL` sharing (parallel item to G5) |

### Severity ranking

- **G1** (CLI feature pass-through): hard correctness failure — `cargo build --features candle-glm-ocr -p kreuzberg-cli` fails entirely.
- **G2** (error source dropped): silent loss of root cause in device-select failure; violates `source: Some(Box::new(e))` contract stated in CLAUDE.md.
- **G3** (backend_options unreachable via CLI `--ocr`): usability gap; workaround via `--config-json` exists.
- **G4/G5/G6** (test coverage): coverage gaps, not runtime defects.
