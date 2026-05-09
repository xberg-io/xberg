```typescript title="TypeScript"
import { extractFileSync } from "kreuzberg";

const result = extractFileSync("document.pdf");

result.tables?.forEach((table) => {
  console.log(`Table with ${table.cells?.length ?? 0} rows`);
  console.log(table.markdown);
  table.cells?.forEach((row) => console.log(row.join(" | ")));
});
```
