# Plugin Testing Pattern

Test custom plugins to verify they implement required interfaces correctly.

```typescript title="WASM"
import init, { registerValidator, registerPostProcessor } from "kreuzberg-wasm";

await init();

// Test fixture: sample extraction result
const sampleResult = {
  text: "Sample extracted text from document",
  metadata: {
    mimeType: "application/pdf",
    source: "test.pdf",
    pageCount: 1
  }
};

// Test post-processor registration
function testPostProcessorRegistration() {
  const processor = {
    processingStage: () => "post-extraction",
    process: (result) => result
  };
  
  try {
    registerPostProcessor(processor);
    console.log("✓ Post-processor registered successfully");
  } catch (error) {
    console.error("✗ Post-processor registration failed:", error);
  }
}

// Test validator registration
function testValidatorRegistration() {
  const validator = {
    validate: (result) => ({
      valid: !!result.text,
      error: result.text ? null : "No text extracted"
    })
  };
  
  try {
    registerValidator(validator);
    console.log("✓ Validator registered successfully");
  } catch (error) {
    console.error("✗ Validator registration failed:", error);
  }
}

// Test required methods validation
function testInterfaceValidation() {
  // Missing required method should fail
  const invalidProcessor = {
    // Missing processingStage() method
    process: (result) => result
  };
  
  try {
    registerPostProcessor(invalidProcessor);
    console.error("✗ Should have rejected processor with missing methods");
  } catch (error) {
    console.log("✓ Correctly rejected invalid processor:", error);
  }
}

// Run tests
testPostProcessorRegistration();
testValidatorRegistration();
testInterfaceValidation();
```

Validate plugin implementations before deploying to production.
