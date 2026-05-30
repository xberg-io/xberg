# WASM Hand-Edit Audit for Alef Template Upstreaming

This document categorizes all hand-edits made to the WASM binding during the alef-hand-edit cycle (commits 86f4510cfd..cf5a8bef8d) by their upstream destination.

## ALEF_GAP: Template & Generator Issues

### 1. Feature-Gated Config Type Stripping
**File**: `crates/kreuzberg-wasm/src/lib.rs`, lines 13405-13424, 13500-13501, 13526-13527
**Issue**: ExtractionConfigInput contains LayoutDetectionConfig and TreeSitterConfig, which are gated behind `layout-types` and `tree-sitter` features that are not enabled in the `wasm-target` feature set. The alef wasm template must generate code that:
- Marks these fields with `#[serde(skip)]` to prevent deserialization errors
- Replaces the typed field value with `Option<serde_json::Value>` as a placeholder
- Silently discards the value when converting from ConfigInput to ExtractionConfig (with inline comment explaining the feature gate)

**Category**: ALEF_GAP

**Suggested Upstream Fix**: Extend alef's wasm config codegen to:
1. Scan the Rust core's `ExtractionConfig` struct for fields whose types are gated behind non-wasm features
2. Generate `skip`-decorated, `serde_json::Value`-typed stub fields in ExtractionConfigInput
3. Emit comment lines in From impl explaining which feature gates the exclusion
4. Pattern: `#[serde(rename = "fieldName", skip)]` + comment `// fieldName: stripped on wasm (Feature requires X feature, not in wasm-target)`

---

### 2. FormatMetadata::Code Variant Stripping
**File**: `crates/kreuzberg-wasm/src/lib.rs`, lines 19678, 19763-19765
**Issue**: The From<WasmFormatMetadata> impl receives a format_type="code" variant that the Rust core builds, but the Rust core's Code variant is gated behind the `tree-sitter` feature (not in wasm-target). Both direction conversions must omit the Code case entirely and fallback to Pdf with a comment.

**Pattern in impl**:
- From<WasmFormatMetadata>: `"code" => Self::Pdf(Default::default()), // comment`
- From<kreuzberg::FormatMetadata>: Match on Code variant must be stripped (match doesn't include Code case, fallback _ catches it)

**Category**: ALEF_GAP

**Suggested Upstream Fix**: In wasm bridge codegen for tagged-union FormatMetadata:
1. Detect variants whose associated types are feature-gated and not in wasm-target
2. Omit those variants from the From impl match statements
3. Emit `// "variantName": stripped on wasm (Variant gated behind Feature)` comment where omitted
4. Ensure match arms are exhaustive with catch-all fallback

---

### 3. Env Shim Import Order in e2e/wasm/setup.ts
**File**: `e2e/wasm/setup.ts`, lines 11–160 (env/wasi patch block) then lines 138–158 (pre-init block)
**Issue**: The generated setup.ts runs the wasm-bindgen pre-init (locating and calling initSync) at the top of the file, but the very first import statement in the wasm bundle tries to require('env') and require('wasi_snapshot_preview1'). These must be patched BEFORE the pre-init runs, otherwise the initial import fails.

**Sequence required**:
1. Patch Module._load to intercept require('env') and require('wasi_snapshot_preview1')
2. Patch WebAssembly.Instance to stash the instance on globalThis.__alef_wasm_memory__
3. THEN import and call initSync

**Category**: ALEF_GAP

**Suggested Upstream Fix**: In alef's wasm e2e setup.ts template, reorder so that:
1. All Module._load patches install FIRST (require('env'), require('wasi_snapshot_preview1'), WebAssembly.Instance)
2. THEN the async pre-init block that does `await import(wasmPkgDir)`
3. Add a clarifying comment: "MUST patch before wasm bundle is imported; patches Module._load, WebAssembly, and stash the instance globally"

---

### 4. Plugin Test Stub Method Naming & Lifecycle
**File**: `e2e/wasm/tests/plugin_api.test.ts`, lines 40–91
**Issue**: The alef e2e generator emitted test stubs with snake_case method names (extract_bytes, supported_mime_types, process_image, supports_language, backend_type, processing_stage) and omitted the initialize/shutdown lifecycle methods. The actual wasm bridges use camelCase via wasm-bindgen and require initialize/shutdown.

**Bridge contracts** (wasm-bindgen generates these names):
- DocumentExtractor: initialize(), shutdown(), extractBytes(), supportedMimeTypes()
- EmbeddingBackend: initialize(), shutdown(), dimensions(), embed()
- OcrBackend: initialize(), shutdown(), processImage(), supportsLanguage(), backendType()
- PostProcessor: initialize(), shutdown(), process(), processingStage()
- Renderer: initialize(), shutdown(), render()
- Validator: initialize(), shutdown(), validate()

**Category**: ALEF_GAP + TEST_FIXTURE

**Suggested Upstream Fix**:
1. Extend alef's e2e fixture schema to track camelCase method naming per trait (use wasm-bindgen's naming convention by default)
2. Emit initialize/shutdown stubs for all trait test classes (body: no-op void or stub return value)
3. Use camelCase names in all generated test stubs

---

## BINDING_BUG: Return-Value Parsing Errors

### 5. WasmEmbeddingBackendBridge::dimensions – Incorrect JSON Parse
**File**: `crates/kreuzberg-wasm/src/lib.rs`, line 14714 (FIXED)
**Status**: Already fixed in commit cf5a8bef8d
**Details**: The JS dimensions() method returns a number (e.g., 1536), but the original code parsed it as a JSON string: `.as_string().and_then(|s| serde_json::from_str::<usize>(&s).ok())`. Fixed to `.as_f64().map(|n| n as usize)`.

**Category**: BINDING_BUG

**Root Cause**: The alef wasm bridge codegen doesn't distinguish between JS return types correctly. Some methods return primitives (number, bool, string), others return JSON-serializable objects. The codegen currently treats all as JSON strings.

---

### 6. WasmOcrBackendBridge::supports_language – Incorrect Bool Parse
**File**: `crates/kreuzberg-wasm/src/lib.rs`, lines 14002–14008 (NOT YET FIXED)
**Details**: Returns bool but parsed as JSON string: `.as_string().and_then(|s| serde_json::from_str::<bool>(&s).ok())`. Should be `.as_bool().unwrap_or_default()`.

**Bridge**: `OcrBackend::supports_language(&self, lang: &str) -> bool`

**Category**: BINDING_BUG

**Pattern**: Any bridge method returning a primitive (bool, number, string) that's not JSON should use the appropriate as_X method, not as_string + JSON parse.

---

### 7. WasmOcrBackendBridge::backend_type – Incorrect OcrBackendType Parse
**File**: `crates/kreuzberg-wasm/src/lib.rs`, lines 14036–14042 (NOT YET FIXED)
**Details**: Returns kreuzberg::OcrBackendType (an enum) but parsed as JSON string. Since OcrBackendType is serializable, this *may* work if the JS side returns a JSON string, but it's fragile. Should be reviewed: either JS must return JSON-stringified enum, or bridge should use serde_wasm_bindgen.

**Bridge**: `OcrBackend::backend_type(&self) -> kreuzberg::OcrBackendType`

**Category**: BINDING_BUG

**Note**: Unlike dimensions (which is a raw usize), backend_type is a complex enum. Check if the JS stub is expected to return a JSON string or a JS object. If object, should deserialize via serde_wasm_bindgen, not manual JSON parse.

---

### 8. WasmPostProcessorBridge::processing_stage – Incorrect ProcessingStage Parse
**File**: `crates/kreuzberg-wasm/src/lib.rs`, lines 14287–14293 (NOT YET FIXED)
**Details**: Returns kreuzberg::ProcessingStage (an enum) but parsed as JSON string with serde_json. Same pattern as backend_type — if JS returns a JSON-stringified enum representation, this works; if it returns an object, needs serde_wasm_bindgen.

**Bridge**: `PostProcessor::processing_stage(&self) -> kreuzberg::ProcessingStage`

**Category**: BINDING_BUG

---

## ROOT_CAUSE: Rust Core Gating Decision

### 9. PST Tempfile Gating Location
**File**: `crates/kreuzberg/src/extraction/pst.rs`, lines 59–84
**Decision**: Hand-edited in Rust core to gate PST extraction behind `#[cfg(all(feature = "email", not(target_arch = "wasm32")))]` with a WASM-safe fallback that returns a validation error.

**Tradeoff**:
- **Current (binding-side)**: Clean — Rust core knows about WASM constraint and provides a user-facing error message.
- **Alternative (feature gate)**: Could move `tempfile` usage behind a separate internal feature, but WASM consumers never care about PST extraction (it requires a filesystem), so binding-side gating is simpler and more discoverable for WASM users (they get an explicit error, not a compile failure).

**Category**: ROOT_CAUSE

**Assessment**: This is the correct location. WASM feature set is fundamentally different from native (no temp files), so it's reasonable to gate it in the Rust core with a clear error message. No upstream change needed; this is binding-specific knowledge that lives in the core for good reason.

---

## Summary by Category

| Category | Count | Items |
|----------|-------|-------|
| ALEF_GAP | 4 | Feature-gated config stripping, FormatMetadata Code variant, env shim order, plugin test stubs |
| BINDING_BUG | 4 | Dimensions JSON parse (fixed), supports_language bool parse, backend_type enum parse, processing_stage enum parse |
| ROOT_CAUSE | 1 | PST tempfile gating in pst.rs |

---

## Suspected Return-Value Parse Bugs Across All Bridges

The following WasmXxxBridge methods use `.as_string()` + `serde_json` to parse their return values. Review each to determine if the JS side returns a JSON-stringified value or a primitive:

### Potentially Broken (Primitives):
- `WasmOcrBackendBridge::supports_language()` → bool (line 14004) — **CONFIRMED BROKEN**, should be `.as_bool()`
- `WasmOcrBackendBridge::backend_type()` → OcrBackendType (line 14038) — **SUSPECTED**, verify JS return type
- `WasmPostProcessorBridge::processing_stage()` → ProcessingStage (line 14289) — **SUSPECTED**, verify JS return type

### Likely Correct (Serializable Objects/Strings):
- `WasmDocumentExtractorBridge::extract_bytes()` → ExtractionResult JSON (via serde_wasm_bindgen) ✓
- `WasmDocumentExtractorBridge::supported_mime_types()` → Vec<String> (line 15014) — uses serde_json, correct if JS returns array
- `WasmEmbeddingBackendBridge::embed()` → Vec<Vec<f32>> (line 14751) — uses serde_json, correct if JS returns array
- `WasmOcrBackendBridge::process_image()` → ExtractionResult (line 13966) — uses serde_wasm_bindgen ✓
- `WasmPostProcessorBridge::process()` → Option<ExtractionResult> (line 14289) — uses serde_wasm_bindgen ✓
- `WasmRendererBridge::render()` → String (line 15227) — uses as_string, correct ✓
- `WasmValidatorBridge::validate()` → Option<ExtractionResult> (line 15136) — uses serde_wasm_bindgen ✓

---

## Suggested Cleanup In-Repo

Before upstreaming to alef templates, consider extracting hand-edits into a companion module:

1. **Feature gating for excluded configs**: Create `crates/kreuzberg-wasm/src/config_stubs.rs` to house the skipped-field conversions (layout, tree_sitter). This makes it clear to future maintainers why these fields exist and are ignored, and centralizes the gating logic.

2. **FormatMetadata Code variant handling**: Similarly, a dedicated conversion module would clarify the variant omission.

**Rationale**: Generated code should remain visibly generated (dense, hard to hand-edit). Helper functions or companion modules make intent explicit and reduce cognitive load on future maintainers reviewing the generated code.

---

## Build/Test Infra Notes

### wasm-pack Build Flags
The generated bindings are built with `cargo build --target wasm32-unknown-unknown --release` (no wasm-pack rebuild in the cycle). Verify that:
- `-p kreuzberg-wasm --release` uses the correct opt-level (should be "z" in Cargo.toml profile)
- No additional flags needed beyond what's in the profile (profile.release.package.kreuzberg-wasm)

### vitest Setup Requirements
The e2e/wasm/setup.ts must:
1. Install env/wasi patches BEFORE importing the wasm bundle
2. Stash WebAssembly.Instance on globalThis for dynamic memory access during tests
3. Call initSync synchronously with the prebuilt .wasm file (not async default, which uses fetch)

### Plugin Test Stub Contract
All trait stubs must implement:
- `name(): string`
- `initialize(): void`
- `shutdown(): void`
- Trait-specific methods in camelCase (extractBytes, supportedMimeTypes, processImage, supportsLanguage, backendType, processingStage, embed, process, render, validate, dimensions)

---

## Upstream Priority

**High**: ALEF_GAP issues 1–4. These are systematic codegen gaps that will recur on every regeneration and affect all Rust polyglot projects with feature-gated types.

**Medium**: BINDING_BUG issues 5–8. Return-value parsing requires careful audits of the alef wasm bridge codegen to distinguish primitives, JSON strings, and serde_wasm_bindgen objects. The fix pattern applies broadly.

**Low**: ROOT_CAUSE issue 9. No action needed; pst.rs gating is correct and binding-specific.
