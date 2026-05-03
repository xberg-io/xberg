# Cycle 5 Alef Queue — Go E2E Test Failures

Date: 2026-05-03
Regenerated with: alef v0.14.3
Target: Drive Go e2e test suite to 100% green

## Bucket A — Alef Codegen Bugs (alef-side fixes)

Note: original go-agent triage misattributed these to "missing fixtures". Verified against `fixtures/batch/batch_bytes_invalid_mime.json` — fixtures are present and correctly structured. Real cause: alef-backend-go's e2e codegen.

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

Issues in test fixtures (tools/benchmark-harness/fixtures/*.json) or alef.toml call overrides.

(To be updated as failures are discovered and triaged)

## Bucket C — Kreuzberg Core Bugs

Issues in kreuzberg core (crates/kreuzberg/src/) requiring code changes.

(To be updated as failures are discovered and triaged)

## Analysis

### Root Cause: Fixture-Test Mismatch

The Go e2e test suite cannot compile due to alef v0.14.3 codegen bugs. The issue stems from `alef.toml [crates.e2e]` pointing to benchmark-harness fixtures that:

1. Have categories like "image", "markup", "archive" (document classification), NOT "batch", "cache_operations", "contract", etc. (API/feature classification)
2. Don't include explicit "input.items" arrays for batch operations
3. Map to single-file extraction tests, not multi-file batch tests

Alef should either:

- Skip fixture categories with missing mappings, OR
- Provide explicit e2e fixtures in a separate directory with proper structure
- Or validate fixture structure before code generation

### Workaround Attempts

- Cannot edit `e2e/go/*_test.go` (auto-generated, explicit task constraint)
- Cannot rebuild without these broken test categories since alef generates them from fixture categories found
- Cannot create fixtures that would generate correct Go code (alef codegen patterns are defined in alef, not by fixture structure)

## Test Summary

- Total: Unable to determine (test suite doesn't compile)
- Passed: 0 (compilation failure)
- Failed: Compilation error in batch_test.go, cache_operations_test.go
- Skipped: All (due to compilation failure)

### Compilation Errors by Category

#### batch_test.go (6 failures)

- Line 20: Unmarshal target type is `[]string` instead of `[]BatchBytesItem`
- Lines 23-63: Multiple calls pass wrong types (nil strings, wrong array types)

#### cache_operations_test.go (4 failures)

- Lines 20, 29, 37, 45: Pass `nil` where string path expected
- Line 24: Accesses non-existent field `.Result` on ExtractionResult

#### Other test files

- Presumed working but untestable due to compilation failure
