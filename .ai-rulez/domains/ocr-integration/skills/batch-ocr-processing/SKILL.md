---
name: batch-ocr-processing
---
Process multiple images concurrently

1. Create OcrProcessor (shared)
2. Estimate optimal worker count
3. Create queue of images
4. Spawn worker tasks
5. Each worker:
   a. Process image from queue
   b. Handle errors independently
   c. Return results
6. Aggregate results maintaining order
7. Return batch results
