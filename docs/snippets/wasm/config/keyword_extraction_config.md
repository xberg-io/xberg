```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const data = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

const config = {
  keywords: {
    algorithm: "yake",
    max_keywords: 10,
    min_score: 0.1,
    ngram_range: [1, 3],
    language: "en",
  },
};

const result = await extract(data, "application/pdf", config);
console.log(`Keywords: ${JSON.stringify(result.keywords)}`);
```
