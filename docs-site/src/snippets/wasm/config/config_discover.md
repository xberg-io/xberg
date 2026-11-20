---
language: typescript
target: wasm
---

```typescript title="WASM"
import init, { WasmExtractInput, WasmExtractionConfig, WasmOcrConfig, extract } from "@xberg-io/xberg-wasm";

await init();

const ocr = WasmOcrConfig.default();
ocr.backend = "tesseract";
ocr.language = ["eng"];

const config = WasmExtractionConfig.default();
config.useCache = true;
config.enableQualityProcessing = true;
config.ocr = ocr;

const buffer = await fetch("document.pdf").then((response) => response.arrayBuffer());
const bytes = new Uint8Array(buffer);
const result = await extract(WasmExtractInput.fromBytes(bytes, "application/pdf"), config);
console.log(result.results[0].content);
```
