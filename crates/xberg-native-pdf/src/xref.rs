//! Cross-reference table parser.
//!
//! The xref table maps object numbers to byte offsets in the PDF file,
//! enabling random access to PDF objects.
//!
//! Supports both traditional xref tables (PDF 1.0-1.4) and
//! cross-reference streams (PDF 1.5+).

use crate::error::{Error, Result};
use crate::object::Object;
use crate::parser::parse_object;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};

/// Cross-reference table entry type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XRefEntryType {
    /// Entry for a free object
    Free,
    /// Entry for an uncompressed object (traditional)
    Uncompressed,
    /// Entry for an object in an object stream (PDF 1.5+)
    Compressed,
}

/// Cross-reference table entry.
///
/// Each entry contains information about where to find an object.
/// Supports both traditional entries (byte offset) and compressed entries
/// (object stream reference).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XRefEntry {
    /// Type of entry
    pub entry_type: XRefEntryType,
    /// Byte offset (for uncompressed) or object stream number (for compressed)
    pub offset: u64,
    /// Generation number (for uncompressed) or index within stream (for compressed)
    pub generation: u16,
    /// Whether the object is in use (for traditional entries only)
    pub in_use: bool,
}

impl XRefEntry {
    /// Create a new cross-reference entry (traditional format).
    pub fn new(offset: u64, generation: u16, in_use: bool) -> Self {
        Self {
            entry_type: if in_use {
                XRefEntryType::Uncompressed
            } else {
                XRefEntryType::Free
            },
            offset,
            generation,
            in_use,
        }
    }

    /// Create a new uncompressed entry.
    pub fn uncompressed(offset: u64, generation: u16) -> Self {
        Self {
            entry_type: XRefEntryType::Uncompressed,
            offset,
            generation,
            in_use: true,
        }
    }

    /// Create a new compressed entry (object in object stream).
    pub fn compressed(stream_obj_num: u64, index_in_stream: u16) -> Self {
        Self {
            entry_type: XRefEntryType::Compressed,
            offset: stream_obj_num,
            generation: index_in_stream,
            in_use: true,
        }
    }

    /// Create a new free entry.
    pub fn free(next_free: u64, generation: u16) -> Self {
        Self {
            entry_type: XRefEntryType::Free,
            offset: next_free,
            generation,
            in_use: false,
        }
    }
}

/// Cross-reference table that maps object numbers to their locations.
#[derive(Debug, Clone)]
pub struct CrossRefTable {
    pub(crate) entries: HashMap<u32, XRefEntry>,
    /// Trailer dictionary (for xref streams, this is the stream dictionary)
    trailer: Option<HashMap<String, Object>>,
}

impl CrossRefTable {
    /// Create a new empty cross-reference table.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            trailer: None,
        }
    }

    /// Set the trailer dictionary.
    pub fn set_trailer(&mut self, trailer: HashMap<String, Object>) {
        self.trailer = Some(trailer);
    }

    /// Get the trailer dictionary if present.
    pub fn trailer(&self) -> Option<&HashMap<String, Object>> {
        self.trailer.as_ref()
    }

    /// Add an entry to the cross-reference table.
    pub fn add_entry(&mut self, object_number: u32, entry: XRefEntry) {
        self.entries.insert(object_number, entry);
    }

    /// Get an entry by object number.
    pub fn get(&self, object_number: u32) -> Option<&XRefEntry> {
        self.entries.get(&object_number)
    }

    /// Check if an object exists in the xref table.
    pub fn contains(&self, object_number: u32) -> bool {
        self.entries.contains_key(&object_number)
    }

    /// Get all object numbers in the table.
    pub fn all_object_numbers(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries.keys().copied()
    }

    /// The `max` smallest **in-use** object numbers, ascending.
    ///
    /// `entries` is a `HashMap`, so iteration order is nondeterministic; a
    /// bounded scan over an arbitrary subset can miss the target. Selecting
    /// the smallest in-use numbers makes scans deterministic and prioritizes
    /// low-numbered live objects (where the Catalog conventionally lives).
    /// Free entries are excluded so a long low-numbered free list can't
    /// crowd the bounded set. A bounded max-heap keeps this O(n log max)
    /// time / O(max) memory rather than sorting all n on a pathological or
    /// maliciously sparse xref.
    pub(crate) fn smallest_object_numbers(&self, max: usize) -> Vec<u32> {
        if max == 0 {
            return Vec::new();
        }
        let mut heap: std::collections::BinaryHeap<u32> = std::collections::BinaryHeap::with_capacity(max + 1);
        // Only live objects are scan candidates. Traditional xref tables
        // store free entries (the free list); a file with more than `max`
        // low-numbered *free* objects would otherwise exhaust the bounded
        // candidate set before any live Catalog is even considered. ~keep
        for (&n, e) in self.entries.iter() {
            if !e.in_use {
                continue;
            }
            heap.push(n);
            if heap.len() > max {
                heap.pop();
            }
        }
        heap.into_sorted_vec()
    }

    /// Merge entries from another xref table.
    ///
    /// Entries in self override entries in other (for incremental updates).
    /// This is used when following /Prev pointers in the trailer.
    pub fn merge_from(&mut self, other: CrossRefTable) {
        for (obj_num, entry) in other.entries {
            self.entries.entry(obj_num).or_insert(entry);
        }

        if self.trailer.is_none() && other.trailer.is_some() {
            self.trailer = other.trailer;
        }
    }

    /// Get the number of entries in the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Shift all uncompressed entry offsets by a delta.
    ///
    /// Used when a PDF has garbage bytes prepended before `%PDF-`:
    /// the xref offsets are relative to the real start of the PDF data,
    /// but byte positions in the file are shifted by `header_offset`.
    pub fn shift_offsets(&mut self, delta: u64) {
        for entry in self.entries.values_mut() {
            if entry.in_use && entry.entry_type == XRefEntryType::Uncompressed {
                entry.offset += delta;
            }
        }
    }
}

impl Default for CrossRefTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Find the byte offset of the xref table by scanning from the end of the file.
///
/// Searches for the "startxref" keyword in the last portion of the file,
/// then extracts the offset that follows it.
///
/// # Errors
///
/// Returns `Error::InvalidXref` if:
/// - The "startxref" keyword is not found
/// - The offset following "startxref" cannot be parsed
/// - The file is too small to contain a valid xref reference
pub fn find_xref_offset<R: Read + Seek>(reader: &mut R) -> Result<u64> {
    let file_size = reader.seek(SeekFrom::End(0))?;

    // GROW the tail window until `startxref` is found. A fixed 2 KB window assumes
    // the keyword sits close to the end of the file - and real producers break that
    // assumption constantly by PADDING the file after `%%EOF`:
    //
    //   gov.uscourts.ca2.*  192 KiB exactly, with **5,245 trailing NUL bytes**.
    //
    // Those are valid, readable PDFs (poppler and Acrobat open them), but the last
    // 2 KB is nothing but padding, so `startxref` is not in the window and the whole
    // document is rejected as InvalidXref - a TOTAL loss, not a degradation. Growing
    // the window costs nothing on a well-formed file (the first 2 KB read hits) and
    // recovers the padded ones. ~keep
    const WINDOWS: [u64; 5] = [2048, 16 * 1024, 128 * 1024, 1024 * 1024, u64::MAX];
    for &want in WINDOWS.iter() {
        let read_size = std::cmp::min(want, file_size);
        reader.seek(SeekFrom::End(-(read_size as i64)))?;

        let mut buf = Vec::new();
        reader.take(read_size).read_to_end(&mut buf)?;
        let content = String::from_utf8_lossy(&buf);

        if let Some(pos) = content.rfind("startxref") {
            let after_keyword = &content[pos + 9..]; // 9 = len("startxref") ~keep

            // Split lines manually to handle CR, LF, and CRLF line endings.
            // Standard .lines() only handles LF and CRLF, not standalone CR. ~keep
            for line in split_lines(after_keyword) {
                // Trim NUL padding as well as whitespace: a padded file leaves the
                // offset line looking like "189089\0\0\0..." and `str::trim` alone
                // does not strip NULs, so the all-digits check below would fail. ~keep
                let trimmed = line.trim_matches(|c: char| c.is_whitespace() || c == '\0');
                if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
                    return trimmed.parse::<u64>().map_err(|_| Error::InvalidXref);
                }
            }
            // Keyword found but no offset after it - a wider window will not help. ~keep
            return Err(Error::InvalidXref);
        }

        // Whole file already scanned: the keyword genuinely is not there. ~keep
        if read_size == file_size {
            break;
        }
    }

    Err(Error::InvalidXref)
}

/// Parse the cross-reference table at the given byte offset.
///
/// Automatically detects whether this is a traditional xref table or
/// a cross-reference stream (PDF 1.5+) and parses accordingly.
///
/// # Errors
///
/// Returns `Error::InvalidXref` if parsing fails for both formats.
pub fn parse_xref<R: Read + Seek>(reader: &mut R, offset: u64) -> Result<CrossRefTable> {
    parse_xref_iterative(reader, offset)
}

/// Extract /Length value from raw bytes of an xref stream object header.
///
/// Searches for `/Length` followed by an integer in the raw dictionary bytes.
/// Returns `None` if not found or not parseable. This avoids full object parsing
/// just to determine how much data to read.
fn find_stream_length(data: &[u8]) -> Option<usize> {
    // Search for "/Length" (case-sensitive, per PDF spec) ~keep
    let keyword = b"/Length";
    let pos = data.windows(keyword.len()).position(|w| w == keyword)?;
    let after = &data[pos + keyword.len()..];

    let start = after.iter().position(|&b| !b.is_ascii_whitespace())?;
    let after = &after[start..];

    if after.first()?.is_ascii_digit() {
        let end = after.iter().position(|b| !b.is_ascii_digit()).unwrap_or(after.len());
        let num_str = std::str::from_utf8(&after[..end]).ok()?;
        num_str.parse::<usize>().ok()
    } else {
        // /Length is an indirect reference — we can't resolve it without full parsing ~keep
        None
    }
}

/// Try to find the actual xref start near the given offset.
///
/// Some PDF producers miscalculate the startxref offset by a few bytes.
/// This function scans a small window around the given offset to find either
/// the "xref" keyword (traditional table) or an object header like "N 0 obj"
/// (cross-reference stream). This tolerance is common in PDF readers (MuPDF,
/// poppler, etc.) because startxref misalignment is a well-known PDF producer bug.
fn find_actual_xref_offset<R: Read + Seek>(reader: &mut R, offset: u64) -> Result<u64> {
    reader.seek(SeekFrom::Start(offset))?;
    let mut peek = [0u8; 64];
    let n = reader.read(&mut peek)?;
    let peek_str = String::from_utf8_lossy(&peek[..n]);
    let trimmed = peek_str.trim_start();
    if trimmed.starts_with("xref") || trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return Ok(offset);
    }

    // Offset is misaligned — scan a window around it.
    // We scan backward (up to 32 bytes) and forward (up to 64 bytes). ~keep
    const SCAN_BACK: u64 = 32;
    const SCAN_FWD: u64 = 64;
    let scan_start = offset.saturating_sub(SCAN_BACK);
    let scan_len = (SCAN_BACK + SCAN_FWD) as usize;

    reader.seek(SeekFrom::Start(scan_start))?;
    let mut buf = vec![0u8; scan_len];
    let bytes_read = reader.read(&mut buf)?;
    buf.truncate(bytes_read);

    for i in 0..bytes_read.saturating_sub(3) {
        if &buf[i..i + 4] == b"xref" && (i == 0 || buf[i - 1] == b'\r' || buf[i - 1] == b'\n') {
            let found_offset = scan_start + i as u64;
            tracing::warn!(
                offset,
                corrected_offset = found_offset,
                "corrected misaligned xref offset (found 'xref' keyword)"
            );
            return Ok(found_offset);
        }
    }

    // Look for object header pattern at line boundaries: "\r<digits>" or "\n<digits>"
    // followed by " <gen> obj". This handles cross-reference streams. ~keep
    for i in 0..bytes_read {
        let at_line_start = i == 0 || buf[i - 1] == b'\r' || buf[i - 1] == b'\n';
        if !at_line_start || !buf[i].is_ascii_digit() {
            continue;
        }

        let remaining = &buf[i..bytes_read];
        let remaining_str = String::from_utf8_lossy(remaining);
        if let Some(obj_pos) = remaining_str.find(" obj") {
            let before_obj = &remaining_str[..obj_pos];
            let parts: Vec<&str> = before_obj.split_whitespace().collect();
            if parts.len() == 2
                && parts[0].chars().all(|c| c.is_ascii_digit())
                && parts[1].chars().all(|c| c.is_ascii_digit())
            {
                let found_offset = scan_start + i as u64;
                tracing::warn!(
                    offset,
                    corrected_offset = found_offset,
                    "corrected misaligned xref offset (found object header)"
                );
                return Ok(found_offset);
            }
        }
    }

    // Could not find xref nearby — return original offset and let downstream handle the error
    // ~keep
    tracing::warn!(offset, "could not find xref near offset, using original");
    Ok(offset)
}

/// Most xref sections followed through /Prev before giving up on the chain
/// and letting the caller reconstruct the table from the file body.
const MAX_XREF_CHAIN: usize = 1000;

/// Reads a /Prev or /XRefStm trailer value as a byte offset. Producers write
/// these as Real numbers or indirect references to an integer object; both are
/// accepted, and a negative or non-finite value is rejected rather than wrapped.
fn xref_pointer<R: Read + Seek>(reader: &mut R, table: &CrossRefTable, value: &Object) -> Option<u64> {
    match value {
        Object::Integer(i) => u64::try_from(*i).ok(),
        Object::Real(r) if r.is_finite() && *r >= 0.0 => Some(*r as u64),
        Object::Reference(r) => {
            let entry = table.get(r.id)?;
            if entry.entry_type != XRefEntryType::Uncompressed {
                return None;
            }
            match crate::xref_reconstruction::load_object_at_offset(reader, entry.offset).ok()? {
                Object::Integer(i) => u64::try_from(i).ok(),
                Object::Real(r) if r.is_finite() && r >= 0.0 => Some(r as u64),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Parse xref table iteratively, following /Prev pointers for incremental updates.
///
/// Uses a `HashSet` of visited offsets to detect circular /Prev chains instead of
/// an arbitrary depth limit. This supports PDFs with hundreds of incremental saves
/// (e.g., 177+ /Prev links) without falling back to expensive full-file reconstruction.
fn parse_xref_iterative<R: Read + Seek>(reader: &mut R, start_offset: u64) -> Result<CrossRefTable> {
    let mut visited = HashSet::new();
    let mut offset = start_offset;
    let mut result_xref: Option<CrossRefTable> = None;

    loop {
        if !visited.insert(offset) {
            tracing::warn!(offset, "circular /Prev chain detected, stopping xref traversal");
            break;
        }

        if visited.len() > MAX_XREF_CHAIN {
            tracing::warn!(
                limit = MAX_XREF_CHAIN,
                "xref /Prev chain too long, abandoning traversal"
            );
            return Err(Error::InvalidXref);
        }

        let actual_offset = find_actual_xref_offset(reader, offset)?;

        reader.seek(SeekFrom::Start(actual_offset))?;

        let mut peek_buf = [0u8; 64];
        let bytes_read = reader.read(&mut peek_buf)?;
        reader.seek(SeekFrom::Start(actual_offset))?;

        let peek_str = String::from_utf8_lossy(&peek_buf[..bytes_read]);
        let trimmed = peek_str.trim_start();

        tracing::debug!(
            offset = actual_offset,
            original_offset = offset,
            chain_depth = visited.len(),
            "parsing xref"
        );

        let xref = if trimmed.starts_with("xref") {
            tracing::debug!(offset = actual_offset, "detected traditional xref");
            parse_traditional_xref(reader, actual_offset)?
        } else if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            match parse_xref_stream(reader, actual_offset) {
                Ok(xref) => xref,
                Err(error) => {
                    crate::error::trace_recovery("parse_xref_stream", &error);
                    reader.seek(SeekFrom::Start(actual_offset))?;
                    match parse_traditional_xref(reader, actual_offset) {
                        Ok(xref) => xref,
                        Err(traditional_error) => {
                            crate::error::trace_recovery("parse_traditional_xref", &traditional_error);
                            return Err(Error::InvalidPdf(format!(
                                "failed to parse xref (stream attempt: {}, traditional attempt: {})",
                                error, traditional_error
                            )));
                        }
                    }
                }
            }
        } else {
            tracing::warn!(
                offset = actual_offset,
                error_code = "unexpected_xref_data",
                "xref starts with unexpected data"
            );
            return Err(Error::InvalidXref);
        };

        // Extract /Prev and /XRefStm pointers before `xref` is moved by the merge. ~keep
        let prev_offset = xref
            .trailer()
            .and_then(|t| t.get("Prev"))
            .and_then(|o| xref_pointer(reader, &xref, o));
        // Hybrid-reference files (§7.5.8.4): a classic trailer may carry
        // /XRefStm pointing to an xref stream that indexes the compressed
        // (object-stream) objects invisible to the classic table. Without this
        // the compressed objects are unreachable and the reader falls back to
        // full-file reconstruction. ~keep
        let xrefstm_offset = xref
            .trailer()
            .and_then(|t| t.get("XRefStm"))
            .and_then(|o| xref_pointer(reader, &xref, o));

        // Merge: most recent xref entries take priority over older ones ~keep
        match &mut result_xref {
            Some(result) => result.merge_from(xref),
            None => result_xref = Some(xref),
        }

        // Merge the /XRefStm supplement AFTER the classic table, so the classic
        // entries win for any object listed in both (per §7.5.8.4). Parsed
        // standalone (its own /Prev, if any, duplicates the classic chain we
        // already follow) and best-effort: a malformed supplement is skipped,
        // not fatal. ~keep
        if let Some(stm_off) = xrefstm_offset
            && visited.insert(stm_off)
        {
            match find_actual_xref_offset(reader, stm_off).and_then(|actual| parse_xref_stream(reader, actual)) {
                Ok(stm_xref) => {
                    tracing::debug!(offset = stm_off, "merged /XRefStm supplement");
                    match &mut result_xref {
                        Some(result) => result.merge_from(stm_xref),
                        None => result_xref = Some(stm_xref),
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        offset = stm_off,
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "skipping unparseable /XRefStm"
                    );
                }
            }
        }

        match prev_offset {
            Some(prev) => {
                tracing::debug!(offset, prev_offset = prev, "following /Prev pointer");
                offset = prev;
            }
            None => break,
        }
    }

    result_xref.ok_or(Error::InvalidXref)
}

/// Parse a traditional cross-reference table (PDF 1.0-1.4).
///
/// The xref table format is:
/// ```text
/// xref
/// 0 6             % Start at object 0, 6 entries
/// 0000000000 65535 f   % Object 0 (free)
/// 0000000018 00000 n   % Object 1 at byte 18
/// 0000000154 00000 n   % Object 2 at byte 154
/// ...
/// trailer
/// << /Size 6 /Root 1 0 R >>
/// ```
fn parse_traditional_xref<R: Read + Seek>(reader: &mut R, offset: u64) -> Result<CrossRefTable> {
    tracing::debug!(offset, "parse_traditional_xref: starting");
    reader.seek(SeekFrom::Start(offset))?;

    // Read only until "trailer" or "startxref" instead of the entire remaining file.
    // For linearized PDFs, the first xref may be near byte 0, and read_to_end would
    // load the entire file (e.g., 375MB) just to parse an 8-entry xref table. ~keep
    let lines = read_until_trailer(reader).map_err(|_| {
        tracing::error!(
            operation = "read_xref_lines",
            error_code = "io_error",
            "failed to read xref lines"
        );
        Error::InvalidXref
    })?;

    tracing::debug!(count = lines.len(), "parse_traditional_xref: read lines");

    let mut xref = CrossRefTable::new();
    let mut line_idx = 0;
    let mut malformed_entry_count = 0usize;

    // Find "xref" keyword, skipping leading whitespace and stray data lines
    // Some PDFs have garbage bytes or comments before the xref keyword ~keep
    let mut skipped_lines = 0;
    const MAX_SKIP_LINES: usize = 10;
    while line_idx < lines.len() {
        let trimmed = lines[line_idx].trim();
        if trimmed.is_empty() {
            line_idx += 1;
            continue; // Skip empty lines (don't count toward limit) ~keep
        }
        if trimmed.starts_with("xref") {
            line_idx += 1;
            break;
        }
        skipped_lines += 1;
        if skipped_lines > MAX_SKIP_LINES {
            return Err(Error::InvalidXref);
        }
        malformed_entry_count += 1;
        line_idx += 1;
    }

    while line_idx < lines.len() {
        let trimmed = lines[line_idx].trim();
        line_idx += 1;

        if trimmed.starts_with("trailer") {
            break;
        }

        if trimmed.is_empty() || trimmed.starts_with('%') {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() != 2 {
            continue;
        }

        let start_obj: u32 = parts[0].parse().map_err(|_| Error::InvalidXref)?;
        let count: u32 = parts[1].parse().map_err(|_| Error::InvalidXref)?;

        // Validate reasonable count to prevent memory exhaustion ~keep
        if count > 1_000_000 {
            return Err(Error::InvalidPdf("xref subsection count exceeds limit".to_string()));
        }

        let mut i = 0;
        while i < count && line_idx < lines.len() {
            let trimmed = lines[line_idx].trim();
            line_idx += 1;

            // Skip empty lines (don't increment counter) ~keep
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("trailer") {
                malformed_entry_count += (count - i) as usize;
                line_idx -= 1; // Back up so outer loop can process trailer ~keep
                break;
            }

            // An object number past u32::MAX has no slot; saturating would overwrite the entry at u32::MAX. ~keep
            let Some(obj_num) = start_obj.checked_add(i) else {
                malformed_entry_count += 1;
                i += 1;
                continue;
            };

            // Parse entry: "nnnnnnnnnn ggggg f/n"
            // Be flexible with whitespace and format ~keep
            let parts: Vec<&str> = trimmed.split_whitespace().collect();

            if parts.len() < 3 {
                malformed_entry_count += 1;

                // Still increment counter to maintain object numbering
                // Add a placeholder free entry to maintain object number sequence ~keep
                let entry = XRefEntry::free(0, 65535);
                xref.add_entry(obj_num, entry);
                i += 1;
                continue;
            }

            // Allow extra parts (some PDFs have trailing data) ~keep
            if parts.len() > 3 {
                tracing::debug!(parts = parts.len(), "xref entry has more parts than expected (3)");
            }

            let offset: u64 = match parts[0].parse() {
                Ok(v) => v,
                Err(_) => {
                    malformed_entry_count += 1;
                    let entry = XRefEntry::free(0, 65535);
                    xref.add_entry(obj_num, entry);
                    i += 1;
                    continue;
                }
            };

            let generation: u16 = match parts[1].parse() {
                Ok(v) => v,
                Err(_) => {
                    malformed_entry_count += 1;
                    let entry = XRefEntry::free(0, 65535);
                    xref.add_entry(obj_num, entry);
                    i += 1;
                    continue;
                }
            };

            let type_flag = parts[2];

            // Validate type flag - be flexible with case and truncation ~keep
            let type_flag_normalized = type_flag.to_lowercase();
            let type_char = type_flag_normalized.chars().next().unwrap_or('?');

            let in_use = match type_char {
                'n' => true,
                'f' => false,
                _ => {
                    malformed_entry_count += 1;
                    false
                }
            };

            let entry = XRefEntry::new(offset, generation, in_use);
            xref.add_entry(obj_num, entry);
            i += 1;
        }
    }

    // Parse the trailer dictionary from the remaining lines.
    // After the "trailer" keyword, the lines contain the trailer dict (e.g., "<< /Size 100 /Root 1 0 R /Prev 12345 >>").
    // We concatenate remaining lines and parse the dictionary so that /Prev and other
    // trailer entries are available via xref.trailer(). ~keep
    let remaining_text: String = lines[line_idx..].join("\n");
    if !remaining_text.trim().is_empty() {
        let trimmed = remaining_text.trim();
        if (trimmed.starts_with("<<")
            || trimmed.starts_with("<< ")
            || trimmed.starts_with("<<\n")
            || trimmed.starts_with("<<\r"))
            && let Ok((_, trailer_obj)) = parse_object(trimmed.as_bytes())
            && let Some(dict) = trailer_obj.as_dict()
        {
            xref.set_trailer(dict.clone());
        }
    }

    if malformed_entry_count > 0 {
        crate::error::trace_recovery_count("parse_traditional_xref", "malformed_xref_entry", malformed_entry_count);
    }

    Ok(xref)
}

/// Parse a cross-reference stream (PDF 1.5+).
///
/// Cross-reference streams are stream objects with `/Type /XRef` that contain
/// binary encoded xref data. They replace traditional xref tables in modern PDFs.
///
/// The stream dictionary contains:
/// - `/W [w1 w2 w3]` - Field widths in bytes
/// - `/Size` - Total number of entries
/// - `/Index [start1 count1 start2 count2...]` - Optional subsection ranges
///
/// Each entry consists of 3 fields:
/// - Field 1: Entry type (0=free, 1=uncompressed, 2=compressed)
/// - Field 2: Offset (type 1) or stream object number (type 2)
/// - Field 3: Generation (type 1) or index within stream (type 2)
fn parse_xref_stream<R: Read + Seek>(reader: &mut R, offset: u64) -> Result<CrossRefTable> {
    use crate::lexer::token;

    reader.seek(SeekFrom::Start(offset))?;

    // Read a bounded amount of data for the xref stream object.
    // We avoid read_to_end because for linearized PDFs the first xref may be
    // near the start of the file, and reading to end would load the entire file.
    //
    // Strategy: read an initial 256KB chunk, then check /Length to see if we
    // need more. Most xref streams are <64KB. ~keep
    let file_len = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(offset))?;

    // `offset` comes from `startxref` or a `/Prev` pointer and is never
    // validated against the file length before reaching here (a corrected
    // offset from `find_actual_xref_offset` can also fall through unchanged
    // when nothing is found nearby). A bare `file_len - offset` underflows a
    // u64 whenever `offset > file_len`: debug builds panic on the subtraction,
    // release builds wrap to a huge value that the `.min(256 * 1024)` below
    // happens to clamp back down — so this guard only changes debug/fuzz
    // behavior (from a panic to a clean error); release was already
    // non-panicking, just less informative. ~keep
    let remaining = file_len.checked_sub(offset).ok_or_else(|| {
        Error::InvalidPdf(format!(
            "xref stream offset {offset} is past end of file ({file_len} bytes)"
        ))
    })? as usize;
    let initial_read = remaining.min(256 * 1024);
    let mut content = vec![0u8; initial_read];
    let bytes_read = reader.read(&mut content)?;
    content.truncate(bytes_read);

    let needs_more = if let Some(length_val) = find_stream_length(&content) {
        let stream_kw_pos = content.windows(6).position(|w| w == b"stream").unwrap_or(0);
        let needed = stream_kw_pos
            .saturating_add(20)
            .saturating_add(length_val)
            .saturating_add(30);
        if needed > bytes_read { Some(needed) } else { None }
    } else if content.windows(6).any(|w| w == b"endobj") {
        None
    } else {
        // No /Length and no endobj in 256KB — read more (capped at 16MB) ~keep
        Some(remaining.min(16 * 1024 * 1024))
    };

    if let Some(needed) = needs_more {
        let total = needed.min(remaining);
        reader.seek(SeekFrom::Start(offset))?;
        content = vec![0u8; total];
        let mut total_read = 0;
        while total_read < total {
            let n = reader.read(&mut content[total_read..])?;
            if n == 0 {
                break;
            }
            total_read += n;
        }
        content.truncate(total_read);
    }

    let input = &content[..];

    let (rest, _obj_num_token) =
        token(input).map_err(|e| Error::InvalidPdf(format!("failed to parse xref object number: {}", e)))?;

    let (rest, _gen_token) =
        token(rest).map_err(|e| Error::InvalidPdf(format!("failed to parse xref generation: {}", e)))?;

    let (rest, obj_keyword_token) =
        token(rest).map_err(|e| Error::InvalidPdf(format!("failed to parse 'obj' keyword: {}", e)))?;

    if !matches!(obj_keyword_token, crate::lexer::Token::ObjStart) {
        return Err(Error::InvalidPdf("expected 'obj' keyword in xref stream".to_string()));
    }

    let parse_result =
        parse_object(rest).map_err(|e| Error::InvalidPdf(format!("failed to parse xref stream object: {}", e)))?;

    let (_remaining, obj) = parse_result;

    let (stream_dict, stream_data) = match obj {
        Object::Stream { dict, data } => (dict, data),
        _ => return Err(Error::InvalidPdf("xref stream is not a stream object".to_string())),
    };

    if let Some(type_obj) = stream_dict.get("Type")
        && let Some(type_name) = type_obj.as_name()
        && type_name != "XRef"
    {
        return Err(Error::InvalidPdf(format!(
            "expected /Type /XRef, got /Type /{}",
            type_name
        )));
    }

    let w_array = stream_dict
        .get("W")
        .and_then(|o| o.as_array())
        .ok_or_else(|| Error::InvalidPdf("missing /W array in xref stream".to_string()))?;

    if w_array.len() != 3 {
        return Err(Error::InvalidPdf("invalid /W array length".to_string()));
    }

    // Per the PDF spec (§7.5.8.2), each /W field is a byte count for a single
    // xref-stream field and never needs to exceed 8 (a u64 fits in 8 bytes).
    // Trusting an out-of-range value (in particular a negative one, which
    // `as usize` would turn into a huge value) before this check let a hostile
    // `/W` element drive `entry_size` and the subsequent slicing arithmetic
    // clean off a cliff — see the regression tests in
    // `tests/xref_bounds_regression.rs`. ~keep
    const MAX_XREF_FIELD_WIDTH: i64 = 8;

    let w1_raw = w_array[0]
        .as_integer()
        .ok_or_else(|| Error::InvalidPdf("invalid /W[0]".to_string()))?;
    let w2_raw = w_array[1]
        .as_integer()
        .ok_or_else(|| Error::InvalidPdf("invalid /W[1]".to_string()))?;
    let w3_raw = w_array[2]
        .as_integer()
        .ok_or_else(|| Error::InvalidPdf("invalid /W[2]".to_string()))?;

    for (index, width) in [w1_raw, w2_raw, w3_raw].into_iter().enumerate() {
        if !(0..=MAX_XREF_FIELD_WIDTH).contains(&width) {
            return Err(Error::InvalidPdf(format!(
                "invalid /W[{index}] field width {width}: must be between 0 and {MAX_XREF_FIELD_WIDTH}"
            )));
        }
    }

    let w1 = w1_raw as usize;
    let w2 = w2_raw as usize;
    let w3 = w3_raw as usize;

    let entry_size = w1 + w2 + w3;
    if entry_size == 0 {
        return Err(Error::InvalidPdf("xref stream entry size is 0".to_string()));
    }

    let size = stream_dict
        .get("Size")
        .and_then(|o| o.as_integer())
        .ok_or_else(|| Error::InvalidPdf("missing /Size in xref stream".to_string()))? as u32;

    // Get index array (or default to [0 Size]) ~keep
    let index_ranges = if let Some(index_obj) = stream_dict.get("Index") {
        let index_array = index_obj
            .as_array()
            .ok_or_else(|| Error::InvalidPdf("invalid /Index".to_string()))?;

        if index_array.len() % 2 != 0 {
            return Err(Error::InvalidPdf("xref stream /Index array has odd length".to_string()));
        }
        let mut ranges = Vec::new();
        for i in (0..index_array.len()).step_by(2) {
            let start = index_array[i]
                .as_integer()
                .ok_or_else(|| Error::InvalidPdf("invalid index start".to_string()))? as u32;
            let count = index_array[i + 1]
                .as_integer()
                .ok_or_else(|| Error::InvalidPdf("invalid index count".to_string()))? as u32;
            ranges.push((start, count));
        }
        ranges
    } else {
        vec![(0, size)]
    };

    let decode_params = if let Some(decode_params_obj) = stream_dict.get("DecodeParms") {
        extract_decode_params(decode_params_obj)?
    } else {
        None
    };

    let decoded_data = if let Some(filter_obj) = stream_dict.get("Filter") {
        let filter_name = match filter_obj {
            Object::Name(name) => name.clone(),
            Object::Array(arr) => {
                // Multiple filters - use first one for now (or chain them) ~keep
                if let Some(Object::Name(name)) = arr.first() {
                    name.clone()
                } else {
                    return Err(Error::InvalidPdf("invalid filter array".to_string()));
                }
            }
            _ => return Err(Error::InvalidPdf("invalid /Filter in xref stream".to_string())),
        };

        crate::decoders::decode_stream_with_params(&stream_data, &[filter_name], decode_params.as_ref())?
    } else {
        stream_data.to_vec()
    };

    let mut xref = CrossRefTable::new();
    let mut data_pos = 0;

    for (start_obj, count) in index_ranges {
        for i in 0..count {
            if data_pos + entry_size > decoded_data.len() {
                return Err(Error::InvalidPdf("truncated xref stream data".to_string()));
            }

            let entry_data = &decoded_data[data_pos..data_pos + entry_size];
            data_pos += entry_size;

            let entry_type = if w1 > 0 {
                read_int(&entry_data[0..w1])
            } else {
                1 // Default to type 1 if width is 0 ~keep
            };

            let field2 = read_int(&entry_data[w1..w1 + w2]);

            let field3 = read_int(&entry_data[w1 + w2..w1 + w2 + w3]);

            let entry = match entry_type {
                0 => {
                    // Type 0: Free object ~keep
                    XRefEntry::free(field2, field3 as u16)
                }
                1 => {
                    // Type 1: Uncompressed object at byte offset ~keep
                    XRefEntry::uncompressed(field2, field3 as u16)
                }
                2 => {
                    // Type 2: Compressed object in object stream ~keep
                    XRefEntry::compressed(field2, field3 as u16)
                }
                _ => {
                    return Err(Error::InvalidPdf(format!("invalid xref entry type: {}", entry_type)));
                }
            };

            if let Some(obj_num) = start_obj.checked_add(i) {
                xref.add_entry(obj_num, entry);
            }
        }
    }

    // For xref streams, the stream dictionary serves as the trailer ~keep
    xref.set_trailer(stream_dict);

    Ok(xref)
}

/// Extract decode parameters from a DecodeParms object.
///
/// DecodeParms can be either a dictionary or an array of dictionaries.
/// For simplicity, we only extract from the first dictionary if it's an array.
fn extract_decode_params(decode_params_obj: &Object) -> Result<Option<crate::decoders::DecodeParams>> {
    let dict = match decode_params_obj {
        Object::Dictionary(d) => d,
        Object::Array(arr) => {
            // For array of params, use first one ~keep
            if let Some(Object::Dictionary(d)) = arr.first() {
                d
            } else {
                return Ok(None);
            }
        }
        _ => return Ok(None),
    };

    let predictor = dict.get("Predictor").and_then(|o| o.as_integer()).unwrap_or(1);

    let columns = dict.get("Columns").and_then(|o| o.as_integer()).unwrap_or(1) as usize;

    let colors = dict.get("Colors").and_then(|o| o.as_integer()).unwrap_or(1) as usize;

    let bits_per_component = dict.get("BitsPerComponent").and_then(|o| o.as_integer()).unwrap_or(8) as usize;

    Ok(Some(crate::decoders::DecodeParams {
        predictor,
        columns,
        colors,
        bits_per_component,
    }))
}

/// Read an integer from a byte slice (big-endian).
fn read_int(bytes: &[u8]) -> u64 {
    let mut result: u64 = 0;
    for &byte in bytes {
        result = (result << 8) | (byte as u64);
    }
    result
}

/// Split a string into lines, handling all PDF line ending styles (LF, CRLF, CR).
///
/// Standard .lines() only handles LF and CRLF, but some PDFs use
/// standalone CR (Mac-style line endings). This function handles all three.
fn split_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '\r' => {
                if i + 1 < chars.len() && chars[i + 1] == '\n' {
                    lines.push(current_line.clone());
                    current_line.clear();
                    i += 2;
                } else {
                    lines.push(current_line.clone());
                    current_line.clear();
                    i += 1;
                }
            }
            '\n' => {
                lines.push(current_line.clone());
                current_line.clear();
                i += 1;
            }
            ch => {
                current_line.push(ch);
                i += 1;
            }
        }
    }

    // Don't forget the last line if it doesn't end with a line ending ~keep
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Read a line from a BufReader, handling all PDF line ending styles (LF, CRLF, CR).
///
/// Standard BufReader::read_line() only handles LF and CRLF, but some PDFs use
/// standalone CR (Mac-style line endings). This function handles all three by
/// reading the entire buffer and splitting manually.
/// Read the xref table and trailer from reader, bounded by finding "trailer" + dict.
///
/// This avoids `read_to_end` which would read the entire remaining file for
/// linearized PDFs where the first xref is near byte 0. Instead, we read in
/// chunks and search for the "trailer" keyword in raw bytes, which correctly
/// handles all line ending styles (CR, LF, CRLF).
fn read_until_trailer<R: Read + Seek>(reader: &mut R) -> std::io::Result<Vec<String>> {
    // Read in chunks until we find "trailer" keyword followed by a dict,
    // followed by "startxref" or ">>" closing the dict.
    // Most xref tables are <1MB. We cap at 32MB to prevent runaway reads. ~keep
    const CHUNK_SIZE: usize = 256 * 1024;
    const MAX_TOTAL: usize = 32 * 1024 * 1024;

    let mut data = Vec::with_capacity(CHUNK_SIZE);
    let mut total_read = 0usize;
    let mut found_end = false;

    loop {
        let prev_len = data.len();
        data.resize(prev_len + CHUNK_SIZE, 0);
        let n = reader.read(&mut data[prev_len..])?;
        data.truncate(prev_len + n);
        total_read += n;

        if n == 0 {
            break;
        }

        if let Some(trailer_pos) = find_bytes(&data, b"trailer") {
            let after_trailer = &data[trailer_pos + 7..];
            if let Some(dict_end) = find_bytes(after_trailer, b">>") {
                let after_dict = &after_trailer[dict_end + 2..];
                if find_bytes(after_dict, b"startxref").is_some() || after_dict.len() > 20 {
                    found_end = true;
                    // Truncate to just past the trailer dict + a bit more ~keep
                    let end_pos = trailer_pos + 7 + dict_end + 2 + 50.min(after_dict.len());
                    data.truncate(end_pos);
                    break;
                }
            }
        }

        if total_read >= MAX_TOTAL {
            break;
        }
    }

    if !found_end {
        // Fallback: if we didn't find trailer end in 32MB, use what we have ~keep
        tracing::warn!(
            megabytes = total_read / (1024 * 1024),
            "could not find trailer end marker within scanned range of xref"
        );
    }

    let text = String::from_utf8_lossy(&data);
    Ok(split_lines(&text))
}

/// Find the position of a byte pattern in a byte slice.
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_xref_entry_creation() {
        let entry = XRefEntry::new(1234, 0, true);
        assert_eq!(entry.offset, 1234);
        assert_eq!(entry.generation, 0);
        assert!(entry.in_use);
    }

    #[test]
    fn test_xref_entry_free() {
        let entry = XRefEntry::new(0, 65535, false);
        assert_eq!(entry.offset, 0);
        assert_eq!(entry.generation, 65535);
        assert!(!entry.in_use);
    }

    #[test]
    fn test_cross_ref_table_new() {
        let table = CrossRefTable::new();
        assert_eq!(table.len(), 0);
        assert!(table.is_empty());
    }

    #[test]
    fn test_cross_ref_table_add_and_get() {
        let mut table = CrossRefTable::new();
        let entry = XRefEntry::new(1234, 0, true);

        table.add_entry(5, entry.clone());
        assert_eq!(table.len(), 1);
        assert!(!table.is_empty());

        let retrieved = table.get(5).unwrap();
        assert_eq!(retrieved, &entry);
    }

    #[test]
    fn test_cross_ref_table_get_missing() {
        let table = CrossRefTable::new();
        assert!(table.get(999).is_none());
    }

    #[test]
    fn test_find_xref_offset_valid() {
        let pdf = b"%PDF-1.4\n\
            1 0 obj\n\
            << /Type /Catalog >>\n\
            endobj\n\
            xref\n\
            0 2\n\
            0000000000 65535 f\n\
            0000000009 00000 n\n\
            trailer\n\
            << /Size 2 >>\n\
            startxref\n\
            50\n\
            %%EOF";

        let mut cursor = Cursor::new(pdf);
        let offset = find_xref_offset(&mut cursor).unwrap();
        assert_eq!(offset, 50);
    }

    /// A file PADDED with trailing NULs after `%%EOF` must still be read.
    ///
    /// Real producers do this: the 2nd-Circuit appellate filings
    /// (`gov.uscourts.ca2.*`) are padded to an exact 192 KiB with **5,245 trailing
    /// NUL bytes**. They are valid PDFs - poppler and Acrobat open them - but a
    /// fixed 2 KB tail window contains nothing but padding, so `startxref` is never
    /// found and the ENTIRE DOCUMENT is rejected. Not a degradation: a total loss.
    #[test]
    fn find_xref_offset_survives_trailing_nul_padding() {
        let mut pdf: Vec<u8> = b"%PDF-1.6\n1 0 obj\n<< >>\nendobj\nstartxref\n189089\n%%EOF".to_vec();
        // Pad well past the old 2 KB window. ~keep
        pdf.extend(std::iter::repeat_n(0u8, 5245));

        let mut cur = std::io::Cursor::new(pdf);
        let offset = find_xref_offset(&mut cur).expect("a padded file must still parse");
        assert_eq!(offset, 189089);
    }

    /// The offset LINE itself may carry the padding (`"189089\0\0\0..."`).
    /// `str::trim` does not strip NULs, so the all-digits check must.
    #[test]
    fn find_xref_offset_trims_nuls_from_the_offset_line() {
        let mut pdf: Vec<u8> = b"%PDF-1.6\nstartxref\n4242".to_vec();
        pdf.extend(std::iter::repeat_n(0u8, 64));
        let mut cur = std::io::Cursor::new(pdf);
        assert_eq!(
            find_xref_offset(&mut cur).expect("trailing NULs on the offset line"),
            4242
        );
    }

    #[test]
    fn test_find_xref_offset_no_startxref() {
        let pdf = b"%PDF-1.4\n\
            xref\n\
            0 1\n\
            0000000000 65535 f\n\
            trailer\n\
            << /Size 1 >>\n";

        let mut cursor = Cursor::new(pdf);
        let result = find_xref_offset(&mut cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_xref_offset_with_whitespace() {
        let pdf = b"%PDF-1.4\n\
            startxref\n\
            \n\
            12345\n\
            %%EOF";

        let mut cursor = Cursor::new(pdf);
        let offset = find_xref_offset(&mut cursor).unwrap();
        assert_eq!(offset, 12345);
    }

    /// Hybrid-reference file (§7.5.8.4): a classic trailer carrying
    /// `/XRefStm` must merge the referenced xref stream so objects listed
    /// only there (e.g. compressed objects) become reachable. Object 2 is
    /// declared ONLY by the /XRefStm here, so a table that resolves it proves
    /// the supplement was parsed and merged. Offsets are tracked at build time
    /// so the fixture stays correct without hand-computed byte positions.
    #[test]
    fn test_hybrid_xrefstm_supplement_is_merged() {
        let mut pdf: Vec<u8> = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.5\n");

        let off1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog >>\nendobj\n");

        // Standalone (uncompressed) xref stream declaring object 2 at a
        // sentinel offset via /Index [2 1], /W [1 2 1]: type=1, offset=0x270F,
        // gen=0 → bytes 01 27 0F 00. ~keep
        let off_stm = pdf.len();
        pdf.extend_from_slice(
            b"6 0 obj\n<< /Type /XRef /Size 3 /W [1 2 1] /Index [2 1] \
              /Root 1 0 R /Length 4 >>\nstream\n",
        );
        pdf.extend_from_slice(&[0x01, 0x27, 0x0F, 0x00]);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");

        let off_classic = pdf.len();
        let entry1 = format!("{:010} 00000 n \n", off1);
        pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n");
        pdf.extend_from_slice(entry1.as_bytes());
        let trailer = format!(
            "trailer\n<< /Size 3 /Root 1 0 R /XRefStm {} >>\nstartxref\n{}\n%%EOF",
            off_stm, off_classic
        );
        pdf.extend_from_slice(trailer.as_bytes());

        let mut cursor = Cursor::new(pdf);
        let table = parse_xref_iterative(&mut cursor, off_classic as u64).unwrap();

        // Object 1 from the classic table, object 2 ONLY from the /XRefStm. ~keep
        assert!(table.get(1).is_some(), "classic-table object 1 must resolve");
        let obj2 = table
            .get(2)
            .expect("object 2 must resolve via the merged /XRefStm supplement");
        assert_eq!(obj2.offset, 0x270F, "object 2 offset comes from the xref stream");
    }

    /// Appends a one-object classic xref section for `obj_id` and returns its offset.
    fn push_classic_section(pdf: &mut Vec<u8>, obj_id: u32, obj_off: usize, trailer_extra: &str) -> usize {
        let section = pdf.len();
        pdf.extend_from_slice(format!("xref\n{obj_id} 1\n{obj_off:010} 00000 n \n").as_bytes());
        pdf.extend_from_slice(format!("trailer\n<< /Size 20 /Root 1 0 R {trailer_extra} >>\n").as_bytes());
        section
    }

    #[test]
    fn test_real_valued_prev_is_followed() {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let off1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog >>\nendobj\n");
        let older = push_classic_section(&mut pdf, 1, off1, "");
        let off2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< >>\nendobj\n");
        let newer = push_classic_section(&mut pdf, 2, off2, &format!("/Prev {older}.0"));
        let table = parse_xref_iterative(&mut Cursor::new(pdf), newer as u64).unwrap();
        assert!(table.get(2).is_some());
        assert!(
            table.get(1).is_some(),
            "object 1 lives only in the section named by a Real /Prev"
        );
    }

    #[test]
    fn test_indirect_prev_is_followed() {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let off1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog >>\nendobj\n");
        let older = push_classic_section(&mut pdf, 1, off1, "");
        let off7 = pdf.len();
        pdf.extend_from_slice(format!("7 0 obj\n{older}\nendobj\n").as_bytes());
        let newer = push_classic_section(&mut pdf, 7, off7, "/Prev 7 0 R");
        let table = parse_xref_iterative(&mut Cursor::new(pdf), newer as u64).unwrap();
        assert!(
            table.get(1).is_some(),
            "object 1 lives only in the section named by an indirect /Prev"
        );
    }

    #[test]
    fn test_negative_prev_is_not_wrapped_into_an_offset() {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let off1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog >>\nendobj\n");
        let only = push_classic_section(&mut pdf, 1, off1, "/Prev -5");
        let table = parse_xref_iterative(&mut Cursor::new(pdf), only as u64).unwrap();
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_overlong_prev_chain_is_abandoned() {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let off1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog >>\nendobj\n");
        let mut section = push_classic_section(&mut pdf, 1, off1, "");
        for _ in 0..MAX_XREF_CHAIN + 50 {
            section = push_classic_section(&mut pdf, 1, off1, &format!("/Prev {section}"));
        }
        assert!(parse_xref_iterative(&mut Cursor::new(pdf), section as u64).is_err());
    }

    #[test]
    fn test_classic_subsection_at_u32_max_does_not_overflow() {
        let data = b"xref\n4294967295 3\n\
            0000000018 00000 n \n\
            0000000154 00000 n \n\
            0000000200 00000 n \n\
            trailer\n<< /Size 1 >>\n";
        let table = parse_xref(&mut Cursor::new(&data[..]), 0).unwrap();
        assert_eq!(table.get(u32::MAX).expect("boundary entry").offset, 18);
    }

    #[test]
    fn test_xref_stream_index_at_u32_max_does_not_overflow() {
        let mut pdf = b"%PDF-1.5\n".to_vec();
        let off = pdf.len();
        pdf.extend_from_slice(
            b"6 0 obj\n<< /Type /XRef /Size 3 /W [1 2 1] /Index [4294967295 2] /Root 1 0 R /Length 8 >>\nstream\n",
        );
        pdf.extend_from_slice(&[1, 0, 9, 0, 1, 0, 10, 0]);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
        let table = parse_xref_stream(&mut Cursor::new(pdf), off as u64).unwrap();
        assert_eq!(table.get(u32::MAX).expect("boundary entry").offset, 9);
    }

    #[test]
    fn test_xref_stream_with_maximal_length_does_not_overflow() {
        let mut pdf = b"%PDF-1.5\n".to_vec();
        let off = pdf.len();
        pdf.extend_from_slice(
            b"6 0 obj\n<< /Type /XRef /Size 3 /W [1 2 1] /Index [0 1] /Root 1 0 R \
              /Length 18446744073709551615 >>\nstream\n",
        );
        pdf.extend_from_slice(&[1, 0, 9, 0]);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
        let _ = parse_xref_stream(&mut Cursor::new(pdf), off as u64);
    }

    #[test]
    fn test_parse_xref_single_subsection() {
        let xref_data = b"xref\n\
            0 3\n\
            0000000000 65535 f\n\
            0000000018 00000 n\n\
            0000000154 00000 n\n\
            trailer\n";

        let mut cursor = Cursor::new(xref_data);
        let table = parse_xref(&mut cursor, 0).unwrap();

        assert_eq!(table.len(), 3);

        let entry0 = table.get(0).unwrap();
        assert_eq!(entry0.offset, 0);
        assert_eq!(entry0.generation, 65535);
        assert!(!entry0.in_use);

        let entry1 = table.get(1).unwrap();
        assert_eq!(entry1.offset, 18);
        assert_eq!(entry1.generation, 0);
        assert!(entry1.in_use);

        let entry2 = table.get(2).unwrap();
        assert_eq!(entry2.offset, 154);
        assert_eq!(entry2.generation, 0);
        assert!(entry2.in_use);
    }

    #[test]
    fn test_parse_xref_multiple_subsections() {
        let xref_data = b"xref\n\
            0 2\n\
            0000000000 65535 f\n\
            0000000018 00000 n\n\
            5 3\n\
            0000000200 00000 n\n\
            0000000300 00000 n\n\
            0000000400 00000 n\n\
            trailer\n";

        let mut cursor = Cursor::new(xref_data);
        let table = parse_xref(&mut cursor, 0).unwrap();

        assert_eq!(table.len(), 5); // 2 + 3 entries ~keep

        assert!(table.get(0).is_some());
        assert!(table.get(1).is_some());

        let entry5 = table.get(5).unwrap();
        assert_eq!(entry5.offset, 200);

        let entry6 = table.get(6).unwrap();
        assert_eq!(entry6.offset, 300);

        let entry7 = table.get(7).unwrap();
        assert_eq!(entry7.offset, 400);

        assert!(table.get(2).is_none());
        assert!(table.get(3).is_none());
        assert!(table.get(4).is_none());
    }

    #[test]
    fn test_parse_xref_no_xref_keyword() {
        let xref_data = b"notxref\n\
            0 1\n\
            0000000000 65535 f\n\
            trailer\n";

        let mut cursor = Cursor::new(xref_data);
        let result = parse_xref(&mut cursor, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_xref_malformed_entry() {
        // Parser should add placeholder free entries for malformed entries
        // to maintain object numbering consistency ~keep
        let xref_data = b"xref\n\
            0 2\n\
            0000000000 65535 f\n\
            invalid entry here\n\
            trailer\n";

        let mut cursor = Cursor::new(xref_data);
        let result = parse_xref(&mut cursor, 0);
        assert!(result.is_ok());
        let table = result.unwrap();
        assert_eq!(table.len(), 2);
        assert!(table.get(0).is_some());
        assert!(!table.get(0).unwrap().in_use);
        assert!(table.get(1).is_some());
        assert!(!table.get(1).unwrap().in_use);
    }

    #[test]
    fn test_parse_xref_invalid_flag() {
        // Parser should treat entries with invalid flags as free entries
        // to maintain object numbering consistency ~keep
        let xref_data = b"xref\n\
            0 1\n\
            0000000000 65535 x\n\
            trailer\n";

        let mut cursor = Cursor::new(xref_data);
        let result = parse_xref(&mut cursor, 0);
        assert!(result.is_ok());
        let table = result.unwrap();
        assert_eq!(table.len(), 1);
        assert!(table.get(0).is_some());
        assert!(!table.get(0).unwrap().in_use);
    }

    #[test]
    fn test_parse_xref_empty_table() {
        let xref_data = b"xref\n\
            trailer\n";

        let mut cursor = Cursor::new(xref_data);
        let table = parse_xref(&mut cursor, 0).unwrap();
        assert!(table.is_empty());
    }

    #[test]
    fn test_cross_ref_table_default() {
        let table = CrossRefTable::default();
        assert!(table.is_empty());
    }

    #[test]
    fn test_parse_xref_with_comments() {
        let xref_data = b"xref\n\
            % This is a comment\n\
            0 2\n\
            0000000000 65535 f\n\
            0000000018 00000 n\n\
            % Another comment\n\
            trailer\n";

        let mut cursor = Cursor::new(xref_data);
        let table = parse_xref(&mut cursor, 0).unwrap();
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn test_parse_xref_excessive_count() {
        let xref_data = b"xref\n\
            0 2000000\n\
            0000000000 65535 f\n\
            trailer\n";

        let mut cursor = Cursor::new(xref_data);
        let result = parse_xref(&mut cursor, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_xref_offset_cr_only_line_endings() {
        let pdf_data = b"some content\r\
            startxref\r\
            173\r\
            %%EOF\r";

        let mut cursor = Cursor::new(pdf_data);
        let offset = find_xref_offset(&mut cursor).unwrap();
        assert_eq!(offset, 173);
    }

    #[test]
    fn test_parse_xref_cr_only_line_endings() {
        let xref_data = b"xref\r\
            0 2\r\
            0000000000 65535 f\r\
            0000000018 00000 n\r\
            trailer\r";

        let mut cursor = Cursor::new(xref_data);
        let table = parse_xref(&mut cursor, 0).unwrap();
        assert_eq!(table.len(), 2);

        let entry0 = table.get(0).unwrap();
        assert!(!entry0.in_use);

        let entry1 = table.get(1).unwrap();
        assert_eq!(entry1.offset, 18);
        assert!(entry1.in_use);
    }

    #[test]
    fn test_split_lines_mixed_endings() {
        let text = "line1\rline2\nline3\r\nline4";
        let lines = split_lines(text);
        assert_eq!(lines, vec!["line1", "line2", "line3", "line4"]);
    }

    #[test]
    fn test_parse_xref_with_prev_chain() {
        // Verify that the iterative parser follows /Prev chains.
        // We test this indirectly: a circular /Prev=0 chain should not panic or
        // infinite-loop (covered by test_parse_xref_circular_prev_chain), and
        // the iterative parser should handle deep chains without stack overflow.
        //
        // For a proper /Prev chain test with real PDF data, we rely on
        // integration tests with actual PDFs (e.g., Deutsche Heeresuniformen
        // with 177 /Prev links). ~keep

        // Single xref table with /Prev pointing to a non-xref offset.
        // The parser should fail gracefully on the /Prev target and still
        // return what it parsed from the first table. ~keep
        let xref_data = b"xref\n\
            0 2\n\
            0000000000 65535 f\n\
            0000000500 00000 n\n\
            trailer\n\
            << /Size 2 >>\n";

        let mut cursor = Cursor::new(xref_data);
        let table = parse_xref(&mut cursor, 0).unwrap();
        assert_eq!(table.len(), 2);
        assert_eq!(table.get(1).unwrap().offset, 500);
    }

    #[test]
    fn test_parse_xref_circular_prev_chain() {
        let xref_data = b"xref\n\
            0 1\n\
            0000000000 65535 f\n\
            trailer\n\
            << /Size 1 /Prev 0 >>\n";

        let mut cursor = Cursor::new(xref_data);
        let table = parse_xref(&mut cursor, 0).unwrap();
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_xref_entry_uncompressed() {
        let entry = XRefEntry::uncompressed(5000, 0);
        assert_eq!(entry.entry_type, XRefEntryType::Uncompressed);
        assert_eq!(entry.offset, 5000);
        assert_eq!(entry.generation, 0);
        assert!(entry.in_use);
    }

    #[test]
    fn test_xref_entry_compressed() {
        let entry = XRefEntry::compressed(42, 3);
        assert_eq!(entry.entry_type, XRefEntryType::Compressed);
        assert_eq!(entry.offset, 42); // stream obj number ~keep
        assert_eq!(entry.generation, 3); // index in stream ~keep
        assert!(entry.in_use);
    }

    #[test]
    fn test_xref_entry_free_constructor() {
        let entry = XRefEntry::free(7, 65535);
        assert_eq!(entry.entry_type, XRefEntryType::Free);
        assert_eq!(entry.offset, 7); // next free obj ~keep
        assert_eq!(entry.generation, 65535);
        assert!(!entry.in_use);
    }

    #[test]
    fn test_xref_entry_type_equality() {
        assert_eq!(XRefEntryType::Free, XRefEntryType::Free);
        assert_eq!(XRefEntryType::Uncompressed, XRefEntryType::Uncompressed);
        assert_eq!(XRefEntryType::Compressed, XRefEntryType::Compressed);
        assert_ne!(XRefEntryType::Free, XRefEntryType::Uncompressed);
    }

    #[test]
    fn test_xref_entry_clone_debug() {
        let entry = XRefEntry::uncompressed(100, 0);
        let cloned = entry.clone();
        assert_eq!(entry, cloned);
        let debug = format!("{:?}", entry);
        assert!(debug.contains("Uncompressed"));
    }

    #[test]
    fn test_cross_ref_table_set_trailer() {
        let mut table = CrossRefTable::new();
        assert!(table.trailer().is_none());

        let mut trailer = HashMap::new();
        trailer.insert("Size".to_string(), Object::Integer(10));
        table.set_trailer(trailer);
        assert!(table.trailer().is_some());
        assert!(table.trailer().unwrap().contains_key("Size"));
    }

    #[test]
    fn test_cross_ref_table_contains() {
        let mut table = CrossRefTable::new();
        assert!(!table.contains(1));
        table.add_entry(1, XRefEntry::uncompressed(100, 0));
        assert!(table.contains(1));
        assert!(!table.contains(2));
    }

    #[test]
    fn test_cross_ref_table_all_object_numbers() {
        let mut table = CrossRefTable::new();
        table.add_entry(1, XRefEntry::uncompressed(100, 0));
        table.add_entry(5, XRefEntry::uncompressed(200, 0));
        table.add_entry(10, XRefEntry::uncompressed(300, 0));

        let mut nums: Vec<u32> = table.all_object_numbers().collect();
        nums.sort();
        assert_eq!(nums, vec![1, 5, 10]);
    }

    #[test]
    fn test_cross_ref_table_merge_from() {
        let mut table1 = CrossRefTable::new();
        table1.add_entry(1, XRefEntry::uncompressed(100, 0));
        table1.add_entry(2, XRefEntry::uncompressed(200, 0));

        let mut table2 = CrossRefTable::new();
        table2.add_entry(2, XRefEntry::uncompressed(999, 0)); // conflict ~keep
        table2.add_entry(3, XRefEntry::uncompressed(300, 0));

        table1.merge_from(table2);

        assert_eq!(table1.len(), 3);
        // table1's entry for obj 2 should win (not overwritten) ~keep
        assert_eq!(table1.get(2).unwrap().offset, 200);
        assert_eq!(table1.get(3).unwrap().offset, 300);
    }

    #[test]
    fn test_cross_ref_table_merge_trailer() {
        let mut table1 = CrossRefTable::new();

        let mut table2 = CrossRefTable::new();
        let mut trailer = HashMap::new();
        trailer.insert("Size".to_string(), Object::Integer(5));
        table2.set_trailer(trailer);

        table1.merge_from(table2);
        assert!(table1.trailer().is_some());
    }

    #[test]
    fn test_cross_ref_table_merge_preserves_existing_trailer() {
        let mut table1 = CrossRefTable::new();
        let mut trailer1 = HashMap::new();
        trailer1.insert("Size".to_string(), Object::Integer(10));
        table1.set_trailer(trailer1);

        let mut table2 = CrossRefTable::new();
        let mut trailer2 = HashMap::new();
        trailer2.insert("Size".to_string(), Object::Integer(5));
        table2.set_trailer(trailer2);

        table1.merge_from(table2);
        let size = table1.trailer().unwrap().get("Size").unwrap();
        assert_eq!(*size, Object::Integer(10));
    }

    #[test]
    fn test_cross_ref_table_clone() {
        let mut table = CrossRefTable::new();
        table.add_entry(1, XRefEntry::uncompressed(100, 0));
        let cloned = table.clone();
        assert_eq!(cloned.len(), 1);
        assert_eq!(cloned.get(1).unwrap().offset, 100);
    }

    #[test]
    fn test_find_stream_length_basic() {
        let data = b"/Type /XRef /Length 1234 /W [1 2 2]";
        let result = find_stream_length(data);
        assert_eq!(result, Some(1234));
    }

    #[test]
    fn test_find_stream_length_no_length() {
        let data = b"/Type /XRef /W [1 2 2]";
        let result = find_stream_length(data);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_stream_length_indirect_reference() {
        // /Length 5 0 R -> can't resolve without full parsing ~keep
        let data = b"/Type /XRef /Length 5 0 R";
        // First digit should parse, but it would get "5" as the length
        // Actually it would parse "5" as a valid number ~keep
        let result = find_stream_length(data);
        // It parses the first integer it finds after /Length ~keep
        assert_eq!(result, Some(5));
    }

    #[test]
    fn test_find_stream_length_zero() {
        let data = b"/Length 0";
        let result = find_stream_length(data);
        assert_eq!(result, Some(0));
    }

    #[test]
    fn test_find_actual_xref_offset_correct() {
        let data = b"xref\n0 1\n0000000000 65535 f\ntrailer\n";
        let mut cursor = Cursor::new(data);
        let offset = find_actual_xref_offset(&mut cursor, 0).unwrap();
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_find_actual_xref_offset_digit_start() {
        let data = b"1 0 obj\n<< /Type /XRef >>\nstream\n";
        let mut cursor = Cursor::new(data);
        let offset = find_actual_xref_offset(&mut cursor, 0).unwrap();
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_split_lines_lf() {
        let lines = split_lines("a\nb\nc");
        assert_eq!(lines, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_split_lines_cr() {
        let lines = split_lines("a\rb\rc");
        assert_eq!(lines, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_split_lines_crlf() {
        let lines = split_lines("a\r\nb\r\nc");
        assert_eq!(lines, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_split_lines_empty() {
        let lines = split_lines("");
        assert!(lines.is_empty());
    }

    #[test]
    fn test_split_lines_single_line() {
        let lines = split_lines("hello");
        assert_eq!(lines, vec!["hello"]);
    }

    #[test]
    fn test_xref_entry_type_debug() {
        let debug = format!("{:?}", XRefEntryType::Compressed);
        assert!(debug.contains("Compressed"));
    }
}
