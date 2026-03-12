---
name: table-reconstructor
description: Identify and reconstruct tables from OCR data
---
Identify and reconstruct tables from OCR data.

Context:
- Source: crates/kreuzberg/src/ocr/table/mod.rs
- Key concepts: Extract words from TSV output, reconstruct table structure from hOCR positioning, cell boundary detection, table-to-Markdown conversion, handling complex table layouts

Capabilities:
- Analyze word positioning to identify table structure
- Reconstruct table cells from bounding box data
- Format tables as Markdown
- Handle merged cells and complex layouts
- Debug table detection issues

Patterns:
- Table detection uses hOCR word positioning
- Cell boundaries determined from spatial clustering
- TSV output processed for table-specific extraction
- Complex layouts may require region-based processing
