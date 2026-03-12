---
name: Async Execution Requirement
priority: high
---
All OCR operations must support async execution

- Implement async OcrBackend trait
- Use tokio::task::spawn_blocking for sync Python
- Never block the async runtime
- Support concurrent image processing
- Handle timeouts gracefully
