```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  languageDetection: {
    enabled: true,
    minConfidence: 0.8,
    detectMultiple: false,
  },
};

const result = await extract({ kind: "uri", uri: "document.pdf" }, config);
if (result.detectedLanguages) {
  console.log(`Detected languages: ${result.detectedLanguages.join(", ")}`);
}
```
