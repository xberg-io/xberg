```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

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

const result = extract({ kind: "uri", uri: "scanned.pdf" }, config);
console.log(`content length: ${result.content.length}`);
```
