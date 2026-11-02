//! Credit card number detection with Luhn checksum validation.
//!
//! Matches 13–19 digit sequences (with optional space/dash separators) and
//! validates them with the Luhn mod-10 algorithm. Without the Luhn check the
//! false-positive rate on document text is unacceptable.

use super::{BorrowedMatch, PatternMatch};
use crate::types::redaction::PiiCategory;
use once_cell::sync::Lazy;
use regex::Regex;
use std::convert::Infallible;

static RE_CC: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b(?:\d[ \-]?){12,18}\d\b").expect("credit card regex compiles"));

/// Find all credit card number spans in `text`, validated with the Luhn algorithm.
pub fn find_all(text: &str) -> Vec<PatternMatch> {
    let mut matches = Vec::new();
    let result = visit_matches(text, &mut |_| Ok::<_, Infallible>(()), &mut |matched| {
        matches.push(PatternMatch {
            start: matched.start,
            end: matched.end,
            category: PiiCategory::CreditCard,
            text: matched.text.to_string(),
        });
        Ok(())
    });
    match result {
        Ok(()) => matches,
        Err(never) => match never {},
    }
}

pub(super) fn visit_matches<'s, E>(
    text: &'s str,
    candidate: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
    eligible: &mut impl FnMut(BorrowedMatch<'s>) -> Result<(), E>,
) -> Result<(), E> {
    for m in RE_CC.find_iter(text) {
        let matched = BorrowedMatch {
            start: m.start(),
            end: m.end(),
            text: m.as_str(),
        };
        candidate(matched)?;
        let digits: String = matched.text.chars().filter(|c| c.is_ascii_digit()).collect();
        if !(13..=19).contains(&digits.len()) {
            continue;
        }
        if !luhn_check(&digits) {
            continue;
        }
        eligible(matched)?;
    }
    Ok(())
}

/// Luhn mod-10 checksum: standard implementation used by Visa/MC/Amex.
fn luhn_check(digits: &str) -> bool {
    let mut sum = 0u32;
    let mut alt = false;
    for c in digits.chars().rev() {
        let d = c.to_digit(10).unwrap_or(0);
        let v = if alt {
            let doubled = d * 2;
            if doubled > 9 { doubled - 9 } else { doubled }
        } else {
            d
        };
        sum += v;
        alt = !alt;
    }
    sum.is_multiple_of(10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_luhn_valid_visa() {
        assert!(luhn_check("4111111111111111"));
    }

    #[test]
    fn test_luhn_invalid() {
        assert!(!luhn_check("4111111111111112"));
    }
}

#[cfg(test)]
mod visitor_tests;
