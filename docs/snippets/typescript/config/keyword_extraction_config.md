```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  keywords: {
    algorithm: "yake",
    maxKeywords: 10,
    minScore: 0.3,
    language: "en",
  },
};

const output = await extract({ kind: "uri", uri: "document.pdf" }, config);
const result = output.results![0];
console.log(`Content: ${result.content}`);
```
