//! Luhn mod-10 checksum validator for credit card numbers.
//!
//! Moved out of [`super::super::patterns::credit_card::find_all`] so the
//! checksum runs once per surviving candidate, post-aggregation, instead of
//! inline during the regex scan. The 13-19 digit length check remains in
//! `patterns::credit_card` — that is a shape check appropriate at scan time.

use super::{EntityValidator, ValidationResult};
use crate::text::redaction::patterns::PatternMatch;

/// Validates credit-card-shaped candidates against the Luhn mod-10 checksum.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(alef, alef(skip))]
pub struct LuhnValidator;

impl EntityValidator for LuhnValidator {
    fn label(&self) -> &'static str {
        "CreditCard"
    }

    fn validate(&self, entity: &PatternMatch, _ctx: &str) -> ValidationResult {
        let digits: String = entity.text.chars().filter(|c| c.is_ascii_digit()).collect();
        if luhn_check(&digits) {
            ValidationResult::Accept
        } else {
            ValidationResult::Reject { reason: "luhn_failed" }
        }
    }
}

/// Luhn mod-10 checksum: standard implementation used by Visa/MC/Amex.
fn luhn_check(digits: &str) -> bool {
    if digits.is_empty() {
        return false;
    }
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
    use crate::types::redaction::PiiCategory;

    fn entity_with(text: &str) -> PatternMatch {
        PatternMatch {
            start: 0,
            end: text.len(),
            category: PiiCategory::CreditCard,
            text: text.to_string(),
        }
    }

    #[test]
    fn label_is_credit_card() {
        assert_eq!(LuhnValidator.label(), "CreditCard");
    }

    #[test]
    fn accepts_valid_visa() {
        assert_eq!(
            LuhnValidator.validate(&entity_with("4111111111111111"), ""),
            ValidationResult::Accept
        );
    }

    #[test]
    fn rejects_invalid_checksum() {
        assert_eq!(
            LuhnValidator.validate(&entity_with("4111111111111112"), ""),
            ValidationResult::Reject { reason: "luhn_failed" }
        );
    }

    #[test]
    fn accepts_valid_number_with_separators() {
        assert_eq!(
            LuhnValidator.validate(&entity_with("4111-1111-1111-1111"), ""),
            ValidationResult::Accept
        );
    }

    #[test]
    fn rejects_non_digit_text() {
        assert_eq!(
            LuhnValidator.validate(&entity_with("not-a-card"), ""),
            ValidationResult::Reject { reason: "luhn_failed" }
        );
    }
}
