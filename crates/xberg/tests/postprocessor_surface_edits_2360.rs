#![cfg(all(
    feature = "office",
    feature = "pdf",
    feature = "redaction",
    feature = "tokio-runtime"
))]

use std::io::{Cursor, Read};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use base64::Engine as _;
use comrak::nodes::NodeValue;
use xberg::core::config::PageConfig;
use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::plugins::Plugin;
use xberg::types::redaction::RedactionStrategy;
use xberg::{
    ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, PostProcessor, ProcessingStage, Result, extract,
    register_post_processor, unregister_post_processor,
};

const SOURCE: &[u8] = b"<p>Original secret end.</p>";
const MIME_DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const NAME: &str = "review-surface-edits-2360";
const FORMAT_SUFFIX: &str = "\n\nformat edit secret ready";
const PAGE_SUFFIX: &str = "\n\npage edit secret ready";
const CONTENT_SUFFIX: &str = "\n\ncontent edit secret ready";

#[derive(Clone, Copy, Debug)]
enum Edit {
    FormattedOnly,
    PageOnly,
    PageAndContent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    content: String,
    formatted: Option<String>,
    pages: Vec<(u32, String)>,
}

impl Snapshot {
    fn of(document: &ExtractedDocument) -> Self {
        Self {
            content: document.content.clone(),
            formatted: document.formatted_content.clone(),
            pages: document
                .pages
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|page| (page.page_number, page.content.clone()))
                .collect(),
        }
    }
}

struct Mutation {
    edit: Edit,
    observations: Arc<Mutex<Vec<(Snapshot, Snapshot)>>>,
}

impl Plugin for Mutation {
    fn name(&self) -> &str {
        NAME
    }
}

#[async_trait]
impl PostProcessor for Mutation {
    async fn process(&self, document: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        let before = Snapshot::of(document);
        match self.edit {
            Edit::FormattedOnly => document
                .formatted_content
                .as_mut()
                .expect("actual generated pre-redaction rendering")
                .push_str(FORMAT_SUFFIX),
            Edit::PageOnly | Edit::PageAndContent => {
                let pages = document.pages.as_mut().expect("retained real DOCX page");
                assert_eq!(pages.len(), 1);
                assert_eq!(pages[0].page_number, 1);
                pages[0].content.push_str(PAGE_SUFFIX);
                if matches!(self.edit, Edit::PageAndContent) {
                    document.content.push_str(CONTENT_SUFFIX);
                }
            }
        }
        self.observations.lock().unwrap().push((before, Snapshot::of(document)));
        Ok(())
    }
    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }
}

struct Registration;
impl Drop for Registration {
    fn drop(&mut self) {
        unregister_post_processor(NAME).expect("remove only the owned surface mutation processor");
    }
}

fn config(output: OutputFormat, redact: bool) -> ExtractionConfig {
    ExtractionConfig {
        output_format: output,
        use_cache: false,
        enable_quality_processing: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        redaction: redact.then(|| RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: vec![RedactionTerm::labeled("private", "secret")],
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn docx_text(bytes: &[u8]) -> String {
    assert!(bytes.starts_with(b"PK"));
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("actual DOCX package");
    assert!(archive.len() >= 6);
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).expect("actual Word XML");
    let mut paragraphs = 0;
    let mut output = String::new();
    for node in tree.descendants().filter(|node| node.has_tag_name((WORD_NS, "p"))) {
        paragraphs += 1;
        for text in node.descendants().filter(|child| child.has_tag_name((WORD_NS, "t"))) {
            output.push_str(text.text().unwrap_or_default());
        }
        output.push('\n');
    }
    assert!(paragraphs > 0 && !output.trim().is_empty());
    normalized(&output)
}

fn pdf_text(bytes: &[u8]) -> String {
    assert!(bytes.starts_with(b"%PDF-"));
    let document = lopdf::Document::load_mem(bytes).expect("actual generated PDF");
    let pages = document.get_pages();
    assert_eq!(pages.len(), 1);
    let operations = lopdf::content::Content::decode(&document.get_page_content(pages[&1])).unwrap();
    assert!(!operations.operations.is_empty());
    normalized(&document.extract_text(&[1]).expect("actual text-bearing PDF"))
}

fn markdown_text(text: &str) -> String {
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, text, &comrak::Options::default());
    let mut output = String::new();
    for node in root.descendants() {
        match &node.data.borrow().value {
            NodeValue::Text(text) => output.push_str(text),
            NodeValue::Code(code) => output.push_str(&code.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak | NodeValue::Paragraph => output.push('\n'),
            _ => {}
        }
    }
    normalized(&output)
}

fn visible(document: &ExtractedDocument) -> String {
    match document.metadata.output_format.as_deref() {
        Some("docx") => docx_text(
            &base64::engine::general_purpose::STANDARD
                .decode(&document.content)
                .unwrap(),
        ),
        Some("pdf") => pdf_text(
            &base64::engine::general_purpose::STANDARD
                .decode(&document.content)
                .unwrap(),
        ),
        Some("markdown") => markdown_text(&document.content),
        Some("plain") => normalized(&document.content),
        other => panic!("unexpected real output format: {other:?}"),
    }
}

async fn source_docx() -> Vec<u8> {
    let response = extract(
        ExtractInput::from_bytes(SOURCE, "text/html", None),
        &config(OutputFormat::Custom("docx".to_string()), false),
    )
    .await
    .expect("real HTML-to-DOCX fixture producer");
    assert_eq!(response.results.len(), 1);
    let document = &response.results[0];
    assert_eq!(document.metadata.output_format.as_deref(), Some("docx"));
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&document.content)
        .unwrap();
    assert_eq!(docx_text(&bytes), "Original secret end.");
    bytes
}

fn assert_observation(edit: Edit, mime: &str, before: &Snapshot, after: &Snapshot) {
    assert_eq!(normalized(&before.content), "Original secret end.");
    assert_eq!(before.formatted.as_ref().unwrap().matches("secret").count(), 1);
    if mime == MIME_DOCX {
        assert_eq!(before.pages.len(), 1);
        assert_eq!(before.pages[0].0, 1);
        assert_eq!(before.pages[0].1.matches("secret").count(), 1);
    }
    match edit {
        Edit::FormattedOnly => {
            assert_eq!(after.content, before.content);
            assert_eq!(after.pages, before.pages);
            assert_eq!(
                after.formatted,
                Some(format!("{}{FORMAT_SUFFIX}", before.formatted.as_ref().unwrap()))
            );
        }
        Edit::PageOnly | Edit::PageAndContent => {
            assert_eq!(after.formatted, before.formatted);
            assert_eq!(after.pages, [(1, format!("{}{PAGE_SUFFIX}", before.pages[0].1))]);
            let expected = if matches!(edit, Edit::PageAndContent) {
                format!("{}{CONTENT_SUFFIX}", before.content)
            } else {
                before.content.clone()
            };
            assert_eq!(after.content, expected);
        }
    }
}

async fn attempt(
    source: &[u8],
    mime: &str,
    output: OutputFormat,
    edit: Edit,
    redact: bool,
) -> Result<ExtractedDocument> {
    let observations = Arc::new(Mutex::new(Vec::new()));
    register_post_processor(Arc::new(Mutation {
        edit,
        observations: Arc::clone(&observations),
    }))?;
    let registration = Registration;
    let response = extract(ExtractInput::from_bytes(source, mime, None), &config(output, redact)).await;
    drop(registration);
    let rows = observations.lock().unwrap();
    assert_eq!(rows.len(), 1, "one actual Middle-stage mutation must run");
    assert_observation(edit, mime, &rows[0].0, &rows[0].1);
    drop(rows);
    let response = response?;
    assert_eq!(response.results.len(), 1);
    Ok(response.results.into_iter().next().unwrap())
}

fn assert_edited_output(result: &ExtractedDocument, edit: Edit, redacted: bool) {
    let token = if redacted { "[REDACTED]" } else { "secret" };
    match edit {
        Edit::FormattedOnly => {
            assert!(visible(result).contains(&format!("format edit {token} ready")));
            if redacted {
                assert_eq!(result.metadata.output_format.as_deref(), Some("plain"));
                assert!(result.formatted_content.is_none());
            }
        }
        Edit::PageOnly | Edit::PageAndContent => {
            let pages = result.pages.as_ref().expect("edited page must remain present");
            assert_eq!(pages.len(), 1);
            assert_eq!(pages[0].page_number, 1);
            assert!(markdown_text(&pages[0].content).contains(&format!("page edit {token} ready")));
            if redacted {
                assert!(!pages[0].content.contains("secret"));
            }
            if matches!(edit, Edit::PageAndContent) {
                assert!(visible(result).contains(&format!("content edit {token} ready")));
            }
        }
    }
    assert!(visible(result).contains("Original"));
    if redacted {
        assert!(!result.content.contains("secret"));
        assert!(result.redaction_report.as_ref().unwrap().total_redacted >= 2);
        assert!(
            result
                .processing_warnings
                .iter()
                .any(|warning| warning.source == "output_format")
        );
    }
}

async fn route_case(edit: Edit, output: OutputFormat, docx: bool) {
    let source = if docx { source_docx().await } else { SOURCE.to_vec() };
    let mime = if docx { MIME_DOCX } else { "text/html" };
    let baseline = extract(
        ExtractInput::from_bytes(source.as_slice(), mime, None),
        &config(output.clone(), false),
    )
    .await
    .expect("unchanged public route baseline");
    assert_eq!(baseline.results.len(), 1);
    assert_eq!(visible(&baseline.results[0]), "Original secret end.");
    let edited = attempt(&source, mime, output.clone(), edit, false)
        .await
        .expect("same edits succeed without redaction");
    assert_edited_output(&edited, edit, false);
    let redacted = attempt(&source, mime, output, edit, true)
        .await
        .expect("legitimate current surface edits must take a safe fallback rather than abort extraction");
    assert_edited_output(&redacted, edit, true);
}

#[tokio::test]
#[serial_test::serial]
async fn formatted_only_docx_to_markdown_preserves_added_text_with_redaction() {
    route_case(Edit::FormattedOnly, OutputFormat::Markdown, true).await;
}

#[tokio::test]
#[serial_test::serial]
async fn formatted_only_html_to_docx_preserves_added_text_with_redaction() {
    route_case(Edit::FormattedOnly, OutputFormat::Custom("docx".to_string()), false).await;
}

#[tokio::test]
#[serial_test::serial]
async fn formatted_only_docx_to_pdf_preserves_added_text_with_redaction() {
    route_case(Edit::FormattedOnly, OutputFormat::Custom("pdf".to_string()), true).await;
}

#[tokio::test]
#[serial_test::serial]
async fn page_only_docx_to_markdown_preserves_added_text_with_redaction() {
    route_case(Edit::PageOnly, OutputFormat::Markdown, true).await;
}

#[tokio::test]
#[serial_test::serial]
async fn page_and_content_docx_to_markdown_preserves_both_edits_with_redaction() {
    route_case(Edit::PageAndContent, OutputFormat::Markdown, true).await;
}
