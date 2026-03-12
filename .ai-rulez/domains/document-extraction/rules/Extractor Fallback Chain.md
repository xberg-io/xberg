---
name: Extractor Fallback Chain
priority: high
---
Implement robust fallback when primary extraction fails

- Select highest-priority extractor for MIME type
- On failure, try next-priority extractor
- Continue chain until success or all extractors fail
- Preserve error information from each attempt
- Batch operations continue with partial results
