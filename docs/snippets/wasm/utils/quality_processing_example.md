```typescript title="WASM"
import init, { extractFile } from "kreuzberg-wasm";

await init();

const config = {
  enableQualityProcessing: true,
};

const result = await extractFile("scanned_document.pdf", undefined, config);
console.log(`Content length: ${result.content.length} characters`);
if (result.qualityScore !== undefined && result.qualityScore !== null) {
  console.log(`Quality score: ${result.qualityScore.toFixed(2)}`);
}
```
