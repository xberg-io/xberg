```typescript title="TypeScript"
import { extractSync } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "tesseract",
  },
  forceOcr: true,
};

const result = extractSync("document.pdf", null, config);
console.log(result.content);
```
