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

fn form_page_texts(page: &str, form: &str) -> Vec<String> {
    let document = PdfDocument::from_bytes(build_page_and_form_pdf(page, form)).expect("fixture PDF parses");
    page_texts(&document)
}

/// The path walker models `rg`, `g` and `k` only: a box filled after `sc` or `scn` keeps the
/// stale white of the earlier `1 g`, so the label over it read as white on nothing.
#[test]
fn white_text_on_a_box_filled_by_sc_is_kept() {
    for fill in [
        "/DeviceRGB cs 0 0 0.5 sc",
        "/DeviceRGB cs 0 0 0.5 scn",
        "/DeviceGray cs",
    ] {
        let content =
            format!("{VISIBLE_LINE}1 g {fill} 40 590 200 30 re f\nBT /F1 10 Tf 1 g 1 0 0 1 50 600 Tm (Label) Tj ET\n");

        assert!(shown(&span_texts(&content), "Label"), "{fill}");
    }
}

/// The colour audit read only the page stream, so a Form filling a box in a colour the path
/// walker cannot model left the page resolvable, and the label over the box was dropped.
#[test]
fn white_text_on_a_box_a_form_fills_in_an_unmodelled_colour_is_kept() {
    let page = format!("{VISIBLE_LINE}1 g /Fm0 Do BT /F1 10 Tf 1 g 1 0 0 1 50 600 Tm (Label) Tj ET");
    for fill in ["/DeviceRGB cs 0 0 0.5 sc", "/CS0 cs 1 sc", "/Pattern cs /P0 scn"] {
        let form = format!("{fill} 40 590 200 30 re f");

        assert!(shown(&form_page_texts(&page, &form), "Label"), "{fill}");
    }
}

#[test]
fn white_text_on_a_white_box_a_form_fills_is_dropped() {
    let page = format!("{VISIBLE_LINE}/Fm0 Do BT /F1 10 Tf 1 g 1 0 0 1 50 600 Tm (Hidden) Tj ET");

    let texts = form_page_texts(&page, "1 g 40 590 200 30 re f");

    assert!(shown(&texts, "Visible") && !shown(&texts, "Hidden"), "{texts:?}");
}

#[test]
fn painted_regions_are_read_once_per_page_during_the_page_passes() {
    let content = format!("{VISIBLE_LINE}0 0 0.5 rg 40 590 200 30 re f\n");
    let document =
        PdfDocument::from_bytes(build_minimal_pdf_with_font(content.as_bytes())).expect("fixture PDF parses");
    let outside = (document.painted_regions(0), document.painted_regions(0));
    assert!(!std::sync::Arc::ptr_eq(
        outside.0.as_ref().unwrap(),
        outside.1.as_ref().unwrap()
    ));

    document.begin_page_passes();
    let first = document.painted_regions(0).expect("the page resolves");
    let second = document.painted_regions(0).expect("the page resolves");
    document.release_page_caches();

    assert_eq!(first.len(), 1);
    assert!(std::sync::Arc::ptr_eq(&first, &second));
    assert_eq!(*first, *outside.0.unwrap());
}

fn char_text(content: &str) -> String {
    let doc = PdfDocument::from_bytes(build_minimal_pdf_with_font(content.as_bytes())).expect("fixture PDF parses");
    doc.extract_chars(0)
        .expect("fixture page extracts")
        .iter()
        .map(|character| character.char)
        .collect()
}

/// Table and script passes read the char layer, which kept the white glyphs the span layer drops.
#[test]
fn char_layer_drops_the_white_text_the_span_layer_drops() {
    let hidden = format!("{VISIBLE_LINE}BT /F1 10 Tf 1 g 1 0 0 1 50 600 Tm (Hidden) Tj ET\n");
    let labelled =
        format!("{VISIBLE_LINE}0 0 0.5 rg 40 590 200 30 re f\nBT /F1 10 Tf 1 g 1 0 0 1 50 600 Tm (Label) Tj ET\n");

    let chars = char_text(&hidden);
    assert!(chars.contains("Visible") && !chars.contains("Hidden"), "{chars:?}");
    let chars = char_text(&labelled);
    assert!(chars.contains("Visible") && chars.contains("Label"), "{chars:?}");
}

const WHITE_LABEL: &str = "BT /F1 10 Tf 1 g 1 0 0 1 50 600 Tm (Label) Tj ET\n";

/// A shading or a pattern colours its area in ways the path walker does not resolve, so a label
/// over one is kept, on the page and inside a Form alike.
#[test]
fn white_text_over_a_shading_or_a_pattern_is_kept() {
    for paint in [
        "q 40 590 200 30 re W n /Sh0 sh Q",
        "/Pattern cs /P0 scn 40 590 200 30 re f",
        "/Pattern CS /P0 SCN 8 w 40 590 200 30 re S",
    ] {
        let content = format!("{VISIBLE_LINE}{paint}\n{WHITE_LABEL}");

        assert!(shown(&span_texts(&content), "Label"), "page: {paint}");
        assert!(char_text(&content).contains("Label"), "page chars: {paint}");

        let page = format!("{VISIBLE_LINE}/Fm0 Do\n{WHITE_LABEL}");
        assert!(shown(&form_page_texts(&page, paint), "Label"), "form: {paint}");
    }
}

#[test]
fn white_text_over_an_inline_image_a_form_paints_is_kept() {
    let page = format!("{VISIBLE_LINE}/Fm0 Do\n{WHITE_LABEL}");
    let form = "q 200 0 0 30 40 590 cm BI /W 1 /H 1 /BPC 8 /CS /G ID\n\u{0}\nEI Q";

    assert!(shown(&form_page_texts(&page, form), "Label"));
}

/// One page painting the Form `Fm0` and the 1x1 grey Image XObject `Im0`; `Fm0` paints `form`
/// and can paint `Im0` too.
fn image_xobject_pdf(page: &str, form: &str) -> Vec<u8> {
    let stream = |dict: &str, body: &str| format!("<< {dict} /Length {} >>\nstream\n{body}\nendstream", body.len());
    let objects: [(u32, String); 5] = [
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> /XObject << /Fm0 6 0 R /Im0 7 0 R >> >> >>"
                .to_string(),
        ),
        (4, stream("", page)),
        (
            5,
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
        ),
        (
            6,
            stream(
                "/Type /XObject /Subtype /Form /BBox [0 0 600 800] /Resources << /XObject << /Im0 7 0 R >> >>",
                form,
            ),
        ),
        (
            7,
            stream(
                "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8",
                "A",
            ),
        ),
    ];
    let extra: Vec<(u32, &[u8])> = objects.iter().map(|(id, body)| (*id, body.as_bytes())).collect();
    super::pdf_fixtures::build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &extra,
    )
}

#[test]
fn white_text_over_an_image_xobject_is_kept_on_the_page_and_in_a_form() {
    let image = "q 200 0 0 30 40 590 cm /Im0 Do Q";
    for (page, form) in [
        (format!("{VISIBLE_LINE}{image}\n{WHITE_LABEL}"), String::new()),
        (format!("{VISIBLE_LINE}/Fm0 Do\n{WHITE_LABEL}"), image.to_string()),
    ] {
        let document = PdfDocument::from_bytes(image_xobject_pdf(&page, &form)).expect("fixture PDF parses");

        assert!(shown(&page_texts(&document), "Label"), "{page}");
    }
}

/// One page painting `page`, with a widget over `[40 590 240 620]` whose appearance fills that
/// area dark blue and shows white text.
fn widget_pdf(page: &str) -> Vec<u8> {
    let appearance = "0 0 0.5 rg 0 0 200 30 re f BT /F1 10 Tf 1 g 5 10 Td (Field) Tj ET";
    let objects: [(u32, String); 5] = [
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> >> /Annots [6 0 R] >>"
                .to_string(),
        ),
        (4, format!("<< /Length {} >>\nstream\n{page}\nendstream", page.len())),
        (
            5,
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_string(),
        ),
        (
            6,
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (name) /Rect [40 590 240 620] /AP << /N 7 0 R >> >>"
                .to_string(),
        ),
        (
            7,
            format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 200 30] /Resources << /Font << /F1 5 0 R >> >> \
                 /Length {} >>\nstream\n{appearance}\nendstream",
                appearance.len()
            ),
        ),
    ];
    let extra: Vec<(u32, &[u8])> = objects.iter().map(|(id, body)| (*id, body.as_bytes())).collect();
    super::pdf_fixtures::build_catalog_test_pdf(
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        &extra,
    )
}

/// Annotations are painted after the page content, so a widget's dark appearance covers white page
/// text under it rather than showing it: that text is dropped as on a bare page. The widget's own
/// white text is read from its appearance stream, which no white-text drop applies to.
#[test]
fn a_widget_appearance_neither_shows_white_page_text_nor_loses_its_own() {
    let page = format!("{VISIBLE_LINE}BT /F1 10 Tf 1 g 1 0 0 1 50 600 Tm (Hidden) Tj ET\n");
    let document = PdfDocument::from_bytes(widget_pdf(&page)).expect("fixture PDF parses");

    let texts = page_texts(&document);
    assert!(shown(&texts, "Visible") && !shown(&texts, "Hidden"), "{texts:?}");

    let annotations = document.get_annotations(0).expect("annotations read");
    let widget = annotations.first().expect("the widget is read");
    let field = document
        .extract_annotation_appearance_text(widget, &HashSet::new())
        .expect("the appearance has text");
    assert!(field.contains("Field"), "{field:?}");
}

const HIDDEN_LINE: &str = "BT /F1 10 Tf 1 g 1 0 0 1 50 600 Tm (Hidden) Tj ET\n";

#[test]
fn dropped_white_text_is_counted_per_page() {
    let dropping = PdfDocument::from_bytes(build_minimal_pdf_with_font(
        format!("{VISIBLE_LINE}{HIDDEN_LINE}BT /F1 10 Tf 1 g 1 0 0 1 50 500 Tm (Gone) Tj ET\n").as_bytes(),
    ))
    .expect("fixture PDF parses");
    let keeping = PdfDocument::from_bytes(build_minimal_pdf_with_font(
        format!("{VISIBLE_LINE}0 0 0.5 rg 40 590 200 30 re f\n{WHITE_LABEL}").as_bytes(),
    ))
    .expect("fixture PDF parses");

    let texts = page_texts(&dropping);
    let _ = page_texts(&dropping);
    let _ = page_texts(&keeping);

    assert!(!shown(&texts, "Hidden") && !shown(&texts, "Gone"), "{texts:?}");
    assert_eq!(dropping.dropped_white_text_spans(), vec![(0, 2)]);
    assert!(keeping.dropped_white_text_spans().is_empty());
}

#[test]
fn keep_white_text_keeps_and_counts_nothing_in_spans_and_chars() {
    let content = format!("{VISIBLE_LINE}{HIDDEN_LINE}");
    let document =
        PdfDocument::from_bytes(build_minimal_pdf_with_font(content.as_bytes())).expect("fixture PDF parses");
    let dropped = page_texts(&document);
    assert!(!shown(&dropped, "Hidden"), "{dropped:?}");

    document.set_keep_white_text(true);
    let spans = page_texts(&document);
    let chars: String = document
        .extract_chars(0)
        .expect("fixture page extracts")
        .iter()
        .map(|character| character.char)
        .collect();

    assert!(shown(&spans, "Visible") && shown(&spans, "Hidden"), "{spans:?}");
    assert!(chars.contains("Hidden"), "{chars:?}");

    let fresh = PdfDocument::from_bytes(build_minimal_pdf_with_font(content.as_bytes())).expect("fixture PDF parses");
    fresh.set_keep_white_text(true);
    let _ = page_texts(&fresh);
    assert!(fresh.dropped_white_text_spans().is_empty());
}
