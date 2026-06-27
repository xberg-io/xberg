<!-- snippet:syntax-only -->

```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

// PaddleOCR backend is not supported in WASM.
// PaddleOCR requires ONNX Runtime and native C++ dependencies unavailable in browser/WASM.
// Use the Tesseract-WASM backend instead, or implement a backend wrapper service.

const paddleOcrConfig = {
  ocr: {
    backend: "paddleocr", // Not supported in WASM
    language: "en",
  },
};

// This example shows the configuration structure for reference only.
```
