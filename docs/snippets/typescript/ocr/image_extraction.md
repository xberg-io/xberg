```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  images: {
    extractImages: true,
    targetDpi: 200,
    maxImageDimension: 2048,
    injectPlaceholders: true, // set to false to extract images without markdown references
    autoAdjustDpi: true,
  },
};

const result = extract({ kind: "uri", uri: "document.pdf" }, config);
console.log(`content length: ${result.content.length}`);
```
