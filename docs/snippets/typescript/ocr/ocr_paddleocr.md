```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "paddle-ocr",
    language: "en",
    // modelTier: 'server', // for max accuracy
  },
};

const result = extract({ kind: "uri", uri: "scanned.pdf" }, config);
console.log(result.content);
```
