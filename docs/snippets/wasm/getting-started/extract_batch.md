```typescript title="Wasm"
import { extractBatch, initWasm } from "@xberg-io/xberg-wasm";

await initWasm();

const pdfBytes = new Uint8Array(
  await fetch("/document.pdf").then((response) => response.arrayBuffer()),
);

const output = await extractBatch([
  {
    kind: "bytes",
    bytes: pdfBytes,
    mimeType: "application/pdf",
    filename: "document.pdf",
  },
  {
    kind: "bytes",
    bytes: new TextEncoder().encode("Hello from memory"),
    mimeType: "text/plain",
    filename: "note.txt",
  },
]);

for (const result of output.results) {
  console.log(result.content);
}
```
