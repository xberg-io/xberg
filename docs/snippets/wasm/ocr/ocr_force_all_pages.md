```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const documentData = await fetch("document.pdf").then((res) => res.arrayBuffer());

const result = await extract(documentData, "application/pdf", {
  force_ocr: true,
  ocr: {
    backend: "tesseract",
    language: "eng",
  },
});

console.log(result.content);
```
