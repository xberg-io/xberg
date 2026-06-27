```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const data = new Uint8Array(await fetch("scanned.pdf").then((r) => r.arrayBuffer()));

const config = {
  ocr: {
    backend: "tesseract",
    language: "eng",
  },
};

const result = await extract(data, "application/pdf", config);
console.log(`Content length: ${result.content.length}`);
console.log(`Tables detected: ${result.tables?.length || 0}`);
```
