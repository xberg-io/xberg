//! Binary Word 97 list identity survives headings; explicit restarts remain independent.

#![cfg(feature = "office")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OutputFormat};

fn extract(name: &str, output_format: OutputFormat) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/office/doc_numbering_2857")
        .join(format!("{name}.doc"));
    let bytes = std::fs::read(path).expect("synthetic Word 97 fixture must exist");
    assert_eq!(&bytes[..8], &[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]);
    let mut compound = cfb::CompoundFile::open(std::io::Cursor::new(&bytes)).expect("valid OLE container");
    assert!(compound.open_stream("/WordDocument").is_ok());
    assert!(compound.open_stream("/1Table").is_ok());
    extract_bytes_document_blocking(
        &bytes,
        "application/msword",
        &ExtractionConfig { output_format, use_cache: false, ..Default::default() },
    )
    .expect("binary DOC extraction succeeds")
    .content
}

fn numbered_lines(content: &str) -> Vec<&str> {
    content.lines().map(str::trim).filter(|line| line.contains("paragraph.")).collect()
}

#[test]
fn numbered_doc_paragraphs_continue_across_three_headings() {
    for format in [OutputFormat::Markdown, OutputFormat::Plain] {
        let content = extract("continuous", format.clone());
        assert_eq!(numbered_lines(&content), vec![
            "1. First paragraph.", "2. Second paragraph.", "3. Third paragraph.",
            "4. Fourth paragraph.", "5. Fifth paragraph.", "6. Sixth paragraph.",
        ], "{content}");
        if format == OutputFormat::Markdown {
            assert_eq!(content.lines().filter(|line| line.starts_with("# ")).count(), 3, "{content}");
        }
    }
}

#[test]
fn explicit_doc_restart_after_heading_starts_at_one() {
    let content = extract("restart", OutputFormat::Markdown);
    assert_eq!(numbered_lines(&content), vec![
        "1. First paragraph.", "2. Second paragraph.", "1. Third paragraph.", "2. Fourth paragraph.",
    ], "{content}");
}

#[test]
fn independent_doc_list_after_heading_starts_at_one() {
    let content = extract("new_list", OutputFormat::Markdown);
    assert_eq!(numbered_lines(&content), vec![
        "1. First paragraph.", "2. Second paragraph.", "1. Third paragraph.", "2. Fourth paragraph.",
    ], "{content}");
}

#[test]
fn doc_list_nondefault_start_continues_after_heading() {
    let content = extract("start_seven", OutputFormat::Markdown);
    assert_eq!(numbered_lines(&content), vec![
        "7. Seventh paragraph.", "8. Eighth paragraph.", "9. Ninth paragraph.",
    ], "{content}");
}
