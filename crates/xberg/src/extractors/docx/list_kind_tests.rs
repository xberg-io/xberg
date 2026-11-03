use super::build_internal_document;
use crate::extraction::docx::parser::parse_document;
use crate::extractors::security::{SecurityBudget, SecurityLimits};
use crate::types::internal::{ElementKind, InternalDocument};
use std::io::{Cursor, Write};
use zip::write::{SimpleFileOptions, ZipWriter};

const ITEMS: [(&str, usize); 6] = [
    ("Outer first", 0),
    ("Child first", 1),
    ("Grandchild first", 2),
    ("Grandchild second", 2),
    ("Child second", 1),
    ("Outer second", 0),
];

fn mixed_list_package(kinds: [bool; 3]) -> Vec<u8> {
    let levels = kinds.iter().enumerate().map(|(level, ordered)| {
        let format = if *ordered { "decimal" } else { "bullet" };
        let label = if *ordered { format!("%{}.", level + 1) } else { "-".to_string() };
        format!(
            r#"<w:lvl w:ilvl="{level}"><w:start w:val="1"/><w:numFmt w:val="{format}"/><w:lvlText w:val="{label}"/></w:lvl>"#
        )
    }).collect::<String>();
    let paragraphs = ITEMS.iter().map(|(text, level)| format!(
        r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="42"/></w:numPr></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#
    )).collect::<String>();
    let ns = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let document = format!(r#"<w:document xmlns:w="{ns}"><w:body>{paragraphs}</w:body></w:document>"#);
    let numbering = format!(
        r#"<w:numbering xmlns:w="{ns}"><w:abstractNum w:abstractNumId="0">{levels}</w:abstractNum><w:num w:numId="42"><w:abstractNumId w:val="0"/></w:num></w:numbering>"#
    );
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in [
        (
            "[Content_Types].xml",
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/></Types>"#,
        ),
        ("word/document.xml", document.as_str()),
        ("word/numbering.xml", numbering.as_str()),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(content.as_bytes()).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn import_package(bytes: &[u8]) -> InternalDocument {
    let limits = SecurityLimits::default();
    let mut budget = SecurityBudget::from_limits(&limits);
    let parsed = parse_document(bytes, &mut budget, &limits).expect("minimal mixed list package must parse");
    build_internal_document(&parsed, false)
}

fn assert_item_kinds(document: &InternalDocument, kinds: [bool; 3]) {
    let actual = document
        .elements
        .iter()
        .filter_map(|element| match element.kind {
            ElementKind::ListItem { ordered } => Some((element.text.as_str(), ordered, element.depth)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let expected = ITEMS
        .iter()
        .map(|(text, level)| (*text, kinds[*level], (*level + 1) as u16))
        .collect::<Vec<_>>();
    assert_eq!(
        actual, expected,
        "each list item must use its own numbering level's type"
    );
    let containers = document
        .elements
        .iter()
        .filter_map(|element| match element.kind {
            ElementKind::ListStart { ordered } => Some(ordered),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        containers, kinds,
        "the one shared numbering id must retain three distinct level kinds"
    );
}

fn assert_mixed_list_round_trip(kinds: [bool; 3]) {
    let document = import_package(&mixed_list_package(kinds));
    assert_item_kinds(&document, kinds);
    let markdown = crate::rendering::render_markdown(&document);
    let lines = markdown
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let expected = ITEMS
        .iter()
        .enumerate()
        .map(|(index, (text, level))| {
            let ordinal = if matches!(index, 3..=5) { 2 } else { 1 };
            if kinds[*level] {
                format!("{ordinal}. {text}")
            } else {
                format!("- {text}")
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(lines, expected, "rendered nesting must keep each level's marker kind");
    let written = crate::rendering::render_docx(&markdown).expect("mixed list must render to DOCX");
    let reread = import_package(&written);
    assert_item_kinds(&reread, kinds);
    assert_eq!(crate::rendering::render_markdown(&reread), markdown);
}

#[test]
fn shared_numbering_id_keeps_ordered_bullet_ordered_levels() {
    assert_mixed_list_round_trip([true, false, true]);
}

#[test]
fn shared_numbering_id_keeps_bullet_ordered_bullet_levels() {
    assert_mixed_list_round_trip([false, true, false]);
}
