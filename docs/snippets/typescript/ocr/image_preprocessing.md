```typescript title="TypeScript"
import { extractFileSync } from "@kreuzberg/node";

const config = {
  ocr: {
    backend: "tesseract",
    language: "eng",
    tesseractConfig: {
      preprocessing: {
        targetDpi: 300,
        denoise: true,
        deskew: true,
        contrastEnhance: true,
        binarizationMethod: "otsu",
      },
    },
  },
};

const result = extractFileSync("document.pdf", config);
console.log(`content length: ${result.content.length}`);
```
