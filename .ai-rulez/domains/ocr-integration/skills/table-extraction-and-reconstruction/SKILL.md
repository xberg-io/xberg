---
name: table-extraction-and-reconstruction
---
Identify and reconstruct tables from OCR data

1. Extract word positions from hOCR
2. Perform spatial clustering:
   - Group words by vertical alignment
   - Identify column boundaries
   - Identify row boundaries
3. Detect table cells:
   - Assign words to cells
   - Handle merged cells
   - Detect cell padding
4. Parse TSV output if available:
   - Map cells to TSV rows/cols
   - Merge with hOCR data
5. Convert to Markdown table:
   - Generate table header
   - Add alignment indicators
   - Handle spanning cells
   - Escape special characters
6. Validate table structure
