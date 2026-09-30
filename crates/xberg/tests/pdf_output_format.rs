//! PDF output (#1943): `OutputFormat::Custom("pdf")` returns a base64-encoded PDF file in
//! `content`.
//!
//! The PDF is laid out from the Markdown rendering after every post-processor has run,
//! so redaction rewrites the text before any of it reaches the document.

#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

use base64::Engine as _;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, extract};

const PDF_MIME_TYPE: &str = "application/pdf";

fn pdf() -> OutputFormat {
    OutputFormat::Custom("pdf".to_string())
}

async fn extract_one(bytes: &[u8], mime_type: &str, config: &ExtractionConfig) -> ExtractedDocument {
    extract(ExtractInput::from_bytes(bytes.to_vec(), mime_type, None), config)
        .await
        .expect("extraction should succeed")
        .results
        .into_iter()
        .next()
        .expect("one input yields one result")
}

fn decode_document(result: &ExtractedDocument) -> Vec<u8> {
    assert_eq!(result.metadata.output_format.as_deref(), Some("pdf"));
    let document = base64::engine::general_purpose::STANDARD
        .decode(&result.content)
        .expect("PDF content should be base64");
    assert!(document.starts_with(b"%PDF-"), "not a PDF file");
    document
}

fn config(output_format: OutputFormat) -> ExtractionConfig {
    ExtractionConfig {
        output_format,
        use_cache: false,
        ..Default::default()
    }
}

fn headings(markdown: &str) -> Vec<&str> {
    markdown.lines().filter(|line| line.starts_with('#')).collect()
}

/// The words of `text` in sorted order, without the list numbers the PDF draws as text.
fn sorted_words(text: &str) -> Vec<&str> {
    let is_list_number = |word: &str| {
        word.strip_suffix('.')
            .is_some_and(|number| number.parse::<u32>().is_ok())
    };
    let mut words: Vec<&str> = text.split_whitespace().filter(|word| !is_list_number(word)).collect();
    words.sort_unstable();
    words
}

const REPORT: &str = "# Quarterly report

Revenue grew **12%** in *Q3*, see [the dashboard](https://example.com/q3).

## Actions

1. Hire two engineers
2. Close the audit

## Regional totals

| Region | Revenue |
| --- | --- |
| North | 120 |
| South | 95 |
";

#[tokio::test]
#[serial_test::serial]
async fn pdf_output_reads_back_through_the_pdf_extractor() {
    let source = extract_one(REPORT.as_bytes(), "text/markdown", &config(OutputFormat::Plain)).await;
    let rendered = extract_one(REPORT.as_bytes(), "text/markdown", &config(pdf())).await;
    let document = decode_document(&rendered);

    // The words, not their order: the PDF extractor's column detection may read a sparse
    // page with a two-column table column by column. The headings below pin the order.
    let reread = extract_one(&document, PDF_MIME_TYPE, &config(OutputFormat::Plain)).await;
    assert_eq!(sorted_words(&reread.content), sorted_words(&source.content));

    let reread = extract_one(&document, PDF_MIME_TYPE, &config(OutputFormat::Markdown)).await;
    assert_eq!(
        headings(&reread.content),
        ["# Quarterly report", "## Actions", "## Regional totals"],
        "{}",
        reread.content
    );
}

/// The PDF extractor recovers headings only when the output format asks for structure.
#[tokio::test]
#[serial_test::serial]
async fn a_pdf_converted_to_pdf_keeps_the_headings_markdown_output_has() {
    let path = format!("{}/../../test_documents/pdf/338298584.pdf", env!("CARGO_MANIFEST_DIR"));
    let source = std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}; fetch the corpus first"));
    let markdown = extract_one(&source, PDF_MIME_TYPE, &config(OutputFormat::Markdown)).await;
    let rendered = extract_one(&source, PDF_MIME_TYPE, &config(pdf())).await;
    let reread = extract_one(
        &decode_document(&rendered),
        PDF_MIME_TYPE,
        &config(OutputFormat::Markdown),
    )
    .await;

    assert!(!headings(&markdown.content).is_empty(), "{}", markdown.content);
    assert_eq!(headings(&reread.content), headings(&markdown.content));
}

#[cfg(feature = "redaction")]
mod redaction {
    use super::*;
    use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
    use xberg::types::redaction::RedactionStrategy;

    fn redacting(output_format: OutputFormat, terms: &[&str]) -> ExtractionConfig {
        ExtractionConfig {
            redaction: Some(RedactionConfig {
                strategy: RedactionStrategy::Mask,
                custom_terms: terms
                    .iter()
                    .map(|term| RedactionTerm::labeled("person", *term))
                    .collect(),
                ..RedactionConfig::default()
            }),
            ..config(output_format)
        }
    }

    /// Every object of `document`, streams decompressed, as one byte string. Text in the
    /// content streams is glyph codes, so this covers the literal strings: the outline,
    /// link targets and document properties.
    fn decompressed_objects(document: &[u8]) -> Vec<u8> {
        let mut document = lopdf::Document::load_mem(document).expect("the PDF should parse");
        document.decompress();
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("the PDF should serialize");
        bytes
    }

    const PERSONNEL: &str = "# Review for Jane Doe

Contact Jane Doe through [her page](https://example.com/jdoe).

- Owner: Jane Doe

| Name | Role |
| --- | --- |
| Jane Doe | Lead |
";

    #[tokio::test]
    #[serial_test::serial]
    async fn redacted_terms_are_absent_from_the_text_and_every_object_of_the_document() {
        let result = extract_one(
            PERSONNEL.as_bytes(),
            "text/markdown",
            &redacting(pdf(), &["Jane Doe", "jdoe"]),
        )
        .await;
        let report = result.redaction_report.as_ref().expect("a redaction report");
        assert!(report.total_redacted >= 6, "{report:?}");
        let document = decode_document(&result);

        let reread = extract_one(&document, PDF_MIME_TYPE, &config(OutputFormat::Plain)).await;
        let text = reread.content.to_lowercase();
        assert!(!text.contains("jane doe"), "{}", reread.content);
        assert!(text.contains("contact"), "{}", reread.content);
        assert!(text.contains("lead"), "{}", reread.content);

        let objects = String::from_utf8_lossy(&decompressed_objects(&document)).to_lowercase();
        assert!(!objects.contains("jane doe"), "a redacted term survives in an object");
        assert!(!objects.contains("jdoe"), "a redacted term survives in an object");
        assert!(objects.contains("/uri"), "the link itself is kept");
    }

    /// An archive member runs its own pipeline and is laid out before the parent's
    /// redaction pass walks into it. `JVBERi0` is how every base64-encoded PDF begins, so a
    /// parent pass that rewrote the member's `content` would corrupt it.
    #[cfg(feature = "archives")]
    #[tokio::test]
    #[serial_test::serial]
    async fn archive_members_keep_intact_documents_when_the_parent_is_redacted() {
        use std::io::{Cursor, Write};

        let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file("member.md", zip::write::SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"# Member\n\nJane Doe signed off.\n").unwrap();
        let bytes = archive.finish().unwrap().into_inner();

        let result = extract_one(&bytes, "application/zip", &redacting(pdf(), &["Jane Doe", "JVBERi0"])).await;
        let children = result.children.as_ref().expect("the archive member is extracted");
        let member = children
            .iter()
            .find(|child| child.path == "member.md")
            .expect("member.md is a child");

        let reread = extract_one(
            &decode_document(&member.result),
            PDF_MIME_TYPE,
            &config(OutputFormat::Plain),
        )
        .await;
        assert!(reread.content.contains("signed off."), "{}", reread.content);
        assert!(
            !reread.content.to_lowercase().contains("jane doe"),
            "{}",
            reread.content
        );
    }
}

/// A post-processor that rewrites `content` without the rendering makes the pipeline
/// discard the rendering (#331). PDF must then report the plain-text fallback like any
/// other format, not lay out text that no processor saw.
mod fallback {
    use super::*;
    use async_trait::async_trait;
    use xberg::plugins::Plugin;
    use xberg::{PostProcessor, ProcessingStage, Result, register_post_processor, unregister_post_processor};

    const PROCESSOR_NAME: &str = "pdf-fallback-rewriter";

    struct ContentOnlyRewriter;

    impl Plugin for ContentOnlyRewriter {
        fn name(&self) -> &str {
            PROCESSOR_NAME
        }
    }

    #[async_trait]
    impl PostProcessor for ContentOnlyRewriter {
        async fn process(&self, result: &mut ExtractedDocument, _config: &ExtractionConfig) -> Result<()> {
            result.content = result.content.replace("SECRET-42", "[gone]");
            Ok(())
        }

        fn processing_stage(&self) -> ProcessingStage {
            ProcessingStage::Late
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn a_discarded_rendering_falls_back_to_plain_text_instead_of_a_document() {
        register_post_processor(std::sync::Arc::new(ContentOnlyRewriter)).expect("register the processor");
        let result = extract_one(b"Invoice SECRET-42 is overdue.", "text/plain", &config(pdf())).await;
        unregister_post_processor(PROCESSOR_NAME).expect("unregister the processor");

        assert_eq!(result.metadata.output_format.as_deref(), Some("plain"));
        assert!(
            result.content.contains("Invoice [gone] is overdue."),
            "{}",
            result.content
        );
        assert!(
            result
                .processing_warnings
                .iter()
                .any(|warning| warning.source == "output_format"),
            "{:?}",
            result.processing_warnings
        );
    }
}
