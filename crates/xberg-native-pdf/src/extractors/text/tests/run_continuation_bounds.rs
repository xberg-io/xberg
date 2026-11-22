use super::super::*;

/// Each quadrant pins all three questions of the continuation rule: on the line, forward along it,
/// and near enough to the run's end to be the next glyph.
#[test]
fn test_run_continuation_by_quadrant() {
    let m = |a, b, c, d| Matrix {
        a,
        b,
        c,
        d,
        e: 100.0,
        f: 500.0,
    };
    let fs = 10.0;
    let at = |mat: Matrix, de: f32, df: f32| {
        TextExtractor::run_continuation_along(mat, 0, mat.e + de, mat.f + df, fs, 14.0).is_some()
    };

    let upright = m(1.0, 0.0, 0.0, 1.0);
    assert!(at(upright, 14.0, 0.0), "upright advance must continue");
    assert!(!at(upright, 0.0, -14.0), "upright line break must not");
    assert!(!at(upright, -14.0, 0.0), "upright backwards must not");
    assert!(at(upright, 14.0, 4.0), "upright sub-glyph offset must not be vetoed");
    assert!(!at(upright, 14.0, 8.0), "upright line step must be vetoed");

    let cw = m(0.0, 1.0, -1.0, 0.0);
    assert!(at(cw, 0.0, 14.0), "90° along-axis advance must continue");
    assert!(!at(cw, 14.0, 0.0), "90° line break must not");
    assert!(at(cw, -4.0, 14.0), "90° sub-glyph offset must not be vetoed");
    assert!(!at(cw, -8.0, 14.0), "90° line step must be vetoed");

    let ccw = m(0.0, -1.0, 1.0, 0.0);
    assert!(at(ccw, 0.0, -14.0), "270° along-axis advance must continue");
    assert!(!at(ccw, 0.0, 14.0), "270° backwards advance must not");
    assert!(!at(ccw, 14.0, 0.0), "270° line break must not");

    let flip = m(-1.0, 0.0, 0.0, -1.0);
    assert!(at(flip, -14.0, 0.0), "180° along-axis advance must continue");
    assert!(!at(flip, 14.0, 0.0), "180° backwards advance must not");

    let degenerate = m(0.0, 0.0, 0.0, 0.0);
    assert!(at(degenerate, 14.0, 0.0));
    assert!(!at(degenerate, -14.0, 0.0));
}

/// A jump past the run's end by more than `max(3 x font size, 30 pt)` is a new run in every quadrant.
#[test]
fn test_run_continuation_bounds_the_gap_in_every_quadrant() {
    let m = |a, b, c, d| Matrix {
        a,
        b,
        c,
        d,
        e: 100.0,
        f: 500.0,
    };
    let fs = 10.0;
    let at = |mat: Matrix, de: f32, df: f32| {
        TextExtractor::run_continuation_along(mat, 0, mat.e + de, mat.f + df, fs, 14.0).is_some()
    };
    for (name, mat, unit) in [
        ("upright", m(1.0, 0.0, 0.0, 1.0), (1.0f32, 0.0f32)),
        ("90°", m(0.0, 1.0, -1.0, 0.0), (0.0, 1.0)),
        ("270°", m(0.0, -1.0, 1.0, 0.0), (0.0, -1.0)),
        ("180°", m(-1.0, 0.0, 0.0, -1.0), (-1.0, 0.0)),
    ] {
        let step = |d: f32| (unit.0 * d, unit.1 * d);
        let (e44, f44) = step(44.0);
        assert!(
            at(mat, e44, f44),
            "{name}: a gap of exactly the threshold must continue"
        );
        let (e60, f60) = step(60.0);
        assert!(
            !at(mat, e60, f60),
            "{name}: a gap beyond the threshold must end the run"
        );
    }
}

/// Vertical writing mode keeps the raw same-line, forward test.
#[test]
fn test_run_continuation_leaves_vertical_writing_mode_alone() {
    let upright = Matrix {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 100.0,
        f: 500.0,
    };
    let fs = 10.0;
    assert_eq!(
        TextExtractor::run_continuation_along(upright, 1, 140.0, 500.0, fs, 0.0),
        Some(40.0)
    );
    assert!(TextExtractor::run_continuation_along(upright, 1, 60.0, 500.0, fs, 0.0).is_none());
    assert!(TextExtractor::run_continuation_along(upright, 1, 140.0, 460.0, fs, 0.0).is_none());
}

/// The same layout at `Tf 12` with an identity `Tm` and at `Tf 1` with a 12x `Tm` must reach the
/// same decision: the 30 pt floor applies in user space, after the font size is scaled.
#[test]
fn test_run_continuation_limit_is_the_same_for_equivalent_font_and_matrix_scales() {
    let m = |scale: f32| Matrix {
        a: scale,
        b: 0.0,
        c: 0.0,
        d: scale,
        e: 100.0,
        f: 500.0,
    };
    let identity_tf12 = TextExtractor::run_continuation_along(m(1.0), 0, 200.0, 500.0, 12.0, 0.0);
    let scaled_tf1 = TextExtractor::run_continuation_along(m(12.0), 0, 200.0, 500.0, 1.0, 0.0);
    assert!(identity_tf12.is_none(), "a 100 pt gap exceeds the 36 pt limit");
    assert!(scaled_tf1.is_none(), "the same gap must end the run under a scaled Tm");
}
