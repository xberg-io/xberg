```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const data = new Uint8Array(await fetch("document.pdf").then((r) => r.arrayBuffer()));

const config = {
  output_format: "html",
  html_output: {
    theme: "github",
  },
};

const result = await extract({ kind: "bytes", bytes: data, mimeType: "application/pdf" }, config);
console.log(result.content); // HTML with kb-* classes
```
