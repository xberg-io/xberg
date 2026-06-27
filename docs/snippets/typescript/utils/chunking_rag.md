```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  chunking: {
    maxChars: 500,
    maxOverlap: 50,
    embedding: {
      preset: "balanced",
    },
  },
};

const result = await extract({ kind: "uri", uri: "research_paper.pdf" }, config);

if (result.chunks) {
  for (const chunk of result.chunks) {
    console.log(`Chunk ${chunk.metadata.chunkIndex + 1}/${chunk.metadata.totalChunks}`);
    console.log(`Position: ${chunk.metadata.charStart}-${chunk.metadata.charEnd}`);
    console.log(`Content: ${chunk.content.slice(0, 100)}...`);
    if (chunk.embedding) {
      console.log(`Embedding: ${chunk.embedding.length} dimensions`);
    }
  }
}
```
