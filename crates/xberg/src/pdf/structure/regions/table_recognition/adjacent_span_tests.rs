use super::metadata_tests::{derived_grid, recognize_rows_with_paint};

#[test]
fn native_tatr_preserves_adjacent_spans_with_vertically_aligned_labels() {
    let rows: &[&[&str]] = &[
        &["Region", "", "", "2024", ""],
        &["", "Country", "", "H1", "H2"],
        &["", "", "Code", "Q1", "Q2"],
        &["North", "US", "NY", "10", "12"],
    ];
    let spans = [
        [0.0, 0.0, 50.0, 80.0],
        [50.0, 0.0, 100.0, 80.0],
        [100.0, 0.0, 150.0, 80.0],
    ];
    let rules = [(20, 150, 250), (40, 150, 250), (60, 0, 250)];
    let result = recognize_rows_with_paint(rows, 3, &spans, &rules, &[]);
    let grid = derived_grid(&result);
    let stubs: Vec<_> = grid
        .cells
        .iter()
        .filter(|cell| cell.row == 0 && cell.col < 3)
        .map(|cell| (cell.row_span, cell.content.as_str()))
        .collect();
    assert_eq!(stubs, [(3, "Region"), (3, "Country"), (3, "Code")]);
}

#[test]
fn native_tatr_adjacent_span_stops_expanding_at_a_contradicted_neighbor() {
    let rows: &[&[&str]] = &[
        &["Region", "Country", "Code", "2024", ""],
        &["", "", "District", "Q1", "Q2"],
        &["North", "US", "NY", "10", "12"],
    ];
    let spans = [
        [0.0, 0.0, 50.0, 60.0],
        [50.0, 0.0, 100.0, 60.0],
        [100.0, 0.0, 150.0, 60.0],
    ];
    let rules = [(20, 100, 250), (40, 0, 250)];
    let without_neighbor = recognize_rows_with_paint(rows, 2, &spans[..2], &rules, &[]);
    let control = derived_grid(&without_neighbor);
    let merged = control.cells.iter().find(|cell| cell.content == "Region").unwrap();
    assert_eq!(
        merged.row_span, 2,
        "the far-side rule alone would confirm the shorter union"
    );

    let result = recognize_rows_with_paint(rows, 2, &spans, &rules, &[]);
    let grid = derived_grid(&result);
    let stubs: Vec<_> = grid
        .cells
        .iter()
        .filter(|cell| cell.row < 2 && cell.col < 3 && !cell.content.is_empty())
        .map(|cell| (cell.row_span, cell.content.as_str()))
        .collect();
    assert_eq!(stubs, [(2, "Region"), (2, "Country"), (1, "Code"), (1, "District")]);
}

#[test]
fn native_tatr_adjacent_opening_accepts_a_shorter_neighbor() {
    let rows: &[&[&str]] = &[
        &["Region", "Country", "2024", ""],
        &["", "", "H1", "H2"],
        &["", "District", "Q1", "Q2"],
        &["North", "US", "10", "12"],
    ];
    let spans = [[0.0, 0.0, 50.0, 80.0], [50.0, 0.0, 100.0, 40.0]];
    let rules = [(20, 100, 200), (40, 50, 200), (60, 0, 200)];
    let result = recognize_rows_with_paint(rows, 3, &spans, &rules, &[]);
    let grid = derived_grid(&result);
    let stubs: Vec<_> = grid
        .cells
        .iter()
        .filter(|cell| cell.row < 3 && cell.col < 2 && !cell.content.is_empty())
        .map(|cell| (cell.row, cell.col, cell.row_span, cell.content.as_str()))
        .collect();
    assert_eq!(
        stubs,
        [(0, 0, 3, "Region"), (0, 1, 2, "Country"), (2, 1, 1, "District")]
    );
}
