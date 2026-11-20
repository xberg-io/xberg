---
language: typescript
target: wasm
---

```typescript title="Wasm"
import init, { WasmExtractInput, extract } from "@xberg-io/xberg-wasm";

async function extractDocuments(files: Uint8Array[], mimeTypes: string[]) {
  await init();

  const results = await Promise.all(
    files.map((bytes, index) => extract(WasmExtractInput.fromBytes(bytes, mimeTypes[index]))),
  );

  return results.map((r) => ({
    content: r.results[0].content,
    metadata: r.results[0].metadata,
  }));
}

const fileBytes = [new Uint8Array([1, 2, 3])];
const mimes = ["application/pdf"];

extractDocuments(fileBytes, mimes)
  .then((results) => console.log(results))
  .catch(console.error);
```
