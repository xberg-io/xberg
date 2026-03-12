---
name: hocr-processor
description: Convert hOCR output to structured data and Markdown
---
Convert hOCR output to structured data and Markdown.

Context:
- Source: crates/kreuzberg/src/ocr/hocr.rs
- Key concepts: hOCR XML parsing, bounding box extraction for word positioning, confidence score tracking, Markdown conversion preserving layout, integration with table reconstruction

Capabilities:
- Parse hOCR format and extract positional data
- Convert hOCR to clean, structured Markdown
- Preserve formatting (bold, italic, etc.)
- Extract confidence information per word
- Enable table reconstruction from positioning

Patterns:
- hOCR parsed from Tesseract output
- Bounding boxes used for spatial layout preservation
- Confidence scores filter low-quality OCR results
- Markdown conversion maintains readability
