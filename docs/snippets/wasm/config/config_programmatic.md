```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const data = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

const config = {
  use_cache: true,
  ocr: {
    backend: "tesseract",
    language: "eng+deu",
    tesseract_config: {
      psm: 6,
    },
  },
  chunking: {
    max_characters: 1000,
    overlap: 200,
  },
  enable_quality_processing: true,
};

const result = await extract(data, "application/pdf", config);
console.log(`Content length: ${result.content.length}`);
```
