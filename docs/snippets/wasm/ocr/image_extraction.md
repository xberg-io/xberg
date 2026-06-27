```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const imageData = await fetch("document.pdf").then((res) => res.arrayBuffer());

const result = await extract(imageData, "application/pdf", {
  images: {
    extract_images: true,
  },
});

console.log(result.images);
```
