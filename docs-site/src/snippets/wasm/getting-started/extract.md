---
language: typescript
target: wasm
---

```typescript title="Wasm"
import init, { WasmExtractInput, extract } from "@xberg-io/xberg-wasm";

await init();

const buffer = await fetch("/document.pdf").then((response) => response.arrayBuffer());
const bytes = new Uint8Array(buffer);

const output = await extract(WasmExtractInput.fromBytes(bytes, "application/pdf", "document.pdf"));

console.log(output.results[0].content);
```
