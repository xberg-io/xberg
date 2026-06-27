```typescript title="TypeScript"
import { extract, type ExtractionConfig } from "@xberg-io/xberg";

const config: ExtractionConfig = {
  useCache: true,
  ocr: {
    backend: "tesseract",
    language: "eng+deu",
    tesseractConfig: {
      psm: 6,
    },
  },
  chunking: {
    maxChars: 1000,
    maxOverlap: 200,
  },
  enableQualityProcessing: true,
};

const result = extract({ kind: "uri", uri: "document.pdf" }, config);
console.log(`Content length: ${result.content.length}`);
```
