//! GH#2332: the retained grid is the source's per-cell role record. Where it proves
//! that row 0 is not a header, Markdown must not promote that record row into the GFM
//! header just because the syntax needs one.
//!
//! Fixtures: `fixtures/gh2332_header_evidence/generate.py`.

#![cfg(feature = "pdf")]

mod helpers;

use flate2::read::GzDecoder;
use helpers::extract_bytes_document_blocking;
use serde::Deserialize;
use std::io::Read;
use xberg::extraction::derive::derive_extraction_result;
use xberg::types::document_structure::NodeContent;
use xberg::types::internal::{ElementKind, InternalDocument, InternalElement};
use xberg::{ExtractedDocument, ExtractionConfig, OutputFormat, Table, TableGrid};

const RETAINED_ROLES: &str = include_str!("fixtures/gh2332_header_evidence/retained-role-tables.json");
const UNIFORM_RULES: &[u8] = include_bytes!("fixtures/gh2332_header_evidence/uniform-rules-body-labels.pdf.gz");
const GRID_ATTRIBUTE: &str = "xberg:internal:native-table-grid";

fn extract(compressed: &[u8], output_format: OutputFormat) -> ExtractedDocument {
    let mut pdf = Vec::new();
    GzDecoder::new(compressed).read_to_end(&mut pdf).unwrap();
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        output_format,
        include_document_structure: true,
        ..Default::default()
    };
    extract_bytes_document_blocking(&pdf, "application/pdf", &config).unwrap()
}

fn published_grid(document: &ExtractedDocument) -> &TableGrid {
    document
        .document
        .as_ref()
        .expect("requested document structure")
        .nodes
        .iter()
        .find_map(|node| match &node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .expect("the fixture is one table")
}

#[derive(Deserialize)]
struct Case {
    table: Table,
    grid: TableGrid,
}

fn retained_case(name: &str) -> Case {
    let fixture: serde_json::Value = serde_json::from_str(RETAINED_ROLES).expect("retained-role fixture");
    serde_json::from_value(fixture.get(name).expect("named fixture case").clone()).expect("well-formed table case")
}

fn retained_document(case: &Case) -> InternalDocument {
    let mut doc = InternalDocument::new("pdf");
    doc.tables.push(case.table.clone());
    let mut element = InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0);
    element.page = Some(1);
    element
        .attributes
        .get_or_insert_with(Default::default)
        .insert(GRID_ATTRIBUTE.to_owned(), serde_json::to_string(&case.grid).unwrap());
    doc.push_element(element);
    doc
}

fn markdown(case: &Case) -> String {
    derive_extraction_result(retained_document(case), true, OutputFormat::Markdown)
        .formatted_content
        .expect("Markdown output")
}

#[test]
fn a_grid_without_header_roles_does_not_promote_the_first_record() {
    // `columns` is populated so the only evidence against a header is the grid's roles.
    let case = retained_case("no_header_roles");
    let labels: Vec<String> = ["Region", "Code", "Rate", "Note"].map(str::to_owned).to_vec();
    assert_eq!(case.table.columns.as_deref(), Some(labels.as_slice()));

    let rendered = markdown(&case);
    let mut lines = rendered.lines();
    let header = lines.next().expect("a GFM header row");
    assert!(
        !header.chars().any(char::is_alphabetic),
        "dense columns alone must not promote a record row: {rendered}"
    );
    assert!(
        lines.next().is_some_and(|separator| separator.contains("---")),
        "a GFM table still has its delimiter row: {rendered}"
    );
    assert!(
        lines.next().is_some_and(|row| row.contains("Region")),
        "the source's first row stays the first data row: {rendered}"
    );
}

#[test]
fn a_source_header_role_stays_the_markdown_header() {
    let case = retained_case("header_roles");
    let rendered = markdown(&case);
    let header = rendered.lines().next().expect("a GFM header row");
    assert!(
        header.contains("Region") && header.contains("Rate"),
        "the source header role keeps the header row: {rendered}"
    );
    assert!(
        rendered.lines().nth(2).is_some_and(|row| row.contains("North")),
        "{rendered}"
    );
}

/// HTML already reports every cell of this rule as body; Markdown still promotes the
/// first band because the native pipeline leaves `columns` empty without recording the
/// abstention. That evidence (`NATIVE_PDF_HEADERLESS_TABLE_ATTRIBUTE`) is only emitted
/// for financial rows today, so the fix belongs to the native table detector (#3348).
#[test]
#[ignore = "needs the native headerless marker; tracked in #3348"]
fn native_rule_without_header_evidence_is_not_promoted_in_markdown() {
    let html = extract(UNIFORM_RULES, OutputFormat::Html);
    assert!(!published_grid(&html).cells.iter().any(|cell| cell.is_header));
    assert!(!html.content.contains("<th"), "{}", html.content);

    let markdown = extract(UNIFORM_RULES, OutputFormat::Markdown);
    let header = markdown.content.lines().next().expect("a GFM header row");
    assert!(
        !header.chars().any(char::is_alphabetic),
        "the first data row must not become the header: {}",
        markdown.content
    );
}
