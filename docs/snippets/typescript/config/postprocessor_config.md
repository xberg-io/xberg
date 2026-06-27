```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  postprocessor: {
    enabled: true,
    enabledProcessors: ["deduplication", "whitespace_normalization"],
    disabledProcessors: ["mojibake_fix"],
  },
};

const result = await extract("document.pdf", null, config);
console.log(result.content);
```
