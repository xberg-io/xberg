```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  tokenReduction: {
    mode: "moderate",
    preserveImportantWords: true,
  },
};

const result = await extract("verbose_document.pdf", null, config);
console.log(`Content length: ${result.content.length}`);
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
```
