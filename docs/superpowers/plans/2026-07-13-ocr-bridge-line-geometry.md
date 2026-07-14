# OCR Bridge: Real Per-Line Geometry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop discarding real per-line OCR confidence/bbox at the Rust↔JS bridge so the web-ui OCR layout viewer shows actual geometry instead of a newline-split flat string.

**Architecture:** The injected JS OCR backend (`packages/xberg-wasm-runtime`) already computes per-line `{text, confidence, bbox}`. Change the Rust WASM bridge (`crates/xberg-wasm`) to deserialize and return that structured result instead of extracting only `.text`, then update `engine.worker.ts`'s `handleOcr` and the `OcrLine`/`ocr-to-layout.ts` data model in `xberg-web-ui` to consume it. The web-ui OCR-layout code (currently only on `lot3/web-ui-advanced-viz`) is merged in first.

**Tech Stack:** Rust (`wasm-bindgen`, `serde`, `serde-wasm-bindgen`), TypeScript (Vitest, jsdom).

## Global Constraints

- Bbox field names are `x, y, w, h` — must match `packages/xberg-wasm-runtime/src/types.ts`'s existing `OcrResult` type exactly (no translation layer).
- No `crates/xberg-wasm/Cargo.toml` edits — it is alef-generated, and `serde`/`serde_json`/`serde-wasm-bindgen`/`js-sys` are already dependencies.
- No multi-page PDF rasterization work — out of scope per the approved spec ([2026-07-13-ocr-bridge-line-geometry-design.md](../specs/2026-07-13-ocr-bridge-line-geometry-design.md)). `OcrLine.page` is optional, caller-supplied, and stays unpopulated until that separate project exists.
- A missing/malformed `lines` array from the injected JS backend degrades to `lines: []`, not an error — `text` alone is still useful.

---

### Task 1: Merge `lot3/web-ui-advanced-viz` into this branch

**Files:** none hand-edited — this is a git merge. `crates/xberg-wasm/src/bridge/ocr.rs`, `src/engine.rs`, and `tests/hybrid_dispatch.rs` are byte-identical between the two branches (verified via `git diff`), so no conflicts are expected there. The merge base is this branch's pre-existing tip (`8f790c8bcc`), and this branch's only extra commit is a docs-only spec file that `lot3` never touches, so the merge should be clean end to end.

**Interfaces:**
- Produces: `packages/xberg-web-ui/src/lib/types.ts`'s `OcrLine`, `src/lib/ocr-to-layout.ts`'s `toParsedOcrOutput`, `src/engine/engine.worker.ts`'s `handleOcr`/`OcrMessage`, `src/components/LayoutBlocks.tsx`, and the vendored `src/components/ui/layout-blocks.tsx` (`ParsedOcrOutput`, `getOcrBlocks`, `OcrBlocksPanel`) — all consumed by later tasks.

- [ ] **Step 1: Confirm working tree is clean**

Run: `git status`
Expected: `nothing to commit, working tree clean` (the design spec commit from the prior session should already be committed).

- [ ] **Step 2: Merge**

Run:
```bash
git merge lot3/web-ui-advanced-viz -m "merge: integrate Lot 3 web-ui OCR layout viewer"
```
Expected: merge completes with no conflicts (`Automatic merge went well` or fast-forward).

- [ ] **Step 3: Install dependencies pulled in by the merge**

Run: `pnpm install`
Expected: completes without error (the merge adds `@embedpdf/*`, `@hugeicons/*`, `@tanstack/react-virtual`, `react-markdown` and related packages to `packages/xberg-web-ui/package.json`).

- [ ] **Step 4: Verify the merge landed cleanly**

Run: `pnpm --filter xberg-web-ui test:run`
Expected: all existing tests pass, including `tests/lib/ocr-to-layout.test.ts` (the one pre-existing OCR test on the merged-in code).

- [ ] **Step 5: Confirm the merge commit exists**

Run: `git log --oneline -3`
Expected: top commit is the merge commit from Step 2.

---

### Task 2: Rust OCR bridge returns structured per-line data

**Files:**
- Modify: `crates/xberg-wasm/src/bridge/ocr.rs` (full rewrite, ~110 lines)
- Modify: `crates/xberg-wasm/src/engine.rs:67-75` (doc comment), `:233-250` (`ocr()` method)
- Test: `crates/xberg-wasm/tests/hybrid_dispatch.rs` (OCR tests only — NER tests untouched)

**Interfaces:**
- Consumes: nothing new — `js_sys`, `wasm_bindgen`, `serde`, `serde_wasm_bindgen` are already dependencies of `crates/xberg-wasm`.
- Produces: `crate::bridge::ocr::{OcrResult, OcrLineResult, OcrBbox}` (Rust structs), and a new JS-visible return shape for `XbergEngine.ocr(bytes, opts)`: `Promise<{ text: string, lines: Array<{ text: string, confidence: number, bbox?: { x: number, y: number, w: number, h: number } }> }>` — consumed by Task 4's `engine.worker.ts` changes.

- [ ] **Step 1: Write the failing test — update `hybrid_dispatch.rs`'s OCR tests to the new contract**

Replace the two OCR-related `#[wasm_bindgen_test]` functions in `crates/xberg-wasm/tests/hybrid_dispatch.rs` (leave the two NER tests above them untouched) with:

```rust
#[wasm_bindgen_test]
async fn resolve_ocr_with_injected_stub() {
    let stub = js_sys::eval(
        "({
            ocr: async (bytes, opts) => ({
                text: 'hello from ocr',
                lines: [
                    { text: 'hello from ocr', confidence: 0.98, bbox: { x: 1, y: 2, w: 3, h: 4 } }
                ]
            })
        })",
    )
    .unwrap()
    .dyn_into::<js_sys::Object>()
    .unwrap();

    let result = xberg_wasm::bridge::ocr::resolve_ocr(Some(stub), &[0xFF, 0xD8, 0xFF, 0xE0], "eng")
        .await
        .unwrap();

    assert_eq!(result.text, "hello from ocr");
    assert_eq!(result.lines.len(), 1);
    assert_eq!(result.lines[0].text, "hello from ocr");
    assert!((result.lines[0].confidence - 0.98).abs() < f64::EPSILON);
    let bbox = result.lines[0].bbox.as_ref().expect("bbox should be present");
    assert_eq!(bbox.x, 1.0);
    assert_eq!(bbox.y, 2.0);
    assert_eq!(bbox.w, 3.0);
    assert_eq!(bbox.h, 4.0);
}

#[wasm_bindgen_test]
async fn resolve_ocr_with_injected_stub_missing_lines_defaults_to_empty() {
    let stub = js_sys::eval(
        "({
            ocr: async (bytes, opts) => ({ text: 'no geometry available' })
        })",
    )
    .unwrap()
    .dyn_into::<js_sys::Object>()
    .unwrap();

    let result = xberg_wasm::bridge::ocr::resolve_ocr(Some(stub), &[0xFF, 0xD8, 0xFF, 0xE0], "eng")
        .await
        .unwrap();

    assert_eq!(result.text, "no geometry available");
    assert!(result.lines.is_empty());
}

#[wasm_bindgen_test]
async fn resolve_ocr_without_injected_returns_error() {
    let result = xberg_wasm::bridge::ocr::resolve_ocr(None, &[0xFF, 0xD8, 0xFF, 0xE0], "eng").await;
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("unavailable"), "expected 'unavailable' in error: {msg}");
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `wasm-pack test --headless --chrome crates/xberg-wasm -- --test hybrid_dispatch`
Expected: FAIL to compile — `resolve_ocr` still returns `Result<String, JsValue>`, so `.text`/`.lines` field access on a `String` is a compile error.

- [ ] **Step 3: Implement `bridge/ocr.rs`**

Replace the full contents of `crates/xberg-wasm/src/bridge/ocr.rs` with:

```rust
//! OCR (Optical Character Recognition) bridge with injected-first dispatch.
//!
//! Similar to the NER bridge, the WASM engine prefers an externally-
//! injected JavaScript object that implements an
//! `ocr(imageBytes, options)` async method. The host returns a promise
//! resolving to `{ text: string, lines: Array<{ text: string, confidence:
//! number, bbox?: { x, y, w, h } }> }` — real per-line geometry, not just
//! a flat string. `lines` is optional on the wire; a missing/malformed
//! `lines` array degrades to an empty vec rather than an error, since
//! `text` alone is still useful.
//!
//! When no injection is present it attempts an in-binary Tesseract
//! fallback under `#[cfg(feature = "ocr-wasm")]`.

#[cfg(target_arch = "wasm32")]
use js_sys::{Function, Object, Promise, Reflect};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// A single OCR-detected line's bounding box, in source-image pixel space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrBbox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// A single OCR-detected line of text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrLineResult {
    pub text: String,
    #[serde(default)]
    pub confidence: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<OcrBbox>,
}

/// Full OCR result for one image: the concatenated text plus per-line
/// geometry, when the backend provides it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrResult {
    pub text: String,
    #[serde(default)] // missing `lines` degrades to empty vec, not an error
    pub lines: Vec<OcrLineResult>,
}

/// Resolve the best available OCR backend and return extracted text with
/// per-line geometry.
///
/// 1. If `injected` is `Some(obj)`, call
///    `obj.ocr(imageBytes, { language })` — the host returns a promise
///    resolving to `{ text, lines }` (see module docs).
/// 2. If `injected` is `None` and `ocr-wasm` is enabled, attempt
///    the in-binary Tesseract backend.
/// 3. Otherwise return an error explaining that OCR is unavailable.
pub async fn resolve_ocr(
    injected: Option<js_sys::Object>,
    image_bytes: &[u8],
    language: &str,
) -> Result<OcrResult, JsValue> {
    resolve_ocr_with_timeout(injected, image_bytes, language, crate::bridge::BRIDGE_TIMEOUT_MS).await
}

/// Like [`resolve_ocr`] but with a configurable bridge timeout.
pub async fn resolve_ocr_with_timeout(
    injected: Option<js_sys::Object>,
    image_bytes: &[u8],
    language: &str,
    timeout_ms: u32,
) -> Result<OcrResult, JsValue> {
    match injected {
        Some(obj) => call_injected_ocr(obj, image_bytes, language, timeout_ms).await,
        None => fallback_ocr(image_bytes, language).await,
    }
}

/// Call the injected JS `ocr(imageBytes, { language })` method.
async fn call_injected_ocr(
    obj: Object,
    image_bytes: &[u8],
    language: &str,
    timeout_ms: u32,
) -> Result<OcrResult, JsValue> {
    let fn_val = Reflect::get(&obj, &JsValue::from_str("ocr"))
        .map_err(|e| js_from_any(format!("failed to read 'ocr' property: {e:?}")))?;
    let func: Function = fn_val
        .dyn_into()
        .map_err(|_| js_from_any("injected OCR object has no 'ocr' function"))?;

    let js_bytes = js_sys::Uint8Array::from(image_bytes);
    let opts = js_sys::Object::new();
    Reflect::set(&opts, &JsValue::from_str("language"), &JsValue::from_str(language))?;

    let args = js_sys::Array::of2(&js_bytes, &opts);
    let result = func.apply(&obj, &args)?;
    let promise = Promise::from(result);
    let js_val = crate::bridge::timed_js_future_with_timeout(promise, timeout_ms).await?;

    serde_wasm_bindgen::from_value(js_val)
        .map_err(|e| js_from_any(format!("failed to deserialize ocr result: {e}")))
}

/// In-binary OCR fallback via Tesseract WASM backend.
async fn fallback_ocr(image_bytes: &[u8], language: &str) -> Result<OcrResult, JsValue> {
    if image_bytes.is_empty() {
        return Err(js_from_any("OCR input image is empty"));
    }

    #[cfg(all(feature = "ocr-wasm", not(feature = "ocr")))]
    {
        // TesseractWasmBackend::new() is pub(crate) in xberg, so we cannot
        // construct it from xberg-wasm.  Return a diagnostic error.
        let _ = language;
        Err(js_from_any(
            "OCR unavailable: no injected backend and ocr-wasm backend constructor is not public; \
             provide an injected JS backend or use the full xberg API directly",
        ))
    }

    #[cfg(not(all(feature = "ocr-wasm", not(feature = "ocr"))))]
    {
        Err(js_from_any(
            "OCR unavailable: no injected backend and ocr-wasm disabled",
        ))
    }
}

/// Convert a Display error into a JsValue suitable for propagation.
fn js_from_any(v: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&v.to_string())
}
```

- [ ] **Step 4: Implement `engine.rs`**

In `crates/xberg-wasm/src/engine.rs`, replace this doc comment line inside `XbergEngine::new`'s doc block (around line 70):

```rust
    /// - `ocr`      — object with `ocr(imageBytes, opts): Promise<string>`
```
with:
```rust
    /// - `ocr`      — object with `ocr(imageBytes, opts): Promise<{ text: string, lines?: Array<{ text: string, confidence: number, bbox?: { x: number, y: number, w: number, h: number } }> }>`
```

Then replace the `ocr()` method (around lines 233-250):

```rust
    /// Perform OCR on image bytes, returning extracted text.
    #[allow(clippy::missing_errors_doc)]
    pub async fn ocr(&self, bytes: Vec<u8>, opts: JsValue) -> Result<JsValue, JsValue> {
        let language = if opts.is_undefined() || opts.is_null() {
            "eng".to_string()
        } else {
            js_sys::Reflect::get(&opts, &JsValue::from_str("language"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| "eng".to_string())
        };

        let text = resolve_ocr_with_timeout(self.ocr.clone(), &bytes, &language, self.bridge_timeout_ms)
            .await
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

        Ok(JsValue::from_str(&text))
    }
```
with:
```rust
    /// Perform OCR on image bytes, returning extracted text with per-line
    /// confidence and bounding-box geometry (when the backend provides it).
    #[allow(clippy::missing_errors_doc)]
    pub async fn ocr(&self, bytes: Vec<u8>, opts: JsValue) -> Result<JsValue, JsValue> {
        let language = if opts.is_undefined() || opts.is_null() {
            "eng".to_string()
        } else {
            js_sys::Reflect::get(&opts, &JsValue::from_str("language"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_else(|| "eng".to_string())
        };

        let result = resolve_ocr_with_timeout(self.ocr.clone(), &bytes, &language, self.bridge_timeout_ms)
            .await
            .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;

        serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }
```

- [ ] **Step 5: Run to verify it passes**

Run: `wasm-pack test --headless --chrome crates/xberg-wasm -- --test hybrid_dispatch`
Expected: PASS — all 5 tests (2 NER + 3 OCR).

- [ ] **Step 6: Commit**

```bash
git add crates/xberg-wasm/src/bridge/ocr.rs crates/xberg-wasm/src/engine.rs crates/xberg-wasm/tests/hybrid_dispatch.rs
git commit -m "fix(xberg-wasm): return real per-line OCR confidence/bbox instead of discarding them"
```

---

### Task 3: Rebuild the WASM package bindings

**Files:**
- Regenerate (build output, not hand-edited): `crates/xberg-wasm/pkg/nodejs/*`

**Interfaces:**
- Consumes: Task 2's Rust changes.
- Produces: an up-to-date `pkg/nodejs/xberg_wasm.js`/`.wasm`/`xberg_wasm.d.ts` for `packages/xberg-wasm-runtime` and `packages/xberg-web-ui` (both depend on `@xberg-io/xberg-wasm` via `file:` link) to consume in Task 4.

- [ ] **Step 1: Build**

Run: `pnpm --filter @xberg-io/xberg-wasm build`
(equivalent to `wasm-pack build --target nodejs --out-dir pkg/nodejs` run from `crates/xberg-wasm`)
Expected: completes without error, updates `crates/xberg-wasm/pkg/nodejs/xberg_wasm.js`, `.wasm`, and `.d.ts`.

- [ ] **Step 2: Verify the generated type signature**

Run: `grep -n "ocr(bytes" crates/xberg-wasm/pkg/nodejs/xberg_wasm.d.ts`
Expected: `ocr(bytes: Uint8Array, opts: any): Promise<any>;` — unchanged (a `JsValue`-returning method without a dedicated `#[wasm_bindgen]` struct still generates `Promise<any>`, same as `ner()`). This confirms the build picked up the Task 2 changes without introducing a type mismatch.

- [ ] **Step 3: Commit the regenerated bindings**

```bash
git add crates/xberg-wasm/pkg/nodejs
git commit -m "chore(xberg-wasm): rebuild pkg/nodejs bindings for the OCR bridge change"
```

---

### Task 4: `engine.worker.ts` uses real per-line OCR data

**Files:**
- Modify: `packages/xberg-web-ui/src/engine/engine.worker.ts` (`handleOcr` function)
- Test: Create `packages/xberg-web-ui/tests/engine/engine.worker.test.ts`

**Interfaces:**
- Consumes: `XbergEngine.ocr(bytes, opts): Promise<{text: string, lines: Array<{text: string, confidence: number, bbox?: {x,y,w,h}}>}>` from Tasks 2–3.
- Produces: the worker posts `{ type: "ocrResult", requestId: string, lines: Array<{text, confidence, bbox?}> }` — consumed by `WorkerClient.ocrLayout` (unchanged) and, downstream, `OcrLine[]` in Task 5.

- [ ] **Step 1: Write the failing test**

Create `packages/xberg-web-ui/tests/engine/engine.worker.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";

const ocrMock = vi.fn();

vi.mock("@xberg-io/xberg-wasm", () => ({
  XbergEngine: vi.fn().mockImplementation(() => ({
    ocr: ocrMock,
  })),
}));

vi.mock("xberg-wasm-runtime", () => ({
  createXbergRuntimeFactory: vi.fn().mockResolvedValue({}),
}));

describe("engine.worker handleOcr", () => {
  beforeEach(() => {
    vi.resetModules();
    ocrMock.mockReset();
  });

  it("posts real per-line text/confidence/bbox instead of a newline-split flat string", async () => {
    ocrMock.mockResolvedValue({
      text: "Hello\nWorld",
      lines: [
        { text: "Hello", confidence: 0.97, bbox: { x: 1, y: 2, w: 3, h: 4 } },
        { text: "World", confidence: 0.88, bbox: { x: 1, y: 10, w: 3, h: 4 } },
      ],
    });
    const postMessageSpy = vi.spyOn(self, "postMessage").mockImplementation(() => undefined);

    await import("../../src/engine/engine.worker.js");
    self.dispatchEvent(
      new MessageEvent("message", {
        data: { type: "ocr", requestId: "r1", bytes: new Uint8Array([1, 2, 3]) },
      })
    );

    await vi.waitFor(() => {
      expect(postMessageSpy).toHaveBeenCalled();
    });

    expect(postMessageSpy).toHaveBeenCalledWith(
      {
        type: "ocrResult",
        requestId: "r1",
        lines: [
          { text: "Hello", confidence: 0.97, bbox: { x: 1, y: 2, w: 3, h: 4 } },
          { text: "World", confidence: 0.88, bbox: { x: 1, y: 10, w: 3, h: 4 } },
        ],
      },
      []
    );
  });
});
```

- [ ] **Step 2: Run to verify it fails**

Run: `pnpm --filter xberg-web-ui test:run -- tests/engine/engine.worker.test.ts`
Expected: FAIL — current `handleOcr` calls `text.split(/\r?\n/)` on the mocked object (cast `as string`), which throws inside the `try` block, so the worker posts `{type: "error", ...}` instead of the expected `{type: "ocrResult", ...}` call.

- [ ] **Step 3: Implement**

In `packages/xberg-web-ui/src/engine/engine.worker.ts`, replace the `handleOcr` function:

```ts
async function handleOcr(msg: OcrMessage): Promise<void> {
  try {
    const xEngine = await getEngine();
    // `engine.ocr` returns the recognized text as a single string (no
    // per-line geometry from the WASM OCR bridge), so split on newlines
    // to recover lines; confidence is unavailable and defaults to 1.
    const text = (await xEngine.ocr(msg.bytes, undefined)) as string;
    const lines = text.split(/\r?\n/).map((t) => ({ text: t, confidence: 1 }));
    post({ type: "ocrResult", requestId: msg.requestId, lines });
  } catch (err) {
    post({ type: "error", requestId: msg.requestId, message: err instanceof Error ? err.message : String(err) });
  }
}
```
with:
```ts
async function handleOcr(msg: OcrMessage): Promise<void> {
  try {
    const xEngine = await getEngine();
    const result = (await xEngine.ocr(msg.bytes, undefined)) as {
      text: string;
      lines: Array<{
        text: string;
        confidence: number;
        bbox?: { x: number; y: number; w: number; h: number };
      }>;
    };
    post({ type: "ocrResult", requestId: msg.requestId, lines: result.lines });
  } catch (err) {
    post({ type: "error", requestId: msg.requestId, message: err instanceof Error ? err.message : String(err) });
  }
}
```

- [ ] **Step 4: Run to verify it passes**

Run: `pnpm --filter xberg-web-ui test:run -- tests/engine/engine.worker.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add packages/xberg-web-ui/src/engine/engine.worker.ts packages/xberg-web-ui/tests/engine/engine.worker.test.ts
git commit -m "fix(web-ui): handleOcr uses real per-line OCR geometry instead of splitting a flat string"
```

---

### Task 5: `OcrLine` gains an optional `page`; `ocr-to-layout.ts` threads it through

**Files:**
- Modify: `packages/xberg-web-ui/src/lib/types.ts` (`OcrLine`)
- Modify: `packages/xberg-web-ui/src/lib/ocr-to-layout.ts`
- Test: Extend `packages/xberg-web-ui/tests/lib/ocr-to-layout.test.ts`

**Interfaces:**
- Consumes: `OcrLine[]` from Task 4's `handleOcr` output (via `WorkerClient.ocrLayout`).
- Produces: `toParsedOcrOutput(lines: OcrLine[], width?: number, height?: number): ParsedOcrOutput` — same signature as before, now honoring `line.page` per block. Consumed by `LayoutBlocks.tsx` (unchanged) and `getOcrBlocks` in the vendored `components/ui/layout-blocks.tsx`, which flattens `chunks` and reads page identity from each block's own `metadata.page` — confirmed by reading that file directly, which is why this task keeps the single-chunk structure rather than grouping into multiple chunks.

- [ ] **Step 1: Write the failing test**

In `packages/xberg-web-ui/tests/lib/ocr-to-layout.test.ts`, add a second test alongside the existing one:

```ts
  it("uses each line's own page identity when present, defaulting to page 1 otherwise", () => {
    const lines: OcrLine[] = [
      { text: "Page one line", confidence: 0.9, page: { number: 1, width: 800, height: 1000 } },
      { text: "Page two line", confidence: 0.8, page: { number: 2, width: 800, height: 1000 } },
      { text: "No page info", confidence: 0.7 },
    ];
    const out = toParsedOcrOutput(lines);
    expect(out.chunks).toHaveLength(1);
    const blocks = out.chunks[0]!.blocks;
    expect(blocks[0]!.metadata.page).toEqual({ number: 1, width: 800, height: 1000 });
    expect(blocks[1]!.metadata.page).toEqual({ number: 2, width: 800, height: 1000 });
    expect(blocks[2]!.metadata.page).toEqual({ number: 1, width: 1000, height: 1400 });
  });
```

(This is added as a second `it(...)` inside the existing `describe("lib/ocr-to-layout", ...)` block, after the pre-existing "maps a single OCR line" test.)

- [ ] **Step 2: Run to verify it fails**

Run: `pnpm --filter xberg-web-ui test:run -- tests/lib/ocr-to-layout.test.ts`
Expected: FAIL — `OcrLine` has no `page` field yet (TS compile error), and `toParsedOcrOutput` hardcodes `page: { number: 1, width, height }` for every block regardless of input.

- [ ] **Step 3: Implement — `types.ts`**

In `packages/xberg-web-ui/src/lib/types.ts`, replace the `OcrLine` interface and its doc comment:

```ts
/**
 * Intentionally has no page identity or page dimensions: `engine.worker.ts`'s
 * `handleOcr` gets a flat string back from `XbergEngine.ocr` (the `@xberg-io/xberg-wasm`
 * binding doesn't expose per-line geometry or multi-page structure yet) and
 * splits it into "lines" on newlines, so every result is already scoped to
 * a single page with no real bounding boxes. Modeling `page`/dimensions here
 * would just be a field nothing populates. Add it once the WASM OCR bridge
 * returns real per-page, per-line geometry (`toParsedOcrOutput` would then
 * need to group blocks by that page identity instead of hardcoding page 1).
 */
export interface OcrLine {
  text: string;
  confidence: number;
  bbox?: { x: number; y: number; w: number; h: number };
}
```
with:
```ts
/**
 * `bbox`/`confidence` are real per-line OCR geometry (see the WASM OCR
 * bridge in `crates/xberg-wasm/src/bridge/ocr.rs`), not derived from a
 * flat-string split. `page` is optional and caller-supplied: nothing in
 * this codebase currently splits a multi-page document into per-page
 * images before calling OCR (`handleOcr` still OCRs one whole file's
 * bytes per call), so `page` stays undefined until that rasterization
 * step exists. `toParsedOcrOutput` uses `page.number`/`width`/`height`
 * per block when present and defaults to page 1 otherwise.
 */
export interface OcrLine {
  text: string;
  confidence: number;
  bbox?: { x: number; y: number; w: number; h: number };
  page?: { number: number; width: number; height: number };
}
```

- [ ] **Step 4: Implement — `ocr-to-layout.ts`**

Replace the full contents of `packages/xberg-web-ui/src/lib/ocr-to-layout.ts`:

```ts
import type { OcrLine } from "./types.js";
import type { ParsedOcrOutput } from "@/components/ui/layout-blocks";

// Every block is assigned to page 1 with the caller-supplied `width`/`height`
// because `OcrLine` carries no real page identity or per-page dimensions
// (see the doc comment on `OcrLine` in lib/types.ts) — there is exactly one
// page's worth of data to place today.
export function toParsedOcrOutput(
  lines: OcrLine[],
  width = 1000,
  height = 1400
): ParsedOcrOutput {
  return {
    chunks: [
      {
        blocks: lines.map((l, i) => ({
          id: `block-${i}`,
          type: "text",
          content: l.text,
          metadata: {
            page: { number: 1, width, height },
            avgOcrConfidence: l.confidence,
          },
          boundingBox: l.bbox
            ? {
                left: l.bbox.x,
                top: l.bbox.y,
                right: l.bbox.x + l.bbox.w,
                bottom: l.bbox.y + l.bbox.h,
              }
            : { left: 0, top: 0, right: width, bottom: height },
        })),
      },
    ],
  };
}
```
with:
```ts
import type { OcrLine } from "./types.js";
import type { ParsedOcrOutput } from "@/components/ui/layout-blocks";

// `getOcrBlocks` (components/ui/layout-blocks.tsx) flattens every chunk's
// blocks into one array and reads page identity purely from each block's
// own `metadata.page` — so a single chunk is sufficient here; what matters
// is that each block carries the right page number/dimensions. Each line's
// own `page` is used when present (real multi-page geometry), falling back
// to page 1 with the caller-supplied `width`/`height` when absent — which
// is every line today, since nothing yet splits a document into per-page
// images before OCR (see the doc comment on `OcrLine.page` in lib/types.ts).
export function toParsedOcrOutput(
  lines: OcrLine[],
  width = 1000,
  height = 1400
): ParsedOcrOutput {
  return {
    chunks: [
      {
        blocks: lines.map((l, i) => {
          const pageNumber = l.page?.number ?? 1;
          const pageWidth = l.page?.width ?? width;
          const pageHeight = l.page?.height ?? height;
          return {
            id: `block-${i}`,
            type: "text",
            content: l.text,
            metadata: {
              page: { number: pageNumber, width: pageWidth, height: pageHeight },
              avgOcrConfidence: l.confidence,
            },
            boundingBox: l.bbox
              ? {
                  left: l.bbox.x,
                  top: l.bbox.y,
                  right: l.bbox.x + l.bbox.w,
                  bottom: l.bbox.y + l.bbox.h,
                }
              : { left: 0, top: 0, right: pageWidth, bottom: pageHeight },
          };
        }),
      },
    ],
  };
}
```

- [ ] **Step 5: Run to verify it passes**

Run: `pnpm --filter xberg-web-ui test:run -- tests/lib/ocr-to-layout.test.ts`
Expected: PASS — both the pre-existing single-line test and the new page-identity test.

- [ ] **Step 6: Typecheck the whole package**

Run: `pnpm --filter xberg-web-ui typecheck`
Expected: PASS — confirms `OcrLine.page` and its consumers are internally consistent.

- [ ] **Step 7: Commit**

```bash
git add packages/xberg-web-ui/src/lib/types.ts packages/xberg-web-ui/src/lib/ocr-to-layout.ts packages/xberg-web-ui/tests/lib/ocr-to-layout.test.ts
git commit -m "feat(web-ui): OcrLine carries real geometry, ocr-to-layout.ts threads page identity"
```

---

### Task 6: Manual end-to-end verification in the browser

**Files:** none — verification only, per this project's convention of testing UI changes live rather than relying on type-checks/unit tests alone.

**Interfaces:**
- Consumes: the full chain from Tasks 2–5.

- [ ] **Step 1: Start the web-ui dev server**

Run: `pnpm --filter xberg-web-ui dev`
Expected: dev server starts (Next.js), reachable in the browser.

- [ ] **Step 2: Exercise the OCR path on a real image**

In the browser: upload or open a document that triggers `WorkerClient.ocrLayout` (the `DocumentPageClient.tsx` flow calling into `engine.worker.ts`'s `handleOcr`), and open the `LayoutBlocks` view for it.

- [ ] **Step 3: Confirm real geometry renders**

Expected: bounding boxes are drawn at real per-line positions (not full-canvas fallback boxes), and displayed confidence values vary per line (not uniformly 100%, which was the old hardcoded `confidence: 1` artifact).

- [ ] **Step 4: Check the browser console for errors**

Expected: no errors from `engine.worker.ts` or `LayoutBlocks.tsx` during the OCR request.

- [ ] **Step 5: Stop the dev server**

Stop the process started in Step 1.
