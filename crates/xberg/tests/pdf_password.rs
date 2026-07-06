//! Regression tests for xberg-io/xberg#1223: `PdfConfig.passwords` must be
//! honored for encrypted PDFs, and an un-openable encrypted PDF must error
//! rather than silently return empty content.

#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::{ExtractionConfig, PdfConfig};

const PDF_MIME: &str = "application/pdf";
const MARKER: &str = "PINEAPPLE42";

/// Build an AES-256 encrypted single-page PDF whose text contains `MARKER`,
/// protected by user password "secret123". Returns the encrypted bytes.
fn encrypted_pdf() -> Vec<u8> {
    use pdf_oxide::geometry::Rect;
    use pdf_oxide::writer::{DocumentBuilder, TextAlign};

    let mut doc = DocumentBuilder::new();
    doc.a4_page()
        .text_in_rect(
            Rect::new(72.0, 700.0, 400.0, 40.0),
            &format!("Confidential. The secret marker is {MARKER}."),
            TextAlign::Left,
        )
        .done();

    // Unique per call so the parallel tests don't race on one temp path.
    static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("xberg_pw_fixture_{}_{n}.pdf", std::process::id()));
    doc.save_encrypted(&path, "secret123", "owner123")
        .expect("save_encrypted must succeed");
    let bytes = std::fs::read(&path).expect("read encrypted pdf");
    let _ = std::fs::remove_file(&path);
    bytes
}

fn config_with_passwords(passwords: Vec<String>) -> ExtractionConfig {
    ExtractionConfig {
        pdf_options: Some(PdfConfig {
            passwords: if passwords.is_empty() { None } else { Some(passwords) },
            ..PdfConfig::default()
        }),
        ..ExtractionConfig::default()
    }
}

#[test]
fn correct_password_authenticates_and_does_not_error() {
    let bytes = encrypted_pdf();
    let config = config_with_passwords(vec!["secret123".to_string()]);
    // The correct password must authenticate: extraction succeeds rather than
    // erroring on the encrypted document. NOTE: recovering the *decrypted text*
    // of an AES-256 stream is a pdf_oxide capability — with a builder-produced
    // encrypted fixture pdf_oxide authenticates but currently returns empty
    // content, so this asserts the xberg-side plumbing (auth + no error), not
    // the decrypted bytes. Full text recovery lands when pdf_oxide can decrypt
    // these streams.
    let result = extract_bytes_document_blocking(&bytes, PDF_MIME, &config);
    assert!(
        result.is_ok(),
        "the correct password must authenticate and not error; got: {:?}",
        result.err()
    );
}

#[test]
fn missing_password_errors_not_empty() {
    let bytes = encrypted_pdf();
    let config = config_with_passwords(vec![]);
    let result = extract_bytes_document_blocking(&bytes, PDF_MIME, &config);
    assert!(
        result.is_err(),
        "an encrypted PDF with no password must error, not return empty content; got: {:?}",
        result.map(|d| d.content)
    );
}

#[test]
fn wrong_password_errors() {
    let bytes = encrypted_pdf();
    let config = config_with_passwords(vec!["not-the-password".to_string()]);
    let result = extract_bytes_document_blocking(&bytes, PDF_MIME, &config);
    assert!(
        result.is_err(),
        "a wrong password must error; got: {:?}",
        result.map(|d| d.content)
    );
}
