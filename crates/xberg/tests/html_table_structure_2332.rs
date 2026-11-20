//! GH#2332: HTML tables must reach HTML output with the header cells and merge
//! spans their source declares, instead of being flattened into body cells.

#![cfg(feature = "html")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::ExtractedDocument;
use xberg::core::config::{ExtractionConfig, OutputFormat};

fn html_document(source: &str) -> ExtractedDocument {
    let config = ExtractionConfig {
        output_format: OutputFormat::Html,
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

#[test]
fn source_header_row_keeps_its_header_cells_and_group() {
    let document = html_document(
        r#"<table>
<tr><th>Name</th><th>Score</th></tr>
<tr><td>Alice</td><td>10</td></tr>
</table>"#,
    );

    assert_eq!(
        first_table(&document.content),
        concat!(
            "<table>\n<thead>\n<tr><th>Name</th><th>Score</th></tr>\n</thead>\n",
            "<tbody>\n<tr><td>Alice</td><td>10</td></tr>\n</tbody>\n</table>"
        )
    );
    assert_eq!(
        document.tables[0].columns,
        Some(vec!["Name".to_string(), "Score".to_string()])
    );
}

#[test]
fn data_row_without_a_header_stays_a_data_row() {
    let document = html_document(
        r#"<table>
<tr><td>Alice</td><td>10</td></tr>
<tr><td>Bob</td><td>20</td></tr>
</table>"#,
    );

    let html = first_table(&document.content);
    assert!(!html.contains("<th"), "no source header cell may be invented: {html}");
    assert!(!html.contains("<thead"), "no header group may be invented: {html}");
    assert_eq!(html.matches("<td>Alice</td>").count(), 1, "{html}");
    assert_eq!(document.tables[0].columns, None);
}

#[test]
fn rowspan_survives_into_html_output() {
    let document = html_document(
        r#"<table>
<tr><th>Group</th><th>Name</th><th>Score</th></tr>
<tr><td rowspan="2">Alpha</td><td>Alice</td><td>10</td></tr>
<tr><td>Bob</td><td>20</td></tr>
</table>"#,
    );

    let html = first_table(&document.content);
    assert_eq!(
        html,
        concat!(
            "<table>\n<thead>\n<tr><th>Group</th><th>Name</th><th>Score</th></tr>\n</thead>\n",
            "<tbody>\n",
            "<tr><td rowspan=\"2\">Alpha</td><td>Alice</td><td>10</td></tr>\n",
            "<tr><td>Bob</td><td>20</td></tr>\n",
            "</tbody>\n</table>"
        )
    );
    assert_eq!(
        html.matches("<td").count(),
        5,
        "every source data cell is emitted once and the spanned slot is not: {html}"
    );
}

#[test]
fn colspan_survives_into_html_output() {
    let document = html_document(
        r#"<table>
<tr><th colspan="2">Fuse</th><th>Circuit</th></tr>
<tr><td>101</td><td>40A</td><td>Blower</td></tr>
</table>"#,
    );

    let html = first_table(&document.content);
    assert_eq!(
        html,
        concat!(
            "<table>\n<thead>\n<tr><th colspan=\"2\">Fuse</th><th>Circuit</th></tr>\n</thead>\n",
            "<tbody>\n<tr><td>101</td><td>40A</td><td>Blower</td></tr>\n</tbody>\n</table>"
        )
    );
    assert_eq!(
        document.tables[0].cells[0],
        vec!["Fuse".to_string(), String::new(), "Circuit".to_string()],
        "the covered column stays empty in the dense grid"
    );
}

#[test]
fn rowspan_past_the_last_row_clamps_to_the_grid() {
    let document = html_document(
        r#"<table>
<tr><th>A</th><th>B</th></tr>
<tr><td rowspan="5">x</td><td>1</td></tr>
<tr><td>2</td></tr>
</table>"#,
    );

    let html = first_table(&document.content);
    assert!(
        html.contains("<td rowspan=\"2\">x</td>"),
        "a rowspan that outruns the table must clamp to its last row, as a browser does: {html}"
    );
}

#[test]
fn row_header_in_a_data_row_keeps_its_role() {
    let document = html_document(
        r#"<table>
<tr><th>Name</th><th>Score</th></tr>
<tr><th>Alice</th><td>10</td></tr>
</table>"#,
    );

    let html = first_table(&document.content);
    assert!(html.contains("<th>Alice</th>"), "{html}");
    assert!(html.contains("<td>10</td>"), "{html}");
    assert_eq!(html.matches("<th>").count(), 3, "{html}");
}

#[test]
fn consecutive_header_rows_share_the_leading_header_group() {
    let document = html_document(
        r#"<table>
<tr><th>Zone</th><th>Zone</th></tr>
<tr><th>Room</th><th>Area</th></tr>
<tr><td>101</td><td>40</td></tr>
</table>"#,
    );

    assert_eq!(
        first_table(&document.content),
        concat!(
            "<table>\n<thead>\n",
            "<tr><th>Zone</th><th>Zone</th></tr>\n",
            "<tr><th>Room</th><th>Area</th></tr>\n",
            "</thead>\n<tbody>\n<tr><td>101</td><td>40</td></tr>\n</tbody>\n</table>"
        )
    );
}

#[test]
fn a_table_caption_is_published_once() {
    let document = html_document(
        r#"<table>
<caption>Results</caption>
<tr><th>Name</th><th>Score</th></tr>
<tr><td>Alice</td><td>10</td></tr>
</table>"#,
    );

    assert_eq!(
        document.content.matches("Results").count(),
        1,
        "the caption must not be published on both sides of its table: {}",
        document.content
    );
    assert_eq!(
        first_table(&document.content),
        concat!(
            "<table>\n<thead>\n<tr><th>Name</th><th>Score</th></tr>\n</thead>\n",
            "<tbody>\n<tr><td>Alice</td><td>10</td></tr>\n</tbody>\n</table>"
        )
    );
}

#[test]
fn sibling_tables_stay_separate_tables() {
    let document = html_document(
        r#"<table><tr><th>A</th></tr><tr><td>1</td></tr></table>
<p>between</p>
<table><tr><th>B</th></tr><tr><td>2</td></tr></table>"#,
    );

    assert_eq!(document.content.matches("<table>").count(), 2, "{}", document.content);
    assert_eq!(document.content.matches("</table>").count(), 2);
    assert_eq!(document.tables.len(), 2);
    assert_eq!(
        document.tables[0].cells,
        vec![vec!["A".to_string()], vec!["1".to_string()]]
    );
    assert_eq!(
        document.tables[1].cells,
        vec![vec!["B".to_string()], vec!["2".to_string()]]
    );
}
