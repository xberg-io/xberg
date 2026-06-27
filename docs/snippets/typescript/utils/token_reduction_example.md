```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  tokenReduction: {
    mode: "moderate",
    preserveImportantWords: true,
  },
};

const result = await extract({ kind: "uri", uri: "verbose_document.pdf" }, config);
console.log(`Content length: ${result.content.length}`);
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
```
