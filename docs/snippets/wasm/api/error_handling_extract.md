```typescript title="WASM"
import init, { extract } from "xberg-wasm";

await init();

// Note: WASM has no native batch API; use Promise.all with per-item error handling
const files = document.getElementById("files") as HTMLInputElement;
const fileList = files.files || [];

// Extract multiple files concurrently (simulated batch)
const extractionPromises = Array.from(fileList).map(async (file) => {
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    const result = await extract(bytes, file.type || "application/octet-stream", undefined);
    return { file: file.name, success: true, result };
  } catch (err) {
    return {
      file: file.name,
      success: false,
      error: err instanceof Error ? err.message : String(err),
    };
  }
});

const results = await Promise.all(extractionPromises);

// Process results with per-item error handling
results.forEach((item) => {
  if (item.success) {
    console.log(`✓ ${item.file}: ${item.result.content.length} characters`);
  } else {
    console.error(`✗ ${item.file}: ${item.error}`);
  }
});

// Summary
const succeeded = results.filter((r) => r.success).length;
const failed = results.filter((r) => !r.success).length;
console.log(`Extracted ${succeeded}/${results.length} files (${failed} errors)`);
```
