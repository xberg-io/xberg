# Quality Score Validator

Register a validator that computes and checks a quality score for extracted text.

```typescript title="WASM"
import init, { registerValidator, extractBytes } from "kreuzberg-wasm";

await init();

// Define a quality score validator
const qualityScoreValidator = {
  validate: (extractionResult) => {
    const text = extractionResult.text || "";
    const metadata = extractionResult.metadata || {};
    
    let score = 100;
    const issues = [];
    
    // Penalize empty text
    if (text.length === 0) {
      score -= 50;
      issues.push("No text extracted");
    }
    
    // Penalize if mostly whitespace
    const nonWhitespace = text.replace(/\s/g, "").length;
    const whitespaceRatio = 1 - (nonWhitespace / text.length);
    if (whitespaceRatio > 0.5) {
      score -= 20;
      issues.push("High whitespace ratio");
    }
    
    // Penalize unusual character distributions
    const unicodeRatio = (text.match(/[^\x00-\x7F]/g) || []).length / text.length;
    if (unicodeRatio > 0.3) {
      score -= 10;
      issues.push("High Unicode character ratio");
    }
    
    // Check confidence if available
    if (metadata.confidence && metadata.confidence < 0.5) {
      score -= 15;
      issues.push("Low confidence score");
    }
    
    const QUALITY_THRESHOLD = 60;
    const isValid = score >= QUALITY_THRESHOLD;
    
    return {
      valid: isValid,
      error: isValid ? null : `Quality score ${score} < ${QUALITY_THRESHOLD}: ${issues.join(", ")}`,
      metadata: {
        qualityScore: score,
        issues: issues
      }
    };
  }
};

try {
  registerValidator(qualityScoreValidator);
  console.log("Quality score validator registered");
} catch (error) {
  console.error("Failed to register validator:", error);
}

// Extract with quality assessment
const pdfBytes = new Uint8Array([/* PDF content */]);
const result = await extractBytes(pdfBytes, "application/pdf", {});
const validation = qualityScoreValidator.validate(result);
console.log("Quality assessment:", validation.metadata);
```

This validator assigns a quality score based on multiple text characteristics.
