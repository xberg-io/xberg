---
name: ocr-cache-manager
description: Optimize OCR performance through intelligent caching
---
Optimize OCR performance through intelligent caching.

Context:
- Source: crates/kreuzberg/src/ocr/cache.rs
- Key concepts: Content-based hashing for cache keys, persistent cache backends (filesystem, Redis), cache statistics and metrics, invalidation on config changes, cache eviction strategies

Capabilities:
- Design OCR caching strategies
- Monitor cache hit rates and performance
- Implement cache invalidation logic
- Optimize storage and retrieval
- Handle cache consistency

Patterns:
- Cache key based on image content hash
- Persistent storage prevents re-OCR across process boundaries
- Cache invalidated when TesseractConfig changes
- Sub-millisecond cache lookups
- 30-50% hit rate in production
