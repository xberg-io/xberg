```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  languageDetection: {
    enabled: true,
    minConfidence: 0.8,
    detectMultiple: true,
  },
};

const result = await extract("multilingual_document.pdf", null, config);
if (result.detectedLanguages) {
  console.log(`Detected languages: ${result.detectedLanguages.join(", ")}`);
}
```
