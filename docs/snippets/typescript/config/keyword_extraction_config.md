```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  keywords: {
    algorithm: "yake",
    maxKeywords: 10,
    minScore: 0.3,
    ngramRange: [1, 3],
    language: "en",
  },
};

const result = await extract("document.pdf", null, config);
console.log(`Content: ${result.content}`);
```
