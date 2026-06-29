//! Smoke tests for the pure-Rust redaction pattern engine.
//!
//! One test per `PiiCategory` that the pattern engine recognises. Each test
//! drives the engine end-to-end through the Late-stage post-processor so the
//! engine + strategy + token rewrite path is exercised together.

#![cfg(feature = "redaction")]

use std::borrow::Cow;

use xberg::ExtractionConfig;
use xberg::core::config::redaction::RedactionConfig;
use xberg::plugins::PostProcessor;
use xberg::plugins::processor::builtin::redaction::RedactionProcessor;
use xberg::types::ExtractedDocument;
use xberg::types::redaction::{PiiCategory, RedactionStrategy};

fn run(content: &str, strategy: RedactionStrategy) -> ExtractedDocument {
    let mut result = ExtractedDocument::default();
    result.content = content.to_string();
    result.mime_type = Cow::Borrowed("text/plain");
    let cfg = ExtractionConfig {
        redaction: Some(RedactionConfig {
            strategy,
            ..RedactionConfig::default()
        }),
        ..Default::default()
    };
    let processor = RedactionProcessor;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime
        .block_on(processor.process(&mut result, &cfg))
        .expect("processor ok");
    result
}

#[test]
fn redacts_email_with_mask_strategy() {
    let result = run("Please email alice@example.com for details.", RedactionStrategy::Mask);
    assert!(result.content.contains("[REDACTED]"));
    assert!(!result.content.contains("alice@example.com"));
    let report = result.redaction_report.expect("report");
    assert_eq!(report.total_redacted, 1);
    assert_eq!(report.findings[0].category, PiiCategory::Email);
}

#[test]
fn redacts_phone_number() {
    let result = run("Call +1-415-555-0123 at noon.", RedactionStrategy::Mask);
    assert!(result.content.contains("[REDACTED]"));
    assert!(!result.content.contains("415"));
    let report = result.redaction_report.expect("report");
    assert!(report.findings.iter().any(|f| f.category == PiiCategory::Phone));
}

#[test]
fn redacts_valid_us_ssn() {
    // Valid SSN: area 123, group 45, serial 6789.
    let result = run("My SSN is 123-45-6789, please file it.", RedactionStrategy::Mask);
    assert!(result.content.contains("[REDACTED]"));
    assert!(!result.content.contains("123-45-6789"));
    let report = result.redaction_report.expect("report");
    assert!(report.findings.iter().any(|f| f.category == PiiCategory::Ssn));
}

#[test]
fn skips_invalid_us_ssn_in_excluded_areas() {
    // 000 area is reserved → must not redact.
    let result = run("Reference 000-12-3456 is not a real SSN.", RedactionStrategy::Mask);
    assert!(!result.content.contains("[REDACTED]"));
    assert!(result.content.contains("000-12-3456"));
}

#[test]
fn redacts_visa_credit_card_with_luhn() {
    // 4111 1111 1111 1111 is the classic Visa test number and Luhn-valid.
    let result = run("Card number: 4111 1111 1111 1111 on file.", RedactionStrategy::Mask);
    assert!(result.content.contains("[REDACTED]"));
    assert!(!result.content.contains("4111"));
    let report = result.redaction_report.expect("report");
    assert!(report.findings.iter().any(|f| f.category == PiiCategory::CreditCard));
}

#[test]
fn skips_invalid_luhn_credit_card() {
    // Restrict to credit-card category so phone/postcode regexes do not fire on the
    // 16-digit run. 1234-1234-1234-1234 fails Luhn → must not redact.
    let mut result = ExtractedDocument::default();
    result.content = "Card: 1234-1234-1234-1234.".to_string();
    result.mime_type = Cow::Borrowed("text/plain");
    let cfg = ExtractionConfig {
        redaction: Some(RedactionConfig {
            strategy: RedactionStrategy::Mask,
            categories: std::iter::once(PiiCategory::CreditCard).collect(),
            ..RedactionConfig::default()
        }),
        ..Default::default()
    };
    let processor = RedactionProcessor;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime
        .block_on(processor.process(&mut result, &cfg))
        .expect("processor ok");
    assert!(!result.content.contains("[REDACTED]"));
}

#[test]
fn redacts_us_postal_code() {
    let result = run("Ship to 90210 by Friday.", RedactionStrategy::Mask);
    let report = result.redaction_report.expect("report");
    assert!(report.findings.iter().any(|f| f.category == PiiCategory::PostalCode));
}

#[test]
fn redacts_uk_postcode() {
    let result = run("Address: SW1A 1AA, London.", RedactionStrategy::Mask);
    let report = result.redaction_report.expect("report");
    assert!(report.findings.iter().any(|f| f.category == PiiCategory::PostalCode));
}

#[test]
fn redacts_ipv4_address() {
    let result = run("Server is at 192.168.1.1 internally.", RedactionStrategy::Mask);
    assert!(!result.content.contains("192.168.1.1"));
    let report = result.redaction_report.expect("report");
    assert!(report.findings.iter().any(|f| f.category == PiiCategory::IpAddress));
}

#[test]
fn redacts_ipv6_address() {
    let result = run("Reach 2001:db8::8a2e:370:7334 via VPN.", RedactionStrategy::Mask);
    let report = result.redaction_report.expect("report");
    assert!(report.findings.iter().any(|f| f.category == PiiCategory::IpAddress));
}

#[test]
fn redacts_german_iban() {
    let result = run("Pay DE89 3704 0044 0532 0130 00 via SEPA.", RedactionStrategy::Mask);
    let report = result.redaction_report.expect("report");
    assert!(report.findings.iter().any(|f| f.category == PiiCategory::Iban));
}

#[test]
fn redacts_swift_bic() {
    let result = run("BIC: DEUTDEFF500 used for routing.", RedactionStrategy::Mask);
    let report = result.redaction_report.expect("report");
    assert!(report.findings.iter().any(|f| f.category == PiiCategory::SwiftBic));
}

#[test]
fn token_replace_reuses_token_for_same_value() {
    let result = run(
        "alice@example.com is alice@example.com — confirmed.",
        RedactionStrategy::TokenReplace,
    );
    // Both mentions should map to the same token (EMAIL_1).
    assert_eq!(result.content.matches("[EMAIL_1]").count(), 2);
    let report = result.redaction_report.expect("report");
    assert_eq!(report.total_redacted, 2);
}

#[test]
fn hash_strategy_yields_deterministic_marker() {
    let result = run("Reach me at alice@example.com.", RedactionStrategy::Hash);
    // Hash markers all start with [HASH: and end with ].
    assert!(result.content.contains("[HASH:"));
    assert!(!result.content.contains("alice@example.com"));
}

#[test]
fn drop_strategy_deletes_the_match() {
    let result = run("Reach me at alice@example.com.", RedactionStrategy::Drop);
    assert!(!result.content.contains("[REDACTED]"));
    assert!(!result.content.contains("alice@example.com"));
}

// ---- User-supplied custom terms / patterns ----------------------------------

fn run_with_config(content: &str, redaction: RedactionConfig) -> ExtractedDocument {
    let mut result = ExtractedDocument::default();
    result.content = content.to_string();
    result.mime_type = Cow::Borrowed("text/plain");
    let cfg = ExtractionConfig {
        redaction: Some(redaction),
        ..Default::default()
    };
    let processor = RedactionProcessor;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime
        .block_on(processor.process(&mut result, &cfg))
        .expect("processor ok");
    result
}

#[test]
fn redacts_user_supplied_literal_term_case_insensitive() {
    use xberg::core::config::redaction::RedactionTerm;

    let redaction = RedactionConfig {
        strategy: RedactionStrategy::Mask,
        custom_terms: vec![RedactionTerm::labeled("project", "Apollo")],
        ..RedactionConfig::default()
    };
    let result = run_with_config("Project apollo launches at noon. APOLLO is the codename.", redaction);
    // Both lowercase + uppercase mentions should be redacted (case-insensitive default).
    assert!(!result.content.contains("apollo"));
    assert!(!result.content.contains("APOLLO"));
    let report = result.redaction_report.expect("report");
    assert_eq!(report.total_redacted, 2);
    assert!(
        report
            .findings
            .iter()
            .all(|f| f.category == PiiCategory::Custom("project".to_string()))
    );
}

#[test]
fn case_sensitive_literal_term_only_matches_exact() {
    use xberg::core::config::redaction::RedactionTerm;

    let term = RedactionTerm {
        label: "code".into(),
        value: "Apollo".into(),
        case_sensitive: true,
    };
    let redaction = RedactionConfig {
        strategy: RedactionStrategy::Mask,
        custom_terms: vec![term],
        ..RedactionConfig::default()
    };
    let result = run_with_config("Apollo, apollo, APOLLO.", redaction);
    // Only the exactly-cased "Apollo" should be redacted.
    let report = result.redaction_report.expect("report");
    assert_eq!(report.total_redacted, 1);
    assert!(result.content.contains("apollo"));
    assert!(result.content.contains("APOLLO"));
    assert!(!result.content.starts_with("Apollo"));
}

#[test]
fn redacts_user_supplied_regex_pattern() {
    use xberg::core::config::redaction::RedactionPattern;

    let redaction = RedactionConfig {
        strategy: RedactionStrategy::Mask,
        custom_patterns: vec![RedactionPattern::labeled("project_id", r"PROJECT-\d{4}")],
        ..RedactionConfig::default()
    };
    let result = run_with_config("Issue PROJECT-1234 is open; project-5678 too.", redaction);
    // Case-insensitive by default; both mentions disappear.
    assert!(!result.content.contains("PROJECT-1234"));
    assert!(!result.content.contains("project-5678"));
    let report = result.redaction_report.expect("report");
    assert_eq!(report.total_redacted, 2);
    assert!(
        report
            .findings
            .iter()
            .all(|f| f.category == PiiCategory::Custom("project_id".to_string()))
    );
}

#[test]
fn invalid_user_regex_pattern_is_rejected_at_validation() {
    use xberg::core::config::redaction::RedactionPattern;

    let bad = RedactionConfig {
        custom_patterns: vec![RedactionPattern::labeled("bad", "(unbalanced")],
        ..RedactionConfig::default()
    };
    assert!(bad.validate().is_err(), "invalid regex must fail validation");
}
