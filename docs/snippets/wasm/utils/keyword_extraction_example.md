```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  keywords: {
    algorithm: "yake",
    maxKeywords: 10,
    minScore: 0.3,
  },
};

const result = await extract({ kind: "uri", uri: "research_paper.pdf" }, config);
console.log(`Content length: ${result.content.length}`);
console.log(`Keywords: ${JSON.stringify(result.metadata?.keywords ?? [])}`);
```
