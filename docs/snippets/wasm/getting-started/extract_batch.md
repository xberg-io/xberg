```typescript title="Wasm"
import { ExtractInputKind, extractBatch, initWasm } from "@xberg-io/xberg-wasm";

await initWasm();

const pdfBytes = new Uint8Array(
  await fetch("/document.pdf").then((response) => response.arrayBuffer()),
);

const output = await extractBatch([
  {
    kind: ExtractInputKind.Bytes,
    bytes: pdfBytes,
    mimeType: "application/pdf",
    filename: "document.pdf",
  },
  {
    kind: ExtractInputKind.Bytes,
    bytes: new TextEncoder().encode("Hello from memory"),
    mimeType: "text/plain",
    filename: "note.txt",
  },
]);

for (const result of output.results) {
  console.log(result.content);
}
```
