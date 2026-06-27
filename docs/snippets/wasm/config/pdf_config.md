```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const data = new Uint8Array(await fetch("encrypted.pdf").then((r) => r.arrayBuffer()));

const config = {
  pdf_options: {
    extract_images: true,
    passwords: ["password123"],
    extract_metadata: true,
    hierarchy: {},
  },
};

const result = await extract(data, "application/pdf", config);
console.log(`Title: ${result.metadata?.title}`);
console.log(`Authors: ${result.metadata?.authors}`);
```
