```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const data = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

const config = {
  postprocessor: {
    enabled: true,
    enabled_processors: ["whitespace_normalizer", "unicode_normalizer"],
  },
};

const result = await extract(data, "application/pdf", config);
console.log(`Processed content: ${result.content}`);
```
