```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "paddle-ocr",
    language: "en",
  },
};

const result = extract({ kind: "uri", uri: "scanned.pdf" }, config);

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
