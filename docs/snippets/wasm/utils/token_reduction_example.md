```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  tokenReduction: {
    mode: "moderate",
    preserveImportantWords: true,
  },
};

const result = await extract("verbose_document.pdf", undefined, config);
console.log(`Content length: ${result.content.length}`);
console.log(`MIME type: ${result.mimeType}`);
```
