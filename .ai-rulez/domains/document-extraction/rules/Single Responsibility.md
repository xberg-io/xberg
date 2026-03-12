---
name: Single Responsibility
priority: high
---
Each component in extraction pipeline has focused responsibility

- MIME detection handles format identification only
- Extractor selection handles routing to appropriate handler
- Extractors implement extraction for specific format
- Post-processors enhance results
- Cache manager handles persistence
