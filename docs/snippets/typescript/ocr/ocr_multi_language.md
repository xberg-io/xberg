```typescript title="TypeScript"
import { extractSync } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "tesseract",
    language: "eng+deu+fra",
  },
};

const result = extractSync("multilingual.pdf", null, config);
console.log(result.content);
```
