```typescript title="TypeScript"
import { getVersion, extractSync } from "@xberg-io/xberg";

const version = getVersion();
console.log(`Xberg version: ${version}`);

const result = extractSync("document.pdf");
console.log(`Extraction successful: ${result.success}`);
```
