```typescript title="TypeScript"
import { getVersion, extract } from "@xberg-io/xberg";

const version = getVersion();
console.log(`Xberg version: ${version}`);

const result = extract("document.pdf");
console.log(`Extraction successful: ${result.success}`);
```
