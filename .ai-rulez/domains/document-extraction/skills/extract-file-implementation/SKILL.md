---
name: extract-file-implementation
---
Document extraction from file paths

1. Read file into memory (or stream for large files)
2. Detect MIME type from extension/magic bytes
3. Query extractor registry for matching extractors
4. Select highest-priority extractor
5. Execute extraction with ExtractionConfig
6. Cache result using content hash
7. Return ExtractionResult or error
