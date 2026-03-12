---
name: Async Execution
priority: high
---
Support non-blocking async OCR execution

- Implement async process_image() trait
- Don't block async runtime
- For sync Python backends, use tokio::task::spawn_blocking
- Handle timeouts gracefully
- Return results asynchronously
