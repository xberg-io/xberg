---
name: Graceful Backend Degradation
priority: high
---
Continue operation when backends fail

- Catch backend exceptions
- Try next-priority backend on failure
- Return partial results when possible
- Document backend failures
- Retry with alternative backend
