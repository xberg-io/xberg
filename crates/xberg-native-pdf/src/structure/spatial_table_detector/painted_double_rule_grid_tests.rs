use super::*;
use crate::structure::spatial_table_detector::detect_tables_with_lines;

fn fill(x: f32, y: f32, width: f32, height: f32, color: Color) -> PathContent {
    let mut path = PathContent::rect(x, y, width, height).with_fill(color);
    path.stroke_color = None;
    path
}

fn fixture() -> (Vec<TextSpan>, Vec<PathContent>) {
    fixture_with_bands(&["Total", "Units"])
}

fn fixture_with_bands(labels: &[&str]) -> (Vec<TextSpan>, Vec<PathContent>) {
    let mut spans = Vec::new();
    let mut paths = Vec::new();
    let mut y = 40.;
    for (row, &label) in labels.iter().enumerate() {
        let height = if labels.len() > 3 && row % 2 != 0 { 16. } else { 12. };
        let color = if row % 2 == 0 {
            Color::white()
        } else {
            Color::new(0.8, 0.9, 1.)
        };
        for column in 0..9 {
            paths.push(fill((column * 10) as f32, y, 10., height, color));
        }
        for (column, text) in [(0, label), (2, "$"), (3, "245"), (6, "$"), (7, "15.25")] {
            spans.push(TextSpan {
                text: text.into(),
                bbox: Rect::new((column * 10 + 1) as f32, y + 3., 7., 6.),
                font_size: 8.,
                ..TextSpan::default()
            });
        }
        y += height;
    }
    paths.push(fill(20., 38.5, 20., 0.75, Color::black()));
    paths.push(fill(20., 40., 20., 0.75, Color::black()));
    (spans, paths)
}

#[test]
fn restores_the_painted_amount_columns_without_emitting_gutters() {
    let (spans, paths) = fixture();
    let tables = detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::strict());
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].col_count, 5);
    let rows: Vec<_> = tables[0]
        .rows
        .iter()
        .map(|row| row.cells.iter().map(|cell| cell.text.as_str()).collect::<Vec<_>>())
        .collect();
    assert_eq!(
        rows,
        [["Units", "$", "245", "$", "15.25"], ["Total", "$", "245", "$", "15.25"],]
    );
}

#[test]
fn unmatched_decorative_rule_cannot_restore_columns() {
    let (spans, mut paths) = fixture();
    paths.pop();
    let tables = detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::strict());
    assert!(tables.iter().all(|table| !table.is_real_grid()));
}

#[test]
fn a_source_word_crossing_into_an_occupied_track_prevents_recovery() {
    let (mut spans, paths) = fixture();
    spans[1].bbox.width = 10.;
    let tables = detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::strict());
    assert!(tables.iter().all(|table| !table.is_real_grid()));
}

#[test]
fn recovered_bounds_follow_source_paint_and_include_owned_glyph_overhang() {
    let (mut spans, paths) = fixture();
    spans[2].bbox.width = 9.25;
    spans[4].bbox.width = 9.25;
    let tables = detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::strict());
    let table = &tables[0];
    assert_eq!(table.bbox, Some(Rect::new(0., 40., 80.25, 24.)));
    assert_eq!(table.rows[1].cells[2].bbox, Some(Rect::new(30., 40., 10.25, 12.)));
    assert_eq!(table.rows[1].cells[4].bbox, Some(Rect::new(70., 40., 10.25, 12.)));
    for cell in table.rows.iter().flat_map(|row| &row.cells) {
        assert!(
            cell.spans
                .iter()
                .all(|word| cell.bbox.unwrap().contains_rect(&word.bbox))
        );
    }
}

#[test]
fn a_word_crossing_the_painted_row_edge_prevents_recovery() {
    let (mut spans, paths) = fixture();
    for word in &mut spans[..5] {
        word.bbox.y = 48.;
    }
    let tables = detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::strict());
    assert!(tables.iter().all(|table| !table.is_real_grid()));
}

#[test]
fn two_cells_cannot_claim_overlapping_overhang_in_the_same_gutter() {
    let (mut spans, paths) = fixture();
    spans[0].bbox.width = 17.;
    spans[1].bbox.x = 16.;
    spans[1].bbox.width = 11.;
    let tables = detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::strict());
    assert!(tables.iter().all(|table| !table.is_real_grid()));
}

#[test]
fn restores_three_painted_amount_rows_above_the_double_rule() {
    let labels = ["Total", "Bunits", "Aunits"];
    let (spans, paths) = fixture_with_bands(&labels);
    let tables = detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::strict());
    assert_eq!(tables.len(), 1);
    assert!(tables[0].is_real_grid());
    assert_eq!(tables[0].col_count, 5);
    let rows: Vec<_> = tables[0]
        .rows
        .iter()
        .map(|row| row.cells.iter().map(|cell| cell.text.as_str()).collect::<Vec<_>>())
        .collect();
    let expected: Vec<_> = labels
        .iter()
        .rev()
        .map(|label| [*label, "$", "245", "$", "15.25"])
        .collect();
    assert_eq!(rows, expected);
}

#[test]
fn partly_occupied_spacer_track_prevents_multirow_recovery() {
    let (mut spans, paths) = fixture_with_bands(&["Total", "Bunits", "Aunits"]);
    spans.push(TextSpan {
        text: "Extra".into(),
        bbox: Rect::new(11., 55., 7., 6.),
        font_size: 8.,
        ..TextSpan::default()
    });
    let tables = detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::strict());
    assert!(tables.iter().all(|table| !table.is_real_grid()));
}

#[test]
fn a_full_painted_grid_keeps_all_occupied_tracks() {
    let (mut spans, paths) = fixture_with_bands(&["Total", "Cunits", "Bunits", "Aunits"]);
    let mut y = 40.;
    for row in 0..4 {
        for column in [1, 4, 5, 8] {
            spans.push(TextSpan {
                text: "Q".into(),
                bbox: Rect::new((column * 10 + 1) as f32, y + 3., 7., 6.),
                font_size: 8.,
                ..TextSpan::default()
            });
        }
        y += if row % 2 == 0 { 12. } else { 16. };
    }
    let tables = detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::strict());
    assert_eq!(tables.len(), 1);
    assert!(tables[0].is_real_grid());
    assert_eq!(tables[0].col_count, 9);
    assert_eq!(tables[0].rows.len(), 4);
    assert_eq!(
        tables[0]
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .flat_map(|cell| &cell.spans)
            .count(),
        36
    );
}

#[test]
fn an_already_valid_painted_grid_retains_its_empty_columns() {
    let (spans, paths) = fixture_with_bands(&["Net 2", "R-4", "X?", "10%"]);
    let tables = detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::strict());
    assert_eq!(tables.len(), 1);
    assert!(tables[0].is_real_grid());
    assert_eq!(tables[0].col_count, 9);
    assert_eq!(tables[0].rows.len(), 4);
    let empty_cells = tables[0].rows.iter().filter_map(|row| row.cells.get(1));
    assert!(empty_cells.count() > 0);
    assert!(
        tables[0]
            .rows
            .iter()
            .filter_map(|row| row.cells.get(1))
            .all(|cell| cell.text.is_empty())
    );
}
