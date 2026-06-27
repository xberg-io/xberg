```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  chunking: {
    maxChars: 500,
    maxOverlap: 50,
    embedding: {
      model: { type: "preset", name: "balanced" },
      normalize: true,
    },
  },
};

const result = await extract("research_paper.pdf", null, config);

for (const chunk of result.chunks ?? []) {
  console.log(`Chunk ${chunk.metadata.chunkIndex + 1}/${chunk.metadata.totalChunks}`);
  console.log(`Position: ${chunk.metadata.byteStart}-${chunk.metadata.byteEnd}`);
  console.log(`Content: ${chunk.content.slice(0, 100)}...`);
  if (chunk.embedding) {
    console.log(`Embedding: ${chunk.embedding.length} dimensions`);
  }
}
```
