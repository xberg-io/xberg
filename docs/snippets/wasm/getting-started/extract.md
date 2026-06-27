```typescript title="Wasm"
import { ExtractInputKind, extract, initWasm } from "@xberg-io/xberg-wasm";

await initWasm();

const bytes = new Uint8Array(
  await fetch("/document.pdf").then((response) => response.arrayBuffer()),
);

const output = await extract({
  kind: "bytes",
  bytes,
  mimeType: "application/pdf",
  filename: "document.pdf",
});

console.log(output.results[0].content);
```
