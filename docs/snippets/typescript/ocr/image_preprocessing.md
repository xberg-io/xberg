```typescript title="TypeScript"
import { extractSync } from "@xberg-io/xberg";

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

const result = extractSync("document.pdf", config);
console.log(`content length: ${result.content.length}`);
```
