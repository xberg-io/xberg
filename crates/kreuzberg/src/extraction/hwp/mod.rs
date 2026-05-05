//! Vendored HWP text extraction from hwpers v0.5.0 (MIT OR Apache-2.0)
//!
//! Supports HWP 5.0 Compound File Binary (CFB) documents.  Only text
//! extraction is implemented; write, render, crypto, HWPX, and preview paths
//! from the original crate are omitted.
//!
//! # Entry point
//!
//! ```ignore
//! let text = extract_hwp_text(bytes)?;
//! ```

pub mod error;
pub mod model;
pub mod parser;
pub mod reader;

use crate::extraction::hwp::model::HwpDocument;
use error::{HwpError, Result};
use parser::{FileHeader, parse_body_text, parse_doc_info};
use reader::CfbReader;

/// Extract the structured document model from an HWP 5.0 document.
pub(crate) fn extract_hwp_document(bytes: &[u8]) -> Result<HwpDocument> {
    let mut cfb = CfbReader::from_bytes(bytes)?;

    // Parse the 256-byte file header
    let header_data = cfb.read_stream("FileHeader")?;
    let header = FileHeader::parse(header_data)?;

    if header.is_encrypted() {
        return Err(HwpError::UnsupportedVersion(
            "Password-encrypted HWP documents are not supported".to_string(),
        ));
    }

    let mut doc = HwpDocument::default();

    // Parse DocInfo for global tables (char shapes, etc.)
    if cfb.stream_exists("DocInfo") {
        let doc_info_data = cfb.read_stream("DocInfo")?;
        if let Ok(char_shapes) = parse_doc_info(doc_info_data) {
            doc.char_shapes = char_shapes;
        }
    }

    // Body text is distributed across Section0..SectionN streams inside BodyText
    let mut streams = cfb.list_streams();
    streams.sort();

    for path in streams {
        if path.starts_with("BodyText/Section") {
            let section_data = cfb.read_stream(&path)?;
            if let Ok(sections) = parse_body_text(section_data, header.is_compressed()) {
                doc.sections.extend(sections);
            }
        }
    }

    // Attempt to extract images from BinData streams
    for path in cfb.list_streams() {
        if path.starts_with("BinData/") {
            let image_data = cfb.read_stream(&path)?;
            doc.images.push(model::HwpImage {
                name: path.clone(),
                data: image_data,
            });
        }
    }

    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_invalid_hwp() {
        let bytes = b"Not a valid HWP file";
        assert!(extract_hwp_document(bytes).is_err());
    }
}
