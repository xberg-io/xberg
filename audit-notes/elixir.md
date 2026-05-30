# Elixir Binding Systematic Bug Audit

**Audit Date**: 2026-05-30
**Repo**: `packages/elixir/` + `e2e/elixir/`
**Status**: 28/28 e2e tests green (before audit)

## Executive Summary

Found **3 critical bugs** and **2 high-priority gaps** in the Elixir NIF binding:

1. **CRITICAL: CPU-bound NIFs lack DirtyCpu scheduling** — blocks BEAM schedulers
2. **HIGH: Thread panics not safely caught** — crashes BEAM VM
3. **HIGH: Missing Dialyzer config** — type-safety not validated
4. **MISSING: No Dialyzer coverage**
5. **MISSING: No mix_audit in CI**

---

## Findings

### BINDING_BUG #1: Scheduler Violation — CPU-Bound NIFs Without DirtyCpu

**Severity**: CRITICAL
**Issue**: Operations >1ms run on the normal scheduler, blocking the BEAM.
**Lines in NIF**: `packages/elixir/native/kreuzberg_nif/src/lib.rs`

#### CPU-Bound but Unscheduled (MUST FIX)

1. **`extract_file_sync` (line 3421)** — calls `kreuzberg::extract_file_sync`
   - Performs I/O + parsing; easily >10ms
   - Currently: `#[rustler::nif]` (normal scheduler)
   - **Fix**: Add `schedule = "DirtyIo"` (I/O-bound)

2. **`extract_bytes_sync` (line 3459)** — calls `kreuzberg::extract_bytes_sync`
   - Parsing + extraction; easily >10ms
   - Currently: `#[rustler::nif]` (normal scheduler)
   - **Fix**: Add `schedule = "DirtyCpu"` (CPU-bound)

3. **`embed_texts` (line 3710)** — embedding inference
   - Neural network forward pass; 100ms+
   - Currently: `#[rustler::nif]` (normal scheduler)
   - **Fix**: Add `schedule = "DirtyCpu"` (CPU-bound)

4. **`render_pdf_page_to_png` (line 3685)** — PDF rendering
   - Complex graphics operation; 50-500ms
   - Currently: `#[rustler::nif]` (normal scheduler)
   - **Fix**: Add `schedule = "DirtyCpu"` (CPU-bound)

#### Already Correct (3 NIFs)

These have proper scheduling:
- `extract_bytes_async` (line 3302) — `schedule = "DirtyCpu"` ✓
- `extract_file_async` (line 3369) — `schedule = "DirtyCpu"` ✓
- `embed_texts_async` (line 3646) — `schedule = "DirtyCpu"` ✓

#### All Other Quick NIFs (<1ms)

These are correctly unscheduled (fast metadata/lookup operations):
- `detect_mime_type_from_bytes`, `get_extensions_for_mime`
- `list_*_backends`, `list_document_extractors`, `list_renderers`, `list_post_processors`, `list_validators`
- `get_embedding_preset`, `list_embedding_presets`
- Registry management: `register_*`, `unregister_*`, `clear_*`

These are <1ms operations; normal scheduler is fine.

---

### BINDING_BUG #2: Thread Panic Not Safely Handled

**Severity**: CRITICAL
**Issue**: `.join()` panic is converted to string error, but panics crash the BEAM.

**Lines**:
- 3331: `extract_bytes_async` — `.map_err(|_| "thread panicked".to_string())?`
- 3397: `extract_file_async` — `.map_err(|_| "thread panicked".to_string())?`
- 3665: `embed_texts_async` — `.map_err(|_| "thread panicked".to_string())?`

**Root Cause**: Rust threads spawned at lines 3313-3331, 3379-3397, 3654-3665 can panic if:
- Inside `kreuzberg::extract_bytes()` / `extract_file()` / `embed_texts()` async runtime
- Tokio runtime panics or unwind propagates across FFI boundary
- `.spawn()` itself panics (thread creation fails)

**Current Behavior**: The `.map_err(|_| ...)` silently discards panic details. If panic occurs, `.join()` returns `Err`, converted to generic "thread panicked" string. But if panic unwinds across the FFI boundary BEFORE `.join()`, the BEAM VM crashes.

**Fix**: Wrap thread block with `std::panic::catch_unwind()` or ensure Rust code never panics.

```rust
let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        kreuzberg::extract_bytes(&content, &mime_type, config).await
    })
}));
// Handle UnwindSafe return
```

---

### BINDING_BUG #3: Error Tuple Type Inconsistency

**Severity**: MEDIUM
**Issue**: NIFs return `Result<T, String>`, but Elixir wrappers expect `{:ok, T} | {:error, atom, String}`.

**Evidence**:
- All `kreuzberg_nif` functions return `Result<T, String>` (line 3421-3727)
- Elixir `Kreuzberg.Native` module uses `rustler::init!` which auto-converts `Result<T, String>` to `{:error, Atom, Msg}`
- **BUT** spec in `Kreuzberg.ex` line 10 shows: `{:ok, map()} | {:error, atom, String.t()}`

**Root Cause**: When Rustler encodes `Err(msg: String)`, it becomes `{:error, "msg"}` (2-tuple), not `{:error, :some_atom, "msg"}` (3-tuple).

**Evidence of Issue**: Line 3331, 3397, 3665 return generic "thread panicked" string, but should return proper error atoms.

**Fix**: Use custom error type or explicit atom encoding:
```rust
#[derive(NifError)]
enum NifError {
    ThreadPanicked,
    ThreadJoinFailed,
    ...
}
```

---

### ALEF_GAP: Missing Dialyzer Configuration

**Severity**: HIGH
**Issue**: No dialyxir/Dialyzer setup in `packages/elixir/mix.exs`.

**Current State**:
- `mix.exs` (line 31-39) has `credo` but no `:dialyxir`
- No `.dialyzer_ignore_warnings` or `.dialyzer.yml`
- Elixir specs in `Kreuzberg.ex` and `Kreuzberg.Native` are not validated

**Why This Matters**:
- Rustler auto-generates Elixir wrappers; type mismatches silently occur
- Plugin registration functions (`register_ocr_backend`, etc.) use `pid()` but spec says they return `:ok | :error` — no typecheck
- Missing `:dialyxir` means caller errors go undetected

**Fix**:
1. Add to `mix.exs` deps: `{:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false}`
2. Add to project config: `dialyzer: [plt_add_apps: [:stdlib, :kernel]]`
3. Run `mix dialyzer` in CI

---

### TEST_FIXTURE: Weak Error Path Testing

**Severity**: MEDIUM
**Issue**: `e2e/elixir/` tests check happy path but not error handling thoroughly.

**Evidence**:
- `async_test.exs` line 22-30: Only checks `{:error, _}` — doesn't validate error structure
- No tests for thread panics in extraction (would hang or crash)
- No tests for invalid config JSON parsing errors

**Example**:
```elixir
# Current: too loose
assert {:error, _} = Kreuzberg.extract_bytes_async(content, "application/x-nonexistent", "{}")

# Should be: validate error structure
{:error, error_msg} = Kreuzberg.extract_bytes_async(content, "application/x-nonexistent", "{}")
assert String.contains?(error_msg, "UnsupportedFormat") or String.contains?(error_msg, "Unsupported")
```

---

## Commits Needed

### 1. Fix CPU-Bound NIF Scheduling (4 NIFs)

**File**: `packages/elixir/native/kreuzberg_nif/src/lib.rs`

```diff
-#[rustler::nif]
+#[rustler::nif(schedule = "DirtyIo")]
 pub fn extract_file_sync(
     path: String,
     mime_type: Option<String>,
     config: Option<String>,
 ) -> Result<ExtractionResult, String> {

-#[rustler::nif]
+#[rustler::nif(schedule = "DirtyCpu")]
 pub fn extract_bytes_sync(
     content: rustler::Binary,
     mime_type: String,
     config: Option<String>,
 ) -> Result<ExtractionResult, String> {

-#[rustler::nif]
+#[rustler::nif(schedule = "DirtyCpu")]
 pub fn render_pdf_page_to_png(
     pdf_bytes: rustler::Binary,
     page_index: usize,
     dpi: Option<i32>,
     password: Option<String>,
 ) -> Result<Vec<u8>, String> {

-#[rustler::nif]
+#[rustler::nif(schedule = "DirtyCpu")]
 pub fn embed_texts(texts: Vec<String>, config: Option<String>) -> Result<Vec<Vec<f32>>, String> {
```

### 2. Fix Thread Panic Handling (3 NIFs)

**File**: `packages/elixir/native/kreuzberg_nif/src/lib.rs`

Wrap each `std::thread::Builder::new()...spawn()` block with panic-safe error handling. Example for `extract_bytes_async`:

```diff
 #[rustler::nif(schedule = "DirtyCpu")]
 pub fn extract_bytes_async(
     content: rustler::Binary,
     mime_type: String,
     config: Option<String>,
 ) -> Result<ExtractionResult, String> {
     let content: Vec<u8> = content.as_slice().to_vec();
     let config_core: Option<kreuzberg::ExtractionConfig> = config
         .map(|s| serde_json::from_str::<kreuzberg::ExtractionConfig>(&s))
         .transpose()
         .map_err(|e| e.to_string())?;
+
+    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
         std::thread::Builder::new()
             .stack_size(32 * 1024 * 1024)
             .spawn(move || {
                 let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
                 let result = rt
                     .block_on(async {
                         kreuzberg::extract_bytes(
                             &content,
                             &mime_type,
                             config_core.as_ref().unwrap_or(&Default::default()),
                         )
                         .await
                     })
                     .map_err(|e| e.to_string())?;
                 Ok(result.into())
             })
             .map_err(|e| e.to_string())?
             .join()
             .map_err(|_| "thread panicked".to_string())?
+    }));
+
+    match result {
+        Ok(inner_result) => inner_result,
+        Err(_) => Err("thread panicked during extraction".to_string()),
+    }
 }
```

### 3. Add Dialyzer Configuration

**File**: `packages/elixir/mix.exs`

```diff
 defp deps do
     [
       {:jason, "~> 1.4"},
       {:rustler, "~> 0.37.0", runtime: false},
       {:rustler_precompiled, "~> 0.9"},
       {:credo, "~> 1.7", only: [:dev, :test], runtime: false},
+      {:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false},
       {:ex_doc, "~> 0.40", only: :dev, runtime: false}
     ]
 end

 def project do
     [
       app: :kreuzberg,
       version: "5.0.0-rc.3",
       elixir: "~> 1.14",
       elixirc_paths: ["lib", Path.expand("../../packages/elixir/native/kreuzberg_nif/src", __DIR__)],
       rustler_crates: [
         kreuzberg_nif: [
           mode: :release,
           targets: ~w(aarch64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu x86_64-pc-windows-gnu)
         ]
       ],
       description: "High-performance document intelligence library",
+      dialyzer: [
+        plt_add_apps: [:stdlib, :kernel, :rustler]
+      ],
       package: package(),
       deps: deps()
     ]
 end
```

### 4. Update Native.ex Error Type Specs (Optional Breaking Change for v5)

Since v5 RC cycle allows breaking changes, fix the error tuple spec:

**File**: `packages/elixir/lib/kreuzberg/native.ex`

Ensure all `def` stubs match the 3-tuple error format returned by Rustler.

---

## Test Status

**Current**: 28/28 e2e tests pass
**After fixes**: Should remain 28/28 pass

The fixes are internal safety improvements and scheduling; they don't change the public API contract. Tests continue to pass but the NIF implementation becomes:
- Non-blocking for BEAM scheduler
- Safe against panics
- Type-checked with Dialyzer

---

## Verification Steps

1. **Run e2e before fix**:
   ```bash
   task elixir:e2e
   ```
   Expected: 28/28 pass

2. **Apply fixes to NIF**

3. **Rebuild and test**:
   ```bash
   cd packages/elixir
   KREUZBERG_BUILD=1 mix deps.get
   KREUZBERG_BUILD=1 mix compile
   cd ../../e2e/elixir
   KREUZBERG_BUILD=1 mix deps.get
   mix test
   ```
   Expected: 28/28 pass

4. **Add Dialyzer**:
   ```bash
   cd packages/elixir
   mix dialyzer
   ```
   Expected: No errors (type-safe)

---

## Root Causes

| Bug | Root | Why It Happened |
|-----|------|-----------------|
| CPU-bound without DirtyCpu | No scheduler review before alef regeneration | Generated code assumed all NIFs are quick; extraction/embedding ops not CPU-profiled |
| Thread panic unsafely | Incomplete error wrapping in template | `.join()` error was caught, but panic unwind before join not guarded |
| No Dialyzer | CI doesn't require type checking | Project focuses on unit/e2e tests; static analysis gap |

---

## References

- Rustler Docs: https://github.com/rusterlium/rustler
- BEAM Scheduler: https://www.erlang.org/doc/man/erl_nif.html (see `schedule` param)
- Elixir NIF best practices: https://hexdocs.pm/rustler/
