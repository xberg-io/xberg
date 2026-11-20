---
language: typescript
target: wasm
---

```typescript title="WASM"
import init, { WasmExtractInput, extract } from "@xberg-io/xberg-wasm";

await init();

const response = await fetch("document.pdf");
const data = new Uint8Array(await response.arrayBuffer());

try {
  const result = await extract(WasmExtractInput.fromBytes(data, "application/pdf"));
  console.log(`Success: ${result.results[0].content.length} characters`);
} catch (error) {
  if (error instanceof Error) {
    console.error("Extraction error:", error.message);
  }
}
```
