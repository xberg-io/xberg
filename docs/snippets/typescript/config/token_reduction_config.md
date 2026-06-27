```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  tokenReduction: {
    mode: "moderate",
    preserveImportantWords: true,
  },
};

const result = await extract({ kind: "uri", uri: "document.pdf" }, config);
console.log(result.content);
```
