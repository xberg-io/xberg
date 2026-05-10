```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const response = await fetch("document.pdf");
const data = new Uint8Array(await response.arrayBuffer());

const result = await extractBytes(data, "application/pdf", undefined);

console.log(`Content: ${result.content}`);
console.log(`Success: true`);
console.log(`Content length: ${result.content.length} characters`);

if (result.tables && result.tables.length > 0) {
  result.tables.forEach((table, i) => {
    console.log(`Table ${i}: ${table.rows?.length ?? 0} rows`);
  });
}

if (result.chunks && result.chunks.length > 0) {
  result.chunks.forEach((chunk, i) => {
    console.log(`Chunk ${i}: ${chunk.text?.length ?? 0} characters`);
  });
}
```
