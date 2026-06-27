```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  chunking: {
    maxChars: 1500,
    chunkOverlap: 200,
  },
};

const result = await extract("document.pdf", undefined, config);
console.log(`Chunks created: ${result.chunks?.length ?? 0}`);
```
