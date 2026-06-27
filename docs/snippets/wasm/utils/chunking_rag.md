```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  chunking: {
    maxChars: 500,
    chunkOverlap: 50,
  },
};

const result = await extract("research_paper.pdf", undefined, config);

if (result.chunks) {
  for (const chunk of result.chunks) {
    const meta = chunk.metadata;
    console.log(`Chunk ${meta.chunkIndex + 1}/${meta.totalChunks}`);
    console.log(`Position: ${meta.byteStart}-${meta.byteEnd}`);
    console.log(`Content: ${chunk.content.slice(0, 100)}...`);
  }
}
```
