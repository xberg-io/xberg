---
name: Registry Thread Safety
priority: high
---
Registries must support concurrent access

- Use Arc<RwLock<>> for registry storage
- Read operations use read locks
- Write operations use write locks
- No deadlock possibilities
- Minimize lock hold times
- Test concurrent registry operations
