//! Recognition of Adobe predefined CIDFont base names.
//!
//! ISO 32000-2:2020 §9.7.5.2 requires a conforming PDF processor to support the
//! Adobe-CNS1-7, Adobe-GB1-5, Adobe-Japan1-7 and Adobe-KR-9 character
//! collections. In practice this means recognising a PDF that references one of
//! Adobe's well-known CIDFont base names — `Ryumin-Light`, `GothicBBB-Medium`,
//! `STSong-Light`, `MHei-Medium`, `HYSMyeongJo-Medium`, `HeiseiMin-W3`,
//! `HeiseiKakuGo-W5`, `MSung-Light`, … — and rendering glyphs for it even when
//! the document doesn't embed the outlines. Such PDFs are widespread: every
//! Japanese government document, most Chinese / Korean academic typesetting
//! pipelines, and a long tail of legacy tooling rely on the reader to
//! materialise the glyphs from a covering font.
//!
//! This module is the name → character-collection registry. Callers consult
//! the `is_predefined` function below to decide whether a CIDFont without
//! `/FontFile{,2,3}` is a candidate for substitution. The actual glyph paint goes through the bundled
//! Droid Sans Fallback (gated on the `cjk-render-fallback` cargo feature) via
//! the CID → Unicode tables in [`crate::fonts::cid_mappings`].
//!
//! ## Style fidelity
//!
//! Substitution renders **sans-serif** glyphs for every input, regardless of
//! whether the requested face was a Mincho / Song (serif), Gothic / Hei
//! (sans-serif), or display variant. This is the same trade conforming readers
//! make in the absence of system fonts: glyph shape integrity is preferable to
//! a blank page. Vertical-form variant glyphs (`vert`/`vrt2` features) are
//! likewise not provided — the renderer uses the same glyphs for horizontal
//! and vertical writing modes and relies on the existing `WMode` advance
//! routing to lay them out vertically.
//!
//! ## Coverage
//!
//! Both the bare base names and the encoding-suffixed combined-resource forms
//! are matched. Adobe's predefined CMaps (`Identity-H`, `Identity-V`,
//! `UniJIS-UCS2-H`, `90ms-RKSJ-H`, …) are appended to the base name by the
//! producer to form synthetic CIDFonts like `Ryumin-Light-Identity-V`. The
//! match strategy splits on the *first* known CMap suffix; anything before is
//! the candidate base name.
//!
//! Name recognition is necessary but not sufficient for substitution: the
//! load-time gate in `FontInfo::from_dict` additionally requires the font's
//! `/Encoding` to resolve to an Identity charcode→CID mapping. Non-Identity
//! predefined CMaps (`90ms-RKSJ-H`, `GBK-EUC-H`, `B5pc-H`, …) put raw
//! Shift-JIS / EUC / Big5 codes — not CIDs — in the content stream, and the
//! CID→Unicode tables would mis-decode them; those fonts are recognised here
//! but left unsubstituted until a charcode→CID CMap pass is wired.

/// One of the four Adobe predefined character collections supported by this
/// renderer.
///
/// The variant selects which [`crate::fonts::cid_mappings`] lookup table is
/// consulted to map a CID to a Unicode code point for the substituted-glyph
/// paint path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CharacterCollection {
    /// Adobe-Japan1 (Japanese): JIS X 0208 / 0212, kana, kanji.
    AdobeJapan1,
    /// Adobe-GB1 (Simplified Chinese): GB 2312 + extensions.
    AdobeGB1,
    /// Adobe-CNS1 (Traditional Chinese): CNS 11643, Big5.
    AdobeCNS1,
    /// Adobe-Korea1 (Korean): KS X 1001.
    AdobeKorea1,
}

impl CharacterCollection {
    /// Look up the Unicode code point for a CID under this collection.
    ///
    /// Returns `None` for CIDs not in the collection's mapping table — the
    /// renderer should fall back to a `.notdef` paint preserving the advance
    /// width.
    #[inline]
    pub fn cid_to_unicode(self, cid: u16) -> Option<u32> {
        match self {
            CharacterCollection::AdobeJapan1 => super::cid_mappings::lookup_adobe_japan1(cid),
            CharacterCollection::AdobeGB1 => super::cid_mappings::lookup_adobe_gb1(cid),
            CharacterCollection::AdobeCNS1 => super::cid_mappings::lookup_adobe_cns1(cid),
            CharacterCollection::AdobeKorea1 => super::cid_mappings::lookup_adobe_korea1(cid),
        }
    }

    /// Look up the CID whose predefined `Uni*-UCS2-H`/`-V`/`Uni*-UTF16-H`/`-V`
    /// code point (GH #1631) is `unicode`. The inverse of [`Self::cid_to_unicode`].
    ///
    /// Only meaningful for a Type0 font whose `/Encoding` is one of the
    /// canonical Unicode-keyed predefined CMap names — see
    /// `is_unicode_keyed_predefined_cmap` in `font_dict.rs`, which gates
    /// calls into this. Returns `None` when `unicode` has no CID in this
    /// collection.
    #[inline]
    pub fn unicode_to_cid(self, unicode: u32) -> Option<u16> {
        match self {
            CharacterCollection::AdobeJapan1 => super::cid_mappings::unicode_to_cid_japan1(unicode),
            CharacterCollection::AdobeGB1 => super::cid_mappings::unicode_to_cid_gb1(unicode),
            CharacterCollection::AdobeCNS1 => super::cid_mappings::unicode_to_cid_cns1(unicode),
            CharacterCollection::AdobeKorea1 => super::cid_mappings::unicode_to_cid_korea1(unicode),
        }
    }
}

/// Adobe predefined CMap suffixes that producers append to a base font name to
/// form a combined-resource Type0 font reference. The split is performed
/// against this list so that `Ryumin-Light-Identity-V` resolves back to
/// `Ryumin-Light`, `STSong-Light-GBK-EUC-H` resolves back to `STSong-Light`,
/// and so on.
///
/// Order matters: longer suffixes are listed first so the split prefers the
/// most specific match. ISO 32000-1 Annex F (predefined CMaps) plus the Adobe
/// Technical Notes #5078 / #5079 / #5080 / #5093 are the upstream references.
const KNOWN_CMAP_SUFFIXES: &[&str] = &[
    "Identity-H",
    "Identity-V",
    "UniJIS-UCS2-H",
    "UniJIS-UCS2-V",
    "UniJIS-UCS2-HW-H",
    "UniJIS-UCS2-HW-V",
    "UniJIS-UTF16-H",
    "UniJIS-UTF16-V",
    "UniJIS-UTF8-H",
    "UniJIS-UTF8-V",
    "UniJIS-X0213-UTF32-H",
    "UniJIS-X0213-UTF32-V",
    "UniJIS-X02132004-UTF32-H",
    "UniJIS-X02132004-UTF32-V",
    "UniJISPro-UCS2-HW-V",
    "UniJISPro-UCS2-V",
    "UniJISX0213-UTF32-H",
    "UniJISX0213-UTF32-V",
    "UniJISX02132004-UTF32-H",
    "UniJISX02132004-UTF32-V",
    "90ms-RKSJ-H",
    "90ms-RKSJ-V",
    "90msp-RKSJ-H",
    "90msp-RKSJ-V",
    "90pv-RKSJ-H",
    "90pv-RKSJ-V",
    "78ms-RKSJ-H",
    "78ms-RKSJ-V",
    "83pv-RKSJ-H",
    "Add-RKSJ-H",
    "Add-RKSJ-V",
    "EUC-H",
    "EUC-V",
    "Ext-RKSJ-H",
    "Ext-RKSJ-V",
    "H",
    "V",
    "WP-Symbol",
    "Hojo-EUC-H",
    "Hojo-EUC-V",
    "Hojo-H",
    "Hojo-V",
    "Hankaku",
    "Hiragana",
    "Katakana",
    "Roman",
    "UniGB-UCS2-H",
    "UniGB-UCS2-V",
    "UniGB-UTF16-H",
    "UniGB-UTF16-V",
    "UniGB-UTF8-H",
    "UniGB-UTF8-V",
    "GB-EUC-H",
    "GB-EUC-V",
    "GBK-EUC-H",
    "GBK-EUC-V",
    "GBK2K-H",
    "GBK2K-V",
    "GBKp-EUC-H",
    "GBKp-EUC-V",
    "GBpc-EUC-H",
    "GBpc-EUC-V",
    "GBT-EUC-H",
    "GBT-EUC-V",
    "GBT-H",
    "GBT-V",
    "GBTpc-EUC-H",
    "UniCNS-UCS2-H",
    "UniCNS-UCS2-V",
    "UniCNS-UTF16-H",
    "UniCNS-UTF16-V",
    "UniCNS-UTF8-H",
    "UniCNS-UTF8-V",
    "B5pc-H",
    "B5pc-V",
    "ETen-B5-H",
    "ETen-B5-V",
    "ETenms-B5-H",
    "ETenms-B5-V",
    "CNS-EUC-H",
    "CNS-EUC-V",
    "HKscs-B5-H",
    "HKscs-B5-V",
    "UniKS-UCS2-H",
    "UniKS-UCS2-V",
    "UniKS-UTF16-H",
    "UniKS-UTF16-V",
    "UniKS-UTF8-H",
    "UniKS-UTF8-V",
    "KSC-EUC-H",
    "KSC-EUC-V",
    "KSCms-UHC-H",
    "KSCms-UHC-V",
    "KSCms-UHC-HW-H",
    "KSCms-UHC-HW-V",
    "KSCpc-EUC-H",
];

/// Strip a trailing `-<suffix>` from `name` if `<suffix>` is one of the
/// recognised Adobe predefined CMap names. Returns the truncated base name; if
/// no suffix matches, returns the input unchanged.
///
/// Prefers the **longest** matching suffix: `STSong-Light-GBK-EUC-H` must strip
/// `-GBK-EUC-H` and yield `STSong-Light`, not strip the short `-H` suffix
/// (which would leave the unresolvable `STSong-Light-GBK-EUC`). The single-
/// letter `-H` / `-V` legacy CMap names are real (ISO 32000-1 Annex F) so we
/// can't drop them from the list, but they must be tried last.
fn strip_cmap_suffix(name: &str) -> &str {
    let mut best: Option<&str> = None;
    let mut best_len: usize = 0;
    for suffix in KNOWN_CMAP_SUFFIXES {
        // Strict `-<suffix>` boundary so we don't truncate a hyphenless base
        // name (e.g. `Ryumin-Light` itself ends in `Light`, and we must NOT
        // consume `Light` as if it were a CMap suffix). ~keep
        let trailer = format!("-{}", suffix);
        if let Some(stripped) = name.strip_suffix(&trailer)
            && trailer.len() > best_len
        {
            best_len = trailer.len();
            best = Some(stripped);
        }
    }
    best.unwrap_or(name)
}

/// Bare base-font name → character collection registry.
///
/// Covers the predefined CIDFont names listed in Adobe Technical Notes #5078
/// (Adobe-Japan1), #5079 (Adobe-GB1), #5080 (Adobe-CNS1), and #5093
/// (Adobe-Korea1), plus a long-tail of well-known producer-emitted aliases
/// (SimSun, SimHei, MingLiU, …) that downstream Asian word processors use to
/// reference the same collections.
///
/// The list is intentionally hand-curated and conservative: matching is by
/// exact base-name string (post-suffix strip). Adding a name here promises
/// that the renderer will substitute glyphs from Droid Sans Fallback for it
/// when the source PDF doesn't ship outlines.
fn collection_for_bare_name(name: &str) -> Option<CharacterCollection> {
    use CharacterCollection::*;
    // Adobe-Japan1 — Mincho / Gothic family + Heisei + Kozuka ~keep
    if matches!(
        name,
        "Ryumin-Light"
            | "Ryumin-Medium"
            | "Ryumin-Regular"
            | "Ryumin-Heavy"
            | "Ryumin-Bold"
            | "Ryumin-Ultra"
            | "GothicBBB-Medium"
            | "GothicMB101-Bold"
            | "FutoGoB101-Bold"
            | "FutoMinA101-Bold"
            | "Jun101-Light"
            | "MidashiGo-MB31"
            | "MidashiMin-MA31"
            | "HeiseiMin-W3"
            | "HeiseiMin-W5"
            | "HeiseiMin-W7"
            | "HeiseiMin-W9"
            | "HeiseiKakuGo-W3"
            | "HeiseiKakuGo-W5"
            | "HeiseiKakuGo-W7"
            | "HeiseiKakuGo-W9"
            | "HeiseiMaruGo-W4"
            | "KozMinPro-Regular"
            | "KozMinPro-Light"
            | "KozMinPro-Medium"
            | "KozMinPro-Bold"
            | "KozMinPro-Heavy"
            | "KozMinProVI-Regular"
            | "KozMinProVI-Light"
            | "KozMinProVI-Medium"
            | "KozMinProVI-Bold"
            | "KozMinProVI-Heavy"
            | "KozGoPro-Regular"
            | "KozGoPro-Light"
            | "KozGoPro-Medium"
            | "KozGoPro-Bold"
            | "KozGoPro-Heavy"
            | "KozGoProVI-Regular"
            | "KozGoProVI-Light"
            | "KozGoProVI-Medium"
            | "KozGoProVI-Bold"
            | "KozGoProVI-Heavy"
            | "Kozuka-Mincho-Pro-VI-R"
            | "Kozuka-Gothic-Pro-VI-M"
    ) {
        return Some(AdobeJapan1);
    }
    // Adobe-GB1 — STSong / STHeiti / SimSun / SimHei ~keep
    if matches!(
        name,
        "STSong-Light"
            | "STSongStd-Light"
            | "STSong-Regular"
            | "STSongStd-Regular"
            | "STHeiti-Regular"
            | "STHeiti-Light"
            | "STHeitiSC-Regular"
            | "STHeitiSC-Light"
            | "STKaiti-Regular"
            | "STKaitiStd-Regular"
            | "STFangsong-Light"
            | "STFangsong-Regular"
            | "SimSun"
            | "SimHei"
            | "SimSun-ExtB"
            | "AdobeSongStd-Light"
            | "AdobeHeitiStd-Regular"
            | "AdobeKaitiStd-Regular"
            | "AdobeFangsongStd-Regular"
    ) {
        return Some(AdobeGB1);
    }
    // Adobe-CNS1 — Traditional Chinese (MHei / MSung / MingLiU / DFKai) ~keep
    if matches!(
        name,
        "MHei-Medium"
            | "MSung-Light"
            | "MSung-Medium"
            | "MSungStd-Light"
            | "MSungStd-Medium"
            | "MSungStd-Light-Acro"
            | "MingLiU"
            | "PMingLiU"
            | "MingLiU-ExtB"
            | "PMingLiU-ExtB"
            | "DFKaiShu-SB-Estd-BF"
            | "DFKaiSho-W5"
            | "HeiseiKakuGoStd-W5"
            | "AdobeMingStd-Light"
            | "AdobeFanHeitiStd-Bold"
            | "AdobeSongStd-Bold"
    ) {
        return Some(AdobeCNS1);
    }
    // Adobe-Korea1 — Korean (HYSMyeongJo / HYGoThic / Adobe-Myungjo) ~keep
    if matches!(
        name,
        "HYSMyeongJo-Medium"
            | "HYSMyeongJoStd-Medium"
            | "HYGoThic-Medium"
            | "HYGothic-Medium"
            | "HYGothic-Bold"
            | "HYGothicStd-Medium"
            | "HYRGoThic-Medium"
            | "HYHeadLine-Medium"
            | "Adobe-MyungjoStd-Medium"
            | "AdobeMyungjoStd-Medium"
            | "AdobeGothicStd-Bold"
            | "Batang"
            | "BatangChe"
            | "Dotum"
            | "DotumChe"
            | "Gulim"
            | "GulimChe"
            | "Gungsuh"
            | "GungsuhChe"
    ) {
        return Some(AdobeKorea1);
    }
    None
}

/// Decide whether `base_font` names a predefined Adobe CIDFont this renderer
/// can substitute glyphs for.
///
/// The base-font name follows the PDF convention of an optional 6-character
/// subset prefix (`ABCDEF+…`), then either a bare base name or a base name
/// concatenated with an Adobe predefined CMap name (`Ryumin-Light-Identity-V`,
/// `STSong-Light-GBK-EUC-H`, …). This function strips the subset prefix and
/// the CMap suffix, then matches the remainder against the curated registry.
///
/// Returns the [`CharacterCollection`] when a match is found, otherwise
/// `None`.
pub fn is_predefined(base_font: &str) -> Option<CharacterCollection> {
    // Strip 6-uppercase-letter subset prefix (`XEAACC+Ryumin-Light`).
    // The prefix is exactly six letters followed by `+`; we only strip when
    // that exact shape matches so we don't truncate Asian-tooling base names
    // that legitimately contain `+` elsewhere. ~keep
    let after_prefix = match base_font.find('+') {
        Some(plus_idx) if plus_idx == 6 => {
            let prefix = &base_font[..plus_idx];
            if prefix.chars().all(|c| c.is_ascii_uppercase()) {
                &base_font[plus_idx + 1..]
            } else {
                base_font
            }
        }
        _ => base_font,
    };
    let bare = strip_cmap_suffix(after_prefix);
    collection_for_bare_name(bare)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_japanese_names_resolve_to_japan1() {
        assert_eq!(is_predefined("Ryumin-Light"), Some(CharacterCollection::AdobeJapan1));
        assert_eq!(
            is_predefined("GothicBBB-Medium"),
            Some(CharacterCollection::AdobeJapan1)
        );
        assert_eq!(is_predefined("HeiseiMin-W3"), Some(CharacterCollection::AdobeJapan1));
        assert_eq!(is_predefined("HeiseiKakuGo-W5"), Some(CharacterCollection::AdobeJapan1));
        assert_eq!(
            is_predefined("KozMinPro-Regular"),
            Some(CharacterCollection::AdobeJapan1)
        );
        assert_eq!(
            is_predefined("Kozuka-Mincho-Pro-VI-R"),
            Some(CharacterCollection::AdobeJapan1)
        );
    }

    #[test]
    fn suffixed_japanese_names_resolve_to_japan1() {
        assert_eq!(
            is_predefined("Ryumin-Light-Identity-V"),
            Some(CharacterCollection::AdobeJapan1)
        );
        assert_eq!(
            is_predefined("Ryumin-Light-Identity-H"),
            Some(CharacterCollection::AdobeJapan1)
        );
        assert_eq!(
            is_predefined("Ryumin-Light-90ms-RKSJ-H"),
            Some(CharacterCollection::AdobeJapan1)
        );
        assert_eq!(
            is_predefined("GothicBBB-Medium-Identity-H"),
            Some(CharacterCollection::AdobeJapan1)
        );
        assert_eq!(
            is_predefined("HeiseiKakuGo-W5-UniJIS-UCS2-H"),
            Some(CharacterCollection::AdobeJapan1)
        );
    }

    #[test]
    fn bare_chinese_simplified_names_resolve_to_gb1() {
        assert_eq!(is_predefined("STSong-Light"), Some(CharacterCollection::AdobeGB1));
        assert_eq!(is_predefined("STSongStd-Light"), Some(CharacterCollection::AdobeGB1));
        assert_eq!(is_predefined("STHeiti-Light"), Some(CharacterCollection::AdobeGB1));
        assert_eq!(is_predefined("SimSun"), Some(CharacterCollection::AdobeGB1));
        assert_eq!(is_predefined("SimHei"), Some(CharacterCollection::AdobeGB1));
    }

    #[test]
    fn suffixed_chinese_simplified_names_resolve_to_gb1() {
        assert_eq!(
            is_predefined("STSong-Light-GBK-EUC-H"),
            Some(CharacterCollection::AdobeGB1)
        );
        assert_eq!(
            is_predefined("STSong-Light-UniGB-UCS2-H"),
            Some(CharacterCollection::AdobeGB1)
        );
        assert_eq!(
            is_predefined("STHeiti-Light-Identity-V"),
            Some(CharacterCollection::AdobeGB1)
        );
    }

    #[test]
    fn bare_chinese_traditional_names_resolve_to_cns1() {
        assert_eq!(is_predefined("MHei-Medium"), Some(CharacterCollection::AdobeCNS1));
        assert_eq!(is_predefined("MSung-Light"), Some(CharacterCollection::AdobeCNS1));
        assert_eq!(is_predefined("MSungStd-Light"), Some(CharacterCollection::AdobeCNS1));
        assert_eq!(is_predefined("MingLiU"), Some(CharacterCollection::AdobeCNS1));
        assert_eq!(is_predefined("PMingLiU"), Some(CharacterCollection::AdobeCNS1));
    }

    #[test]
    fn suffixed_chinese_traditional_names_resolve_to_cns1() {
        assert_eq!(
            is_predefined("MHei-Medium-B5pc-H"),
            Some(CharacterCollection::AdobeCNS1)
        );
        assert_eq!(
            is_predefined("MSung-Light-Identity-H"),
            Some(CharacterCollection::AdobeCNS1)
        );
        assert_eq!(is_predefined("MingLiU-ETen-B5-V"), Some(CharacterCollection::AdobeCNS1));
    }

    #[test]
    fn bare_korean_names_resolve_to_korea1() {
        assert_eq!(
            is_predefined("HYSMyeongJo-Medium"),
            Some(CharacterCollection::AdobeKorea1)
        );
        assert_eq!(is_predefined("HYGoThic-Medium"), Some(CharacterCollection::AdobeKorea1));
        assert_eq!(
            is_predefined("Adobe-MyungjoStd-Medium"),
            Some(CharacterCollection::AdobeKorea1)
        );
        assert_eq!(is_predefined("Batang"), Some(CharacterCollection::AdobeKorea1));
    }

    #[test]
    fn suffixed_korean_names_resolve_to_korea1() {
        assert_eq!(
            is_predefined("HYSMyeongJo-Medium-KSC-EUC-H"),
            Some(CharacterCollection::AdobeKorea1)
        );
        assert_eq!(
            is_predefined("HYGoThic-Medium-Identity-V"),
            Some(CharacterCollection::AdobeKorea1)
        );
    }

    #[test]
    fn subset_prefix_is_stripped() {
        assert_eq!(
            is_predefined("ABCDEF+Ryumin-Light-Identity-V"),
            Some(CharacterCollection::AdobeJapan1)
        );
        assert_eq!(
            is_predefined("XEAACC+STSong-Light"),
            Some(CharacterCollection::AdobeGB1)
        );
    }

    #[test]
    fn unrelated_fonts_return_none() {
        assert_eq!(is_predefined("ArialMT"), None);
        assert_eq!(is_predefined("Helvetica"), None);
        assert_eq!(is_predefined("Times-Roman"), None);
        assert_eq!(is_predefined("DejaVuSans"), None);
        // Not in registry — Adobe Garamond is Latin, not CJK. ~keep
        assert_eq!(is_predefined("AGaramondPro-Regular"), None);
    }

    #[test]
    fn cmap_suffix_strip_does_not_swallow_base_name() {
        // `Ryumin-Light` legitimately ends in `Light`, which is NOT a CMap
        // suffix. Strip logic must NOT truncate it. ~keep
        assert_eq!(strip_cmap_suffix("Ryumin-Light"), "Ryumin-Light");
        assert_eq!(strip_cmap_suffix("STSong-Light"), "STSong-Light");
    }

    #[test]
    fn character_collection_cid_to_unicode_routes_to_correct_table() {
        assert_eq!(CharacterCollection::AdobeJapan1.cid_to_unicode(34), Some(0x0041));
        assert_eq!(CharacterCollection::AdobeJapan1.cid_to_unicode(843), Some(0x3042));
        assert_eq!(CharacterCollection::AdobeGB1.cid_to_unicode(34), Some(0x0041));
        assert_eq!(CharacterCollection::AdobeCNS1.cid_to_unicode(34), Some(0x0041));
        assert_eq!(CharacterCollection::AdobeKorea1.cid_to_unicode(1086), Some(0xAC00));
    }
}
