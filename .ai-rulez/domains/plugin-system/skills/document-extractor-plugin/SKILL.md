---
name: document-extractor-plugin
---
Implement custom document extractors

1. Create struct implementing DocumentExtractor
2. Declare supported MIME types
3. Set priority level (0-255)
4. Implement extract_bytes():
   a. Accept bytes, MIME type, config
   b. Validate input
   c. Apply config options
   d. Perform extraction
   e. Return ExtractionResult
5. Implement extract_file():
   a. Read file to bytes
   b. Call extract_bytes()
6. Handle all error cases
7. Validate result structure
8. Test extraction quality
