//! ISO 13616 mod-97 IBAN checksum validator.
//!
//! Moved out of [`super::super::patterns::iban::find_all`] so the checksum
//! runs once per surviving candidate, post-aggregation, instead of inline
//! during the regex scan. Country-code allowlisting and length validation
//! remain in `patterns::iban` — those are shape checks appropriate at scan
//! time; the checksum needs no regex-adjacent context and belongs here.

use super::{EntityValidator, ValidationResult};
use crate::text::redaction::patterns::PatternMatch;

/// Validates IBAN candidates against the ISO 13616 mod-97 checksum.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(alef, alef(skip))]
pub struct IbanChecksumValidator;

impl EntityValidator for IbanChecksumValidator {
    fn label(&self) -> &'static str {
        "Iban"
    }

    fn validate(&self, entity: &PatternMatch, _ctx: &str) -> ValidationResult {
        // Candidate text preserves the original casing/spacing from the
        // source document; normalise the same way `patterns::iban::find_all`
        // used to before checking the checksum.
        let compact: String = entity
            .text
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| c.to_ascii_uppercase())
            .collect();
        if iban_checksum_valid(&compact) {
            ValidationResult::Accept
        } else {
            ValidationResult::Reject {
                reason: "iban_checksum_failed",
            }
        }
    }
}

/// ISO 13616 IBAN checksum: move the first 4 characters to the end, convert
/// letters to numbers (A=10, B=11, ... Z=35), and verify the resulting
/// number mod 97 equals 1. Rejects the ~1-in-100 non-checksum-valid strings
/// that the country-code + length filter alone lets through.
fn iban_checksum_valid(compact: &str) -> bool {
    if compact.len() < 4 {
        return false;
    }
    let rearranged = format!("{}{}", &compact[4..], &compact[..4]);
    let mut remainder: u64 = 0;
    for c in rearranged.chars() {
        let value = if c.is_ascii_digit() {
            c.to_digit(10).unwrap_or(0) as u64
        } else if c.is_ascii_uppercase() {
            (c as u64) - ('A' as u64) + 10
        } else {
            return false;
        };
        // Fold digit-by-digit (or two-digit for letters) to avoid overflow
        // on IBANs up to 34 chars (~68 decimal digits after expansion).
        let digits = if value >= 10 { 2 } else { 1 };
        remainder = (remainder * 10u64.pow(digits) + value) % 97;
    }
    remainder == 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::redaction::PiiCategory;

    fn entity_with(text: &str) -> PatternMatch {
        PatternMatch {
            start: 0,
            end: text.len(),
            category: PiiCategory::Iban,
            text: text.to_string(),
        }
    }

    #[test]
    fn label_is_iban() {
        assert_eq!(IbanChecksumValidator.label(), "Iban");
    }

    #[test]
    fn accepts_checksum_valid_iban() {
        assert_eq!(
            IbanChecksumValidator.validate(&entity_with("FR7630006000011234567890189"), ""),
            ValidationResult::Accept
        );
    }

    #[test]
    fn rejects_checksum_invalid_iban() {
        // Same IBAN as above with the last BBAN digit flipped (9 -> 8).
        assert_eq!(
            IbanChecksumValidator.validate(&entity_with("FR7630006000011234567890188"), ""),
            ValidationResult::Reject {
                reason: "iban_checksum_failed"
            }
        );
    }

    #[test]
    fn accepts_checksum_valid_iban_with_pretty_print_spaces_and_lowercase() {
        // Pretty-printed IBANs keep the four-character space groups and may
        // appear lowercase in source text; the validator must normalise both
        // before checking the checksum.
        assert_eq!(
            IbanChecksumValidator.validate(&entity_with("de89 3704 0044 0532 0130 00"), ""),
            ValidationResult::Accept
        );
    }

    #[test]
    fn rejects_too_short_input() {
        assert_eq!(
            IbanChecksumValidator.validate(&entity_with("FR7"), ""),
            ValidationResult::Reject {
                reason: "iban_checksum_failed"
            }
        );
    }
}
