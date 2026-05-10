```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const response = await fetch("scanned.pdf");
const data = new Uint8Array(await response.arrayBuffer());

const config = {
  force_ocr: true,
  ocr: {
    backend: "tesseract",
    language: "eng"
  }
};

const result = await extractBytes(data, "application/pdf", config);
console.log(result.content);
console.log(`Detected languages: ${result.detected_languages?.join(", ") ?? "unknown"}`);
```
