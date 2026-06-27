```typescript title="TypeScript"
import { extractSync } from "@xberg-io/xberg";

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

const result = extractSync("scanned.pdf", null, config);
console.log(`content length: ${result.content.length}`);
```
