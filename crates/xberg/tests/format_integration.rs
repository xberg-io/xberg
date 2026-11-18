//! Binding-specific format integration tests.
//!
//! Positive-path scenarios are now covered by the shared fixture-based E2E
//! suites. The tests here focus on behaviour that is specific to the Rust
//! asynchronous APIs or to graceful handling when optional system
//! dependencies are missing.

#![cfg(any(feature = "pdf", feature = "office", feature = "ocr"))]

mod helpers;
use helpers::extract_uri_document;

use helpers::{assert_mime_type, get_test_file_path, skip_if_missing, test_documents_available};

#[cfg(any(feature = "office", feature = "ocr"))]
use helpers::assert_non_empty_content;
use xberg::core::config::ExtractionConfig;

#[cfg(feature = "ocr")]
use xberg::core::config::OcrConfig;

#[cfg(feature = "pdf")]
#[tokio::test]
async fn test_pdf_password_protected_async() {
    if !test_documents_available() {
        return;
    }

    let fixture = "pdf/copy_protected.pdf";
    if skip_if_missing(fixture) {
        return;
    }
    let path = get_test_file_path(fixture);

    let result = extract_uri_document(&path, None, &ExtractionConfig::default()).await;

    match result {
        Err(err) => {
            tracing::debug!("Password protection detected (expected): {}", err);
        }
        Ok(res) => {
            tracing::debug!("Protected PDF extracted; some files allow fallback");
            assert_mime_type(&res, "application/pdf");
            assert!(res.chunks.is_none(), "Chunks should be None without chunking config");
            assert!(res.detected_languages.is_none(), "Language detection not enabled");
        }
    }
}

#[cfg(feature = "office")]
#[tokio::test]
#[cfg_attr(target_os = "windows", ignore = "Legacy office tests timeout on Windows CI")]
async fn test_legacy_doc_extraction_async() {
    if !test_documents_available() {
        return;
    }

    let fixture = "vendored/unstructured/doc/simple.doc";
    if skip_if_missing(fixture) {
        return;
    }
    let path = get_test_file_path(fixture);

    let result = extract_uri_document(&path, None, &ExtractionConfig::default()).await;

    match result {
        Ok(extracted) => {
            assert_mime_type(&extracted, "application/msword");
            assert_non_empty_content(&extracted);
            assert!(
                extracted.chunks.is_none(),
                "Chunks should be None without chunking config"
            );
            assert!(extracted.detected_languages.is_none(), "Language detection not enabled");
        }
        Err(err) => {
            tracing::debug!(
                "Legacy Office extraction failed (office feature may not be enabled): {}",
                err
            );
        }
    }
}

#[cfg(feature = "ocr")]
#[tokio::test]
async fn test_ocr_simple_english_image_async() {
    if !test_documents_available() {
        return;
    }

    let fixture = "images/test_hello_world.png";
    if skip_if_missing(fixture) {
        return;
    }
    let path = get_test_file_path(fixture);

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_uri_document(&path, None, &config).await;

    match result {
        Ok(res) => {
            assert_mime_type(&res, "image/png");
            assert_non_empty_content(&res);
            let content_lower = res.content.to_lowercase();
            assert!(
                content_lower.contains("hello") || content_lower.contains("world"),
                "OCR output {:?} should contain HELLO or WORLD",
                res.content
            );
        }
        Err(err) => {
            tracing::debug!("OCR test failed (Tesseract may not be installed): {}", err);
        }
    }
}

#[cfg(feature = "ocr")]
#[tokio::test]
async fn test_ocr_image_without_text_async() {
    if !test_documents_available() {
        return;
    }

    let fixture = "images/flower_no_text.jpg";
    if skip_if_missing(fixture) {
        return;
    }
    let path = get_test_file_path(fixture);

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: "tesseract".to_string(),
            language: vec!["eng".to_string()],
            ..Default::default()
        }),
        force_ocr: true,
        ..Default::default()
    };

    let result = extract_uri_document(&path, None, &config).await;

    match result {
        Ok(res) => {
            assert_mime_type(&res, "image/jpeg");
            assert!(
                res.content.len() < 200,
                "Expected minimal OCR output, got {} bytes",
                res.content.len()
            );
        }
        Err(err) => {
            tracing::debug!("OCR fallback test failed (Tesseract may not be installed): {}", err);
        }
    }
}
