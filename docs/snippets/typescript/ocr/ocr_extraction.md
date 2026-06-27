```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "tesseract",
    language: "eng",
  },
};

const result = extract({ kind: "uri", uri: "scanned.pdf" }, config);
console.log(result.content);
```
