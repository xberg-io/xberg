//! Email address detection.

use super::{BorrowedMatch, PatternMatch};
use crate::types::redaction::PiiCategory;
use once_cell::sync::Lazy;
use regex::Regex;

static RE_EMAIL: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").expect("email regex compiles"));

/// Find all email address spans in `text`.
pub fn find_all(text: &str) -> Vec<PatternMatch> {
    super::collect_matches(text, &PiiCategory::Email)
}

pub(super) fn visit_matches<'s, E>(
    text: &'s str,
    candidate: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
    eligible: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
) -> Result<(), E> {
    for m in RE_EMAIL.find_iter(text) {
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
