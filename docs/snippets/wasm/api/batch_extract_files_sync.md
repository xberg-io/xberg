```typescript title="WASM"
// WASM has no batch helper; await extractBytes for each file (in parallel via Promise.all).
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const input = document.getElementById("files") as HTMLInputElement;
const files = Array.from(input.files ?? []);

const results = await Promise.all(
  files.map(async (file) => {
    const bytes = new Uint8Array(await file.arrayBuffer());
    return extractBytes(bytes, file.type || "application/pdf", undefined);
  }),
);

results.forEach((result, i) => {
  console.log(`File ${i + 1}: ${result.content.length} characters`);
});
```
