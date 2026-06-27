```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const documentData = await fetch("scanned.pdf").then((res) => res.arrayBuffer());

const result = await extract(documentData, "application/pdf", {
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
