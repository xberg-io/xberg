```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  languageDetection: {
    enabled: true,
    minConfidence: 0.9,
    detectMultiple: true,
  },
};

const result = await extract("document.pdf", null, config);
console.log(result.content);
```
