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
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    const result = await extract(WasmExtractInput.fromBytes(bytes, file.type || "application/pdf"));
    console.log(`Extracted: ${result.results[0].content.length} characters`);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error("Extraction failed:", message);
  }
}
```
