#![cfg(all(
    feature = "office",
    feature = "pdf",
    feature = "html",
    feature = "redaction",
    feature = "tokio-runtime"
))]

use base64::Engine as _;
use std::io::{Cursor, Read};
use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::types::redaction::RedactionStrategy;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, extract};

const MIXED: &[u8] = b"<p>Jane Doe and Jane <u>Doe</u> end</p>";
const SPLIT_ONLY: &[u8] = b"<p>Call Jane <u>Doe</u> today</p>";
const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

async fn redacted(source: &[u8], format: OutputFormat) -> ExtractedDocument {
    let config = ExtractionConfig {
        output_format: format,
        use_cache: false,
        redaction: Some(RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: vec![RedactionTerm::labeled("person", "Jane Doe")],
            ..Default::default()
        }),
        ..Default::default()
    };
    let response = extract(ExtractInput::from_bytes(source, "text/html", None), &config)
        .await
        .expect("actual public extraction with redaction");
    assert_eq!(
        response.results.len(),
        1,
        "one synthetic source must produce one result"
    );
    response.results.into_iter().next().unwrap()
}

fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

enum Output {
    Binary(Vec<u8>),
    Plain(String),
}

fn verified_plain_fallback(result: &ExtractedDocument, expected: &str) -> String {
    assert_eq!(result.metadata.output_format.as_deref(), Some("plain"));
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "output_format"
                && warning.message.contains("rendering was discarded")
                && warning.message.contains("plain text")),
        "the format downgrade must be explicit: {:?}",
        result.processing_warnings
    );
    assert!(result.formatted_content.is_none());
    let text = normalized(&result.content);
    assert_eq!(text, expected, "the fallback must be the fully redacted source text");
    text
}

async fn package(source: &[u8], format: &str, expected_plain: &str) -> Output {
    let plain = redacted(source, OutputFormat::Plain).await;
    assert_eq!(
        normalized(&plain.content),
        expected_plain,
        "the whole term must match the actual plain source"
    );
    let result = redacted(source, OutputFormat::Custom(format.into())).await;
    if result.metadata.output_format.as_deref() == Some("plain") {
        return Output::Plain(verified_plain_fallback(&result, expected_plain));
    }
    assert_eq!(result.metadata.output_format.as_deref(), Some(format));
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&result.content)
        .expect("actual binary output");
    assert!(!bytes.is_empty());
    Output::Binary(bytes)
}

async fn word_text(source: &[u8], expected_plain: &str) -> String {
    let bytes = match package(source, "docx", expected_plain).await {
        Output::Binary(bytes) => bytes,
        Output::Plain(text) => return text,
    };
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("actual DOCX package");
    assert!(archive.len() >= 6, "canonical package parts must exist");
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .expect("actual main document")
        .read_to_string(&mut xml)
        .unwrap();
    let document = roxmltree::Document::parse(&xml).expect("well-formed actual Word XML");
    let texts: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NS, "t")))
        .collect();
    assert!(!texts.is_empty(), "real selectable text runs must exist");
    let text: String = texts.iter().filter_map(|node| node.text()).collect();
    assert!(!text.is_empty());
    normalized(&text)
}

async fn pdf_text(source: &[u8], expected_plain: &str) -> String {
    let bytes = match package(source, "pdf", expected_plain).await {
        Output::Binary(bytes) => bytes,
        Output::Plain(text) => return text,
    };
    assert!(bytes.starts_with(b"%PDF-"));
    let document = lopdf::Document::load_mem(&bytes).expect("actual PDF output");
    let pages = document.get_pages();
    assert_eq!(pages.len(), 1, "controlled paragraph must occupy one actual page");
    let operations =
        lopdf::content::Content::decode(&document.get_page_content(pages[&1])).expect("drawing operations");
    assert!(!operations.operations.is_empty(), "actual PDF drawing must occur");
    let text = document.extract_text(&[1]).expect("actual selectable PDF text");
    assert!(!text.trim().is_empty());
    normalized(&text)
}

fn assert_private_output(actual: &str, expected: &str, context: &[&str]) {
    for word in context {
        assert!(actual.contains(word), "source context {word:?} must survive: {actual}");
    }
    assert_eq!(
        actual, expected,
        "binary text must redact every complete term occurrence"
    );
    assert!(
        !actual.contains("Jane Doe"),
        "the confidential term must not remain selectable"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn docx_redacts_bare_and_inline_split_occurrences_2361() {
    let expected = "[REDACTED] and [REDACTED] end";
    let text = word_text(MIXED, expected).await;
    assert_private_output(&text, expected, &["and", "end"]);
}

#[tokio::test]
#[serial_test::serial]
async fn pdf_redacts_bare_and_inline_split_occurrences_2361() {
    let expected = "[REDACTED] and [REDACTED] end";
    let text = pdf_text(MIXED, expected).await;
    assert_private_output(&text, expected, &["and", "end"]);
}

#[tokio::test]
#[serial_test::serial]
async fn docx_preserves_single_split_plain_fallback_2361() {
    let expected = "Call [REDACTED] today";
    let Output::Plain(text) = package(SPLIT_ONLY, "docx", expected).await else {
        panic!("the existing single-split case must retain its explicit plain fallback");
    };
    assert_private_output(&text, expected, &["Call", "today"]);
}

#[tokio::test]
#[serial_test::serial]
async fn pdf_preserves_single_split_plain_fallback_2361() {
    let expected = "Call [REDACTED] today";
    let Output::Plain(text) = package(SPLIT_ONLY, "pdf", expected).await else {
        panic!("the existing single-split case must retain its explicit plain fallback");
    };
    assert_private_output(&text, expected, &["Call", "today"]);
}
