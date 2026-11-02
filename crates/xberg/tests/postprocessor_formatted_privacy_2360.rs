//! Processor-only formatted edits must not return escaped or split sensitive terms as redacted data.
#![cfg(all(feature = "office", feature = "redaction", feature = "tokio-runtime"))]

use std::io::{Cursor, Read, Write};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use comrak::nodes::NodeValue;
use xberg::core::config::PageConfig;
use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::plugins::Plugin;
use xberg::types::redaction::RedactionStrategy;
use xberg::{
    ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, PostProcessor, ProcessingStage, Result, extract,
    register_post_processor, unregister_post_processor,
};

const NAME: &str = "formatted-privacy-review-2360";
const MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
const BASELINE: &str = "Public baseline.";
const BODY: &str = r#"<w:document
 xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>
<w:p>
<w:r>
<w:t>Public baseline.</w:t>
</w:r>
</w:p>
<w:sectPr/>
</w:body>
</w:document>"#;

fn source_docx() -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, text) in [
        (
            "[Content_Types].xml",
            r#"<Types
 xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default
 Extension="rels"
 ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default
 Extension="xml"
 ContentType="application/xml"/>
<Override
 PartName="/word/document.xml"
 ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
<Override
 PartName="/word/styles.xml"
 ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
<Override
 PartName="/docProps/core.xml"
 ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<Relationships
 xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship
 Id="rId1"
 Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument"
 Target="word/document.xml"/>
</Relationships>"#,
        ),
        ("word/document.xml", BODY),
        (
            "word/_rels/document.xml.rels",
            r#"<Relationships
 xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship
 Id="rIdStyles"
 Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles"
 Target="styles.xml"/>
</Relationships>"#,
        ),
        (
            "word/styles.xml",
            r#"<w:styles
 xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:style
 w:type="paragraph"
 w:default="1"
 w:styleId="Normal">
<w:name
 w:val="Normal"/>
</w:style>
</w:styles>"#,
        ),
        (
            "docProps/core.xml",
            r#"<cp:coreProperties
 xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"/>"#,
        ),
    ] {
        archive
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(text.as_bytes()).unwrap();
    }
    let bytes = archive.finish().unwrap().into_inner();
    assert!(bytes.starts_with(b"PK"));
    let mut opened = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    assert_eq!(opened.len(), 6);
    let mut actual = String::new();
    opened
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut actual)
        .unwrap();
    assert_eq!(actual, BODY);
    let xml = roxmltree::Document::parse(&actual).unwrap();
    let text: Vec<_> = xml
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NS, "t")))
        .map(|node| node.text().unwrap())
        .collect();
    assert_eq!(text, [BASELINE]);
    bytes
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Snapshot {
    plain: String,
    formatted: Option<String>,
    pages: Vec<(u32, String)>,
    redacted: Option<u32>,
}

impl Snapshot {
    fn of(document: &ExtractedDocument) -> Self {
        Self {
            plain: document.content.clone(),
            formatted: document.formatted_content.clone(),
            pages: document
                .pages
                .iter()
                .flatten()
                .map(|page| (page.page_number, page.content.clone()))
                .collect(),
            redacted: document.redaction_report.as_ref().map(|report| report.total_redacted),
        }
    }
}

type Calls = Arc<Mutex<Vec<(Snapshot, Snapshot)>>>;

struct ReplaceFormatted {
    text: &'static str,
    calls: Calls,
}

impl Plugin for ReplaceFormatted {
    fn name(&self) -> &str {
        NAME
    }
}

#[async_trait]
impl PostProcessor for ReplaceFormatted {
    async fn process(&self, document: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        let before = Snapshot::of(document);
        assert!(
            document.formatted_content.is_some(),
            "actual renderer produced the edited field"
        );
        document.formatted_content = Some(self.text.to_owned());
        self.calls.lock().unwrap().push((before, Snapshot::of(document)));
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }
}

struct Registration;
impl Drop for Registration {
    fn drop(&mut self) {
        unregister_post_processor(NAME).expect("remove only this test's processor");
    }
}

fn config(terms: &[&str]) -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        enable_quality_processing: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        redaction: Some(RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: terms
                .iter()
                .map(|term| RedactionTerm::labeled("private", *term))
                .collect(),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn visible(text: &str) -> String {
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, text, &comrak::Options::default());
    let mut value = String::new();
    for node in root.descendants() {
        match &node.data.borrow().value {
            NodeValue::Text(text) => value.push_str(text),
            NodeValue::Code(code) => value.push_str(&code.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak => value.push(' '),
            _ => {}
        }
    }
    value
}

async fn edited(text: &'static str, terms: &[&str]) -> ExtractedDocument {
    let bytes = source_docx();
    let calls: Calls = Arc::default();
    register_post_processor(Arc::new(ReplaceFormatted {
        text,
        calls: Arc::clone(&calls),
    }))
    .unwrap();
    let _registration = Registration;
    let response = extract(ExtractInput::from_bytes(bytes, MIME, None), &config(terms))
        .await
        .expect("supported formatted-only edit through the real DOCX-to-Markdown pipeline");
    let observed = calls.lock().unwrap();
    assert_eq!(observed.len(), 1, "exactly one real Middle callback");
    let (before, after) = &observed[0];
    assert_eq!(before.plain, BASELINE);
    assert_eq!(before.formatted.as_deref(), Some("Public baseline.\n"));
    assert_eq!(before.pages, [(1, "Public baseline.\n".into())]);
    assert_eq!(before.redacted, None, "callback ran before the real builtin");
    assert_eq!(after.plain, before.plain);
    assert_eq!(after.pages, before.pages);
    assert_eq!(after.redacted, None);
    assert_eq!(after.formatted.as_deref(), Some(text));
    for term in terms {
        assert!(
            !before.plain.contains(term),
            "term must be introduced ONLY by the callback"
        );
    }
    assert_eq!(response.results.len(), 1);
    let document = response.results.into_iter().next().unwrap();
    assert!(
        document.redaction_report.is_some(),
        "the real builtin created its report"
    );
    assert_eq!(document.formatted_content, None);
    assert_eq!(document.metadata.output_format.as_deref(), Some("plain"));
    assert!(
        document
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "output_format")
    );
    document
}

fn assert_literal_result(document: &ExtractedDocument, expected: &str, findings: u32) {
    assert_eq!(
        document.content, expected,
        "preserve introduced safe text and mask the actual formatted term"
    );
    assert_eq!(document.formatted_content, None);
    assert_eq!(document.metadata.output_format.as_deref(), Some("plain"));
    assert_eq!(document.redaction_report.as_ref().unwrap().total_redacted, findings);
    assert!(
        document
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "output_format")
    );
    let serialized = serde_json::to_value(document).unwrap();
    assert_eq!(serialized["content"].as_str(), Some(expected));
    assert!(serialized["formatted_content"].is_null());
}

async fn hidden_term_case(text: &'static str, term: &str) {
    assert!(
        !text.contains(term),
        "raw-only matching must not accidentally cover this case"
    );
    assert_eq!(visible(text), format!("Visit {term} now safe-branch"));
    let document = edited(text, &[term]).await;
    assert_literal_result(&document, "Visit [REDACTED] now safe-branch", 1);
    assert!(!visible(&document.content).contains(term));
}

#[tokio::test]
#[serial_test::serial]
async fn formatted_only_escaped_ampersand_is_private_without_a_plain_match() {
    hidden_term_case(r"Visit AT\&T now safe-branch", "AT&T").await;
}

#[tokio::test]
#[serial_test::serial]
async fn formatted_only_escaped_less_than_is_private_without_a_plain_match() {
    hidden_term_case(r"Visit a\<b now safe-branch", "a<b").await;
}

#[tokio::test]
#[serial_test::serial]
async fn formatted_only_underlined_split_is_private_without_a_plain_match() {
    let text = "Visit Jane<u>Doe</u> now safe-branch";
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, text, &comrak::Options::default());
    let tags: Vec<_> = root
        .descendants()
        .filter_map(|node| match &node.data.borrow().value {
            NodeValue::HtmlInline(tag) => Some(tag.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(tags, ["<u>", "</u>"]);
    hidden_term_case(text, "JaneDoe").await;
}

#[tokio::test]
#[serial_test::serial]
async fn formatted_only_code_keeps_literal_stars_while_a_real_finding_is_redacted() {
    let text = "Visit `Jane**Doe**` now force safe-branch";
    let arena = comrak::Arena::new();
    let root = comrak::parse_document(&arena, text, &comrak::Options::default());
    let code: Vec<_> = root
        .descendants()
        .filter_map(|node| match &node.data.borrow().value {
            NodeValue::Code(code) => Some(code.literal.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(code, ["Jane**Doe**"]);
    assert!(!visible(text).contains("JaneDoe"));
    let document = edited(text, &["JaneDoe", "force"]).await;
    assert_literal_result(&document, "Visit `Jane**Doe**` now [REDACTED] safe-branch", 1);
}

#[tokio::test]
#[serial_test::serial]
async fn formatted_only_escaped_stars_stay_literal_while_a_real_finding_is_redacted() {
    let text = r"Visit Jane\*\*Doe\*\* now force safe-branch";
    assert_eq!(visible(text), "Visit Jane**Doe** now force safe-branch");
    let document = edited(text, &["JaneDoe", "force"]).await;
    assert_literal_result(&document, r"Visit Jane\*\*Doe\*\* now [REDACTED] safe-branch", 1);
}

#[tokio::test]
#[serial_test::serial]
async fn formatted_only_unmatched_edit_preserves_all_introduced_safe_bytes() {
    let text = "Visit public **message** now safe-branch";
    assert_eq!(visible(text), "Visit public message now safe-branch");
    let document = edited(text, &["not-in-document"]).await;
    assert_literal_result(&document, text, 0);
}
