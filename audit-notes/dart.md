# Dart Binding Hand-Edit Audit

This document catalogues hand-edits made to the Dart binding and e2e tests during the current alef-assisted development cycle, categorized for upstream submission to alef templates.

## ALEF_GAP — Missing template surfaces

### 1. Trait-bridge type stubs in traits.dart
**Location:** `packages/dart/lib/src/traits.dart` lines 679-689
**Changes:** Hand-added four type stubs required by e2e test fixtures:
- `OcrBackendType` enum (line 680) — tesseract, easyocr, paddleocr, rapidocr variants
- `ProcessingStage` enum (line 683) — preProcessing, processing, postProcessing variants
- `InternalDocument` class (line 686) — empty stub, used by DocumentExtractor trait bridge
- `SyncExtractor` abstract class (line 689) — empty stub, used by DocumentExtractor trait bridge

**Rationale:** These types are generated as part of the C FFI ABI but are not exposed in the public Dart surface because they're only used by test fixtures (e2e plugin_api_test.dart), not by public extraction functions. The alef generator strips them from lib.dart.

**Suggested upstream fix:** Alef should always generate trait-bridge type stubs (OcrBackendType, ProcessingStage, InternalDocument, SyncExtractor) into traits.dart, even if they're not referenced by public functions. This mirrors the trait bridge codegen pattern — the types are part of the plugin protocol contract and tests must be able to construct them.

---

### 2. EmbeddingConfig default values in wrapper methods
**Location:** `packages/dart/lib/src/kreuzberg.dart` lines 486-491, 529-534
**Changes:** Added inline EmbeddingConfig constructor with defaults:
```dart
config: config ?? EmbeddingConfig(
  model: EmbeddingModelType.preset(name: 'balanced'),
  normalize: true,
  batchSize: 32,
  showDownloadProgress: false,
)
```

**Rationale:** EmbeddingConfig struct fields have defaults in the Rust source, but the Dart FFI-generated constructor required them as positional/named parameters. The wrapper provides sensible fallbacks matching the Rust `EmbeddingConfig::default()` implementation.

**Suggested upstream fix:**
1. Alef should annotate `#[serde(default)]` fields in Rust structs and pass those annotations to the Dart codegen.
2. The `FrbDartOptionalFieldsWithDefaults` post-processor (currently invoked after FRB generation) should be enhanced to populate default values in wrapper methods, not just make fields optional in constructors.
3. Alternatively, store embedding defaults in a separate `EmbeddingConfigDefaults` factory function that alef generates alongside the config struct.

---

### 3. Async wrapper init functions for plugin trait bridge stubs
**Location:** `e2e/dart/test/plugin_api_test.dart` lines 64-75, 86-93, 110-123, 137-147, 157-163, 175-183
**Changes:** Converted trait bridge stub initialization from synchronous final values to async init functions:
```dart
late final DocumentExtractorDartImpl _TestStubRegisterDocumentExtractorTraitBridge_wrapped;

Future<void> _initTestStubRegisterDocumentExtractorTraitBridge() async {
  _TestStubRegisterDocumentExtractorTraitBridge_wrapped = await createDocumentExtractorDartImpl(
    // ... callbacks wrapped with Future.value() for sync methods
  );
}
```

And `setUpAll()` calls all init functions (lines 189-194).

**Rationale:** The FFI layer wraps all trait callbacks as async (Future-returning). Test stubs implement sync methods per the abstract class contract. The callbacks must be wrapped with `Future.value()` to convert sync returns to Futures. Static initialization (`final =`) cannot `await`, so init functions are required.

**Suggested upstream fix:** Alef e2e generator should emit async init functions for trait bridge stubs by default, not static final assignments. The generated fixture template should detect which trait methods are sync in the Dart stub class and automatically wrap them with `Future.value()` in the callback lambdas. Additionally, generate `setUpAll()` calls to initialize all wrapped impls before tests run.

---

## BINDING_BUG — Binding code issues (none)

No binding code bugs were found. All hand-edits are necessary adaptations to alef gaps, not fixes for incorrect Dart binding generation.

---

## TEST_FIXTURE — e2e generator issues

### 1. Plugin API e2e test stub class signatures mismatch
**Location:** `e2e/dart/test/plugin_api_test.dart` class stubs (lines 52-60, 78-82, 96-106, 126-133, 150-153, 166-171)
**Issue:** The generated abstract trait classes expect async methods, but test stub implementations were declared sync. For example:
- `DocumentExtractor.extractBytes()` signature expects `Future<InternalDocument>`
- Test stub implementation was `Future<InternalDocument> extractBytes(...) async => InternalDocument()`

This creates a signature mismatch: the trait bridge requires methods that return `Future<T>`, but tests provide sync implementations.

**Suggested fix:** The alef e2e generator should emit wrapper methods in test stub classes that return `Future<T>` even when the underlying implementation is sync, using `Future.value()` internally. Alternatively, regenerate the callback bindings to properly wrap sync returns.

---

## ROOT_CAUSE — Kreuzberg core or FRB codegen issues

### 1. Reserved-keyword collision: Uri → ExtractedUri
**Commit:** `5393349c7a` (fix(rust)!: rename Uri to ExtractedUri to avoid dart:core collision)
**Location:** Affects all bindings; Dart codegen regression
**Issue:** The Rust struct `Uri` collides with `dart:core.Uri` in flutter_rust_bridge-generated Dart bindings. The FRB codegen at `frb_generated.dart` line 41-46 declares:
```dart
packageRoot.resolve(...) returns dart:core.Uri
```
but was typed as the local Rust-derived `Uri` struct, producing 3 type-mismatch errors and blocking all 23 dart e2e tests.

**Fix applied:** Renamed Rust `Uri` to `ExtractedUri` throughout the crate, triggering a major breaking change (v5.0.0-rc cycle acceptable). All FFI bindings and language packages automatically inherit the renamed type.

**Impact on other bindings:** This is a polyglot issue affecting the C FFI ABI itself. The alef-generated bindings for all languages (Go, Java, C#, Dart, Swift, Zig, R) all regenerate with the new `ExtractedUri` type, ensuring consistency.

---

### 2. FRB codegen pipeline integration with alef post-processors
**Location:** `Taskfile.yml` dart:setup task, commit `177d8f3ee0`
**Issue:** The dart:setup task was calling both `task dart:codegen` (direct FRB invocation) and `alef build` (which also invokes FRB). The second invocation regenerated files and overwrote post-processor changes.

**Fix applied:** Remove the duplicate `dart:codegen` call; only `alef build` is responsible for FRB generation and post-processing in the correct order. This ensures:
- FRB runs once via alef
- Post-processor (`FrbDartOptionalFieldsWithDefaults`) runs after
- Changes persist in the committed bindings

**Suggested upstream fix:** Alef's Dart codegen integration should ensure post-processors are invoked atomically after FRB, with no separate regeneration steps that can overwrite changes. This is a tooling / CI/CD concern, not a hand-edit issue, but important for bindings stability.

---

### 3. FRB post-processor for optional fields with defaults
**Location:** `packages/dart/rust/src/frb_generated.rs` build script hook
**Issue:** The `FrbDartOptionalFieldsWithDefaults` post-processor transforms Rust struct fields marked with `#[serde(default)]` into optional Dart constructor parameters. Without it:
- `EmbeddingConfig(model: ..., normalize: ..., ...)` required all fields
- Tests failed on `EmbeddingConfig()` constructor calls without fields

**Fix applied:** The post-processor runs during `alef build` and makes fields optional in generated code:
```dart
// Before:
class EmbeddingConfig {
  final EmbeddingModelType model;  // required
  final bool normalize;            // required
  // ...
}

// After (via post-processor):
class EmbeddingConfig {
  final EmbeddingModelType? model;  // optional
  final bool? normalize;            // optional
  // ...
}
```

**Suggested upstream fix:** This should be an alef-native feature, not a post-processor hack. Alef should read Rust `#[serde(default)]` and `#[serde(default = "...")]` attributes and emit optional Dart fields automatically. This eliminates the need for external post-processors and ensures consistency across all language bindings.

---

## Summary

| Category | Count | Details |
|----------|-------|---------|
| ALEF_GAP | 3 | Trait-bridge type stubs, EmbeddingConfig defaults, async wrapper init template |
| BINDING_BUG | 0 | — |
| TEST_FIXTURE | 1 | Plugin API e2e stub class async/sync signature mismatch |
| ROOT_CAUSE | 3 | Uri→ExtractedUri collision, FRB post-processor integration, FRB codegen ordering |

---

## Flutter_rust_bridge codegen notes

1. **Post-processor execution order:** The `FrbDartOptionalFieldsWithDefaults` post-processor must run after FRB generation, not before. Alef should orchestrate this as an atomic step to prevent regeneration from overwriting changes.

2. **Single FRB invocation requirement:** Do not invoke FRB multiple times in the same build. Each invocation regenerates files and can lose post-processor changes. Alef should coordinate FRB + post-processor as a single unit of work.

3. **Serde attribute propagation:** The Dart codegen should read Rust `#[serde(default)]` and `#[serde(default = "fn")]` attributes and emit optional Dart constructor parameters matching those defaults. This is a FRB feature request, not an alef issue, but critical for bindings that expose structured configs.

4. **Async callback wrapping in trait bridges:** When Dart trait bridge stubs implement sync methods but the Rust trait expects async callbacks, the wrapper layer (createDocumentExtractorDartImpl, etc.) must provide `Future.value()` adapters. The e2e generator should emit these adapters automatically in test fixtures.

---

## Reserved-keyword collisions

Beyond `Uri` → `ExtractedUri`, audit the Rust public API for other collisions with `dart:core` symbols:

| Rust type | Dart collision | Potential issue |
|-----------|----------------|-----------------|
| `Uri` | `dart:core.Uri` | **FIXED**: renamed to `ExtractedUri` |
| `Duration` | `dart:core.Duration` | Check if used in public API; FRB would collide |
| `List` | `dart:core.List` | Generic type param; unlikely collision if used as struct field name only |
| `Map` | `dart:core.Map` | Generic type param; unlikely collision if used as struct field name only |
| `Set` | `dart:core.Set` | Unlikely if not used as public struct name |
| `String` | `dart:core.String` | FRB maps to Dart String; aliasing would conflict |
| `Error` / `Exception` | `dart:core.Error` / `Exception` | Check if used as public enum variant or struct name |

**Recommendation:** Run the kreuzberg e2e suite with `dart analyze packages/dart` regularly. FRB type collisions produce clear compile-time errors, so the test suite serves as a collision detector. No preemptive renaming needed unless a collision is encountered.

---

## Remaining hand-edits for future consideration

None at this time. All current hand-edits are necessary gaps or root causes that should be addressed upstream in:
- Alef template generation for trait-bridge type stubs and async wrappers
- Alef serde attribute propagation to Dart optional field defaults
- Alef FRB post-processor integration and ordering
- Flutter_rust_bridge serde attribute support and async callback adaptation
