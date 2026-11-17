use super::*;

fn single_title_rows() -> [TableRow; 4] {
    [
        row(vec![
            cell("Discount Factor", 0, 2, HEIGHT, true),
            cell("", 2, 1, HEIGHT, false),
            cell("", 3, 1, HEIGHT, false),
            cell("", 4, 1, HEIGHT, false),
        ]),
        leaves(["Form", "Device Type", "P1", "P2", "P3"], true),
        row(["HO-3", "No", "1.000", "1.000", "1.000"]
            .iter()
            .enumerate()
            .map(|(column, value)| cell(value, column, 1, -HEIGHT, false))
            .collect()),
        row(["HO-3", "Valve Closure", "1.000", "0.950", "1.000"]
            .iter()
            .enumerate()
            .map(|(column, value)| cell(value, column, 1, -2.0 * HEIGHT, false))
            .collect()),
    ]
}

const FILLED_RULE_WIDTH: f32 = 0.72;

fn filled_rect(x: f32, y: f32, width: f32, height: f32, color: Color) -> PathContent {
    let mut path = PathContent::rect(x, y, width, height);
    path.stroke_color = None;
    path.fill_color = Some(color);
    path
}

fn single_title_rules() -> Vec<PathContent> {
    let width = COLUMNS as f32 * WIDTH;
    let mut paths = vec![filled_rect(
        0.0,
        -2.0 * HEIGHT,
        width,
        4.0 * HEIGHT,
        Color::new(1.0, 1.0, 0.0),
    )];
    paths.extend((0..=COLUMNS).map(|column| {
        // The first divider ends at the title boundary; the others span the full table. ~keep
        let height = if column == 1 { 3.0 * HEIGHT } else { 4.0 * HEIGHT };
        filled_rect(
            column as f32 * WIDTH - FILLED_RULE_WIDTH / 2.0,
            -2.0 * HEIGHT,
            FILLED_RULE_WIDTH,
            height,
            Color::black(),
        )
    }));
    paths.extend((-2..=2).map(|row| {
        filled_rect(
            0.0,
            row as f32 * HEIGHT - FILLED_RULE_WIDTH / 2.0,
            width,
            FILLED_RULE_WIDTH,
            Color::black(),
        )
    }));
    paths
}

fn single_title_roles(rows: &mut [TableRow], paths: &[PathContent]) -> Vec<bool> {
    retain_grouped_leaf_header(rows, paths, COLUMNS);
    rows.iter().map(|row| row.is_header).collect()
}

/// ParseBench 8097f387f220a9b3dbd9, source PDF SHA-256
/// 6b0f7be463671afe94083182922d3362ae2d6d049a43436d139a598c86b63a85.
/// Synthetic reduction of the filled rules and shared yellow paint behind bold leaf labels. ~keep
#[test]
fn single_merged_title_promotes_bold_native_leaf_labels_without_changing_cells() {
    use crate::structure::spatial_table_detector::{TableDetectionConfig, detect_tables_from_intersections};

    let paths = single_title_rules();
    let authored = single_title_rows();
    let mut rows = authored.clone();
    assert_eq!(single_title_roles(&mut rows, &paths), [true, true, false, false]);
    for row in &mut rows {
        row.is_header = false;
        for cell in &mut row.cells {
            cell.is_header = false;
        }
    }
    assert_eq!(format!("{rows:?}"), format!("{authored:?}"));
    let spans: Vec<_> = authored
        .iter()
        .flat_map(|row| &row.cells)
        .flat_map(|cell| cell.spans.iter().cloned())
        .collect();
    let expected: Vec<Vec<_>> = authored
        .iter()
        .map(|row| row.cells.iter().map(|cell| cell.text.as_str()).collect())
        .collect();
    let tables = detect_tables_from_intersections(&spans, &paths, &TableDetectionConfig::default());
    let [table] = tables.as_slice() else {
        panic!("expected one native table, received {}", tables.len());
    };
    assert_eq!(table.col_count, COLUMNS);
    assert_eq!(
        table.rows.iter().map(|row| row.is_header).collect::<Vec<_>>(),
        [true, true, false, false]
    );
    assert_eq!(table.rows[0].cells[0].colspan, 2);
    for (index, row) in table.rows.iter().enumerate() {
        assert_eq!(
            row.cells.iter().map(|cell| cell.text.as_str()).collect::<Vec<_>>(),
            expected[index]
        );
        assert!(row.cells.iter().all(|cell| cell.is_header == (index < 2)));
    }
}

#[test]
fn single_merged_title_does_not_promote_regular_leaves() {
    let mut rows = single_title_rows();
    rows[1] = leaves(["Form", "Device Type", "P1", "P2", "P3"], false);
    assert_eq!(single_title_roles(&mut rows, &single_title_rules()), [false; 4]);
}

#[test]
fn single_merged_title_refuses_a_divider_running_through_the_title() {
    let mut rules = single_title_rules();
    rules.push(filled_rect(
        WIDTH - FILLED_RULE_WIDTH / 2.0,
        HEIGHT,
        FILLED_RULE_WIDTH,
        HEIGHT,
        Color::black(),
    ));
    assert_eq!(single_title_roles(&mut single_title_rows(), &rules), [false; 4]);
}

#[test]
fn single_merged_title_requires_a_continuous_lower_leaf_rule() {
    let mut rules = single_title_rules();
    let lower_rule = rules
        .iter_mut()
        .find(|path| {
            path.is_horizontal_line(FILLED_RULE_WIDTH * 2.0) && path.rendered_bbox().center().y.abs() <= f32::EPSILON
        })
        .expect("the lower leaf rule");
    *lower_rule = filled_rect(
        0.0,
        -FILLED_RULE_WIDTH / 2.0,
        2.0 * WIDTH,
        FILLED_RULE_WIDTH,
        Color::black(),
    );
    rules.push(filled_rect(
        3.0 * WIDTH,
        -FILLED_RULE_WIDTH / 2.0,
        2.0 * WIDTH,
        FILLED_RULE_WIDTH,
        Color::black(),
    ));
    assert_eq!(single_title_roles(&mut single_title_rows(), &rules), [false; 4]);
}

#[test]
fn single_merged_title_refuses_a_divider_with_later_pale_or_dashed_stroke() {
    let mut pale = PathContent::line(WIDTH, 2.0, WIDTH, HEIGHT - 2.0);
    pale.stroke_color = Some(Color::white());
    let mut dashed = pale.clone();
    dashed.stroke_color = Some(Color::black());
    dashed.dash_pattern = Some((vec![1.0, 1.0], 0.0));
    for overpaint in [pale, dashed] {
        let mut earlier = single_title_rules();
        earlier.insert(0, overpaint.clone());
        assert_eq!(
            single_title_roles(&mut single_title_rows(), &earlier),
            [true, true, false, false]
        );

        let mut later = single_title_rules();
        later.push(overpaint);
        assert_eq!(single_title_roles(&mut single_title_rows(), &later), [false; 4]);
    }
}

#[test]
fn single_merged_title_refuses_ink_that_stays_in_the_first_track() {
    let mut rows = single_title_rows();
    let title = &mut rows[0].cells[0];
    title.text = "Rates".to_string();
    title.spans[0].text.clone_from(&title.text);
    title.spans[0].bbox = Rect::new(1.0, HEIGHT + 1.0, WIDTH - 2.0, HEIGHT - 2.0);
    assert_eq!(title.colspan, 2);
    assert_eq!(single_title_roles(&mut rows, &single_title_rules()), [false; 4]);
}

#[test]
fn single_merged_title_refuses_dark_fill_extended_by_a_pale_stroke() {
    let cases = [
        (
            (WIDTH, HEIGHT / 2.0),
            filled_rect(
                WIDTH - FILLED_RULE_WIDTH / 2.0,
                2.0,
                FILLED_RULE_WIDTH,
                HEIGHT - 4.0,
                Color::black(),
            ),
        ),
        (
            (COLUMNS as f32 * WIDTH / 2.0, 0.0),
            filled_rect(
                2.0,
                -FILLED_RULE_WIDTH / 2.0,
                COLUMNS as f32 * WIDTH - 4.0,
                FILLED_RULE_WIDTH,
                Color::black(),
            ),
        ),
    ];
    for ((x, y), mut mixed) in cases {
        let mut rules = single_title_rules();
        let original = rules
            .iter_mut()
            .find(|path| {
                let bounds = path.bbox;
                if y == 0.0 {
                    path.is_horizontal_line(FILLED_RULE_WIDTH * 2.0) && (bounds.center().y - y).abs() <= f32::EPSILON
                } else {
                    path.is_vertical_line(FILLED_RULE_WIDTH * 2.0) && (bounds.center().x - x).abs() <= f32::EPSILON
                }
            })
            .expect("the synthetic rule");
        mixed.stroke_color = Some(Color::white());
        mixed.stroke_width = 4.0;
        *original = mixed;
        assert_eq!(single_title_roles(&mut single_title_rows(), &rules), [false; 4]);
    }
}

fn set_upper_span_overhang(row: &mut TableRow, overhang: f32) {
    for cell in &mut row.cells {
        let bounds = cell.bbox.expect("fixture cell bounds");
        for span in &mut cell.spans {
            span.bbox.height = bounds.bottom() + overhang - span.bbox.top();
        }
    }
}

#[test]
fn single_merged_title_accepts_measured_upper_font_metric_overhang() {
    let mut rows = single_title_rows();
    for row in &mut rows[..3] {
        set_upper_span_overhang(row, 0.36);
    }
    assert!(!complete_source_band(&rows[2], COLUMNS, false));
    let original = rows.clone();
    assert_eq!(
        single_title_roles(&mut rows, &single_title_rules()),
        [true, true, false, false]
    );
    for (index, row) in rows.iter().enumerate() {
        assert!(
            row.cells.iter().all(|cell| cell.is_header == (index < 2)),
            "row {index}"
        );
    }
    for row in &mut rows[..2] {
        row.is_header = false;
        for cell in &mut row.cells {
            cell.is_header = false;
        }
    }
    assert_eq!(format!("{rows:?}"), format!("{original:?}"));
}

#[test]
fn single_merged_title_rejects_overhang_beyond_the_upper_metric_allowance() {
    let paths = single_title_rules();
    for row in 0..3 {
        let mut rows = single_title_rows();
        set_upper_span_overhang(&mut rows[row], 0.51);
        assert_eq!(single_title_roles(&mut rows, &paths), [false; 4], "row {row}");
    }
}

#[test]
fn single_merged_title_rejects_small_overhang_on_each_other_edge() {
    let paths = single_title_rules();
    for row in 0..3 {
        for edge in ["left", "right", "lower"] {
            let mut rows = single_title_rows();
            let cell = &mut rows[row].cells[0];
            let bounds = cell.bbox.expect("fixture cell bounds");
            let span = &mut cell.spans[0].bbox;
            match edge {
                "left" => span.x = bounds.left() - 0.36,
                "right" => span.width = bounds.right() + 0.36 - span.left(),
                "lower" => span.y = bounds.top() - 0.36,
                _ => unreachable!(),
            }
            assert_eq!(single_title_roles(&mut rows, &paths), [false; 4], "row {row}, {edge}");
        }
    }
}

#[test]
fn single_merged_title_allowance_does_not_change_the_legacy_grouped_proof() {
    let paths = standard_rules();
    let mut rows = standard_rows();
    assert!(retained(rows.clone(), &paths));
    set_upper_span_overhang(&mut rows[2], 0.36);
    assert!(!retained(rows, &paths));
}
