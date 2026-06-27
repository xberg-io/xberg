```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  chunking: {
    maxChars: 1000,
    embedding: {
      preset: "quality",
    },
  },
};

const result = await extract({ kind: "uri", uri: "document.pdf" }, config);
if (result.chunks && result.chunks.length > 0) {
  console.log(`Chunk embeddings: ${result.chunks[0].embedding?.length ?? 0} dimensions`);
}
```
