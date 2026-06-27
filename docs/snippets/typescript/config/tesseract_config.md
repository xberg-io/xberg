```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "tesseract",
    language: "eng+fra+deu",
    tesseractConfig: {
      psm: 6,
      tesseditCharWhitelist: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?",
      enableTableDetection: true,
    },
  },
};

const result = await extract("document.pdf", null, config);
console.log(result.content);
```
