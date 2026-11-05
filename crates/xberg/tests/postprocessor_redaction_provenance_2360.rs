#![cfg(all(feature = "office", feature = "redaction", feature = "tokio-runtime"))]

use std::io::{Cursor, Read};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use base64::Engine as _;
use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::plugins::Plugin;
use xberg::types::redaction::RedactionStrategy;
use xberg::{
    ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, PostProcessor, ProcessingStage, Result, extract,
    register_post_processor, unregister_post_processor,
};

const SOURCE: &[u8] = b"<p>Original secret and <u>under</u> end</p>";
const SUFFIX: &str = "\n\nprocessor secret done";
const NAME: &str = "middle-provenance-2360";
const WORD_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

#[derive(Clone, Debug)]
struct Observation {
    before: String,
    formatted_before: Option<String>,
    after: String,
    formatted_after: Option<String>,
}

struct AppendProcessor {
    formatted: bool,
    observations: Arc<Mutex<Vec<Observation>>>,
}

impl Plugin for AppendProcessor {
    fn name(&self) -> &str {
        NAME
    }
}

#[async_trait]
impl PostProcessor for AppendProcessor {
    async fn process(&self, document: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
        let before = document.content.clone();
        let formatted_before = document.formatted_content.clone();
        document.content.push_str(SUFFIX);
        if self.formatted
            && let Some(formatted) = &mut document.formatted_content
        {
            formatted.push_str(SUFFIX);
        }
        self.observations
            .lock()
            .expect("owned observation lock")
            .push(Observation {
                before,
                formatted_before,
                after: document.content.clone(),
                formatted_after: document.formatted_content.clone(),
            });
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        ProcessingStage::Middle
    }
}

struct Registration;
impl Drop for Registration {
    fn drop(&mut self) {
        unregister_post_processor(NAME).expect("remove only the owned Middle processor after extraction");
    }
}

fn config(redact: bool) -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Custom("docx".into()),
        use_cache: false,
        enable_quality_processing: false,
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

fn docx_text(document: &ExtractedDocument) -> String {
    assert_eq!(document.metadata.output_format.as_deref(), Some("docx"));
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&document.content)
        .expect("actual binary package encoding");
    assert!(bytes.starts_with(b"PK"));
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("actual DOCX archive");
    assert!(archive.len() >= 6);
    let mut xml = String::new();
    archive
        .by_name("word/document.xml")
        .unwrap()
        .read_to_string(&mut xml)
        .unwrap();
    let tree = roxmltree::Document::parse(&xml).expect("actual well-formed Word document");
    let paragraphs: Vec<_> = tree
        .descendants()
        .filter(|node| node.has_tag_name((WORD_NS, "p")))
        .map(|node| {
            node.descendants()
                .filter(|child| child.has_tag_name((WORD_NS, "t")))
                .filter_map(|child| child.text())
                .collect::<String>()
        })
        .collect();
    assert!(!paragraphs.is_empty());
    normalized(&paragraphs.join("\n"))
}

async fn unchanged(redact: bool) -> ExtractedDocument {
    let response = extract(ExtractInput::from_bytes(SOURCE, "text/html", None), &config(redact))
        .await
        .expect("actual unchanged public conversion");
    assert_eq!(response.results.len(), 1);
    response.results.into_iter().next().unwrap()
}

async fn mutated(formatted: bool, redact: bool) -> Result<ExtractedDocument> {
    let observations = Arc::new(Mutex::new(Vec::new()));
    register_post_processor(Arc::new(AppendProcessor {
        formatted,
        observations: Arc::clone(&observations),
    }))?;
    let registration = Registration;
    let response = extract(ExtractInput::from_bytes(SOURCE, "text/html", None), &config(redact)).await;
    drop(registration);
    let rows = observations.lock().expect("owned observation lock");
    assert_eq!(
        rows.len(),
        1,
        "actual public pipeline must invoke exactly one Middle processor"
    );
    let row = &rows[0];
    assert_eq!(normalized(&row.before), "Original secret and under end");
    assert_eq!(row.after, format!("{}{SUFFIX}", row.before));
    let before = row
        .formatted_before
        .as_deref()
        .expect("actual pre-redaction Markdown rendering");
    assert!(before.contains("secret") && before.contains("<u>under</u>"));
    if formatted {
        assert_eq!(
            row.formatted_after.as_deref(),
            Some(format!("{before}{SUFFIX}").as_str())
        );
    } else {
        assert_eq!(
            row.formatted_after, row.formatted_before,
            "content-only mutation leaves the rendering exact"
        );
    }
    drop(rows);
    let response = response?;
    assert_eq!(response.results.len(), 1);
    Ok(response.results.into_iter().next().unwrap())
}

async fn mutation_case(formatted: bool) {
    let original = unchanged(false).await;
    assert_eq!(docx_text(&original), "Original secret and under end");
    let baseline = mutated(formatted, false)
        .await
        .expect("the same processor works without redaction");
    let baseline_text = if formatted {
        docx_text(&baseline)
    } else {
        assert_eq!(baseline.metadata.output_format.as_deref(), Some("plain"));
        normalized(&baseline.content)
    };
    assert_eq!(baseline_text, "Original secret and under end processor secret done");
    let result = mutated(formatted, true)
        .await
        .expect("current Middle-stage text must remain processable with redaction");
    assert_eq!(result.metadata.output_format.as_deref(), Some("plain"));
    assert_eq!(
        normalized(&result.content),
        "Original [REDACTED] and under end processor [REDACTED] done"
    );
    assert!(!result.content.contains("secret"));
    assert!(result.formatted_content.is_none());
    assert_eq!(
        result
            .redaction_report
            .as_ref()
            .expect("real redaction report")
            .total_redacted,
        2
    );
    assert!(
        result
            .processing_warnings
            .iter()
            .any(|warning| warning.source == "output_format")
    );
}

#[tokio::test]
#[serial_test::serial]
async fn should_keep_unchanged_public_docx_redaction_processable() {
    assert_eq!(docx_text(&unchanged(false).await), "Original secret and under end");
    let result = unchanged(true).await;
    assert_eq!(docx_text(&result), "Original [REDACTED] and under end");
    assert!(!result.content.is_empty());
}

#[tokio::test]
#[serial_test::serial]
async fn should_preserve_middle_content_and_rendering_changes_with_redaction() {
    mutation_case(true).await;
}

#[tokio::test]
#[serial_test::serial]
async fn should_preserve_middle_content_only_changes_without_using_stale_rendering() {
    mutation_case(false).await;
}
