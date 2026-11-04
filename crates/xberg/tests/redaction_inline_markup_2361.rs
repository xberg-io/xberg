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

struct CacheObserver(std::sync::Arc<std::sync::atomic::AtomicUsize>);

impl xberg::plugins::Plugin for CacheObserver {
    fn name(&self) -> &str {
        "redaction-2361-cache-observer"
    }
}

#[async_trait::async_trait]
impl xberg::PostProcessor for CacheObserver {
    async fn process(&self, _result: &mut ExtractedDocument, config: &ExtractionConfig) -> xberg::Result<()> {
        if config.use_cache {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        Ok(())
    }

    fn processing_stage(&self) -> xberg::ProcessingStage {
        xberg::ProcessingStage::Middle
    }
}

struct CacheRegistration;

impl Drop for CacheRegistration {
    fn drop(&mut self) {
        let _ = xberg::unregister_post_processor("redaction-2361-cache-observer");
    }
}

async fn cached_file(path: &std::path::Path, config: &ExtractionConfig) -> ExtractedDocument {
    let response = extract(ExtractInput::from_uri(path.to_string_lossy().into_owned()), config)
        .await
        .expect("actual cached HTML file extraction");
    assert_eq!(response.results.len(), 1);
    response.results.into_iter().next().unwrap()
}

#[tokio::test]
#[serial_test::serial]
async fn cached_file_keeps_redaction_coverage_and_invalidates_changed_terms_2361() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("private.html");
    std::fs::write(&path, MIXED).unwrap();
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    xberg::register_post_processor(std::sync::Arc::new(CacheObserver(std::sync::Arc::clone(&calls)))).unwrap();
    let _registration = CacheRegistration;
    let mut config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: true,
        enable_quality_processing: false,
        cache_namespace: Some(format!(
            "redaction2361_{}",
            directory
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim_start_matches('.')
        )),
        redaction: Some(RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: vec![RedactionTerm::labeled("person", "Jane Doe")],
            ..Default::default()
        }),
        ..Default::default()
    };
    for expected in [1, 1] {
        let result = cached_file(&path, &config).await;
        assert_eq!(normalized(&result.content), "[REDACTED] and [REDACTED] end");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), expected);
    }
    config
        .redaction
        .as_mut()
        .unwrap()
        .custom_terms
        .push(RedactionTerm::labeled("private", "end"));
    for expected in [2, 2] {
        let result = cached_file(&path, &config).await;
        assert_eq!(normalized(&result.content), "[REDACTED] and [REDACTED] [REDACTED]");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), expected);
    }
}

async fn parsed_text(result: &ExtractedDocument) -> String {
    match result.metadata.output_format.as_deref().expect("actual output format") {
        "plain" => normalized(&result.content),
        "json" => {
            let value: serde_json::Value = serde_json::from_str(&result.content).expect("actual JSON renderer");
            let body = value["body"].as_array().unwrap();
            assert_eq!(body.len(), 1);
            assert_eq!(body[0]["type"], "paragraph");
            normalized(body[0]["text"].as_str().unwrap())
        }
        "doctags" => {
            let document = roxmltree::Document::parse(&result.content).expect("actual DocTags tree");
            assert_eq!(document.root_element().tag_name().name(), "doctag");
            let text = document
                .descendants()
                .filter(|node| node.is_text())
                .filter_map(|node| node.text())
                .collect::<String>();
            normalized(&text)
        }
        format => {
            let mime = match format {
                "markdown" => "text/markdown",
                "djot" => "text/djot",
                "html" => "text/html",
                other => panic!("unexpected output format {other}"),
            };
            let config = ExtractionConfig {
                output_format: OutputFormat::Plain,
                use_cache: false,
                ..Default::default()
            };
            let response = extract(ExtractInput::from_bytes(result.content.as_bytes(), mime, None), &config)
                .await
                .unwrap();
            assert_eq!(response.results.len(), 1);
            normalized(&response.results[0].content)
        }
    }
}

#[tokio::test]
async fn every_text_output_removes_both_tokens_of_repeated_split_terms_2361() {
    let formats = [
        OutputFormat::Plain,
        OutputFormat::Markdown,
        OutputFormat::Html,
        OutputFormat::Djot,
        OutputFormat::Json,
        OutputFormat::DocTags,
    ];
    let mut actual = Vec::new();
    for format in formats {
        let result = redacted(MIXED, format.clone()).await;
        assert!(result.redaction_report.is_some(), "{format}: redaction must execute");
        actual.push((format.to_string(), parsed_text(&result).await));
    }
    let expected = ["plain", "markdown", "html", "djot", "json", "doctags"]
        .map(|format| (format.to_string(), "[REDACTED] and [REDACTED] end".to_string()));
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn html_without_redaction_preserves_existing_rendered_bytes_2361() {
    let config = ExtractionConfig {
        output_format: OutputFormat::Html,
        use_cache: false,
        ..Default::default()
    };
    let response = extract(ExtractInput::from_bytes(MIXED, "text/html", None), &config)
        .await
        .unwrap();
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].content, "<p>Jane Doe and Jane <u>Doe</u> end</p>\n");
    assert!(response.results[0].redaction_report.is_none());
}

#[tokio::test]
async fn zero_match_html_preserves_visible_text_and_zero_token_accounting_2361() {
    let mut config = ExtractionConfig {
        output_format: OutputFormat::Html,
        use_cache: false,
        ..Default::default()
    };
    config.redaction = Some(RedactionConfig {
        strategy: RedactionStrategy::TokenReplace,
        custom_terms: vec![RedactionTerm::labeled("person", "Jane Doe")],
        ..Default::default()
    });
    let response = extract(
        ExtractInput::from_bytes(b"<p>safe <u>body</u></p>".as_slice(), "text/html", None),
        &config,
    )
    .await
    .unwrap();
    assert_eq!(response.results.len(), 1);
    let result = &response.results[0];
    assert_eq!(result.metadata.output_format.as_deref(), Some("html"));
    assert_eq!(parsed_text(result).await, "safe body");
    let report = result.redaction_report.as_ref().unwrap();
    assert_eq!(report.total_redacted, 0);
    assert_eq!(report.findings.len(), 0);
    assert_eq!(result.content.matches("[PERSON_").count(), 0);
    if result.content.starts_with("<pre>") {
        assert!(
            result
                .processing_warnings
                .iter()
                .any(|warning| warning.source == "output_format" && warning.message.contains("formatting"))
        );
    }
}

#[tokio::test]
async fn styled_html_override_redacts_every_occurrence_through_real_private_producer_2361() {
    let mut config = ExtractionConfig {
        output_format: OutputFormat::Html,
        use_cache: false,
        html_output: Some(xberg::HtmlOutputConfig {
            css: Some("p { color: red; }".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let before = extract(ExtractInput::from_bytes(MIXED, "text/html", None), &config)
        .await
        .unwrap();
    assert_eq!(before.results.len(), 1);
    assert!(before.results[0].content.contains("kb-doc"));
    assert!(before.results[0].content.contains("p { color: red; }"));
    config.redaction = Some(RedactionConfig {
        custom_terms: vec![RedactionTerm::labeled("person", "Jane Doe")],
        ..Default::default()
    });
    let response = extract(ExtractInput::from_bytes(MIXED, "text/html", None), &config)
        .await
        .unwrap();
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].metadata.output_format.as_deref(), Some("html"));
    assert_eq!(parsed_text(&response.results[0]).await, "[REDACTED] and [REDACTED] end");
    assert_eq!(response.results[0].redaction_report.as_ref().unwrap().total_redacted, 2);
}

#[tokio::test]
async fn builtin_custom_html_keeps_requested_syntax_and_complete_redaction_2361() {
    let result = redacted(MIXED, OutputFormat::Custom("html".into())).await;
    assert_eq!(result.metadata.output_format.as_deref(), Some("html"));
    assert_eq!(parsed_text(&result).await, "[REDACTED] and [REDACTED] end");
    assert_eq!(result.redaction_report.as_ref().unwrap().total_redacted, 2);
}

#[tokio::test]
#[serial_test::serial]
async fn html_cache_hits_keep_verified_redaction_and_changed_term_invalidation_2361() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("private.html");
    std::fs::write(&path, MIXED).unwrap();
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    xberg::register_post_processor(std::sync::Arc::new(CacheObserver(std::sync::Arc::clone(&calls)))).unwrap();
    let _registration = CacheRegistration;
    let mut config = ExtractionConfig {
        output_format: OutputFormat::Html,
        use_cache: true,
        enable_quality_processing: false,
        cache_namespace: Some(format!(
            "html2361_{}",
            directory
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .trim_start_matches('.')
        )),
        redaction: Some(RedactionConfig {
            custom_terms: vec![RedactionTerm::labeled("person", "Jane Doe")],
            ..Default::default()
        }),
        ..Default::default()
    };
    for expected in [1, 1] {
        let result = cached_file(&path, &config).await;
        assert_eq!(result.metadata.output_format.as_deref(), Some("html"));
        assert_eq!(parsed_text(&result).await, "[REDACTED] and [REDACTED] end");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), expected);
    }
    config
        .redaction
        .as_mut()
        .unwrap()
        .custom_terms
        .push(RedactionTerm::labeled("person", "end"));
    for expected in [2, 2] {
        let result = cached_file(&path, &config).await;
        assert_eq!(parsed_text(&result).await, "[REDACTED] and [REDACTED] [REDACTED]");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), expected);
    }
}
