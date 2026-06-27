```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  chunking: {
    maxChars: 1024,
    maxOverlap: 100,
    embedding: {
      preset: "balanced",
    },
  },
};

const result = await extract("document.pdf", null, config);
console.log(`Chunks: ${result.chunks?.length ?? 0}`);
```
