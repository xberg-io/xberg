```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  useCache: true,
  enableQualityProcessing: true,
};

const result = await extract("document.pdf", null, config);
console.log(result.content);
```
