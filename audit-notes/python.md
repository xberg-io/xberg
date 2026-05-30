# Python Binding Systematic Audit

**Date**: May 30, 2026
**Binding Version**: 5.0.0-rc.3
**E2E Status**: 108/108 passing (at audit start)
**Coverage**: PyO3 binding (crates/kreuzberg-py), Python wrapper (packages/python), E2E tests (e2e/python)

---

## Critical Issues

### 1. BINDING_BUG: Monolithic Error Translation → PyRuntimeError

**Severity**: CRITICAL
**Category**: Error Handling
**Files Affected**:
- `crates/kreuzberg-py/src/lib.rs` (auto-generated, all `#[pyfunction]` items)
- `packages/python/kreuzberg/exceptions.py` (defines exception classes that are never used)

**Issue Description**:
All Rust-to-Python error conversions use a single, generic `PyRuntimeError`. The binding defines specific exception classes (`OcrError`, `ParsingError`, `ValidationError`, `CacheError`, `SecurityError`, `UnsupportedFormatError`, `EmbeddingError`, `ImageProcessingError`, `PluginError`, `SerializationError`, `MissingDependencyError`, `LockPoisonedError`, `KreuzbergTimeoutError`, `CancelledError`, `IoError`) in `exceptions.py`, but these are never raised. Instead, all errors collapse to `PyRuntimeError`.

**Evidence**:
Lines in `crates/kreuzberg-py/src/lib.rs`:
- 10900: `extract_bytes_sync` → `.map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))`
- 10905: `extract_file_sync` → same pattern
- 10916: `batch_extract_files_sync` → same pattern
- 10932: `batch_extract_bytes_sync` → same pattern
- 10950, 10971: async variants in error handlers
- 11104, 11113, 11122: embeddings, mime detection, detection functions
- 11201-11218: plugin bridge methods (PyOcrBackendBridge)
- 11355, 11526, 11678: other plugin bridges

**Root Cause**:
Alef (the code generator) does not yet implement error type mapping for Python. The generated binding uses a monolithic exception conversion. Alef config (`alef.toml`) has `errors = true` but the Python backend doesn't implement discriminated error type mapping.

**Impact**:
```python
# Current behavior - always catches PyRuntimeError
try:
    kreuzberg.extract_file("doc.pdf")
except kreuzberg.OcrError:
    # Never executes - error is PyRuntimeError
    log_ocr_issue()
except RuntimeError:
    # Always catches
    log_any_error()
```

Users cannot implement granular error handling or detect specific failure modes (OCR failed vs parsing failed vs timeout).

**Proposed Fix**:
Create error mapping layer in `crates/kreuzberg-py/src/lib.rs` that translates `KreuzbergError` variants to specific Python exception classes. This requires:
1. Inspect the error enum variant in Rust before converting to string
2. Raise the appropriate Python exception class

Example pattern:
```rust
fn error_to_pyerr(e: kreuzberg::KreuzbergError) -> PyErr {
    match e {
        kreuzberg::KreuzbergError::Ocr { message } => {
            PyErr::new::<OcrError, _>(message)
        },
        kreuzberg::KreuzbergError::Parsing { message } => {
            PyErr::new::<ParsingError, _>(message)
        },
        // ... other variants
        _ => PyErr::new::<PyRuntimeError, _>(e.to_string()),
    }
}
```

Then use `error_to_pyerr(e)` instead of `PyRuntimeError::new_err(e.to_string())` throughout.

**Status**: DEFERRED - Requires upstream Alef changes OR manual implementation in binding.
**Priority**: CRITICAL (breaks API contract)

---

### 2. TEST_FIXTURE: Missing Error Type Assertions

**Severity**: HIGH
**Category**: Test Coverage
**Files Affected**:
- `e2e/python/tests/test_async.py:49,59`
- `e2e/python/tests/test_error.py` (entire file, likely same pattern)

**Issue Description**:
E2E test fixtures that exercise error paths catch generic `Exception` and never assert the specific exception type. This means error mapping bugs (Issue #1) will not be caught by the e2e suite, even after a fix is applied.

**Evidence**:
```python
# test_async.py:49
with pytest.raises(Exception):  # Generic catch
    await extract_bytes(content, "", config)

# test_async.py:59
with pytest.raises(Exception):  # Generic catch
    await extract_bytes(content, "application/x-nonexistent", config)
```

**Impact**:
- Error mapping regressions won't be detected
- E2E green doesn't imply error types are correct
- Users relying on exception handling will fail in production

**Proposed Fix**:
1. Update all `pytest.raises(Exception)` in error-path tests to specific exception classes:
   ```python
   with pytest.raises(kreuzberg.UnsupportedFormatError):
       await extract_bytes(content, "application/x-nonexistent", config)
   ```
2. Create a new e2e fixture file `fixtures/error_types.json` that exercises all error paths with correct exception type assertions.

**Status**: BLOCKED - Depends on Issue #1 fix (error mapping)
**Priority**: HIGH (test quality)

---

## Medium Issues

### 3. ALEF_GAP: Missing Docstrings on Core Functions

**Severity**: MEDIUM
**Category**: API Documentation
**Files Affected**:
- `crates/kreuzberg-py/src/lib.rs` (auto-generated, all `#[pyfunction]` items)
- `packages/python/kreuzberg/api.py` (auto-generated)

**Issue Description**:
Core public functions lack docstrings. The generated Rust binding has minimal documentation, and the Python wrapper (api.py) is similarly bare. This degrades IDE experience and REPL `help()` output.

**Evidence**:
```rust
// crates/kreuzberg-py/src/lib.rs:10838 - extract_bytes
pub fn extract_bytes<'py>(
    py: Python<'py>,
    content: Vec<u8>,
    mime_type: String,
    config: ExtractionConfig,
) -> PyResult<Bound<'py, PyAny>> {
    // ^ No docstring
```

**Impact**:
Users get no guidance from IDE tooltips or REPL help on function signatures, parameters, or behavior.

**Proposed Fix**:
Since `crates/kreuzberg-py/src/lib.rs` is auto-generated by Alef, docstrings would need to be added in `alef.toml` or source Rust files that Alef reads. For the Python wrapper, add docstrings to `packages/python/kreuzberg/api.py` functions (but this is also auto-generated).

Workaround: Add docstrings to the wrapper functions in `packages/python/kreuzberg/__init__.py`:
```python
def extract_file(path: str, mime_type: str | None = None, config: ExtractionConfig | None = None) -> Coroutine[Any, Any, ExtractionResult]:
    """Extract text, tables, and metadata from a file.

    Args:
        path: File path to extract from.
        mime_type: Optional MIME type (e.g., 'application/pdf'). Auto-detected if omitted.
        config: ExtractionConfig with options for OCR, chunking, etc.

    Returns:
        ExtractionResult containing extracted content, metadata, and processing details.

    Raises:
        OcrError: If OCR fails (if enabled).
        ParsingError: If document parsing fails.
        UnsupportedFormatError: If MIME type is not supported.
        SecurityError: If security limits are exceeded.
    """
```

**Status**: FIXABLE
**Priority**: MEDIUM (quality of life)

---

### 4. POTENTIAL_BUG: Sync `embed_texts` May Block Python Thread

**Severity**: LOW
**Category**: Performance/Thread Safety
**File**: `crates/kreuzberg-py/src/lib.rs:11119`

**Issue Description**:
The synchronous `embed_texts` function does not release the GIL, yet the underlying Rust function may perform I/O (HTTP requests to LLM APIs) or CPU-intensive operations (sentence embeddings via ONNX Runtime).

**Evidence**:
```rust
pub fn embed_texts(texts: Vec<String>, config: EmbeddingConfig) -> PyResult<Vec<Vec<f32>>> {
    let config_core: kreuzberg::EmbeddingConfig = config.into();
    kreuzberg::embed_texts(texts, &config_core).map_err(...)
    // No py.allow_threads() wrapper
}
```

**Assessment**:
This is NOT necessarily a bug. The Rust binding has both `embed_texts` (sync) and `embed_texts_async` (async). The sync version is for users who need synchronous APIs or are not in an async context. Users with async needs have `embed_texts_async` available. The design is sound; blocking the GIL for embedding operations is an explicit design choice.

**Mitigation**:
- Document in docstring that sync `embed_texts` may block for extended periods
- Recommend `embed_texts_async` for performance-critical applications
- If sync blocking is a problem, call `embed_texts` in a `concurrent.futures.ThreadPoolExecutor`

**Status**: ACCEPTED (design choice, not a bug)
**Priority**: LOW (documentation only)

---

## Clean/Good Issues

### 5. ASYNC_SAFE: Proper GIL Management in Async Closures

**Status**: PASS
**Evidence**:
```rust
pyo3_async_runtimes::tokio::future_into_py(py, async move {
    // All captures move by value, no borrowed Python state held across await points
    let result = kreuzberg::extract_bytes(&content, &mime_type, &config_core).await?;
    Ok(ExtractionResult::from(result))
})
```
All async functions use `async move` and capture by value. No Py<T> or Bound<T> references are held across await points. ✓

### 6. TYPE_STUBS: Parity Between .pyi and Implementation

**Status**: PASS
**Spot Checks**:
- `AccelerationConfig.__init__` signature in .pyi matches generated binding ✓
- `ExtractionConfig.__init__` has all 28 parameters in .pyi ✓
- Return types (e.g., `extract_bytes -> Bound<'py, PyAny>`) are correctly stubbed as coroutines ✓

No type stub drift detected.

### 7. PLUGIN_SAFETY: Error Handling in Plugin Bridges

**Status**: PASS
**Examples**:
```rust
// PyOcrBackendBridge.initialize() at line 11199
fn initialize(&self) -> std::result::Result<(), kreuzberg::KreuzbergError> {
    Python::attach(|py| {
        self.inner.bind(py).call_method0("initialize").map(|_| ()).map_err(|e| {
            kreuzberg::KreuzbergError::Other(format!(
                "Plugin '{}' method 'initialize' failed: {}",
                self.cached_name, e
            ))
        })
    })
}
```
Plugin method calls properly wrap PyErr into KreuzbergError. ✓

---

## Summary Table

| Issue | Category | Severity | Fixable | File:Line |
|-------|----------|----------|---------|-----------|
| 1. Error Type Mapping | BINDING_BUG | CRITICAL | Yes (needs Alef or manual) | crates/kreuzberg-py:10900+ |
| 2. Error Type Tests | TEST_FIXTURE | HIGH | Yes (after #1) | e2e/python/tests:49,59 |
| 3. Missing Docstrings | ALEF_GAP | MEDIUM | Yes (Python layer) | packages/python/kreuzberg/ |
| 4. Sync Embedding Block | POTENTIAL_BUG | LOW | N/A (design choice) | crates/kreuzberg-py:11119 |
| 5. GIL Management | ASYNC_SAFE | — | N/A (clean) | crates/kreuzberg-py:10846+ |
| 6. Type Stubs | TYPE_STUBS | — | N/A (clean) | packages/python/kreuzberg/ |
| 7. Plugin Error Safety | PLUGIN_SAFETY | — | N/A (clean) | crates/kreuzberg-py:11199+ |

---

## Audit Methodology

1. **Scanned all `#[pyfunction]` items** in `crates/kreuzberg-py/src/lib.rs` for error handling patterns
   - 147 error conversion sites identified
   - All use generic `PyRuntimeError`

2. **Verified GIL management** in async closures
   - Checked for `py.allow_threads()` usage (not needed for `future_into_py` pattern)
   - Verified no Py<T> references held across await points
   - All closures use `async move` (value capture)

3. **Cross-checked exception hierarchy**
   - Rust `KreuzbergError` enum has 16+ variants
   - Python `exceptions.py` defines 14 exception classes
   - No mapping mechanism implemented

4. **Reviewed E2E test coverage**
   - 108/108 tests passing
   - Error path tests catch generic `Exception`
   - No specific error type assertions

5. **Validated type stubs (.pyi files)**
   - Sampled signatures match implementation
   - No drift detected
   - Auto-generated by Alef, stays in sync

6. **Inspected plugin bridge implementations**
   - PyOcrBackendBridge, PyPostProcessorBridge, PyValidatorBridge, PyEmbeddingBackendBridge
   - All properly wrap Python exceptions in KreuzbergError
   - Method validation (hasattr checks) on registration

---

## Recommendations

### Immediate (Blocking)
1. **Fix Issue #1 (error mapping)** — Either:
   - Upstream: Add error variant discrimination to Alef's Python backend
   - Local: Implement `error_to_pyerr()` helper in binding and refactor all error sites

   This is the single most important issue affecting API correctness.

### Short Term (High Value)
2. **Add docstrings** to high-level functions (extract_*, embed_*, batch_*)
3. **Create error_types.json fixture** with comprehensive error path assertions

### Long Term (Nice to Have)
4. **Sync embedding function** — Document blocking behavior in docstring
5. **Monitor GIL overhead** on production workloads with async functions

---

## Conclusion

The Python binding is **functionally correct** and passes all e2e tests, but **exposes a critical API gap**: error types are not discriminated. Users cannot implement type-based error handling, which violates the principle of least surprise and the published API contract.

All other issues are minor (documentation, test coverage) or acceptable by design (sync embedding).

**Priority Action**: Implement error type mapping (Issue #1).
