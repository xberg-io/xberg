---
language: typescript
target: wasm
---

```typescript title="WASM (Browser)"
import init, { WasmExtractInput, WasmExtractionConfig, WasmOcrConfig, extract } from "@xberg-io/xberg-wasm";

await init();

const fileInput = document.getElementById("file") as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const ocr = WasmOcrConfig.default();
  ocr.enabled = true;
  ocr.backend = "tesseract";
  ocr.language = ["eng"];

  const config = WasmExtractionConfig.default();
  config.ocr = ocr;

  const result = await extract(WasmExtractInput.fromBytes(bytes, file.type), config);
  console.log(result.results[0].content);
}
```

```typescript title="WASM (Node.js / Deno / Bun)"
import init, { WasmExtractInput, WasmExtractionConfig, WasmOcrConfig, extract } from "@xberg-io/xberg-wasm";

// Outside the browser the default `fetch`-based init cannot read a `file://`
// URL: pass the `xberg_wasm_bg.wasm` bytes yourself, either as
// `init({ module_or_path: bytes })` or via the synchronous `initSync({ module: bytes })`.
await init();

const ocr = WasmOcrConfig.default();
ocr.enabled = true;
ocr.backend = "tesseract";
ocr.language = ["eng"];

const config = WasmExtractionConfig.default();
config.ocr = ocr;

const result = await extract(WasmExtractInput.fromUri("./scanned_document.png"), config);
console.log(result.results[0].content);
```
