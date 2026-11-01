use super::super::{Table, TableDetectionConfig, detect_tables_from_intersections, header_style_contrasts};
use super::{COLUMN_WIDTH, PITCH, detect, span};
use crate::elements::PathContent;
use crate::layout::TextSpan;

pub(super) const ROWS: usize = 5;
const SOURCE_CELLS: usize = 14;
const DATA: [[&str; 3]; 3] = [["Alpha", "12", "15"], ["Beta", "18", "21"], ["Gamma", "24", "27"]];

#[derive(Clone, Copy)]
pub(super) enum Style {
    TwoTiers,
    SingleTier,
    Regular,
    LaterBoldBody,
}

impl Style {
    fn bold(self, row: usize) -> bool {
        match self {
            Self::TwoTiers => row < 2,
            Self::SingleTier => row == 0,
            Self::Regular => false,
            Self::LaterBoldBody => row == 0 || row == 3,
        }
    }
}

pub(super) fn source(style: Style) -> Vec<TextSpan> {
    let mut source = vec![
        span("Category", 0, 0, ROWS, style.bold(0)),
        span("Measurements", 0, 1, ROWS, style.bold(0)),
    ];
    for (row, cells) in [["Product", "Current", "Previous"]].iter().chain(&DATA).enumerate() {
        for (column, text) in cells.iter().enumerate() {
            source.push(span(text, row + 1, column, ROWS, style.bold(row + 1)));
        }
    }
    assert_eq!(source.len(), SOURCE_CELLS);
    source
}

pub(super) fn paths() -> Vec<PathContent> {
    let height = ROWS as f32 * PITCH;
    let mut paths = (0..=ROWS)
        .map(|row| PathContent::line(0.0, row as f32 * PITCH, 3.0 * COLUMN_WIDTH, row as f32 * PITCH))
        .collect::<Vec<_>>();
    for column in 0..=3 {
        let top = if column == 2 { height - PITCH } else { height };
        let x = column as f32 * COLUMN_WIDTH;
        paths.push(PathContent::line(x, 0.0, x, top));
    }
    paths
}

pub(super) fn detect_fixture(source: &[TextSpan], paths: &[PathContent]) -> Table {
    let config = TableDetectionConfig {
        min_table_cells: 6,
        ..TableDetectionConfig::strict()
    };
    let before = serde_json::to_value(source).unwrap();
    let intersections = detect_tables_from_intersections(source, paths, &config);
    assert_eq!(intersections.len(), 1, "the actual intersection producer must fire");
    let table = detect(source, paths, ROWS, 6);
    assert_eq!(format!("{table:?}"), format!("{:?}", intersections[0]));
    assert_eq!(serde_json::to_value(source).unwrap(), before);
    assert_eq!(table.rows[0].cells.len(), 2);
    assert_eq!(
        (table.rows[0].cells[0].text.as_str(), table.rows[0].cells[0].colspan),
        ("Category", 1)
    );
    assert_eq!(
        (table.rows[0].cells[1].text.as_str(), table.rows[0].cells[1].colspan),
        ("Measurements", 2)
    );
    let source_texts: Vec<_> = source.iter().map(|span| span.text.as_str()).collect();
    let table_texts: Vec<_> = table
        .rows
        .iter()
        .flat_map(|row| &row.cells)
        .filter(|cell| !cell.text.is_empty())
        .map(|cell| cell.text.as_str())
        .collect();
    assert_eq!(
        table_texts, source_texts,
        "each source cell must survive exactly once and in order"
    );
    table
}

fn assert_fixture(table: &Table) {
    assert_eq!(
        table.rows.iter().map(|row| row.cells.len()).collect::<Vec<_>>(),
        vec![2, 3, 3, 3, 3]
    );
    assert_eq!(
        table.rows.iter().map(|row| row.cells.len()).sum::<usize>(),
        SOURCE_CELLS
    );
    assert!(
        table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .all(|cell| cell.rowspan == 1)
    );
    let first: Vec<_> = table.rows[0].cells.iter().flat_map(|cell| &cell.spans).collect();
    let rest: Vec<_> = table.rows[1..]
        .iter()
        .flat_map(|row| &row.cells)
        .flat_map(|cell| &cell.spans)
        .collect();
    assert!(
        header_style_contrasts(&first, &rest),
        "unchanged first-row inference must already qualify"
    );
    assert!(table.has_header && table.rows[0].is_header);
    assert!(table.rows[0].cells.iter().all(|cell| cell.is_header));
    assert!(
        table.rows[2..]
            .iter()
            .all(|row| !row.is_header && row.cells.iter().all(|cell| !cell.is_header))
    );
}

#[test]
fn intersection_two_tier_headers_keep_lower_roles_with_colspan_2332() {
    let table = detect_fixture(&source(Style::TwoTiers), &paths());
    assert_fixture(&table);
    assert!(
        table.rows[1].is_header && table.rows[1].cells.iter().all(|cell| cell.is_header),
        "the source's bold lower header tier must retain header roles"
    );
}

#[test]
fn intersection_single_tier_does_not_invent_lower_headers_2332() {
    let table = detect_fixture(&source(Style::SingleTier), &paths());
    assert_fixture(&table);
    assert!(!table.rows[1].is_header && table.rows[1].cells.iter().all(|cell| !cell.is_header));
}

#[test]
fn intersection_regular_merged_grid_keeps_unknown_header_roles_2332() {
    let table = detect_fixture(&source(Style::Regular), &paths());
    assert_eq!(
        table.rows.iter().map(|row| row.cells.len()).sum::<usize>(),
        SOURCE_CELLS
    );
    super::assert_headerless(&table);
}

#[test]
fn intersection_later_bold_body_does_not_extend_header_band_2332() {
    let table = detect_fixture(&source(Style::LaterBoldBody), &paths());
    assert_fixture(&table);
    assert!(!table.rows[1].is_header && table.rows[1].cells.iter().all(|cell| !cell.is_header));
}
