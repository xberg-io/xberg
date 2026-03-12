---
name: batch-extract-files
---
Concurrent extraction of multiple documents

1. Estimate optimal worker pool size
2. Create async task queue for documents
3. Spawn worker tasks up to pool size
4. Process documents concurrently
5. Track errors per document
6. Preserve input document ordering
7. Return batch results
