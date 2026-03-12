---
name: Efficient Lookups
priority: high
---
Optimize plugin selection performance

- O(log n) MIME type lookup via indexing
- O(n) priority sorting (acceptable for small plugin counts)
- Cache registry queries when appropriate
- Avoid O(n) operations in hot paths
- Profile and optimize lookup performance
