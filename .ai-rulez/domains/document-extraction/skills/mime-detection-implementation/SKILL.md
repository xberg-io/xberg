---
name: mime-detection-implementation
---
Detect document formats reliably

1. Check file extension
2. Read magic bytes
3. Match against known signatures
4. Fall back to content analysis if ambiguous
5. Detect legacy Office formats
6. Coordinate format conversion if needed
7. Return definitive MIME type
