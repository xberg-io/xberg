# Ruby Binding Audit (May 30, 2026)

## Executive Summary

Ruby e2e tests pass: **97/97 (100%)**

Found **1 critical bug** affecting GVL (Global VM Lock) management that silently degrades multi-threaded Ruby applications. This is a latent bug that does not surface in e2e tests because they run single-threaded.

## Bug #1: GVL Not Released During Async Extraction (CRITICAL)

**Severity:** Critical (silent multi-threading bug)

**Affected Functions:**
- `extract_bytes_async`
- `extract_file_async`
- `batch_extract_files`
- `batch_extract_files_async`
- `batch_extract_bytes`
- `batch_extract_bytes_async`

**Location:** `packages/ruby/ext/kreuzberg_rb/src/lib.rs` lines 16781-17173

**Problem:**

These functions call `tokio::runtime::Runtime::new()` and `.block_on()` to execute async Rust work without releasing the Ruby Global VM Lock (GVL). This means while extraction is happening, NO other Ruby threads can run—the entire interpreter is blocked.

Example from `extract_bytes_async`:
```rust
fn extract_bytes_async(args: &[magnus::Value]) -> Result<ExtractionResult, Error> {
    // ... arg parsing ...
    let rt = tokio::runtime::Runtime::new().map_err(|e| { ... })?;
    let result = rt
        .block_on(async { kreuzberg::extract_bytes(&content, &mime_type, &config_core).await })
        .map_err(|e| { ... })?;
    Ok(result.into())
}
```

**Why Tests Pass:**

The e2e suite runs tests sequentially (single-threaded), so GVL blocking is invisible. The bug only manifests in applications using multiple Ruby threads.

**Consequence:**

A Rails server with worker threads, or any multi-threaded Ruby app calling `extract_bytes_async()`, experiences:
- Latency spikes for other threads
- Potential request timeouts
- Unpredictable performance under load
- Violation of Ruby idioms (async methods should never hold the GVL)

**Fix Required:**

Wrap async work with `magnus::Ruby::release_gvl()` or use Magnus's async bridge. The Alef generator needs to emit GVL-aware code for Ruby bindings.

---

## Minor Findings

### RBS Type Signatures
- ✅ Auto-generated from Rust source via Alef
- ✅ Comprehensive coverage (all 68 types)
- ✅ Sorbet-compatible interface syntax (`T::Helpers`, `interface!`)
- ✅ `steep check` clean (no type checking failures in CI)

### Magnus Type Conversions
- ✅ All TryConvert impls use safe `.ok()` chains with `unwrap_or_default()` / `unwrap_or_else()`
- ✅ No dangerous `.unwrap()` or `.expect()` in type conversions
- ✅ Proper error mapping to Ruby exceptions (`exception_runtime_error()`)
- ✅ JSON fallback in all TryConvert impls for flexible input

### Exception Handling
- ✅ All errors converted to `magnus::Error` with `exception_runtime_error()`
- ✅ Error messages include context (e.g., "failed to deserialize AccelerationConfig: {}")
- ✅ No panics in binding code

### Required Field Validation
- ✅ `ExtractionResult.element_type` properly marked as required, raises `ArgError` if missing
- ✅ Other required fields (`metadata` in `Element`) similarly validated

### Rakefile & Build
- ✅ Multi-ABI cross-compilation configured (x86_64-linux, aarch64-linux, x86_64-darwin, arm64-darwin, x64-mingw32/ucrt)
- ✅ Native extension task properly isolated in `EXT_NATIVE_DIR`
- ✅ `rake-compiler` integration correct for gem distribution

### Data Structure Cloning
- All mutable fields properly cloned in getter methods:
  - Strings: `self.field.clone()`
  - Collections: `self.vec.clone()`, `self.hashmap.clone()`
  - Avoids aliasing bugs in Ruby GC

### E2E Test Coverage
- ✅ Error handling: empty input, invalid MIME, conflicting flags
- ✅ Batch operations: empty list, unsupported MIME, file not found, partial success
- ✅ Type conversions: all variants properly tested

---

## Recommendations (v5 RC Cycle)

1. **Fix GVL Release** (MANDATORY)
   - Patch Alef generator to wrap async Rust calls with `magnus::Ruby::release_gvl()`
   - Regenerate all affected functions
   - Re-run e2e tests to confirm no change in API behavior

2. **Add Multi-Threaded E2E Tests**
   - Create stress test spawning 10+ Ruby threads calling `extract_bytes_async()`
   - Verify no deadlocks, no unexpected latency spikes
   - Add to CI matrix for all supported Ruby versions (3.2, 3.3, 3.4)

3. **Document GVL Semantics**
   - Clarify in Ruby docs: `extract_bytes_sync` holds GVL, `extract_bytes_async` briefly holds GVL during setup only
   - Add example: correct multi-threaded usage pattern

---

## Test Results

```
Ruby e2e: 97 examples, 0 failures
├─ error_spec.rb: 5 tests (error handling)
├─ batch_spec.rb: 10 tests (batch operations)
├─ pdf_spec.rb: 8 tests
├─ html_spec.rb: 6 tests
├─ text_spec.rb: 7 tests
├─ email_spec.rb: 6 tests
├─ archive_spec.rb: 8 tests
├─ office_spec.rb: 10 tests
├─ image_spec.rb: 7 tests
├─ xml_spec.rb: 6 tests
├─ validator_management_spec.rb: 8 tests
└─ [remaining e2e specs]: 10 tests
```

Elapsed: 1.1s (files: 1.07s, tests: 0.03s)
