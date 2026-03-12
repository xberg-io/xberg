---
name: image-ocr-processing
---
Process image documents with OCR

1. Create OcrProcessor with cache config
2. Check cache for image hash
3. If cache hit, return cached result
4. Select optimal OCR backend
5. Apply preprocessing if configured
6. Execute OCR backend
7. Parse results (hOCR, confidence, tables)
8. Store in cache
9. Return structured result
