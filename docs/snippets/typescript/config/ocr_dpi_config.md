```typescript title="TypeScript"
import { extractFileSync } from "@kreuzberg/node";

const config = {
  ocr: {
    backend: "tesseract",
    tesseractConfig: {
      preprocessing: {
        targetDpi: 300,
      },
    },
  },
};

const result = extractFileSync("scanned.pdf", null, config);
console.log(`content length: ${result.content.length}`);
```
