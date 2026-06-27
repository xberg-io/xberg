# PDF Metadata Post-Processor

Register a post-processor that extracts and enriches extraction results with PDF metadata.

```typescript title="WASM"
import init, { registerPostProcessor, extract } from "xberg-wasm";

await init();

// Define a PDF metadata extractor post-processor
const pdfMetadataProcessor = {
  processingStage: () => "post-extraction",
  process: (extractionResult) => {
    // Enrich extraction with metadata
    const enriched = {
      ...extractionResult,
      metadata: {
        ...extractionResult.metadata,
        processorName: "pdf-metadata",
        processedAt: new Date().toISOString(),
        wordCount: (extractionResult.text || "").split(/\s+/).length,
      },
    };

    return enriched;
  },
};

try {
  registerPostProcessor(pdfMetadataProcessor);
  console.log("PDF metadata post-processor registered");
} catch (error) {
  console.error("Failed to register post-processor:", error);
}

// Extract with post-processing
const pdfBytes = new Uint8Array([
  /* PDF content */
]);
const config = {
  ocr: null,
  chunking: null,
};

const result = await extract(pdfBytes, "application/pdf", config);
console.log("Enriched metadata:", result.metadata);
```

The post-processor runs after extraction to enrich or transform the results.
