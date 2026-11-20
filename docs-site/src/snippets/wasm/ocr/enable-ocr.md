---
language: typescript
target: wasm
---

```typescript title="Wasm"
import init, { WasmExtractInput, WasmExtractionConfig, WasmOcrConfig, extract } from "@xberg-io/xberg-wasm";

async function extractWithOcr() {
  await init();

  const buffer = await fetch("scanned-page.png").then((response) => response.arrayBuffer());
  const bytes = new Uint8Array(buffer);

  // OCR is turned on per extraction through the `ocr` config block. There is no
  // separate global "enable OCR" call — the backend is selected by name here.
  const ocr = WasmOcrConfig.default();
  ocr.enabled = true;
  ocr.backend = "tesseract";
  ocr.language = ["eng"];

  const config = WasmExtractionConfig.default();
  config.ocr = ocr;

  const output = await extract(WasmExtractInput.fromBytes(bytes, "image/png", "scanned-page.png"), config);

  console.log("Extracted text:");
  console.log(output.results[0].content);
}

extractWithOcr().catch(console.error);
```
