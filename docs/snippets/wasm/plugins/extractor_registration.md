# Document Extractor Registration

Register a custom document extractor plugin in WASM that implements the required interface.

```typescript title="WASM"
import init, {
  registerDocumentExtractor,
  unregisterDocumentExtractor,
  listDocumentExtractors,
  extract,
} from "xberg-wasm";

await init();

// Define a custom extractor as a plain JS object with required methods
const customExtractor = {
  // Required: extract document bytes
  // Takes: (bytes: Uint8Array, mimeType: string, config: object) -> Promise<{text: string, ...}>
  extract: async (bytes, mimeType, config) => {
    if (mimeType !== "application/x-custom") {
      throw new Error("Unsupported MIME type");
    }
    // Custom extraction logic
    const text = new TextDecoder().decode(bytes);
    return JSON.stringify({
      text: `Extracted: ${text.slice(0, 100)}`,
      page_count: 1,
      language: "en",
    });
  },

  // Required: list supported MIME types as JSON array
  supportedMimeTypes: () => {
    return JSON.stringify(["application/x-custom"]);
  },

  // Optional: plugin name (returned by Plugin trait)
  version: () => "1.0.0",
};

// Register the custom extractor
try {
  registerDocumentExtractor(customExtractor);
  console.log("Extractor registered successfully");
} catch (error) {
  console.error("Failed to register extractor:", error);
}

// List all extractors (includes your custom one)
const extractors = listDocumentExtractors();
console.log("Available extractors:", extractors);

// Use the custom extractor via normal extraction
const customBytes = new Uint8Array([0x00, 0x01, 0x02]);
const result = await extract(customBytes, "application/x-custom", {});
console.log("Extraction result:", result);

// Unregister when done
try {
  unregisterDocumentExtractor("wasm_bridge");
  console.log("Extractor unregistered");
} catch (error) {
  console.error("Failed to unregister:", error);
}
```

The extractor object must implement `extract` and `supportedMimeTypes` methods. Optional methods: `initialize()`, `shutdown()`, and `version()` for lifecycle management.
