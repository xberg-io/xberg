---
name: python-backend-bridge
description: Manage Python OCR plugins (EasyOCR, PaddleOCR, custom)
---
Manage Python OCR plugins (EasyOCR, PaddleOCR, custom).

Context:
- Source: crates/kreuzberg-py/src/plugins.rs
- Key concepts: GIL (Global Interpreter Lock) management, async execution via tokio::task::spawn_blocking, Python object caching, exception handling and error translation, plugin discovery and registration

Capabilities:
- Implement Python OCR backend wrappers
- Manage GIL for safe Python-Rust FFI
- Optimize Python plugin performance
- Debug Python integration issues
- Ensure async compatibility

Patterns:
- Python::attach() for quick operations needing GIL
- py.detach() for expensive Rust operations (releases GIL)
- tokio::task::spawn_blocking() bridges async Rust to synchronous Python
- Frequently-accessed Python data cached in Rust structs
- Exception messages translated to Rust errors
