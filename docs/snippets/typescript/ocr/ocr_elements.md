```typescript title="TypeScript"
import { extractFileSync } from "@kreuzberg/node";

const config = {
  ocr: {
    backend: "paddle-ocr",
    language: "en",
  },
};

const result = extractFileSync("scanned.pdf", null, config);

if (result.ocrElements) {
  for (const element of result.ocrElements) {
    console.log(`Text: ${element.text}`);
    console.log(`Confidence: ${element.confidence.recognition.toFixed(2)}`);
    console.log(`Geometry:`, element.geometry);
    if (element.rotation) {
      console.log(`Rotation: ${element.rotation.angle}°`);
    }
    console.log();
  }
}
```
