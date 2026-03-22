//! Regression tests: EPUB headings should be preserved for Markdown/Djot output.
//!
//! The native EPUB extractor historically returned plain text only, flattening
//! `<h1>`–`<h6>` into regular lines. When `ExtractionConfig.output_format` is set
//! to Markdown (or Djot), we should run the XHTML through the HTML→Markdown
//! converter so headings become `#` / `##` etc.

#![cfg(feature = "office")]

use kreuzberg::core::config::{ExtractionConfig, OutputFormat};
use kreuzberg::extractors::EpubExtractor;
use kreuzberg::plugins::DocumentExtractor;
use std::io::{Cursor, Write};
use zip::write::FileOptions;

fn build_minimal_epub_bytes() -> Vec<u8> {
    let container_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;

    let opf_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="3.0" unique-identifier="bookid" xmlns="http://www.idpf.org/2007/opf">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Test Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="c1" href="chapter1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="c1"/>
  </spine>
</package>"#;

    let chapter_xhtml = r#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><title>Chapter One</title></head>
  <body>
    <h1>Chapter One</h1>
    <p>Some text.</p>
  </body>
</html>"#;

    let mut cursor = Cursor::new(Vec::<u8>::new());
    let mut writer = zip::ZipWriter::new(&mut cursor);
    let options = FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

    writer.start_file("mimetype", options).expect("zip start_file failed");
    writer
        .write_all(b"application/epub+zip")
        .expect("zip write mimetype failed");

    writer
        .add_directory("META-INF/", options)
        .expect("zip add_directory failed");
    writer
        .add_directory("OEBPS/", options)
        .expect("zip add_directory failed");

    writer
        .start_file("META-INF/container.xml", options)
        .expect("zip start_file failed");
    writer
        .write_all(container_xml.as_bytes())
        .expect("zip write container.xml failed");

    writer
        .start_file("OEBPS/content.opf", options)
        .expect("zip start_file failed");
    writer
        .write_all(opf_xml.as_bytes())
        .expect("zip write content.opf failed");

    writer
        .start_file("OEBPS/chapter1.xhtml", options)
        .expect("zip start_file failed");
    writer
        .write_all(chapter_xhtml.as_bytes())
        .expect("zip write chapter1.xhtml failed");

    writer.finish().expect("zip finish failed");
    cursor.into_inner()
}

#[tokio::test]
async fn test_epub_markdown_output_keeps_headings() {
    let bytes = build_minimal_epub_bytes();
    let extractor = EpubExtractor::new();

    let mut config = ExtractionConfig::default();
    config.output_format = OutputFormat::Markdown;

    let result = extractor
        .extract_bytes(&bytes, "application/epub+zip", &config)
        .await
        .expect("EPUB extraction should succeed");

    assert!(
        result.processing_warnings.is_empty(),
        "Expected no warnings, got: {:?}",
        result.processing_warnings
    );
    assert!(
        result.content.contains("# Chapter One"),
        "Expected Markdown heading, got:\n{}",
        result.content
    );
    assert!(
        !result.content.starts_with("---"),
        "Expected no YAML frontmatter injection, got:\n{}",
        result.content
    );
    assert!(result.content.contains("Some text."), "Expected body text");
}

#[tokio::test]
async fn test_epub_djot_output_keeps_headings() {
    let bytes = build_minimal_epub_bytes();
    let extractor = EpubExtractor::new();

    let mut config = ExtractionConfig::default();
    config.output_format = OutputFormat::Djot;

    let result = extractor
        .extract_bytes(&bytes, "application/epub+zip", &config)
        .await
        .expect("EPUB extraction should succeed");

    assert!(
        result.processing_warnings.is_empty(),
        "Expected no warnings, got: {:?}",
        result.processing_warnings
    );
    assert!(
        result.content.contains("# Chapter One"),
        "Expected Djot heading, got:\n{}",
        result.content
    );
    assert!(
        !result.content.starts_with("---"),
        "Expected no YAML frontmatter injection, got:\n{}",
        result.content
    );
    assert!(result.content.contains("Some text."), "Expected body text");
}

#[tokio::test]
async fn test_epub_plain_output_does_not_inject_markdown_headings() {
    let bytes = build_minimal_epub_bytes();
    let extractor = EpubExtractor::new();

    let config = ExtractionConfig::default();

    let result = extractor
        .extract_bytes(&bytes, "application/epub+zip", &config)
        .await
        .expect("EPUB extraction should succeed");

    assert!(
        result.processing_warnings.is_empty(),
        "Expected no warnings, got: {:?}",
        result.processing_warnings
    );
    assert!(
        !result.content.contains("# Chapter One"),
        "Plain output should not contain Markdown heading markers, got:\n{}",
        result.content
    );
    assert!(result.content.contains("Chapter One"), "Expected heading text");
}
