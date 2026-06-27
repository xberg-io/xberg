```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const documentData = await fetch("document.pdf").then((res) => res.arrayBuffer());

const result = await extract(documentData, "application/pdf", {
  images: {
    extract_images: true,
    target_dpi: 300,
    max_image_dimension: 2000,
  },
});

console.log(result.content);
```
