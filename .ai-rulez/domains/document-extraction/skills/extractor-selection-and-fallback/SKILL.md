---
name: extractor-selection-and-fallback
---
Route documents to appropriate extractors with fallback chains

1. Query registry for MIME type
2. Get list of supporting extractors
3. Sort by priority (highest first)
4. For each extractor in order:
   a. Execute extraction
   b. On success, return result
   c. On failure, record error and continue
5. If all fail, return aggregate error
6. For batch: continue with next document
