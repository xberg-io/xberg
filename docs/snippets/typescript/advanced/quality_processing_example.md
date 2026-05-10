```typescript title="TypeScript"
import { extractFile } from "@kreuzberg/node";

const config = {
  enableQualityProcessing: true,
};

const result = await extractFile("scanned_document.pdf", null, config);

if (result.qualityScore !== null && result.qualityScore !== undefined) {
  if (result.qualityScore < 0.5) {
    console.warn(`Warning: Low quality extraction (${result.qualityScore.toFixed(2)})`);
  } else {
    console.log(`Quality score: ${result.qualityScore.toFixed(2)}`);
  }
}
```
