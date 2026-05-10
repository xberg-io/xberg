```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const multilingualData = await fetch("multilingual.pdf").then((res) =>
  res.arrayBuffer()
);

const result = await extractBytes(multilingualData, "application/pdf", {
  ocr: {
    backend: "tesseract",
    language: "eng+deu+fra",
  },
});

console.log(result.content);
```
