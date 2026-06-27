```typescript title="TypeScript"
import { extractSync } from "@xberg-io/xberg";

const config = {
  images: {
    extractImages: true,
    targetDpi: 200,
    maxImageDimension: 2048,
    injectPlaceholders: true, // set to false to extract images without markdown references
    autoAdjustDpi: true,
  },
};

const result = extractSync("document.pdf", config);
console.log(`content length: ${result.content.length}`);
```
