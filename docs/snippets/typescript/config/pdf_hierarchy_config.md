```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const config = {
  pdfOptions: {
    extractMetadata: true,
    hierarchy: {
      enabled: true,
      kClusters: 6,
      includeBbox: true,
      ocrCoverageThreshold: 0.8,
    },
  },
};

const result = await extract("document.pdf", null, config);
if (result.pages) {
  result.pages.forEach((page) => {
    console.log(`Page ${page.pageNumber}:`);
    console.log(`  Content: ${page.content.substring(0, 100)}...`);
  });
}
```
