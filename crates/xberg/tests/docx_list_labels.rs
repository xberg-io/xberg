//! A numbered Word list kept only its order: the number Word paints, `Q4.`, `(b)` or `3.2`, is not
//! in the paragraph's text but computed from `word/numbering.xml`, so the output counted the items
//! from 1 and a question or clause could no longer be found by its own number.

#![allow(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)] // ~keep: test binaries print by design; org logging policy exempts tests
#![cfg(feature = "office")]

mod helpers;
use helpers::extract_bytes_document_blocking;

use std::io::Write;
use xberg::ExtractionConfig;
use xberg::types::NodeContent;
use zip::write::{SimpleFileOptions, ZipWriter};

const WORD_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// List 1 numbers exam questions (`Q%1.`, then `(%2)`). Lists 2 and 3 share a clause definition
/// that starts at 3 and whose third level never restarts; list 3 restarts it at 10. List 4 is a
/// plain `1.`, `2.` list, and list 5 a bullet list with a numbered second level. List 6 numbers
/// questions from 3 without writing a `w:numFmt`. Lists 7 and 8 share a plain definition, and
/// only list 8 overrides its level, with its own `%1)` and a restart at 4.
const NUMBERING_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="Q%1."/></w:lvl>
    <w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="lowerLetter"/><w:lvlText w:val="(%2)"/></w:lvl>
  </w:abstractNum>
  <w:abstractNum w:abstractNumId="1">
    <w:lvl w:ilvl="0"><w:start w:val="3"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/></w:lvl>
    <w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1.%2"/></w:lvl>
    <w:lvl w:ilvl="2"><w:start w:val="1"/><w:numFmt w:val="lowerRoman"/><w:lvlRestart w:val="0"/><w:lvlText w:val="%3)"/></w:lvl>
  </w:abstractNum>
  <w:abstractNum w:abstractNumId="2">
    <w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/></w:lvl>
  </w:abstractNum>
  <w:abstractNum w:abstractNumId="3">
    <w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="-"/></w:lvl>
    <w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%2."/></w:lvl>
  </w:abstractNum>
  <w:abstractNum w:abstractNumId="4">
    <w:lvl w:ilvl="0"><w:start w:val="3"/><w:lvlText w:val="Q%1."/></w:lvl>
  </w:abstractNum>
  <w:abstractNum w:abstractNumId="5">
    <w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/></w:lvl>
  </w:abstractNum>
  <w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>
  <w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num>
  <w:num w:numId="3"><w:abstractNumId w:val="1"/><w:lvlOverride w:ilvl="0"><w:startOverride w:val="10"/></w:lvlOverride></w:num>
  <w:num w:numId="4"><w:abstractNumId w:val="2"/></w:num>
  <w:num w:numId="5"><w:abstractNumId w:val="3"/></w:num>
  <w:num w:numId="6"><w:abstractNumId w:val="4"/></w:num>
  <w:num w:numId="7"><w:abstractNumId w:val="5"/></w:num>
  <w:num w:numId="8"><w:abstractNumId w:val="5"/><w:lvlOverride w:ilvl="0"><w:startOverride w:val="1"/><w:lvl w:ilvl="0"><w:start w:val="4"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1)"/></w:lvl></w:lvlOverride></w:num>
</w:numbering>"#;

/// Build a minimal `.docx` with `body_xml` and [`NUMBERING_XML`].
fn build_docx(body_xml: &str) -> Vec<u8> {
    let document_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="{W_NS}"><w:body>{body_xml}</w:body></w:document>"#
    );

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(std::io::Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
    <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
    <Default Extension="xml" ContentType="application/xml"/>
    <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
    <Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/>
</Types>"#,
        )
        .unwrap();

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(document_xml.as_bytes()).unwrap();
        zip.start_file("word/numbering.xml", options).unwrap();
        zip.write_all(NUMBERING_XML.as_bytes()).unwrap();

        let _ = zip.finish().unwrap();
    }
    buffer
}

fn list_item(num_id: u32, level: u32, text: &str) -> String {
    format!(
        r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="{num_id}"/></w:numPr></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#
    )
}

fn extract(body: &str) -> xberg::types::ExtractedDocument {
    let config = ExtractionConfig {
        use_cache: false,
        include_document_structure: true,
        output_format: xberg::core::config::OutputFormat::Markdown,
        ..Default::default()
    };
    extract_bytes_document_blocking(&build_docx(body), WORD_MIME_TYPE, &config).expect("extraction must succeed")
}

/// The `list_marker` of every list item in the document tree, in order.
fn list_markers(doc: &xberg::types::ExtractedDocument) -> Vec<Option<String>> {
    let structure = doc.document.as_ref().expect("document structure was requested");
    structure
        .nodes
        .iter()
        .filter(|node| matches!(node.content, NodeContent::ListItem { .. }))
        .map(|node| node.attributes.as_ref().and_then(|a| a.get("list_marker").cloned()))
        .collect()
}

#[test]
fn a_numbered_word_list_keeps_the_numbers_word_paints() {
    let body = [
        list_item(1, 0, "Define speed."),
        list_item(1, 1, "Give its unit."),
        list_item(1, 1, "Give an example."),
        list_item(1, 0, "Define velocity."),
        list_item(1, 1, "Give its unit."),
        list_item(2, 0, "Scope."),
        list_item(2, 1, "Who it covers."),
        list_item(2, 2, "Staff."),
        list_item(2, 1, "Where it applies."),
        list_item(2, 2, "Offices."),
        list_item(2, 0, "Terms."),
        list_item(2, 1, "Definitions."),
        list_item(3, 0, "Restarted."),
    ]
    .concat();

    let doc = extract(&body);

    let expected = [
        "Q1.", "(a)", "(b)", "Q2.", "(a)", "3.", "3.1", "i)", "3.2", "ii)", "4.", "4.1", "10.",
    ];
    assert_eq!(
        list_markers(&doc),
        expected.map(|label| Some(label.to_string())).to_vec(),
        "content:\n{}",
        doc.content
    );
    // Decimal labels become ordered starts; other Word labels remain visible bullet text. ~keep
    for line in [
        "- Q1. Define speed.",
        "  - (b) Give an example.",
        "    - ii) Offices.",
        "10. Restarted.",
    ] {
        assert!(doc.content.contains(line), "missing {line:?} in:\n{}", doc.content);
    }
}

/// A plain list reads the same with or without its labels, so it is written as it always was.
/// Once a paragraph splits it, Word carries on at 3 where the output's own count would restart.
#[test]
fn a_plain_numbered_list_is_unchanged_and_one_carried_on_keeps_its_number() {
    let body = format!(
        "{}{}<w:p><w:r><w:t>Between.</w:t></w:r></w:p>{}",
        list_item(4, 0, "First."),
        list_item(4, 0, "Second."),
        list_item(4, 0, "Third."),
    );

    let doc = extract(&body);

    assert_eq!(
        list_markers(&doc),
        vec![None, None, Some("3.".to_string())],
        "content:\n{}",
        doc.content
    );
    assert!(
        doc.content.contains("1. First.\n2. Second.\n") && doc.content.contains("3. Third."),
        "content:\n{}",
        doc.content
    );
}

/// A numbered level under a bullet keeps its ordered kind and needs no redundant source label.
#[test]
fn a_numbered_level_under_a_bullet_remains_ordered() {
    let body = format!("{}{}", list_item(5, 0, "Fruit."), list_item(5, 1, "Apple."));

    let doc = extract(&body);

    assert_eq!(list_markers(&doc), vec![None, None], "content:\n{}", doc.content);
    assert!(doc.content.contains("1. Apple."), "content:\n{}", doc.content);
}

/// A level that writes no `w:numFmt` is decimal (ISO/IEC 29500-1 §17.9.18), so Word paints
/// `Q3.` and `Q4.` here.
#[test]
fn a_level_without_a_number_format_is_numbered_in_decimal() {
    let body = format!(
        "{}{}",
        list_item(6, 0, "Define speed."),
        list_item(6, 0, "Define velocity.")
    );

    let doc = extract(&body);

    assert_eq!(
        list_markers(&doc),
        vec![Some("Q3.".to_string()), Some("Q4.".to_string())],
        "content:\n{}",
        doc.content
    );
    for line in ["- Q3. Define speed.", "- Q4. Define velocity."] {
        assert!(doc.content.contains(line), "missing {line:?} in:\n{}", doc.content);
    }
}

/// Two lists share one definition and only the second overrides its level. Word paints the
/// override's `%1)` from its restart at 4, and keeps one count for both lists, so the first
/// list carries on at 6.
#[test]
fn two_lists_sharing_a_definition_keep_one_count_and_their_own_levels() {
    let body = [
        list_item(7, 0, "First."),
        list_item(7, 0, "Second."),
        list_item(8, 0, "Restarted."),
        list_item(8, 0, "Next."),
        list_item(7, 0, "Carried on."),
    ]
    .concat();

    let doc = extract(&body);

    assert_eq!(
        list_markers(&doc),
        vec![
            None,
            None,
            Some("4)".to_string()),
            Some("5)".to_string()),
            Some("6.".to_string())
        ],
        "content:\n{}",
        doc.content
    );
    for line in ["1. First.\n2. Second.", "6. Carried on."] {
        assert!(doc.content.contains(line), "missing {line:?} in:\n{}", doc.content);
    }
}

#[test]
fn a_numbered_list_in_a_table_cell_continues_the_sequence() {
    let body = format!(
        "{}<w:tbl><w:tr><w:tc>{}</w:tc></w:tr></w:tbl>{}",
        list_item(1, 0, "Before the table."),
        list_item(1, 0, "In the table."),
        list_item(1, 0, "After the table."),
    );

    let doc = extract(&body);

    assert_eq!(
        list_markers(&doc),
        vec![Some("Q1.".to_string()), Some("Q3.".to_string())],
        "content:\n{}",
        doc.content
    );
}
