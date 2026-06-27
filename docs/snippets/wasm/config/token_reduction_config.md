```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const data = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

const config = {
  token_reduction: {
    mode: "moderate",
    preserve_important_words: true,
  },
};

const result = await extract(data, "application/pdf", config);
console.log(`Original tokens: ${result.token_count}`);
console.log(`Reduced content: ${result.content}`);
```
