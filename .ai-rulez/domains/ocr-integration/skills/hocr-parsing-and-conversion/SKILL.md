---
name: hocr-parsing-and-conversion
---
Extract structured data from Tesseract hOCR output

1. Parse hOCR XML from Tesseract
2. Extract word elements with:
   - Text content
   - Bounding box (x, y, width, height)
   - Confidence score
   - Language info
3. Group words into:
   - Lines
   - Paragraphs
   - Text blocks
4. Convert formatting:
   - Bold text
   - Italic text
   - Spacing/alignment
5. Generate Markdown output:
   - Heading levels
   - Lists
   - Text formatting
6. Feed positioning data to table reconstruction
