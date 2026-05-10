```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const data = new Uint8Array(await fetch("document.pdf").then(r => r.arrayBuffer()));

const config = {
  postprocessor: {
    enabled: true,
    enabled_processors: [
      "whitespace_normalizer",
      "unicode_normalizer",
    ],
  },
};

const result = await extractBytes(data, "application/pdf", config);
console.log(`Processed content: ${result.content}`);
```
