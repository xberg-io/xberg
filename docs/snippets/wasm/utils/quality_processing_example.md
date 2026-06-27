```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const config = {
  enableQualityProcessing: true,
};

const result = await extract({ kind: "uri", uri: "scanned_document.pdf" }, config);
console.log(`Content length: ${result.content.length} characters`);
if (result.qualityScore !== undefined && result.qualityScore !== null) {
  console.log(`Quality score: ${result.qualityScore.toFixed(2)}`);
}
```
