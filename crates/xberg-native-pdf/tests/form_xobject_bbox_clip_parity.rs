//! The character layer and the span layer must agree on what a Form XObject
//! actually paints.
//!
//! A form's marks are clipped to its `/BBox` (ISO 32000-1:2008 §8.10.1), so
//! text a form draws outside that box is invisible in a conformant renderer
//! and must not be extracted. The span layer applies that clip. The character
//! layer did not, so on the pdfTeX pattern — a whole draft page embedded as a
//! figure-sized form — `extract_chars` returned a second, invisible copy of
//! the article that `extract_spans` correctly withheld, and the two APIs
//! disagreed about what was on the page.
//!
//! The clip deliberately spares forms that cover most of the page: those are
//! content-frame wrappers rather than figures, and their body text is often
//! the page's only copy. Both behaviours are pinned here, in both layers.

use xberg_native_pdf::PdfDocument;

const PAGE_W: f32 = 612.0;
const PAGE_H: f32 = 792.0;

/// One page that draws `page_text` directly and then invokes a form whose
/// `/BBox` is `bbox`, where the form paints `inside_text` within the box and
/// `outside_text` beyond it.
fn pdf_with_form(bbox: [f32; 4], inside_text: &str, outside_text: &str, page_text: &str) -> Vec<u8> {
    let inside_y = bbox[1] + (bbox[3] - bbox[1]) * 0.5;
    let inside_x = bbox[0] + 10.0;
    // Painted above the box, so it is outside the clip for every fixture here.
    let outside_y = bbox[3] + 60.0;
    let form = format!(
        "BT /F1 12 Tf 1 0 0 1 {inside_x} {inside_y} Tm ({inside_text}) Tj ET\n\
         BT /F1 12 Tf 1 0 0 1 {inside_x} {outside_y} Tm ({outside_text}) Tj ET\n"
    );
    let content = format!("BT /F1 12 Tf 1 0 0 1 72 60 Tm ({page_text}) Tj ET\n/Fm0 Do\n");

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 6];
    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");

    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id - 1] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };
    let stream_obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, dict: &str, s: &str| {
        off[id - 1] = buf.len();
        buf.extend_from_slice(
            format!(
                "{id} 0 obj\n<< {dict} /Length {} >>\nstream\n{s}\nendstream\nendobj\n",
                s.len()
            )
            .as_bytes(),
        );
    };

    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        &format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W} {PAGE_H}] \
             /Resources << /Font << /F1 5 0 R >> /XObject << /Fm0 6 0 R >> >> \
             /Contents 4 0 R >>"
        ),
    );
    stream_obj(&mut buf, &mut off, 4, "", &content);
    obj(
        &mut buf,
        &mut off,
        5,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    );
    stream_obj(
        &mut buf,
        &mut off,
        6,
        &format!(
            "/Type /XObject /Subtype /Form /FormType 1 \
             /BBox [{} {} {} {}] /Resources << /Font << /F1 5 0 R >> >>",
            bbox[0], bbox[1], bbox[2], bbox[3]
        ),
        &form,
    );

    let xref = buf.len();
    buf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for o in off.iter() {
        buf.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    buf
}

fn char_text(doc: &PdfDocument) -> String {
    doc.extract_chars(0)
        .expect("extract_chars")
        .iter()
        .map(|c| c.char)
        .filter(|c| !c.is_whitespace())
        .collect()
}

fn span_text(doc: &PdfDocument) -> String {
    doc.extract_spans(0)
        .expect("extract_spans")
        .iter()
        .flat_map(|s| s.text.chars())
        .filter(|c| !c.is_whitespace())
        .collect()
}

fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn char_layer_clips_figure_form_text_outside_its_bbox() {
    // 200 x 150 on a 612 x 792 page — ~6% of the page, unambiguously a figure.
    let pdf = pdf_with_form(
        [100.0, 100.0, 300.0, 250.0],
        "INSIDE FIGURE BOX",
        "DRAFT GALLEY COPY",
        "PUBLISHED BODY TEXT",
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse");

    let chars = char_text(&doc);
    let spans = span_text(&doc);

    assert!(
        chars.contains(&squash("PUBLISHED BODY TEXT")),
        "page's own text missing from the character layer: {chars:?}"
    );
    assert!(
        chars.contains(&squash("INSIDE FIGURE BOX")),
        "in-BBox form text missing from the character layer: {chars:?}"
    );
    assert!(
        !spans.contains(&squash("DRAFT GALLEY COPY")),
        "control fixture is wrong: the span layer should already clip this"
    );
    assert!(
        !chars.contains(&squash("DRAFT GALLEY COPY")),
        "character layer returned form text painted outside the form's /BBox, \
         which no conformant renderer paints: {chars:?}"
    );
}

#[test]
fn both_layers_keep_text_outside_a_page_wrapper_bbox() {
    // 512 x 630 — ~67% of the page, over the threshold at which a form counts
    // as a content-frame wrapper rather than a figure. Its body may be the
    // page's only copy, so the clip spares it and both layers keep the
    // out-of-BBox run.
    let pdf = pdf_with_form(
        [50.0, 50.0, 562.0, 680.0],
        "WRAPPER INNER TEXT",
        "WRAPPER OUTER TEXT",
        "PUBLISHED BODY TEXT",
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse");

    let chars = char_text(&doc);
    let spans = span_text(&doc);

    for layer in [&chars, &spans] {
        assert!(
            layer.contains(&squash("WRAPPER INNER TEXT")),
            "wrapper in-BBox text lost: {layer:?}"
        );
        assert!(
            layer.contains(&squash("WRAPPER OUTER TEXT")),
            "wrapper text outside its /BBox must be kept — a page-covering form \
             is a content frame, not a figure: {layer:?}"
        );
    }
}

#[test]
fn character_and_span_layers_agree_on_form_content() {
    let pdf = pdf_with_form(
        [100.0, 100.0, 300.0, 250.0],
        "INSIDE FIGURE BOX",
        "DRAFT GALLEY COPY",
        "PUBLISHED BODY TEXT",
    );
    let doc = PdfDocument::from_bytes(pdf).expect("parse");

    let mut chars: Vec<char> = char_text(&doc).chars().collect();
    let mut spans: Vec<char> = span_text(&doc).chars().collect();
    chars.sort_unstable();
    spans.sort_unstable();
    assert_eq!(
        chars.iter().collect::<String>(),
        spans.iter().collect::<String>(),
        "extract_chars and extract_spans disagree about what the page paints"
    );
}
