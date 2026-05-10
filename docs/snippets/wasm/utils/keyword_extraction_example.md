```typescript title="WASM"
import init, { extractFile } from "kreuzberg-wasm";

await init();

const config = {
  keywords: {
    algorithm: "yake",
    maxKeywords: 10,
    minScore: 0.3,
  },
};

const result = await extractFile("research_paper.pdf", undefined, config);
console.log(`Content length: ${result.content.length}`);
console.log(`Keywords: ${JSON.stringify(result.metadata?.keywords ?? [])}`);
```
