---
name: Thread Safety Requirement
priority: high
---
All plugins must be Send + Sync for concurrent execution

- Plugin implementations must be thread-safe
- Use Arc for shared state
- Protect mutable state with locks (RwLock, Mutex)
- Avoid thread-local storage in plugins
- Test thread safety with concurrent access
