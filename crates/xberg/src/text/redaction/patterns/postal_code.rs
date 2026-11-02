//! Postal-code detection for US ZIP, UK postcode, German PLZ, French CP,
//! and Canadian FSA+LDU.
//!
//! These countries cover the majority of corpora we expect to encounter and
//! have well-defined shapes that survive regex extraction without a country
//! code in scope. Additional locales can be added without breaking changes.

use super::{BorrowedMatch, PatternMatch};
use crate::types::redaction::PiiCategory;
use once_cell::sync::Lazy;
use regex::Regex;

static RE_US: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b\d{5}(?:-\d{4})?\b").expect("us zip regex compiles"));

static RE_UK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Z]{1,2}[0-9][0-9A-Z]?\s?[0-9][A-Z]{2}\b").expect("uk postcode regex compiles"));

static RE_CA: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b[A-Z][0-9][A-Z]\s?[0-9][A-Z][0-9]\b").expect("ca postal regex compiles"));

/// Find all postal code spans in `text` (US ZIP, UK postcode, German PLZ, Canadian FSA, and more).
pub fn find_all(text: &str) -> Vec<PatternMatch> {
    super::collect_matches(text, &PiiCategory::PostalCode)
}

pub(super) fn visit_matches<'s, E>(
    text: &'s str,
    candidate: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
    eligible: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
) -> Result<(), E> {
    let upper = text.to_ascii_uppercase();
    for m in RE_UK.find_iter(&upper) {
        let matched = BorrowedMatch {
            start: m.start(),
            end: m.end(),
            text: &text[m.start()..m.end()],
        };
        candidate(matched)?;
        eligible(matched)?;
    }
    for m in RE_CA.find_iter(&upper) {
        let matched = BorrowedMatch {
            start: m.start(),
            end: m.end(),
            text: &text[m.start()..m.end()],
        };
        candidate(matched)?;
        eligible(matched)?;
    }
    for m in RE_US.find_iter(text) {
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
