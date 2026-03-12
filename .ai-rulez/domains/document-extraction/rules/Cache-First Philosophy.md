---
name: Cache-First Philosophy
priority: high
---
Check cache before expensive extraction operations

- Use content-based hash for cache keys
- Check cache on every extraction request
- Invalidate cache when config changes
- Return cached results with metadata
- Monitor cache hit rates
