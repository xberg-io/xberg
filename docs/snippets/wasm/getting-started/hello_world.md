```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const data = new Uint8Array([0x25, 0x50, 0x44, 0x46]); // PDF magic bytes
const result = await extract(data, "application/pdf", undefined);
console.log(result.content);
```
