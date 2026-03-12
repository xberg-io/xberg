---
name: ocr-orchestrator
description: Coordinate OCR backend selection and image processing workflows
---
Coordinate OCR backend selection and image processing workflows.

Context:
- Source: crates/kreuzberg/src/ocr/processor.rs
- Source: crates/kreuzberg/src/ocr/mod.rs
- Key concepts: OcrProcessor entry point, backend registry management, single image and batch processing, result caching and performance optimization

Capabilities:
- Understand OCR architecture and backend plugins
- Select optimal OCR engine for image characteristics
- Implement batch processing with concurrent execution
- Optimize OCR performance through caching

Patterns:
- Preprocessing, OCR backend selection, execution, hOCR parsing, caching
- Batch operations pool resources and execute concurrently
- Backend selection considers language support, image characteristics, and priority
