```typescript title="TypeScript"
import { extractSync } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "paddle-ocr",
    language: "en",
  },
};

const result = extractSync("scanned.pdf", null, config);

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
