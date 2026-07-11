//! Redaction & anonymisation output types.
//!
//! Produced by the redaction post-processor
//! (`crates/xberg/src/text/redaction/`) and attached to
//! [`ExtractedDocument::redaction_report`](super::extraction::ExtractedDocument::redaction_report).
//!
//! Redaction is a **Late-stage** post-processor: it always runs after NER,
//! summarisation, translation, page classification, and captioning have populated
//! their own fields. The processor rewrites `result.content`, `result.formatted_content`,
//! every `result.chunks[i].content`, and the textual fields of `result.entities`,
//! `summary`, `translation`, `page_classifications`. The original text never appears in
//! the returned `ExtractedDocument` — this struct is the audit trail of what was found.

use serde::{Deserialize, Serialize};

/// Audit report describing what the redaction processor found and how it replaced it.
///
/// The redactor returns this alongside the rewritten content so compliance, replay, and
/// audit-log consumers can see exactly what fired. Offsets are relative to the *original*
/// pre-redaction `content` and are intended for audit reconstruction only — the original
/// bytes are dropped at the end of the pipeline.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct RedactionReport {
    /// Individual redaction findings in original-source byte order.
    pub findings: Vec<RedactionFinding>,
    /// Total number of redactions applied across the document.
    pub total_redacted: u32,
    /// Post-detection validator rejection counts for the main document
    /// content (e.g. a failed-checksum IBAN or a failed-Luhn card number),
    /// keyed by reason. Audit-only: rejected candidates never appear in
    /// `findings` — the validator determined they were never PII in the
    /// first place, so nothing was redacted for them.
    #[serde(default)]
    #[cfg_attr(alef, alef(skip))]
    pub rejection_counts: Vec<RejectionCount>,
}

/// One rejection-reason tally emitted by the redaction engine's
/// post-detection validators (see
/// [`crate::text::redaction::validators::EntityValidator`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct RejectionCount {
    /// Static reason identifier reported by the validator (e.g.
    /// `"iban_checksum_failed"`). Never contains the underlying PII text.
    pub reason: String,
    /// Number of candidates rejected for this reason.
    pub count: u32,
}

/// One redaction event: which span was rewritten, why, and with what.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct RedactionFinding {
    /// Byte-offset start in the original (pre-redaction) `ExtractedDocument::content`.
    pub start: u32,
    /// Byte-offset end (exclusive) in the original `ExtractedDocument::content`.
    pub end: u32,
    /// PII category that fired this redaction.
    pub category: PiiCategory,
    /// Strategy applied to this finding (mask, hash, token-replace, drop).
    pub strategy: RedactionStrategy,
    /// String that replaced the original mention. Always present; for `Drop` the
    /// replacement is the empty string.
    pub replacement_token: String,
}

/// Strategy applied when a PII match is rewritten.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RedactionStrategy {
    /// Replace the matched span with a fixed mask token (default `"[REDACTED]"`).
    #[default]
    Mask,
    /// Replace with a SHA-256 hash of the original value (truncated to 16 hex chars).
    /// Lets downstream consumers do equality joins without recovering the source.
    Hash,
    /// Replace with a per-category running token (`"[PERSON_1]"`, `"[PERSON_2]"`, …)
    /// so the same person referenced twice gets the same token within the document.
    TokenReplace,
    /// Delete the matched span entirely.
    Drop,
}

/// PII categories the pattern engine recognises.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum PiiCategory {
    /// Email address (e.g. `user@example.com`).
    Email,
    /// Phone number in any common format.
    Phone,
    /// US Social Security Number.
    Ssn,
    /// Payment card number (Visa, Mastercard, Amex, etc.).
    CreditCard,
    /// Postal / ZIP code.
    PostalCode,
    /// IPv4 or IPv6 address.
    IpAddress,
    /// International Bank Account Number.
    Iban,
    /// SWIFT / BIC bank identifier code.
    SwiftBic,
    /// Date of birth.
    DateOfBirth,
    /// Person name, surfaced by the optional NER backend.
    Person,
    /// Organization name, surfaced by the optional NER backend.
    Organization,
    /// Location, surfaced by the optional NER backend.
    Location,
    /// Caller-supplied custom category (e.g. internal employee IDs).
    ///
    /// Surfaced by the redaction engine when a hit comes from
    /// [`RedactionConfig::custom_terms`](crate::core::config::redaction::RedactionConfig::custom_terms)
    /// or [`RedactionConfig::custom_patterns`](crate::core::config::redaction::RedactionConfig::custom_patterns).
    /// The string is the label passed alongside the term/pattern. Use those
    /// fields rather than constructing `Custom` directly via the
    /// `categories` filter — the pattern engine cannot detect arbitrary text
    /// from a category name alone.
    Custom(String),
}

impl Default for PiiCategory {
    fn default() -> Self {
        Self::Custom(String::new())
    }
}

impl From<String> for PiiCategory {
    fn from(s: String) -> Self {
        match s.as_str() {
            "email" => Self::Email,
            "phone" => Self::Phone,
            "ssn" => Self::Ssn,
            "credit_card" => Self::CreditCard,
            "postal_code" => Self::PostalCode,
            "ip_address" => Self::IpAddress,
            "iban" => Self::Iban,
            "swift_bic" => Self::SwiftBic,
            "date_of_birth" => Self::DateOfBirth,
            "person" => Self::Person,
            "organization" => Self::Organization,
            "location" => Self::Location,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl std::str::FromStr for PiiCategory {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s.to_string()))
    }
}

impl From<String> for RedactionStrategy {
    fn from(s: String) -> Self {
        match s.as_str() {
            "mask" => Self::Mask,
            "hash" => Self::Hash,
            "token_replace" => Self::TokenReplace,
            "drop" => Self::Drop,
            _ => Self::Mask,
        }
    }
}

impl std::str::FromStr for RedactionStrategy {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s.to_string()))
    }
}
