```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  enableQualityProcessing: true,
};

const result = await extract("document.pdf", null, config);
console.log(result.content);
```
