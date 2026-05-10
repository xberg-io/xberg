# Document Extractor Registration

Document extractors are built-in and cannot be registered dynamically in WASM at runtime. However, you can list and use the available extractors.

<!-- snippet:skip -->

WASM binding does not expose custom document extractor registration; extractors are compiled into the binary at build time. Only OCR backends, post-processors, and validators can be dynamically registered.

```typescript title="WASM"
import init, { listDocumentExtractors } from "kreuzberg-wasm";

await init();

// List available extractors
const extractors = listDocumentExtractors();
console.log("Available extractors:", extractors);

// Use extraction with the built-in extractors
import { extractBytes } from "kreuzberg-wasm";

const pdfBytes = new Uint8Array([/* PDF content */]);
const config = {
  ocr: null,
  chunking: null
};

const result = await extractBytes(pdfBytes, "application/pdf", config);
console.log("Extraction result:", result);
```

To extend extraction capabilities, create a custom post-processor instead (see `word_count_processor.md`).
