//! GH#2332: a source table's header group, merge spans and table boundaries must
//! survive into the rendered output instead of being flattened into body rows.
//!
//! The fixtures are written by `fixtures/html_table_structure_2332/generate.py`;
//! each one isolates a single structural property.

#![cfg(feature = "html")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::ExtractedDocument;
use xberg::core::config::{ExtractionConfig, OutputFormat};

const SPANS: &str = include_str!("fixtures/html_table_structure_2332/spans-and-two-tier-header.html");
const SHORT_ROW: &str = include_str!("fixtures/html_table_structure_2332/short-row.html");
const HEADERLESS: &str = include_str!("fixtures/html_table_structure_2332/headerless-body-rows.html");
const BLANK_GUTTER: &str = include_str!("fixtures/html_table_structure_2332/blank-gutter-tables.html");

fn document(source: &str, output_format: OutputFormat) -> ExtractedDocument {
    let config = ExtractionConfig {
        output_format,
        use_cache: false,
        ..Default::default()
    };
    extract_bytes_document_blocking(source.as_bytes(), "text/html", &config).expect("HTML extraction must succeed")
}

/// The first `<table>` element of the rendered HTML, without surrounding blocks.
fn first_table(html: &str) -> &str {
    let start = html.find("<table").expect("rendered HTML must hold a table");
    let end = html[start..].find("</table>").expect("a rendered table must be closed") + start + "</table>".len();
    &html[start..end]
}

fn cell_tags(row: &str) -> usize {
    row.matches("<th").count() + row.matches("<td").count()
}

#[test]
fn two_tier_header_and_body_spans_keep_the_header_group_and_every_span() {
    let document = document(SPANS, OutputFormat::Html);
    let html = first_table(&document.content);

    assert_eq!(
        html,
        concat!(
            "<table>\n<thead>\n",
            "<tr><th rowspan=\"2\">Item</th><th colspan=\"2\">Measurement</th></tr>\n",
            "<tr><th>Value</th><th>Unit</th></tr>\n",
            "</thead>\n<tbody>\n",
            "<tr><td rowspan=\"2\">Widget</td><td>10</td><td>mm</td></tr>\n",
            "<tr><td>20</td><td>mm</td></tr>\n",
            "</tbody>\n</table>"
        )
    );

    let rows: Vec<&str> = html
        .split("<tr>")
        .skip(1)
        .map(|row| row.split("</tr>").next().unwrap_or_default())
        .collect();
    assert_eq!(rows.len(), 4, "four rows reach the output: {html}");
    assert_eq!(
        rows.iter().map(|row| cell_tags(row)).collect::<Vec<_>>(),
        [2, 2, 3, 2],
        "an upper-tier cell spans two columns and a body cell spans two rows, \
         so each row emits only its own cells: {html}"
    );
    assert_eq!(
        html.matches("<th>").count() + html.matches("<th ").count(),
        4,
        "every source header cell is a header: {html}"
    );
    assert_eq!(
        html.matches("<td").count(),
        5,
        "the row-spanned slot is not emitted twice: {html}"
    );
}

#[test]
fn short_row_is_padded_without_inventing_a_merge() {
    let document = document(SHORT_ROW, OutputFormat::Html);
    let html = first_table(&document.content);

    assert!(
        !html.contains("colspan") && !html.contains("rowspan"),
        "a row that merely has fewer cells is not a merge: {html}"
    );
    assert!(
        html.contains("<tr><td>1</td><td></td><td></td></tr>"),
        "the short row keeps the grid width with empty slots: {html}"
    );
}

#[test]
fn headerless_table_keeps_every_row_a_body_row() {
    let html_document = document(HEADERLESS, OutputFormat::Html);
    assert_eq!(html_document.tables[0].columns, None);

    let html = first_table(&html_document.content);
    assert!(!html.contains("<th"), "no header cell is invented: {html}");
    assert!(!html.contains("<thead"), "no header group is invented: {html}");
    assert_eq!(
        html.matches("<td").count(),
        4,
        "both source rows stay body rows: {html}"
    );
}

#[test]
fn blank_gutter_keeps_sibling_tables_apart() {
    let document = document(BLANK_GUTTER, OutputFormat::Html);
    assert_eq!(document.tables.len(), 2, "the gutter separates two source tables");
    assert_eq!(document.content.matches("<table>").count(), 2, "{}", document.content);
    assert_eq!(document.tables[0].cells[1], ["North", "350"]);
    assert_eq!(document.tables[1].cells[1], ["East", "470"]);
}
