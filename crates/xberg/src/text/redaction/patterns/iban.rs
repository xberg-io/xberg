//! IBAN (International Bank Account Number) detection.
//!
//! Format: two-letter ISO 3166-1 country code + two check digits + up to 30
//! alphanumeric BBAN characters. The regex matches IBANs with optional space
//! separators every four characters (the common pretty-print form). Surviving
//! matches are further validated against the ISO 13616 mod-97 checksum by
//! [`crate::text::redaction::validators::iban::IbanChecksumValidator`],
//! post-aggregation.

use super::PatternMatch;
use crate::types::redaction::PiiCategory;
use once_cell::sync::Lazy;
use regex::Regex;

// Known IBAN-using country codes (ISO 3166-1 alpha-2). Filter cuts down the
// false-positive surface that any two upper-case letters would otherwise allow.
const IBAN_COUNTRIES: &[&str] = &[
    "AD", "AE", "AL", "AT", "AZ", "BA", "BE", "BG", "BH", "BR", "BY", "CH", "CR", "CY", "CZ", "DE", "DK", "DO", "EE",
    "EG", "ES", "FI", "FO", "FR", "GB", "GE", "GI", "GL", "GR", "GT", "HR", "HU", "IE", "IL", "IQ", "IS", "IT", "JO",
    "KW", "KZ", "LB", "LC", "LI", "LT", "LU", "LV", "LY", "MC", "MD", "ME", "MK", "MR", "MT", "MU", "NL", "NO", "PK",
    "PL", "PS", "PT", "QA", "RO", "RS", "SA", "SC", "SE", "SI", "SK", "SM", "ST", "SV", "TL", "TN", "TR", "UA", "VA",
    "VG", "XK",
];

static RE_IBAN: Lazy<Regex> = Lazy::new(|| {
    // Country (2 letters) + check (2 digits) + BBAN as whole 4-character groups
    // (2-7 of them) plus an optional short trailing group (1-3 chars), each
    // group preceded by at most one space. Requiring *whole* groups (rather
    // than an optional space before every single character) keeps the match
    // from bleeding into an adjacent all-caps word that happens to be
    // separated by a single space, e.g. "...0130 00 VIA SEPA" would
    // otherwise be swallowed whole because "VIA"/"SEPA" also look like valid
    // alphanumeric BBAN characters.
    Regex::new(r"\b[A-Z]{2}\d{2}(?:[ ]?[A-Z0-9]{4}){2,7}(?:[ ]?[A-Z0-9]{1,3})?\b").expect("iban regex compiles")
});

/// Find all IBAN spans in `text`, validated against the country-code allowlist
/// and length range. The ISO 13616 mod-97 checksum runs later, post-aggregation,
/// via [`crate::text::redaction::validators::iban::IbanChecksumValidator`] — it
/// needs no regex-adjacent context, so it is not duplicated here.
pub fn find_all(text: &str) -> Vec<PatternMatch> {
    let upper = text.to_ascii_uppercase();

    RE_IBAN
        .find_iter(&upper)
        .filter_map(|m| {
            let raw = &upper[m.start()..m.end()];
            let cc = &raw[..2];
            if !IBAN_COUNTRIES.contains(&cc) {
                return None;
            }
            // Strip whitespace and verify total length is within the IBAN range (15-34 chars).
            let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
            if !(15..=34).contains(&compact.len()) {
                return None;
            }
            Some(PatternMatch {
                start: m.start(),
                end: m.end(),
                category: PiiCategory::Iban,
                text: text[m.start()..m.end()].to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_valid_iban_is_detected() {
        // Checksum validity is not checked here — that is now
        // `validators::iban::IbanChecksumValidator`'s job, applied
        // post-aggregation. This only exercises country-code + length shape
        // filtering.
        let matches = find_all("IBAN: FR7630006000011234567890189 for transfer.");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].category, PiiCategory::Iban);
        assert_eq!(matches[0].text, "FR7630006000011234567890189");
    }

    #[test]
    fn does_not_bleed_into_trailing_uppercase_words() {
        // Regression test: the regex must stop at the end of the real IBAN and
        // not swallow following all-caps words that happen to look like
        // continued alphanumeric BBAN characters.
        let matches = find_all("Pay DE89 3704 0044 0532 0130 00 via SEPA.");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "DE89 3704 0044 0532 0130 00");
    }
}
