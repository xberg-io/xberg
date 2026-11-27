//! GH#3423: the hint-free repair orders a rotated run's lines top to bottom in the run's own
//! frame, whatever order the spans arrive in.

use super::*;

const HEIGHT: f32 = 10.0;
const WIDTH: f32 = 30.0;

/// A span of a `rotation`-degree run placed at `advance` along its reading
/// direction and `cross` up its upright frame.
fn at(text: &str, rotation: f32, advance: f32, cross: f32) -> TextSpan {
    let (x, y) = match rotation as i32 {
        90 => (-cross, advance),
        180 => (-advance, -cross),
        270 => (cross, -advance),
        _ => (advance, cross),
    };
    TextSpan {
        text: text.to_string(),
        x,
        y,
        width: WIDTH,
        height: HEIGHT,
        rotation_degrees: rotation,
    }
}

/// Three rows of two cells, top row first.
fn table(rotation: f32) -> Vec<TextSpan> {
    [("a", "b", 100.0), ("c", "d", 76.0), ("e", "f", 52.0)]
        .into_iter()
        .flat_map(|(left, right, cross)| [at(left, rotation, 0.0, cross), at(right, rotation, 60.0, cross)])
        .collect()
}

fn shuffled(spans: &[TextSpan], permutation: &[usize]) -> Vec<TextSpan> {
    permutation.iter().map(|&index| spans[index].clone()).collect()
}

const PERMUTATIONS: [[usize; 6]; 4] = [
    [0, 1, 2, 3, 4, 5],
    [5, 4, 3, 2, 1, 0],
    [1, 3, 5, 0, 2, 4],
    [4, 0, 3, 5, 2, 1],
];

#[test]
fn should_read_a_shuffled_270_degree_table_row_by_row() {
    for permutation in PERMUTATIONS {
        let spans = shuffled(&table(270.0), &permutation);
        assert_eq!(
            repair_rotated_page_text(&spans).as_deref(),
            Some("a b\nc d\ne f"),
            "{permutation:?}"
        );
    }
}

#[test]
fn should_read_a_shuffled_90_degree_table_row_by_row() {
    for permutation in PERMUTATIONS {
        let spans = shuffled(&table(90.0), &permutation);
        assert_eq!(
            repair_rotated_page_text(&spans).as_deref(),
            Some("a b\nc d\ne f"),
            "{permutation:?}"
        );
    }
}

#[test]
fn should_read_a_shuffled_180_degree_table_row_by_row() {
    for permutation in PERMUTATIONS {
        let spans = shuffled(&table(180.0), &permutation);
        assert_eq!(
            repair_rotated_page_text(&spans).as_deref(),
            Some("a b\nc d\ne f"),
            "{permutation:?}"
        );
    }
}

#[test]
fn should_keep_same_line_jitter_on_one_line() {
    for jitter in [0.1, 0.25, 0.4] {
        let spans = vec![
            at("three", 90.0, 120.0, 100.0),
            at("two", 90.0, 60.0, 100.0 + jitter * HEIGHT),
            at("one", 90.0, 0.0, 100.0),
        ];
        assert_eq!(
            repair_rotated_page_text(&spans).as_deref(),
            Some("one two three"),
            "jitter {jitter}"
        );
    }
}

#[test]
fn should_not_let_a_taller_span_join_a_row_its_neighbours_are_too_far_from() {
    let tall = |text: &str, advance: f32, cross: f32| TextSpan {
        height: 2.0 * HEIGHT,
        ..at(text, 90.0, advance, cross)
    };
    let spans = vec![
        at("A", 90.0, 0.0, 100.0),
        at("1", 90.0, 60.0, 100.0),
        at("B", 90.0, 0.0, 90.0),
        tall("2", 60.0, 91.0),
    ];
    assert_eq!(repair_rotated_page_text(&spans).as_deref(), Some("A 1\nB 2"));
}

fn wide(text: &str, advance: f32, cross: f32, width: f32) -> TextSpan {
    TextSpan {
        width,
        ..at(text, 90.0, advance, cross)
    }
}

/// Two columns of prose, `rows` lines each, `gutter` apart, drawn on shared baselines.
fn aligned_columns(gutter: f32, width: f32, rows: usize) -> Vec<TextSpan> {
    (0..rows)
        .flat_map(|row| {
            let cross = 100.0 - 20.0 * row as f32;
            [
                wide(&format!("A{}", row + 1), 0.0, cross, width),
                wide(&format!("B{}", row + 1), width + gutter, cross, width),
            ]
        })
        .collect()
}

#[test]
fn should_keep_columns_apart_when_their_baselines_align() {
    let spans = aligned_columns(30.0, 200.0, 3);
    assert_eq!(
        repair_rotated_page_text(&spans).as_deref(),
        Some("A1\nA2\nA3\nB1\nB2\nB3")
    );
}

#[test]
fn should_read_a_full_width_title_before_the_two_columns_under_it() {
    let mut spans = aligned_columns(30.0, 200.0, 3);
    spans.push(wide("Title", 0.0, 130.0, 430.0));
    spans.reverse();
    assert_eq!(
        repair_rotated_page_text(&spans).as_deref(),
        Some("Title\nA1\nA2\nA3\nB1\nB2\nB3")
    );
}

#[test]
fn should_read_a_full_width_footer_after_the_two_columns_above_it() {
    let mut spans = aligned_columns(30.0, 200.0, 3);
    spans.push(wide("Footer", 0.0, 20.0, 430.0));
    assert_eq!(
        repair_rotated_page_text(&spans).as_deref(),
        Some("A1\nA2\nA3\nB1\nB2\nB3\nFooter")
    );
}

#[test]
fn should_read_a_ragged_table_with_missing_cells_row_by_row() {
    let cell = |text: &str, advance: f32, cross: f32| wide(text, advance, cross, 40.0);
    let spans = vec![
        cell("h", 100.0, 60.0),
        cell("c", 200.0, 100.0),
        cell("d", 0.0, 80.0),
        cell("a", 0.0, 100.0),
        cell("i", 200.0, 60.0),
        cell("b", 100.0, 100.0),
        cell("f", 200.0, 80.0),
    ];
    assert_eq!(repair_rotated_page_text(&spans).as_deref(), Some("a b c\nd f\nh i"));
}

#[test]
fn should_not_cut_at_a_gap_of_exactly_one_and_a_half_body_heights() {
    let spans = aligned_columns(15.0, 200.0, 2);
    assert_eq!(repair_rotated_page_text(&spans).as_deref(), Some("A1 B1\nA2 B2"));
}

#[test]
fn should_cut_at_a_gutter_just_under_half_the_narrower_column() {
    let spans = aligned_columns(29.0, 60.0, 2);
    assert_eq!(repair_rotated_page_text(&spans).as_deref(), Some("A1\nA2\nB1\nB2"));
}

#[test]
fn should_not_cut_at_a_gutter_just_over_half_the_narrower_column() {
    let spans = aligned_columns(31.0, 60.0, 2);
    assert_eq!(repair_rotated_page_text(&spans).as_deref(), Some("A1 B1\nA2 B2"));
}

#[test]
fn should_read_touching_spans_as_one_block() {
    let spans = vec![
        wide("a", 0.0, 100.0, 100.0),
        wide("b", 100.0, 100.0, 100.0),
        wide("c", 0.0, 80.0, 100.0),
        wide("d", 100.0, 80.0, 100.0),
    ];
    assert_eq!(repair_rotated_page_text(&spans).as_deref(), Some("ab\ncd"));
}

#[test]
fn should_keep_a_half_height_superscript_on_its_line() {
    let marker = TextSpan {
        height: HEIGHT / 2.0,
        ..at("2", 90.0, 80.0, 103.3)
    };
    let spans = vec![at("x", 90.0, 0.0, 100.0), marker, at("y", 90.0, 40.0, 100.0)];
    assert_eq!(repair_rotated_page_text(&spans).as_deref(), Some("x y 2"));
}

#[test]
fn should_not_give_a_zero_or_nan_height_span_its_own_line() {
    for height in [0.0, f32::NAN] {
        let odd = TextSpan {
            height,
            ..at("y", 90.0, 40.0, 101.0)
        };
        let spans = vec![at("x", 90.0, 0.0, 100.0), odd, at("z", 90.0, 80.0, 100.0)];
        assert_eq!(repair_rotated_page_text(&spans).as_deref(), Some("x y z"), "{height}");
    }
}

#[test]
fn should_order_a_run_whose_spans_all_lack_a_height() {
    let blank = |text: &str, advance: f32, cross: f32| TextSpan {
        height: 0.0,
        ..at(text, 90.0, advance, cross)
    };
    let spans = vec![
        blank("c", 0.0, 80.0),
        blank("b", 40.0, 100.0),
        blank("d", 40.0, 80.0),
        blank("a", 0.0, 100.0),
    ];
    assert_eq!(repair_rotated_page_text(&spans).as_deref(), Some("a b\nc d"));
}

#[test]
fn should_keep_side_by_side_columns_with_offset_baselines_apart() {
    let spans = vec![
        wide("B1", 230.0, 90.0, 200.0),
        wide("A1", 0.0, 100.0, 200.0),
        wide("B2", 230.0, 70.0, 200.0),
        wide("A2", 0.0, 80.0, 200.0),
        wide("B3", 230.0, 50.0, 200.0),
        wide("A3", 0.0, 60.0, 200.0),
    ];
    assert_eq!(
        repair_rotated_page_text(&spans).as_deref(),
        Some("A1\nA2\nA3\nB1\nB2\nB3")
    );
}

#[test]
fn should_keep_a_layout_hint_order_for_a_rotated_run() {
    let spans = table(90.0);
    // A hint order that reads the bottom row first must survive assembly, with the
    // hint path's space between lines.
    let order = vec![4, 5, 0, 1, 2, 3];
    assert_eq!(assemble_reading_order_text(&spans, &order), "e f a b c d");
}

#[test]
fn should_keep_a_one_row_table_on_one_line() {
    let spans = vec![
        wide("H3", 160.0, 100.0, 60.0),
        wide("H1", 0.0, 100.0, 60.0),
        wide("H2", 80.0, 100.0, 60.0),
    ];
    assert_eq!(repair_rotated_page_text(&spans).as_deref(), Some("H1 H2 H3"));
}

#[test]
fn should_keep_a_one_row_tracked_heading_on_one_line() {
    let spans: Vec<TextSpan> = ["T", "R", "A", "C", "K"]
        .iter()
        .enumerate()
        .map(|(i, word)| wide(word, 70.0 * i as f32, 100.0, 50.0))
        .collect();
    assert_eq!(repair_rotated_page_text(&spans).as_deref(), Some("T R A C K"));
}

#[test]
fn should_read_a_large_heading_above_small_body_lines() {
    let heading = TextSpan {
        height: 30.0,
        ..wide("Heading", 0.0, 150.0, 100.0)
    };
    let spans = vec![
        wide("c", 0.0, 80.0, 40.0),
        wide("b", 60.0, 100.0, 40.0),
        heading,
        wide("d", 60.0, 80.0, 40.0),
        wide("a", 0.0, 100.0, 40.0),
    ];
    assert_eq!(repair_rotated_page_text(&spans).as_deref(), Some("Heading\na b\nc d"));
}

#[test]
fn should_read_a_mid_page_band_between_two_column_regions() {
    let mut spans = aligned_columns(30.0, 200.0, 2);
    spans.push(wide("Band", 0.0, 50.0, 430.0));
    for (row, cross) in [30.0, 10.0].into_iter().enumerate() {
        spans.push(wide(&format!("C{}", row + 1), 0.0, cross, 200.0));
        spans.push(wide(&format!("D{}", row + 1), 230.0, cross, 200.0));
    }
    spans.reverse();
    assert_eq!(
        repair_rotated_page_text(&spans).as_deref(),
        Some("A1\nA2\nB1\nB2\nBand\nC1\nC2\nD1\nD2")
    );
}

#[test]
fn should_read_three_columns_one_after_another() {
    let spans: Vec<TextSpan> = (0..3)
        .flat_map(|row| {
            (0..3).map(move |column| {
                wide(
                    &format!("{}{}", ["A", "B", "C"][column], row + 1),
                    125.0 * column as f32,
                    100.0 - 20.0 * row as f32,
                    100.0,
                )
            })
        })
        .collect();
    assert_eq!(
        repair_rotated_page_text(&spans).as_deref(),
        Some("A1\nA2\nA3\nB1\nB2\nB3\nC1\nC2\nC3")
    );
}

/// 5,000 spans in 500 lines with no band to find must not cost a re-sort per line.
#[test]
fn should_order_a_large_table_without_a_band_quickly() {
    let (lines, cells) = (500, 10);
    let spans: Vec<TextSpan> = (0..lines * cells)
        .map(|n| {
            let shuffled = (n * 7919) % (lines * cells);
            let (row, column) = (shuffled / cells, shuffled % cells);
            wide("x", 50.0 * column as f32, 5000.0 - 20.0 * row as f32, 30.0)
        })
        .collect();
    let started = std::time::Instant::now();
    let text = repair_rotated_page_text(&spans).expect("rotated page is repaired");
    assert_eq!(text.lines().count(), lines);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "{:?}",
        started.elapsed()
    );
}
