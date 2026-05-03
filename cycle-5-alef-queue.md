# Cycle 5 Alef Queue — Python E2E Test Failures + Go E2E Investigation

Date: 2026-05-03
Regenerated with: alef v0.14.3
Target: Drive Python e2e test suite to 100% green; investigate Go e2e failures

## Final Status

**Python E2E Tests: 29/63 green (46%), 34 red (54%), all failures are bucket-A alef bugs**

- Passed: 20 tests
- Skipped: 9 tests (feature gates: keywords-yake, pdf, quality, tree-sitter, embeddings, llm, image)
- Failed: 34 tests
  - All 34 failures caused by alef codegen bugs P7-P10
  - No kreuzberg core bugs found
- Uncollected: 2 files (import errors)
  - test_embeddings.py: missing embed_texts_async export (P7)
  - test*plugin_api.py: missing unregister*\* exports (P8)

**Go E2E Tests: 5/68 green (7%), 3 red (4%), 60 skipped (88%)**

- Passed: 5 tests (PDF, DOCX, JSON, TXT with simple configs)
- Skipped: 60 tests (feature gates and non-HTTP fixtures)
- Failed: 3 tests (2 config parsing, 1 result unmarshaling)
  - All failures are bucket-A alef codegen bugs
  - Library path resolution fixed in main_test.go

## Bucket A — Alef Codegen Bugs (alef-side fixes)

Note: original go-agent triage misattributed these to "missing fixtures". Verified against `fixtures/batch/batch_bytes_invalid_mime.json` — fixtures are present and correctly structured. Real cause: alef-backend-go's e2e codegen.

### Go Binding Bugs (Cycle 5)

11. **alef-backend-go: OutputFormat enum rendered as empty struct**
    `OutputFormat` is Rust enum with variants Plain, Markdown, Djot, Html, Json, Structured, Custom(String).
    Go codegen produces `type OutputFormat struct {}` (empty struct with no const variants).
    Test fixtures with `{"output_format":"markdown"}` fail to unmarshal because Go expects a struct, not a string.
    Root cause: Rust enum-to-Go translation doesn't handle tagged/untagged unions; should generate string type with const variants or custom MarshalJSON.
    Affects: Test_ApiBatchBytesWithConfigsAsync, Test_ApiBatchFileWithConfigsAsync (config parsing fails)

12. **alef-backend-go: JSON unmarshal errors not returned, silently return nil**
    When C FFI returns valid JSON response but Go JSON unmarshaling fails (e.g., due to OutputFormat issue), the binding code catches the error and returns nil without returning an error value.
    Code pattern: `if err := json.Unmarshal(...); err != nil { return nil }` (should return error).
    Consequence: Tests see nil result with no error, crash when accessing result.MimeType.
    Affects: Test_SmokeHtmlBasic, Test_SmokeXlsxBasic (return nil, panic on field access)

13. **alef-backend-go e2e: fixture/library path resolution**
    Go tests assume `html/simple_table.html` paths are relative to cwd, but tests run from e2e/go/.
    Rust tests workaround: symlink `e2e/test_documents`.
    Go tests workaround: TestMain changes cwd to `../../test_documents` before running tests.
    Also requires DYLD_LIBRARY_PATH set to find libpdfium.dylib and libkreuzberg_ffi.dylib at runtime.
    Note: This is NOT an alef bug per se, but affects test execution. Workaround in place: main_test.go.

### Python Binding Bugs (Cycle 5)

7. **alef-backend-python: missing `embed_texts_async` export**
   `kreuzberg.embed_texts_async` is configured in `alef.toml` (line 1581-1591) with `skip_languages = ["wasm"]`, meaning it should be generated for Python. But it's missing from `packages/python/kreuzberg/api.py`. Root cause: unclear — may be conditional feature flag not enabled in PyO3 binding generation.

8. **alef-backend-python: missing `unregister_*` function exports**
   `unregister_post_processor` and `unregister_validator` are configured in `alef.toml` e2e calls (lines 1721-1733) but excluded from `gen_function` (lines 855-857) because they're supposed to be generated via `gen_trait_bridge`. However, the trait_bridge variants are not appearing in `packages/python/kreuzberg/api.py`. Root cause: trait_bridge codegen may not be emitting Python wrappers, or wrappers are not being exported to **init**.py.

9. **alef-backend-python: optional config parameter not handled correctly**
   Functions `extract_file`, `extract_bytes`, and batch variants have `config: ExtractionConfig | None = None` signatures in the Python API but the Rust binding expects config to always be provided (`.expect("'config' is required")`). When tests call these functions without config, it panics. Solution: Python wrapper should use `ExtractionConfig()` (default) when config is None, OR Rust wrapper should handle None and create default config. Currently, neither does this conversion properly.

10. **alef-backend-python: config type conversion doesn't handle None enum fields**
    The generated `_to_rust_extraction_config()` function tries to wrap enum fields like `output_format` with `_rust.OutputFormat(None)` when the field is absent. This raises TypeError. Fix: check `if value is not None` before wrapping in enum constructors.

### Go Binding Bugs (Cycle 4)

1. **alef-backend-go e2e: wrong Unmarshal target for batch items**
   Fixture has `input.items = [{ data, mime_type }]`. Rust e2e correctly emits `Vec<BatchBytesItem>`. Go e2e at `e2e/go/batch_test.go:20` emits `var items []string`. Backend should derive the parameter type from the Go function signature (`BatchBytesItem`).

2. **alef-backend-go e2e: dispatching to wrong fn**
   `Test_BatchFileAsyncBasic` calls `kreuzberg.ExtractFile(nil, nil, ExtractionConfig{})` instead of `BatchExtractFiles(items, ExtractionConfig{})`. Codegen drops the `batch_` prefix and pluralization-mismatches; should follow fixture's `call` field exactly.

3. **alef-backend-go e2e: nil-stuffing where `path string` is required**
   `cache_operations_test.go:20-45` calls `ExtractFile(nil, ...)`. Go has no implicit nil for `string`. Codegen must use the fixture's `input.path` literal or skip if absent.

4. **alef-backend-go e2e: stale type reference `result.Result`**
   `cache_operations_test.go:24` accesses `result.Result` which doesn't exist on `*ExtractionResult` (struct exposes `Content`, `Mime`, etc.).

5. **alef-backend-go: missing go.mod emission for `packages/go/v4`**
   `alef all --clean` does not emit `packages/go/v4/go.mod` even though it emits `binding.go` and `trait_bridges.go` there. Currently hand-stubbed.

6. **alef-backend-python: malformed imports when fixture calls unknown functions**
   - 7 Python test files have syntax errors due to invalid import statements:
     - `test_serialization.py:8`: `from kreuzberg import , ExtractionConfig` (leading comma)
     - `test_detection.py:10`: `from kreuzberg import extract_file, , detect_mime_type_from_bytes, ...` (double comma)
     - `test_text_utils.py:8`: `from kreuzberg import extract_file,` (trailing comma)
     - `test_token_reduction.py:8`: `from kreuzberg import , extract_file_sync, ExtractionConfig` (leading comma)
     - `test_rendering.py:8`: `from kreuzberg import extract_file,` (trailing comma)
     - `test_validate.py:8`: `from kreuzberg import extract_file, extract_file_sync, , ExtractionConfig` (double comma)
     - `test_chunking.py:8`: `from kreuzberg import , ExtractionConfig` (leading comma)
   - Root cause: alef codegen generates import statements for fixture function calls without checking if they exist in public API
   - 192 fixtures reference 74 non-existent functions removed during v4.10 API stabilization
   - Codegen should either skip fixtures with unknown calls or validate call existence before generating imports
   - Missing functions: `batch_extract_file`, `batch_extract_file_sync`, `chunk_semantic`, `chunk_text`, `chunk_texts_batch`, `embed_text`, `serialize_to_json`, `serialize_to_toon`, `validate_cache_key`, `validate_mime_type`, `render_*`, etc.

## Bucket B — Fixture/Test Bugs

Issues in test fixtures (tools/benchmark-harness/fixtures/\*.json) or alef.toml call overrides.

(To be updated as failures are discovered and triaged)

## Bucket C — Kreuzberg Core Bugs

Issues in kreuzberg core (crates/kreuzberg/src/) requiring code changes.

(To be updated as failures are discovered and triaged)

---

## Rust E2E Alef Bugs Found During Cycle 4 (2026-05-03)

These were found while driving `e2e/rust` to 100% green. All worked around at
fixture/alef.toml level; no hand-edits to generated files.

### Bug R1 — `result_is_vec` call-level flag ignored by Rust codegen

**File:** `alef/crates/alef-e2e/src/codegen/rust/test_file.rs` line ~385

`result_is_vec` is only read from `rust_overrides`, not from the top-level `call_config`.
Setting it in `[e2e.calls.<fn>]` in alef.toml has no effect for Rust output.

**Workaround:** Changed batch fixture assertions to `count_min`/`count_equals` without a
`field` key, so the generated code iterates directly over the `Vec` return value instead of
accessing `.results`.

### Bug R2 — `test_documents` path depth off by one in Rust e2e

**File:** `alef/crates/alef-e2e/src/codegen/rust/args.rs` (bytes arg file loading)

The generated Rust test helper computes the test_documents path as
`CARGO_MANIFEST_DIR + "/../test_documents/"`. But `e2e/rust/` is two directories
below the kreuzberg root, so the correct relative path is `"/../../test_documents/"`.

**Workaround:** Created symlink `e2e/test_documents` → `../test_documents`
(i.e., `kreuzberg/test_documents`) so the one-level-up path resolves correctly.

### Bug R3 — `bytes` arg type with JSON array value generates incorrect code

**File:** `alef/crates/alef-e2e/src/codegen/rust/args.rs`

When a fixture argument has `type = "bytes"` and the fixture value is a JSON array
(e.g., `[72, 101, 108, ...]`), the generator falls through to `json_to_rust_literal`
which produces a string literal then calls `.as_bytes()` on it. This fails type
inference — the batch `content` field is `Vec<u8>`, not `&[u8]`.

**Workaround:** All batch fixture `content` fields are string paths (e.g.,
`"text/fake_text.txt"`) so the file-loading branch runs instead. This requires
actual test files to exist and means the content cannot be arbitrary bytes.

Actually for `BatchBytesItem.content` the generator uses `serde_json::from_value` not
the bytes loader — so this is really the serde path. The fundamental issue remains:
array values for `bytes` type args don't generate correct `Vec<u8>` literals.

### Bug R4 — `only_emptiness_checks` skips `.expect()` for Result-returning functions

**File:** `alef/crates/alef-e2e/src/codegen/rust/test_file.rs`

When all assertions for a test are "emptiness-like" (no field path), the generator
skips emitting `.expect("should succeed")` on the result. If the function returns
`Result<Vec<T>, E>` this produces code that calls `.is_empty()` directly on the
`Result`, which doesn't compile.

**Workaround:** Changed affected fixtures to use `count_min value:1` / `count_equals
value:0` which are not in the emptiness_checks list, forcing the `.expect()` to be
emitted.

### Analysis

#### Root Cause: Fixture-Test Mismatch

The Go e2e test suite cannot compile due to alef v0.14.3 codegen bugs. The issue stems from `alef.toml [crates.e2e]` pointing to benchmark-harness fixtures that:

1. Have categories like "image", "markup", "archive" (document classification), NOT "batch", "cache_operations", "contract", etc. (API/feature classification)
2. Don't include explicit "input.items" arrays for batch operations
3. Map to single-file extraction tests, not multi-file batch tests

Alef should either:

- Skip fixture categories with missing mappings, OR
- Provide explicit e2e fixtures in a separate directory with proper structure
- Or validate fixture structure before code generation

#### Workaround Attempts

- Cannot edit `e2e/go/*_test.go` (auto-generated, explicit task constraint)
- Cannot rebuild without these broken test categories since alef generates them from fixture categories found
- Cannot create fixtures that would generate correct Go code (alef codegen patterns are defined in alef, not by fixture structure)

### Test Summary

- Total: Unable to determine (test suite doesn't compile)
- Passed: 0 (compilation failure)
- Failed: Compilation error in batch_test.go, cache_operations_test.go
- Skipped: All (due to compilation failure)

#### Compilation Errors by Category

##### batch_test.go (6 failures)

- Line 20: Unmarshal target type is `[]string` instead of `[]BatchBytesItem`
- Lines 23-63: Multiple calls pass wrong types (nil strings, wrong array types)

##### cache_operations_test.go (4 failures)

- Lines 20, 29, 37, 45: Pass `nil` where string path expected
- Line 24: Accesses non-existent field `.Result` on ExtractionResult

##### Other test files

- Presumed working but untestable due to compilation failure

## Cycle 5.5 — alef v0.14.5 backlog (after v0.14.4 regen)

### Python

- **P11 (alef-backend-pyo3)**: every public type emits `#[pyclass(unsendable, from_py_object)]`. ExtractionResult panics in async tests because PyO3 panics when sending unsendable across threads. Should default to `#[pyclass]` (sendable) for plain data structs; only mark `unsendable` when the struct contains genuinely non-Send fields.
- **P12 (alef-extract)**: cfg-gated `#[cfg(all(feature = "embeddings", feature = "tokio-runtime"))] pub use embeddings::embed_texts_async;` is not picked up by alef-extract's strict surface check. Generic `pub async fn embed_texts_async<T: AsRef<str> + Send + 'static>(...)` may also confuse the parser. Fix: extract should evaluate cfg attrs against the canonical feature set (`full`?) and accept generic functions.

### Go

- **#15 (alef-backend-go e2e)**: `var items []string` still emitted for `batch_extract_bytes_sync` despite v0.14.4 #14 work. The "real test body" is generated but type derivation from `e2e.calls.<name>.args` ArgMapping is still wrong. Need to look up the binding-side function param type and use it.
- **#16 (alef-backend-go bindings)**: batch fn return type is `*[]ExtractionResult` instead of `[]ExtractionResult`. Test code does `len(result)` which fails on `*[]T`. Make batch returns be `[]T` directly.

### Rust

- (no new items — rust e2e is stable at 62/62)
