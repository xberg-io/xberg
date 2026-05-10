```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const data = new Uint8Array(await fetch("document.pdf").then(r => r.arrayBuffer()));

const config = {
  enable_quality_processing: true,
  use_cache: true,
};

const result = await extractBytes(data, "application/pdf", config);
console.log(`Quality score: ${result.quality_score}`);
console.log(`Processing time: ${result.processing_time}`);
```
