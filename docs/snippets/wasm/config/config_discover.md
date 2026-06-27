```typescript title="WASM"
import { initWasm, extract } from "@xberg-io/xberg-wasm";

await initWasm();

const config = {
  use_cache: true,
  enable_quality_processing: true,
  ocr: {
    backend: "tesseract-wasm",
    language: "eng",
  },
};

const bytes = new Uint8Array(buffer);
const result = await extract({ kind: "bytes", bytes, mimeType: "application/pdf" }, config);
console.log(result.content);
```
