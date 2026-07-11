//! Credit card number detection with Luhn checksum validation.
//!
//! Matches 13–19 digit sequences (with optional space/dash separators).
//! Surviving matches are further validated with the Luhn mod-10 algorithm by
//! [`crate::text::redaction::validators::luhn::LuhnValidator`],
//! post-aggregation. Without the Luhn check the false-positive rate on
//! document text is unacceptable.

use super::PatternMatch;
use crate::types::redaction::PiiCategory;
use once_cell::sync::Lazy;
use regex::Regex;

static RE_CC: Lazy<Regex> = Lazy::new(|| {
    // 13–19 digits with optional space or dash separators.
    Regex::new(r"\b(?:\d[ \-]?){12,18}\d\b").expect("credit card regex compiles")
});

/// Find all credit card number spans in `text`. The Luhn checksum runs
/// later, post-aggregation, via
/// [`crate::text::redaction::validators::luhn::LuhnValidator`] — it needs no
/// regex-adjacent context, so it is not duplicated here.
pub fn find_all(text: &str) -> Vec<PatternMatch> {
    RE_CC
        .find_iter(text)
        .filter_map(|m| {
            let raw = m.as_str();
            let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
            if !(13..=19).contains(&digits.len()) {
                return None;
            }
            Some(PatternMatch {
                start: m.start(),
                end: m.end(),
                category: PiiCategory::CreditCard,
                text: raw.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_valid_number_is_detected() {
        // Luhn validity is not checked here — that is now
        // `validators::luhn::LuhnValidator`'s job, applied post-aggregation.
        // This only exercises the 13-19 digit length shape filter.
        let matches = find_all("Card: 4111111111111111 on file.");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].category, PiiCategory::CreditCard);
        assert_eq!(matches[0].text, "4111111111111111");
    }
}
