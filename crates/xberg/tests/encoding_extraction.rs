//! Regression tests for xberg-io/xberg#1223: charset handling must be
//! consistent and correct across the text-family extractors — legacy-encoded
//! bytes must not become U+FFFD mojibake, and a UTF-8 BOM must not pollute the
//! first token.

#![cfg(all(feature = "quality", feature = "xml"))]

mod helpers;
use helpers::extract_bytes_document_blocking;

use xberg::core::config::ExtractionConfig;

fn extract(bytes: &[u8], mime: &str) -> String {
    extract_bytes_document_blocking(bytes, mime, &ExtractionConfig::default())
        .expect("extraction must succeed")
        .content
}

/// A Latin-1 (ISO-8859-1) plain-text file must decode its accented bytes, not
/// replace them with U+FFFD.
#[test]
fn plain_text_latin1_decodes_without_replacement() {
    // "café naïve résumé" in ISO-8859-1.
    let latin1: &[u8] = b"caf\xe9 na\xefve r\xe9sum\xe9";
    let content = extract(latin1, "text/plain");
    assert!(
        !content.contains('\u{FFFD}'),
        "Latin-1 text must not decode to replacement chars: {content:?}"
    );
    assert!(
        content.contains("café") && content.contains("résumé"),
        "expected accented words: {content:?}"
    );
}

/// An XML document whose declaration says ISO-8859-1 must honor it.
#[test]
fn xml_honors_encoding_declaration() {
    let xml: &[u8] = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><doc><name>caf\xe9</name></doc>";
    let content = extract(xml, "application/xml");
    assert!(
        !content.contains('\u{FFFD}'),
        "declared encoding must be honored: {content:?}"
    );
    assert!(content.contains("café"), "expected decoded accent: {content:?}");
}

/// A UTF-8 CSV with a leading BOM (Excel's default export) must not carry the
/// BOM into the first header cell.
#[test]
fn csv_strips_utf8_bom() {
    let csv: &[u8] = b"\xEF\xBB\xBFName,Age\nAlice,30\n";
    let content = extract(csv, "text/csv");
    assert!(!content.contains('\u{FEFF}'), "UTF-8 BOM must be stripped: {content:?}");
    assert!(content.contains("Name"), "header must be present: {content:?}");
}
