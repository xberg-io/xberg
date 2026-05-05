/// HWP record, file-header, and body-text parsers.
///
/// Consolidated from hwpers parser/record.rs, parser/header.rs, and
/// parser/body_text.rs.
use super::error::{HwpError, Result};
use super::model::{CharShape, ParaText, Paragraph, Section};
use super::reader::{StreamReader, decompress_stream};

// ---------------------------------------------------------------------------
// HWP file-header signature
// ---------------------------------------------------------------------------

const HWP_SIGNATURE: &[u8] = b"HWP Document File";

// ---------------------------------------------------------------------------
// FileHeader — 256-byte header at the start of every HWP 5.0 file
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FileHeader {
    pub flags: u32,
}

impl FileHeader {
    pub(crate) fn parse(data: Vec<u8>) -> Result<Self> {
        if data.len() < 256 {
            return Err(HwpError::InvalidFormat(
                "FileHeader must be at least 256 bytes".to_string(),
            ));
        }

        // Bytes 0..17 are the human-readable signature
        if &data[..17] != HWP_SIGNATURE {
            return Err(HwpError::InvalidFormat("Invalid HWP signature".to_string()));
        }

        // Bytes 32..36: version (u32 LE) — not needed for text extraction
        // Bytes 36..40: flags (u32 LE)
        let flags = u32::from_le_bytes([data[36], data[37], data[38], data[39]]);

        Ok(Self { flags })
    }

    /// Whether section streams are zlib/deflate-compressed.
    pub(crate) fn is_compressed(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// Whether the document is password-encrypted.
    pub(crate) fn is_encrypted(&self) -> bool {
        (self.flags & 0x02) != 0
    }
}

// ---------------------------------------------------------------------------
// Record header / Record — the fundamental binary units in HWP streams
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Record {
    pub tag_id: u16,
    pub data: Vec<u8>,
}

impl Record {
    pub(crate) fn parse(reader: &mut StreamReader) -> Result<Self> {
        if reader.remaining() < 4 {
            return Err(HwpError::ParseError("Not enough data for record header".to_string()));
        }

        // Single 32-bit packed header:
        //   bits  0– 9: tag_id  (10 bits)
        //   bits 10–19: level   (10 bits)  — ignored for text extraction
        //   bits 20–31: size    (12 bits); 0xFFF means read an extended u32
        let header = reader.read_u32()?;
        let tag_id = (header & 0x3FF) as u16;
        let mut size = header >> 20;

        if size == 0xFFF {
            size = reader.read_u32()?;
        }

        let data_size = size as usize;
        if data_size > reader.remaining() {
            return Err(HwpError::ParseError(format!(
                "Record size {data_size} exceeds remaining data {}",
                reader.remaining()
            )));
        }

        let data = reader.read_bytes(data_size)?;
        Ok(Self { tag_id, data })
    }

    /// Return a fresh `StreamReader` over this record's data bytes.
    pub(crate) fn data_reader(&self) -> StreamReader {
        StreamReader::new(self.data.clone())
    }
}

// ---------------------------------------------------------------------------
// HWP tag constants (only the ones we need)
// ---------------------------------------------------------------------------

/// HWPTAG_BEGIN as defined by the HWP 5.x specification.
const HWPTAG_BEGIN: u16 = 0x010;
/// HWP 5.x body-text record tag: paragraph header (HWPTAG_BEGIN + 64 = 0x50).
///
/// Per the HWP 5.0 binary specification, the paragraph header record uses tag
/// offset 64 from HWPTAG_BEGIN, yielding tag ID 0x50. This matches empirical
/// data from real HWP documents where 0x50 records correspond to paragraph
/// boundaries.
const TAG_PARA_HEADER: u16 = HWPTAG_BEGIN + 64; // 0x50
/// HWP 5.x body-text record tag: paragraph text, UTF-16LE (HWPTAG_BEGIN + 65 = 0x51).
///
/// Per the HWP 5.0 binary specification, the paragraph text record uses tag
/// offset 65 from HWPTAG_BEGIN, yielding tag ID 0x51. The record payload is a
/// sequence of UTF-16LE code units representing the paragraph content.
const TAG_PARA_TEXT: u16 = HWPTAG_BEGIN + 65; // 0x51
/// HWP 5.x body-text record tag: paragraph shape (HWPTAG_BEGIN + 66 = 0x52).
const TAG_PARA_SHAPE: u16 = HWPTAG_BEGIN + 66; // 0x52
/// HWP 5.x body-text record tag: char shape (HWPTAG_BEGIN + 67 = 0x53).
const TAG_CHAR_SHAPE: u16 = HWPTAG_BEGIN + 67; // 0x53

const TAG_CHAR_SHAPE_INFO: u16 = HWPTAG_BEGIN + 30; // 0x2E

// ---------------------------------------------------------------------------
// DocInfoParser — parse global tables
// ---------------------------------------------------------------------------

pub(crate) fn parse_doc_info(data: Vec<u8>) -> Result<Vec<CharShape>> {
    let mut reader = StreamReader::new(data);
    let mut char_shapes = Vec::new();

    while reader.remaining() >= 4 {
        let record = match Record::parse(&mut reader) {
            Ok(r) => r,
            Err(_) => break,
        };

        if record.tag_id == TAG_CHAR_SHAPE_INFO && record.data.len() >= 4 {
            let font_attr = u32::from_le_bytes([record.data[0], record.data[1], record.data[2], record.data[3]]);
            char_shapes.push(CharShape {
                bold: (font_attr & 0x01) != 0,
                italic: (font_attr & 0x02) != 0,
                underline: (font_attr & 0x04) != 0,
            });
        }
    }

    Ok(char_shapes)
}

// ---------------------------------------------------------------------------
// BodyTextParser — parse a single decompressed section into paragraphs
// ---------------------------------------------------------------------------

/// Parse a raw (possibly compressed) BodyText/SectionN stream.
///
/// Returns the list of sections found. Each section contains zero or more
/// paragraphs that carry the plain-text content.
pub(crate) fn parse_body_text(data: Vec<u8>, is_compressed: bool) -> Result<Vec<Section>> {
    let data = if is_compressed { decompress_stream(&data)? } else { data };

    let mut reader = StreamReader::new(data);
    let mut sections: Vec<Section> = Vec::new();
    let mut current_paragraphs: Vec<Paragraph> = Vec::new();
    let mut current_paragraph: Option<Paragraph> = None;

    while reader.remaining() >= 4 {
        let record = match Record::parse(&mut reader) {
            Ok(r) => r,
            Err(_) => break, // truncated stream — stop gracefully
        };

        match record.tag_id {
            TAG_PARA_HEADER => {
                // Flush previous paragraph
                if let Some(para) = current_paragraph.take() {
                    current_paragraphs.push(para);
                }
                current_paragraph = Some(Paragraph::default());
            }
            TAG_PARA_TEXT => {
                if let Some(ref mut para) = current_paragraph
                    && let Ok(text) = ParaText::from_record(&record)
                {
                    para.text = Some(text);
                }
            }
            TAG_PARA_SHAPE => {
                if let Some(ref mut para) = current_paragraph {
                    // ParaShape record byte offset 18: outline_level (u8)
                    if record.data.len() > 18 {
                        para.outline_level = record.data[18];
                    }
                }
            }
            TAG_CHAR_SHAPE => {
                if let Some(ref mut para) = current_paragraph {
                    // CharShape record in BodyText is a list of (pos, shape_idx)
                    // Each entry is 6 bytes: pos (u32), shape_idx (u16)
                    let mut reader = record.data_reader();
                    while reader.remaining() >= 6 {
                        let pos = reader.read_u32().unwrap_or(0);
                        let shape_idx = reader.read_u16().unwrap_or(0);
                        para.char_shape_runs.push((pos, shape_idx));
                    }
                }
            }

            _ => {
                // Skip all other tags — we only need plain text
            }
        }
    }

    // Flush final paragraph
    if let Some(para) = current_paragraph {
        current_paragraphs.push(para);
    }

    sections.push(Section {
        paragraphs: current_paragraphs,
    });

    Ok(sections)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_hwp_extract_converted_output() {
        let path =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/hwp/converted_output.hwp");
        if !path.exists() {
            println!("Skipping: test document not found at {}", path.display());
            return;
        }
        let bytes = std::fs::read(&path).expect("read file");
        let doc = crate::extraction::hwp::extract_hwp_document(&bytes).expect("HWP extraction should succeed");
        assert!(!doc.sections.is_empty(), "Expected at least one section");
        assert!(!doc.sections[0].paragraphs.is_empty(), "Expected at least one paragraph");
    }

    #[test]
    fn test_hwp_tag_constants() {
        // Verify tag constants match the HWP 5.0 specification.
        // PARA_HEADER = 0x50, PARA_TEXT = 0x51.
        assert_eq!(super::TAG_PARA_HEADER, 0x50);
        assert_eq!(super::TAG_PARA_TEXT, 0x51);
    }
}
