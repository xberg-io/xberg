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
    paths_with_resource_objects(content, &format!("/ExtGState << {states} >>"), &[])
}

fn paths_with_resource_objects(content: &[u8], resources: &str, extra_objects: &[&str]) -> Vec<PathContent> {
    document_with_resource_objects(content, resources, extra_objects)
        .extract_paths(0)
        .expect("extract paths")
}

fn document_with_resource_objects(content: &[u8], resources: &str, extra_objects: &[&str]) -> PdfDocument {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << {resources} >> >>"),
    ];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    for object in extra_objects {
        let id = offsets.len();
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{id} 0 obj\n{object}\nendobj\n").as_bytes());
    }
    PdfDocument::from_bytes(common::finalize_pdf(pdf, &offsets)).expect("parse fixture")
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

#[test]
fn an_extgstate_empty_dash_array_overrides_an_earlier_dash() {
    let paths = paths_with_extgstate(
        b"/Dash gs 0 700 m 300 700 l S /Solid gs 0 690 m 300 690 l S",
        "/Dash << /D [[3 2] 1] >> /Solid << /D [[] 0] >>",
    );
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
    assert_eq!(paths[1].dash_pattern, None);
    assert_eq!(paths[1].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
}

#[test]
fn a_form_solid_opaque_override_preserves_the_page_state_and_restore() {
    let paths = paths_with_form_resources(
        b"q /GS gs /X1 Do /GS gs 0 700 m 300 700 l S Q 0 690 m 300 690 l S",
        b"/GS gs 0 50 m 100 50 l S",
        "[1 0 0 1 0 0]",
        "/ExtGState << /GS << /D [[3 2] 0] /CA 0 >> >>",
        "/Resources << /ExtGState << /GS << /D [[] 0] /CA 1 >> >> >>",
    );
    assert_eq!(paths.len(), 3);
    assert_eq!(paths[0].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[0].dash_pattern, None);
    assert_eq!(paths[1].stroke_color, None);
    assert_eq!(paths[2].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[2].dash_pattern, None);
}

#[test]
fn a_form_with_own_resources_without_extgstate_does_not_use_page_states() {
    let paths = paths_with_form_resources(
        b"/X1 Do 0 700 m 300 700 l S",
        b"/GS gs 0 50 m 100 50 l S",
        "[1 0 0 1 0 0]",
        "/ExtGState << /GS << /D [[3 2] 0] /CA 0 >> >>",
        "/Resources << >>",
    );
    assert_eq!(paths.len(), 2);
    for path in paths {
        assert_eq!(path.stroke_color, Some(xberg_native_pdf::layout::Color::black()));
        assert_eq!(path.dash_pattern, None);
    }
}

#[test]
fn indirect_extgstate_state_and_dash_objects_preserve_the_pattern() {
    for (resources, object) in [
        ("/ExtGState 5 0 R", "<< /GS << /D [[3 2] 1] >> >>"),
        ("/ExtGState << /GS 5 0 R >>", "<< /D [[3 2] 1] >>"),
        ("/ExtGState << /GS << /D 5 0 R >> >>", "[[3 2] 1]"),
    ] {
        let paths = paths_with_resource_objects(b"/GS gs 0 700 m 300 700 l S", resources, &[object]);
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].dash_pattern, Some((vec![3.0, 2.0], 1.0)), "{resources}");
    }
}

#[test]
fn negative_stroke_opacity_clamps_to_zero() {
    let paths = paths_with_extgstate(b"/GS gs 0 700 m 300 700 l S", "/GS << /CA -1 >>");
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].stroke_color, None);
}

#[test]
fn above_one_stroke_opacity_clamps_to_one_after_a_clear_state() {
    let paths = paths_with_extgstate(
        b"/Clear gs /High gs 0 700 m 300 700 l S",
        "/Clear << /CA 0 >> /High << /CA 2 >>",
    );
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
}

#[path = "test_path_dash_pattern/allocations.rs"]
mod allocations;

#[path = "test_path_dash_pattern/object_loads.rs"]
mod object_loads;

#[path = "test_path_dash_pattern/retries.rs"]
mod retries;

#[test]
fn finite_negative_ca_outside_f32_range_clamps_to_zero() {
    let paths = paths_with_extgstate(
        b"/GS gs 0 700 m 300 700 l S",
        "/GS << /CA -9999999999999999999999999999999999999999.0 >>",
    );
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].stroke_color, None);
}

#[test]
fn finite_positive_ca_outside_f32_range_clamps_to_one() {
    let paths = paths_with_extgstate(
        b"/Clear gs /GS gs 0 700 m 300 700 l S",
        "/Clear << /CA 0 >> /GS << /CA 9999999999999999999999999999999999999999.0 >>",
    );
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
}

#[test]
fn repeated_dash_only_state_preserves_the_current_inherited_alpha() {
    let paths = paths_with_extgstate(
        b"/Dash gs /Clear gs /Dash gs 0 700 m 300 700 l S /Visible gs /Dash gs 0 690 m 300 690 l S",
        "/Dash << /D [[3 2] 1] >> /Clear << /CA 0 >> /Visible << /CA 1 >>",
    );
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].stroke_color, None);
    assert_eq!(paths[1].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[1].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
}

#[test]
fn repeated_alpha_only_state_preserves_the_current_inherited_dash() {
    let paths = paths_with_extgstate(
        b"/Visible gs [7 1] 2 d /Visible gs 0 700 m 300 700 l S [] 0 d /Visible gs 0 690 m 300 690 l S",
        "/Visible << /CA 1 >>",
    );
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].dash_pattern, Some((vec![7.0, 1.0], 2.0)));
    assert_eq!(paths[1].dash_pattern, None);
    for path in paths {
        assert_eq!(path.stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    }
}

#[test]
fn missing_page_state_does_not_shadow_form_override_or_leak_back_after_restore() {
    let paths = paths_with_form_resources(
        b"/GS gs /X1 Do /GS gs 0 700 m 300 700 l S",
        b"/GS gs 0 50 m 100 50 l S",
        "[1 0 0 1 0 0]",
        "",
        "/Resources << /ExtGState << /GS << /CA 0 >> >> >>",
    );
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].stroke_color, None);
    assert_eq!(paths[1].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[1].dash_pattern, None);
}

#[test]
fn warmed_page_state_is_not_reused_inside_form_resources_without_extgstate() {
    let paths = paths_with_form_resources(
        b"/GS gs q [] 0 d /Visible gs /X1 Do Q /Visible gs 0 700 m 300 700 l S",
        b"/GS gs 0 50 m 100 50 l S",
        "[1 0 0 1 0 0]",
        "/ExtGState << /GS << /D [[3 2] 1] /CA 0 >> /Visible << /CA 1 >> >>",
        "/Resources << >>",
    );
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[0].dash_pattern, None);
    assert_eq!(paths[1].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[1].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
}

#[test]
fn page_extgstate_cache_survives_a_form_that_redefines_the_same_name() {
    let paths = paths_with_form_resources(
        b"/G1 gs q /X1 Do Q /Reset gs /G1 gs 0 700 m 300 700 l S",
        b"/G1 gs 0 50 m 100 50 l S",
        "[1 0 0 1 0 0]",
        "/ExtGState << /G1 << /D [[3 2] 1] /CA 1 >> /Reset << /D [[] 0] /CA 1 >> >>",
        "/Resources << /ExtGState << /G1 << /D [[7 1] 0] /CA 0 >> >> >>",
    );
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].stroke_color, None);
    assert_eq!(paths[1].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[1].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
}

#[test]
fn a_form_uses_its_own_indirect_extgstate_for_a_shared_page_name() {
    let form = "/Shared gs 0 50 m 100 50 l S";
    let form_object = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] \
         /Resources << /ExtGState 7 0 R >> /Length {} >>\nstream\n{form}\nendstream",
        form.len()
    );
    let paths = paths_with_resource_objects(
        b"/Shared gs 0 700 m 300 700 l S /F Do 0 690 m 300 690 l S",
        "/ExtGState 5 0 R /XObject << /F 6 0 R >>",
        &[
            "<< /Shared << /D [[3 2] 1] /CA 1 >> >>",
            &form_object,
            "<< /Shared << /D [[7 1] 2] /CA 0 >> >>",
        ],
    );
    assert_eq!(paths.len(), 3, "page, Form, restored page");
    assert_eq!(paths[0].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[0].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
    assert_eq!(paths[1].stroke_color, None, "the Form's shared name hides its stroke");
    assert_eq!(paths[1].dash_pattern, None);
    assert_eq!(paths[2].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[2].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
}

#[test]
fn a_name_first_used_after_a_form_reads_the_restored_indirect_page_dictionary() {
    let form = "/Shared gs 0 50 m 100 50 l S";
    let form_object = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 612 792] \
         /Resources << /ExtGState 7 0 R >> /Length {} >>\nstream\n{form}\nendstream",
        form.len()
    );
    let document = document_with_resource_objects(
        b"/Shared gs 0 700 m 300 700 l S /F Do /After gs 0 690 m 300 690 l S",
        "/ExtGState 5 0 R /XObject << /F 6 0 R >>",
        &[
            "<< /Shared << /D [[3 2] 1] /CA 1 >> /After << /D [[11 5] 4] /CA 1 >> >>",
            &form_object,
            "<< /Shared << /D [[7 1] 2] /CA 0 >> /After << /D [[9 4] 3] /CA 0 >> >>",
        ],
    );
    let (paths, page_container_loads) = object_loads::count(5, || document.extract_paths(0).unwrap());
    assert_eq!(paths.len(), 3, "page, Form, first page lookup of /After");
    assert_eq!(paths[0].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[0].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
    assert_eq!(paths[1].stroke_color, None);
    assert_eq!(paths[1].dash_pattern, None);
    assert_eq!(paths[2].stroke_color, Some(xberg_native_pdf::layout::Color::black()));
    assert_eq!(paths[2].dash_pattern, Some((vec![11.0, 5.0], 4.0)));
    assert_eq!(
        page_container_loads, 1,
        "restore must retain the page container, not reload it"
    );
}

#[test]
fn repeated_extgstate_dash_uses_the_current_matrix_not_the_lookup_matrix() {
    let paths = paths_with_extgstate(
        b"/GS gs q 2 0 0 2 0 0 cm /GS gs 0 350 m 150 350 l S Q /GS gs 0 690 m 300 690 l S",
        "/GS << /D [[3 2] 1] >>",
    );
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].dash_pattern, Some((vec![6.0, 4.0], 2.0)));
    assert_eq!(paths[1].dash_pattern, Some((vec![3.0, 2.0], 1.0)));
}
