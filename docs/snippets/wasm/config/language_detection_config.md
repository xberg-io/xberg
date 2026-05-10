```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const data = new Uint8Array(await fetch("document.pdf").then(r => r.arrayBuffer()));

const config = {
  language_detection: {
    enabled: true,
    min_confidence: 0.8,
    detect_multiple: true,
  },
};

const result = await extractBytes(data, "application/pdf", config);
console.log(`Detected language: ${result.language}`);
console.log(`Confidence: ${result.language_confidence}`);
```
