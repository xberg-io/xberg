//! CID to Unicode mappings for predefined Adobe character collections.
//!
//! This module provides CID (Character Identifier) to Unicode mappings for the
//! standard Adobe CJK character collections used in PDF documents.
//!
//! Per PDF Spec ISO 32000-1:2008 Section 9.7.5.2, these predefined CMaps map
//! CIDs from specific character collections to Unicode code points.
//!
//! # Supported Character Collections
//!
//! - **Adobe-GB1**: Simplified Chinese (GB 2312 + extensions)
//! - **Adobe-Japan1**: Japanese (JIS X 0208, JIS X 0212)
//! - **Adobe-CNS1**: Traditional Chinese (CNS 11643)
//! - **Adobe-Korea1**: Korean (KS X 1001)
//!
//! # References
//!
//! - Adobe Technical Note #5078: Adobe-Japan1-7
//! - Adobe Technical Note #5079: Adobe-GB1-5
//! - Adobe Technical Note #5080: Adobe-CNS1-7
//! - Adobe Technical Note #5093: Adobe-Korea1-2
//!
//! # Implementation Notes
//!
//! This module uses `phf_map!` for O(1) CID-to-Unicode lookup, following the
//! same pattern as `adobe_glyph_list.rs`.

mod adobe_arabic;
mod adobe_cns1;
mod adobe_gb1;
mod adobe_japan1;
mod adobe_korea1;

/// Look up Unicode code point for a CID in Adobe-GB1 (Simplified Chinese).
///
/// This mapping corresponds to the UniGB-UCS2-H CMap.
///
/// # Arguments
///
/// * `cid` - Character Identifier (0-29063 for Adobe-GB1-5)
///
/// # Returns
///
/// The corresponding Unicode code point, or None if not mapped.
#[inline]
pub fn lookup_adobe_gb1(cid: u16) -> Option<u32> {
    adobe_gb1::lookup(cid)
}

/// Look up Unicode code point for a CID in Adobe-Japan1 (Japanese).
///
/// This mapping corresponds to the UniJIS-UCS2-H CMap.
///
/// # Arguments
///
/// * `cid` - Character Identifier (0-23057 for Adobe-Japan1-7)
///
/// # Returns
///
/// The corresponding Unicode code point, or None if not mapped.
#[inline]
pub fn lookup_adobe_japan1(cid: u16) -> Option<u32> {
    adobe_japan1::lookup(cid)
}

/// Look up Unicode code point for a CID in Adobe-CNS1 (Traditional Chinese).
///
/// This mapping corresponds to the UniCNS-UCS2-H CMap.
///
/// # Arguments
///
/// * `cid` - Character Identifier (0-19155 for Adobe-CNS1-7)
///
/// # Returns
///
/// The corresponding Unicode code point, or None if not mapped.
#[inline]
pub fn lookup_adobe_cns1(cid: u16) -> Option<u32> {
    adobe_cns1::lookup(cid)
}

/// Look up Unicode code point for a CID in Adobe-Korea1 (Korean).
///
/// This mapping corresponds to the UniKS-UCS2-H CMap.
///
/// # Arguments
///
/// * `cid` - Character Identifier (0-18351 for Adobe-Korea1-2)
///
/// # Returns
///
/// The corresponding Unicode code point, or None if not mapped.
#[inline]
pub fn lookup_adobe_korea1(cid: u16) -> Option<u32> {
    adobe_korea1::lookup(cid)
}

/// look up Unicode code point for a CID in
/// Adobe-Arabic-1 / Adobe-Persian-1 (used by Persian / Farsi /
/// Pashto / Urdu fonts that ship without ToUnicode CMaps).
///
/// Stub implementation: identity mapping for the Arabic block
/// (U+0600–U+06FF) and Arabic Presentation Forms (U+FB50–U+FDFF +
/// U+FE70–U+FEFF). The official Adobe-Arabic-1-UCS2 CMap is
/// follow-up work.
///
/// Returns the Unicode code point, or `None` if the CID isn't in
/// the supported range (caller falls back to the existing chain).
///
/// # Arguments
///
/// * `cid` - Character Identifier
#[inline]
pub fn lookup_adobe_arabic(cid: u16) -> Option<u32> {
    adobe_arabic::lookup(cid)
}

/// Reverse (Unicode → CID) lookups for the predefined `Uni*-UCS2-*` /
/// `Uni*-UTF16-*` CMap family (GH #1631).
///
/// Those predefined CMaps map a 2-byte character code that IS the UCS-2 (BMP)
/// encoding of the intended Unicode scalar directly to a CID — so inverting
/// the CID→Unicode tables above (already sourced from the matching
/// `UniXXX-UCS2-H` CMap data, see each table's module doc) reconstructs
/// exactly the code→CID mapping a conforming reader needs, without
/// hardcoding a per-document offset.
///
/// A Unicode value can be the target of more than one CID in these tables
/// (compatibility/duplicate entries merged in from UTF16/UTF32 fallback
/// data). On such a collision the *lowest* CID wins — an arbitrary but
/// deterministic and defensible tie-break, since the lowest CID in a
/// character collection is conventionally the canonical/primary one. This
/// module does not attempt to disambiguate further.
mod reverse {
    use std::collections::HashMap;
    use std::sync::OnceLock;

    /// Build a Unicode → CID map by inverting a `cid -> Option<unicode>`
    /// forward lookup over the full `u16` domain. Run once and cached by
    /// the caller's `OnceLock`; `0..=u16::MAX` is 65536 O(1) `phf` probes,
    /// negligible next to the parse cost of a single PDF page.
    fn build_reverse_map(forward: fn(u16) -> Option<u32>) -> HashMap<u32, u16> {
        let mut map = HashMap::new();
        for cid in 0..=u16::MAX {
            if let Some(unicode) = forward(cid) {
                map.entry(unicode).or_insert(cid);
            }
        }
        map
    }

    fn reverse_lookup(
        cache: &OnceLock<HashMap<u32, u16>>,
        forward: fn(u16) -> Option<u32>,
        unicode: u32,
    ) -> Option<u16> {
        cache.get_or_init(|| build_reverse_map(forward)).get(&unicode).copied()
    }

    static REV_GB1: OnceLock<HashMap<u32, u16>> = OnceLock::new();
    static REV_JAPAN1: OnceLock<HashMap<u32, u16>> = OnceLock::new();
    static REV_CNS1: OnceLock<HashMap<u32, u16>> = OnceLock::new();
    static REV_KOREA1: OnceLock<HashMap<u32, u16>> = OnceLock::new();

    pub(super) fn unicode_to_cid_gb1(unicode: u32) -> Option<u16> {
        reverse_lookup(&REV_GB1, super::lookup_adobe_gb1, unicode)
    }

    pub(super) fn unicode_to_cid_japan1(unicode: u32) -> Option<u16> {
        reverse_lookup(&REV_JAPAN1, super::lookup_adobe_japan1, unicode)
    }

    pub(super) fn unicode_to_cid_cns1(unicode: u32) -> Option<u16> {
        reverse_lookup(&REV_CNS1, super::lookup_adobe_cns1, unicode)
    }

    pub(super) fn unicode_to_cid_korea1(unicode: u32) -> Option<u16> {
        reverse_lookup(&REV_KOREA1, super::lookup_adobe_korea1, unicode)
    }
}

/// Reverse lookup for Adobe-GB1: the CID whose `UniGB-UCS2-H` code point is
/// `unicode`, or `None` if no CID maps there.
#[inline]
pub fn unicode_to_cid_gb1(unicode: u32) -> Option<u16> {
    reverse::unicode_to_cid_gb1(unicode)
}

/// Reverse lookup for Adobe-Japan1: the CID whose `UniJIS-UCS2-H` code point
/// is `unicode`, or `None` if no CID maps there.
#[inline]
pub fn unicode_to_cid_japan1(unicode: u32) -> Option<u16> {
    reverse::unicode_to_cid_japan1(unicode)
}

/// Reverse lookup for Adobe-CNS1: the CID whose `UniCNS-UCS2-H` code point is
/// `unicode`, or `None` if no CID maps there.
#[inline]
pub fn unicode_to_cid_cns1(unicode: u32) -> Option<u16> {
    reverse::unicode_to_cid_cns1(unicode)
}

/// Reverse lookup for Adobe-Korea1: the CID whose `UniKS-UCS2-H` code point
/// is `unicode`, or `None` if no CID maps there.
#[inline]
pub fn unicode_to_cid_korea1(unicode: u32) -> Option<u16> {
    reverse::unicode_to_cid_korea1(unicode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adobe_gb1_ascii_from_cid() {
        // CID 34 maps to 'A' (U+0041), CID 91 maps to 'z' (U+007A)
        // Note: CIDs are indices in the character collection, not Unicode values ~keep
        assert_eq!(lookup_adobe_gb1(34), Some(0x41));
        assert_eq!(lookup_adobe_gb1(91), Some(0x7A));
    }

    #[test]
    fn test_adobe_japan1_ascii_from_cid() {
        assert_eq!(lookup_adobe_japan1(34), Some(0x41));
        assert_eq!(lookup_adobe_japan1(91), Some(0x7A));
    }

    #[test]
    fn test_adobe_japan1_hiragana() {
        assert_eq!(lookup_adobe_japan1(843), Some(0x3042));
    }

    #[test]
    fn test_cid_tables_map_to_ideographs_not_radical_forms() {
        // A CID whose glyph is shared between a kanji and its radical
        // presentation form must resolve to the unified ideograph, never a
        // U+2E80–2FDF dictionary codepoint (Equivalent_Unified_Ideograph). ~keep
        assert_eq!(lookup_adobe_japan1(2664), Some(0x9752)); // 青, not U+2ED8 ⻘ ~keep
        assert_eq!(lookup_adobe_japan1(2666), Some(0x6589)); // 斉, not U+2EEB ⻫ ~keep
    }

    #[test]
    fn test_adobe_cns1_ascii_from_cid() {
        assert_eq!(lookup_adobe_cns1(34), Some(0x41));
    }

    #[test]
    fn test_adobe_korea1_ascii_from_cid() {
        assert_eq!(lookup_adobe_korea1(34), Some(0x41));
    }

    #[test]
    fn test_adobe_korea1_hangul() {
        // Test Hangul syllable from CID (from Adobe cid2code.txt) ~keep
        assert_eq!(lookup_adobe_korea1(1086), Some(0xAC00));
    }

    #[test]
    fn test_unicode_to_cid_cns1_round_trips_ascii() {
        // GH #1631: 'T' (U+0054) must resolve to CID 53 under UniCNS-UCS2-H,
        // matching the CID = code - 31 relationship measured on the reported
        // document — reconstructed here from the vendored table, not
        // hardcoded. ~keep
        assert_eq!(unicode_to_cid_cns1(0x54), Some(53));
        assert_eq!(lookup_adobe_cns1(53), Some(0x54));
    }

    #[test]
    fn test_unicode_to_cid_gb1_round_trips_ascii() {
        assert_eq!(unicode_to_cid_gb1(0x41), Some(34));
    }

    #[test]
    fn test_unicode_to_cid_japan1_round_trips_ascii() {
        assert_eq!(unicode_to_cid_japan1(0x41), Some(34));
    }

    #[test]
    fn test_unicode_to_cid_korea1_round_trips_ascii() {
        assert_eq!(unicode_to_cid_korea1(0x41), Some(34));
    }

    #[test]
    fn test_unicode_to_cid_returns_none_for_unmapped_codepoint() {
        // U+FFFE is a noncharacter; no Adobe collection should map to it. ~keep
        assert_eq!(unicode_to_cid_cns1(0xFFFE), None);
    }
}
