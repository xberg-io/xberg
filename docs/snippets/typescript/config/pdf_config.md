```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  pdfOptions: {
    extractImages: true,
    extractMetadata: true,
    passwords: ["password1", "password2"],
    hierarchy: { enabled: true, kClusters: 6, includeBbox: true },
  },
};

const result = await extract("document.pdf", null, config);
console.log(result.content);
```
