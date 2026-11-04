//! SWIFT/BIC bank identifier detection.
//!
//! Format: 4-letter bank code + 2-letter ISO country code + 2-character
//! location code + optional 3-character branch code (total 8 or 11 chars).

use super::{BorrowedMatch, PatternMatch};
use crate::types::redaction::PiiCategory;
use once_cell::sync::Lazy;
use regex::Regex;

static RE_SWIFT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Z]{4}[A-Z]{2}[A-Z0-9]{2}(?:[A-Z0-9]{3})?\b").expect("swift regex compiles"));

/// Find all SWIFT/BIC code spans in `text` (8 or 11 uppercase alphanumeric characters).
pub fn find_all(text: &str) -> Vec<PatternMatch> {
    super::collect_matches(text, &PiiCategory::SwiftBic)
}

pub(super) fn visit_matches<'s, E>(
    text: &'s str,
    candidate: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
    eligible: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
) -> Result<(), E> {
    for m in RE_SWIFT.find_iter(text) {
        let matched = BorrowedMatch {
            start: m.start(),
            end: m.end(),
            text: &text[m.start()..m.end()],
        };
        candidate(matched)?;
        eligible(matched)?;
    }
    Ok(())
}
