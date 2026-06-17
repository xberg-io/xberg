//! Font utility helpers for pdfium text extraction.

/// Known proportional font families where pdfium's `font_is_fixed_pitch()`
/// returns a false positive (common with non-embedded Type 1 fonts).
const KNOWN_PROPORTIONAL_FONTS: &[&str] = &[
    "helvetica",
    "arial",
    "times",
    "georgia",
    "verdana",
    "tahoma",
    "trebuchet",
    "calibri",
    "cambria",
    "garamond",
    "palatino",
    "book antiqua",
    "century",
    "dejavu sans",
    "dejavu serif",
    "liberation sans",
    "liberation serif",
    "noto sans",
    "noto serif",
    "roboto",
    "open sans",
    "lato",
    "inter",
    "segoe",
    "gill sans",
    "optima",
    "futura",
    "avenir",
    "lucida sans",
    "lucida bright",
];

/// Check if pdfium's fixed-pitch flag should be trusted for the given font.
///
/// Returns `true` only if the font is truly monospace — overrides false
/// positives from pdfium for known proportional fonts.
pub(crate) fn is_truly_monospace(pdfium_fixed_pitch: bool, font_name: &str) -> bool {
    if !pdfium_fixed_pitch {
        return false;
    }
    let lower = font_name.to_ascii_lowercase();
    // If the font name matches a known proportional family, ignore the flag.
    !KNOWN_PROPORTIONAL_FONTS.iter().any(|p| lower.contains(p))
}
