```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "tesseract",
    language: "eng+fra",
    tesseractConfig: {
      psm: 3,
    },
  },
};

const result = await extract("document.pdf", null, config);
console.log(result.content);
```
