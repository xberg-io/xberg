use byteorder::{BigEndian, ReadBytesExt};
/// TrueType cmap table extraction for font character mapping
///
/// This module extracts Unicode mappings from TrueType font cmap tables,
/// providing a fallback for Type0 fonts missing ToUnicode CMaps.
///
/// The cmap table maps glyph IDs (GIDs) to Unicode code points.
/// We support formats 4 (BMP), 6 (trimmed), and 12 (Unicode full).
use std::collections::HashMap;
use std::io::Cursor;

/// Mac Roman (platform 1, encoding 0) high-half → Unicode mapping.
///
/// Bytes 0x00..0x7F are identical to ASCII and are handled by the caller.
/// This table covers 0x80..0xFF per Apple's Mac Roman → Unicode reference.
const MAC_ROMAN_HIGH: [char; 128] = [
    'Ä', 'Å', 'Ç', 'É', 'Ñ', 'Ö', 'Ü', 'á', 'à', 'â', 'ä', 'ã', 'å', 'ç', 'é', 'è', 'ê', 'ë', 'í', 'ì', 'î', 'ï', 'ñ',
    'ó', 'ò', 'ô', 'ö', 'õ', 'ú', 'ù', 'û', 'ü', '†', '°', '¢', '£', '§', '•', '¶', 'ß', '®', '©', '™', '´', '¨', '≠',
    'Æ', 'Ø', '∞', '±', '≤', '≥', '¥', 'µ', '∂', '∑', '∏', 'π', '∫', 'ª', 'º', 'Ω', 'æ', 'ø', '¿', '¡', '¬', '√', 'ƒ',
    '≈', '∆', '«', '»', '…', '\u{00A0}', 'À', 'Ã', 'Õ', 'Œ', 'œ', '–', '—', '"', '"', '\'', '\'', '÷', '◊', 'ÿ', 'Ÿ',
    '⁄', '€', '‹', '›', 'ﬁ', 'ﬂ', '‡', '·', '‚', '„', '‰', 'Â', 'Ê', 'Á', 'Ë', 'È', 'Í', 'Î', 'Ï', 'Ì', 'Ó', 'Ô',
    '\u{F8FF}', 'Ò', 'Ú', 'Û', 'Ù', 'ı', 'ˆ', '˜', '¯', '˘', '˙', '˚', '¸', '˝', '˛', 'ˇ',
];

#[inline]
fn mac_roman_to_unicode(byte: u8) -> char {
    debug_assert!(byte >= 0x80);
    MAC_ROMAN_HIGH[(byte - 0x80) as usize]
}

/// One format-4 cmap segment's per-char_code resolution context, bundled to
/// keep `resolve_cmap_format4_gid`'s parameter count within the
/// repository's guideline. ~keep
struct Format4Segment {
    start: u32,
    seg: usize,
    seg_count: usize,
    id_delta: i32,
    id_range_offset: u16,
}

/// Represents a TrueType cmap table extracted from an embedded font
#[derive(Debug, Clone)]
pub struct TrueTypeCMap {
    /// Mapping from Glyph ID to Unicode character
    gid_to_unicode: HashMap<u16, char>,
    /// Content-byte → GID, from a `(3,0)`/`(1,0)` subtable. Empty unless the
    /// font is symbolic-style; lets decode do byte→GID before GID→Unicode.
    symbol_code_to_gid: HashMap<u32, u16>,
}

impl TrueTypeCMap {
    /// Parse TrueType cmap table from font data
    ///
    /// The TrueType sfnt structure contains a directory of tables.
    /// We locate the 'cmap' table and parse the best available subtable.
    ///
    /// Priority for cmap subtables:
    /// 1. Platform 3 (Windows), Encoding 10 (Unicode full repertoire) - supports all Unicode
    /// 2. Platform 3 (Windows), Encoding 1 (Unicode BMP) - supports basic multilingual plane
    /// 3. Platform 0 (Unicode), Encoding 3 - fallback to old Unicode platform
    pub fn from_font_data(data: &[u8]) -> Result<Self, String> {
        let mut cursor = Cursor::new(data);

        let (num_tables, search_range, entry_selector, range_shift) = Self::parse_sfnt_header(&mut cursor)?;

        let cmap_offset = Self::find_cmap_table(&mut cursor, num_tables, search_range, entry_selector, range_shift)?;

        cursor.set_position(cmap_offset as u64);
        let cmap_version = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read cmap version: {}", e))?;

        if cmap_version != 0 {
            return Err(format!("Unsupported cmap table version: {}", cmap_version));
        }

        let num_subtables = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read cmap subtable count: {}", e))?;

        let mut best_subtable: Option<(u32, u32, u32)> = None;
        let mut best_priority = -1i32;

        for _ in 0..num_subtables {
            let platform_id = cursor
                .read_u16::<BigEndian>()
                .map_err(|e| format!("Failed to read platform ID: {}", e))?;
            let encoding_id = cursor
                .read_u16::<BigEndian>()
                .map_err(|e| format!("Failed to read encoding ID: {}", e))?;
            let offset = cursor
                .read_u32::<BigEndian>()
                .map_err(|e| format!("Failed to read subtable offset: {}", e))?;

            let priority = match (platform_id, encoding_id) {
                (3, 10) => 30, // Windows, Unicode full repertoire ~keep
                (3, 1) => 20,  // Windows, Unicode BMP ~keep
                (0, 3) => 10,  // Unicode platform, Unicode 2.0 ~keep
                _ => 0,
            };

            if priority > best_priority {
                best_priority = priority;
                best_subtable = Some((platform_id as u32, encoding_id as u32, offset));
            }
        }

        let (platform_id, encoding_id, subtable_offset) =
            best_subtable.ok_or_else(|| "No suitable cmap subtable found".to_string())?;

        tracing::debug!(
            "TrueType cmap: selected platform={} encoding={} offset={}",
            platform_id,
            encoding_id,
            subtable_offset
        );

        // The subtable offset is attacker-controlled: reject anything past
        // the end of the font data, computed in u64 so the sum cannot
        // overflow. A checked u32 add would only remove the debug panic
        // while release silently wrapped and walked garbage. ~keep
        let subtable_start = u64::from(cmap_offset) + u64::from(subtable_offset);
        if subtable_start >= data.len() as u64 {
            return Err(format!(
                "cmap subtable offset {} out of bounds (font data length {})",
                subtable_offset,
                data.len()
            ));
        }

        cursor.set_position(subtable_start);
        let gid_to_unicode = Self::parse_cmap_subtable(&mut cursor)?;

        Ok(TrueTypeCMap {
            gid_to_unicode,
            // Symbolic fonts index glyphs by content byte, not GID — build
            // byte→GID from the (3,0)/(1,0) subtable so decode can do that hop. ~keep
            symbol_code_to_gid: Self::build_symbol_code_to_gid(data),
        })
    }

    /// Build a content-byte→GID map from the font's `(3,0)` symbol or `(1,0)`
    /// Macintosh cmap subtable, resolving the `0xF000` symbol-PUA offset at build
    /// time. Reuses `ttf-parser` (already a dependency). Empty when the font has
    /// no such subtable or fails to parse — decode then treats the byte as a GID.
    fn build_symbol_code_to_gid(data: &[u8]) -> HashMap<u32, u16> {
        use skrifa::raw::{TableProvider, tables::cmap::PlatformId};

        let mut map = HashMap::new();
        let Ok(font) = skrifa::raw::FontRef::new(data) else {
            return map;
        };
        let Ok(cmap) = font.cmap() else {
            return map;
        };

        // (3,0) is the Windows symbol encoding and wins outright; (1,0) Mac
        // Roman is the fallback. Anything else is not a byte-indexed cmap and
        // is ignored — the caller then treats the content byte as a GID. ~keep
        let mut chosen = None;
        let mut best = 0;
        for record in cmap.encoding_records() {
            let priority = match (record.platform_id(), record.encoding_id()) {
                (PlatformId::Windows, 0) => 2,
                (PlatformId::Macintosh, 0) => 1,
                _ => 0,
            };
            if priority > best
                && let Ok(subtable) = record.subtable(cmap.offset_data())
            {
                best = priority;
                chosen = Some(subtable);
            }
        }

        if let Some(subtable) = chosen {
            for code in 0u32..256 {
                // Symbol fonts address their glyphs in the F000..F0FF private-use
                // block, so a bare byte misses and the offset form hits. ~keep
                if let Some(gid) = subtable
                    .map_codepoint(code)
                    .or_else(|| subtable.map_codepoint(0xF000 | code))
                {
                    map.insert(code, gid.to_u32() as u16);
                }
            }
        }
        map
    }

    /// Content-byte → GID via the font's symbol/Mac cmap. `None` when the font
    /// has no such subtable (decode then falls back to treating the byte as a GID).
    pub fn code_to_gid(&self, code: u16) -> Option<u16> {
        self.symbol_code_to_gid.get(&(code as u32)).copied()
    }

    /// Get Unicode character for a glyph ID
    pub fn get_unicode(&self, gid: u16) -> Option<char> {
        self.gid_to_unicode.get(&gid).copied()
    }

    /// Get the number of glyph mappings
    pub fn len(&self) -> usize {
        self.gid_to_unicode.len()
    }

    /// Check if cmap is empty
    pub fn is_empty(&self) -> bool {
        self.gid_to_unicode.is_empty()
    }

    fn parse_sfnt_header(cursor: &mut Cursor<&[u8]>) -> Result<(u16, u16, u16, u16), String> {
        let version = cursor
            .read_u32::<BigEndian>()
            .map_err(|e| format!("Failed to read sfnt version: {}", e))?;

        // 0x00010000 = TrueType, 0x4F54544F = OpenType (OTTO), 0x74727565 = Apple TrueType ("true")
        // ~keep
        if version != 0x00010000 && version != 0x4F54544F && version != 0x74727565 {
            return Err(format!("Invalid sfnt version: 0x{:08X}", version));
        }

        let num_tables = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read table count: {}", e))?;
        let search_range = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read search range: {}", e))?;
        let entry_selector = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read entry selector: {}", e))?;
        let range_shift = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read range shift: {}", e))?;

        Ok((num_tables, search_range, entry_selector, range_shift))
    }

    fn find_cmap_table(
        cursor: &mut Cursor<&[u8]>,
        num_tables: u16,
        _search_range: u16,
        _entry_selector: u16,
        _range_shift: u16,
    ) -> Result<u32, String> {
        // Linear search through table directory for 'cmap' tag (0x636D6170) ~keep
        const CMAP_TAG: u32 = 0x636D6170;

        for _ in 0..num_tables {
            let tag = cursor
                .read_u32::<BigEndian>()
                .map_err(|e| format!("Failed to read table tag: {}", e))?;
            let _checksum = cursor
                .read_u32::<BigEndian>()
                .map_err(|e| format!("Failed to read table checksum: {}", e))?;
            let offset = cursor
                .read_u32::<BigEndian>()
                .map_err(|e| format!("Failed to read table offset: {}", e))?;
            let _length = cursor
                .read_u32::<BigEndian>()
                .map_err(|e| format!("Failed to read table length: {}", e))?;

            if tag == CMAP_TAG {
                return Ok(offset);
            }
        }

        Err("cmap table not found in font".to_string())
    }

    fn parse_cmap_subtable(cursor: &mut Cursor<&[u8]>) -> Result<HashMap<u16, char>, String> {
        let format = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read cmap format: {}", e))?;

        match format {
            0 => Self::parse_cmap_format0(cursor),
            4 => Self::parse_cmap_format4(cursor),
            6 => Self::parse_cmap_format6(cursor),
            12 => Self::parse_cmap_format12(cursor),
            _ => Err(format!("Unsupported cmap format: {}", format)),
        }
    }

    /// Parse cmap format 0 (legacy 1-byte indexed, Mac Roman era).
    ///
    /// Structure per Apple TrueType reference (subtable length 262):
    ///   u16 format       (already read by caller — 2 bytes)
    ///   u16 length       (262: 2+2+2+256)
    ///   u16 language
    ///   u8  glyphIdArray[256]   — glyphId for each byte code 0..255
    ///
    /// Microsoft Office subset fonts (Calibri, Times New Roman subsets in
    /// Word/Excel exports) still ship with a format-0 cmap for the `(1,0)`
    /// Macintosh encoding alongside their `(3,1)` Unicode cmap. When the
    /// Unicode cmap is missing or malformed we fall back to this one; the
    /// byte code acts as the Mac Roman character code.
    ///
    /// Benchmark canary: ~8 MS Office corpus fixtures previously logged
    /// "Unsupported cmap format: 0" warnings and lost font glyph mapping
    /// as a consequence (B9).
    fn parse_cmap_format0(cursor: &mut Cursor<&[u8]>) -> Result<HashMap<u16, char>, String> {
        let _length = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read format 0 length: {}", e))?;
        let _language = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read format 0 language: {}", e))?;

        let mut glyph_ids = [0u8; 256];
        std::io::Read::read_exact(cursor, &mut glyph_ids)
            .map_err(|e| format!("Failed to read format 0 glyphIdArray: {}", e))?;

        let mut gid_to_unicode = HashMap::new();
        for (byte, &gid) in glyph_ids.iter().enumerate() {
            if gid == 0 {
                continue;
            }
            // ASCII pass-through for 0x00..0x7F (Mac Roman lower half is
            // identical to ASCII). Above 0x7F we route through the Mac
            // Roman → Unicode table so byte 0x8A (a-umlaut in Mac Roman)
            // maps to U+00E4 instead of U+008A. ~keep
            let ch = if byte < 0x80 {
                char::from_u32(byte as u32)
            } else {
                Some(mac_roman_to_unicode(byte as u8))
            };
            if let Some(ch) = ch {
                gid_to_unicode.insert(gid as u16, ch);
            }
        }
        Ok(gid_to_unicode)
    }

    /// Parse cmap format 4 (BMP - supports characters U+0000 to U+FFFF)
    fn parse_cmap_format4(cursor: &mut Cursor<&[u8]>) -> Result<HashMap<u16, char>, String> {
        let start = cursor.position() as usize - 2;
        let length = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read format 4 length: {e}"))? as usize;
        const HEADER_BYTES: usize = 16;
        const SEGMENT_BYTES: usize = 8;
        if length < HEADER_BYTES || length % 2 != 0 {
            return Err("Invalid cmap format 4 subtable length".into());
        }
        let data = cursor
            .get_ref()
            .get(start..start.saturating_add(length))
            .ok_or("Truncated cmap format 4 subtable")?;
        let mut subtable = Cursor::new(&data[4..]);
        let seg_count = Self::read_cmap_format4_header(&mut subtable)?;
        if seg_count > (length - HEADER_BYTES) / SEGMENT_BYTES {
            return Err("Cmap format 4 segment arrays exceed subtable length".into());
        }
        let (end_codes, start_codes, id_deltas, id_range_offsets, glyph_id_array) =
            Self::read_cmap_format4_segments(&mut subtable, seg_count)?;
        Self::validate_cmap_format4_segments(&start_codes, &end_codes, u16::MAX as usize)?;
        Ok(Self::build_cmap_format4_gid_map(
            seg_count,
            &end_codes,
            &start_codes,
            &id_deltas,
            &id_range_offsets,
            &glyph_id_array,
        ))
    }

    /// Read the format-4 subtable header and return `seg_count`
    /// (`segCountX2 / 2`) after the format and length fields.
    fn read_cmap_format4_header(cursor: &mut Cursor<&[u8]>) -> Result<usize, String> {
        let _language = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read format 4 language: {}", e))?;

        let seg_count_x2 = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read segCountX2: {}", e))? as usize;
        if seg_count_x2 == 0 || seg_count_x2 % 2 != 0 {
            return Err("Invalid cmap format 4 segCountX2".into());
        }
        let seg_count = seg_count_x2 / 2;

        let _search_range = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read searchRange: {}", e))?;
        let _entry_selector = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read entrySelector: {}", e))?;
        let _range_shift = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read rangeShift: {}", e))?;

        Ok(seg_count)
    }

    /// Read the format-4 subtable's parallel segment arrays
    /// (endCode/startCode/idDelta/idRangeOffset) plus the trailing
    /// glyphIdArray. Split out of `parse_cmap_format4` purely to keep that
    /// function within the repository's line-length guideline; behavior is
    /// unchanged. ~keep
    #[allow(clippy::type_complexity)]
    fn read_cmap_format4_segments(
        cursor: &mut Cursor<&[u8]>,
        seg_count: usize,
    ) -> Result<(Vec<u16>, Vec<u16>, Vec<i16>, Vec<u16>, Vec<u16>), String> {
        let mut end_codes = vec![0u16; seg_count];
        for i in 0..seg_count {
            end_codes[i] = cursor
                .read_u16::<BigEndian>()
                .map_err(|e| format!("Failed to read endCode[{}]: {}", i, e))?;
        }

        let reserved = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read reserved pad: {}", e))?;

        if reserved != 0 {
            return Err("Nonzero cmap format 4 reserved pad".into());
        }

        let mut start_codes = vec![0u16; seg_count];
        for i in 0..seg_count {
            start_codes[i] = cursor
                .read_u16::<BigEndian>()
                .map_err(|e| format!("Failed to read startCode[{}]: {}", i, e))?;
        }

        let mut id_deltas = vec![0i16; seg_count];
        for i in 0..seg_count {
            id_deltas[i] = cursor
                .read_i16::<BigEndian>()
                .map_err(|e| format!("Failed to read idDelta[{}]: {}", i, e))?;
        }

        let mut id_range_offsets = vec![0u16; seg_count];
        for i in 0..seg_count {
            id_range_offsets[i] = cursor
                .read_u16::<BigEndian>()
                .map_err(|e| format!("Failed to read idRangeOffset[{}]: {}", i, e))?;
        }

        let mut glyph_id_array = Vec::new();
        while let Ok(val) = cursor.read_u16::<BigEndian>() {
            glyph_id_array.push(val);
        }

        Ok((end_codes, start_codes, id_deltas, id_range_offsets, glyph_id_array))
    }

    fn validate_cmap_format4_segments(starts: &[u16], ends: &[u16], mut remaining: usize) -> Result<(), String> {
        if ends.last() != Some(&u16::MAX) {
            return Err("Missing cmap format 4 terminal endCode".into());
        }
        let mut previous_end = None;
        for (&start, &end) in starts.iter().zip(ends) {
            if start > end || previous_end.is_some_and(|previous| start <= previous) {
                return Err("Cmap format 4 segments must be ordered and non-overlapping".into());
            }
            previous_end = Some(end);
            // The mapping walk stops before the terminal code. ~keep
            let width = usize::from(end) - usize::from(start) + usize::from(end != u16::MAX);
            remaining = remaining
                .checked_sub(width)
                .ok_or("Cmap format 4 iteration budget exhausted")?;
        }
        Ok(())
    }

    /// Build the gid→Unicode map from a format-4 subtable's parsed
    /// segments. Split out of `parse_cmap_format4` purely to keep that
    /// function within the repository's line-length guideline; behavior is
    /// unchanged. ~keep
    fn build_cmap_format4_gid_map(
        seg_count: usize,
        end_codes: &[u16],
        start_codes: &[u16],
        id_deltas: &[i16],
        id_range_offsets: &[u16],
        glyph_id_array: &[u16],
    ) -> HashMap<u16, char> {
        let mut gid_to_unicode = HashMap::new();

        for seg in 0..seg_count {
            let start = start_codes[seg] as u32;
            let end = end_codes[seg] as u32;
            let id_delta = id_deltas[seg] as i32;

            for char_code in start..=end {
                if char_code == 0xFFFF {
                    break; // End segment marker ~keep
                }

                let segment = Format4Segment {
                    start,
                    seg,
                    seg_count,
                    id_delta,
                    id_range_offset: id_range_offsets[seg],
                };
                let gid = Self::resolve_cmap_format4_gid(char_code, &segment, glyph_id_array);

                if gid != 0
                    && let Some(ch) = char::from_u32(char_code)
                {
                    gid_to_unicode.insert(gid, ch);
                }
            }
        }

        gid_to_unicode
    }

    /// Resolve one format-4 char_code → GID, either directly via idDelta or
    /// through glyphIdArray. Split out of `parse_cmap_format4` purely to
    /// keep that function within the repository's line-length guideline;
    /// behavior is unchanged. ~keep
    fn resolve_cmap_format4_gid(char_code: u32, segment: &Format4Segment, glyph_id_array: &[u16]) -> u16 {
        if segment.id_range_offset == 0 {
            return (char_code as i32 + segment.id_delta) as u16;
        }
        // Per TrueType spec: index into glyphIdArray
        // offset = idRangeOffset[i]/2 + (charCode - startCode[i]) + i - segCount ~keep
        // A malformed subtable can push the value below zero before the
        // segCount subtraction; underflow resolves to nothing, exactly like
        // the out-of-bounds case (release's wrapped huge index always failed
        // the bounds check and returned 0). ~keep
        let index =
            (segment.id_range_offset as usize) / 2 + (char_code as usize - segment.start as usize) + segment.seg;
        let Some(offset) = index.checked_sub(segment.seg_count) else {
            return 0;
        };
        if offset < glyph_id_array.len() {
            let raw = glyph_id_array[offset];
            if raw != 0 {
                (raw as i32 + segment.id_delta) as u16
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Parse cmap format 6 (trimmed table)
    fn parse_cmap_format6(cursor: &mut Cursor<&[u8]>) -> Result<HashMap<u16, char>, String> {
        let _length = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read format 6 length: {}", e))?;
        let _language = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read format 6 language: {}", e))?;

        let first_code = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read firstCode: {}", e))?;
        let count = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read entryCount: {}", e))? as usize;

        let mut gid_to_unicode = HashMap::new();

        for i in 0..count {
            let gid = cursor
                .read_u16::<BigEndian>()
                .map_err(|e| format!("Failed to read glyphId[{}]: {}", i, e))?;

            let char_code = first_code as u32 + i as u32;
            if let Some(ch) = char::from_u32(char_code) {
                gid_to_unicode.insert(gid, ch);
            }
        }

        Ok(gid_to_unicode)
    }

    /// Parse cmap format 12 (segmented coverage - supports full Unicode)
    fn parse_cmap_format12(cursor: &mut Cursor<&[u8]>) -> Result<HashMap<u16, char>, String> {
        let _reserved = cursor
            .read_u16::<BigEndian>()
            .map_err(|e| format!("Failed to read reserved: {}", e))?;

        let _length = cursor
            .read_u32::<BigEndian>()
            .map_err(|e| format!("Failed to read format 12 length: {}", e))?;
        let _language = cursor
            .read_u32::<BigEndian>()
            .map_err(|e| format!("Failed to read format 12 language: {}", e))?;

        let num_groups = cursor
            .read_u32::<BigEndian>()
            .map_err(|e| format!("Failed to read numGroups: {}", e))? as usize;

        let mut gid_to_unicode = HashMap::new();
        let mut remaining = MAX_CMAP_EXAMINED_CODE_POINTS;

        for _ in 0..num_groups {
            let start_char_code = cursor
                .read_u32::<BigEndian>()
                .map_err(|e| format!("Failed to read startCharCode: {}", e))?;
            let end_char_code = cursor
                .read_u32::<BigEndian>()
                .map_err(|e| format!("Failed to read endCharCode: {}", e))?;
            let start_gid = cursor
                .read_u32::<BigEndian>()
                .map_err(|e| format!("Failed to read startGlyphId: {}", e))?;

            // GH#3034: a crafted endCharCode can reach 0xFFFFFFFF, whose
            // walk costs ~4.3 billion iterations per group. Clamp the walk
            // to Unicode's maximum; a group whose start lies past it (or
            // whose end is below its start) maps nothing and is skipped
            // without iterating — and without spending any of the budget,
            // since code points past U+10FFFF are not characters and can
            // never be examined. ~keep
            let end_char_code = end_char_code.min(char::MAX as u32);
            if end_char_code < start_char_code {
                continue;
            }

            // The budget is spent by total examined code points, not by
            // distinct mappings: a single 65,535-wide group fills the gid
            // map (short of the map's own capacity, since surrogate code
            // points are never inserted), so a mapping-count cap lets
            // ~87k further overlapping groups re-walk the same gids —
            // about 5.7 billion iterations (GH#3034 review). Charging each
            // group's full clamped width up front bounds a subtable's work
            // at the budget itself; a subtable that exceeds it is rejected
            // rather than silently truncated, so the font falls back to
            // its declared encoding exactly as a malformed cmap does.
            let width = (end_char_code - start_char_code + 1) as usize;
            if width > remaining {
                return Err(format!(
                    "cmap format 12 group of {} code points exceeds the remaining iteration budget ({})",
                    width, remaining
                ));
            }
            remaining -= width;

            for (offset, char_code) in (start_char_code..=end_char_code).enumerate() {
                // A malformed group's startGlyphId can overflow the u32 sum;
                // wrapping matches release behaviour exactly (the `as u16` cast
                // then truncates the same low 16 bits). ~keep
                let gid = start_gid.wrapping_add(offset as u32) as u16;
                if let Some(ch) = char::from_u32(char_code) {
                    gid_to_unicode.insert(gid, ch);
                }
            }
        }

        Ok(gid_to_unicode)
    }
}

/// Upper bound on the code points one cmap subtable may examine, as a total
/// across its groups (each group's clamped width is charged against it up
/// front). A well-formed font may alias code points: two or more groups can
/// map overlapping code-point runs onto gids a previous group already
/// mapped, so a subtable's group widths can legitimately sum to a multiple
/// of the glyph count even though every gid is mapped once (GH#3064) — a
/// budget at the glyph space itself rejects such fonts and loses their
/// whole cmap to the declared-encoding fallback. The bound is therefore
/// 16 times the glyph space (`numGlyphs` is a u16): far above any sane
/// aliasing (a face's mapped code points cannot exceed its glyph count
/// times its aliasing factor, typically one), still walked in
/// milliseconds, and still rejecting the crafted unbounded groups of
/// GH#3034 — ~87k overlapping 65,535-wide groups need ~5.7 billion
/// examined code points, orders of magnitude past this budget. Tighter
/// than the core crate's `SecurityLimits::max_iterations` work budget
/// (10M), which lives in a crate this one does not depend on.
const MAX_CMAP_EXAMINED_CODE_POINTS: usize = (u16::MAX as usize + 1) * 16;

#[cfg(test)]
mod tests;
