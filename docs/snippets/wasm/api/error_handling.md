```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

const fileInput = document.getElementById("file") as HTMLInputElement;
const file = fileInput.files?.[0];

if (file) {
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    const result = await extract(bytes, file.type || "application/pdf", undefined);
    console.log(`Extracted: ${result.content.length} characters`);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    console.error("Extraction failed:", message);
  }
}
```
