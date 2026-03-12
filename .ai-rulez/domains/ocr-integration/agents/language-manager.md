---
name: language-manager
description: Manage OCR language support and detection
---
Manage OCR language support and detection.

Context:
- Source: crates/kreuzberg/src/ocr/language_registry.rs
- Key concepts: Language pack detection and validation, multi-language support, language configuration, runtime language availability checks

Capabilities:
- Validate Tesseract language packs
- Configure multi-language OCR
- Detect and handle missing language support
- Optimize language selection
- Debug language-related OCR issues

Patterns:
- Language packs validated at initialization
- Multi-language OCR improves accuracy for mixed documents
- Missing language packs trigger fallback or error
- Language configuration per document improves accuracy
