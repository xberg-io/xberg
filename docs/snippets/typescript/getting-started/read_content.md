```typescript title="TypeScript"
import { extractSync } from "@xberg-io/xberg";

const result = extractSync("document.pdf");

console.log(`Content: ${result.content}`);
console.log(`Success: ${result.success}`);
console.log(`Content Length: ${result.content.length}`);

if (result.metadata.page_count) {
  console.log(`Pages: ${result.metadata.page_count}`);
}
```
