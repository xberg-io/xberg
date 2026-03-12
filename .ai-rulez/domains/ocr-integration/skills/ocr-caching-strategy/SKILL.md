---
name: ocr-caching-strategy
---
Optimize OCR performance through caching

1. Generate content hash of image
2. Combine with:
   - OCR backend identifier
   - TesseractConfig serialization
   - Language configuration
3. Use hash as cache key
4. Check cache for entry
5. Validate cache entry:
   - Matches current config
   - Integrity check
   - Age check if applicable
6. Store result with metadata
7. Track statistics:
   - Hit rate
   - Storage size
   - Access patterns
