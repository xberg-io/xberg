```typescript title="TypeScript"
import { extractFile } from "@kreuzberg/node";

const config = {
  tokenReduction: {
    level: "Moderate",
    preserveMarkdown: true,
  },
};

const result = await extractFile("verbose_document.pdf", null, config);

console.log(`Reduced content length: ${result.content?.length ?? 0} chars`);
```
