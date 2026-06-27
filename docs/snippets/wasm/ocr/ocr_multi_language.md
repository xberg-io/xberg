```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const multilingualData = await fetch("multilingual.pdf").then((res) => res.arrayBuffer());

const result = await extract(multilingualData, "application/pdf", {
  ocr: {
    backend: "tesseract",
    language: "eng+deu+fra",
  },
});

console.log(result.content);
```
