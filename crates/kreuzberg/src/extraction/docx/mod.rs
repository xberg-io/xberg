//! DOCX (Microsoft Word) text extraction.
//!
//! This module provides high-performance text extraction from DOCX files using
//! streaming XML parsing for efficiency.
//!
//! Page break detection is best-effort, detecting only explicit page breaks (`<w:br w:type="page"/>`)
//! in the document XML. This does not account for automatic pagination based on content reflowing.

pub mod drawing;
pub mod math;
pub mod parser;
pub mod section;
pub mod styles;
pub mod table;
pub mod theme;

use crate::error::Result;
use crate::types::PageBoundary;

// --- DOCX Constants ---

/// Maximum uncompressed size per file in a DOCX archive (100 MB).
pub const MAX_UNCOMPRESSED_FILE_SIZE: u64 = 100 * 1024 * 1024;
/// Maximum number of entries in a DOCX ZIP archive.
pub const MAX_ZIP_ENTRIES: usize = 10_000;
/// Maximum total uncompressed size of all files in a DOCX archive (500 MB).
pub const MAX_TOTAL_UNCOMPRESSED_SIZE: u64 = 500 * 1024 * 1024;
/// Maximum image file size for extraction (100 MB).
pub const MAX_IMAGE_FILE_SIZE: u64 = 100 * 1024 * 1024;
/// EMUs (English Metric Units) per inch.
pub const EMUS_PER_INCH: i64 = 914_400;
/// EMUs per pixel at 96 DPI.
pub const EMUS_PER_PIXEL_96DPI: i64 = 9_525;

/// Extract text from DOCX bytes.
pub fn extract_text(bytes: &[u8]) -> Result<String> {
    parser::extract_text_from_bytes(bytes)
}

/// Extract text and page boundaries from DOCX bytes.
///
/// Detects explicit page breaks (`<w:br w:type="page"/>`) in the document XML and maps them to
/// character offsets in the extracted text. This is a best-effort approach that only detects
/// explicit page breaks, not automatic pagination.
///
/// # Arguments
/// * `bytes` - The DOCX file contents as bytes
///
/// # Returns
/// * `Ok((String, Option<Vec<PageBoundary>>))` - Extracted text and optional page boundaries
/// * `Err(KreuzbergError)` - If extraction fails
///
/// # Limitations
/// - Only detects explicit page breaks, not reflowed content
/// - Page numbers are estimates, not guaranteed accurate
/// - Word's pagination may differ from detected breaks
/// - No page dimensions available (would require layout engine)
///
/// # Performance
/// Performs two passes: one with docx-lite for text extraction and one for page break detection.
pub fn extract_text_with_page_breaks(bytes: &[u8]) -> Result<(String, Option<Vec<PageBoundary>>)> {
    let doc = parser::parse_document(bytes)?;
    // Default to markdown as requested by typical extraction config
    let (text, boundaries) = doc.extract_text_with_boundaries(true);

    if boundaries.is_empty() {
        return Ok((text, None));
    }

    Ok((text, Some(boundaries)))
}

/// Detect explicit page break positions in document.xml and extract full text with page boundaries.
///
/// This is a convenience function for the extractor that combines text extraction with page
/// break detection. It returns the extracted text along with page boundaries.
///
/// # Arguments
/// * `bytes` - The DOCX file contents (ZIP archive)
///
/// # Returns
/// * `Ok(Option<Vec<PageBoundary>>)` - Optional page boundaries
/// * `Err(KreuzbergError)` - If extraction fails
///
/// # Limitations
/// - Only detects explicit page breaks, not reflowed content
/// - Page numbers are estimates based on detected breaks
pub fn detect_page_breaks_from_docx(bytes: &[u8]) -> Result<Option<Vec<PageBoundary>>> {
    match extract_text_with_page_breaks(bytes) {
        Ok((_, boundaries)) => Ok(boundaries),
        Err(e) => {
            tracing::debug!("Page break detection failed: {}", e);
            Ok(None)
        }
    }
}

/// Compute the 1-based page number for each top-level table in the document.
///
/// Scans `word/document.xml` for page-break markers (`<w:br w:type="page"/>`) and
/// top-level table opens (`<w:tbl>`), walking them in document order. Nested tables
/// (tables inside table cells) are skipped by tracking the nesting depth.
///
/// Returns a `Vec<usize>` with one entry per top-level table in document order.
/// If the document cannot be read or parsed, returns an empty Vec (callers should
/// fall back to page 1 for all tables).
///
/// # Limitations
/// - Only detects explicit page breaks, not reflowed/automatic pagination.
pub fn detect_table_page_numbers(bytes: &[u8]) -> Result<Vec<usize>> {
    let doc = parser::parse_document(bytes)?;
    Ok(doc.table_page_numbers())
}

// detect_page_breaks and map_page_breaks_to_boundaries are removed as their logic
// is now integrated into the DocxParser and parser::Document struct.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_empty() {
        let result = extract_text(b"");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_text_invalid() {
        let result = extract_text(b"not a docx file");
        assert!(result.is_err());
    }

    /// Build a minimal in-memory DOCX ZIP with the given document.xml body content.
    fn build_test_docx(body: &str) -> Vec<u8> {
        use std::io::Write;

        let document_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>{}</w:body>
</w:document>"#,
            body
        );

        let content_types = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

        let cursor = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(cursor);
        let opts: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        zip.start_file("[Content_Types].xml", opts).unwrap();
        zip.write_all(content_types.as_bytes()).unwrap();
        zip.start_file("word/document.xml", opts).unwrap();
        zip.write_all(document_xml.as_bytes()).unwrap();
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn test_extract_text_with_page_breaks_accurate_boundaries() {
        // Page 1: Very long text
        // Page 2: Very short text
        // The old heuristic split in half.
        // The new logic should split exactly at the \f boundary.
        let body = r#"
<w:p><w:r><w:t>This is page one. It has a lot of text to ensure we don't just split in half.</w:t></w:r></w:p>
<w:p><w:r><w:br w:type="page"/></w:r></w:p>
<w:p><w:r><w:t>Short</w:t></w:r></w:p>
"#;
        let docx = build_test_docx(body);
        let (text, boundaries) = extract_text_with_page_breaks(&docx).unwrap();
        let boundaries = boundaries.expect("Should have detected page break");

        assert_eq!(boundaries.len(), 2);
        assert_eq!(boundaries[0].page_number, 1);
        assert_eq!(boundaries[1].page_number, 2);

        let p1_content = &text[boundaries[0].byte_start..boundaries[0].byte_end];
        let p2_content = &text[boundaries[1].byte_start..boundaries[1].byte_end];

        assert!(p1_content.contains("This is page one"));
        assert_eq!(p2_content.trim(), "Short");
        assert!(!p1_content.contains("Short"));

        // Verify \f is at the expected position (between boundaries)
        assert_eq!(text.as_bytes()[boundaries[0].byte_end], b'\x0c');
    }

    #[test]
    fn test_extract_text_with_last_rendered_page_break() {
        let body = r#"
<w:p><w:r><w:t>Page 1</w:t><w:lastRenderedPageBreak/><w:t>Page 2</w:t></w:r></w:p>
"#;
        let docx = build_test_docx(body);
        let (text, boundaries) = extract_text_with_page_breaks(&docx).unwrap();
        let boundaries = boundaries.expect("Should have detected last rendered page break");

        assert_eq!(boundaries.len(), 2);
        assert_eq!(boundaries[0].page_number, 1);
        assert_eq!(boundaries[1].page_number, 2);

        let p1_content = &text[boundaries[0].byte_start..boundaries[0].byte_end];
        let p2_content = &text[boundaries[1].byte_start..boundaries[1].byte_end];

        assert_eq!(p1_content.trim(), "Page 1");
        assert_eq!(p2_content.trim(), "Page 2");
    }

    #[test]
    fn test_extract_text_with_page_breaks_no_breaks() {
        let body = r#"<w:p><w:r><w:t>Single page</w:t></w:r></w:p>"#;
        let docx = build_test_docx(body);
        let result = extract_text_with_page_breaks(&docx).unwrap();
        assert!(result.1.is_none());
        assert_eq!(result.0.trim(), "Single page");
    }

    #[test]
    fn test_detect_table_page_numbers_accurate() {
        let body = r#"
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Table 1</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
<w:p><w:r><w:br w:type="page"/></w:r></w:p>
<w:p><w:r><w:t>Small gap</w:t></w:r></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Table 2</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
"#;
        let docx = build_test_docx(body);
        let result = detect_table_page_numbers(&docx).unwrap();
        assert_eq!(result, vec![1, 2]);
    }

    #[test]
    fn test_detect_table_page_numbers_no_tables() {
        let body = r#"<w:p><w:r><w:t>Hello</w:t></w:r></w:p>"#;
        let docx = build_test_docx(body);
        let result = detect_table_page_numbers(&docx).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_detect_table_page_numbers_invalid_docx() {
        let result = detect_table_page_numbers(b"not a docx");
        assert!(result.is_err());
    }

    /// A `<w:lastRenderedPageBreak/>` inside a table cell must NOT create a phantom page
    /// boundary before the table. The table should remain on page 1.
    #[test]
    fn test_page_break_inside_table_not_counted() {
        let body = r#"
<w:p><w:r><w:t>Page 1</w:t></w:r></w:p>
<w:tbl>
  <w:tr><w:tc><w:p><w:r><w:lastRenderedPageBreak/><w:t>Cell content</w:t></w:r></w:p></w:tc></w:tr>
</w:tbl>
<w:p><w:r><w:t>Still page 1</w:t></w:r></w:p>
"#;
        let docx = build_test_docx(body);
        // No page breaks should be detected since the break is inside a table cell
        let result = extract_text_with_page_breaks(&docx).unwrap();
        assert!(
            result.1.is_none(),
            "Page break inside table cell should not create page boundaries"
        );

        // The table should be on page 1
        let table_pages = detect_table_page_numbers(&docx).unwrap();
        assert_eq!(table_pages, vec![1]);
    }

    /// Explicit page break inside a table cell must not disrupt document-level page count.
    #[test]
    fn test_explicit_page_break_inside_table_not_counted() {
        let body = r#"
<w:tbl>
  <w:tr><w:tc><w:p><w:r><w:br w:type="page"/></w:r></w:p></w:tc></w:tr>
</w:tbl>
<w:p><w:r><w:t>After table</w:t></w:r></w:p>
"#;
        let docx = build_test_docx(body);
        let result = extract_text_with_page_breaks(&docx).unwrap();
        assert!(
            result.1.is_none(),
            "Explicit page break inside table cell should not create page boundaries"
        );
    }
}
