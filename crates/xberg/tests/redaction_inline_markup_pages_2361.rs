#![cfg(all(
    feature = "html",
    feature = "office",
    feature = "redaction",
    feature = "tokio-runtime"
))]

use base64::Engine as _;
use std::io::{Cursor, Read};
use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::types::redaction::RedactionStrategy;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, extract};

const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

async fn convert_to_docx(source: &[u8], mime: &str) -> Vec<u8> {
    let config = ExtractionConfig {
        output_format: OutputFormat::Custom("docx".into()),
        use_cache: false,
        ..Default::default()
    };
    let response = extract(ExtractInput::from_bytes(source, mime, None), &config)
        .await
        .expect("actual public conversion");
    assert_eq!(response.results.len(), 1);
    let result = &response.results[0];
    assert_eq!(result.metadata.output_format.as_deref(), Some("docx"));
    base64::engine::general_purpose::STANDARD
        .decode(&result.content)
        .expect("actual package bytes")
}

fn word(bytes: &[u8]) -> (String, Vec<String>) {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("real DOCX source");
    assert!(archive.len() >= 6);
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let document = roxmltree::Document::parse(&xml).unwrap();
    assert!(document.root_element().has_tag_name((WORD_NS, "document")));
    let texts = document
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NS, "t")))
        .collect::<Vec<_>>();
    assert!(!texts.is_empty());
    let underlined = document
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NS, "r")))
        .filter(|run| run.descendants().any(|node| node.has_tag_name((WORD_NS, "u"))))
        .map(|run| {
            run.descendants()
                .filter(|node| node.has_tag_name((WORD_NS, "t")))
                .filter_map(|node| node.text())
                .collect::<String>()
        })
        .collect();
    (texts.iter().filter_map(|node| node.text()).collect(), underlined)
}

async fn pages(source: &[u8], redaction: Option<RedactionConfig>) -> ExtractedDocument {
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        include_document_structure: false,
        pages: Some(xberg::core::config::PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        redaction,
        ..Default::default()
    };
    let response = extract(ExtractInput::from_bytes(source, DOCX, None), &config)
        .await
        .expect("actual public DOCX page extraction");
    assert_eq!(response.results.len(), 1);
    response.results.into_iter().next().unwrap()
}

async fn page_docx(markdown: &str) -> Vec<u8> {
    let mut options = comrak::Options::default();
    options.render.r#unsafe = true;
    let html = comrak::markdown_to_html(markdown, &options);
    convert_to_docx(html.as_bytes(), "text/html").await
}

async fn verified_source() -> Vec<u8> {
    let source = convert_to_docx(b"<p>Call Jane <u>Doe</u> today</p>", "text/html").await;
    assert_eq!(word(&source), ("Call Jane Doe today".into(), vec!["Doe".into()]));
    let baseline = pages(&source, None).await;
    assert_eq!(baseline.metadata.output_format.as_deref(), Some("markdown"));
    assert_eq!(baseline.content.trim(), "Call Jane <u>Doe</u> today");
    let baseline_pages = baseline.pages.as_ref().expect("actual pages requested");
    assert_eq!(baseline_pages.len(), 1);
    assert_eq!(baseline_pages[0].page_number, 1);
    assert_eq!(baseline_pages[0].content.trim(), "Call Jane <u>Doe</u> today");
    assert!(baseline.redaction_report.is_none());
    let baseline_page_bytes = page_docx(&baseline_pages[0].content).await;
    assert_eq!(
        word(&baseline_page_bytes),
        ("Call Jane Doe today".into(), vec!["Doe".into()])
    );
    source
}

fn assert_selected_page_fallback(result: &ExtractedDocument) {
    assert_eq!(result.metadata.output_format.as_deref(), Some("plain"));
    assert_eq!(result.content, "Call [REDACTED] today");
    assert_eq!(result.pages.as_ref().unwrap()[0].content, r"Call \[REDACTED\] today");
    let report = result.redaction_report.as_ref().expect("actual selected findings");
    assert_eq!(report.total_redacted, 2);
    assert_eq!(
        report
            .findings
            .iter()
            .map(|finding| (finding.start, finding.end))
            .collect::<Vec<_>>(),
        [(5, 13), (5, 13)]
    );
    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.replacement_token == "[REDACTED]")
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "output_format" && warning.message.contains("rendering was discarded"))
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "output_format"
                && warning.message.contains("Page 1")
                && warning.message.contains("before escaping"))
    );
}

#[tokio::test]
#[serial_test::serial]
async fn public_docx_page_redacts_a_name_split_by_a_real_underline_run() {
    let source = verified_source().await;
    let result = pages(
        &source,
        Some(RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: vec![RedactionTerm::labeled("private", "Jane Doe")],
            ..Default::default()
        }),
    )
    .await;
    let redacted_pages = result.pages.as_ref().expect("redaction retains public pages");
    assert_eq!(redacted_pages.len(), 1);
    assert_eq!(redacted_pages[0].page_number, 1);
    let page_bytes = page_docx(&redacted_pages[0].content).await;
    assert_eq!(word(&page_bytes).0, "Call [REDACTED] today");
    assert_selected_page_fallback(&result);
}
