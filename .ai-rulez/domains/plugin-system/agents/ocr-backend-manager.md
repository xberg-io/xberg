---
name: ocr-backend-manager
description: Manage OCR backend plugin lifecycle and selection
---
Manage OCR backend plugin lifecycle and selection.

Context:
- Source: crates/kreuzberg-py/src/plugins.rs
- Key concepts: OcrBackend trait for alternative OCR engines, language support capabilities, async processing interface, Python plugin support via FFI
- Integration points: OCR processor pipeline, capability-based backend selection

Capabilities:
- Register OCR backend plugins (Rust or Python)
- Implement OcrBackend trait for new engines
- Configure backend priority
- Manage Python backend GIL
- Select optimal backend for language/image characteristics

Patterns:
- Multiple backends can handle same language
- Backend selection considers language support and capability match
- Python backends executed via tokio::task::spawn_blocking
- GIL management critical for performance
