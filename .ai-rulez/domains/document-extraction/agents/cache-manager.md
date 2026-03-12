---
name: cache-manager
description: Optimize extraction performance through intelligent caching
---
Optimize extraction performance through intelligent caching.

Context:
- Key concepts: Content-based hashing for cache keys, persistent cache storage, cache invalidation on config changes, cache statistics and monitoring

Capabilities:
- Design and implement caching strategies
- Optimize cache hit rates
- Monitor cache performance metrics
- Handle cache invalidation

Patterns:
- Cache keys based on file content hash, not filename
- Cache invalidated when extraction config changes
- Sub-millisecond cache lookups
- 40-60% cache hit rate in production
