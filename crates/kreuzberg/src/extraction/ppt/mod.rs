//! Native PPT (PowerPoint 97-2003) text extraction.
//!
//! Extracts text directly from PowerPoint Binary File Format using OLE/CFB
//! compound document parsing, without requiring LibreOffice.
//!
//! Supports PowerPoint 97, 2000, XP, and 2003 (.ppt) files.

use crate::error::{KreuzbergError, Result};
use std::io::Cursor;

/// Result of PPT text extraction.
pub struct PptExtractionResult {
    /// Extracted text content, with slides separated by double newlines.
    pub text: String,
    /// Number of slides found.
    pub slide_count: usize,
    /// Document metadata.
    pub metadata: PptMetadata,
    /// Speaker notes text per slide (if available).
    pub speaker_notes: Vec<String>,
}

/// Metadata extracted from PPT files.
#[derive(Default)]
pub struct PptMetadata {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub author: Option<String>,
    pub last_author: Option<String>,
}

// PowerPoint record types for text extraction
const RT_TEXT_CHARS_ATOM: u16 = 0x0FA0; // Unicode (UTF-16LE) text
const RT_TEXT_BYTES_ATOM: u16 = 0x0FA8; // ANSI (CP1252) text
const RT_SLIDE_LIST_WITH_TEXT: u16 = 0x0FF0; // Container for slide text
const RT_MAIN_MASTER: u16 = 0x03F8; // Main master slide
const RT_NOTES: u16 = 0x03F0; // Notes container

/// Extract text from PPT bytes.
///
/// Parses the OLE/CFB compound document, reads the "PowerPoint Document" stream,
/// and extracts text from TextCharsAtom and TextBytesAtom records.
///
/// When `include_master_slides` is `true`, master slide content (placeholder text
/// like "Click to edit Master title style") is included instead of being skipped.
#[cfg(test)]
pub(crate) fn extract_ppt_text(content: &[u8]) -> Result<PptExtractionResult> {
    extract_ppt_text_with_options(content, false)
}

/// Extract text from PPT bytes with configurable master slide inclusion.
///
/// When `include_master_slides` is `true`, `RT_MAIN_MASTER` containers are not
/// skipped, so master slide placeholder text is included in the output.
pub(crate) fn extract_ppt_text_with_options(
    content: &[u8],
    include_master_slides: bool,
) -> Result<PptExtractionResult> {
    let cursor = Cursor::new(content);
    let mut comp = cfb::CompoundFile::open(cursor)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to open PPT as OLE container: {e}")))?;

    // Extract metadata from summary information
    let metadata = extract_ppt_metadata(&mut comp);

    // Read the PowerPoint Document stream
    let ppt_stream = read_stream(&mut comp, "/PowerPoint Document")?;
    if ppt_stream.is_empty() {
        return Err(KreuzbergError::parsing("PowerPoint Document stream is empty"));
    }

    // Extract text from the stream
    let (texts, slide_count, speaker_notes) = extract_texts_from_records(&ppt_stream, include_master_slides)?;

    let text = texts
        .into_iter()
        .filter(|t| !t.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(PptExtractionResult {
        text: text.trim().to_string(),
        slide_count,
        metadata,
        speaker_notes,
    })
}

/// Parse PowerPoint record headers and extract text atoms.
///
/// Returns `(slide_texts, slide_count, speaker_notes)`.
///
/// When `include_master_slides` is `true`, master slide containers are not
/// skipped, allowing their placeholder text to appear in the output.
fn extract_texts_from_records(data: &[u8], include_master_slides: bool) -> Result<(Vec<String>, usize, Vec<String>)> {
    let mut texts = Vec::new();
    let mut slide_count = 0;
    let mut pos = 0;
    let mut in_slide_text = false;
    let mut current_slide_texts: Vec<String> = Vec::new();
    let mut speaker_notes = Vec::new();
    let mut in_notes = false;
    let mut current_notes_texts: Vec<String> = Vec::new();

    while pos + 8 <= data.len() {
        // Record header: 8 bytes
        // Bytes 0-1: recVer (4 bits) + recInstance (12 bits)
        // Bytes 2-3: recType (16 bits)
        // Bytes 4-7: recLen (32 bits)
        let rec_ver_instance = u16::from_le_bytes([data[pos], data[pos + 1]]);
        let rec_ver = rec_ver_instance & 0x000F;
        let rec_type = u16::from_le_bytes([data[pos + 2], data[pos + 3]]);
        let rec_len = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;

        // Prevent infinite loops on invalid data
        if rec_len > data.len() - pos {
            break;
        }

        let is_container = rec_ver == 0x0F;
        let content_start = pos + 8;
        let content_end = content_start + rec_len;

        match rec_type {
            RT_SLIDE_LIST_WITH_TEXT => {
                // Start tracking slide text
                if in_slide_text && !current_slide_texts.is_empty() {
                    texts.push(current_slide_texts.join("\n"));
                    current_slide_texts.clear();
                }
                in_slide_text = true;
                slide_count += 1;
                // Recurse into container
                pos += 8;
                continue;
            }
            RT_NOTES => {
                // Notes container -- track it
                if in_notes && !current_notes_texts.is_empty() {
                    let notes_text = current_notes_texts.join("\n");
                    let trimmed = notes_text.trim().to_string();
                    if !trimmed.is_empty() {
                        speaker_notes.push(trimmed);
                    }
                    current_notes_texts.clear();
                }
                in_notes = true;
                pos += 8;
                continue;
            }
            RT_MAIN_MASTER if !include_master_slides => {
                // Skip entire master slide container to avoid extracting
                // placeholder text like "Click to edit Master title style"
                pos = content_end;
                continue;
            }
            RT_TEXT_CHARS_ATOM => {
                // Unicode (UTF-16LE) text
                if content_end <= data.len() {
                    let text_data = &data[content_start..content_end];
                    let chars: Vec<u16> = text_data
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    let text = String::from_utf16_lossy(&chars);
                    let cleaned = clean_ppt_text(&text);
                    if !cleaned.is_empty() {
                        if in_notes {
                            current_notes_texts.push(cleaned.clone());
                        }
                        if in_slide_text {
                            current_slide_texts.push(cleaned);
                        } else if !in_notes {
                            texts.push(cleaned);
                        }
                    }
                }
                pos = content_end;
                continue;
            }
            RT_TEXT_BYTES_ATOM => {
                // ANSI (CP1252) text
                if content_end <= data.len() {
                    let text_data = &data[content_start..content_end];
                    let text: String = text_data.iter().map(|&b| cp1252_to_char(b)).collect();
                    let cleaned = clean_ppt_text(&text);
                    if !cleaned.is_empty() {
                        if in_notes {
                            current_notes_texts.push(cleaned.clone());
                        }
                        if in_slide_text {
                            current_slide_texts.push(cleaned);
                        } else if !in_notes {
                            texts.push(cleaned);
                        }
                    }
                }
                pos = content_end;
                continue;
            }
            _ => {}
        }

        if is_container {
            // Step into container records to find nested text atoms
            pos += 8;
        } else {
            // Skip non-container records
            pos = content_end;
        }
    }

    // Flush any remaining slide text
    if !current_slide_texts.is_empty() {
        texts.push(current_slide_texts.join("\n"));
    }

    // Flush any remaining notes
    if !current_notes_texts.is_empty() {
        let notes_text = current_notes_texts.join("\n");
        let trimmed = notes_text.trim().to_string();
        if !trimmed.is_empty() {
            speaker_notes.push(trimmed);
        }
    }

    // If no SlideListWithText containers found but we have text, count it
    if slide_count == 0 && !texts.is_empty() {
        slide_count = 1;
    }

    Ok((texts, slide_count, speaker_notes))
}

/// Clean PPT text: replace control characters and normalize whitespace.
fn clean_ppt_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '\r' => result.push('\n'),
            '\x0B' => result.push('\n'),                    // Vertical tab
            c if c < '\x20' && c != '\n' && c != '\t' => {} // Skip control chars
            _ => result.push(c),
        }
    }

    // Trim trailing whitespace from each line
    let cleaned = result
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n");

    // Filter out placeholder bullet text from slide masters/layouts.
    // These appear as lines containing only bullet-like characters (*, -, etc.)
    let trimmed = cleaned.trim();
    if trimmed.chars().all(|c| c == '*' || c == '\n' || c.is_whitespace()) {
        return String::new();
    }

    cleaned
}

/// Convert CP1252 byte to Unicode char.
fn cp1252_to_char(b: u8) -> char {
    match b {
        0x80 => '\u{20AC}',
        0x82 => '\u{201A}',
        0x83 => '\u{0192}',
        0x84 => '\u{201E}',
        0x85 => '\u{2026}',
        0x86 => '\u{2020}',
        0x87 => '\u{2021}',
        0x88 => '\u{02C6}',
        0x89 => '\u{2030}',
        0x8A => '\u{0160}',
        0x8B => '\u{2039}',
        0x8C => '\u{0152}',
        0x8E => '\u{017D}',
        0x91 => '\u{2018}',
        0x92 => '\u{2019}',
        0x93 => '\u{201C}',
        0x94 => '\u{201D}',
        0x95 => '\u{2022}',
        0x96 => '\u{2013}',
        0x97 => '\u{2014}',
        0x98 => '\u{02DC}',
        0x99 => '\u{2122}',
        0x9A => '\u{0161}',
        0x9B => '\u{203A}',
        0x9C => '\u{0153}',
        0x9E => '\u{017E}',
        0x9F => '\u{0178}',
        b => b as char,
    }
}

/// Read a named stream from the CFB compound file.
fn read_stream(comp: &mut cfb::CompoundFile<Cursor<&[u8]>>, name: &str) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut stream = comp
        .open_stream(name)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to open stream '{name}': {e}")))?;
    let mut data = Vec::new();
    stream
        .read_to_end(&mut data)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to read stream '{name}': {e}")))?;
    Ok(data)
}

/// Extract metadata from OLE summary information streams.
fn extract_ppt_metadata(comp: &mut cfb::CompoundFile<Cursor<&[u8]>>) -> PptMetadata {
    let mut meta = PptMetadata::default();

    if let Ok(data) = read_stream(comp, "/\x05SummaryInformation") {
        parse_summary_info(&data, &mut meta);
    }

    meta
}

/// Parse OLE SummaryInformation for PPT metadata.
fn parse_summary_info(data: &[u8], meta: &mut PptMetadata) {
    if data.len() < 48 {
        return;
    }

    let set_offset = u32::from_le_bytes([data[44], data[45], data[46], data[47]]) as usize;

    if set_offset + 8 > data.len() {
        return;
    }

    let num_props = u32::from_le_bytes([
        data[set_offset + 4],
        data[set_offset + 5],
        data[set_offset + 6],
        data[set_offset + 7],
    ]) as usize;

    let props_start = set_offset + 8;

    for i in 0..num_props {
        let entry_offset = props_start + i * 8;
        if entry_offset + 8 > data.len() {
            break;
        }

        let prop_id = u32::from_le_bytes([
            data[entry_offset],
            data[entry_offset + 1],
            data[entry_offset + 2],
            data[entry_offset + 3],
        ]);
        let prop_offset = u32::from_le_bytes([
            data[entry_offset + 4],
            data[entry_offset + 5],
            data[entry_offset + 6],
            data[entry_offset + 7],
        ]) as usize;

        let abs_offset = set_offset + prop_offset;
        if abs_offset + 8 > data.len() {
            continue;
        }

        if let Some(value) = read_property_value(data, abs_offset) {
            match prop_id {
                2 => meta.title = Some(value),
                3 => meta.subject = Some(value),
                4 => meta.author = Some(value),
                8 => meta.last_author = Some(value),
                _ => {}
            }
        }
    }
}

/// Read a property value from an OLE property entry.
fn read_property_value(data: &[u8], offset: usize) -> Option<String> {
    if offset + 8 > data.len() {
        return None;
    }

    let vt_type = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);

    match vt_type {
        30 => {
            // VT_LPSTR
            let len =
                u32::from_le_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]) as usize;
            if len == 0 || offset + 8 + len > data.len() {
                return None;
            }
            let bytes = &data[offset + 8..offset + 8 + len];
            let trimmed = bytes.iter().take_while(|&&b| b != 0).copied().collect::<Vec<_>>();
            Some(String::from_utf8_lossy(&trimmed).to_string())
        }
        31 => {
            // VT_LPWSTR
            let len =
                u32::from_le_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]) as usize;
            if len == 0 || offset + 8 + len * 2 > data.len() {
                return None;
            }
            let bytes = &data[offset + 8..offset + 8 + len * 2];
            let chars: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .take_while(|&c| c != 0)
                .collect();
            Some(String::from_utf16_lossy(&chars))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_ppt_text() {
        assert_eq!(clean_ppt_text("Hello\rWorld"), "Hello\nWorld");
        assert_eq!(clean_ppt_text("A\x0BB"), "A\nB");
    }

    #[test]
    fn test_cp1252_to_char() {
        assert_eq!(cp1252_to_char(b'A'), 'A');
        assert_eq!(cp1252_to_char(0x80), '\u{20AC}');
    }

    #[test]
    fn test_extract_ppt_real_file() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/ppt/simple.ppt");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test PPT");
        let result = extract_ppt_text(&content).expect("Failed to extract PPT text");
        assert!(!result.text.is_empty(), "PPT extraction should produce text");
    }

    #[test]
    fn test_extract_ppt_invalid_data() {
        let result = extract_ppt_text(b"not a ppt file");
        assert!(result.is_err());
    }
}
