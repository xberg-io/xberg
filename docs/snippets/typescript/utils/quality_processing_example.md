```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  enableQualityProcessing: true,
};

const result = await extract({ kind: "uri", uri: "scanned_document.pdf" }, config);
console.log(`Content length: ${result.content.length} characters`);
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
```
