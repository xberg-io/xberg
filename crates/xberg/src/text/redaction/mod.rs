//! Redaction & anonymisation engine.
//!
//! The engine is invoked from the Late-stage post-processor at
//! [`crate::plugins::processor::builtin::redaction`]. It runs the pure-Rust
//! pattern engine (and optionally a NER backend for PERSON / ORGANIZATION /
//! LOCATION) over [`ExtractedDocument::content`](crate::ExtractedDocument::content)
//! and rewrites every textual field in place. The original text is dropped at
//! the end of the pipeline; the audit trail lives in
//! [`ExtractedDocument::redaction_report`](crate::ExtractedDocument::redaction_report).

pub mod engine;
pub mod eval;
pub mod patterns;
#[cfg(feature = "redaction-rehydrate")]
pub mod rehydration;
pub mod strategy;
pub mod validators;

pub use engine::redact;
#[cfg(feature = "redaction-rehydrate")]
pub use engine::{TextRedactionOutcome, redact_capturing_rehydration_map};
#[cfg(feature = "redaction-rehydrate")]
pub use rehydration::{RehydrationMap, SubjectMatch, decrypt_map, encrypt_map, find_subject, forget_subject};
pub use validators::{EntityValidator, RejectionCounts, ValidationResult};
