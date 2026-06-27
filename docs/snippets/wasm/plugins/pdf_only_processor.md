# PDF-Only Post-Processor

Register a post-processor that only processes PDF documents and filters others.

```typescript title="WASM"
import init, { registerPostProcessor, extract } from "xberg-wasm";

await init();

// Define a PDF-only post-processor
const pdfOnlyProcessor = {
  processingStage: () => "post-extraction",
  process: (extractionResult) => {
    // Check if this is a PDF extraction
    const isPdf =
      extractionResult.metadata?.mimeType === "application/pdf" ||
      extractionResult.metadata?.source?.endsWith(".pdf");

    if (!isPdf) {
      // Skip processing for non-PDF documents
      return extractionResult;
    }

    // Apply PDF-specific processing
    const processed = {
      ...extractionResult,
      metadata: {
        ...extractionResult.metadata,
        pdfProcessed: true,
        pageCount: extractionResult.metadata?.pageCount || 1,
      },
      // Normalize text for PDFs
      text: (extractionResult.text || "")
        .replace(/\n{3,}/g, "\n\n") // Remove excessive line breaks
        .trim(),
    };

    return processed;
  },
};

try {
  registerPostProcessor(pdfOnlyProcessor);
  console.log("PDF-only post-processor registered");
} catch (error) {
  console.error("Failed to register post-processor:", error);
}

// Test with various documents
const testDocs = [
  {
    bytes: new Uint8Array([
      /* PDF */
    ]),
    type: "application/pdf",
  },
  {
    bytes: new Uint8Array([
      /* HTML */
    ]),
    type: "text/html",
  },
];

for (const doc of testDocs) {
  const result = await extract(doc.bytes, doc.type, {});
  console.log(`${doc.type}: PDF-specific processing applied:`, result.metadata?.pdfProcessed);
}
```

This processor applies PDF-specific transformations only to PDF documents.
