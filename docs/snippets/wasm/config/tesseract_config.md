```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const data = new Uint8Array(await fetch("scanned.pdf").then(r => r.arrayBuffer()));

const config = {
  ocr: {
    backend: "tesseract",
    language: "eng+deu",
    tesseract_config: {
      psm: 6,
      oem: 3,
    },
  },
};

const result = await extractBytes(data, "application/pdf", config);
console.log(`OCR text: ${result.content}`);
```
