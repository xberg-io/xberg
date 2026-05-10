```typescript title="TypeScript"
import { batchExtractBytesSync } from "kreuzberg";
import { readFileSync } from "fs";

const doc1 = readFileSync("doc1.pdf");
const doc2 = readFileSync("doc2.pdf");

const items = [
  { content: doc1, mimeType: "application/pdf", config: undefined },
  { content: doc2, mimeType: "application/pdf", config: undefined },
];

const results = batchExtractBytesSync(items);

results.forEach((result, i) => {
  console.log(`Document ${i}: ${result.content.length} chars`);
});
```
