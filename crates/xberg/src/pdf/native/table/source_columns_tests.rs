use super::*;

const PAGE_HEIGHT: f32 = 792.0;

fn word(text: &str, left: u32, top: u32, width: u32) -> HocrWord {
    HocrWord {
        text: text.into(),
        left,
        top,
        width,
        height: 8,
        confidence: 100.0,
    }
}

fn numeric_rows() -> Vec<HocrWord> {
    [
        ("First", "8,604", "8,545"),
        ("Second", "9,264", "9,180"),
        ("Third", "9,634", "9,553"),
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, (label, left, right))| {
        let top = 100 + index as u32 * 20;
        [
            word(label, 10, top, 20),
            word(left, 100, top, 20),
            word(right, 138, top, 20),
        ]
    })
    .collect()
}

fn vertical(x: f32, bottom: f32, top: f32) -> PathContent {
    PathContent::from_operations(vec![PathOperation::MoveTo(x, bottom), PathOperation::LineTo(x, top)])
}

fn row_rules() -> Vec<PathContent> {
    [100, 120, 140]
        .into_iter()
        .map(|top| vertical(125.0, PAGE_HEIGHT - top as f32 - 8.0, PAGE_HEIGHT - top as f32))
        .collect()
}

fn limit(words: &[HocrWord], paths: &[PathContent]) -> Option<u32> {
    column_gap_limit_with_work(
        words,
        PAGE_HEIGHT.into(),
        &vertical_rules(paths),
        &mut RuleWork::default(),
    )
}

#[test]
fn should_preserve_recurring_numeric_cells_separated_by_painted_rules() {
    let words = numeric_rows();
    let cap = limit(&words, &row_rules()).expect("independent rows corroborate the same painted boundary");
    assert_eq!(cap, 37);
    let (grid, positions) = crate::table_core::reconstruct_table_with_columns(&words, 51.min(cap), 0.5);
    assert_eq!(positions, [10, 100, 138]);
    assert_eq!(
        grid,
        [
            ["First", "8,604", "8,545"],
            ["Second", "9,264", "9,180"],
            ["Third", "9,634", "9,553"],
        ]
    );
}

#[test]
fn should_preserve_a_wide_numeric_date_cell_without_internal_rules() {
    let words = vec![
        word("Period", 10, 0, 20),
        word("Date", 100, 0, 20),
        word("First", 10, 20, 20),
        word("12", 100, 20, 20),
        word("2020", 138, 20, 20),
        word("Second", 10, 40, 20),
        word("11", 100, 40, 20),
        word("2021", 138, 40, 20),
    ];
    assert_eq!(limit(&words, &[]), None);
    assert_eq!(
        crate::table_core::reconstruct_table_with_columns(&words, 51, 0.5).0,
        [["Period", "Date"], ["First", "12 2020"], ["Second", "11 2021"],]
    );
}

#[test]
fn should_leave_unruled_dates_amounts_thousands_and_units_unchanged() {
    for texts in [
        vec!["12", "03", "2020"],
        vec!["1", "234"],
        vec!["-", "12", ".", "5"],
        vec!["$", "100", "USD"],
        vec!["12", "%"],
        vec!["100", "kg"],
    ] {
        let words: Vec<_> = [100, 120, 140]
            .into_iter()
            .flat_map(|top| {
                texts
                    .iter()
                    .enumerate()
                    .map(move |(index, text)| word(text, 100 + index as u32 * 11, top, 8))
            })
            .collect();
        assert_eq!(limit(&words, &[]), None, "single-cell source {texts:?}");
        let grid = crate::table_core::reconstruct_table_with_columns(&words, 51, 0.5).0;
        assert_eq!(grid, vec![vec![texts.join(" ")]; 3]);
    }
}

#[test]
fn should_not_treat_one_row_as_independent_recurring_boundary_support() {
    assert_eq!(limit(&numeric_rows()[..3], &row_rules()), None);
}

#[test]
fn should_ignore_paintless_or_non_finite_strokes() {
    for invalid in [None, Some(0.0), Some(f32::INFINITY), Some(f32::NAN)] {
        let mut paths = row_rules();
        for path in &mut paths {
            match invalid {
                None => path.stroke_color = None,
                Some(width) => path.stroke_width = width,
            }
        }
        assert_eq!(limit(&numeric_rows(), &paths), None);
    }
}

#[test]
fn should_ignore_unresolved_matrix_or_optional_layer_paths() {
    for matrix in [false, true] {
        let mut paths = row_rules();
        for path in &mut paths {
            if matrix {
                path.matrix = Some([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
            } else {
                path.layer = Some("hidden grid".into());
            }
        }
        assert_eq!(limit(&numeric_rows(), &paths), None);
    }
}

#[test]
fn should_ignore_diagonal_thick_and_non_finite_rule_geometry() {
    for kind in 0..3 {
        let mut paths = row_rules();
        for path in &mut paths {
            match kind {
                0 => path.operations[1] = PathOperation::LineTo(126.0, 692.0),
                1 => path.stroke_width = 4.0,
                _ => path.bbox.x = f32::INFINITY,
            }
        }
        assert_eq!(limit(&numeric_rows(), &paths), None);
    }
}

#[test]
fn should_ignore_figure_lines_outside_or_crossing_numeric_tokens() {
    for paths in [vec![vertical(125.0, 0.0, 100.0)], vec![vertical(110.0, 600.0, 700.0)]] {
        assert_eq!(limit(&numeric_rows(), &paths), None);
    }
}

#[test]
fn should_require_rules_to_cover_the_whole_source_row() {
    let paths: Vec<_> = row_rules()
        .into_iter()
        .map(|mut path| {
            if let PathOperation::LineTo(_, top) = &mut path.operations[1] {
                *top -= 4.0;
            }
            path
        })
        .collect();
    assert_eq!(limit(&numeric_rows(), &paths), None);
}

#[test]
fn should_not_merge_numeric_evidence_from_wrapped_rows_or_prose() {
    let mut wrapped = numeric_rows();
    for word in wrapped.iter_mut().skip(2).step_by(3) {
        word.top += 1;
    }
    assert_eq!(limit(&wrapped, &row_rules()), None);
    let prose: Vec<_> = [100, 120, 140]
        .into_iter()
        .flat_map(|top| {
            [
                word("In", 10, top, 8),
                word("2020", 100, top, 20),
                word("we", 125, top, 8),
                word("counted", 138, top, 20),
            ]
        })
        .collect();
    assert_eq!(limit(&prose, &row_rules()), None);
}

#[test]
fn should_preserve_exact_ordered_words_when_only_cell_boundaries_change() {
    let original = vec![vec!["Total -100 100".into(), "12% ................".into()]];
    let corrected = vec![vec![
        "Total".into(),
        "-100".into(),
        "100".into(),
        "12%".into(),
        "................".into(),
    ]];
    assert!(same_cell_words(&original, &corrected));
    for changed in [
        "Total 100 100 12% ................",
        "Total -100 12% ................",
        "Total -100 100 12%",
        "Total 100 -100 12% ................",
    ] {
        assert!(
            !same_cell_words(&original, &[vec![changed.into()]]),
            "changed source inventory: {changed}"
        );
    }
}

#[test]
fn should_reject_a_stroke_that_overpaints_a_numeric_token() {
    let paths = vec![vertical(120.0, 600.0, 700.0)];
    assert_eq!(limit(&numeric_rows(), &paths), None);
}

#[test]
fn should_reject_repartitioned_rows_even_when_all_table_words_remain_ordered() {
    let original = vec![vec!["Total -100".into()], vec!["Other 12%".into()]];
    let merged = vec![vec!["Total -100 Other 12%".into()]];
    let shifted = vec![vec!["Total".into()], vec!["-100 Other 12%".into()]];
    assert!(!same_cell_words(&original, &merged), "source rows must not merge");
    assert!(
        !same_cell_words(&original, &shifted),
        "source words must not move between existing rows"
    );
}

#[test]
fn should_not_split_an_unruled_date_beside_a_ruled_numeric_pair() {
    let mut words = numeric_rows();
    for top in [100, 120, 140] {
        words.extend([word("12", 300, top, 20), word("2020", 338, top, 20)]);
    }
    let original = super::super::reconstruct_region_table(&words, PAGE_HEIGHT, 1, false, 0)
        .expect("mixed source is already admitted before column correction");
    assert!(original.cells.iter().all(|row| row.last().unwrap() == "12 2020"));
    let corrected =
        reconstruct_region_table(&words, PAGE_HEIGHT, 1, false, 0, &PageRules::new(&row_rules(), true)).unwrap();
    assert_eq!(
        corrected.cells, original.cells,
        "an unsupported date split must reject the entire candidate"
    );
}

#[test]
fn should_bound_duplicate_path_proof_work_without_losing_supported_evidence() {
    let paths: Vec<_> = (0..2_000).map(|_| vertical(125.0, -5_000.0, 800.0)).collect();
    let words: Vec<_> = (0..250)
        .flat_map(|index| {
            let top = 100 + index * 20;
            [
                word("Row", 10, top, 20),
                word("8,604", 100, top, 20),
                word("8,545", 138, top, 20),
            ]
        })
        .collect();
    let mut work = RuleWork::default();
    let rules = vertical_rules_with_work(&paths, &mut work).expect("bounded duplicate path input");
    assert_eq!(
        column_gap_limit_with_work(&words, PAGE_HEIGHT.into(), &rules, &mut work),
        Some(37)
    );
    assert!(
        work.visits <= MAX_RULE_PROOF_WORK,
        "duplicate paths caused {} proof visits",
        work.visits
    );
    assert!(work.visits >= paths.len(), "the actual path scan must fire");
    assert_eq!(rules.len(), 1, "identical painted paths have one geometric certificate");
    let expected = paths.len() + words.iter().map(|word| word.text.len() + 1).sum::<usize>() + words.len() - 1 + 250;
    assert_eq!(
        work.visits, expected,
        "count every input byte, adjacent pair and unique rule visit"
    );
}

fn complete_row_rules() -> Vec<PathContent> {
    let mut paths = row_rules();
    paths.extend(
        [100, 120, 140]
            .into_iter()
            .map(|top| vertical(80.0, PAGE_HEIGHT - top as f32 - 8.0, PAGE_HEIGHT - top as f32)),
    );
    paths
}

#[test]
fn should_refine_an_admitted_table_when_each_column_has_its_own_rule() {
    let words = numeric_rows();
    let table = reconstruct_region_table(
        &words,
        PAGE_HEIGHT,
        1,
        false,
        0,
        &PageRules::new(&complete_row_rules(), true),
    )
    .expect("the original admitted table remains available");
    assert_eq!(
        table.cells,
        [
            ["First", "8,604", "8,545"],
            ["Second", "9,264", "9,180"],
            ["Third", "9,634", "9,553"],
        ]
    );
}

#[test]
fn should_stop_collecting_rules_at_the_geometry_or_input_work_limit() {
    let paths: Vec<_> = (0..=MAX_UNIQUE_VERTICAL_RULES)
        .map(|index| vertical(index as f32, 0.0, 10.0))
        .collect();
    let mut work = RuleWork::default();
    assert!(vertical_rules_with_work(&paths, &mut work).is_none());
    assert_eq!(work.visits, MAX_UNIQUE_VERTICAL_RULES + 1);
    let paths: Vec<_> = (0..=MAX_RULE_PROOF_WORK).map(|_| vertical(125.0, 0.0, 10.0)).collect();
    let mut work = RuleWork::default();
    assert!(vertical_rules_with_work(&paths, &mut work).is_none());
    assert_eq!(work.visits, 0, "oversized input must be rejected before its scan");
}

#[test]
fn should_return_no_certificate_when_source_words_or_text_exhaust_the_budget() {
    let rules = vertical_rules(&row_rules());
    let words = vec![word("1", 100, 100, 20); MAX_SOURCE_PROOF_WORDS + 1];
    let mut work = RuleWork::default();
    assert_eq!(
        column_gap_limit_with_work(&words, PAGE_HEIGHT.into(), &rules, &mut work),
        None
    );
    assert_eq!(work.visits, 0, "oversized words must be rejected before sorting");
    let mut words = numeric_rows();
    words[0].text = "x".repeat(MAX_RULE_PROOF_WORK + 1);
    let mut work = RuleWork::default();
    assert_eq!(
        column_gap_limit_with_work(&words, PAGE_HEIGHT.into(), &rules, &mut work),
        None
    );
    assert_eq!(work.visits, MAX_RULE_PROOF_WORK);
}

#[test]
fn should_fail_closed_when_boundary_verification_exhausts_shared_work() {
    let words = numeric_rows();
    let original = crate::table_core::reconstruct_table_with_columns(&words, 51, 0.5).0;
    let corrected = crate::table_core::reconstruct_table_with_columns(&words, 37, 0.5).0;
    let rules = vertical_rules(&complete_row_rules());
    let mut work = RuleWork {
        visits: MAX_RULE_PROOF_WORK,
    };
    assert_eq!(
        source_boundaries::preserves_source_boundaries(
            &original,
            &corrected,
            &words,
            &rules,
            PAGE_HEIGHT.into(),
            &mut work,
        ),
        None
    );
    assert_eq!(work.visits, MAX_RULE_PROOF_WORK);
}

#[test]
fn should_admit_the_original_table_before_collecting_and_caching_rules() {
    let paths = complete_row_rules();
    let page_rules = PageRules::new(&paths, true);
    let words = numeric_rows();
    assert!(reconstruct_region_table(&words[..3], PAGE_HEIGHT, 1, false, 0, &page_rules).is_none());
    assert!(
        page_rules.cached.get().is_none(),
        "rejected regions must not start source-rule collection"
    );
    let first = reconstruct_region_table(&words, PAGE_HEIGHT, 1, false, 0, &page_rules).unwrap();
    assert_eq!(first.cells[0], ["First", "8,604", "8,545"]);
    let collected = page_rules
        .cached
        .get()
        .expect("admitted region collects rules")
        .as_ptr();
    let second = reconstruct_region_table(&words, PAGE_HEIGHT, 1, false, 0, &page_rules).unwrap();
    assert_eq!(first.cells, second.cells);
    assert_eq!(
        page_rules.cached.get().unwrap().as_ptr(),
        collected,
        "same page reuses its rule collection"
    );
}

fn segment(text: &str, rotation: f32, x: f32, y: f32) -> crate::pdf::hierarchy::SegmentData {
    crate::pdf::hierarchy::SegmentData {
        text: text.into(),
        x,
        y,
        width: 20.0,
        height: 8.0,
        font_size: 8.0,
        is_bold: false,
        is_italic: false,
        is_monospace: false,
        baseline_y: y,
        rotation_degrees: rotation,
        assigned_role: None,
    }
}

fn zero_lift_rotated_segments() -> Vec<crate::pdf::hierarchy::SegmentData> {
    [100.0, 120.0]
        .into_iter()
        .flat_map(|top| {
            let page_x = -(PAGE_HEIGHT - top - 8.0);
            [
                segment("8,604", 90.0, page_x, 100.0),
                segment("8,545", 90.0, page_x, 138.0),
            ]
        })
        .collect()
}

fn assert_unmatched_page_frame_cannot_import_rules(segments: &[crate::pdf::hierarchy::SegmentData]) {
    let lifted = crate::pdf::table_reconstruct::page_has_lifted_rotation_frame(segments, PAGE_HEIGHT);
    assert!(!lifted, "the zero-lift path must be exercised");
    let words = crate::pdf::table_reconstruct::segments_to_words(segments, PAGE_HEIGHT);
    let paths = row_rules();
    assert_eq!(
        column_gap_limit_with_work(
            &words,
            PAGE_HEIGHT.into(),
            &vertical_rules(&paths),
            &mut RuleWork::default()
        ),
        Some(37),
        "wrong-frame coordinates would otherwise borrow the unrelated page rules"
    );
    let page_rules = PageRules::new(&paths, supports_page_frame(segments));
    assert!(
        page_rules.rules().is_empty(),
        "page-XY paths must not certify rotated upright-word axes"
    );
}

#[test]
fn should_reject_zero_lift_rotated_word_axes_as_page_rule_authority() {
    let segments = zero_lift_rotated_segments();
    let words = crate::pdf::table_reconstruct::segments_to_words(&segments, PAGE_HEIGHT);
    assert_eq!((words[0].left, words[0].top), (100, 100));
    assert!(
        segments[0].x < 0.0,
        "the physical page origin differs from the upright word frame"
    );
    assert_unmatched_page_frame_cannot_import_rules(&segments);
}

#[test]
fn should_reject_mixed_upright_and_zero_lift_rotated_frames() {
    let mut segments = vec![segment("Header", 0.0, 10.0, 700.0)];
    segments.extend(zero_lift_rotated_segments());
    assert_unmatched_page_frame_cannot_import_rules(&segments);
}

#[test]
fn should_reject_negative_stroke_widths_retained_by_native_path_extraction() {
    let mut extractor = xberg_native_pdf::extractors::paths::PathExtractor::new();
    extractor.set_line_width(-1.0);
    extractor.move_to(125.0, 684.0);
    extractor.line_to(125.0, 692.0);
    extractor.stroke();
    let paths = extractor.finish();
    assert_eq!(paths.len(), 1);
    assert_eq!(
        paths[0].stroke_width, -1.0,
        "native path extraction retains the invalid width"
    );
    assert!(!paths[0].has_stroke(), "existing positive-width admission rejects it");
    assert!(vertical_rules(&paths).is_empty());
}

#[test]
fn should_accept_page_axes_only_when_every_segment_is_unrotated() {
    let mut segments = vec![segment("First", 0.0, 10.0, 700.0), segment("Second", 0.0, 50.0, 700.0)];
    assert!(supports_page_frame(&segments));
    segments[1].rotation_degrees = f32::NAN;
    assert!(!supports_page_frame(&segments));
}

#[test]
fn fractional_rule_coverage_requires_unique_words_on_the_same_exact_source_row() {
    use crate::pdf::table_reconstruct::project_segment_words;

    for (case, right_y, right_x, add_duplicate, expected) in [
        ("same source row", 684.001, 138.0, false, true),
        ("rounded row collision", 684.002, 138.0, false, false),
        ("paint overlaps true right glyph", 684.001, 137.8, false, false),
        ("duplicate projected occurrence", 684.001, 138.0, true, false),
    ] {
        let mut segments = vec![
            segment("8,604", 0.0, 100.0, 684.001),
            segment("8,545", 0.0, right_x, right_y),
        ];
        if add_duplicate {
            segments.push(segments[1].clone());
        }
        let mut sources = Vec::new();
        let words = project_segment_words(&segments, PAGE_HEIGHT, Some(&mut sources));
        assert_eq!(words[0].top, words[1].top, "{case}");
        let rules = vertical_rules(&[vertical(137.4, 684.001, 694.0)]);
        let [rule] = rules.as_slice() else {
            panic!("one painted vertical rule")
        };
        let exact = ExactRows::new(&segments, &words, &sources);
        assert!(
            !separates_pair(rule, &words[0], &words[1], PAGE_HEIGHT.into()),
            "{case}"
        );
        assert_eq!(
            separates_pair(rule, &words[0], &words[1], exact.frame(PAGE_HEIGHT).unwrap()),
            expected,
            "{case}"
        );
    }
}

#[test]
fn only_terminal_unmerged_words_witness_a_source_span() {
    use crate::pdf::table_reconstruct::project_segment_words;

    let segments = [
        segment("12", 0.0, 100.0, 684.001),
        segment("345", 0.0, 120.0, 684.001),
        segment("word pair", 0.0, 160.0, 684.001),
        segment("6,789", 0.0, 200.0, 684.001),
    ];
    let mut sources = Vec::new();
    let words = project_segment_words(&segments, PAGE_HEIGHT, Some(&mut sources));
    assert_eq!(
        words.iter().map(|word| word.text.as_str()).collect::<Vec<_>>(),
        ["12345", "word", "pair", "6,789"]
    );
    assert_eq!(sources, [None, None, Some(2), Some(3)]);
}

#[test]
fn terminal_word_cannot_certify_a_rule_crossed_by_its_source_span() {
    use crate::pdf::table_reconstruct::project_segment_words;

    let mut segments = [
        segment("label 12", 0.0, 100.0, 684.001),
        segment("34", 0.0, 150.0, 684.001),
    ];
    segments[0].width = 37.45;
    let mut sources = Vec::new();
    let words = project_segment_words(&segments, PAGE_HEIGHT, Some(&mut sources));
    assert_eq!(sources, [None, Some(0), Some(1)]);
    let mut path = vertical(137.35, 684.001, 694.0);
    path.stroke_width = 0.2;
    let [rule] = vertical_rules(&[path])
        .try_into()
        .unwrap_or_else(|_| panic!("one painted rule"));
    let terminal = &words[1];
    assert!(terminal.left.saturating_add(terminal.width) as f32 <= rule.x - rule.stroke_width / 2.0);
    assert!(segments[0].x + segments[0].width > rule.x - rule.stroke_width / 2.0);
    let exact = ExactRows::new(&segments, &words, &sources);
    assert!(!separates_pair(
        &rule,
        terminal,
        &words[2],
        exact.frame(PAGE_HEIGHT).unwrap()
    ));
}

#[test]
fn source_rule_rejects_ink_overhang_hidden_by_f32_edge_rounding() {
    use crate::pdf::table_reconstruct::project_segment_words;

    let mut segments = [segment("12", 0.0, 100.0, 684.001), segment("34", 0.0, 150.0, 684.001)];
    segments[0].width = 37.0_f32.next_up();
    let mut sources = Vec::new();
    let words = project_segment_words(&segments, PAGE_HEIGHT, Some(&mut sources));
    let [rule] = vertical_rules(&[vertical(137.5, 684.001, 694.0)])
        .try_into()
        .unwrap_or_else(|_| panic!("one painted rule"));
    let rounded_right = segments[0].x + segments[0].width;
    assert_eq!(rounded_right, rule.x - rule.stroke_width / 2.0);
    assert!(f64::from(segments[0].x) + f64::from(segments[0].width) > f64::from(rounded_right));
    assert!(!separates_pair(&rule, &words[0], &words[1], PAGE_HEIGHT.into()));
    let exact = ExactRows::new(&segments, &words, &sources);
    assert!(!separates_pair(
        &rule,
        &words[0],
        &words[1],
        exact.frame(PAGE_HEIGHT).unwrap()
    ));
}

#[test]
fn exact_fallback_can_split_old_cells_but_cannot_move_their_cuts_or_blanks() {
    let row = |cells: &[&str]| vec![cells.iter().map(|s| (*s).to_string()).collect::<Vec<_>>()];
    let original = row(&["12 34", "", "56"]);
    let refinement = row(&["12", "34", "", "56"]);
    let moved_cut = row(&["12", "34 56", "", ""]);
    let moved_blank = row(&["", "12", "34", "56"]);
    for candidate in [&refinement, &moved_cut, &moved_blank] {
        assert!(same_cell_words(&original, candidate));
    }
    assert!(source_boundaries::preserves_old_partitions(&original, &refinement));
    assert!(!source_boundaries::preserves_old_partitions(&original, &moved_cut));
    assert!(!source_boundaries::preserves_old_partitions(&original, &moved_blank));
}

#[test]
fn coarse_rule_rejects_positive_stroke_width_hidden_by_edge_rounding() {
    let left = word("12", 100, 100, 37);
    let right = word("34", 150, 100, 20);
    let mut path = vertical(137.0, 680.0, 700.0);
    path.stroke_width = 1e-6;
    let [rule] = vertical_rules(&[path])
        .try_into()
        .unwrap_or_else(|_| panic!("one painted rule"));
    assert_eq!(rule.x - rule.stroke_width / 2.0, (left.left + left.width) as f32);
    assert!(rule.stroke_width > 0.0);
    assert!(!separates_pair(&rule, &left, &right, PAGE_HEIGHT.into()));
}
