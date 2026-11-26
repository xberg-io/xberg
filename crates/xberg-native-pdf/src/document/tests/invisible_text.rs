//! White text is dropped only over unpainted page area; render mode 3 text is never dropped.

use super::super::*;
use super::common::{build_minimal_pdf_with_font, build_page_and_form_pdf};

fn span_texts(content: &str) -> Vec<String> {
    let doc = PdfDocument::from_bytes(build_minimal_pdf_with_font(content.as_bytes())).expect("fixture PDF parses");
    doc.extract_spans(0)
        .expect("fixture page extracts")
        .into_iter()
        .map(|span| span.text.trim().to_string())
        .filter(|text| !text.is_empty())
        .collect()
}

fn shown(texts: &[String], word: &str) -> bool {
    texts.iter().any(|text| text.contains(word))
}

const VISIBLE_LINE: &str = "BT /F1 10 Tf 1 0 0 1 50 700 Tm (Visible) Tj ET\n";

#[test]
fn white_text_on_an_unpainted_page_is_dropped() {
    for fill in ["1 g", "1 1 1 rg", "0 0 0 0 k", "/DeviceRGB cs 1 1 1 sc"] {
        let content = format!("{VISIBLE_LINE}BT /F1 10 Tf {fill} 1 0 0 1 50 600 Tm (Hidden) Tj ET\n");

        let texts = span_texts(&content);

        assert!(shown(&texts, "Visible"), "{fill}: {texts:?}");
        assert!(!shown(&texts, "Hidden"), "{fill}: {texts:?}");
    }
}

#[test]
fn white_text_on_a_dark_box_is_kept() {
    let content =
        format!("{VISIBLE_LINE}0 0 0.5 rg 40 590 200 30 re f\nBT /F1 10 Tf 1 g 1 0 0 1 50 600 Tm (Label) Tj ET\n");

    assert!(shown(&span_texts(&content), "Label"));
}

#[test]
fn white_text_on_a_white_box_is_dropped() {
    let content = format!("{VISIBLE_LINE}1 g 40 590 200 30 re f\nBT /F1 10 Tf 1 g 1 0 0 1 50 600 Tm (Hidden) Tj ET\n");

    assert!(!shown(&span_texts(&content), "Hidden"));
}

#[test]
fn white_text_over_an_image_is_kept() {
    let content = format!(
        "{VISIBLE_LINE}q 200 0 0 30 40 590 cm BI /W 1 /H 1 /BPC 8 /CS /G ID\n\u{0}\nEI Q\nBT /F1 10 Tf 1 g 1 0 0 1 50 600 Tm (Caption) Tj ET\n"
    );

    assert!(shown(&span_texts(&content), "Caption"));
}

/// An OCR text layer is drawn in render mode 3 and is the page's text even where the scan under
/// it is drawn by means the painted-content check cannot see.
#[test]
fn invisible_render_mode_text_is_kept() {
    let content = format!("{VISIBLE_LINE}BT /F1 10 Tf 3 Tr 1 0 0 1 50 600 Tm (Layer) Tj ET\n");

    assert!(shown(&span_texts(&content), "Layer"));
}

#[test]
fn white_text_in_an_unresolved_colour_space_is_kept() {
    let content = format!("{VISIBLE_LINE}BT /F1 10 Tf /CS0 cs 1 sc 1 0 0 1 50 600 Tm (Spot) Tj ET\n");

    assert!(shown(&span_texts(&content), "Spot"));
}

#[test]
fn black_text_is_untouched() {
    let content = format!("{VISIBLE_LINE}BT /F1 10 Tf 0 g 1 0 0 1 50 600 Tm (Black) Tj ET\n");

    let texts = span_texts(&content);

    assert!(shown(&texts, "Visible") && shown(&texts, "Black"), "{texts:?}");
}

fn page_texts(document: &PdfDocument) -> Vec<String> {
    document
        .extract_spans(0)
        .expect("fixture page extracts")
        .into_iter()
        .map(|span| span.text.trim().to_string())
        .filter(|text| !text.is_empty())
        .collect()
}

/// A Form's span-cache hit replays its spans without `push_span`, so the white ones once missed
/// the drop list: the cold read dropped them, every warm read kept them.
#[test]
fn white_text_in_a_cached_form_is_dropped_on_every_read() {
    let form = "BT /F1 12 Tf 50 700 Td (Shown) Tj ET BT /F1 12 Tf 1 g 50 600 Td (Hidden) Tj ET";
    let document = PdfDocument::from_bytes(build_page_and_form_pdf("/Fm0 Do", form)).expect("fixture PDF parses");

    let cold = page_texts(&document);
    assert_eq!(
        document.xobject_spans_cache.lock().unwrap().len(),
        1,
        "the form is cached"
    );
    document.clear_completed_span_caches();
    let warm = page_texts(&document);

    assert!(shown(&cold, "Shown") && !shown(&cold, "Hidden"), "{cold:?}");
    assert_eq!(warm, cold);
}
