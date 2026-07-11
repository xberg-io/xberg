//! Post-detection entity validators for the redaction pattern engine.
//!
//! Pattern matching (regex + shape checks in [`super::patterns`]) is
//! intentionally permissive about anything that needs surrounding-match
//! context to evaluate (whitespace-insensitive checksums, cross-field
//! lookups). Validators run once, after every text field's matches have been
//! deduplicated, and either accept, reject, or leave a candidate match
//! unchanged. Rejections are aggregated into [`RejectionCounts`] so callers
//! can audit false-positive suppression without retaining the underlying
//! text.
//!
//! Design mirrors the post-aggregation validator pattern used by other
//! Xberg PII pipelines: one [`EntityValidator`] per concern, matched to
//! candidates by category label, with the first rejection short-circuiting
//! the remaining validators for that candidate.

use std::collections::BTreeMap;

use super::patterns::PatternMatch;

/// ISO 13616 mod-97 IBAN checksum validator.
pub mod iban;
/// Luhn mod-10 checksum validator (credit card numbers).
pub mod luhn;

/// Outcome of a single validator on a single candidate match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationResult {
    /// Pass-through, no change.
    Accept,
    /// Drop the candidate. `reason` is a static identifier used as a
    /// [`RejectionCounts`] key (e.g. `"iban_checksum_failed"`).
    Reject {
        /// Static identifier for the rejection cause.
        reason: &'static str,
    },
}

/// A label-targeted deterministic post-detection check.
///
/// Implementations must be `Send + Sync` because the redaction engine may
/// run validators from within async extraction pipelines shared across
/// tasks.
pub trait EntityValidator: Send + Sync + std::fmt::Debug {
    /// The category label this validator applies to, matched against
    /// `format!("{:?}", pattern_match.category)` — the same string the
    /// engine already uses for category-keyed counters.
    fn label(&self) -> &'static str;

    /// Run the check. `ctx` is the original document text the match was
    /// found in.
    fn validate(&self, entity: &PatternMatch, ctx: &str) -> ValidationResult;
}

/// Counter for rejections, keyed by validator reason string.
pub type RejectionCounts = BTreeMap<&'static str, usize>;

/// Apply a chain of validators to a list of candidate matches.
///
/// Only validators whose [`EntityValidator::label`] matches the candidate's
/// category (via `format!("{:?}", category)`) are run against it. The first
/// [`ValidationResult::Reject`] short-circuits the remaining validators for
/// that candidate — later validators targeting the same label are not
/// evaluated.
///
/// Returns the surviving matches (in their original relative order) plus a
/// count of rejections keyed by reason.
#[cfg_attr(alef, alef(skip))]
pub fn apply_validators(
    matches: Vec<PatternMatch>,
    text: &str,
    validators: &[Box<dyn EntityValidator>],
) -> (Vec<PatternMatch>, RejectionCounts) {
    let mut kept = Vec::with_capacity(matches.len());
    let mut counts: RejectionCounts = BTreeMap::new();
    'outer: for m in matches {
        let label = format!("{:?}", m.category);
        for v in validators.iter().filter(|v| v.label() == label) {
            if let ValidationResult::Reject { reason } = v.validate(&m, text) {
                *counts.entry(reason).or_insert(0) += 1;
                continue 'outer;
            }
        }
        kept.push(m);
    }
    (kept, counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::redaction::PiiCategory;

    fn iban_match(text: &str) -> PatternMatch {
        PatternMatch {
            start: 0,
            end: text.len(),
            category: PiiCategory::Iban,
            text: text.to_string(),
        }
    }

    #[derive(Debug)]
    struct AlwaysAccept;
    impl EntityValidator for AlwaysAccept {
        fn label(&self) -> &'static str {
            "test"
        }
        fn validate(&self, _e: &PatternMatch, _c: &str) -> ValidationResult {
            ValidationResult::Accept
        }
    }

    #[derive(Debug)]
    struct AlwaysReject {
        label: &'static str,
        reason: &'static str,
    }
    impl EntityValidator for AlwaysReject {
        fn label(&self) -> &'static str {
            self.label
        }
        fn validate(&self, _e: &PatternMatch, _c: &str) -> ValidationResult {
            ValidationResult::Reject { reason: self.reason }
        }
    }

    #[derive(Debug)]
    struct PanicsIfInvoked {
        label: &'static str,
    }
    impl EntityValidator for PanicsIfInvoked {
        fn label(&self) -> &'static str {
            self.label
        }
        fn validate(&self, _e: &PatternMatch, _c: &str) -> ValidationResult {
            panic!("validator chain must short-circuit before reaching this validator");
        }
    }

    #[derive(Debug)]
    struct RejectIfTextEquals {
        label: &'static str,
        target: &'static str,
        reason: &'static str,
    }
    impl EntityValidator for RejectIfTextEquals {
        fn label(&self) -> &'static str {
            self.label
        }
        fn validate(&self, e: &PatternMatch, _c: &str) -> ValidationResult {
            if e.text == self.target {
                ValidationResult::Reject { reason: self.reason }
            } else {
                ValidationResult::Accept
            }
        }
    }

    #[test]
    fn trait_is_object_safe() {
        let _: Box<dyn EntityValidator> = Box::new(AlwaysAccept);
    }

    #[test]
    fn validation_result_equality() {
        assert_eq!(ValidationResult::Accept, ValidationResult::Accept);
        assert_eq!(
            ValidationResult::Reject { reason: "x" },
            ValidationResult::Reject { reason: "x" },
        );
        assert_ne!(
            ValidationResult::Reject { reason: "x" },
            ValidationResult::Reject { reason: "y" },
        );
    }

    #[test]
    fn apply_validators_empty_validators_keeps_all() {
        let (kept, counts) = apply_validators(vec![iban_match("anything")], "", &[]);
        assert_eq!(kept.len(), 1);
        assert!(counts.is_empty());
    }

    #[test]
    fn apply_validators_short_circuits_on_first_rejection() {
        let validators: Vec<Box<dyn EntityValidator>> = vec![
            Box::new(AlwaysReject {
                label: "Iban",
                reason: "reason_one",
            }),
            Box::new(PanicsIfInvoked { label: "Iban" }),
        ];
        let (kept, counts) = apply_validators(vec![iban_match("XX00")], "", &validators);
        assert!(kept.is_empty());
        assert_eq!(counts.get("reason_one"), Some(&1));
    }

    #[test]
    fn apply_validators_only_runs_validators_matching_label() {
        let validators: Vec<Box<dyn EntityValidator>> = vec![Box::new(AlwaysReject {
            label: "CreditCard",
            reason: "should_not_fire",
        })];
        let (kept, counts) = apply_validators(vec![iban_match("XX00")], "", &validators);
        assert_eq!(kept.len(), 1, "validator targeting a different label must not run");
        assert!(counts.is_empty());
    }

    #[test]
    fn rejection_counts_are_keyed_by_reason_not_by_category() {
        let validators: Vec<Box<dyn EntityValidator>> = vec![
            Box::new(RejectIfTextEquals {
                label: "Iban",
                target: "bad1",
                reason: "reason_one",
            }),
            Box::new(RejectIfTextEquals {
                label: "Iban",
                target: "bad2",
                reason: "reason_two",
            }),
        ];
        let matches = vec![iban_match("bad1"), iban_match("bad2")];
        let (kept, counts) = apply_validators(matches, "", &validators);
        assert!(kept.is_empty());
        assert_eq!(
            counts.len(),
            2,
            "same category, two reasons, must produce two counter entries"
        );
        assert_eq!(counts.get("reason_one"), Some(&1));
        assert_eq!(counts.get("reason_two"), Some(&1));
    }
}
