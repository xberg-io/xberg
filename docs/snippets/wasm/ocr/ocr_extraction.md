```typescript title="WASM (Browser)"
import { enableOcr, extract, initWasm } from "@xberg-io/xberg-wasm";

await initWasm();
await enableOcr();

const fileInput = document.getElementById("file") as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
  const result = await extract(
    { kind: "bytes", bytes: file, mimeType: file.type },
    {
      ocr: {
        backend: "xberg-tesseract",
        language: "eng",
      },
    },
  );
  console.log(result.content);
}
```

```typescript title="WASM (Node.js / Deno / Bun)"
import { enableOcr, extract, initWasm } from "@xberg-io/xberg-wasm";

await initWasm();
await enableOcr(); // Uses native xberg-tesseract backend

const result = await extract(
  { kind: "uri", uri: "./scanned_document.png" },
  {
    ocr: {
      backend: "xberg-tesseract",
      language: "eng",
    },
  },
);
console.log(result.content);
```
