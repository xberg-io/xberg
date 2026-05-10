```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const imageData = await fetch("document.pdf").then((res) =>
  res.arrayBuffer()
);

const result = await extractBytes(imageData, "application/pdf", {
  images: {
    extract_images: true,
  },
});

console.log(result.images);
```
