#![cfg(all(feature = "office", feature = "redaction", feature = "tokio-runtime"))]

use std::io::{Cursor, Write};
use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::types::redaction::RedactionStrategy;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, extract};

const DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const TARGET: &str = "https://example.test/shared";
const SPLIT: &str = r#"
  <w:hyperlink r:id="rId1">
    <w:r><w:t xml:space="preserve">Jane </w:t></w:r>
    <w:r><w:rPr><w:u w:val="single"/></w:rPr><w:t>Doe</w:t></w:r>
  </w:hyperlink>
"#;
const ADJACENT: &str = r#"
  <w:hyperlink r:id="rId1"><w:r><w:t xml:space="preserve">Safe first </w:t></w:r></w:hyperlink>
  <w:hyperlink r:id="rId1"><w:r><w:t>Safe second</w:t></w:r></w:hyperlink>
"#;

fn docx(paragraph: &str) -> Vec<u8> {
    let document = format!(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
             <w:body><w:p>{paragraph}</w:p></w:body>
           </w:document>"#
    );
    let relationships = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
             <Relationship Id="rId1" TargetMode="External" Target="{TARGET}"
               Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink"/>
           </Relationships>"#
    );
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in [
        (
            "[Content_Types].xml",
            include_str!("redaction_structured_fields_2363/content-types.xml"),
        ),
        (
            "_rels/.rels",
            include_str!("redaction_structured_fields_2363/package-relationships.xml"),
        ),
        ("word/document.xml", document.as_str()),
        ("word/_rels/document.xml.rels", relationships.as_str()),
    ] {
        zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

async fn extract_docx(bytes: &[u8], format: OutputFormat, redact: bool) -> ExtractedDocument {
    let config = ExtractionConfig {
        output_format: format,
        use_cache: false,
        redaction: redact.then(|| RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: vec![RedactionTerm::labeled("person", "Jane Doe")],
            ..Default::default()
        }),
        ..Default::default()
    };
    let response = extract(ExtractInput::from_bytes(bytes, DOCX, None), &config)
        .await
        .unwrap();
    assert_eq!(response.results.len(), 1);
    response.results.into_iter().next().unwrap()
}

fn labels(result: &ExtractedDocument) -> Vec<&str> {
    result
        .uris
        .as_ref()
        .expect("actual extracted hyperlinks")
        .iter()
        .map(|uri| {
            assert_eq!(uri.url, TARGET);
            uri.label.as_deref().expect("actual hyperlink label")
        })
        .collect()
}

#[tokio::test]
async fn should_redact_one_docx_hyperlink_label_split_across_styled_runs() {
    let bytes = docx(SPLIT);
    let baseline = extract_docx(&bytes, OutputFormat::Plain, false).await;
    assert_eq!(baseline.content, "Jane Doe");
    assert!(baseline.redaction_report.is_none());

    let result = extract_docx(&bytes, OutputFormat::Markdown, true).await;
    assert!(result.content.contains("[REDACTED]"));
    assert!(!result.content.contains("Jane"));
    assert!(!result.content.contains("Doe"));
    assert_eq!(labels(&result), ["[REDACTED]"]);
}

#[tokio::test]
async fn should_keep_the_first_label_for_adjacent_distinct_docx_links_with_the_same_target() {
    let bytes = docx(ADJACENT);
    let baseline = extract_docx(&bytes, OutputFormat::Plain, false).await;
    assert_eq!(baseline.content, "Safe first Safe second");
    assert_eq!(labels(&baseline), ["Safe first "]);

    let result = extract_docx(&bytes, OutputFormat::Markdown, true).await;
    assert_eq!(labels(&result), ["Safe first "]);
    assert_eq!(result.redaction_report.unwrap().total_redacted, 0);
}

#[tokio::test]
async fn should_emit_only_the_complete_first_docx_link_label_without_redaction() {
    let paragraph = format!("{SPLIT}{ADJACENT}");
    let bytes = docx(&paragraph);
    let result = extract_docx(&bytes, OutputFormat::Plain, false).await;
    assert_eq!(result.content, "Jane DoeSafe first Safe second");
    assert!(result.redaction_report.is_none());
    assert_eq!(labels(&result), ["Jane Doe"]);
}

#[tokio::test]
async fn should_not_join_unproved_nested_docx_hyperlink_labels() {
    let bytes = docx(
        r#"
        <w:hyperlink r:id="rId1">
          <w:r><w:t xml:space="preserve">Outer </w:t></w:r>
          <w:hyperlink r:id="rId1"><w:r><w:t>Inner</w:t></w:r></w:hyperlink>
        </w:hyperlink>
    "#,
    );
    let result = extract_docx(&bytes, OutputFormat::Plain, false).await;
    assert_eq!(result.content, "Outer Inner");
    assert_eq!(labels(&result), ["Outer "]);
}

#[tokio::test]
async fn should_keep_unproved_docx_field_link_label_unchanged() {
    let paragraph = format!(
        r#"<w:fldSimple w:instr="HYPERLINK &quot;{TARGET}&quot;">
            <w:r><w:t xml:space="preserve">Field </w:t></w:r>
            <w:r><w:t>value</w:t></w:r>
          </w:fldSimple>"#
    );
    let result = extract_docx(&docx(&paragraph), OutputFormat::Plain, false).await;
    assert_eq!(result.content, "Field value");
    assert_eq!(labels(&result), ["Field "]);
}
