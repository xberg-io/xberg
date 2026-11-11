//! Plain output of a DOCX file holds the text of the document and nothing else: no style note
//! after a heading, no Markdown link syntax in a table cell, no image placeholder line. Markdown
//! output keeps its links.

#![cfg(feature = "office")]

mod helpers;
use helpers::{extract_uri_document_blocking, get_test_file_path};

use xberg::{ExtractionConfig, OutputFormat};

const WORD_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

fn extract_content(relative_path: &str, output_format: OutputFormat) -> String {
    let path = get_test_file_path(relative_path);
    assert!(path.exists(), "missing test document {}", path.display());
    let config = ExtractionConfig {
        output_format,
        use_cache: false,
        ..Default::default()
    };
    extract_uri_document_blocking(&path, Some(WORD_MIME_TYPE), &config)
        .expect("DOCX extraction should succeed")
        .content
}

#[test]
fn plain_heading_carries_no_style_note() {
    let content = extract_content("docx/budget_fy2027_memo.docx", OutputFormat::Plain);
    assert!(
        content.contains("Interoffice Memorandum"),
        "the heading text is document text: {content}"
    );
    assert!(
        !content.contains("style_name"),
        "Plain content must not carry style notes: {content}"
    );
}

#[test]
fn plain_table_cell_keeps_link_text_and_drops_the_target() {
    let content = extract_content("docx/docx_tables.docx", OutputFormat::Plain);
    assert!(
        content.contains("A link example"),
        "the link text is document text: {content}"
    );
    assert!(
        !content.contains("](") && !content.contains("tabul.com"),
        "Plain content must not carry the link target: {content}"
    );
}

#[test]
fn markdown_table_cell_keeps_its_link() {
    let content = extract_content("docx/docx_tables.docx", OutputFormat::Markdown);
    assert!(
        content.contains("[A link example](https://tabul.com/)"),
        "Markdown output keeps links: {content}"
    );
}

#[test]
fn plain_image_carries_no_placeholder() {
    let content = extract_content("docx/test_emf_docx.docx", OutputFormat::Plain);
    assert!(
        !content.contains("[Image:"),
        "Plain content must not carry an image placeholder: {content}"
    );
}

#[test]
fn plain_table_cell_keeps_its_math_delimiters() {
    let content = extract_content("vendored/docling/docx/table_with_equations.docx", OutputFormat::Plain);
    assert!(
        content.contains(r"$$A=\pi r^{2}$$"),
        "a Plain table cell keeps the notation of its equation: {content}"
    );
}
