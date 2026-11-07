//! `d` sets the dash pattern of later strokes; `PathContent::dash_pattern` must carry it so
//! consumers can tell a dashed rule from a solid one.

mod common;
use common::build_minimal_pdf_raw;

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::elements::PathContent;

fn paths_of(content: &[u8]) -> Vec<PathContent> {
    let pdf = build_minimal_pdf_raw(content, b"/Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]");
    PdfDocument::from_bytes(pdf)
        .expect("parse fixture")
        .extract_paths(0)
        .expect("extract paths")
}

fn rule(y: u32) -> String {
    format!("0 {y} m 300 {y} l S\n")
}

#[test]
fn a_stroke_after_a_dash_operator_carries_the_pattern() {
    let paths = paths_of(format!("[3 2] 1 d {}", rule(700)).as_bytes());
    let [path] = <[_; 1]>::try_from(paths).expect("one path");
    assert_eq!(path.dash_pattern, Some((vec![3.0, 2.0], 1.0)));
}

#[test]
fn a_solid_stroke_has_no_pattern() {
    let paths = paths_of(rule(700).as_bytes());
    assert_eq!(paths[0].dash_pattern, None);
}

#[test]
fn an_empty_dash_array_restores_a_solid_line() {
    let paths = paths_of(format!("[3 2] 0 d {}[] 0 d {}", rule(700), rule(690)).as_bytes());
    assert_eq!(paths[0].dash_pattern, Some((vec![3.0, 2.0], 0.0)));
    assert_eq!(paths[1].dash_pattern, None);
}

#[test]
fn restoring_the_graphics_state_restores_the_dash() {
    let paths = paths_of(format!("q [3 2] 0 d {}Q {}", rule(700), rule(690)).as_bytes());
    assert!(paths[0].dash_pattern.is_some());
    assert_eq!(paths[1].dash_pattern, None);
}

#[test]
fn dash_lengths_scale_with_the_current_matrix() {
    let paths = paths_of(format!("q 2 0 0 2 0 0 cm [3 2] 0 d {}Q", rule(350)).as_bytes());
    assert_eq!(paths[0].dash_pattern, Some((vec![6.0, 4.0], 0.0)));
}

#[test]
fn a_fill_only_path_has_no_pattern() {
    let paths = paths_of(b"[3 2] 0 d 10 10 50 50 re f\n");
    assert_eq!(paths[0].dash_pattern, None);
}
