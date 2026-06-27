```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  tokenReduction: {
    level: "Moderate",
    preserveMarkdown: true,
  },
};

const result = await extract({ kind: "uri", uri: "verbose_document.pdf" }, config);

console.log(`Reduced content length: ${result.content?.length ?? 0} chars`);
```
