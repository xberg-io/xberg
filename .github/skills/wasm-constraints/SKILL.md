---
name: wasm-constraints
description: "wasm constraints"
---

## priority: high

# WASM Build Constraints

## Overview

WASM target in `crates/kreuzberg-wasm/`. Uses wasm-bindgen with sync-only internal APIs.

## Feature Flags

```toml
[features]
wasm-target = ["pdf", "html", "xml", "email", "language-detection", "chunking", "quality", "office"]
wasm-threads = ["dep:wasm-bindgen-rayon"]  # Optional
```

## Critical Constraints

### 1. No Tokio Runtime

All operations must be synchronous internally. Use `#[cfg(not(feature = "tokio-runtime"))]` paths.

### 2. SyncExtractor Required

Every WASM-compatible extractor MUST implement `SyncExtractor`:

```rust
impl SyncExtractor for MyExtractor {
    fn extract_sync(&self, content: &[u8], mime_type: &str, config: &ExtractionConfig)
        -> Result<ExtractionResult> { /* sync implementation */ }
}

impl DocumentExtractor for MyExtractor {
    fn as_sync_extractor(&self) -> Option<&dyn SyncExtractor> {
        Some(self)  // MUST return Some for WASM
    }
}
```

### 3. HTML Size Limit

```rust
const MAX_HTML_SIZE: usize = 2 * 1024 * 1024;  // 2MB - stack constraint
```

### 4. PDFium Initialization (from JS)

```typescript
import init, { initialize_pdfium_render } from './kreuzberg_wasm.js';
const wasm = await init();
const pdfium = await pdfiumModule();
initialize_pdfium_render(pdfium, wasm, false);  // REQUIRED for PDF
```

## Build Config

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[profile.release.package.kreuzberg-wasm]
opt-level = "z"       # Size optimization
codegen-units = 1
```

## API Pattern

```rust
#[wasm_bindgen]
pub async fn extract_from_bytes(content: Vec<u8>, config: JsValue) -> Result<JsValue, JsValue> {
    let config: ExtractionConfig = serde_wasm_bindgen::from_value(config)?;
    let result = extract_bytes_sync(&content, mime_type, &config)?;
    Ok(serde_wasm_bindgen::to_value(&result)?)
}
```

Functions can be `async` for JS compatibility, but internal extraction is sync.

## Critical Rules

1. **No tokio** — all operations synchronous
2. **Implement SyncExtractor** for all WASM-compatible extractors
3. **HTML limited to 2MB** due to stack constraints
4. **PDFium requires** manual JS initialization
5. **Size optimization** via `opt-level = "z"`
6. **Feature gate** with `#[cfg(target_arch = "wasm32")]`
