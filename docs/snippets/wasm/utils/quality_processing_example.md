```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  enableQualityProcessing: true,
};

const result = await extract("scanned_document.pdf", undefined, config);
console.log(`Content length: ${result.content.length} characters`);
if (result.qualityScore !== undefined && result.qualityScore !== null) {
  console.log(`Quality score: ${result.qualityScore.toFixed(2)}`);
}
```
