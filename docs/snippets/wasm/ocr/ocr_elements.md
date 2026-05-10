```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const documentData = await fetch("scanned.pdf").then((res) =>
  res.arrayBuffer()
);

const result = await extractBytes(documentData, "application/pdf", {
  ocr: {
    backend: "tesseract",
    language: "eng",
    element_config: {
      include_elements: true,
    },
  },
});

if (result.ocr_elements) {
  for (const element of result.ocr_elements) {
    console.log("Text:", element.text);
    console.log("Confidence:", element.confidence);
  }
}
```
