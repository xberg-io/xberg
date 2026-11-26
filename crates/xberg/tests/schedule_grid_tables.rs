//! Synthetic construction-sheet schedules next to plan geometry.
//!
//! A schedule is a line grid with small text, often rotated on the sheet. These
//! tests drive the existing spatial table detector with a rotated schedule and
//! with an upright one, on the same page as plan walls, and assert that each
//! schedule comes back as exactly one table with the expected rows and columns
//! while the plan geometry does not join it.

#![cfg(feature = "pdf")]

use xberg_native_pdf::elements::PathContent;
use xberg_native_pdf::layout::{Color, FontWeight, TextSpan};
use xberg_native_pdf::structure::TableDetectionConfig;
use xberg_native_pdf::structure::spatial_table_detector::{TableStrategy, detect_tables_with_lines};

fn span(text: &str, left: f32, bottom: f32, width: f32, height: f32, rotation: f32) -> TextSpan {
    TextSpan {
        provenance: None,
        artifact_type: None,
        text: text.to_string(),
        bbox: xberg_native_pdf::geometry::Rect::new(left, bottom, width, height),
        font_name: "ScheduleFont".into(),
        font_size: 8.0,
        font_weight: FontWeight::Normal,
        is_italic: false,
        is_monospace: false,
        color: Color::black(),
        mcid: None,
        mcid_scope: None,
        sequence: 0,
        split_boundary_before: false,
        offset_semantic: false,
        char_spacing: 0.0,
        word_spacing: 0.0,
        horizontal_scaling: 1.0,
        primary_detected: false,
        char_widths: vec![],
        char_x_offsets: Vec::new(),
        heading_level: None,
        rotation_degrees: rotation,
        wmode: 0,
        text_rise: 0.0,
        rtl_draw_logical: false,
        ligature_glyph: false,
        mirrored: false,
        page_rotation_applied: 0,
    }
}

/// A `columns` by `rows` line grid whose physical top-left corner is
/// `(left, bottom)`. Rules are extended a little past the outer corners so the
/// detector sees full-length crossings.
fn grid_lines(left: f32, bottom: f32, columns: usize, rows: usize, width: f32, height: f32) -> Vec<PathContent> {
    let mut lines = Vec::new();
    for row in 0..=rows {
        let y = bottom + row as f32 * height;
        lines.push(PathContent::line(left - 5.0, y, left + columns as f32 * width + 5.0, y));
    }
    for column in 0..=columns {
        let x = left + column as f32 * width;
        lines.push(PathContent::line(
            x,
            bottom - 5.0,
            x,
            bottom + rows as f32 * height + 5.0,
        ));
    }
    lines
}

/// One span per cell, labelled `R{physical_row}C{physical_column}` and centred on
/// the cell. Physical row 0 is the top band, matching the detector's fixture
/// convention.
fn grid_spans(
    left: f32,
    bottom: f32,
    columns: usize,
    rows: usize,
    width: f32,
    height: f32,
    rotation: f32,
) -> Vec<TextSpan> {
    let mut spans = Vec::new();
    for row in 0..rows {
        let y = bottom + (rows - 1 - row) as f32 * height + height / 2.0;
        for column in 0..columns {
            let x = left + column as f32 * width + width / 2.0;
            spans.push(span(&format!("R{row}C{column}"), x - 5.0, y - 4.0, 10.0, 8.0, rotation));
        }
    }
    spans
}

/// Plan walls far from the schedule: a room outline with a door gap and two
/// partitions. No cell text, so no table is expected from it.
fn plan_geometry(left: f32, bottom: f32) -> Vec<PathContent> {
    vec![
        PathContent::line(left, bottom, left + 120.0, bottom),
        PathContent::line(left + 150.0, bottom, left + 300.0, bottom),
        PathContent::line(left, bottom, left, bottom + 200.0),
        PathContent::line(left, bottom + 200.0, left + 300.0, bottom + 200.0),
        PathContent::line(left + 300.0, bottom, left + 300.0, bottom + 200.0),
        PathContent::line(left + 150.0, bottom + 80.0, left + 150.0, bottom + 200.0),
        PathContent::line(left, bottom + 80.0, left + 80.0, bottom + 80.0),
    ]
}

fn config() -> TableDetectionConfig {
    TableDetectionConfig {
        horizontal_strategy: TableStrategy::Lines,
        vertical_strategy: TableStrategy::Lines,
        min_table_cells: 4,
        min_table_columns: 2,
        max_table_columns: 12,
        ..TableDetectionConfig::default()
    }
}

fn row_texts(table: &xberg_native_pdf::structure::Table) -> Vec<Vec<&str>> {
    table
        .rows
        .iter()
        .map(|row| row.cells.iter().map(|cell| cell.text.as_str()).collect())
        .collect()
}

#[test]
fn a_rotated_schedule_is_one_table_and_does_not_absorb_plan_walls() {
    // 4 physical columns x 3 physical rows, rotated 90 degrees.
    let mut lines = grid_lines(0.0, 0.0, 4, 3, 30.0, 20.0);
    let spans = grid_spans(0.0, 0.0, 4, 3, 30.0, 20.0, 90.0);
    // Plan geometry 400 points to the right.
    lines.extend(plan_geometry(400.0, 0.0));

    let tables = detect_tables_with_lines(&spans, &lines, &config());
    assert_eq!(tables.len(), 1, "the plan walls must not become a second table");
    let table = &tables[0];

    assert_eq!(
        table.col_count, 3,
        "the schedule's 4 physical columns transpose into 3 table columns, got {}",
        table.col_count
    );
    assert_eq!(
        table.rows.len(),
        4,
        "the schedule's 3 physical rows become 4 table rows"
    );
    assert_eq!(
        row_texts(table),
        vec![
            vec!["R2C0", "R1C0", "R0C0"],
            vec!["R2C1", "R1C1", "R0C1"],
            vec!["R2C2", "R1C2", "R0C2"],
            vec!["R2C3", "R1C3", "R0C3"],
        ],
        "each logical row must read bottom-to-top in physical space"
    );
    let bbox = table.bbox.expect("the schedule has bounds");
    assert!(
        bbox.x + bbox.width < 200.0,
        "the table must stay inside the schedule, got {bbox:?}"
    );

    // The plan text must not have been swept into the schedule.
    assert!(
        table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .all(|cell| cell.text.starts_with('R')),
        "plan text leaked into the schedule table"
    );
}

#[test]
fn two_schedules_on_one_sheet_each_come_back_as_their_own_table() {
    let mut lines = grid_lines(0.0, 0.0, 4, 3, 30.0, 20.0);
    let mut spans = grid_spans(0.0, 0.0, 4, 3, 30.0, 20.0, 90.0);

    // An upright 3 x 3 schedule far to the right.
    lines.extend(grid_lines(2000.0, 0.0, 3, 3, 40.0, 20.0));
    spans.extend(grid_spans(2000.0, 0.0, 3, 3, 40.0, 20.0, 0.0));

    // Plan geometry below both schedules.
    lines.extend(plan_geometry(0.0, 400.0));

    let tables = detect_tables_with_lines(&spans, &lines, &config());
    assert_eq!(tables.len(), 2, "each schedule is one table; the plan is not");

    let rotated = tables
        .iter()
        .find(|table| table.bbox.is_some_and(|bbox| bbox.x < 200.0))
        .expect("the rotated schedule");
    assert_eq!(rotated.col_count, 3);
    assert_eq!(rotated.rows.len(), 4);

    let upright = tables
        .iter()
        .find(|table| table.bbox.is_some_and(|bbox| bbox.x >= 1900.0))
        .expect("the upright schedule");
    assert_eq!(upright.col_count, 3);
    assert_eq!(upright.rows.len(), 3);
    assert_eq!(
        row_texts(upright),
        vec![
            vec!["R0C0", "R0C1", "R0C2"],
            vec!["R1C0", "R1C1", "R1C2"],
            vec!["R2C0", "R2C1", "R2C2"],
        ]
    );
}
