---
name: ocr-error-recovery
---
Handle OCR failures gracefully

1. Attempt OCR with selected backend
2. On error:
   a. Log error details
   b. Record backend that failed
   c. Store original image
3. Try fallback backend if available
4. Return best available result:
   - Full result if successful
   - Partial result if some data extracted
   - Error if complete failure
5. Document failure for monitoring
6. Provide recovery suggestions
