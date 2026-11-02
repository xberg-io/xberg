use super::*;
use crate::core::config::OutputFormat;
use crate::types::document_structure::NodeContent;

fn table(has_header: bool) -> Table {
    let cells = vec![
        vec!["A1".to_owned(), "B1".to_owned()],
        vec!["A2".to_owned(), "B2".to_owned()],
    ];
    Table {
        columns: has_header.then(|| cells[0].clone()),
        cells,
        ..Default::default()
    }
}

fn document(has_header: bool) -> InternalDocument {
    let mut doc = InternalDocument::new("pdf");
    doc.tables.push(table(has_header));
    doc.push_element(InternalElement::text(ElementKind::Table { table_index: 0 }, "", 0).with_page(1));
    doc
}

fn assert_rendered_cells_once(output: &str) {
    for content in ["A1", "B1", "A2", "B2"] {
        assert_eq!(output.matches(content).count(), 1, "cell {content}: {output}");
    }
}

#[test]
fn headerless_table_keeps_data_cells_in_rendering_and_derived_grid() {
    let result = derive_extraction_result(document(false), true, OutputFormat::Html);
    let html = result.formatted_content.expect("HTML output");
    assert_rendered_cells_once(&html);
    assert_eq!(html.matches("<th>").count(), 0, "{html}");
    assert_eq!(html.matches("<td>").count(), 4, "{html}");
    let structure = result.document.expect("document structure");
    let grid = structure
        .nodes
        .iter()
        .find_map(|node| match &node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .expect("derived table grid");
    assert_eq!(grid.rows, 2);
    assert_eq!(grid.cols, 2);
    assert_eq!(grid.cells.iter().filter(|cell| cell.is_header).count(), 0);
    let markdown = derive_extraction_result(document(false), false, OutputFormat::Markdown)
        .formatted_content
        .expect("Markdown output");
    assert_rendered_cells_once(&markdown);
    assert_eq!(markdown, "| A1 | B1 |\n| --- | --- |\n| A2 | B2 |\n");
}

#[test]
fn explicit_table_header_is_retained_in_rendering_and_derived_grid() {
    let result = derive_extraction_result(document(true), true, OutputFormat::Html);
    let html = result.formatted_content.expect("HTML output");
    assert_rendered_cells_once(&html);
    assert_eq!(html.matches("<th>").count(), 2, "{html}");
    assert_eq!(html.matches("<td>").count(), 2, "{html}");
    let structure = result.document.expect("document structure");
    let grid = structure
        .nodes
        .iter()
        .find_map(|node| match &node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .expect("derived table grid");
    assert_eq!(
        grid.cells.iter().map(|cell| cell.is_header).collect::<Vec<_>>(),
        [true, true, false, false]
    );
    let markdown = derive_extraction_result(document(true), false, OutputFormat::Markdown)
        .formatted_content
        .expect("Markdown output");
    assert_rendered_cells_once(&markdown);
    assert_eq!(markdown, "| A1 | B1 |\n| --- | --- |\n| A2 | B2 |\n");
}

#[test]
fn verified_native_grid_header_metadata_remains_authoritative() {
    let table = table(false);
    let mut native = table_to_grid(&table, None);
    for cell in &mut native.cells {
        cell.is_header = cell.row == 1;
    }
    assert_eq!(
        table_to_grid(&table, Some(&native))
            .cells
            .iter()
            .map(|cell| cell.is_header)
            .collect::<Vec<_>>(),
        [false, false, true, true]
    );
}

#[test]
fn borrowed_header_labels_do_not_mark_the_first_data_row_as_headers() {
    let mut document = document(false);
    document.tables[0].columns = Some(vec!["Known A".to_owned(), "Known B".to_owned()]);
    let result = derive_extraction_result(document.clone(), true, OutputFormat::Html);
    assert_eq!(
        result.tables[0].columns,
        Some(vec!["Known A".to_owned(), "Known B".to_owned()])
    );
    let structure = result.document.expect("document structure");
    let grid = structure
        .nodes
        .iter()
        .find_map(|node| match &node.content {
            NodeContent::Table { grid } => Some(grid),
            _ => None,
        })
        .expect("derived table grid");
    assert_eq!(grid.rows, 2);
    assert_eq!(grid.cols, 2);
    assert_eq!(grid.cells.iter().filter(|cell| cell.is_header).count(), 0);
    let html = result.formatted_content.expect("HTML output");
    assert_rendered_cells_once(&html);
    assert_eq!(html.matches("<th>").count(), 0, "{html}");
    assert_eq!(html.matches("<td>").count(), 4, "{html}");
    assert!(!html.contains("Known A") && !html.contains("Known B"), "{html}");
    let markdown = derive_extraction_result(document, false, OutputFormat::Markdown)
        .formatted_content
        .expect("Markdown output");
    assert_rendered_cells_once(&markdown);
    assert_eq!(markdown, "| A1 | B1 |\n| --- | --- |\n| A2 | B2 |\n");
}
