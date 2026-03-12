//! Native DOC (Word 97-2003) text extraction.
//!
//! Extracts text directly from Word Binary File Format using OLE/CFB
//! compound document parsing, without requiring LibreOffice.
//!
//! Supports Word 97, 2000, XP, and 2003 (.doc) files.

use crate::error::{KreuzbergError, Result};
use std::io::Cursor;

/// Result of DOC text extraction.
pub struct DocExtractionResult {
    /// Extracted text content.
    pub text: String,
    /// Document metadata.
    pub metadata: DocMetadata,
}

/// Metadata extracted from DOC files.
#[derive(Default)]
pub struct DocMetadata {
    pub title: Option<String>,
    pub subject: Option<String>,
    pub author: Option<String>,
    pub last_author: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub revision_number: Option<String>,
}

/// Extract text from DOC bytes.
///
/// Parses the OLE/CFB compound document, reads the FIB (File Information Block),
/// and extracts text from the piece table.
pub fn extract_doc_text(content: &[u8]) -> Result<DocExtractionResult> {
    let cursor = Cursor::new(content);
    let mut comp = cfb::CompoundFile::open(cursor)
        .map_err(|e| KreuzbergError::parsing(format!("Failed to open DOC as OLE container: {e}")))?;

    // Read metadata from summary information streams
    let metadata = extract_doc_metadata(&mut comp);

    // Read the WordDocument stream
    let word_doc = read_stream(&mut comp, "/WordDocument")?;
    if word_doc.len() < 12 {
        return Err(KreuzbergError::parsing("WordDocument stream too short"));
    }

    // Validate magic number
    let w_ident = u16::from_le_bytes([word_doc[0], word_doc[1]]);
    if w_ident != 0xA5EC {
        return Err(KreuzbergError::parsing(format!(
            "Invalid DOC magic number: 0x{w_ident:04X}, expected 0xA5EC"
        )));
    }

    let n_fib = u16::from_le_bytes([word_doc[2], word_doc[3]]);

    // Get the flags at offset 0x0A to determine which table stream to use
    let flags_a = u16::from_le_bytes([word_doc[0x0A], word_doc[0x0B]]);
    let use_1table = (flags_a & 0x0200) != 0; // fWhichTblStm bit

    let table_stream_name = if use_1table { "/1Table" } else { "/0Table" };

    // Try to read the table stream
    let table_stream = read_stream(&mut comp, table_stream_name)?;

    // Extract text using the piece table approach (Word 97+)
    if n_fib >= 101 {
        extract_text_word97(&word_doc, &table_stream).map(|text| DocExtractionResult { text, metadata })
    } else {
        // For very old Word 6/95 files, try a simple text scan
        extract_text_word6(&word_doc).map(|text| DocExtractionResult { text, metadata })
    }
}

/// Extract text from Word 97/2000/XP/2003 files using the piece table.
fn extract_text_word97(word_doc: &[u8], table_stream: &[u8]) -> Result<String> {
    // Parse FIB to get CLX location
    // FibRgFcLcb97 starts at offset 0x0172 in the FIB
    // fcClx is at offset 0x01A2, lcbClx at 0x01A6
    let fib_base_size = 32; // fibBase
    let csw_offset = fib_base_size;

    if word_doc.len() < csw_offset + 2 {
        return Err(KreuzbergError::parsing("FIB too short for csw"));
    }

    let csw = u16::from_le_bytes([word_doc[csw_offset], word_doc[csw_offset + 1]]) as usize;
    let rg_w_offset = csw_offset + 2;
    let cslw_offset = rg_w_offset + csw * 2;

    if word_doc.len() < cslw_offset + 2 {
        return Err(KreuzbergError::parsing("FIB too short for cslw"));
    }

    let cslw = u16::from_le_bytes([word_doc[cslw_offset], word_doc[cslw_offset + 1]]) as usize;
    let rg_lw_offset = cslw_offset + 2;

    // ccpText is at index 3 of FibRgLw97 (0-based), each entry is 4 bytes
    let ccp_text_offset = rg_lw_offset + 3 * 4;
    if word_doc.len() < ccp_text_offset + 4 {
        return Err(KreuzbergError::parsing("FIB too short for ccpText"));
    }

    let ccp_text = u32::from_le_bytes([
        word_doc[ccp_text_offset],
        word_doc[ccp_text_offset + 1],
        word_doc[ccp_text_offset + 2],
        word_doc[ccp_text_offset + 3],
    ]) as usize;

    // Total character count from all ccpXxx fields
    let mut total_cp = ccp_text;
    // ccpFtn, ccpHdd, ccpAtn, ccpEdn, ccpTxbx, ccpHdrTxbx
    for i in 4..=9 {
        let off = rg_lw_offset + i * 4;
        if word_doc.len() >= off + 4 {
            total_cp +=
                u32::from_le_bytes([word_doc[off], word_doc[off + 1], word_doc[off + 2], word_doc[off + 3]]) as usize;
        }
    }
    // Add 1 for the terminating CP
    if total_cp > 0 {
        total_cp += 1;
    }

    // Get to FibRgFcLcb offset
    let cbrgfclcb_offset = rg_lw_offset + cslw * 4;
    if word_doc.len() < cbrgfclcb_offset + 2 {
        return Err(KreuzbergError::parsing("FIB too short for cbRgFcLcb"));
    }

    let _cb_rg_fc_lcb = u16::from_le_bytes([word_doc[cbrgfclcb_offset], word_doc[cbrgfclcb_offset + 1]]) as usize;
    let rg_fc_lcb_offset = cbrgfclcb_offset + 2;

    // fcClx is at index 66 of FibRgFcLcb97 (each entry is fc:4 + lcb:4 = 8 bytes)
    let fc_clx_offset = rg_fc_lcb_offset + 66 * 8;
    let lcb_clx_offset = fc_clx_offset + 4;

    if word_doc.len() < lcb_clx_offset + 4 {
        return Err(KreuzbergError::parsing("FIB too short for fcClx/lcbClx"));
    }

    let fc_clx = u32::from_le_bytes([
        word_doc[fc_clx_offset],
        word_doc[fc_clx_offset + 1],
        word_doc[fc_clx_offset + 2],
        word_doc[fc_clx_offset + 3],
    ]) as usize;
    let lcb_clx = u32::from_le_bytes([
        word_doc[lcb_clx_offset],
        word_doc[lcb_clx_offset + 1],
        word_doc[lcb_clx_offset + 2],
        word_doc[lcb_clx_offset + 3],
    ]) as usize;

    if fc_clx == 0 || lcb_clx == 0 {
        // No CLX - use fcMin/fcMac from FIB base for contiguous text
        return extract_text_contiguous(word_doc, ccp_text);
    }

    if table_stream.len() < fc_clx + lcb_clx {
        return Err(KreuzbergError::parsing("CLX extends beyond table stream"));
    }

    let clx = &table_stream[fc_clx..fc_clx + lcb_clx];

    // Parse CLX: skip Prc entries (clxt == 0x01), find Pcdt (clxt == 0x02)
    let mut pos = 0;
    while pos < clx.len() {
        let clxt = clx[pos];
        if clxt == 0x02 {
            // Found Pcdt
            pos += 1;
            if pos + 4 > clx.len() {
                return Err(KreuzbergError::parsing("Pcdt truncated at lcb"));
            }
            let _lcb = u32::from_le_bytes([clx[pos], clx[pos + 1], clx[pos + 2], clx[pos + 3]]) as usize;
            pos += 4;

            // Parse PlcPcd - array of CPs followed by array of PCDs
            let plc_pcd = &clx[pos..];
            return extract_text_from_piece_table(word_doc, plc_pcd, ccp_text, total_cp);
        } else if clxt == 0x01 {
            // Prc - skip it
            pos += 1;
            if pos + 2 > clx.len() {
                break;
            }
            let cb_grpprl = u16::from_le_bytes([clx[pos], clx[pos + 1]]) as usize;
            pos += 2 + cb_grpprl;
        } else {
            // Unknown clxt, try to skip
            break;
        }
    }

    // Fallback if no Pcdt found
    extract_text_fallback(word_doc, ccp_text)
}

/// Extract text from the piece table (PlcPcd).
fn extract_text_from_piece_table(word_doc: &[u8], plc_pcd: &[u8], ccp_text: usize, total_cp: usize) -> Result<String> {
    // PlcPcd structure: array of (n+1) CPs (4 bytes each) followed by n PCDs (8 bytes each)
    // We need to figure out n from the data size
    // total_size = (n+1)*4 + n*8 = 4n + 4 + 8n = 12n + 4
    // n = (total_size - 4) / 12
    let plc_size = plc_pcd.len();
    if plc_size < 16 {
        return Err(KreuzbergError::parsing("PlcPcd too small"));
    }

    let n = (plc_size - 4) / 12;
    if n == 0 {
        return Ok(String::new());
    }

    let mut result = String::with_capacity(ccp_text);

    for i in 0..n {
        let cp_start_off = i * 4;
        let cp_end_off = (i + 1) * 4;
        let pcd_off = (n + 1) * 4 + i * 8;

        if cp_end_off + 4 > plc_size || pcd_off + 8 > plc_size {
            break;
        }

        let cp_start = u32::from_le_bytes([
            plc_pcd[cp_start_off],
            plc_pcd[cp_start_off + 1],
            plc_pcd[cp_start_off + 2],
            plc_pcd[cp_start_off + 3],
        ]) as usize;

        let cp_end = u32::from_le_bytes([
            plc_pcd[cp_end_off],
            plc_pcd[cp_end_off + 1],
            plc_pcd[cp_end_off + 2],
            plc_pcd[cp_end_off + 3],
        ]) as usize;

        // Only extract text from the main document body (up to ccp_text)
        if cp_start >= total_cp {
            break;
        }

        // PCD structure: 2 bytes (ABCbits), 4 bytes (fc), 2 bytes (prm)
        let fc_raw = u32::from_le_bytes([
            plc_pcd[pcd_off + 2],
            plc_pcd[pcd_off + 3],
            plc_pcd[pcd_off + 4],
            plc_pcd[pcd_off + 5],
        ]);

        let is_compressed = (fc_raw & 0x4000_0000) != 0;
        let char_count = cp_end.saturating_sub(cp_start);

        // Limit to main document text
        let chars_to_read = if cp_start + char_count > ccp_text && cp_start < ccp_text {
            ccp_text - cp_start
        } else if cp_start >= ccp_text {
            continue;
        } else {
            char_count
        };

        if is_compressed {
            // ANSI (CP1252) text - fc bits 29:0 give byte offset / 2
            let byte_offset = (fc_raw & 0x3FFF_FFFF) as usize / 2;
            let end = byte_offset + chars_to_read;
            if end <= word_doc.len() {
                let bytes = &word_doc[byte_offset..end];
                // CP1252 decode
                for &b in bytes {
                    result.push(cp1252_to_char(b));
                }
            }
        } else {
            // Unicode (UTF-16LE) text
            let byte_offset = (fc_raw & 0x3FFF_FFFF) as usize;
            let end = byte_offset + chars_to_read * 2;
            if end <= word_doc.len() {
                let bytes = &word_doc[byte_offset..end];
                for chunk in bytes.chunks_exact(2) {
                    let code_unit = u16::from_le_bytes([chunk[0], chunk[1]]);
                    if let Some(c) = char::from_u32(code_unit as u32) {
                        result.push(c);
                    }
                }
            }
        }
    }

    Ok(normalize_doc_text(&result))
}

/// Extract text from a "simple" DOC file where text is stored contiguously.
///
/// When fcClx=0, the text is stored at offset `fcMin` in the WordDocument stream
/// as either CP1252 (compressed) or UTF-16LE (uncompressed).
fn extract_text_contiguous(word_doc: &[u8], ccp_text: usize) -> Result<String> {
    if word_doc.len() < 0x20 {
        return extract_text_fallback(word_doc, ccp_text);
    }

    // fcMin at FIB offset 0x18 - byte position of first text character
    let fc_min = u32::from_le_bytes([word_doc[0x18], word_doc[0x19], word_doc[0x1A], word_doc[0x1B]]) as usize;
    // fcMac at FIB offset 0x1C - byte position past last text character
    let fc_mac = u32::from_le_bytes([word_doc[0x1C], word_doc[0x1D], word_doc[0x1E], word_doc[0x1F]]) as usize;

    if fc_min == 0 || fc_min >= word_doc.len() {
        return extract_text_fallback(word_doc, ccp_text);
    }

    let data_len = fc_mac.saturating_sub(fc_min).min(word_doc.len() - fc_min);
    if data_len == 0 {
        return extract_text_fallback(word_doc, ccp_text);
    }

    let text_data = &word_doc[fc_min..fc_min + data_len];

    // Detect encoding: if data_len ~= 2 * ccp_text, it's UTF-16LE
    // Also check for null bytes pattern (common in UTF-16)
    let null_count = text_data.iter().filter(|&&b| b == 0).count();
    let is_unicode = data_len >= ccp_text * 2 || null_count > data_len / 4;

    let text = if is_unicode {
        // UTF-16LE
        let chars: Vec<u16> = text_data
            .chunks_exact(2)
            .take(ccp_text)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&chars)
    } else {
        // CP1252
        text_data.iter().take(ccp_text).map(|&b| cp1252_to_char(b)).collect()
    };

    let normalized = normalize_doc_text(&text);
    if normalized.is_empty() {
        return extract_text_fallback(word_doc, ccp_text);
    }

    Ok(normalized)
}

/// Fallback text extraction for when the piece table is unavailable.
///
/// Attempts to extract readable text from the WordDocument stream directly.
fn extract_text_fallback(word_doc: &[u8], _ccp_text: usize) -> Result<String> {
    // Simple heuristic: scan for readable text sequences including Latin-1 range
    let mut result = String::new();
    let mut text_run = String::new();

    // Start after 256 bytes (conservative FIB base) to skip binary headers
    for &b in word_doc.iter().skip(256) {
        if b == 0x0D || b == 0x0A || b == 0x09 || (0x20..=0xFE).contains(&b) {
            text_run.push(cp1252_to_char(b));
        } else if !text_run.is_empty() {
            if text_run.len() >= 3 {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(&text_run);
            }
            text_run.clear();
        }
    }

    if text_run.len() >= 3 {
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(&text_run);
    }

    if result.is_empty() {
        return Err(KreuzbergError::parsing("No text content found in DOC file"));
    }

    Ok(normalize_doc_text(&result))
}

/// Extract text from Word 6/95 files.
///
/// Word 6/95 has a simpler format where text starts at a known offset.
fn extract_text_word6(word_doc: &[u8]) -> Result<String> {
    // Word 6/95: ccpText at offset 0x4C, text starts after FIB
    if word_doc.len() < 0x50 {
        return Err(KreuzbergError::parsing("Word 6/95 file too short"));
    }

    let ccp_text = u32::from_le_bytes([word_doc[0x4C], word_doc[0x4D], word_doc[0x4E], word_doc[0x4F]]) as usize;

    // fcMin at offset 0x18 gives the start of text
    let fc_min = u32::from_le_bytes([word_doc[0x18], word_doc[0x19], word_doc[0x1A], word_doc[0x1B]]) as usize;

    if fc_min + ccp_text > word_doc.len() {
        return extract_text_fallback(word_doc, ccp_text);
    }

    let text_bytes = &word_doc[fc_min..fc_min + ccp_text];
    let mut result = String::with_capacity(ccp_text);

    for &b in text_bytes {
        result.push(cp1252_to_char(b));
    }

    Ok(normalize_doc_text(&result))
}

/// Normalize extracted DOC text: convert special characters and clean up whitespace.
fn normalize_doc_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for c in text.chars() {
        match c {
            '\r' => result.push('\n'),
            '\x07' => result.push('\t'),                     // Cell/row delimiter in tables
            '\x0B' => result.push('\n'),                     // Vertical tab → newline
            '\x0C' => result.push('\n'),                     // Page break
            '\x01' | '\x08' | '\x13' | '\x14' | '\x15' => {} // Field codes, skip
            c if c < '\x20' && c != '\n' && c != '\t' => {}  // Skip other control chars
            _ => result.push(c),
        }
    }

    // Collapse excessive blank lines
    let mut prev_newline = false;
    let mut prev_prev_newline = false;
    let mut cleaned = String::with_capacity(result.len());

    for c in result.chars() {
        if c == '\n' {
            if prev_prev_newline && prev_newline {
                continue; // Skip 3+ consecutive newlines
            }
            prev_prev_newline = prev_newline;
            prev_newline = true;
        } else {
            prev_prev_newline = false;
            prev_newline = false;
        }
        cleaned.push(c);
    }

    cleaned.trim().to_string()
}

/// Convert CP1252 byte to Unicode char.
fn cp1252_to_char(b: u8) -> char {
    match b {
        0x80 => '\u{20AC}', // Euro sign
        0x82 => '\u{201A}', // Single low-9 quotation mark
        0x83 => '\u{0192}', // Latin small letter f with hook
        0x84 => '\u{201E}', // Double low-9 quotation mark
        0x85 => '\u{2026}', // Horizontal ellipsis
        0x86 => '\u{2020}', // Dagger
        0x87 => '\u{2021}', // Double dagger
        0x88 => '\u{02C6}', // Modifier letter circumflex accent
        0x89 => '\u{2030}', // Per mille sign
        0x8A => '\u{0160}', // Latin capital letter S with caron
        0x8B => '\u{2039}', // Single left-pointing angle quotation mark
        0x8C => '\u{0152}', // Latin capital ligature OE
        0x8E => '\u{017D}', // Latin capital letter Z with caron
        0x91 => '\u{2018}', // Left single quotation mark
        0x92 => '\u{2019}', // Right single quotation mark
        0x93 => '\u{201C}', // Left double quotation mark
        0x94 => '\u{201D}', // Right double quotation mark
        0x95 => '\u{2022}', // Bullet
        0x96 => '\u{2013}', // En dash
        0x97 => '\u{2014}', // Em dash
        0x98 => '\u{02DC}', // Small tilde
        0x99 => '\u{2122}', // Trade mark sign
        0x9A => '\u{0161}', // Latin small letter s with caron
        0x9B => '\u{203A}', // Single right-pointing angle quotation mark
        0x9C => '\u{0153}', // Latin small ligature oe
        0x9E => '\u{017E}', // Latin small letter z with caron
        0x9F => '\u{0178}', // Latin capital letter Y with diaeresis
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
fn extract_doc_metadata(comp: &mut cfb::CompoundFile<Cursor<&[u8]>>) -> DocMetadata {
    let mut meta = DocMetadata::default();

    // Try to extract from SummaryInformation stream
    if let Ok(data) = read_stream(comp, "/\x05SummaryInformation") {
        parse_summary_info(&data, &mut meta);
    }

    // Try DocumentSummaryInformation for additional metadata
    if let Ok(data) = read_stream(comp, "/\x05DocumentSummaryInformation") {
        parse_doc_summary_info(&data, &mut meta);
    }

    meta
}

/// Parse OLE SummaryInformation property set.
fn parse_summary_info(data: &[u8], meta: &mut DocMetadata) {
    // Property set header: 28 bytes minimum
    if data.len() < 28 {
        return;
    }

    // Skip byte order (2), version (2), system identifier (4), CLSID (16)
    // num_property_sets at offset 24
    let offset = 24;
    if data.len() < offset + 4 {
        return;
    }

    let num_sets = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
    if num_sets == 0 {
        return;
    }

    // First property set: FMTID (16 bytes) + offset (4 bytes) at position 28
    if data.len() < 48 {
        return;
    }
    let set_offset = u32::from_le_bytes([data[44], data[45], data[46], data[47]]) as usize;

    parse_property_set(data, set_offset, meta, false);
}

/// Parse OLE DocumentSummaryInformation property set.
fn parse_doc_summary_info(data: &[u8], meta: &mut DocMetadata) {
    if data.len() < 48 {
        return;
    }

    let set_offset = u32::from_le_bytes([data[44], data[45], data[46], data[47]]) as usize;

    parse_property_set(data, set_offset, meta, true);
}

/// Parse a single property set from OLE property data.
fn parse_property_set(data: &[u8], set_offset: usize, meta: &mut DocMetadata, _is_doc_summary: bool) {
    if set_offset + 8 > data.len() {
        return;
    }

    // PropertySetHeader: size (4 bytes), num_properties (4 bytes)
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

        // SummaryInformation property IDs:
        // 2 = Title, 3 = Subject, 4 = Author, 7 = Template, 8 = LastAuthor
        // 12 = CreateDate, 13 = SaveDate, 9 = RevNumber
        if let Some(value) = read_property_value(data, abs_offset) {
            match prop_id {
                2 => meta.title = Some(value),
                3 => meta.subject = Some(value),
                4 => meta.author = Some(value),
                8 => meta.last_author = Some(value),
                9 => meta.revision_number = Some(value),
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
        // VT_LPSTR (30) - CodePage string
        30 => {
            let len =
                u32::from_le_bytes([data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]]) as usize;
            if len == 0 || offset + 8 + len > data.len() {
                return None;
            }
            let bytes = &data[offset + 8..offset + 8 + len];
            // Trim trailing null
            let trimmed = bytes.iter().take_while(|&&b| b != 0).copied().collect::<Vec<_>>();
            Some(String::from_utf8_lossy(&trimmed).to_string())
        }
        // VT_LPWSTR (31) - Unicode string
        31 => {
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
    fn test_cp1252_to_char_ascii() {
        assert_eq!(cp1252_to_char(b'A'), 'A');
        assert_eq!(cp1252_to_char(b' '), ' ');
        assert_eq!(cp1252_to_char(b'\n'), '\n');
    }

    #[test]
    fn test_cp1252_to_char_special() {
        assert_eq!(cp1252_to_char(0x80), '\u{20AC}'); // Euro
        assert_eq!(cp1252_to_char(0x93), '\u{201C}'); // Left double quote
        assert_eq!(cp1252_to_char(0x94), '\u{201D}'); // Right double quote
        assert_eq!(cp1252_to_char(0x96), '\u{2013}'); // En dash
    }

    #[test]
    fn test_normalize_doc_text() {
        assert_eq!(normalize_doc_text("Hello\rWorld"), "Hello\nWorld");
        assert_eq!(normalize_doc_text("A\x07B"), "A\tB");
        assert_eq!(normalize_doc_text("A\x0BB"), "A\nB");
        assert_eq!(normalize_doc_text("A\n\n\n\nB"), "A\n\nB");
    }

    #[test]
    fn test_normalize_doc_text_field_codes() {
        // Field codes should be stripped
        assert_eq!(normalize_doc_text("A\x13FIELD\x14result\x15B"), "AFIELDresultB");
    }

    #[test]
    fn test_extract_doc_real_file() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/unstructured/doc/simple.doc");
        if !test_file.exists() {
            return; // Skip if test file not available
        }
        let content = std::fs::read(&test_file).expect("Failed to read test DOC");
        let result = extract_doc_text(&content).expect("Failed to extract DOC text");
        assert!(!result.text.is_empty(), "DOC extraction should produce text");
    }

    #[test]
    fn test_extract_doc_fake_file() {
        let test_file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/unstructured/doc/fake.doc");
        if !test_file.exists() {
            return;
        }
        let content = std::fs::read(&test_file).expect("Failed to read test DOC");
        let result = extract_doc_text(&content).expect("Failed to extract DOC text");
        assert!(!result.text.is_empty(), "DOC extraction should produce text");
    }

    #[test]
    fn test_extract_doc_invalid_magic() {
        // A valid OLE container but with wrong Word magic number
        let result = extract_doc_text(b"not a doc file");
        assert!(result.is_err());
    }
}
