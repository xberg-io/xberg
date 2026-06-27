<!-- snippet:syntax-only -->

```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

// Cloud OCR backends are not supported in WASM directly.
// WASM runs in a sandboxed environment without direct network access.
// To use cloud OCR services, implement a wrapper on your server
// or use a cloud platform with built-in OCR integration.

const cloudOcrConfig = {
  ocr: {
    backend: "custom", // Custom backends must be registered via native runtime
    language: "eng",
  },
};

// This example shows the configuration structure.
// In production, route cloud OCR requests through your backend service.
```
