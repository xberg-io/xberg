---
name: Registry Consistency
priority: high
---
Maintain registry consistency across operations

- Register and indices stay synchronized
- MIME type index updated with registrations
- Unregister removes from index atomically
- Query results reflect current registry state
- Clear operations empty all data structures
