//! Embedded charcode → CID CMap stream parser.
//!
//! ISO 32000-1:2008 §9.7.5.2: when a Type0 font's `/Encoding` is a CMap
//! *stream* (rather than a predefined name like `Identity-H`), the stream is
//! a small PostScript-like program declaring:
//!
//! - `begincodespacerange` / `endcodespacerange` — the valid domain of
//!   character codes AND how many bytes each occupies. A codespace can be
//!   variable-width (e.g. a Shift-JIS-flavoured CMap declaring both 1-byte
//!   and 2-byte ranges); [`CidCMap::code_length`] resolves this per code.
//! - `begincidrange` / `endcidrange` — `<lo> <hi> cid` triples: codes
//!   `lo..=hi` map to `cid, cid+1, cid+2, …`.
//! - `begincidchar` / `endcidchar` — `<code> cid` pairs: a single code maps
//!   to a single CID.
//!
//! This is a *different* mapping from [`super::cmap::CMap`] (`/ToUnicode`:
//! charcode → Unicode string) even though the container syntax is nearly
//! identical — cidrange/cidchar targets are bare decimal integers, not hex
//! strings, and there is no `bfrange`-style array form. The shared
//! `begin…end` block tokenizer (`extract_sections`, `significant_lines`) is
//! reused from `cmap.rs` rather than duplicated (see GH #1631).
//!
//! `usecmap` (a CMap extending a base CMap by reference) is not resolved —
//! an `/Encoding` stream containing only `usecmap` with no inline
//! `cidrange`/`cidchar` data parses to an empty, mapping-less [`CidCMap`],
//! which callers treat the same as "no embedded CID map" and fall back to
//! the honest degraded path (see `FontInfo::code_to_cid`).

use crate::error::Result;
use std::collections::HashMap;

use super::cmap::{extract_sections, significant_lines};

/// One `begincodespacerange` entry: character codes `low..=high` (inclusive,
/// big-endian) occupy `n_bytes` bytes each (Adobe CMap & CIDFont Files Spec
/// §7.3, ISO 32000-1 §9.7.6.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CodespaceRange {
    low: u32,
    high: u32,
    n_bytes: u8,
}

/// One `begincidrange` entry: codes `start..=end` map to `start_cid,
/// start_cid + 1, …`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CidRangeEntry {
    start: u32,
    end: u32,
    start_cid: u16,
}

/// Parsed charcode → CID mapping from an embedded `/Encoding` CMap stream.
///
/// `Default` (empty) is a legitimate value: it means the stream declared no
/// codespace/cidrange/cidchar data this parser recognised (e.g. a bare
/// `usecmap` reference), and callers must treat it exactly like "no embedded
/// CID map" rather than as CID 0 for every code.
///
/// `pub` only so it can appear in the type of the `pub`
/// [`FontInfo::embedded_cid_map`](super::font_dict::FontInfo::embedded_cid_map)
/// field (every `FontInfo` field is `pub`, including this one, for
/// struct-literal construction in this crate's own integration tests).
/// Every constructor and lookup method stays `pub(crate)`, and every field
/// stays private — external code can hold, clone, `Debug`-print, or
/// compare a value of this type, or set the field to `None`, but cannot
/// construct a meaningful instance or read anything out of one.
///
/// `#[doc(hidden)]`: NOT part of this published crate's public API or
/// semver contract — it is reachable only because Rust requires a `pub`
/// field's type to be at least as visible as the field itself.
#[doc(hidden)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CidCMap {
    chars: HashMap<u32, u16>,
    ranges: Vec<CidRangeEntry>,
    codespace: Vec<CodespaceRange>,
}

impl CidCMap {
    /// `true` when nothing was parsed — no cidchar/cidrange entries at all.
    /// A codespace with no CID data is still "empty" for lookup purposes.
    pub(crate) fn is_empty(&self) -> bool {
        self.chars.is_empty() && self.ranges.is_empty()
    }

    /// Resolve a character code to its CID.
    ///
    /// 1. `chars` (`begincidchar`) — O(1) exact match.
    /// 2. `ranges` (`begincidrange`) — linear scan. Embedded CID CMaps seen
    ///    in practice have at most a few hundred ranges, so the simplicity
    ///    of a linear scan outweighs binary search's bookkeeping here.
    /// 3. `None` when the code is in neither — the caller (`FontInfo::
    ///    code_to_cid`) decides the fallback (CID 0 / `.notdef`, letting
    ///    `/DW` apply, rather than guessing).
    pub(crate) fn lookup(&self, code: u32) -> Option<u16> {
        if let Some(&cid) = self.chars.get(&code) {
            return Some(cid);
        }
        self.ranges.iter().find_map(|r| {
            if code < r.start || code > r.end {
                return None;
            }
            let offset = u16::try_from(code - r.start).ok()?;
            Some(r.start_cid.wrapping_add(offset))
        })
    }

    /// The distinct byte-widths declared by `begincodespacerange`, sorted
    /// ascending and deduplicated. Empty when no codespace was declared.
    pub(crate) fn codespace_widths(&self) -> Vec<u8> {
        let mut widths: Vec<u8> = self.codespace.iter().map(|r| r.n_bytes).collect();
        widths.sort_unstable();
        widths.dedup();
        widths
    }

    /// Number of bytes the code starting at `bytes[pos]` occupies, per the
    /// declared codespace (ISO 32000-1 §9.7.6.2).
    ///
    /// Tries each declared width in ascending order and returns the first
    /// one whose leading bytes fall inside a codespace range of that width —
    /// this is what makes a mixed-width codespace (e.g. 1-byte ASCII plus
    /// 2-byte CJK) resolve per-code rather than as one fixed width for the
    /// whole stream. Falls back to the narrowest declared width (clamped to
    /// the bytes actually remaining) when no range matches, so a single
    /// out-of-gamut or malformed code cannot stall the scan.
    ///
    /// Returns `0` when `pos` is already past the end of `bytes`.
    pub(crate) fn code_length(&self, bytes: &[u8], pos: usize) -> usize {
        let remaining = bytes.len().saturating_sub(pos);
        if remaining == 0 {
            return 0;
        }
        let widths = self.codespace_widths();
        if widths.is_empty() {
            return 1;
        }
        for width in &widths {
            let w = *width as usize;
            if w > remaining {
                continue;
            }
            let mut code: u32 = 0;
            for &b in &bytes[pos..pos + w] {
                code = (code << 8) | u32::from(b);
            }
            let matches = self
                .codespace
                .iter()
                .any(|r| r.n_bytes == *width && code >= r.low && code <= r.high);
            if matches {
                return w;
            }
        }
        (widths[0] as usize).min(remaining)
    }
}

/// Structural keywords that mark a stream as at least attempting to be a CID
/// CMap (Adobe CMap & CIDFont Files Spec §7). Mirrors
/// `cmap::CMAP_STRUCTURAL_KEYWORDS`'s "not a CMap at all vs. a malformed
/// CMap" distinction for the CID-keyed keywords.
const CID_CMAP_STRUCTURAL_KEYWORDS: [&str; 4] = ["begincmap", "begincidchar", "begincidrange", "begincodespacerange"];

fn has_any_structural_keyword(content: &str) -> bool {
    CID_CMAP_STRUCTURAL_KEYWORDS
        .iter()
        .any(|keyword| content.contains(keyword))
}

/// Parse a `<lo> <hi>` codespace entry. The hex-digit count of the wider of
/// the two operands determines the byte width (2 digits/byte, rounded up).
fn parse_codespacerange_entry(line: &str) -> Option<CodespaceRange> {
    static RE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]+)>").unwrap());
    let caps = RE.captures(line)?;
    let lo_hex = &caps[1];
    let hi_hex = &caps[2];
    let n_bytes = lo_hex.len().max(hi_hex.len()).div_ceil(2).clamp(1, 4) as u8;
    let low = u32::from_str_radix(lo_hex, 16).ok()?;
    let high = u32::from_str_radix(hi_hex, 16).ok()?;
    if low > high {
        return None;
    }
    Some(CodespaceRange { low, high, n_bytes })
}

/// Parse a `begincidchar` line, returning every `<code> cid` pair found
/// (multiple pairs per line are legal, mirroring `bfchar`'s syntax).
fn parse_cidchar_line(line: &str) -> Vec<(u32, u16)> {
    static RE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"<([0-9A-Fa-f]+)>\s*(\d+)").unwrap());
    RE.captures_iter(line)
        .filter_map(|caps| {
            let code = u32::from_str_radix(&caps[1], 16).ok()?;
            let cid: u32 = caps[2].parse().ok()?;
            let cid = u16::try_from(cid).ok()?;
            Some((code, cid))
        })
        .collect()
}

/// Parse a `begincidrange` line: `<lo> <hi> cid`.
fn parse_cidrange_line(line: &str) -> Option<CidRangeEntry> {
    static RE: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"<([0-9A-Fa-f]+)>\s*<([0-9A-Fa-f]+)>\s*(\d+)").unwrap());
    let caps = RE.captures(line)?;
    let start = u32::from_str_radix(&caps[1], 16).ok()?;
    let end = u32::from_str_radix(&caps[2], 16).ok()?;
    let start_cid: u32 = caps[3].parse().ok()?;
    let start_cid = u16::try_from(start_cid).ok()?;
    if start > end {
        return None;
    }
    Some(CidRangeEntry { start, end, start_cid })
}

/// Parse an embedded `/Encoding` CMap stream into a charcode → CID map.
///
/// # Errors
///
/// Only a **non-empty** stream carrying none of `begincmap`/`begincidchar`/
/// `begincidrange`/`begincodespacerange` fails loudly (wrong stream, not a
/// CMap at all — mirrors `parse_tounicode_cmap`'s error policy). A
/// zero-length stream, a `usecmap`-only stream, and any malformed
/// line/block are all DEGRADED: parsing continues with whatever the rest of
/// the stream yields, and the result may legitimately be empty.
pub(crate) fn parse_cid_cmap(data: &[u8]) -> Result<CidCMap> {
    let mut map = CidCMap::default();
    let content = String::from_utf8_lossy(data);

    if !data.is_empty() && !has_any_structural_keyword(&content) {
        return Err(crate::error::Error::Font(format!(
            "Encoding CMap stream ({} byte(s)) has none of begincmap/begincidchar/begincidrange/\
             begincodespacerange; not a valid CID CMap",
            data.len()
        )));
    }

    for section in extract_sections(&content, "begincodespacerange", "endcodespacerange") {
        for line in significant_lines(section) {
            if let Some(range) = parse_codespacerange_entry(line) {
                map.codespace.push(range);
            }
        }
    }

    for section in extract_sections(&content, "begincidchar", "endcidchar") {
        for line in significant_lines(section) {
            for (code, cid) in parse_cidchar_line(line) {
                map.chars.insert(code, cid);
            }
        }
    }

    for section in extract_sections(&content, "begincidrange", "endcidrange") {
        for line in significant_lines(section) {
            if let Some(entry) = parse_cidrange_line(line) {
                map.ranges.push(entry);
            }
        }
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_single_cidrange() {
        let data = b"1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
                      1 begincidrange\n<0000> <005E> 1\nendcidrange\n";
        let map = parse_cid_cmap(data).unwrap();
        assert_eq!(map.lookup(0x0000), Some(1));
        assert_eq!(map.lookup(0x0041), Some(0x42));
        assert_eq!(map.lookup(0x005E), Some(0x5F));
        assert_eq!(map.lookup(0x005F), None);
        assert_eq!(map.codespace_widths(), vec![2]);
    }

    #[test]
    fn parses_cidchar_single_pairs() {
        let data = b"1 begincidchar\n<0041> 34\nendcidchar\n";
        let map = parse_cid_cmap(data).unwrap();
        assert_eq!(map.lookup(0x41), Some(34));
    }

    #[test]
    fn cidchar_takes_precedence_over_overlapping_cidrange() {
        // Document declares both; the single-char override must win
        // regardless of parse order (mirrors bfchar/bfrange precedence). ~keep
        let data = b"1 begincidrange\n<0000> <00FF> 100\nendcidrange\n\
                      1 begincidchar\n<0041> 999\nendcidchar\n";
        let map = parse_cid_cmap(data).unwrap();
        assert_eq!(map.lookup(0x41), Some(999));
        assert_eq!(map.lookup(0x42), Some(100 + 0x42));
    }

    #[test]
    fn variable_width_codespace_segments_per_code() {
        // A Shift-JIS-flavoured codespace: single bytes below 0x80, and a
        // 2-byte lead range 0x8140-0x9FFC. ~keep
        let data = b"2 begincodespacerange\n<00> <80>\n<8140> <9FFC>\nendcodespacerange\n";
        let map = parse_cid_cmap(data).unwrap();
        assert_eq!(map.codespace_widths(), vec![1, 2]);
        let bytes = [0x41u8, 0x81, 0x40, 0x20];
        assert_eq!(map.code_length(&bytes, 0), 1); // 'A' -> 1 byte
        assert_eq!(map.code_length(&bytes, 1), 2); // 0x8140 -> 2 bytes
        assert_eq!(map.code_length(&bytes, 3), 1); // space -> 1 byte
        assert_eq!(map.code_length(&bytes, 4), 0); // past the end
    }

    #[test]
    fn fixed_two_byte_codespace_reports_single_width() {
        let data = b"1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n";
        let map = parse_cid_cmap(data).unwrap();
        assert_eq!(map.codespace_widths(), vec![2]);
        assert_eq!(map.code_length(&[0x00, 0x41], 0), 2);
    }

    #[test]
    fn usecmap_only_stream_parses_to_empty_map() {
        // No inline cidrange/cidchar/codespacerange data — degraded, not an
        // error, and callers must see this as "no embedded CID map". ~keep
        let data = b"/Identity-H usecmap\nbegincmap\nendcmap\n";
        let map = parse_cid_cmap(data).unwrap();
        assert!(map.is_empty());
        assert!(map.codespace_widths().is_empty());
    }

    #[test]
    fn non_cmap_stream_is_rejected() {
        let data = b"this is not a cmap at all, just some random text of enough length";
        assert!(parse_cid_cmap(data).is_err());
    }

    #[test]
    fn empty_stream_is_legitimately_empty() {
        let map = parse_cid_cmap(b"").unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn truncated_cidrange_block_is_dropped_without_panicking() {
        let data = b"1 begincidrange\n<0000> <005E> 1\n"; // no endcidrange
        let map = parse_cid_cmap(data).unwrap();
        assert!(map.is_empty());
    }
}
