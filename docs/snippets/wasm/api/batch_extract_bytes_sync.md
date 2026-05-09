```typescript title="WASM"
// WASM has no batch helper; await extractBytes for each input (in parallel via Promise.all).
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const urls = ["document1.pdf", "document2.pdf"];

const results = await Promise.all(
  urls.map(async (url) => {
    const resp = await fetch(url);
    const bytes = new Uint8Array(await resp.arrayBuffer());
    return extractBytes(bytes, "application/pdf", undefined);
  }),
);

results.forEach((result, i) => {
  console.log(`Document ${i + 1}: ${result.content.length} characters`);
});
```
