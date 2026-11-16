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
fn native_tatr_adjacent_span_vetoes_a_rule_beyond_its_neighbor() {
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
    assert!(grid.cells.iter().all(|cell| cell.row_span == 1));
    assert!(
        grid.cells
            .iter()
            .any(|cell| cell.row == 1 && cell.content == "District")
    );
}
