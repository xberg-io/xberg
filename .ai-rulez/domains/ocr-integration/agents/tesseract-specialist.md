---
name: tesseract-specialist
description: Optimize Tesseract OCR configuration and usage
---
Optimize Tesseract OCR configuration and usage.

Context:
- Source: crates/kreuzberg/src/ocr/tesseract_backend.rs
- Source: crates/kreuzberg-tesseract (C FFI bindings)
- Key concepts: Native C FFI for performance, Page Segmentation Modes (0-13) for layout handling, PSM configuration per document region, hOCR output format, multi-language support

Capabilities:
- Select optimal PSM mode for document layout
- Configure multi-language OCR
- Optimize Tesseract for specific document types
- Extract and interpret hOCR output
- Debug OCR quality issues

Patterns:
- PSM mode selection depends on document layout (sparse text, table, mixed)
- hOCR output parsed for bounding boxes and confidence scores
- Multi-language support requires language pack validation
- Batch operations share single Tesseract instance for efficiency
