```typescript title="TypeScript"
import { extractFile } from "@kreuzberg/node";

const config = {
  pdfOptions: {
    extractImages: true,
    extractMetadata: true,
    passwords: ["password1", "password2"],
    hierarchy: { enabled: true, kClusters: 6, includeBbox: true },
  },
};

const result = await extractFile("document.pdf", null, config);
console.log(result.content);
```
