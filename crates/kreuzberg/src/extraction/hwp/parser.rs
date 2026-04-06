/// HWP record, file-header, and body-text parsers.
///
/// Consolidated from hwpers parser/record.rs, parser/header.rs, and
/// parser/body_text.rs.
use super::error::{HwpError, Result};
use super::model::{ParaText, Paragraph, Section};
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
    pub fn parse(data: Vec<u8>) -> Result<Self> {
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
    pub fn is_compressed(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// Whether the document is password-encrypted.
    pub fn is_encrypted(&self) -> bool {
        (self.flags & 0x02) != 0
    }

    /// Whether the document is a distribution document (text in ViewText/).
    pub fn is_distribute(&self) -> bool {
        (self.flags & 0x04) != 0
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
    pub fn parse(reader: &mut StreamReader) -> Result<Self> {
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
    pub fn data_reader(&self) -> StreamReader {
        StreamReader::new(self.data.clone())
    }
}

// ---------------------------------------------------------------------------
// HWP tag constants (only the ones we need)
// ---------------------------------------------------------------------------

/// HWPTAG_BEGIN as defined by the HWP 5.x specification.
const HWPTAG_BEGIN: u16 = 0x010;
/// HWP 5.x body-text record tag: paragraph header (HWPTAG_BEGIN + 50).
const TAG_PARA_HEADER: u16 = HWPTAG_BEGIN + 50; // 0x42
/// HWP 5.x body-text record tag: paragraph text, UTF-16LE (HWPTAG_BEGIN + 51).
const TAG_PARA_TEXT: u16 = HWPTAG_BEGIN + 51; // 0x43

// ---------------------------------------------------------------------------
// BodyTextParser — parse a single decompressed section into paragraphs
// ---------------------------------------------------------------------------

/// Parse a raw (possibly compressed) BodyText/SectionN stream.
///
/// Returns the list of sections found. Each section contains zero or more
/// paragraphs that carry the plain-text content.
pub fn parse_body_text(data: Vec<u8>, is_compressed: bool) -> Result<Vec<Section>> {
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
