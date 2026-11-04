//! Phone number detection.
//!
//! Matches international E.164 form and North American formats with
//! optional country code, area-code grouping, and common separators
//! (space, dot, hyphen). Length is constrained to 7–15 digits so that
//! generic 4-digit numbers and credit-card chunks are excluded.

use super::{BorrowedMatch, PatternMatch};
use crate::types::redaction::PiiCategory;
use once_cell::sync::Lazy;
use regex::Regex;

static RE_PHONE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?x)
        \b
        (?:\+\d{1,3}[\s.\-]?)?       # optional country code
        (?:\(?\d{2,4}\)?[\s.\-]?)?   # optional area code (may be parenthesised)
        \d{3,4}[\s.\-]?\d{3,4}       # subscriber digits
        \b
        ",
    )
    .expect("phone regex compiles")
});

/// Find all phone number spans in `text` (international E.164 and North American formats).
pub fn find_all(text: &str) -> Vec<PatternMatch> {
    super::collect_matches(text, &PiiCategory::Phone)
}

pub(super) fn visit_matches<'s, E>(
    text: &'s str,
    candidate: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
    eligible: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
) -> Result<(), E> {
    for m in RE_PHONE.find_iter(text) {
        let matched = BorrowedMatch {
            start: m.start(),
            end: m.end(),
            text: &text[m.start()..m.end()],
        };
        candidate(matched)?;
        let raw = m.as_str();
        let digit_count = raw.chars().filter(|c| c.is_ascii_digit()).count();
        if !(7..=15).contains(&digit_count) {
            continue;
        }
        eligible(matched)?;
    }
    Ok(())
}
