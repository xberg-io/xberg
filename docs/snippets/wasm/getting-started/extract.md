```typescript title="Wasm"
import { ExtractInputKind, extract, initWasm } from "@xberg-io/xberg-wasm";

await initWasm();

const bytes = new Uint8Array(
  await fetch("/document.pdf").then((response) => response.arrayBuffer()),
);

const output = await extract({
  kind: ExtractInputKind.Bytes,
  bytes,
  mimeType: "application/pdf",
  filename: "document.pdf",
});

console.log(output.results[0].content);
```
