//! US Social Security Number detection.
//!
//! Format: `AAA-GG-SSSS` where:
//! - Area number (AAA) is 001-665 or 667-899 (666 and 9xx historically reserved)
//! - Group number (GG) is 01-99
//! - Serial number (SSSS) is 0001-9999
//!
//! Spaces and dashes are accepted between groups. The format check filters out
//! obvious non-SSNs like `000-00-0000` or `666-12-3456`.

use super::{BorrowedMatch, PatternMatch};
use crate::types::redaction::PiiCategory;
use once_cell::sync::Lazy;
use regex::Regex;

static RE_SSN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(\d{3})[\s\-](\d{2})[\s\-](\d{4})\b").expect("ssn regex compiles"));

/// Find all US Social Security Number spans in `text` (format: NNN-NN-NNNN).
pub fn find_all(text: &str) -> Vec<PatternMatch> {
    super::collect_matches(text, &PiiCategory::Ssn)
}

pub(super) fn visit_matches<'s, E>(
    text: &'s str,
    candidate: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
    eligible: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
) -> Result<(), E> {
    for cap in RE_SSN.captures_iter(text) {
        let Some(full) = cap.get(0) else { continue };
        let matched = BorrowedMatch {
            start: full.start(),
            end: full.end(),
            text: full.as_str(),
        };
        candidate(matched)?;
        let Some(area) = cap.get(1).and_then(|m| m.as_str().parse::<u16>().ok()) else {
            continue;
        };
        let Some(group) = cap.get(2).and_then(|m| m.as_str().parse::<u16>().ok()) else {
            continue;
        };
        let Some(serial) = cap.get(3).and_then(|m| m.as_str().parse::<u16>().ok()) else {
            continue;
        };
        if area == 0 || area == 666 || (900..=999).contains(&area) {
            continue;
        }
        if group == 0 || serial == 0 {
            continue;
        }
        eligible(matched)?;
    }
    Ok(())
}
