```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  enableQualityProcessing: true,
};

const result = await extract({ kind: "uri", uri: "scanned_document.pdf" }, config);

if (result.qualityScore !== null && result.qualityScore !== undefined) {
  if (result.qualityScore < 0.5) {
    console.warn(`Warning: Low quality extraction (${result.qualityScore.toFixed(2)})`);
  } else {
    console.log(`Quality score: ${result.qualityScore.toFixed(2)}`);
  }
}
```
