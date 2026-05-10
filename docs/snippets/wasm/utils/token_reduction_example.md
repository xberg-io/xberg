```typescript title="WASM"
import init, { extractFile } from "kreuzberg-wasm";

await init();

const config = {
  tokenReduction: {
    mode: "moderate",
    preserveImportantWords: true,
  },
};

const result = await extractFile("verbose_document.pdf", undefined, config);
console.log(`Content length: ${result.content.length}`);
console.log(`MIME type: ${result.mimeType}`);
```
