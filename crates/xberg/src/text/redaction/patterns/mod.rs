//! Pure-Rust regex pattern engine for PII detection.
//!
//! Each submodule owns one [`PiiCategory`]
//! and exposes a `find_all(text) -> Vec<PatternMatch>` function. The dispatcher
//! [`scan_text`] selects the requested patterns and returns merged matches.
//!
//! Patterns are intentionally conservative: false positives are worse than a
//! missed match because downstream callers will display the redacted text to
//! end users. Shape validation feasible at scan time (ISO country-code +
//! length prefix for IBAN, digit-count for credit cards, area lookup for SSN)
//! is applied here. Checksum-style validation that needs no regex-adjacent
//! context (Luhn for credit cards, ISO 13616 mod-97 for IBAN) runs later,
//! post-aggregation, via [`crate::text::redaction::validators`] — see
//! [`crate::text::redaction::validators::EntityValidator`].
//!
//! Pattern source attribution: regex shapes are derived from `censgate/redact`
//! (Apache-2.0) and adapted to the Rust `regex` crate (no look-around) while
//! keeping the same coverage envelope.

pub mod credit_card;
pub mod email;
pub mod iban;
pub mod ip_address;
pub mod phone;
pub mod postal_code;
pub mod ssn;
pub mod swift_bic;

use crate::types::redaction::PiiCategory;

/// One detected PII span in the input text.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PatternMatch {
    /// Inclusive byte-offset start of the match in the source text.
    pub start: usize,
    /// Exclusive byte-offset end of the match.
    pub end: usize,
    /// Category the match belongs to.
    pub category: PiiCategory,
    /// Matched substring (owned copy — pattern engine returns owned data so the
    /// caller can free the original text if needed before replacement).
    pub text: String,
}

/// Scan `text` for every PII category in `categories` and return all matches
/// in source-byte order.
///
/// When `categories` is empty every supported regex-detectable category fires.
/// Person / Organization / Location are *not* covered by the pattern engine —
/// they must be supplied by a NER backend through the redaction engine.
pub fn scan_text(text: &str, categories: &[PiiCategory]) -> Vec<PatternMatch> {
    let active: Vec<PiiCategory> = if categories.is_empty() {
        default_pattern_categories()
    } else {
        categories.to_vec()
    };

    let mut all = Vec::new();
    for category in &active {
        match category {
            PiiCategory::Email => all.extend(email::find_all(text)),
            PiiCategory::Phone => all.extend(phone::find_all(text)),
            PiiCategory::Ssn => all.extend(ssn::find_all(text)),
            PiiCategory::CreditCard => all.extend(credit_card::find_all(text)),
            PiiCategory::PostalCode => all.extend(postal_code::find_all(text)),
            PiiCategory::IpAddress => all.extend(ip_address::find_all(text)),
            PiiCategory::Iban => all.extend(iban::find_all(text)),
            PiiCategory::SwiftBic => all.extend(swift_bic::find_all(text)),
            // Pattern engine cannot identify free-form text categories.
            PiiCategory::Person
            | PiiCategory::Organization
            | PiiCategory::Location
            | PiiCategory::DateOfBirth
            | PiiCategory::Custom(_) => {}
        }
    }
    all.sort_by_key(|m| m.start);
    all
}

/// Default set of pattern-detectable categories — used when callers leave the
/// configured category set empty.
fn default_pattern_categories() -> Vec<PiiCategory> {
    vec![
        PiiCategory::Email,
        PiiCategory::Phone,
        PiiCategory::Ssn,
        PiiCategory::CreditCard,
        PiiCategory::PostalCode,
        PiiCategory::IpAddress,
        PiiCategory::Iban,
        PiiCategory::SwiftBic,
    ]
}
