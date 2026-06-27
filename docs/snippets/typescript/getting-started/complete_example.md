```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  useCache: true,
  enableQualityProcessing: true,
  forceOcr: false,
  ocr: {
    backend: "tesseract",
    language: "eng+fra",
    tesseractConfig: {
      psm: 3,
      enableTableDetection: true,
    },
  },
  pdfOptions: {
    extractImages: true,
    extractMetadata: true,
  },
  images: {
    extractImages: true,
    targetDpi: 150,
    maxImageDimension: 2048,
  },
  chunking: {
    maxChars: 1000,
    maxOverlap: 200,
    embedding: {
      preset: "balanced",
    },
  },
  tokenReduction: {
    mode: "moderate",
    preserveImportantWords: true,
  },
  languageDetection: {
    enabled: true,
    minConfidence: 0.8,
    detectMultiple: false,
  },
  postprocessor: {
    enabled: true,
  },
};

const result = await extract({ kind: "uri", uri: "document.pdf" }, config);
console.log(`Extracted content length: ${result.content.length}`);
```
