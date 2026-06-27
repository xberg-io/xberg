# Custom Plugin Usage Pattern

Demonstrate the pattern for using registered plugins during document extraction.

```typescript title="WASM"
import init, { extract, registerPostProcessor } from "xberg-wasm";

await init();

// Register a custom post-processor
const customProcessor = {
  processingStage: () => "post-extraction",
  process: (result) => {
    console.log("Post-processor: enriching extraction result");
    return {
      ...result,
      metadata: {
        ...result.metadata,
        enriched: true,
        processorApplied: "customProcessor",
      },
    };
  },
};

registerPostProcessor(customProcessor);

// Extract document with registered plugin
async function extractWithPlugins(fileBytes, mimeType) {
  const config = {
    ocr: null,
    chunking: null,
    enableQualityProcessing: false,
  };

  // Extraction automatically applies registered post-processors
  const result = await extract(fileBytes, mimeType, config);

  console.log("Extraction complete");
  console.log("Plugins applied:", result.metadata?.enriched);

  return result;
}

// Usage
const pdfBytes = new Uint8Array([
  /* PDF content */
]);
const result = await extractWithPlugins(pdfBytes, "application/pdf");
console.log("Final result:", result);
```

The extraction pipeline automatically applies all registered plugins in the correct order.
