```typescript title="TypeScript"
import { extractFileSync } from "@kreuzberg/node";

const config = {
  images: {
    extractImages: true,
    targetDpi: 200,
    maxImageDimension: 2048,
    injectPlaceholders: true, // set to false to extract images without markdown references
    autoAdjustDpi: true,
  },
};

const result = extractFileSync("document.pdf", config);
console.log(`content length: ${result.content.length}`);
```
