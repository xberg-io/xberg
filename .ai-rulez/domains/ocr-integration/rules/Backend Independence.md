---
name: Backend Independence
priority: high
---
Backends must not depend on each other

- Each backend has independent initialization
- Failures in one backend don't affect others
- Shared caching doesn't create backend coupling
- Languages configurable per backend
- Graceful degradation if backend unavailable
