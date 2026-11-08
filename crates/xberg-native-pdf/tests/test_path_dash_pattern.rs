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

#[test]
fn dashes_that_paint_a_solid_line_leave_no_pattern() {
    for dash in ["[3 0] 0 d", "[0 0] 0 d", "[0] 0 d", "[3 0 3 0] 0 d", "[-3 2] 0 d"] {
        let paths = paths_of(format!("{dash} {}", rule(700)).as_bytes());
        assert_eq!(paths[0].dash_pattern, None, "{dash}");
    }
}

#[test]
fn dashes_with_a_visible_gap_keep_their_pattern() {
    for (dash, expected) in [
        ("[5] 0 d", vec![5.0]),
        ("[3 2 1] 0 d", vec![3.0, 2.0, 1.0]),
        ("[0 3] 0 d", vec![0.0, 3.0]),
    ] {
        let paths = paths_of(format!("{dash} {}", rule(700)).as_bytes());
        assert_eq!(paths[0].dash_pattern, Some((expected, 0.0)), "{dash}");
    }
}

#[test]
fn a_collapsed_matrix_leaves_no_pattern() {
    let paths = paths_of(format!("q 0 0 0 0 0 0 cm [3 2] 0 d {}Q", rule(350)).as_bytes());
    assert_eq!(paths[0].dash_pattern, None);
}

fn paths_with_extgstate(content: &[u8], states: &str) -> Vec<PathContent> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /ExtGState << {states} >> >> >>"
        ),
    ];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    PdfDocument::from_bytes(common::finalize_pdf(pdf, &offsets))
        .expect("parse fixture")
        .extract_paths(0)
        .expect("extract paths")
}

#[test]
fn a_dash_set_through_an_extgstate_carries_the_pattern() {
    let paths = paths_with_extgstate(b"/GS1 gs 0 700 m 300 700 l S", "/GS1 << /D [[3 2] 1] >>");
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
}

#[test]
fn restoring_extgstate_dash_and_opacity_restores_the_visible_stroke() {
    let paths = paths_with_extgstate(
        b"q /GS1 gs 0 700 m 300 700 l S Q 0 690 m 300 690 l S",
        "/GS1 << /D [[3 2] 0] /CA 0 >>",
    );
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].stroke_color, None);
    assert_eq!(paths[1].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[1].dash_pattern, None);
}

#[test]
fn an_opaque_extgstate_can_restore_stroke_paint_without_clearing_dash() {
    let paths = paths_with_extgstate(
        b"/Hidden gs /Visible gs 0 700 m 300 700 l S",
        "/Hidden << /CA 0 /D [[3 2] 0] >> /Visible << /CA 1 >>",
    );
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[0].dash_pattern, Some((vec![3.0, 2.0], 0.0)));
}

#[test]
fn malformed_or_missing_extgstate_values_leave_the_dash_unchanged() {
    for state in ["/Missing", "/Bad"] {
        let content = format!("[3 2] 1 d {state} gs 0 700 m 300 700 l S");
        let paths = paths_with_extgstate(content.as_bytes(), "/Bad << /D [[3 /invalid] 0] /CA /invalid >>");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
        assert_eq!(paths[0].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    }
}

fn paths_with_form(page_content: &[u8], form_content: &[u8], form_matrix: &str) -> Vec<PathContent> {
    paths_with_form_resources(page_content, form_content, form_matrix, "", "")
}

fn paths_with_form_resources(
    page_content: &[u8],
    form_content: &[u8],
    form_matrix: &str,
    page_resources: &str,
    form_resources: &str,
) -> Vec<PathContent> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];
    for object in [
        b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".as_slice(),
        b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n",
        format!("3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /XObject << /X1 5 0 R >> {page_resources} >> >>\nendobj\n").as_bytes(),
    ] {
        offsets.push(pdf.len());
        pdf.extend_from_slice(object);
    }
    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", page_content.len()).as_bytes());
    pdf.extend_from_slice(page_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(
        format!(
            "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] /Matrix {form_matrix} \
             {form_resources} /Length {} >>\nstream\n",
            form_content.len()
        )
        .as_bytes(),
    );
    pdf.extend_from_slice(form_content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    PdfDocument::from_bytes(common::finalize_pdf(pdf, &offsets))
        .expect("parse fixture")
        .extract_paths(0)
        .expect("extract paths")
}

#[test]
fn a_form_inherits_the_page_dash_keeps_its_own_inside_and_scales_by_its_matrix() {
    let form = b"0 50 m 100 50 l S [7 1] 0 d 0 100 m 100 100 l S";
    let paths = paths_with_form(b"[3 2] 0 d /X1 Do 0 700 m 300 700 l S", form, "[2 0 0 2 0 0]");
    let dashes: Vec<_> = paths.iter().map(|path| path.dash_pattern.clone()).collect();
    assert_eq!(
        dashes,
        [
            Some((vec![6.0, 4.0], 0.0)),
            Some((vec![14.0, 2.0], 0.0)),
            Some((vec![3.0, 2.0], 0.0)),
        ]
    );
}

#[test]
fn a_form_without_a_matrix_does_not_rescale_the_dash() {
    let paths = paths_with_form(b"[3 2] 0 d /X1 Do", b"0 50 m 100 50 l S", "[1 0 0 1 0 0]");
    assert_eq!(paths[0].dash_pattern, Some((vec![3.0, 2.0], 0.0)));
}

#[test]
fn a_form_resolves_its_own_extgstate_and_restores_the_page_state() {
    let paths = paths_with_form_resources(
        b"/GS gs /X1 Do 0 700 m 300 700 l S",
        b"/GS gs 0 50 m 100 50 l S",
        "[2 0 0 2 0 0]",
        "/ExtGState << /GS << /D [[3 2] 1] >> >>",
        "/Resources << /ExtGState << /GS << /D [[7 1] 0] >> >> >>",
    );
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].dash_pattern, Some((vec![14.0, 2.0], 0.0)));
    assert_eq!(paths[1].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
}

#[test]
fn a_form_without_resources_inherits_extgstate_and_restores_stroke_opacity() {
    let paths = paths_with_form_resources(
        b"/X1 Do 0 700 m 300 700 l S",
        b"/GS gs 0 50 m 100 50 l S",
        "[1 0 0 1 0 0]",
        "/ExtGState << /GS << /CA 0 >> >>",
        "",
    );
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].stroke_color, None);
    assert_eq!(paths[1].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
}
