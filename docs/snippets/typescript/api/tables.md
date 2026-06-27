```typescript title="TypeScript"
import { ExtractInputKind, extract } from "@xberg-io/xberg";

const output = await extract({
  kind: "uri",
  uri: "document.pdf",
});

output.results[0].tables?.forEach((table) => {
  console.log(`Table with ${table.cells?.length ?? 0} rows`);
  console.log(table.markdown);
  table.cells?.forEach((row) => console.log(row.join(" | ")));
});
```
