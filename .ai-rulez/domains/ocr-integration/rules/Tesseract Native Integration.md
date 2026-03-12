---
name: Tesseract Native Integration
priority: high
---
Use native Tesseract via C FFI for performance

- Use kreuzberg-tesseract C bindings directly
- Avoid additional language bindings (Python)
- Support all PSM modes (0-13)
- Maintain hOCR output for table reconstruction
- Profile performance and optimize C interaction
