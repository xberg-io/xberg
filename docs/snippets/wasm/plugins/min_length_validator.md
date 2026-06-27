# Minimum Length Text Validator

Register a validator that filters out extraction results with text below a minimum length threshold.

```typescript title="WASM"
import init, { registerValidator, extract } from "xberg-wasm";

await init();

const MIN_LENGTH = 10;

// Define a minimum length validator
const minLengthValidator = {
  validate: (extractionResult) => {
    const textLength = extractionResult.text?.length || 0;

    if (textLength < MIN_LENGTH) {
      return {
        valid: false,
        error: `Text too short: ${textLength} < ${MIN_LENGTH}`,
      };
    }

    return {
      valid: true,
      error: null,
    };
  },
};

try {
  registerValidator(minLengthValidator);
  console.log(`Min length validator registered (threshold: ${MIN_LENGTH})`);
} catch (error) {
  console.error("Failed to register validator:", error);
}

// Now extract with validation enabled
const pdfBytes = new Uint8Array([
  /* PDF content */
]);
const config = {
  ocr: null,
  chunking: null,
};

const result = await extract(pdfBytes, "application/pdf", config);
console.log("Validated result:", result);
```

This validator ensures extracted text meets minimum quality standards by checking length.
