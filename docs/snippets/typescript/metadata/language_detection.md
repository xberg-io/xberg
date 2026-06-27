```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  languageDetection: {
    enabled: true,
    minConfidence: 0.9,
    detectMultiple: true,
  },
};

const result = await extract({ kind: "uri", uri: "document.pdf" }, config);
console.log(result.content);
```
