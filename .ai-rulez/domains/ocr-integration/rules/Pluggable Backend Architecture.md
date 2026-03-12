---
name: Pluggable Backend Architecture
priority: high
---
OCR backends must be interchangeable and selectable

- Implement OcrBackend trait for all backends
- Support Tesseract, EasyOCR, PaddleOCR natively
- Enable custom backend registration
- Document backend capabilities
- Use priority system for backend selection
