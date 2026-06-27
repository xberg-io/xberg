```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  forceOcr: true,
  ocr: {
    backend: "tesseract",
    language: "eng",
  },
};

const result = extract({ kind: "uri", uri: "scanned.pdf" }, config);

console.log(result.content);
console.log(`Detected Languages: ${result.detectedLanguages?.join(", ") ?? "none"}`);
```
