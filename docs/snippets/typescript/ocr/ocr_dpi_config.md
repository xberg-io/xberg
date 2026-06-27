```typescript title="TypeScript"
import { extractSync } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "tesseract",
  },
  pdfOptions: {
    extractImages: true,
  },
};

const result = extractSync("scanned.pdf", null, config);
console.log(result.content);
```
