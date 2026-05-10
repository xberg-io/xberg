```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const documentData = await fetch("document.pdf").then((res) =>
  res.arrayBuffer()
);

const result = await extractBytes(documentData, "application/pdf", {
  images: {
    extract_images: true,
    target_dpi: 300,
    max_image_dimension: 2000,
  },
});

console.log(result.content);
```
