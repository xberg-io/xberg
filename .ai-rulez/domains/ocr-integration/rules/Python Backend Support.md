---
name: Python Backend Support
priority: high
---
Enable EasyOCR/PaddleOCR via Python plugin interface

- Wrap Python backends via PyO3 FFI
- Manage GIL safely for async execution
- Cache Python object references
- Translate Python exceptions to Rust errors
- Support configurable model loading
