use super::super::{
    assign_intersection_grid_tracked, build_grid_from_lines, detect_tables_from_intersections,
    detect_tables_with_lines, extract_edges, finalize_intersection_tables_tracked, merge_intersection_tables,
    snap_and_merge, snap_edges, split_intersection_table_at_section_dividers,
};
use super::*;
use crate::elements::PathContent;

fn source(replacements: &[(usize, usize, Option<&str>)]) -> Vec<TextSpan> {
    let mut spans = Vec::new();
    for row in 0..5 {
        for column in 0..3 {
            let label = format!("R{row}C{column}");
            let default = if (row, column) == (0, 0) {
                Some("Anchor")
            } else if (row, column) == (1, 0) {
                None
            } else {
                Some(label.as_str())
            };
            let text = replacements
                .iter()
                .find(|&&(r, c, _)| r == row && c == column)
                .map(|&(_, _, text)| text)
                .unwrap_or(default);
            if let Some(text) = text {
                assert!(!text.is_empty());
                spans.push(TextSpan {
                    text: text.to_owned(),
                    bbox: Rect::new(
                        column as f32 * 100.0 + 8.0,
                        (4 - row) as f32 * 30.0 + 8.0,
                        text.chars().count() as f32 * 5.0,
                        10.0,
                    ),
                    font_name: "source-origin-font".to_owned(),
                    font_size: 10.0,
                    sequence: row * 3 + column,
                    ..Default::default()
                });
            }
        }
    }
    spans
}

fn lines(cut: bool) -> Vec<PathContent> {
    let mut lines: Vec<_> = (0..=5)
        .map(|row| {
            let y = row as f32 * 30.0;
            PathContent::line(if row == 4 { 100.0 } else { 0.0 }, y, 300.0, y)
        })
        .collect();
    for column in 0..=3 {
        let x = column as f32 * 100.0;
        if cut {
            lines.push(PathContent::line(x, 0.0, x, 90.0));
            lines.push(PathContent::line(x, 90.0, x, 150.0));
        } else {
            lines.push(PathContent::line(x, 0.0, x, 150.0));
        }
    }
    lines
}

fn config(minimum: usize) -> TableDetectionConfig {
    TableDetectionConfig {
        min_table_cells: minimum,
        ..TableDetectionConfig::strict()
    }
}

fn before_cut(
    source: &[TextSpan],
    paths: &[PathContent],
    config: &TableDetectionConfig,
) -> (Vec<IntersectionTable>, Vec<Option<GroupOrigin>>) {
    let (groups, edges, physical) = build_grid_from_lines(paths, config);
    assert!(physical);
    assert_eq!(groups.len(), 1, "actual physical group admission must fire");
    let (cells, xs, ys, columns) = &groups[0];
    assert_eq!(cells.len(), 14);
    let grid = assign_intersection_grid_tracked(cells, xs, ys, *columns, source, &edges, physical).unwrap();
    assert_eq!(grid.rows.len(), 5);
    let mut plan = GroupOrigin::from_grid(cells, xs, ys, &grid, physical).unwrap();
    let mut tables = finalize_intersection_tables_tracked(
        grid.rows,
        &grid.indices,
        source,
        config,
        *columns,
        Some((0, &grid.origins, &mut plan)),
    );
    merge_intersection_tables(&mut tables);
    assert!(!tables.is_empty());
    assert!(plan.emissions.iter().all(|&count| count == 1));
    (tables, vec![Some(plan)])
}

fn actual(source: &[TextSpan], paths: &[PathContent], minimum: usize) -> Vec<Table> {
    let before = format!("{source:?}");
    let config = config(minimum);
    let direct = detect_tables_from_intersections(source, paths, &config);
    let main = detect_tables_with_lines(source, paths, &config);
    assert!(!direct.is_empty());
    assert_eq!(format!("{direct:?}"), format!("{main:?}"));
    assert_eq!(format!("{source:?}"), before);
    direct
}

fn anchor(table: &Table) -> &TableCell {
    let anchors: Vec<_> = table
        .rows
        .iter()
        .flat_map(|row| &row.cells)
        .filter(|cell| cell.text.contains("Anchor"))
        .collect();
    assert_eq!(anchors.len(), 1);
    anchors[0]
}

fn assert_untouched_positive() {
    let source = source(&[]);
    let tables = actual(&source, &lines(false), 6);
    assert_eq!(tables.len(), 1);
    assert_eq!(
        anchor(&tables[0]).rowspan,
        2,
        "positive control must exercise real normalization"
    );
    assert_eq!(tables[0].rows[1].cells.len(), 2);
}

#[test]
fn actual_selected_section_cut_declines_cross_cut_footprint() {
    assert_untouched_positive();
    let source = source(&[]);
    let paths = lines(true);
    let config = config(3);
    let (mut before, groups) = before_cut(&source, &paths, &config);
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].table.rows.len(), 5);
    let (mut horizontal, mut vertical) = extract_edges(&paths);
    snap_and_merge(&mut horizontal);
    snap_edges(&mut vertical);
    let cut = split_intersection_table_at_section_dividers(before.remove(0), &horizontal, &vertical, &config);
    assert_eq!(
        cut.iter().map(|table| table.table.rows.len()).collect::<Vec<_>>(),
        [1, 4],
        "the actual existing classifier must select this cut before the safety oracle"
    );
    assert_eq!(
        cut.iter()
            .map(|table| table.origins.as_ref().unwrap().len())
            .collect::<Vec<_>>(),
        [1, 4]
    );
    let normalized: Vec<_> = cut
        .into_iter()
        .map(|table| normalize(table, &groups, &source, &config))
        .collect();
    assert_eq!(anchor(&normalized[0]).rowspan, 1);
    assert_eq!(normalized[1].rows[0].cells.len(), 3);
    assert_eq!(
        normalized
            .iter()
            .flat_map(|table| &table.rows)
            .flat_map(|row| &row.cells)
            .map(|cell| cell.spans.len())
            .sum::<usize>(),
        source.len()
    );
    assert_eq!(format!("{normalized:?}"), format!("{:?}", actual(&source, &paths, 3)));
}

#[test]
fn actual_cleanup_text_is_not_restored_by_rowspan() {
    assert_untouched_positive();
    let source = source(&[(0, 0, Some("1 $14.60"))]);
    let tables = actual(&source, &lines(false), 6);
    assert_eq!(tables.len(), 1);
    let cell = &tables[0].rows[0].cells[0];
    assert_eq!(cell.text, "$14.60", "the actual cleanup prefix classifier must fire");
    assert_eq!(cell.spans[0].text, "1 $14.60");
    assert_eq!(cell.rowspan, 1, "metadata must not reconstruct cleaned source text");
    assert_eq!(tables[0].rows[1].cells.len(), 3);
}

#[test]
fn actual_cleanup_removed_band_invalidates_complete_footprint() {
    assert_untouched_positive();
    let source = source(&[(1, 1, Some("1")), (1, 2, Some("2"))]);
    assert_eq!(source.len(), 14);
    let config = config(6);
    let (before, _) = before_cut(&source, &lines(false), &config);
    assert_eq!(before.len(), 1);
    assert_eq!(
        before[0].table.rows.len(),
        4,
        "actual digit-only row retention must remove the band"
    );
    let tables = actual(&source, &lines(false), 6);
    assert_eq!(tables[0].rows.len(), 4);
    assert_eq!(anchor(&tables[0]).rowspan, 1);
    let retained: Vec<_> = tables[0]
        .rows
        .iter()
        .flat_map(|row| &row.cells)
        .flat_map(|cell| &cell.spans)
        .collect();
    assert_eq!(retained.len(), 12);
    assert!(retained.iter().all(|span| !matches!(span.text.as_str(), "1" | "2")));
}

#[test]
fn actual_rotated_carrier_preserves_legacy_rectangular_reorientation() {
    assert_untouched_positive();
    let mut source = source(&[]);
    for span in &mut source {
        span.rotation_degrees = 90.0;
    }
    let (before, groups) = before_cut(&source, &lines(false), &config(6));
    assert_eq!(super::super::dominant_table_rotation_quadrant(&before[0].table), 90.0);
    let legacy = format!("{:?}", before[0].table);
    let candidate = before.into_iter().next().unwrap();
    let unchanged = normalize(candidate, &groups, &source, &config(6));
    assert_eq!(format!("{unchanged:?}"), legacy);
    let tables = actual(&source, &lines(false), 6);
    assert_eq!(tables.len(), 1);
    assert_eq!((tables[0].rows.len(), tables[0].col_count), (3, 5));
    assert!(tables[0].rows.iter().all(|row| row.cells.len() == 5));
    assert!(
        tables[0]
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .all(|cell| cell.rowspan == 1)
    );
    assert_eq!(
        tables[0]
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .map(|cell| cell.spans.len())
            .sum::<usize>(),
        source.len()
    );
}

#[test]
fn actual_ordinary_blank_separator_and_admission_remain_unchanged() {
    assert_untouched_positive();
    let source = source(&[(1, 1, None), (1, 2, None)]);
    assert_eq!(source.len(), 12);
    let tables = actual(&source, &lines(false), 6);
    assert_eq!(tables.len(), 1);
    assert_eq!(
        tables[0].rows.len(),
        3,
        "blank separator partitions; undersized upper table stays rejected"
    );
    assert_eq!(
        tables[0]
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .map(|cell| cell.spans.len())
            .sum::<usize>(),
        9
    );
    assert!(
        tables[0]
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .all(|cell| cell.rowspan == 1)
    );
    assert!(
        tables[0]
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .all(|cell| !cell.text.contains("Anchor"))
    );
}

#[test]
fn actual_nonempty_cell_admission_floor_abstains_without_dropping_legacy_table() {
    let source = source(&[(1, 0, Some("detail")), (4, 2, None)]);
    assert_eq!(source.len(), 14);
    let (before, _) = before_cut(&source, &lines(false), &config(14));
    assert_eq!(before.len(), 1);
    assert_eq!(
        before[0]
            .table
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .filter(|cell| !cell.text.is_empty())
            .count(),
        14
    );
    let floor = actual(&source, &lines(false), 14);
    assert_eq!(floor.len(), 1);
    assert_eq!(anchor(&floor[0]).rowspan, 1);
    let positive = actual(&source, &lines(false), 13);
    assert_eq!(positive.len(), 1);
    assert_eq!(anchor(&positive[0]).rowspan, 2);
    assert_eq!(anchor(&positive[0]).text, "Anchor\ndetail");
    assert_eq!(
        positive[0]
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .filter(|cell| !cell.text.is_empty())
            .count(),
        13
    );
}

#[test]
fn actual_merge_padding_keeps_unknown_slots_out_of_source_ownership() {
    assert_untouched_positive();
    let mut source = source(&[]);
    for span in &mut source {
        span.bbox.y += 135.0;
    }
    let mut paths = lines(false);
    for path in &mut paths {
        *path = PathContent::line(
            path.bbox.x,
            path.bbox.y + 135.0,
            path.bbox.x + path.bbox.width,
            path.bbox.y + path.bbox.height + 135.0,
        );
    }
    for row in 0..4 {
        for column in 0..4 {
            source.push(TextSpan {
                text: format!("Lower{row}C{column}"),
                bbox: Rect::new(column as f32 * 100.0 + 8.0, (3 - row) as f32 * 30.0 + 8.0, 45.0, 10.0),
                font_size: 10.0,
                font_name: "source-origin-font".to_owned(),
                sequence: 15 + row * 4 + column,
                ..Default::default()
            });
        }
    }
    paths.extend((0..=4).map(|row| PathContent::line(0.0, row as f32 * 30.0, 400.0, row as f32 * 30.0)));
    paths.extend((0..=4).map(|column| PathContent::line(column as f32 * 100.0, 0.0, column as f32 * 100.0, 120.0)));
    assert_eq!(source.len(), 30);
    let config = config(6);
    let (groups, edges, physical) = build_grid_from_lines(&paths, &config);
    assert!(physical);
    assert_eq!(groups.len(), 2, "actual separate ruled groups precede the merge");
    let mut plans = Vec::new();
    let mut candidates = Vec::new();
    for (group, (cells, xs, ys, columns)) in groups.iter().enumerate() {
        let grid = assign_intersection_grid_tracked(cells, xs, ys, *columns, &source, &edges, physical).unwrap();
        let mut plan = GroupOrigin::from_grid(cells, xs, ys, &grid, physical).unwrap();
        candidates.extend(finalize_intersection_tables_tracked(
            grid.rows,
            &grid.indices,
            &source,
            &config,
            *columns,
            Some((group, &grid.origins, &mut plan)),
        ));
        plans.push(Some(plan));
    }
    assert_eq!(candidates.len(), 2);
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.table.col_count)
            .sum::<usize>(),
        7
    );
    merge_intersection_tables(&mut candidates);
    assert_eq!(candidates.len(), 1, "actual existing adjacency predicate must merge");
    assert_eq!(candidates[0].table.col_count, 4);
    let origins = candidates[0].origins.as_ref().unwrap();
    assert_eq!(origins.len(), 9);
    assert_eq!(
        origins
            .iter()
            .flat_map(|row| &row.cells)
            .filter(|cell| cell.is_none())
            .count(),
        5
    );
    assert!(
        origins
            .iter()
            .zip(&candidates[0].table.rows)
            .all(|(origin, row)| origin.cells.len() == row.cells.len())
    );
    let source_count = origins
        .iter()
        .flat_map(|row| &row.cells)
        .flatten()
        .map(|cell| cell.indices.len())
        .sum::<usize>();
    assert_eq!(source_count, 30);
    let normalized = normalize(candidates.remove(0), &plans, &source, &config);
    assert_eq!(anchor(&normalized).rowspan, 2);
    assert_eq!(
        normalized
            .rows
            .iter()
            .flat_map(|row| &row.cells)
            .map(|cell| cell.spans.len())
            .sum::<usize>(),
        30
    );
}
