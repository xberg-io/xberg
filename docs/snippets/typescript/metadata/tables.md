```typescript title="TypeScript"
import { extract } from "@xberg-io/xberg";

const result = extract("document.pdf");

if (result.tables) {
  for (const table of result.tables) {
    const rowCount = table.cells?.length ?? 0;
    console.log(`Table with ${rowCount} rows`);

    if (table.markdown) {
      console.log(table.markdown);
    }

    if (table.cells) {
      for (const row of table.cells) {
        console.log(row);
      }
    }
  }
}
```
