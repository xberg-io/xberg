---
name: Async/Await Consistency
priority: high
---
Use async extraction APIs for non-blocking I/O

- Primary APIs are async (extract_file, extract_bytes)
- Synchronous wrappers use global Tokio runtime
- No blocking in async contexts
- Batch operations use concurrent execution
- Properly handle cancellation and timeouts
