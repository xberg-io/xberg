#![cfg(all(feature = "office", feature = "pdf", feature = "tokio-runtime"))]

use base64::Engine as _;
use lopdf::content::Content;
use std::io::{Cursor, Read};
use xberg::{ExtractInput, ExtractionConfig, OutputFormat, extract};

const SEMANTIC: &[u8] = b"<p>x<sup>2</sup> and <u>under</u> end</p>";
const LITERAL: &[u8] = b"<p><code>&lt;sup&gt;2&lt;/sup&gt; &lt;u&gt;under&lt;/u&gt;</code> end</p>";
const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

async fn converted(source: &[u8], format: &str) -> Vec<u8> {
    let config = ExtractionConfig {
        output_format: OutputFormat::Custom(format.into()),
        use_cache: false,
        ..Default::default()
    };
    let response = extract(ExtractInput::from_bytes(source, "text/html", None), &config)
        .await
        .expect("public HTML conversion must succeed");
    assert_eq!(response.results.len(), 1, "one real input must produce one output");
    let result = &response.results[0];
    assert_eq!(result.metadata.output_format.as_deref(), Some(format));
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&result.content)
        .expect("binary output must decode");
    assert!(!bytes.is_empty(), "actual writer must produce a package");
    bytes
}

async fn document_xml(source: &[u8]) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(converted(source, "docx").await)).expect("real DOCX archive");
    assert!(zip.len() >= 6, "canonical DOCX package parts must exist");
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .expect("actual document part")
        .read_to_string(&mut xml)
        .expect("UTF-8 document XML");
    xml
}

fn text_of(node: roxmltree::Node<'_, '_>) -> String {
    node.descendants()
        .filter(|node| node.has_tag_name((WORD_NS, "t")))
        .filter_map(|node| node.text())
        .collect()
}

fn property(node: roxmltree::Node<'_, '_>, name: &str, value: &str) -> bool {
    node.descendants()
        .any(|node| node.has_tag_name((WORD_NS, name)) && node.attribute((WORD_NS, "val")) == Some(value))
}

async fn pdf(source: &[u8]) -> (String, Content) {
    let bytes = converted(source, "pdf").await;
    assert!(bytes.starts_with(b"%PDF-"));
    let document = lopdf::Document::load_mem(&bytes).expect("actual PDF must parse");
    let pages = document.get_pages();
    assert_eq!(pages.len(), 1, "controlled single paragraph must produce one page");
    let text = document.extract_text(&[1]).expect("embedded font must expose text");
    let content = Content::decode(&document.get_page_content(pages[&1]).expect("actual page content"))
        .expect("PDF drawing operations");
    assert!(!content.operations.is_empty(), "writer must draw actual content");
    (text, content)
}

fn compact(text: &str) -> String {
    text.chars().filter(|character| !character.is_whitespace()).collect()
}

#[tokio::test]
#[serial_test::serial]
async fn should_preserve_superscript_and_underline_in_public_docx_conversion() {
    let xml = document_xml(SEMANTIC).await;
    let document = roxmltree::Document::parse(&xml).expect("well-formed actual document XML");
    assert_eq!(text_of(document.root()), "x2 and under end");
    let runs: Vec<_> = document
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NS, "r")))
        .collect();
    assert!(!runs.is_empty(), "real text runs required");
    let raised: Vec<_> = runs
        .iter()
        .filter(|node| property(**node, "vertAlign", "superscript"))
        .map(|node| text_of(*node))
        .collect();
    let underlined: Vec<_> = runs
        .iter()
        .filter(|node| property(**node, "u", "single"))
        .map(|node| text_of(*node))
        .collect();
    assert_eq!(raised, ["2"], "source exponent must remain a semantic Word run");
    assert_eq!(
        underlined,
        ["under"],
        "source underline must remain a semantic Word run"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn should_preserve_superscript_and_underline_in_public_pdf_conversion() {
    let (text, content) = pdf(SEMANTIC).await;
    assert_eq!(
        compact(&text),
        "x2andunderend",
        "PDF text must contain source text, not formatting tags"
    );
    assert!(
        content.operations.iter().any(|operation| {
            operation.operator == "Ts"
                && operation
                    .operands
                    .first()
                    .and_then(|value| value.as_float().ok())
                    .is_some_and(|rise| rise > 0.0)
        }),
        "the exponent must have actual positive text rise"
    );
    assert_eq!(
        content
            .operations
            .iter()
            .filter(|operation| operation.operator == "l")
            .count(),
        1,
        "the single underline must have an actual stroke"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn should_keep_tag_looking_code_literal_in_public_docx_conversion() {
    let xml = document_xml(LITERAL).await;
    let document = roxmltree::Document::parse(&xml).expect("well-formed actual document XML");
    assert_eq!(text_of(document.root()), "<sup>2</sup> <u>under</u> end");
    assert!(!property(document.root(), "vertAlign", "superscript"));
    assert!(!property(document.root(), "u", "single"));
}

#[tokio::test]
#[serial_test::serial]
async fn should_keep_tag_looking_code_literal_in_public_pdf_conversion() {
    let (text, content) = pdf(LITERAL).await;
    assert_eq!(compact(&text), "<sup>2</sup><u>under</u>end");
    for operation in content.operations.iter().filter(|operation| operation.operator == "Ts") {
        assert_eq!(operation.operands.len(), 1, "text rise must have one numeric operand");
        assert_eq!(
            operation.operands[0].as_float().expect("text rise must be numeric"),
            0.0,
            "literal tags must not introduce superscript or subscript rise"
        );
    }
    assert_eq!(
        content
            .operations
            .iter()
            .filter(|operation| operation.operator == "l")
            .count(),
        0
    );
}
