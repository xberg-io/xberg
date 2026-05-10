# Register Custom Validator Plugin

Register a custom validator that checks extraction results for quality or correctness.

```typescript title="WASM"
import init, { registerValidator, extractBytes } from "kreuzberg-wasm";

await init();

// Define a custom validator
const customValidator = {
  validate: (extractionResult) => {
    const text = extractionResult.text || "";
    
    // Check for minimum content
    if (text.length === 0) {
      return {
        valid: false,
        error: "No text extracted from document"
      };
    }
    
    // Check for suspicious patterns
    const hasRepeatingChars = /(.)\1{5,}/.test(text);
    if (hasRepeatingChars) {
      return {
        valid: false,
        error: "Text contains excessive repeating characters (possible OCR error)"
      };
    }
    
    // Check if text is mostly whitespace
    if (text.trim().length < text.length * 0.5) {
      return {
        valid: false,
        error: "Text is mostly whitespace"
      };
    }
    
    return {
      valid: true,
      error: null
    };
  }
};

try {
  registerValidator(customValidator);
  console.log("Custom validator registered");
} catch (error) {
  console.error("Failed to register validator:", error);
}

// Extract and validate
async function extractAndValidate(fileBytes, mimeType) {
  const result = await extractBytes(fileBytes, mimeType, {});
  
  const validation = customValidator.validate(result);
  if (!validation.valid) {
    console.warn("Validation failed:", validation.error);
  } else {
    console.log("✓ Extraction passed validation");
  }
  
  return result;
}
```

Validators run after extraction to ensure results meet quality standards.
