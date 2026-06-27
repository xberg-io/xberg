```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  tokenReduction: {
    mode: "moderate",
    preserveImportantWords: true,
  },
};

const result = await extract("document.pdf", undefined, config);
console.log(result.content);
```
