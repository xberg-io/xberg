//! Regression tests for xberg-io/xberg#3414: `&`, `<` and `>` stored in `word/document.xml` as
//! named or numeric references must be decoded, not dropped, in paragraphs and table cells,
//! for plain and Markdown output alike.

#![cfg(feature = "office")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use std::io::{Cursor, Write};
use xberg::{ExtractionConfig, OutputFormat};
use zip::CompressionMethod;
use zip::write::{FileOptions, ZipWriter};

const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

fn build_docx() -> Vec<u8> {
    let para = |text: &str| format!(r#"<w:p><w:r><w:t xml:space="preserve">{text}</w:t></w:r></w:p>"#);
    let body = [
        para("PARA-AMP: Tom &amp; Jerry"),
        para("PARA-LT: 3 &lt; 5 and 7 &gt; 2"),
        para("PARA-ENT: &#38; &#60; &#x3E;"),
        para("PARA-HTML: &lt;b&gt;bold&lt;/b&gt;"),
        format!(
            "<w:tbl><w:tblGrid><w:gridCol/></w:tblGrid><w:tr><w:tc>{}</w:tc></w:tr></w:tbl>",
            para("CELL: R&amp;D &lt;x&gt;")
        ),
    ]
    .concat();
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}</w:body></w:document>"#
    );

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        let options: FileOptions<()> = FileOptions::default().compression_method(CompressionMethod::Stored);
        let parts: [(&str, &str); 3] = [
            (
                "[Content_Types].xml",
                r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#,
            ),
            (
                "_rels/.rels",
                r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#,
            ),
            ("word/document.xml", &document),
        ];
        for (name, data) in parts {
            zip.start_file(name, options).expect("zip entry");
            zip.write_all(data.as_bytes()).expect("zip write");
        }
        zip.finish().expect("zip finish");
    }
    cursor.into_inner()
}

fn extract(format: OutputFormat) -> xberg::ExtractedDocument {
    let config = ExtractionConfig {
        output_format: format,
        use_cache: false,
        ..Default::default()
    };
    extract_bytes_document_blocking(&build_docx(), DOCX_MIME, &config).expect("DOCX extraction should succeed")
}

/// Markdown may backslash-escape punctuation; the characters must still be present.
fn unescaped(text: &str) -> String {
    text.replace('\\', "")
}

fn assert_text_kept(content: &str, label: &str) {
    for expected in [
        "PARA-AMP: Tom & Jerry",
        "PARA-LT: 3 < 5 and 7 > 2",
        "PARA-ENT: & < >",
        "PARA-HTML: <b>bold</b>",
        "CELL: R&D <x>",
    ] {
        assert!(
            content.contains(expected),
            "{label}: missing {expected:?} in {content:?}"
        );
    }
}

#[test]
fn plain_output_decodes_xml_entities_in_paragraphs_and_cells() {
    let result = extract(OutputFormat::Plain);
    assert_text_kept(&result.content, "plain");
}

#[test]
fn markdown_output_decodes_xml_entities_in_paragraphs_and_cells() {
    let result = extract(OutputFormat::Markdown);
    assert_text_kept(&unescaped(&result.content), "markdown");
}

#[test]
fn extracted_table_keeps_decoded_entities() {
    let result = extract(OutputFormat::Markdown);
    let table = result.tables.first().expect("a table must be extracted");
    let cells: Vec<&str> = table.cells.iter().flatten().map(String::as_str).collect();
    assert!(
        cells.iter().any(|cell| cell.contains("R&D <x>")),
        "table cell lost its entities: {cells:?}"
    );
    assert!(
        unescaped(&table.markdown).contains("R&D <x>"),
        "table markdown lost its entities: {:?}",
        table.markdown
    );
}
