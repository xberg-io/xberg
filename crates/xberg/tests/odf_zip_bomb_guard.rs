//! Regression test: ODT and ODP opened their ZIP containers with no validation at all.
//!
//! `OdtExtractor::extract_content` and `OdpExtractor::extract_content` each call
//! `zip::ZipArchive::new` twice -- once for the document body, once again for
//! metadata (`meta.xml`) -- and neither open was ever run through
//! `ZipBombValidator`. A crafted `.odt`/`.odp` could declare an extreme
//! compression ratio (a classic zip bomb) or an absurd file count and sail
//! straight through to full decompression, with `SecurityLimits` never
//! consulted.
//!
//! The fix adds `ZipBombValidator::new(limits).validate(&mut archive)` right
//! after *both* `zip::ZipArchive::new` calls in each extractor, matching the
//! pattern already used by `hwpx.rs` and `iwork/mod.rs`. Against unfixed code,
//! every "must be rejected" test below instead returns `Ok` (the archive
//! extracts normally), so the `expect_err` calls fail.
//!
//! The positive-control tests are the important half: a validator that
//! rejects everything passes every negative test here, so both formats also
//! get an ordinary, small, real document that must extract with its exact
//! text intact.

#![cfg(feature = "office")]

use std::io::Write;
use xberg::{ExtractedDocument, ExtractionConfig, Result, SecurityLimits, XbergError};
use zip::write::{FileOptions, ZipWriter};

mod helpers;
use helpers::extract_bytes_document_blocking;

const ODT_MIME: &str = "application/vnd.oasis.opendocument.text";
const ODP_MIME: &str = "application/vnd.oasis.opendocument.presentation";

/// Build a minimal well-formed ODT whose `office:text` body is exactly `body_xml`.
fn odt_zip_with_extra_entries(body_xml: &str, extra_entries: &[(&str, &[u8])]) -> Vec<u8> {
    let content_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
    xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
    xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:text>{body_xml}</office:text>
  </office:body>
</office:document-content>"#
    );

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let stored = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(ODT_MIME.as_bytes()).unwrap();

        let deflated = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("content.xml", deflated).unwrap();
        zip.write_all(content_xml.as_bytes()).unwrap();

        for (name, data) in extra_entries {
            zip.start_file(*name, deflated).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    buf
}

/// Build a minimal well-formed ODP with one slide containing `slide_text`.
fn odp_zip_with_extra_entries(slide_text: &str, extra_entries: &[(&str, &[u8])]) -> Vec<u8> {
    let content_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
    xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
    xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
    xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
  <office:body>
    <office:presentation>
      <draw:page draw:name="Slide1"><draw:frame><draw:text-box>
        <text:p>{slide_text}</text:p>
      </draw:text-box></draw:frame></draw:page>
    </office:presentation>
  </office:body>
</office:document-content>"#
    );

    let mut buf = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buf));
        let stored = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);
        zip.start_file("mimetype", stored).unwrap();
        zip.write_all(ODP_MIME.as_bytes()).unwrap();

        let deflated = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("content.xml", deflated).unwrap();
        zip.write_all(content_xml.as_bytes()).unwrap();

        for (name, data) in extra_entries {
            zip.start_file(*name, deflated).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }
    buf
}

/// A payload that compresses far past any sane ratio: one repeated byte,
/// deflated. 32 MiB is the smallest member the ratio cap applies to (GH#3416),
/// far short of the hundreds of megabytes a real attack would use -- the code
/// path under test does not care about scale beyond that, only about the ratio
/// comparison firing.
fn compressible_bomb_payload() -> Vec<u8> {
    vec![0u8; 32 * 1024 * 1024]
}

fn low_ratio_limits() -> SecurityLimits {
    SecurityLimits {
        max_compression_ratio: 5,
        ..Default::default()
    }
}

fn low_file_count_limits() -> SecurityLimits {
    SecurityLimits {
        max_files_in_archive: 2,
        ..Default::default()
    }
}

fn assert_is_security_error(result: Result<ExtractedDocument>, must_contain: &str) {
    let error = result.expect_err("a security-limit-violating archive must be rejected");
    assert!(
        matches!(error, XbergError::Security { .. }),
        "expected XbergError::Security, got: {error:?}"
    );
    let message = error.to_string();
    assert!(
        message.contains(must_contain),
        "expected the error to name the specific violation ({must_contain:?}), got: {message}"
    );
}

fn config_with_limits(limits: SecurityLimits) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        security_limits: Some(limits),
        ..Default::default()
    }
}

fn default_config_no_cache() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ..Default::default()
    }
}

#[test]
fn test_odt_rejects_high_compression_ratio_archive() {
    let bytes = odt_zip_with_extra_entries("<text:p>hi</text:p>", &[("bomb.bin", &compressible_bomb_payload())]);
    let config = config_with_limits(low_ratio_limits());
    let result = extract_bytes_document_blocking(&bytes, ODT_MIME, &config);
    assert_is_security_error(result, "ZIP bomb");
}

#[test]
fn test_odp_rejects_high_compression_ratio_archive() {
    let bytes = odp_zip_with_extra_entries("hi", &[("bomb.bin", &compressible_bomb_payload())]);
    let config = config_with_limits(low_ratio_limits());
    let result = extract_bytes_document_blocking(&bytes, ODP_MIME, &config);
    assert_is_security_error(result, "ZIP bomb");
}

#[test]
fn test_odt_rejects_archive_exceeding_file_count() {
    // mimetype + content.xml + two extra entries = 4 files, over the 2-file cap.
    let bytes = odt_zip_with_extra_entries("<text:p>hi</text:p>", &[("extra1.txt", b"a"), ("extra2.txt", b"b")]);
    let config = config_with_limits(low_file_count_limits());
    let result = extract_bytes_document_blocking(&bytes, ODT_MIME, &config);
    assert_is_security_error(result, "too many files");
}

#[test]
fn test_odp_rejects_archive_exceeding_file_count() {
    let bytes = odp_zip_with_extra_entries("hi", &[("extra1.txt", b"a"), ("extra2.txt", b"b")]);
    let config = config_with_limits(low_file_count_limits());
    let result = extract_bytes_document_blocking(&bytes, ODP_MIME, &config);
    assert_is_security_error(result, "too many files");
}

/// Positive control: an ordinary, small ODT with default security limits must
/// still extract, and its text must match exactly. A validator that rejects
/// every archive (an over-eager fix) would fail this, not just the bomb
/// tests above.
#[test]
fn test_ordinary_odt_extracts_exact_text() {
    let bytes = odt_zip_with_extra_entries("<text:p>The quick brown fox jumps over the lazy dog.</text:p>", &[]);
    let result = extract_bytes_document_blocking(&bytes, ODT_MIME, &default_config_no_cache())
        .expect("an ordinary ODT with default security limits must extract");
    assert!(
        result.content.contains("The quick brown fox jumps over the lazy dog."),
        "extracted text must contain the source paragraph's exact text, got: {:?}",
        result.content
    );
}

/// Positive control for ODP: same reasoning as the ODT one above.
#[test]
fn test_ordinary_odp_extracts_exact_text() {
    let bytes = odp_zip_with_extra_entries("Hello from slide one", &[]);
    let result = extract_bytes_document_blocking(&bytes, ODP_MIME, &default_config_no_cache())
        .expect("an ordinary ODP with default security limits must extract");
    assert!(
        result.content.contains("Hello from slide one"),
        "extracted text must contain the slide's exact text, got: {:?}",
        result.content
    );
}
