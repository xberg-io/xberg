```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  ocr: {
    backend: "tesseract",
    language: "eng+deu",
  },
  chunking: {
    maxChars: 1000,
    maxOverlap: 100,
  },
  tokenReduction: {
    mode: "aggressive",
  },
  languageDetection: {
    enabled: true,
    detectMultiple: true,
  },
  useCache: true,
  enableQualityProcessing: true,
};

const result = extract({ kind: "uri", uri: "document.pdf" }, config);

if (result.chunks) {
  for (const chunk of result.chunks) {
    console.log(`Chunk: ${chunk.content.substring(0, 100)}...`);
  }
}

if (result.detectedLanguages) {
  console.log(`Languages: ${result.detectedLanguages.join(", ")}`);
}
```
