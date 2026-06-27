```typescript title="TypeScript"
import { extractSync } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "paddle-ocr",
    language: "en",
    // modelTier: 'server', // for max accuracy
  },
};

const result = extractSync("scanned.pdf", null, config);
console.log(result.content);
```
