---
name: tesseract-backend-usage
---
Optimize Tesseract OCR configuration

1. Validate language packs installed
2. Select appropriate PSM mode:
   - 0: OSD only
   - 3: Assume single column (default)
   - 6: Assume single uniform block
   - 11: Sparse text
   - 13: Raw line detection
3. Configure OCR engine mode (OEM)
4. Run Tesseract with config
5. Collect hOCR output
6. Parse confidence scores
7. Extract and reconstruct tables
