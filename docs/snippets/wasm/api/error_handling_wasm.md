```typescript title="WASM"
import init, { extractBytes } from "kreuzberg-wasm";

await init();

const response = await fetch("document.pdf");
const data = new Uint8Array(await response.arrayBuffer());

try {
  const result = await extractBytes(data, "application/pdf", undefined);
  console.log(`Success: ${result.content.length} characters`);
} catch (error) {
  if (error instanceof Error) {
    console.error("Extraction error:", error.message);
  }
}
```
