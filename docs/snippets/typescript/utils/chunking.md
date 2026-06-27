```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  chunking: {
    maxChars: 1500,
    maxOverlap: 200,
    embedding: {
      preset: "quality",
    },
  },
};

const result = await extract("document.pdf", null, config);
console.log(`Chunks created: ${result.chunks?.length ?? 0}`);
```
