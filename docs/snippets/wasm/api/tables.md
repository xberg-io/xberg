```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const fileInput = document.getElementById("file") as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const result = await extractBytes(bytes, file.type || "application/pdf", undefined);

  result.tables?.forEach((table) => {
    console.log(`Table with ${table.cells?.length ?? 0} rows`);
    if (table.markdown) {
      console.log(table.markdown);
    }
    table.cells?.forEach((row) => console.log(row.join(" | ")));
  });
}
```
