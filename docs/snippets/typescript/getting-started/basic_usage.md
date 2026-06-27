```typescript title="TypeScript"
import { extractSync } from "@xberg-io/xberg";

const config = {
  useCache: true,
  enableQualityProcessing: true,
};

const result = extractSync("document.pdf", null, config);

console.log(result.content);
console.log(`MIME Type: ${result.mimeType}`);
```
