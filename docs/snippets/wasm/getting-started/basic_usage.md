```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const response = await fetch("document.pdf");
const data = new Uint8Array(await response.arrayBuffer());

const result = await extractBytes(data, "application/pdf", undefined);
console.log(result.content);
console.log(`MIME Type: ${result.mime_type}`);
```
