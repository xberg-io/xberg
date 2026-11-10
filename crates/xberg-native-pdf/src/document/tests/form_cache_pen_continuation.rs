use super::super::PdfDocument;
use super::pdf_fixtures::build_catalog_test_pdf;

/// A form that writes "The quick brown fox" as two `TJ` arrays in Helvetica, the second placed by a
/// `Td` with no line move 0.01 points before the pen of the first (GH#2664). Helvetica AFM widths at
/// 10 points: "The quick" 43.35, the -300 offset 3.0, "bro" 14.45, so the pen is at 60.80. The page
/// shares the font resource, so the span merge sees a font with no `/Widths` and reads the seam
/// through the pen mark; a font the merge cannot find counts as reliable. ~keep
fn pen_continuation_form_pdf() -> Vec<u8> {
    let paint = "BT /F1 10 Tf 1 0 0 1 20 50 Tm [(The quick) -300 (bro)] TJ 60.79 0 Td [(wn fox)] TJ ET";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 400 120] \
         /Resources << /Font << /F1 7 0 R >> >> /Length {} >>\nstream\n{paint}\nendstream",
        paint.len()
    );
    let content = "/Form Do";
    let contents = format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len());
    build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 120] \
                   /Contents 5 0 R /Resources << /Font << /F1 7 0 R >> /XObject << /Form 6 0 R >> >> >>",
            ),
            (5, contents.as_bytes()),
            (6, form.as_bytes()),
            (
                7,
                b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
            ),
        ],
    )
}

fn runs(document: &PdfDocument) -> String {
    document
        .extract_spans(0)
        .expect("extract the form page")
        .iter()
        .map(|span| span.text.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

#[test]
fn a_replayed_form_keeps_the_pen_continuation_of_its_arrays() {
    let document = PdfDocument::from_bytes(pen_continuation_form_pdf()).expect("open the form page");
    let cold = runs(&document);
    assert!(cold.contains("brown"), "cold form run: {cold}");
    assert_eq!(
        document.xobject_spans_cache.lock().unwrap().len(),
        1,
        "the form is cached"
    );
    // Bypass the completed page spans so this read replays the cached form spans. ~keep
    document.page_spans_cache.lock().unwrap().clear();
    let warm = runs(&document);
    assert!(warm.contains("brown"), "warm form run: {warm}");
    assert_eq!(warm, cold, "the replay must equal the cold run");
}

/// A page that draws "abc", paints a form whose text sits elsewhere, then sets the pen back over "abc" and
/// leads with a word gap. The form is another content stream under its own matrix: the run drawn before it
/// is not the run the pen sets back into, and the form's own end is not either.
fn form_between_runs_pdf() -> Vec<u8> {
    let paint = "BT /F1 10 Tf 200 500 Td (zz) Tj ET";
    let form = format!(
        "<< /Type /XObject /Subtype /Form /BBox [0 0 600 800] \
         /Resources << /Font << /F1 7 0 R >> >> /Length {} >>\nstream\n{paint}\nendstream",
        paint.len()
    );
    let content = "BT /F1 10 Tf 72 700 Td (abc) Tj ET /Fm0 Do BT /F1 10 Tf 72 700 Td [-3000 (def)] TJ ET";
    let contents = format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len());
    build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &[
            (
                3,
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] \
                   /Contents 5 0 R /Resources << /Font << /F1 7 0 R >> /XObject << /Fm0 6 0 R >> >> >>",
            ),
            (5, contents.as_bytes()),
            (6, form.as_bytes()),
            (
                7,
                b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
            ),
        ],
    )
}

#[test]
fn a_form_between_two_runs_ends_the_run_the_pen_sets_back_into() {
    let document = PdfDocument::from_bytes(form_between_runs_pdf()).expect("open the form page");
    let spans = document.extract_spans(0).expect("extract the form page");
    let texts: Vec<(&str, f32)> = spans.iter().map(|s| (s.text.as_str(), s.bbox.x)).collect();
    assert!(texts.iter().any(|(t, _)| *t == "zz"), "{texts:?}");
    let space = texts.iter().find(|(t, _)| t.trim().is_empty()).map(|(_, x)| *x);
    assert_eq!(
        space,
        Some(72.0),
        "the space sits at the pen, not at an end read across the form: {texts:?}"
    );
}
