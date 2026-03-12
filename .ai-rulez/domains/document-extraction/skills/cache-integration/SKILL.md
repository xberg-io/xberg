---
name: cache-integration
---
Optimize extraction with content-based caching

1. Compute content hash of file
2. Generate cache key from hash
3. Query cache for entry
4. If cache hit, return cached result
5. If cache miss, proceed with extraction
6. Validate cache entry matches current config
7. Store result with metadata
8. Track cache statistics
