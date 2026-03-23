/// Minimal document model for HWP text extraction.
///
/// Only the types needed to walk body-text sections and collect plain text.
use super::error::Result;
use super::parser::Record;

// ---------------------------------------------------------------------------
// Document model
// ---------------------------------------------------------------------------

/// An extracted HWP document, consisting of one or more body-text sections.
#[derive(Debug, Default)]
pub struct HwpDocument {
    /// All sections from all BodyText/SectionN streams.
    pub sections: Vec<Section>,
}

impl HwpDocument {
    /// Concatenate the text of every paragraph in every section, separated by
    /// newlines.
    pub fn extract_text(&self) -> String {
        let mut out = String::new();
        for section in &self.sections {
            for para in &section.paragraphs {
                if let Some(ref t) = para.text
                    && !t.content.is_empty()
                {
                    out.push_str(&t.content);
                    out.push('\n');
                }
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Section
// ---------------------------------------------------------------------------

/// A body-text section containing a flat list of paragraphs.
#[derive(Debug, Default)]
pub struct Section {
    pub paragraphs: Vec<Paragraph>,
}

// ---------------------------------------------------------------------------
// Paragraph
// ---------------------------------------------------------------------------

/// A single paragraph; may or may not carry a text payload.
#[derive(Debug, Default)]
pub struct Paragraph {
    pub text: Option<ParaText>,
}

// ---------------------------------------------------------------------------
// ParaText — decodes a TAG_PARA_TEXT (0x51) record
// ---------------------------------------------------------------------------

/// Plain text content decoded from a ParaText record (tag 0x51).
#[derive(Debug)]
pub struct ParaText {
    pub content: String,
}

impl ParaText {
    /// Decode a ParaText record from raw bytes.
    ///
    /// The data field of a TAG_PARA_TEXT record is a sequence of UTF-16LE code
    /// units.  Control characters < 0x0020 are mapped to whitespace or skipped;
    /// characters in the private-use range 0xF020–0xF07F (HWP internal controls)
    /// are discarded.
    pub fn from_record(record: &Record) -> Result<Self> {
        let mut reader = record.data_reader();
        let mut chars: Vec<u16> = Vec::with_capacity(record.data.len() / 2);

        while reader.remaining() >= 2 {
            chars.push(reader.read_u16()?);
        }

        let mut content = String::with_capacity(chars.len());
        let mut i = 0;
        while i < chars.len() {
            let ch = chars[i];
            match ch {
                0x0000 => {} // null — skip
                // Extended control characters 0x0001–0x0008 occupy 8 u16 units
                // total (the control char + 7 parameter units). Skip all 8.
                0x0001..=0x0008 => {
                    i += 7; // skip 7 additional parameter units (loop increments by 1)
                }
                0x0009 => content.push('\t'),           // horizontal tab
                0x000A => content.push('\n'),           // line feed
                0x000D => content.push('\r'),           // carriage return
                0x000B..=0x000C | 0x000E..=0x001F => {} // skip remaining C0 controls
                0xF020..=0xF07F => {}                   // HWP private-use controls — skip
                _ => {
                    if let Some(c) = char::from_u32(ch as u32) {
                        content.push(c);
                    }
                }
            }
            i += 1;
        }

        Ok(Self { content })
    }
}
