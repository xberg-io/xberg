```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const data = new Uint8Array(await fetch("document.pdf").then(r => r.arrayBuffer()));

const config = {
  pdf_options: {
    hierarchy: {
      enabled: true,
      detection_threshold: 0.75,
      ocr_coverage_threshold: 0.8,
      min_level: 1,
      max_level: 5,
    },
  },
};

const result = await extractBytes(data, "application/pdf", config);
console.log(`Hierarchy levels: ${result.hierarchy?.length || 0}`);
```
