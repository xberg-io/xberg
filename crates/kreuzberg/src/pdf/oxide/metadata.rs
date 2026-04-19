//! PDF metadata extraction using the pdf_oxide backend.
//!
//! Provides equivalent functionality to the pdfium-based `metadata.rs` module,
//! extracting document info dictionary fields (title, author, keywords, dates,
//! producer, creator) and PDF-specific properties (version, encryption, dimensions,
//! page count). Also builds `PageStructure` from page boundaries.

use super::OxideDocument;
use crate::pdf::error::{PdfError, Result};
use crate::pdf::metadata::{CommonPdfMetadata, PdfExtractionMetadata, PdfMetadata};
use crate::types::{PageBoundary, PageInfo, PageStructure, PageUnitType};

/// Extract complete PDF metadata from an oxide document.
///
/// Combines common metadata (title, authors, dates, etc.) with PDF-specific
/// metadata (version, encryption, dimensions) and optional page structure.
/// This is the oxide equivalent of `extract_metadata_from_document_impl`.
pub(crate) fn extract_metadata_from_oxide_document(
    doc: &mut OxideDocument,
    page_boundaries: Option<&[PageBoundary]>,
    content: &str,
) -> Result<PdfExtractionMetadata> {
    let pdf_specific = extract_pdf_specific_metadata(doc)?;
    let common = extract_common_metadata(doc)?;

    let page_structure = if let Some(boundaries) = page_boundaries {
        Some(build_page_structure(doc, boundaries, content)?)
    } else {
        None
    };

    Ok(PdfExtractionMetadata {
        title: common.title,
        subject: common.subject,
        authors: common.authors,
        keywords: common.keywords,
        created_at: common.created_at,
        modified_at: common.modified_at,
        created_by: common.created_by,
        pdf_specific,
        page_structure,
    })
}

/// Extract only PDF-specific metadata (version, producer, encryption, dimensions, page count).
fn extract_pdf_specific_metadata(doc: &mut OxideDocument) -> Result<PdfMetadata> {
    let (major, minor) = doc.doc.version();
    let pdf_version = if major > 0 {
        Some(format!("{}.{}", major, minor))
    } else {
        None
    };

    let is_encrypted = Some(doc.doc.is_encrypted());

    let page_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("Failed to get page count: {}", e)))?;

    // Get first page dimensions from MediaBox
    let (width, height) = if page_count > 0 {
        match doc.doc.get_page_media_box(0) {
            Ok((llx, lly, urx, ury)) => {
                let w = (urx - llx).abs().round() as i64;
                let h = (ury - lly).abs().round() as i64;
                (Some(w), Some(h))
            }
            Err(_) => (None, None),
        }
    } else {
        (None, None)
    };

    // Extract producer from Info dictionary
    let producer = get_info_string(&mut doc.doc, "Producer");

    Ok(PdfMetadata {
        pdf_version,
        producer,
        is_encrypted,
        width,
        height,
        page_count: Some(page_count),
    })
}

/// Extract common document metadata (title, author, keywords, dates, creator)
/// from the PDF Info dictionary.
fn extract_common_metadata(doc: &mut OxideDocument) -> Result<CommonPdfMetadata> {
    let title = get_info_string(&mut doc.doc, "Title");
    let subject = get_info_string(&mut doc.doc, "Subject");
    let created_by = get_info_string(&mut doc.doc, "Creator");

    let authors = get_info_string(&mut doc.doc, "Author")
        .map(|author_str| parse_authors(&author_str))
        .filter(|parsed| !parsed.is_empty());

    let keywords = get_info_string(&mut doc.doc, "Keywords")
        .map(|kw_str| parse_keywords(&kw_str))
        .filter(|parsed| !parsed.is_empty());

    let created_at = get_info_string(&mut doc.doc, "CreationDate").map(|d| parse_pdf_date(&d));
    let modified_at = get_info_string(&mut doc.doc, "ModDate").map(|d| parse_pdf_date(&d));

    Ok(CommonPdfMetadata {
        title,
        subject,
        authors,
        keywords,
        created_at,
        modified_at,
        created_by,
    })
}

/// Retrieve a string value from the PDF Info dictionary.
///
/// Accesses the trailer `/Info` reference, resolves it, then looks up the given
/// key. Returns `None` if the Info dict is absent, the key is missing, or the
/// value cannot be decoded as a string.
fn get_info_string(doc: &mut pdf_oxide::PdfDocument, key: &str) -> Option<String> {
    // Get Info reference from trailer
    let trailer = doc.trailer().clone();
    let info_ref_obj = trailer.as_dict()?.get("Info")?.clone();

    // Resolve the reference to get the actual Info dictionary.
    // The Info entry might be a direct dictionary or an indirect reference.
    let info_obj = match info_ref_obj.as_reference() {
        Some(obj_ref) => doc.load_object(obj_ref).ok()?,
        None => info_ref_obj,
    };

    let info_dict = info_obj.as_dict()?;

    let value = info_dict.get(key)?;

    // PDF strings are stored as byte vectors; names as Strings
    match value {
        pdf_oxide::object::Object::String(bytes) => decode_pdf_string(bytes),
        pdf_oxide::object::Object::Name(name) => {
            let trimmed = name.trim().to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        }
        _ => None,
    }
}

/// Decode a PDF string (byte vector) into a Rust String.
///
/// Handles UTF-16BE encoding (BOM: 0xFE 0xFF) and falls back to Latin-1
/// (PDFDocEncoding) for byte strings without a BOM.
fn decode_pdf_string(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }

    // Check for UTF-16BE BOM
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let utf16: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        let decoded = String::from_utf16_lossy(&utf16);
        let trimmed = decoded.trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    } else if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        // UTF-8 BOM
        let decoded = String::from_utf8_lossy(&bytes[3..]);
        let trimmed = decoded.trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    } else {
        // Try UTF-8 first, then fall back to Latin-1 (PDFDocEncoding)
        match std::str::from_utf8(bytes) {
            Ok(s) => {
                let trimmed = s.trim().to_string();
                if trimmed.is_empty() { None } else { Some(trimmed) }
            }
            Err(_) => {
                let decoded: String = bytes.iter().map(|&b| b as char).collect();
                let trimmed = decoded.trim().to_string();
                if trimmed.is_empty() { None } else { Some(trimmed) }
            }
        }
    }
}

/// Build a `PageStructure` from an oxide document and page boundaries.
///
/// Mirrors `build_page_structure` in the pdfium metadata module: validates
/// boundary count against page count, collects per-page dimensions from
/// MediaBox, and determines blank status from content slices.
fn build_page_structure(doc: &mut OxideDocument, boundaries: &[PageBoundary], content: &str) -> Result<PageStructure> {
    let total_count = doc
        .doc
        .page_count()
        .map_err(|e| PdfError::MetadataExtractionFailed(format!("Failed to get page count: {}", e)))?;

    if boundaries.is_empty() {
        return Err(PdfError::MetadataExtractionFailed(
            "No page boundaries provided for PageStructure".to_string(),
        ));
    }

    if boundaries.len() != total_count {
        return Err(PdfError::MetadataExtractionFailed(format!(
            "Boundary count {} doesn't match page count {}",
            boundaries.len(),
            total_count,
        )));
    }

    let mut pages = Vec::new();
    for (index, boundary) in boundaries.iter().enumerate() {
        let page_number = boundary.page_number;

        let dimensions = match doc.doc.get_page_media_box(index) {
            Ok((llx, lly, urx, ury)) => {
                let w = (urx - llx).abs() as f64;
                let h = (ury - lly).abs() as f64;
                Some((w, h))
            }
            Err(_) => None,
        };

        let is_blank = if boundary.byte_start <= boundary.byte_end && boundary.byte_end <= content.len() {
            let page_text = &content[boundary.byte_start..boundary.byte_end];
            Some(crate::extraction::blank_detection::is_page_text_blank(page_text))
        } else {
            None
        };

        pages.push(PageInfo {
            number: page_number,
            title: None,
            dimensions,
            image_count: None,
            table_count: None,
            hidden: None,
            is_blank,
        });
    }

    Ok(PageStructure {
        total_count,
        unit_type: PageUnitType::Page,
        boundaries: Some(boundaries.to_vec()),
        pages: if pages.is_empty() { None } else { Some(pages) },
    })
}

// --- Helper functions for parsing metadata strings ---
// These mirror the implementations in `pdf::metadata` for the pdfium backend.

fn parse_authors(author_str: &str) -> Vec<String> {
    let author_str = author_str.replace(" and ", ", ");
    let mut authors = Vec::new();

    for segment in author_str.split(';') {
        for author in segment.split(',') {
            let trimmed = author.trim();
            if !trimmed.is_empty() {
                authors.push(trimmed.to_string());
            }
        }
    }

    authors
}

fn parse_keywords(keywords_str: &str) -> Vec<String> {
    keywords_str
        .replace(';', ",")
        .split(',')
        .filter_map(|k| {
            let trimmed = k.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

fn parse_pdf_date(date_str: &str) -> String {
    let cleaned = date_str.trim();

    if cleaned.starts_with("D:") && cleaned.len() >= 10 {
        let year = &cleaned[2..6];
        let month = &cleaned[6..8];
        let day = &cleaned[8..10];

        if cleaned.len() >= 16 {
            let hour = &cleaned[10..12];
            let minute = &cleaned[12..14];
            let second = &cleaned[14..16];
            format!("{}-{}-{}T{}:{}:{}Z", year, month, day, hour, minute, second)
        } else if cleaned.len() >= 14 {
            let hour = &cleaned[10..12];
            let minute = &cleaned[12..14];
            format!("{}-{}-{}T{}:{}:00Z", year, month, day, hour, minute)
        } else {
            format!("{}-{}-{}T00:00:00Z", year, month, day)
        }
    } else if cleaned.len() >= 8 {
        let year = &cleaned[0..4];
        let month = &cleaned[4..6];
        let day = &cleaned[6..8];
        format!("{}-{}-{}T00:00:00Z", year, month, day)
    } else {
        date_str.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_authors_single() {
        assert_eq!(parse_authors("John Doe"), vec!["John Doe"]);
    }

    #[test]
    fn test_parse_authors_comma() {
        assert_eq!(parse_authors("John Doe, Jane Smith"), vec!["John Doe", "Jane Smith"]);
    }

    #[test]
    fn test_parse_authors_and() {
        assert_eq!(parse_authors("John Doe and Jane Smith"), vec!["John Doe", "Jane Smith"]);
    }

    #[test]
    fn test_parse_keywords_comma() {
        assert_eq!(parse_keywords("pdf, document, test"), vec!["pdf", "document", "test"]);
    }

    #[test]
    fn test_parse_keywords_semicolon() {
        assert_eq!(parse_keywords("pdf;document;test"), vec!["pdf", "document", "test"]);
    }

    #[test]
    fn test_parse_pdf_date_full() {
        assert_eq!(parse_pdf_date("D:20230115123045"), "2023-01-15T12:30:45Z");
    }

    #[test]
    fn test_parse_pdf_date_no_time() {
        assert_eq!(parse_pdf_date("D:20230115"), "2023-01-15T00:00:00Z");
    }

    #[test]
    fn test_parse_pdf_date_no_prefix() {
        assert_eq!(parse_pdf_date("20230115"), "2023-01-15T00:00:00Z");
    }

    #[test]
    fn test_decode_pdf_string_ascii() {
        assert_eq!(decode_pdf_string(b"Hello World"), Some("Hello World".to_string()));
    }

    #[test]
    fn test_decode_pdf_string_utf16be() {
        let mut bytes = vec![0xFE, 0xFF]; // BOM
        bytes.extend_from_slice(&[0x00, b'H', 0x00, b'i']);
        assert_eq!(decode_pdf_string(&bytes), Some("Hi".to_string()));
    }

    #[test]
    fn test_decode_pdf_string_empty() {
        assert_eq!(decode_pdf_string(b""), None);
    }

    #[test]
    fn test_decode_pdf_string_whitespace_only() {
        assert_eq!(decode_pdf_string(b"   "), None);
    }
}
