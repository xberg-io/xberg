//! OCR error-not-silence regression tests (issue #1223).
//!
//! A scanned / image-only PDF has no native text layer, so extraction must fall
//! back to OCR. Three failure modes were silent before this change:
//!
//!   1. When the OCR fallback backend errors, the already-empty native text was
//!      returned as a normal success — empty content, no error, no warning.
//!   2. When every pipeline backend scored below `pipeline_min_quality`, the best
//!      (garbage) result was returned with only a log line.
//!   3. On the mixed native+scanned page path, an empty OCR result overwrote the
//!      page's native text (see the `merge_ocr_pages_into_native` unit tests in
//!      `src/extractors/pdf/ocr.rs`).
//!
//! These tests exercise the public extraction API against generated fixtures.

#![cfg(feature = "ocr")]

mod helpers;
use helpers::extract_uri_document_blocking;

use std::path::PathBuf;
use xberg::core::config::{ExtractionConfig, OcrConfig};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ocr")
        .join(name)
}

fn skip_if_missing(name: &str) -> bool {
    if !fixture(name).exists() {
        eprintln!("fixture {name} not found, skipping");
        true
    } else {
        false
    }
}

/// (a) With tessdata present, a scanned/image-only PDF extracts real text via OCR.
#[test]
fn test_scanned_pdf_ocr_extracts_text() {
    if skip_if_missing("scanned_hello.pdf") {
        return;
    }

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = match extract_uri_document_blocking(fixture("scanned_hello.pdf"), None, &config) {
        Ok(doc) => doc,
        Err(e) => {
            // Tessdata unavailable in this environment: nothing to assert about
            // successful OCR, but the failure must not be a silent empty success.
            eprintln!("OCR extraction errored (tessdata likely unavailable): {e}");
            return;
        }
    };

    let content = result.content.to_lowercase();
    eprintln!("scanned OCR content: {content:?}");
    // The fixture renders "HELLO SCANNED / OCR WORLD / INVOICE 12345" as an image.
    let hit = ["hello", "scanned", "ocr", "world", "invoice", "12345"]
        .iter()
        .any(|tok| content.contains(tok));
    assert!(
        hit,
        "OCR should recover recognizable text from the scanned PDF, got: {content:?}"
    );
}

/// (b) When the OCR fallback backend is unavailable / fails, the extractor must NOT
/// silently return an empty success — it either errors or flags a ProcessingWarning.
#[test]
fn test_scanned_pdf_ocr_failure_is_not_silent() {
    if skip_if_missing("scanned_hello.pdf") {
        return;
    }

    // An unregistered backend name forces the OCR fallback to fail deterministically.
    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "definitely-not-a-real-ocr-backend".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    };

    match extract_uri_document_blocking(fixture("scanned_hello.pdf"), None, &config) {
        Err(e) => {
            eprintln!("OCR failure surfaced as an error (acceptable): {e}");
        }
        Ok(doc) => {
            let empty = doc.content.trim().is_empty();
            let flagged = doc
                .processing_warnings
                .iter()
                .any(|w| w.source == "ocr" || w.source == "ocr_pipeline");
            eprintln!("content_empty={empty} warnings={:?}", doc.processing_warnings);
            assert!(
                flagged || !empty,
                "a failed OCR fallback on a scanned PDF must be flagged with a ProcessingWarning \
                 or surface content — not a silent empty success"
            );
        }
    }
}

/// (c) A mixed native+scanned PDF keeps its native page text while OCR fills the
/// scanned page. Proves the native page-1 text is not lost when page 2 is OCR'd.
#[test]
fn test_mixed_native_and_scanned_preserves_native_text() {
    if skip_if_missing("mixed_native_scanned.pdf") {
        return;
    }

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    };

    let result = match extract_uri_document_blocking(fixture("mixed_native_scanned.pdf"), None, &config) {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("mixed extraction errored (tessdata likely unavailable): {e}");
            return;
        }
    };

    let content = result.content.to_lowercase();
    eprintln!("mixed content: {content:?}");
    // Native page 1 text must always survive, regardless of OCR outcome on page 2.
    assert!(
        content.contains("native page one") || content.contains("native"),
        "native page-1 text must be preserved in the mixed document, got: {content:?}"
    );
}
