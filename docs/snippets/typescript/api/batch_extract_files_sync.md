```typescript title="TypeScript"
import { batchExtractFilesSync } from "kreuzberg";

const items = [
  { path: "doc1.pdf", config: undefined },
  { path: "doc2.docx", config: undefined },
  { path: "report.pdf", config: undefined },
];

const results = batchExtractFilesSync(items);

results.forEach((result, i) => {
  console.log(`File ${i}: ${result.content.length} chars`);
});
```
