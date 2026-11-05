#![cfg(all(
    feature = "html",
    feature = "office",
    feature = "redaction",
    feature = "tokio-runtime"
))]

use base64::Engine as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg::core::config::PageConfig;
use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::plugins::{Plugin, PostProcessor, ProcessingStage};
use xberg::types::redaction::RedactionStrategy;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, extract};

const STRONG: &[u8] = b"<p>Jane Doe and Jane <strong>Doe</strong> end</p>";
const SUPERSCRIPT: &[u8] = b"<p>Jane Doe and Jane <sup>Doe</sup> end</p>";
const DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const REDACTED: &str = "[REDACTED] and [REDACTED] end";

async fn convert(
    bytes: &[u8],
    mime: &str,
    output_format: OutputFormat,
    redact: bool,
    pages: bool,
) -> ExtractedDocument {
    let config = config(output_format, redact, pages);
    let response = extract(ExtractInput::from_bytes(bytes, mime, None), &config)
        .await
        .expect("public extraction of controlled fixture");
    assert_eq!(response.results.len(), 1);
    response.results.into_iter().next().unwrap()
}

fn config(output_format: OutputFormat, redact: bool, pages: bool) -> ExtractionConfig {
    ExtractionConfig {
        output_format,
        use_cache: false,
        pages: pages.then(|| PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        redaction: redact.then(|| RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: vec![RedactionTerm::labeled("person", "Jane Doe")],
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn visible(content: &str, format: Option<&str>) -> String {
    match format {
        Some("djot") => jotdown::Parser::new(content)
            .filter_map(|event| match event {
                jotdown::Event::Str(text) => Some(text),
                _ => None,
            })
            .collect(),
        Some("plain") => content.into(),
        format => panic!("unexpected output format {format:?}"),
    }
}

fn assert_verified_fallback(result: &ExtractedDocument, count: usize) {
    assert_eq!(result.metadata.output_format.as_deref(), Some("plain"));
    assert!(result.formatted_content.is_none());
    assert!(result.processing_warnings.iter().any(|warning| {
        warning.source == "output_format"
            && warning.message == "Djot rendering was discarded. Verified redacted plain text is returned. Finding offsets refer to the original plain text"
    }));
    let report = result.redaction_report.as_ref().unwrap();
    assert_eq!(report.total_redacted as usize, count);
    let ranges: Vec<_> = report
        .findings
        .iter()
        .map(|finding| (finding.start, finding.end))
        .collect();
    assert_eq!(ranges, [(0, 8), (13, 21)].repeat(count / 2));
}

async fn assert_root_redacted(source: &[u8], original: &str) {
    let baseline = convert(source, "text/html", OutputFormat::Djot, false, false).await;
    assert_eq!(baseline.metadata.output_format.as_deref(), Some("djot"));
    assert_eq!(baseline.content, original);
    assert!(baseline.redaction_report.is_none());

    let redacted = convert(source, "text/html", OutputFormat::Djot, true, false).await;
    assert_eq!(
        visible(&redacted.content, redacted.metadata.output_format.as_deref()),
        REDACTED
    );
    assert_verified_fallback(&redacted, 2);
}

#[tokio::test]
#[serial_test::serial]
async fn should_redact_djot_terms_split_by_strong_markup() {
    assert_root_redacted(STRONG, "Jane Doe and Jane *Doe* end\n").await;
}

#[tokio::test]
#[serial_test::serial]
async fn should_redact_djot_terms_split_by_superscript_markup() {
    assert_root_redacted(SUPERSCRIPT, "Jane Doe and Jane ^Doe^ end\n").await;
}

#[tokio::test]
#[serial_test::serial]
async fn should_redact_djot_terms_in_a_rendered_page() {
    let package = convert(STRONG, "text/html", OutputFormat::Custom("docx".into()), false, false).await;
    assert_eq!(package.metadata.output_format.as_deref(), Some("docx"));
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(package.content)
        .expect("actual DOCX bytes");

    let baseline = convert(&bytes, DOCX, OutputFormat::Djot, false, true).await;
    assert_eq!(baseline.metadata.output_format.as_deref(), Some("djot"));
    let pages = baseline.pages.expect("actual DOCX pages");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    assert_eq!(pages[0].content.trim_end(), "Jane Doe and Jane *Doe* end");

    let redacted = convert(&bytes, DOCX, OutputFormat::Djot, true, true).await;
    let pages = redacted.pages.as_ref().expect("returned DOCX pages");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    let format = redacted.metadata.output_format.as_deref();
    assert_eq!(visible(&pages[0].content, format), REDACTED);
    assert_eq!(visible(&redacted.content, format), REDACTED);
    assert_verified_fallback(&redacted, 4);
}

#[tokio::test]
#[serial_test::serial]
async fn should_reject_unverified_djot_without_mutating_it() {
    let mut result = ExtractedDocument::default();
    result.content = "Jane *Doe* end".into();
    result.metadata.output_format = Some("djot".into());
    let before = serde_json::to_value(&result).unwrap();
    let config = config(OutputFormat::Djot, true, false);
    let error = xberg::text::redaction::redact(&mut result, config.redaction.as_ref().unwrap())
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Djot redaction requires"));
    assert_eq!(serde_json::to_value(&result).unwrap(), before);
}

#[tokio::test]
#[serial_test::serial]
async fn should_redact_verified_djot_links_and_lists() {
    let source = b"<ul><li>Jane <a href='https://example.test'>Doe</a> end</li><li>Another item</li></ul>";
    let baseline = convert(source, "text/html", OutputFormat::Djot, false, false).await;
    assert_eq!(baseline.metadata.output_format.as_deref(), Some("djot"));
    assert!(baseline.content.contains("[Doe](https://example.test)"));
    let result = convert(source, "text/html", OutputFormat::Djot, true, false).await;
    assert_eq!(result.metadata.output_format.as_deref(), Some("plain"));
    assert_eq!(result.content, "[REDACTED] end\nAnother item");
    let report = result.redaction_report.as_ref().unwrap();
    assert_eq!(report.total_redacted, 1);
    assert_eq!((report.findings[0].start, report.findings[0].end), (0, 8));
}

#[tokio::test]
#[serial_test::serial]
async fn should_keep_verified_djot_link_list_text_without_findings() {
    let source = b"<ul><li>Keep <a href='https://example.test'>this</a></li><li>Another item</li></ul>";
    let result = convert(source, "text/html", OutputFormat::Djot, true, false).await;
    assert_eq!(result.content, "Keep this\nAnother item");
    assert_verified_fallback(&result, 0);
}

#[tokio::test]
#[serial_test::serial]
async fn should_reject_djot_with_an_opaque_raw_block() {
    let source = b"```=html\nJane <b>Doe</b> end\n```\n";
    let baseline = convert(source, "text/djot", OutputFormat::Djot, false, false).await;
    assert_eq!(baseline.metadata.output_format.as_deref(), Some("djot"));
    assert!(baseline.content.contains("Jane <b>Doe</b> end"));
    let error = extract(
        ExtractInput::from_bytes(source, "text/djot", None),
        &config(OutputFormat::Djot, true, false),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("Djot redaction requires"));
}

struct DjotCallback {
    stage: ProcessingStage,
    mutate: bool,
    calls: Arc<AtomicUsize>,
}

impl Plugin for DjotCallback {
    fn name(&self) -> &str {
        "djot-redaction-mutation"
    }
}

#[async_trait::async_trait]
impl PostProcessor for DjotCallback {
    async fn process(&self, document: &mut ExtractedDocument, _: &ExtractionConfig) -> xberg::Result<()> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if self.mutate {
            document.formatted_content = Some("Jane *Doe* reinserted\n".into());
        }
        Ok(())
    }

    fn processing_stage(&self) -> ProcessingStage {
        self.stage
    }

    fn priority(&self) -> i32 {
        1
    }
}

struct RegisteredDjotMutation;

impl Drop for RegisteredDjotMutation {
    fn drop(&mut self) {
        xberg::unregister_post_processor("djot-redaction-mutation").unwrap();
    }
}

async fn assert_callback_rejected(stage: ProcessingStage) {
    let calls = Arc::new(AtomicUsize::new(0));
    xberg::register_post_processor(Arc::new(DjotCallback {
        stage,
        mutate: true,
        calls: Arc::clone(&calls),
    }))
    .unwrap();
    let _registration = RegisteredDjotMutation;
    let error = extract(
        ExtractInput::from_bytes(STRONG, "text/html", None),
        &config(OutputFormat::Djot, true, false),
    )
    .await
    .unwrap_err();
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert!(error.to_string().contains("Djot"), "{error}");
}

#[tokio::test]
#[serial_test::serial]
async fn should_reject_djot_replaced_before_redaction() {
    assert_callback_rejected(ProcessingStage::Middle).await;
}

#[tokio::test]
#[serial_test::serial]
async fn should_reject_djot_reinserted_after_redaction() {
    assert_callback_rejected(ProcessingStage::Late).await;
}

#[tokio::test]
#[serial_test::serial]
async fn should_keep_verified_djot_fallback_after_a_noop_callback() {
    let calls = Arc::new(AtomicUsize::new(0));
    xberg::register_post_processor(Arc::new(DjotCallback {
        stage: ProcessingStage::Late,
        mutate: false,
        calls: Arc::clone(&calls),
    }))
    .unwrap();
    let _registration = RegisteredDjotMutation;
    let result = convert(STRONG, "text/html", OutputFormat::Djot, true, false).await;
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(result.content, REDACTED);
    assert_verified_fallback(&result, 2);
}

#[tokio::test]
#[serial_test::serial]
async fn should_keep_original_djot_when_redaction_processor_is_disabled() {
    use xberg::core::config::PostProcessorConfig;

    let source = b"<p>A <a href='https://example.test' title='Link title'>link</a>.</p><ol start='4'><li>First</li><li>Second</li></ol>";
    let baseline = convert(source, "text/html", OutputFormat::Djot, false, false).await;
    for postprocessor in [
        PostProcessorConfig {
            enabled: false,
            ..Default::default()
        },
        PostProcessorConfig {
            disabled_processors: Some(vec!["redaction".into()]),
            ..Default::default()
        },
        PostProcessorConfig {
            enabled_processors: Some(vec!["some-other-processor".into()]),
            ..Default::default()
        },
    ] {
        let mut config = config(OutputFormat::Djot, true, false);
        config.postprocessor = Some(postprocessor);
        let result = extract(ExtractInput::from_bytes(source, "text/html", None), &config)
            .await
            .unwrap();
        assert_eq!(result.results.len(), 1);
        let result = &result.results[0];
        assert_eq!(result.metadata.output_format.as_deref(), Some("djot"));
        assert_eq!(result.content, baseline.content);
        assert!(result.redaction_report.is_none());
    }
}

#[tokio::test]
#[serial_test::serial]
async fn should_keep_original_djot_when_builtin_redaction_is_absent() {
    let source = b"<ol start='4'><li>First</li><li>Second</li></ol>";
    let baseline = convert(source, "text/html", OutputFormat::Djot, false, false).await;
    assert_eq!(baseline.content, "4. First\n5. Second\n");
    xberg::unregister_post_processor("redaction").unwrap();
    let response = extract(
        ExtractInput::from_bytes(source, "text/html", None),
        &config(OutputFormat::Djot, true, false),
    )
    .await;
    xberg::clear_post_processors().unwrap();
    let response = response.unwrap();
    assert_eq!(response.results.len(), 1);
    assert_eq!(response.results[0].content, baseline.content);
    assert_eq!(response.results[0].metadata.output_format.as_deref(), Some("djot"));
    assert!(response.results[0].redaction_report.is_none());
}

#[tokio::test]
#[serial_test::serial]
async fn should_reject_an_opaque_djot_prerender_from_a_public_document() {
    use xberg::core::pipeline::run_pipeline;
    use xberg::types::internal_builder::InternalDocumentBuilder;

    let mut builder = InternalDocumentBuilder::new("html");
    builder.push_paragraph("different safe paragraph", vec![], None, None);
    let mut doc = builder.build();
    doc.metadata.output_format = Some("djot".into());
    doc.pre_rendered_content = Some("Jane *Doe* end\n".into());

    let baseline = run_pipeline(doc.clone(), &config(OutputFormat::Djot, false, false))
        .await
        .unwrap();
    assert_eq!(baseline.content, "Jane *Doe* end\n");
    assert_eq!(baseline.metadata.output_format.as_deref(), Some("djot"));
    let error = run_pipeline(doc, &config(OutputFormat::Djot, true, false))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Djot redaction requires"));
}

#[tokio::test]
#[serial_test::serial]
async fn should_redact_verified_djot_from_a_local_html_file() {
    let file = tempfile::Builder::new().suffix(".html").tempfile().unwrap();
    std::fs::write(file.path(), STRONG).unwrap();
    let result = extract(
        ExtractInput::from_uri(file.path().to_str().unwrap()),
        &config(OutputFormat::Djot, true, false),
    )
    .await
    .unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].content, REDACTED);
    assert_verified_fallback(&result.results[0], 2);
}
