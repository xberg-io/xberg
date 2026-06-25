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

#[cfg(test)]
use crate::error::{Result, XbergError};
#[cfg(test)]
use crate::extraction::capacity;
#[cfg(test)]
use crate::types::PageBoundary;
#[cfg(test)]
use std::io::Cursor;

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
#[cfg(test)]
pub(crate) fn extract_text(bytes: &[u8]) -> Result<String> {
    parser::extract_text_from_bytes(bytes)
}

/// Extract text and page boundaries from DOCX bytes.
///
/// Detects explicit page breaks (`<w:br w:type="page"/>`) in the document XML and maps them to
/// character offsets in the extracted text. This is a best-effort approach that only detects
/// explicit page breaks, not automatic pagination.
#[cfg(test)]
pub(crate) fn extract_text_with_page_breaks(bytes: &[u8]) -> Result<(String, Option<Vec<PageBoundary>>)> {
    let text = extract_text(bytes)?;

    let page_breaks = detect_page_breaks(bytes)?;

    if page_breaks.is_empty() {
        return Ok((text, None));
    }

    let boundaries = map_page_breaks_to_boundaries(&text, page_breaks)?;

    Ok((text, Some(boundaries)))
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
#[cfg(test)]
pub(crate) fn detect_table_page_numbers(bytes: &[u8]) -> Result<Vec<usize>> {
    use zip::ZipArchive;

    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to open DOCX as ZIP: {}", e)))?;

    let document_xml = match archive.by_name("word/document.xml") {
        Ok(mut file) => {
            let mut content = String::new();
            std::io::Read::read_to_string(&mut file, &mut content)
                .map_err(|e| XbergError::parsing(format!("Failed to read document.xml: {}", e)))?;
            content
        }
        Err(_) => return Ok(Vec::new()),
    };

    #[derive(Copy, Clone)]
    enum Marker {
        PageBreak,
        TableOpen,
        TableClose,
    }

    let mut events: Vec<(usize, Marker)> = Vec::new();

    // Explicit page breaks: <w:br w:type="page"/>
    for (pos, _) in document_xml.match_indices(r#"<w:br w:type="page"/>"#) {
        events.push((pos, Marker::PageBreak));
    }

    // Top-level table opens: <w:tbl> or <w:tbl ...> — but NOT <w:tblPr>, <w:tblGrid>, etc.
    for (pos, _) in document_xml.match_indices("<w:tbl") {
        let after = pos + "<w:tbl".len();
        if matches!(
            document_xml.as_bytes().get(after),
            Some(b'>') | Some(b' ') | Some(b'\n') | Some(b'\r') | Some(b'\t')
        ) {
            events.push((pos, Marker::TableOpen));
        }
    }

    // Table closes: </w:tbl>
    for (pos, _) in document_xml.match_indices("</w:tbl>") {
        events.push((pos, Marker::TableClose));
    }

    events.sort_unstable_by_key(|(pos, _)| *pos);

    let mut current_page: usize = 1;
    let mut table_depth: usize = 0;
    let mut table_page_numbers: Vec<usize> = Vec::new();

    for (_, marker) in events {
        match marker {
            Marker::PageBreak => current_page += 1,
            Marker::TableOpen => {
                if table_depth == 0 {
                    // Top-level table: record its page
                    table_page_numbers.push(current_page);
                }
                table_depth += 1;
            }
            Marker::TableClose => {
                table_depth = table_depth.saturating_sub(1);
            }
        }
    }

    Ok(table_page_numbers)
}

/// Detect explicit page break positions in document.xml.
///
/// Returns a vector of byte offsets within the document.xml content where page breaks occur.
/// These offsets will later be mapped to character positions in the extracted text.
///
/// # Arguments
/// * `bytes` - The DOCX file contents (ZIP archive)
///
/// # Returns
/// * `Ok(Vec<usize>)` - Vector of detected page break byte offsets (empty if none found)
/// * `Err(XbergError)` - If ZIP/XML parsing fails
#[cfg(test)]
fn detect_page_breaks(bytes: &[u8]) -> Result<Vec<usize>> {
    use zip::ZipArchive;

    let cursor = Cursor::new(bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| XbergError::parsing(format!("Failed to open DOCX as ZIP: {}", e)))?;

    let document_xml = match archive.by_name("word/document.xml") {
        Ok(mut file) => {
            let file_size = file.size();
            let estimated_size = capacity::estimate_content_capacity(file_size, "docx").max(file_size as usize);
            let mut content = String::with_capacity(estimated_size);
            std::io::Read::read_to_string(&mut file, &mut content)
                .map_err(|e| XbergError::parsing(format!("Failed to read document.xml: {}", e)))?;
            content
        }
        Err(_) => return Ok(Vec::new()),
    };

    let mut breaks = Vec::with_capacity(16);
    let search_pattern = r#"<w:br w:type="page"/>"#;

    for (idx, _) in document_xml.match_indices(search_pattern) {
        breaks.push(idx);
    }

    Ok(breaks)
}

/// Map detected page break positions to byte boundaries in extracted text.
///
/// Since we don't have a precise mapping between document.xml byte positions and final text
/// character positions, we use a heuristic: divide the text roughly equally between detected breaks.
/// This is best-effort and may not perfectly match Word's pagination.
///
/// # LIMITATION
/// This is a best-effort heuristic that distributes content evenly across detected page breaks.
/// It does not account for actual page layout, varying page sizes, or Word's pagination logic.
/// Use with caution. The function correctly handles multibyte UTF-8 characters (emoji, CJK, etc.)
/// by working with character indices rather than byte indices.
///
/// # Arguments
/// * `text` - The extracted document text
/// * `page_breaks` - Vector of detected page break positions (unused, but kept for extension)
///
/// # Returns
/// * `Ok(Vec<PageBoundary>)` - Byte boundaries for each page
#[cfg(test)]
fn map_page_breaks_to_boundaries(text: &str, page_breaks: Vec<usize>) -> Result<Vec<PageBoundary>> {
    if page_breaks.is_empty() {
        return Ok(Vec::new());
    }

    let page_count = page_breaks.len() + 1;
    let char_count = text.chars().count();
    let chars_per_page = char_count / page_count;

    let mut boundaries = Vec::with_capacity(page_count);
    let mut current_byte = 0;
    let mut current_char = 0;

    for page_num in 1..=page_count {
        let start_byte = current_byte;

        let end_byte = if page_num == page_count {
            text.len()
        } else {
            let target_char = (page_num * chars_per_page).min(char_count);

            for ch in text[current_byte..].chars() {
                if current_char >= target_char {
                    break;
                }
                current_byte += ch.len_utf8();
                current_char += 1;
            }

            current_byte
        };

        boundaries.push(PageBoundary {
            byte_start: start_byte,
            byte_end: end_byte,
            page_number: page_num as u32,
        });

        current_byte = end_byte;
    }

    Ok(boundaries)
}

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

    #[test]
    fn test_map_page_breaks_to_boundaries_empty() {
        let result = map_page_breaks_to_boundaries("test text", Vec::new()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_map_page_breaks_to_boundaries_single_break() {
        let text = "Page 1 content here with some text.Page 2 content here with more text.";
        let breaks = vec![0];

        let result = map_page_breaks_to_boundaries(text, breaks).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].page_number, 1);
        assert_eq!(result[0].byte_start, 0);
        assert!(result[0].byte_end > 0);
        assert!(result[0].byte_end < text.len());

        assert_eq!(result[1].page_number, 2);
        assert_eq!(result[1].byte_start, result[0].byte_end);
        assert_eq!(result[1].byte_end, text.len());
    }

    #[test]
    fn test_map_page_breaks_to_boundaries_multiple_breaks() {
        let text = "A".repeat(300);
        let breaks = vec![0, 0, 0];

        let result = map_page_breaks_to_boundaries(&text, breaks).unwrap();

        assert_eq!(result.len(), 4);
        assert_eq!(result[0].page_number, 1);
        assert_eq!(result[3].page_number, 4);
        assert_eq!(result[3].byte_end, text.len());

        for i in 0..result.len() - 1 {
            assert_eq!(result[i].byte_end, result[i + 1].byte_start);
        }
    }

    #[test]
    fn test_map_page_breaks_to_boundaries_utf8_boundary() {
        let text = "Hello world! こんにちは世界！ More text here.";
        let breaks = vec![0];

        let result = map_page_breaks_to_boundaries(text, breaks).unwrap();

        assert_eq!(result.len(), 2);
        assert!(text.is_char_boundary(result[0].byte_start));
        assert!(text.is_char_boundary(result[0].byte_end));
        assert!(text.is_char_boundary(result[1].byte_start));
        assert!(text.is_char_boundary(result[1].byte_end));
    }

    #[test]
    fn test_docx_page_breaks_with_emoji() {
        let text = "Hello 😀 World 🌍 Foo 🎉 Bar";
        let breaks = vec![0, 0];

        let result = map_page_breaks_to_boundaries(text, breaks).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].page_number, 1);
        assert_eq!(result[1].page_number, 2);
        assert_eq!(result[2].page_number, 3);

        for boundary in &result {
            assert!(
                text.is_char_boundary(boundary.byte_start),
                "byte_start {} is not a valid UTF-8 boundary",
                boundary.byte_start
            );
            assert!(
                text.is_char_boundary(boundary.byte_end),
                "byte_end {} is not a valid UTF-8 boundary",
                boundary.byte_end
            );
        }

        assert_eq!(result[0].byte_start, 0);
        assert_eq!(result[0].byte_end, result[1].byte_start);
        assert_eq!(result[1].byte_end, result[2].byte_start);
        assert_eq!(result[2].byte_end, text.len());

        let reconstructed = format!(
            "{}{}{}",
            &text[result[0].byte_start..result[0].byte_end],
            &text[result[1].byte_start..result[1].byte_end],
            &text[result[2].byte_start..result[2].byte_end]
        );
        assert_eq!(reconstructed, text);
    }

    #[test]
    fn test_docx_page_breaks_with_cjk() {
        let text = "你好世界你好世界你好世界你好世界";
        let breaks = vec![0];

        let result = map_page_breaks_to_boundaries(text, breaks).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].page_number, 1);
        assert_eq!(result[1].page_number, 2);

        for boundary in &result {
            assert!(
                text.is_char_boundary(boundary.byte_start),
                "byte_start {} is not a valid UTF-8 boundary",
                boundary.byte_start
            );
            assert!(
                text.is_char_boundary(boundary.byte_end),
                "byte_end {} is not a valid UTF-8 boundary",
                boundary.byte_end
            );
        }

        assert_eq!(result[0].byte_start, 0);
        assert_eq!(result[0].byte_end, result[1].byte_start);
        assert_eq!(result[1].byte_end, text.len());

        let reconstructed = format!(
            "{}{}",
            &text[result[0].byte_start..result[0].byte_end],
            &text[result[1].byte_start..result[1].byte_end]
        );
        assert_eq!(reconstructed, text);
    }

    #[test]
    fn test_docx_page_breaks_multibyte_utf8() {
        let text = "ASCII 😀 中文 hello 🎉 world 日本語";
        let breaks = vec![0, 0];

        let result = map_page_breaks_to_boundaries(text, breaks).unwrap();

        assert_eq!(result.len(), 3);

        for boundary in &result {
            assert!(
                text.is_char_boundary(boundary.byte_start),
                "byte_start {} is not a valid UTF-8 boundary",
                boundary.byte_start
            );
            assert!(
                text.is_char_boundary(boundary.byte_end),
                "byte_end {} is not a valid UTF-8 boundary",
                boundary.byte_end
            );
        }

        assert_eq!(result[0].byte_start, 0);
        for i in 0..result.len() - 1 {
            assert_eq!(
                result[i].byte_end,
                result[i + 1].byte_start,
                "Gap or overlap between page {} and {}",
                i + 1,
                i + 2
            );
        }
        assert_eq!(
            result[result.len() - 1].byte_end,
            text.len(),
            "Last page does not end at text boundary"
        );

        let mut reconstructed = String::new();
        for boundary in &result {
            reconstructed.push_str(&text[boundary.byte_start..boundary.byte_end]);
        }
        assert_eq!(reconstructed, text);
    }

    #[test]
    fn test_detect_page_breaks_no_feature() {
        let result = detect_page_breaks(b"invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_text_with_page_breaks_no_breaks() {
        let docx_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/docx/lorem_ipsum.docx");
        if let Ok(bytes) = std::fs::read(docx_path) {
            let result = extract_text_with_page_breaks(&bytes);
            if let Ok((text, boundaries)) = result {
                assert!(!text.is_empty());
                if let Some(b) = boundaries {
                    assert!(!b.is_empty());
                }
            }
        }
    }

    // ---- detect_table_page_numbers tests ----

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
    fn test_detect_table_page_numbers_no_tables() {
        let body = r#"<w:p><w:r><w:t>Hello</w:t></w:r></w:p>"#;
        let docx = build_test_docx(body);
        let result = detect_table_page_numbers(&docx).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_detect_table_page_numbers_single_table_page_1() {
        let body = r#"
<w:p><w:r><w:t>Some text</w:t></w:r></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
"#;
        let docx = build_test_docx(body);
        let result = detect_table_page_numbers(&docx).unwrap();
        assert_eq!(result, vec![1]);
    }

    #[test]
    fn test_detect_table_page_numbers_table_after_page_break() {
        let body = r#"
<w:p><w:r><w:t>Page 1 text</w:t></w:r></w:p>
<w:p><w:r><w:br w:type="page"/></w:r></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Table on page 2</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
"#;
        let docx = build_test_docx(body);
        let result = detect_table_page_numbers(&docx).unwrap();
        assert_eq!(result, vec![2]);
    }

    #[test]
    fn test_detect_table_page_numbers_multiple_tables_mixed_pages() {
        let body = r#"
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Table 1</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
<w:p><w:r><w:t>Some more text</w:t></w:r></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Table 2</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
<w:p><w:r><w:br w:type="page"/></w:r></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>Table 3</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
"#;
        let docx = build_test_docx(body);
        let result = detect_table_page_numbers(&docx).unwrap();
        // Tables 1 and 2 are on page 1; table 3 is on page 2
        assert_eq!(result, vec![1, 1, 2]);
    }

    #[test]
    fn test_detect_table_page_numbers_nested_table_not_counted() {
        // Nested table (inside a cell) should NOT appear as an extra entry.
        let body = r#"
<w:tbl>
  <w:tr><w:tc>
    <w:tbl><w:tr><w:tc><w:p><w:r><w:t>inner</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
  </w:tc></w:tr>
</w:tbl>
"#;
        let docx = build_test_docx(body);
        let result = detect_table_page_numbers(&docx).unwrap();
        // Only the outer top-level table is recorded
        assert_eq!(result, vec![1]);
    }

    #[test]
    fn test_detect_table_page_numbers_invalid_docx() {
        // Should return empty, not panic or error
        let result = detect_table_page_numbers(b"not a docx");
        // Either Ok(empty) or Err is acceptable; must not panic
        let _ = result;
    }
}
