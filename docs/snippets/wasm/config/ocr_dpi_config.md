```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const data = new Uint8Array(await fetch("document.pdf").then(r => r.arrayBuffer()));

const config = {
  images: {
    extract_images: true,
    target_dpi: 300,
    max_image_dimension: 4096,
    auto_adjust_dpi: true,
    min_dpi: 150,
    max_dpi: 600,
  },
};

const result = await extractBytes(data, "application/pdf", config);
console.log(`Extracted images: ${result.images?.length || 0}`);
```
