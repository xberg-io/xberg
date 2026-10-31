use super::*;
use crate::elements::PathContent;
use crate::geometry::Rect;
use crate::layout::FontWeight;

const CELLS: [[&str; 3]; 3] = [
    ["Code", "Count", "Status"],
    ["ItemA", "17", "Ready"],
    ["ItemB", "23", "Done"],
];
const PITCH: f32 = 30.0;
const COLUMN_WIDTH: f32 = 100.0;

fn grid(rows: usize, caption: bool) -> Vec<PathContent> {
    let height = rows as f32 * PITCH;
    let mut lines = (0..=rows)
        .map(|row| PathContent::line(0.0, row as f32 * PITCH, 300.0, row as f32 * PITCH))
        .collect::<Vec<_>>();
    for col in 0..=3 {
        let top = if caption && col > 0 && col < 3 {
            height - PITCH
        } else {
            height
        };
        lines.push(PathContent::line(
            col as f32 * COLUMN_WIDTH,
            0.0,
            col as f32 * COLUMN_WIDTH,
            top,
        ));
    }
    lines
}

fn span(text: &str, row: usize, column: usize, rows: usize, bold: bool) -> TextSpan {
    TextSpan {
        text: text.into(),
        bbox: Rect::new(
            8.0 + column as f32 * COLUMN_WIDTH,
            (rows - row - 1) as f32 * PITCH + 8.0,
            text.chars().count() as f32 * 5.49,
            10.98,
        ),
        font_name: if bold { "Subset+Arial,Bold" } else { "Subset+ArialMT" }.into(),
        font_size: 10.98,
        font_weight: if bold { FontWeight::Bold } else { FontWeight::Normal },
        sequence: row * 3 + column,
        char_widths: vec![5.49; text.chars().count()],
        char_x_offsets: (0..text.chars().count())
            .map(|index| 8.0 + column as f32 * COLUMN_WIDTH + index as f32 * 5.49)
            .collect(),
        horizontal_scaling: 1.0,
        ..Default::default()
    }
}

fn spans(cells: &[[&str; 3]], bold: impl Fn(usize, usize) -> bool) -> Vec<TextSpan> {
    let rows = cells.len();
    cells
        .iter()
        .enumerate()
        .flat_map(|(row, cells)| {
            cells
                .iter()
                .enumerate()
                .map(|(column, text)| span(text, row, column, rows, bold(row, column)))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn detect(source: &[TextSpan], paths: &[PathContent], rows: usize, minimum_cells: usize) -> Table {
    let before = serde_json::to_value(source).unwrap();
    let config = TableDetectionConfig {
        min_table_cells: minimum_cells,
        ..TableDetectionConfig::strict()
    };
    let tables = detect_tables_with_lines(source, paths, &config);
    assert_eq!(
        tables.len(),
        1,
        "the actual intersection path must detect one nonempty table: {tables:?}"
    );
    assert_eq!(
        serde_json::to_value(source).unwrap(),
        before,
        "native source witnesses must stay immutable"
    );
    let table = tables.into_iter().next().unwrap();
    assert_eq!((table.rows.len(), table.col_count), (rows, 3));
    assert_eq!(table.bbox, Some(Rect::new(0.0, 0.0, 300.0, rows as f32 * PITCH)));
    let projected = table
        .rows
        .iter()
        .flat_map(|row| row.cells.iter().flat_map(|cell| &cell.spans))
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(projected).unwrap(),
        before,
        "every glyph witness must survive once in cell order"
    );
    table
}

fn assert_headerless(table: &Table) {
    assert!(!table.has_header);
    assert!(
        table
            .rows
            .iter()
            .all(|row| !row.is_header && row.cells.iter().all(|cell| !cell.is_header))
    );
}

#[test]
fn intersection_bold_leading_row_retains_its_whole_header_role() {
    let source = spans(&CELLS, |row, _| row == 0);
    let table = detect(&source, &grid(3, false), 3, 6);
    for (row, expected) in table.rows.iter().zip(CELLS) {
        assert_eq!(
            row.cells.iter().map(|cell| cell.text.as_str()).collect::<Vec<_>>(),
            expected
        );
    }
    assert!(
        table.has_header,
        "located first-row bold/body contrast must be retained"
    );
    assert!(table.rows[0].is_header);
    assert!(table.rows[0].cells.iter().all(|cell| cell.is_header));
    assert!(
        table.rows[1..]
            .iter()
            .all(|row| !row.is_header && row.cells.iter().all(|cell| !cell.is_header))
    );
}

#[test]
fn intersection_equal_normal_rows_have_no_header_evidence() {
    let source = spans(&CELLS, |_, _| false);
    assert_headerless(&detect(&source, &grid(3, false), 3, 6));
}

#[test]
fn intersection_uniform_bold_fragment_has_no_header_body_contrast() {
    let cells = [
        ["Aggregate", "Method", "Unit"],
        ["Combined", "Passing", "Sieve"],
        ["Percent", "ASTM", "Count"],
    ];
    let source = spans(&cells, |_, _| true);
    assert_headerless(&detect(&source, &grid(3, false), 3, 6));
}

#[test]
fn intersection_bold_row_labels_do_not_become_column_headers() {
    let source = spans(&CELLS, |_, column| column == 0);
    assert_headerless(&detect(&source, &grid(3, false), 3, 6));
}

#[test]
fn intersection_single_spanning_caption_is_not_a_column_header_row() {
    let mut source = vec![span("Schedule", 0, 0, 3, true)];
    for (row, cells) in CELLS.iter().enumerate().skip(1) {
        for (column, text) in cells.iter().enumerate() {
            source.push(span(text, row, column, 3, false));
        }
    }
    let table = detect(&source, &grid(3, true), 3, 6);
    assert_eq!(table.rows[0].cells.len(), 1);
    assert_eq!(
        (table.rows[0].cells[0].text.as_str(), table.rows[0].cells[0].colspan),
        ("Schedule", 3)
    );
    assert_headerless(&table);
}

#[test]
fn intersection_header_only_row_has_no_body_for_style_comparison() {
    // Admit one real row so the negative cannot pass because the detector discarded the table. ~keep
    let source = spans(&CELLS[..1], |_, _| true);
    assert_headerless(&detect(&source, &grid(1, false), 1, 3));
}
