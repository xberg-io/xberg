```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "tesseract",
  },
  forceOcr: true,
};

const result = extract({ kind: "uri", uri: "document.pdf" }, config);
console.log(result.content);
```
