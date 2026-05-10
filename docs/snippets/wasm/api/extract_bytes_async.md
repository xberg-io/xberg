```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const response = await fetch("document.pdf");
const data = new Uint8Array(await response.arrayBuffer());

const result = await extractBytes(data, "application/pdf", undefined);
console.log(`Extracted: ${result.content.length} characters`);
console.log(`Metadata: ${JSON.stringify(result.metadata)}`);
```
