<!-- snippet:syntax-only -->

```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

// EasyOCR backend is not supported in WASM.
// EasyOCR requires PyTorch and Python runtime, which are unavailable in browser/WASM.
// Use the Tesseract-WASM backend instead, or route requests through a backend service.

const easyOcrConfig = {
  ocr: {
    backend: "easyocr", // Not supported in WASM
    language: "en",
  },
};

// This example shows the configuration structure for reference only.
```
