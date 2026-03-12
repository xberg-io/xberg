---
name: Result Caching Correctness
priority: high
---
Cache OCR results reliably and invalidate appropriately

- Use content hash for cache keys (not filename)
- Include preprocessing config in cache key
- Include OCR backend and config in cache key
- Validate cache entries match current config
- Clear cache when major config changes
