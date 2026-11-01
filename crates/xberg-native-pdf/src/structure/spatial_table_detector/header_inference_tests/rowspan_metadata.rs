use super::multi_tier::{ROWS, Style, detect_fixture, paths, source};
use super::{COLUMN_WIDTH, PITCH, assert_headerless};
use crate::elements::PathContent;
use crate::geometry::Rect;
use crate::structure::table_extractor::Table;

fn actual_spanning_cell() -> Table {
    let mut source = source(Style::Regular);
    assert_eq!(source[2].text, "Product");
    source.remove(2);
    assert_eq!(source.len(), 13);
    let mut paths = paths();
    let boundary = ROWS - 1;
    let y = boundary as f32 * PITCH;
    assert_eq!(paths[boundary].bbox, Rect::new(0.0, y, 3.0 * COLUMN_WIDTH, 0.0));
    paths[boundary] = PathContent::line(COLUMN_WIDTH, y, 3.0 * COLUMN_WIDTH, y);
    assert_eq!(
        paths[boundary].bbox,
        Rect::new(COLUMN_WIDTH, y, 2.0 * COLUMN_WIDTH, 0.0)
    );
    let table = detect_fixture(&source, &paths);
    assert_headerless(&table);
    assert_eq!(
        table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .filter(|cell| !cell.text.is_empty())
            .count(),
        13
    );
    assert_eq!(
        table.rows[1]
            .cells
            .iter()
            .filter(|cell| !cell.text.is_empty())
            .map(|cell| cell.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Current", "Previous"]
    );
    assert_eq!(
        table.rows[2..].iter().map(|row| row.cells.len()).collect::<Vec<_>>(),
        vec![3, 3, 3]
    );
    assert_eq!(table.rows[0].cells[0].text, "Category");
    assert_eq!(table.rows[0].cells[0].colspan, 1);
    assert_eq!(table.rows[0].cells[0].spans.len(), 1);
    assert!(
        table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .all(|cell| cell.bbox.is_some())
    );
    table
}

#[test]
fn intersection_missing_internal_horizontal_border_retains_rowspan_2332() {
    let table = actual_spanning_cell();
    assert_eq!(
        table.rows[0].cells[0].rowspan, 2,
        "the drawn Category cell crosses both upper source row intervals"
    );
    assert_eq!(
        table.rows[1].cells.len(),
        2,
        "a covered lower slot must not survive as a separate empty cell"
    );
    assert_eq!(table.rows.iter().map(|row| row.cells.len()).sum::<usize>(), 13);
}
