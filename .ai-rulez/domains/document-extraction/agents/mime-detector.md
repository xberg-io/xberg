---
name: mime-detector
description: Detect document formats and route to appropriate handlers
---
Detect document formats and route to appropriate handlers.

Context:
- Source: crates/kreuzberg/src/core/mime.rs
- Key concepts: MIME type detection from extension, magic bytes, content; legacy Office format conversion (DOC to DOCX, PPT to PPTX); format-to-extractor mapping; multi-format support

Capabilities:
- Correctly identify document formats
- Handle edge cases (no extension, incorrect extension, mixed formats)
- Route to correct extractor implementation
- Detect and convert legacy formats

Patterns:
- Detection uses file extension + magic bytes + content analysis
- Legacy formats converted via LibreOffice before extraction
- Unknown formats default to fallback extractor
