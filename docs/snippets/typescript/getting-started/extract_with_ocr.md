```typescript title="TypeScript"
import { extractSync } from "@xberg-io/xberg";

const config = {
  forceOcr: true,
  ocr: {
    backend: "tesseract",
    language: "eng",
  },
};

const result = extractSync("scanned.pdf", null, config);

console.log(result.content);
console.log(`Detected Languages: ${result.detectedLanguages?.join(", ") ?? "none"}`);
```
