```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  languageDetection: {
    enabled: true,
    minConfidence: 0.8,
    detectMultiple: false,
  },
};

const result = await extract("document.pdf", null, config);
if (result.detectedLanguages) {
  console.log(`Detected languages: ${result.detectedLanguages.join(", ")}`);
}
```
