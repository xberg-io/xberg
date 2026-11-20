//! Integration tests for HEIF / HEIC / AVIF support via the public API.
//!
//! Exercises:
//! - `extract_bytes_document` end-to-end for HEIC, HEIF, AVIF inputs (decoded to PNG
//!   internally, then routed through the image extractor; OCR is disabled so
//!   the test is independent of any Tesseract binary).
//! - `list_supported_formats()` reports HEIC / HEIF / AVIF / HEICS / AVCS, so
//!   every language binding picks them up via the single Rust registry.
//!
//! Fixtures come from the repo-root `test_documents/` submodule. No HEIF
//! binaries live in the xberg codebase itself.
//!
//! `test_documents/` is a bucket-fetched corpus (see
//! `test_documents/scripts/fetch_corpus.py`) and is not present in a bare
//! checkout, so fixtures are read at runtime rather than baked in via
//! `include_bytes!`. Local runs can skip unavailable fixtures. Runs with
//! `XBERG_REQUIRE_TEST_FIXTURES=1` require them.

#![cfg(feature = "heic")]
#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test/bench binaries print by design; org logging policy exempts tests

mod helpers;
use helpers::{extract_bytes_document, get_test_file_path, skip_if_missing};

use xberg::FormatMetadata;
use xberg::core::config::ExtractionConfig;
use xberg::core::mime::list_supported_formats;

fn load_fixture(relative: &str) -> Option<Vec<u8>> {
    if skip_if_missing(relative) {
        eprintln!(
            "SKIP: fixture {relative} not available; run `python3 test_documents/scripts/fetch_corpus.py` to fetch it"
        );
        return None;
    }

    let path = get_test_file_path(relative);
    Some(
        std::fs::read(&path).unwrap_or_else(|error| panic!("could not read image fixture {}: {error}", path.display())),
    )
}

fn ocr_disabled_config() -> ExtractionConfig {
    ExtractionConfig {
        disable_ocr: true,
        ..ExtractionConfig::default()
    }
}

async fn extract_image_metadata(bytes: &[u8], mime: &str) -> xberg::ImageMetadata {
    let config = ocr_disabled_config();
    let result = extract_bytes_document(bytes, mime, &config)
        .await
        .unwrap_or_else(|e| panic!("extract_bytes_document failed for {mime}: {e}"));
    match result.metadata.format {
        Some(FormatMetadata::Image(img)) => img,
        other => panic!("expected ImageMetadata for {mime}, got {other:?}"),
    }
}

#[tokio::test]
async fn extract_bytes_handles_heic() {
    let Some(bytes) = load_fixture("images/test.heic") else {
        return;
    };
    let img = extract_image_metadata(&bytes, "image/heic").await;
    assert!(img.width > 0, "HEIC width should be > 0");
    assert!(img.height > 0, "HEIC height should be > 0");
    assert_eq!(img.format, "HEIF");
}

#[tokio::test]
async fn extract_bytes_handles_heif() {
    let Some(bytes) = load_fixture("images/test.heif") else {
        return;
    };
    let img = extract_image_metadata(&bytes, "image/heif").await;
    assert!(img.width > 0);
    assert!(img.height > 0);
    assert_eq!(img.format, "HEIF");
}

#[tokio::test]
async fn extract_bytes_handles_avif() {
    let Some(bytes) = load_fixture("images/test.avif") else {
        return;
    };
    let img = extract_image_metadata(&bytes, "image/avif").await;
    assert!(img.width > 0);
    assert!(img.height > 0);
    assert_eq!(img.format, "HEIF");
}

#[test]
fn heif_mime_types_and_extensions_are_registered() {
    let formats = list_supported_formats();
    let mimes: std::collections::HashSet<_> = formats.iter().map(|f| f.mime_type.as_str()).collect();
    for mime in ["image/heic", "image/heif", "image/avif", "image/avcs"] {
        assert!(mimes.contains(mime), "{mime} missing from list_supported_formats()");
    }

    let exts: std::collections::HashSet<_> = formats.iter().map(|f| f.extension.as_str()).collect();
    for ext in ["heic", "heics", "heif", "avif", "avcs"] {
        assert!(exts.contains(ext), "extension `{ext}` missing");
    }
}
