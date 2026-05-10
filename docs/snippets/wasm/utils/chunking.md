```typescript title="WASM"
import init, { extractFile } from "kreuzberg-wasm";

await init();

const config = {
  chunking: {
    maxChars: 1500,
    chunkOverlap: 200,
  },
};

const result = await extractFile("document.pdf", undefined, config);
console.log(`Chunks created: ${result.chunks?.length ?? 0}`);
```
