---
name: ocr-backend-plugin
---
Implement custom OCR backends

1. Create struct implementing OcrBackend
2. Initialize OCR model/engine
3. Implement async process_image():
   a. Accept image bytes
   b. Extract language from config
   c. Validate image format
   d. Apply preprocessing if needed
   e. Run OCR engine
   f. Parse results
   g. Return ExtractionResult
4. Implement supported_languages()
5. Declare capabilities()
6. Handle async execution
7. Test OCR accuracy
8. Benchmark performance
