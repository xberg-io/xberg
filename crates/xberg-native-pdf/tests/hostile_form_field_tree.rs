//! AcroForm field-tree walks must terminate on cyclic or absurdly deep /Kids
//! chains, survive an unreadable field, and honour inheritable attributes
//! (ISO 32000-1 §12.7.3.1). Walks run on a small thread stack so unbounded
//! recursion aborts the test binary instead of passing unnoticed.

mod common;

use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::extractors::{FieldType, FieldValue, FormExtractor};

fn build_pdf(fields: &str, objects: &[(u32, String)]) -> Vec<u8> {
    let mut all: Vec<(u32, String)> = vec![
        (
            1,
            format!("<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [{fields}] >> >>"),
        ),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string()),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>".to_string()),
    ];
    all.extend(objects.iter().cloned());
    common::build_pdf_from_objects(&all)
}

fn extract_on_small_stack(pdf: Vec<u8>) -> Vec<xberg_native_pdf::extractors::FormField> {
    common::on_small_stack(move || {
        let doc = PdfDocument::from_bytes(pdf).expect("parse");
        FormExtractor::extract_fields(&doc).expect("fields")
    })
}

#[test]
fn cyclic_kids_terminate() {
    let pdf = build_pdf(
        "4 0 R",
        &[
            (4, "<< /T (A) /FT /Tx /Kids [5 0 R] >>".to_string()),
            (5, "<< /T (B) /FT /Tx /Parent 4 0 R /Kids [4 0 R] >>".to_string()),
        ],
    );
    let fields = extract_on_small_stack(pdf);
    let names: Vec<&str> = fields.iter().map(|f| f.full_name.as_str()).collect();
    assert_eq!(names, vec!["A.B", "A"]);
}

#[test]
fn very_deep_kids_chain_is_truncated() {
    const DEPTH: u32 = 5000;
    let objects: Vec<(u32, String)> = (4..4 + DEPTH)
        .map(|id| {
            let kids = if id + 1 < 4 + DEPTH {
                format!("/Kids [{} 0 R]", id + 1)
            } else {
                String::new()
            };
            (id, format!("<< /T (n) /FT /Tx {kids} >>"))
        })
        .collect();
    let fields = extract_on_small_stack(build_pdf("4 0 R", &objects));
    assert!(!fields.is_empty());
    assert!(
        fields.len() <= 66,
        "depth cap must truncate the chain, got {}",
        fields.len()
    );
}

#[test]
fn unreadable_field_does_not_abort_siblings() {
    let pdf = build_pdf(
        "5 0 R 4 0 R",
        &[
            (4, "<< /T (ok) /FT /Tx /V (fine) >>".to_string()),
            (
                5,
                "<< /Length 5 0 R >>
stream
abc
endstream"
                    .to_string(),
            ),
        ],
    );
    let fields = extract_on_small_stack(pdf);
    assert!(fields.iter().any(|f| f.name == "ok"), "sibling field lost: {fields:?}");
}

#[test]
fn inheritable_attributes_resolve_through_parent_and_opt_maps_value() {
    let pdf = build_pdf(
        "4 0 R",
        &[
            (
                4,
                "<< /T (Color) /FT /Ch /Ff 131072 /V (g) /DA (/Helv 0 Tf) /Q 1 /MaxLen 5 \
                 /Opt [[(r) (Red)] [(g) (Green)]] /Kids [5 0 R] >>"
                    .to_string(),
            ),
            (5, "<< /Parent 4 0 R /Rect [0 0 10 10] >>".to_string()),
        ],
    );
    let fields = extract_on_small_stack(pdf);
    let kid = fields
        .iter()
        .find(|f| f.object_ref.is_some_and(|r| r.id == 5))
        .unwrap_or_else(|| panic!("nameless terminal widget dropped: {fields:?}"));
    assert_eq!(kid.field_type, FieldType::Choice);
    assert_eq!(kid.flags, Some(131072));
    assert_eq!(kid.alignment, Some(1));
    assert_eq!(kid.max_length, Some(5));
    assert_eq!(kid.default_appearance.as_deref(), Some("/Helv 0 Tf"));
    assert_eq!(kid.value, FieldValue::Text("Green".to_string()));
}

fn choice_form() -> Vec<u8> {
    build_pdf(
        "4 0 R",
        &[(
            4,
            "<< /T (Color) /FT /Ch /V (g) /Opt [[(r) (Red)] [(g) (Green)] [(g) (Second)]] >>".to_string(),
        )],
    )
}

#[test]
fn fdf_and_xfdf_export_choice_export_values_not_display_text() {
    let pdf = choice_form();
    let dir = tempfile::tempdir().expect("tempdir");
    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    assert_eq!(
        FormExtractor::extract_fields(&doc).expect("fields")[0].value,
        FieldValue::Text("Green".to_string())
    );
    let fdf = dir.path().join("out.fdf");
    let xfdf = dir.path().join("out.xfdf");
    FormExtractor::export_fdf(&doc, &fdf).expect("fdf");
    FormExtractor::export_xfdf(&doc, &xfdf).expect("xfdf");
    let fdf = String::from_utf8_lossy(&std::fs::read(fdf).unwrap()).into_owned();
    let xfdf = std::fs::read_to_string(xfdf).unwrap();
    assert!(fdf.contains("(g)") && !fdf.contains("Green"), "fdf: {fdf}");
    assert!(xfdf.contains(">g<") && !xfdf.contains("Green"), "xfdf: {xfdf}");
}

#[test]
fn many_selected_values_map_through_a_large_option_list() {
    const OPTIONS: usize = 20_000;
    let opts: String = (0..OPTIONS).map(|i| format!("[(e{i}) (d{i})]")).collect();
    let selected: String = (0..OPTIONS).map(|i| format!("(e{i})")).collect();
    let pdf = build_pdf(
        "4 0 R",
        &[(
            4,
            format!("<< /T (Many) /FT /Ch /Ff 2097152 /V [{selected}] /Opt [{opts}] >>"),
        )],
    );
    let fields = common::on_small_stack(move || {
        let doc = PdfDocument::from_bytes(pdf).expect("parse");
        let started = std::time::Instant::now();
        let fields = FormExtractor::extract_fields(&doc).expect("fields");
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "option mapping is quadratic"
        );
        fields
    });
    let FieldValue::Array(values) = &fields[0].value else {
        panic!("expected array value, got {:?}", fields[0].value);
    };
    assert_eq!(values.len(), OPTIONS);
    assert_eq!(values[7], "d7");
}
