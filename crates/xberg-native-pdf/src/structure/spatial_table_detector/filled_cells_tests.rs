use super::*;

fn painted_cell(x: f32, y: f32, width: f32, height: f32, color: crate::layout::Color) -> crate::elements::PathContent {
    let mut path = crate::elements::PathContent::rect(x, y, width, height);
    path.stroke_color = None;
    path.fill_color = Some(color);
    path
}

fn painted_rows(decorated: bool) -> (Vec<TextSpan>, Vec<crate::elements::PathContent>) {
    use crate::layout::Color;
    let mut spans = Vec::new();
    let mut paths = Vec::new();
    for row in 0..4 {
        let top = 120. - row as f32 * 24.;
        let color = if row % 2 == 0 {
            Color::new(0.8, 0.93, 1.)
        } else {
            Color::white()
        };
        for (x, width) in [(0., 40.), (40., 5.), (45., 200.)] {
            paths.push(painted_cell(x, top - 24., width, 24., color));
        }
        for (text, x, y, width) in [
            (format!("R{row}"), 2., top - 10., 18.),
            (format!("Record {row} source"), 47., top - 10., 190.),
            ("continued".into(), 47., top - 21., 160.),
        ] {
            spans.push(TextSpan {
                text,
                bbox: crate::geometry::Rect::new(x, y, width, 8.),
                font_size: 9.,
                ..Default::default()
            });
        }
    }
    if decorated {
        for row in 0..4 {
            let top = 120. - row as f32 * 24.;
            if row % 2 == 1 {
                paths.push(painted_cell(0., top - 12., 20., 12., Color::white()));
                paths.push(painted_cell(45., top - 12., 192., 12., Color::white()));
                paths.push(painted_cell(45., top - 24., 162., 12., Color::white()));
            }
            paths.push(painted_cell(47., top - 11.5, 190., 0.7, Color::new(0., 0., 0.93)));
            paths.push(painted_cell(47., top - 22.5, 160., 0.7, Color::new(0., 0., 0.93)));
        }
    }
    (spans, paths)
}

#[test]
fn text_paint_inside_tiled_source_cells_does_not_repartition_records() {
    let (spans, backgrounds) = painted_rows(false);
    let config = TableDetectionConfig::strict();
    let expected = super::super::detect_tables_with_lines(&spans, &backgrounds, &config);
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].rows.len(), 4);
    let (spans, decorations) = painted_rows(true);
    let actual = super::super::detect_tables_with_lines(&spans, &decorations, &config);
    assert_eq!(
        super::super::header_source_diagnostic::tables(&actual),
        super::super::header_source_diagnostic::tables(&expected)
    );
}

#[test]
fn incomplete_or_overlapping_tiles_do_not_hide_source_boundaries() {
    let (spans, paths) = painted_rows(true);
    let config = TableDetectionConfig::default();
    let mut missing = paths.clone();
    missing.remove(1);
    assert!(prepare(&spans, &missing, &config).is_none());

    let mut overlapping = paths.clone();
    overlapping.push(painted_cell(39., 96., 8., 24., crate::layout::Color::white()));
    assert!(prepare(&spans, &overlapping, &config).is_none());

    let mut page_background = paths;
    page_background.insert(0, painted_cell(-10., 0., 270., 140., crate::layout::Color::white()));
    assert!(prepare(&spans, &page_background, &config).is_none());
}

#[test]
fn only_text_decoration_may_disappear_from_a_certified_cell() {
    let (spans, paths) = painted_rows(true);
    let config = TableDetectionConfig::default();
    assert!(prepare(&spans, &paths, &config).is_some());
    let mut unmatched = paths.clone();
    unmatched.push(painted_cell(60., 99., 40., 0.7, crate::layout::Color::black()));
    assert!(prepare(&spans, &unmatched, &config).is_none());

    let mut nested_grid = paths.clone();
    for y in [98., 104.] {
        for x in [60., 80.] {
            nested_grid.push(painted_cell(x, y, 20., 6., crate::layout::Color::white()));
        }
    }
    assert!(prepare(&spans, &nested_grid, &config).is_none());

    let mut ruled = paths;
    let mut hairline = PathContent::line(55., 103., 190., 103.);
    hairline.stroke_width = 0.;
    ruled.push(hairline);
    assert!(prepare(&spans, &ruled, &config).is_none());
}

#[test]
fn mixed_paint_or_word_frames_keep_original_detection() {
    let (spans, paths) = painted_rows(true);
    let config = TableDetectionConfig::default();
    let mut transformed = paths.clone();
    transformed[0].matrix = Some([1., 0., 0., 1., 10., 0.]);
    assert!(prepare(&spans, &transformed, &config).is_none());

    let mut layered = paths.clone();
    layered[0].layer = Some("optional background".into());
    assert!(prepare(&spans, &layered, &config).is_none());

    let mut rotated = spans.clone();
    rotated[0].rotation_degrees = 90.;
    assert!(prepare(&rotated, &paths, &config).is_none());

    let mut invalid = spans;
    invalid[0].bbox.x = f32::NAN;
    assert!(prepare(&invalid, &paths, &config).is_none());
}

#[test]
fn painted_prose_or_figures_do_not_supply_paired_source_cells() {
    let (mut spans, paths) = painted_rows(true);
    let config = TableDetectionConfig::default();
    let mut unowned = spans.clone();
    unowned[0].bbox.width = 48.;
    assert!(prepare(&unowned, &paths, &config).is_none());

    spans.retain(|span| span.bbox.x < 40.);
    assert!(prepare(&spans, &paths, &config).is_none());
    assert!(prepare(&spans, &[], &config).is_none());
}

#[test]
fn duplicate_paint_stops_at_the_work_limit_and_does_not_filter_paths() {
    let (spans, paths) = painted_rows(true);
    let duplicates = vec![paths[0].clone(); MAX_PATHS];
    let mut work = Work::new();
    assert!(prepare_with_work(&spans, &duplicates, &TableDetectionConfig::default(), &mut work).is_none());
    assert_eq!(work.0, 0);

    let oversized = vec![paths[0].clone(); MAX_PATHS + 1];
    let mut work = Work::new();
    assert!(prepare_with_work(&spans, &oversized, &TableDetectionConfig::default(), &mut work).is_none());
    assert_eq!(work.0, MAX_WORK);
}

#[test]
fn candidate_must_keep_the_original_occurrence_and_visible_cell_value() {
    let (spans, paths) = painted_rows(true);
    let config = TableDetectionConfig::strict();
    let mut prepared = prepare(&spans, &paths, &config).expect("certified source tiling");
    let mut recovery = super::super::RecoveredIntersection::default();
    let mut tables = super::super::detect_with_unmodified_paths(&spans, &prepared.paths, &config, Some(&mut recovery));
    assert!(prepared.accepts(&tables));

    let original = tables[0].rows[0].cells[2].spans[1].clone();
    let other = tables[0].rows[1].cells[2].spans[1].clone();
    assert_eq!(original.text, other.text);
    assert_ne!(original.bbox, other.bbox);
    tables[0].rows[0].cells[2].spans[1] = other;
    assert!(!prepared.accepts(&tables));

    tables[0].rows[0].cells[2].spans[1] = original;
    tables[0].rows[0].cells[2].text = "Record 0 source".into();
    assert!(!prepared.accepts(&tables));
}

#[test]
fn a_multiword_date_stays_in_its_painted_cell() {
    let (mut spans, paths) = painted_rows(true);
    spans[1].text = "April 12 2020".into();
    let tables = super::super::detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::strict());
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].rows.len(), 4);
    assert_eq!(tables[0].rows[0].cells[2].text, "April 12 2020\ncontinued");
}

#[test]
fn numeric_paint_overhang_requires_the_exact_cell_and_stays_within_its_row_allowance() {
    let paint = Rect::new(15.0, 380.0, 40.0, 21.0);
    let word = TextSpan {
        text: "10.10".into(),
        bbox: Rect::new(17.0, 393.0, 20.0, 9.0),
        ..Default::default()
    };
    let cell = super::super::TableCell {
        text: word.text.clone(),
        bbox: Some(paint),
        spans: vec![word.clone()],
        rowspan: 1,
        colspan: 1,
        mcids: Vec::new(),
        is_header: false,
    };
    assert_eq!(certified_overhang(&cell, paint, &word, 2.0), Some(word.bbox));
    for bounds in [
        Rect::new(17.0, 393.0, 20.0, 8.0),
        Rect::new(14.9, 393.0, 20.0, 9.0),
        Rect::new(17.0, 393.0, 20.0, 10.1),
        Rect::new(17.0, 401.0, 20.0, 1.0),
        Rect::new(17.0, 379.0, 20.0, 9.0),
    ] {
        let changed = TextSpan {
            bbox: bounds,
            ..word.clone()
        };
        assert!(certified_overhang(&cell, paint, &changed, 2.0).is_none());
    }
    assert!(certified_overhang(&cell, Rect::new(15.0, 380.0, 41.0, 21.0), &word, 2.0).is_none());
}

fn alternating_record_bands() -> (Vec<TextSpan>, Vec<PathContent>) {
    use crate::layout::Color;
    record_bands_fixture(
        &[(10., 40.), (50., 8.), (58., 50.), (108., 8.), (116., 40.)],
        [Color::new(0.8, 0.93, 1.), Color::white()],
    )
}

fn record_bands_fixture(
    tracks: &[(f32, f32)],
    [record_fill, spacer_fill]: [crate::layout::Color; 2],
) -> (Vec<TextSpan>, Vec<PathContent>) {
    let mut spans = Vec::new();
    let mut paths = Vec::new();
    let mut top = 170.0;
    for band in 0..5 {
        let is_record = band % 2 == 0;
        let (height, color) = if is_record {
            (24., record_fill)
        } else {
            (10., spacer_fill)
        };
        for &(x, width) in tracks {
            paths.push(painted_cell(x, top - height, width, height, color));
        }
        if is_record {
            for (x, y, text) in [
                (12., top - 8., "Source name"),
                (60., top - 8., "Background"),
                (60., top - 21., "continued"),
                (118., top - 8., "Ownership"),
            ] {
                spans.push(TextSpan {
                    text: text.to_string(),
                    bbox: Rect::new(x, y, 22., 8.5),
                    font_size: 8.5,
                    ..Default::default()
                });
            }
        }
        top -= height;
    }
    (spans, paths)
}

fn assigned_alternating_bands(
    spans: &[TextSpan],
    paths: &[PathContent],
) -> (Vec<super::super::TableRow>, Vec<Vec<Vec<usize>>>) {
    let config = TableDetectionConfig::default();
    let (groups, edges, intersections) = super::super::build_grid_from_lines(paths, &config);
    let [(cells, xs, ys, columns)] = groups.as_slice() else {
        panic!("one exact physical source grid");
    };
    assert!(intersections);
    super::super::assign_spans_to_intersection_grid(cells, xs, ys, *columns, spans, &edges, true).unwrap()
}

#[test]
fn painted_spacers_keep_repeated_source_names_and_wrapped_cells_in_separate_records() {
    let (spans, paths) = alternating_record_bands();
    let (rows, owners) = assigned_alternating_bands(&spans, &paths);
    let table = preserve_record_bands(&rows, &owners, &spans, &paths, &TableDetectionConfig::default()).unwrap();
    assert_eq!(table.col_count, 5);
    assert_eq!(table.rows.len(), 3);
    for (index, row) in table.rows.iter().enumerate() {
        assert_eq!(row.cells[0].text, "Source name");
        assert_eq!(row.cells[2].text, "Background\ncontinued");
        assert_eq!(row.cells[4].text, "Ownership");
        assert_eq!(row.cells[2].spans[0].bbox, spans[index * 4 + 1].bbox);
        assert_eq!(row.cells[2].spans[1].bbox, spans[index * 4 + 2].bbox);
        let bounds = row.cells[2].bbox.unwrap();
        assert!(row.cells[2].spans.iter().all(|span| contains(bounds, span.bbox)));
    }
}

#[test]
fn painted_record_bands_do_not_admit_a_grid_with_too_many_empty_tracks() {
    let (mut spans, paths) = alternating_record_bands();
    assert_eq!(spans.pop().unwrap().text, "Ownership");
    let mut sparse = paths.clone();
    let (bands, remainder) = paths.as_chunks::<5>();
    assert!(remainder.is_empty());
    for band in bands {
        let last = &band[4];
        for offset in [0.0, 10.0] {
            sparse.push(painted_cell(
                last.bbox.right() + offset,
                last.bbox.y,
                10.0,
                last.bbox.height,
                last.fill_color.unwrap(),
            ));
        }
    }
    let (rows, owners) = assigned_alternating_bands(&spans, &sparse);
    assert!(rows.iter().all(|row| row.cells.len() == 7));
    assert!(preserve_record_bands(&rows, &owners, &spans, &sparse, &TableDetectionConfig::default()).is_none());
}

#[test]
fn independent_rule_or_text_in_the_separator_preserves_existing_section_detection() {
    let (spans, paths) = alternating_record_bands();
    let (rows, owners) = assigned_alternating_bands(&spans, &paths);
    let accepts = |words: &[TextSpan], paint: &[PathContent]| {
        preserve_record_bands(&rows, &owners, words, paint, &TableDetectionConfig::default()).is_some()
    };
    assert!(accepts(&spans, &paths));

    let mut divider = paths.clone();
    divider.push(PathContent::line(10., 142., 156., 142.));
    assert!(!accepts(&spans, &divider));

    let mut filled_divider = paths.clone();
    filled_divider.push(painted_cell(10., 142., 146., 0.5, crate::layout::Color::black()));
    assert!(!accepts(&spans, &filled_divider));

    let mut caption = spans.clone();
    caption.push(TextSpan {
        text: "Separate table".into(),
        bbox: Rect::new(12., 140., 35., 4.),
        ..Default::default()
    });
    assert!(!accepts(&caption, &paths));

    let mut straddling = spans.clone();
    straddling[0].bbox.width = 42.;
    assert!(!accepts(&straddling, &paths));
}

#[test]
fn an_outer_glyph_overhang_cannot_capture_unowned_heading_text() {
    let (mut spans, paths) = alternating_record_bands();
    let (rows, owners) = assigned_alternating_bands(&spans, &paths);
    let config = TableDetectionConfig::default();
    assert!(preserve_record_bands(&rows, &owners, &spans, &paths, &config).is_some());

    spans.push(TextSpan {
        text: "Independent heading".into(),
        bbox: Rect::new(12., 170.25, 35., 8.),
        font_size: 8.,
        ..Default::default()
    });
    assert!(preserve_record_bands(&rows, &owners, &spans, &paths, &config).is_none());
}

#[test]
fn adjacent_painted_columns_keep_wrapped_text_inside_its_record() {
    use crate::layout::Color;
    let (spans, paths) = record_bands_fixture(
        &[(10., 48.), (58., 58.), (116., 40.)],
        [Color::new(0.8, 0.93, 1.), Color::white()],
    );
    let tables = super::super::detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::default());
    assert_eq!(tables.len(), 1);
    assert_record_cells(&tables[0], &spans, 3);
}

#[test]
fn contrasting_painted_spacers_keep_wrapped_text_inside_its_record() {
    use crate::layout::Color;
    let tracks = [(10., 40.), (50., 8.), (58., 50.), (108., 8.), (116., 40.)];
    let spacer = Color::new(0.8, 0.93, 1.);
    for record in [Color::white(), Color::new(1., 0.91, 0.75)] {
        let (spans, paths) = record_bands_fixture(&tracks, [record, spacer]);
        let tables = super::super::detect_tables_with_lines(&spans, &paths, &TableDetectionConfig::default());
        assert_eq!(tables.len(), 1);
        assert_record_cells(&tables[0], &spans, 5);
    }
}

fn assert_record_cells(table: &Table, spans: &[TextSpan], columns: usize) {
    assert_eq!(table.col_count, columns);
    assert_eq!(table.rows.len(), 3);
    for (index, row) in table.rows.iter().enumerate() {
        let cell = &row.cells[columns / 2];
        assert_eq!(cell.text, "Background\ncontinued");
        assert_eq!(cell.spans.len(), 2);
        assert_eq!(cell.spans[0].bbox, spans[index * 4 + 1].bbox);
        assert_eq!(cell.spans[1].bbox, spans[index * 4 + 2].bbox);
        assert!(cell.spans.iter().all(|span| contains(cell.bbox.unwrap(), span.bbox)));
    }
}
