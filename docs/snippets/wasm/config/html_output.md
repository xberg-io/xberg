```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const data = new Uint8Array(await fetch("document.pdf").then(r => r.arrayBuffer()));

const config = {
  output_format: "html",
  html_output: {
    theme: "github",
  },
};

const result = await extractBytes(data, "application/pdf", config);
console.log(result.content); // HTML with kb-* classes
```
