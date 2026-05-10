```typescript title="TypeScript"
import { extractFileSync } from "kreuzberg";

const result = extractFileSync("document.pdf");

console.log(result.content);
console.log(`MIME type: ${result.mime_type}`);
console.log(`Tables: ${result.tables?.length ?? 0}`);
```
