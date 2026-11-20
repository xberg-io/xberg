---
language: typescript
target: wasm
---

```typescript title="WASM"
import init, { WasmExtractInput, extract } from "@xberg-io/xberg-wasm";

await init();

const fileInput = document.getElementById("file") as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const result = await extract(WasmExtractInput.fromBytes(bytes, file.type || "application/pdf"));

  result.results[0].tables?.forEach((table) => {
    console.log(`Table with ${table.cells?.length ?? 0} rows`);
    if (table.markdown) {
      console.log(table.markdown);
    }
    table.cells?.forEach((row: string[]) => console.log(row.join(" | ")));
  });
}
```
