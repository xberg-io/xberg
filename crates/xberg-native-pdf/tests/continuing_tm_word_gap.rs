//! A producer that writes `Tm /F Tf (word) Tj` for each word continues the run at every `Tm`. When an
//! operator ends that run before the next glyph, the run must end at its last glyph, so that the word
//! gap stays visible to the span merge (#2675).

use xberg_native_pdf::document::PdfDocument;
use xberg_native_pdf::layout::TextSpan;

const SIZE: f32 = 12.0;
const GLYPH: f32 = 7.2;

/// `A` at x = 100 with its own `Tm`; `after_a` follows it, before `ET`.
fn page(after_a: &str) -> PdfDocument {
    let content = format!("BT /F1 {SIZE} Tf\n1 0 0 1 100 500 Tm 100 Tz (A) Tj\n{after_a}\nET");
    PdfDocument::from_bytes(build(content.as_bytes())).expect("open fixture")
}

fn show(after_a: &str) -> String {
    page(after_a).extract_text(0).expect("extract_text").trim().to_string()
}

fn spans(after_a: &str) -> Vec<TextSpan> {
    page(after_a).extract_spans(0).expect("extract_spans")
}

/// The origin of a show `gap_em` after `A`'s advance.
fn x_after_a(gap_em: f32) -> f32 {
    100.0 + GLYPH + gap_em * SIZE
}

/// `A` at x = 100 and `B` one word gap after its advance, each with its own `Tm`; `between` sits after
/// the second `Tm` and before `(B) Tj`.
fn text(between: &str, gap_em: f32) -> String {
    show(&format!("1 0 0 1 {} 500 Tm {between}(B) Tj", x_after_a(gap_em)))
}

#[test]
fn same_font_tf_after_continuing_tm_splits_at_a_word_gap() {
    assert_eq!(text("/F1 12 Tf 100 Tz ", 0.278), "A B");
}

#[test]
fn new_font_tf_after_continuing_tm_splits_at_a_word_gap() {
    assert_eq!(text("/F2 12 Tf 100 Tz ", 0.278), "A B");
}

#[test]
fn tj_array_after_continuing_tm_splits_at_a_word_gap() {
    let x_b = 100.0 + GLYPH + 0.278 * SIZE;
    let content = format!("BT /F1 {SIZE} Tf\n1 0 0 1 100 500 Tm (A) Tj\n1 0 0 1 {x_b} 500 Tm [(B)] TJ\nET");
    let doc = PdfDocument::from_bytes(build(content.as_bytes())).expect("open fixture");
    assert_eq!(doc.extract_text(0).expect("extract_text").trim(), "A B");
}

#[test]
fn same_font_tf_after_continuing_tm_stays_joined_at_gap_zero() {
    assert_eq!(text("/F1 12 Tf 100 Tz ", 0.0), "AB");
}

#[test]
fn tz_only_after_continuing_tm_stays_joined() {
    assert_eq!(text("100 Tz ", 0.278), "AB");
}

#[test]
fn bare_continuing_tm_stays_joined() {
    assert_eq!(text("", 0.278), "AB");
}

/// Two continuing `Tm`s with no glyph between them, the first a tenth of an em before `B`, keep the
/// first record: the run still ends at `A`, not at the first `Tm`'s pen.
#[test]
fn second_continuing_tm_before_tf_keeps_the_run_at_its_last_glyph() {
    let (x_near, x_b) = (x_after_a(0.178), x_after_a(0.278));
    let after_a = format!("1 0 0 1 {x_near} 500 Tm 1 0 0 1 {x_b} 500 Tm /F1 12 Tf 100 Tz (B) Tj");
    assert_eq!(show(&after_a), "A B");
}

/// A glyph after the continuing `Tm`, even `C` with no width, ends the record: the same-font `Tf`
/// after it does not end the run, as at the base.
#[test]
fn zero_width_glyph_after_continuing_tm_ends_the_record() {
    let x_b = x_after_a(0.278);
    assert_eq!(
        show(&format!("1 0 0 1 {x_b} 500 Tm (C) Tj /F1 12 Tf 100 Tz (B) Tj")),
        "ACB"
    );
}

/// A `Tm` that restates the run under a new horizontal scale (GH#2664) after a continuing `Tm` keeps
/// the record in the new unit: `A` stays 7.2 wide at x = 100, `B` is drawn at x = 160 twice as wide,
/// and the blank 52.8 pt between them reads as a word gap when a new font ends the run.
#[test]
fn stretched_tm_after_continuing_tm_keeps_the_run_at_its_last_glyph() {
    let after_a = "1 0 0 1 160 500 Tm 2 0 0 1 160 500 Tm /F2 12 Tf 100 Tz (B) Tj";
    let spans = spans(after_a);
    let boxes: Vec<(String, f32, f32)> = spans.iter().map(|s| (s.text.clone(), s.bbox.x, s.bbox.width)).collect();
    assert_eq!(boxes.len(), 2, "{boxes:?}");
    for (span, (text, x, width)) in boxes.iter().zip([("A", 100.0, GLYPH), ("B", 160.0, 2.0 * GLYPH)]) {
        assert_eq!(span.0.trim(), text, "{boxes:?}");
        assert!((span.1 - x).abs() < 0.05 && (span.2 - width).abs() < 0.05, "{boxes:?}");
    }
    assert_eq!(show(after_a), "A B");
}

/// The same under a compressing scale and a repeated same-font `Tf`: `B` sits one word gap after
/// `A` at half width, and the run still ends at `A`.
#[test]
fn stretched_tm_after_continuing_tm_splits_at_a_word_gap_under_same_font_tf() {
    let x_b = x_after_a(0.278);
    let after_a = format!("1 0 0 1 {x_b} 500 Tm 0.5 0 0 1 {x_b} 500 Tm /F1 12 Tf 100 Tz (B) Tj");
    assert_eq!(show(&after_a), "A B");
}

/// Inside an `ActualText` scope the first show gives the text `AB` and a later show adds width only.
/// The `Tm` before that later show continues the run, but the width it adds ends the record, so the
/// same-font `Tf` after the scope does not end the run and `A` joins it, as at the base.
#[test]
fn actualtext_later_tj_after_continuing_tm_keeps_the_run() {
    let content = "BT /F1 12 Tf /Span <</ActualText (AB)>> BDC 1 0 0 1 100 500 Tm (A) Tj \
                   1 0 0 1 107.7 500 Tm (B) Tj EMC /F1 12 Tf (A) Tj ET";
    let doc = PdfDocument::from_bytes(build(content.as_bytes())).expect("open fixture");
    assert_eq!(doc.extract_text(0).expect("extract_text").trim(), "ABA");
}

/// The same with a `TJ` array as the later show.
#[test]
fn actualtext_later_tj_array_after_continuing_tm_keeps_the_run() {
    let content = "BT /F1 12 Tf /Span <</ActualText (AB)>> BDC 1 0 0 1 100 500 Tm (A) Tj \
                   1 0 0 1 107.7 500 Tm [(B)] TJ EMC /F1 12 Tf (A) Tj ET";
    let doc = PdfDocument::from_bytes(build(content.as_bytes())).expect("open fixture");
    assert_eq!(doc.extract_text(0).expect("extract_text").trim(), "ABA");
}

/// An offsets-only `TJ` inside an `ActualText` scope adds width and no space: the scope's string is
/// the text. It ends the record too, so `B` joins the run, as at the base.
#[test]
fn actualtext_offset_only_tj_after_continuing_tm_keeps_the_run() {
    let x_b = x_after_a(0.278);
    let content = format!(
        "BT /F1 12 Tf /Span <</ActualText (A)>> BDC 1 0 0 1 100 500 Tm (A) Tj \
         1 0 0 1 {x_b} 500 Tm [-600] TJ EMC /F1 12 Tf 100 Tz (B) Tj ET"
    );
    let doc = PdfDocument::from_bytes(build(content.as_bytes())).expect("open fixture");
    assert_eq!(doc.extract_text(0).expect("extract_text").trim(), "AB");
}

/// `F3`'s `D` has a width and no text (its `ToUnicode` map misses the code, the control row). After
/// the continuing `Tm` it adds width without text, which ends the record: the same-font `Tf` after it
/// does not end the run, so `B` joins it, as at the base.
#[test]
fn glyph_without_text_after_continuing_tm_ends_the_record() {
    let control = "BT /F3 12 Tf 1 0 0 1 100 500 Tm (A) Tj (D) Tj ET";
    let doc = PdfDocument::from_bytes(build(control.as_bytes())).expect("open fixture");
    assert_eq!(doc.extract_text(0).expect("extract_text").trim(), "A");
    let x_b = x_after_a(0.278);
    let content =
        format!("BT /F3 12 Tf 1 0 0 1 100 500 Tm (A) Tj 1 0 0 1 {x_b} 500 Tm (D) Tj /F3 12 Tf 100 Tz (B) Tj ET");
    let doc = PdfDocument::from_bytes(build(content.as_bytes())).expect("open fixture");
    assert_eq!(doc.extract_text(0).expect("extract_text").trim(), "AB");
}

/// An empty show draws nothing and moves nothing, so the record stays and the same-font `Tf` splits
/// at the word gap.
#[test]
fn empty_show_after_continuing_tm_splits_at_a_word_gap() {
    assert_eq!(text("() Tj /F1 12 Tf 100 Tz ", 0.278), "A B");
}

/// `Tz`, `Tc`, `Tw` and `Ts` set state and move nothing, so the record stays and the same-font `Tf`
/// splits at the word gap.
#[test]
fn tz_tc_tw_ts_after_continuing_tm_split_at_a_word_gap() {
    assert_eq!(text("90 Tz 0 Tc 0 Tw 0 Ts 100 Tz /F1 12 Tf ", 0.278), "A B");
}

/// A scope that never closes: the later show inside it still adds width and ends the record, so the
/// run `AB` reaches the pen when `ET` flushes it: 7.7 pt to `B`'s origin and `B`'s 7.2 pt, not `A`'s
/// 7.2 pt alone.
#[test]
fn actualtext_later_show_in_an_unclosed_scope_ends_the_record() {
    let content = "BT /F1 12 Tf /Span <</ActualText (AB)>> BDC 1 0 0 1 100 500 Tm (A) Tj \
                   1 0 0 1 107.7 500 Tm (B) Tj ET";
    let doc = PdfDocument::from_bytes(build(content.as_bytes())).expect("open fixture");
    let spans = doc.extract_spans(0).expect("extract_spans");
    let boxes: Vec<(String, f32, f32)> = spans.iter().map(|s| (s.text.clone(), s.bbox.x, s.bbox.width)).collect();
    assert_eq!(boxes.len(), 1, "{boxes:?}");
    assert_eq!(boxes[0].0.trim(), "AB", "{boxes:?}");
    assert!((boxes[0].1 - 100.0).abs() < 0.05 && (boxes[0].2 - (7.7 + GLYPH)).abs() < 0.05, "{boxes:?}");
}

/// A nested scope: the inner scope's first show gives its own text `C` and adds width, which ends the
/// record as a later show in one scope does, so the same-font `Tf` after both scopes does not end
/// the run and `A` joins it.
#[test]
fn actualtext_nested_scope_show_after_continuing_tm_keeps_the_run() {
    let content = "BT /F1 12 Tf /Span <</ActualText (AB)>> BDC 1 0 0 1 100 500 Tm (A) Tj \
                   /Span <</ActualText (C)>> BDC 1 0 0 1 107.7 500 Tm (B) Tj EMC EMC /F1 12 Tf (A) Tj ET";
    let doc = PdfDocument::from_bytes(build(content.as_bytes())).expect("open fixture");
    assert_eq!(doc.extract_text(0).expect("extract_text").trim(), "ABCA");
}

/// A continuing `Tm` past `AB`, then a `Tm` set back into `AB` (GH#2690): the set-back records
/// nothing and drops the first record, so the same-font `Tf` does not end the run and `A` joins it
/// at the set-back pen, as at the base.
#[test]
fn set_back_tm_after_continuing_tm_drops_the_record_before_same_font_tf() {
    let content = "BT /F1 12 Tf 1 0 0 1 100 500 Tm (AB) Tj 1 0 0 1 120 500 Tm 1 0 0 1 110 500 Tm \
                   /F1 12 Tf 100 Tz (A) Tj ET";
    let doc = PdfDocument::from_bytes(build(content.as_bytes())).expect("open fixture");
    assert_eq!(doc.extract_text(0).expect("extract_text").trim(), "ABA");
}

/// `A` is 3.6 pt wide under `50 Tz`. After a continuing `Tm` one word gap past it and a `Tm` that
/// restates the run twice as wide, the record still ends the run at `A`'s 3.6 pt when a new font
/// ends it, and `B` is drawn 7.2 pt wide at the restated pen.
#[test]
fn stretched_tm_after_continuing_tm_under_tz_keeps_the_run_at_its_last_glyph() {
    let x_b = 100.0 + 0.5 * GLYPH + 0.278 * SIZE;
    let content = format!(
        "BT /F1 {SIZE} Tf 50 Tz 1 0 0 1 100 500 Tm (A) Tj 1 0 0 1 {x_b} 500 Tm 2 0 0 1 {x_b} 500 Tm \
         /F2 12 Tf (B) Tj ET"
    );
    let doc = PdfDocument::from_bytes(build(content.as_bytes())).expect("open fixture");
    let spans = doc.extract_spans(0).expect("extract_spans");
    let boxes: Vec<(String, f32, f32)> = spans.iter().map(|s| (s.text.clone(), s.bbox.x, s.bbox.width)).collect();
    assert_eq!(boxes.len(), 2, "{boxes:?}");
    for (span, (text, x, width)) in boxes.iter().zip([("A", 100.0, 0.5 * GLYPH), ("B", x_b, GLYPH)]) {
        assert_eq!(span.0.trim(), text, "{boxes:?}");
        assert!((span.1 - x).abs() < 0.05 && (span.2 - width).abs() < 0.05, "{boxes:?}");
    }
    assert_eq!(doc.extract_text(0).expect("extract_text").trim(), "A B");
}

/// The same `50 Tz` run and restating `Tm`, then a show inside an `ActualText` scope: its width ends
/// the record, so the run reaches the pen in the restated unit, `B` and the `A` after the same-font
/// `Tf` join it, and the one span spans the 6.936 pt to the pen and the two 7.2 pt glyphs.
#[test]
fn actualtext_show_after_stretched_tm_under_tz_reaches_the_pen() {
    let x_b = 100.0 + 0.5 * GLYPH + 0.278 * SIZE;
    let content = format!(
        "BT /F1 {SIZE} Tf 50 Tz 1 0 0 1 100 500 Tm (A) Tj 1 0 0 1 {x_b} 500 Tm 2 0 0 1 {x_b} 500 Tm \
         /Span <</ActualText (B)>> BDC (B) Tj EMC /F1 12 Tf (A) Tj ET"
    );
    let doc = PdfDocument::from_bytes(build(content.as_bytes())).expect("open fixture");
    let spans = doc.extract_spans(0).expect("extract_spans");
    let boxes: Vec<(String, f32, f32)> = spans.iter().map(|s| (s.text.clone(), s.bbox.x, s.bbox.width)).collect();
    assert_eq!(boxes.len(), 1, "{boxes:?}");
    assert_eq!(boxes[0].0.trim(), "ABA", "{boxes:?}");
    let width = (x_b - 100.0) + 2.0 * GLYPH;
    assert!((boxes[0].1 - 100.0).abs() < 0.05 && (boxes[0].2 - width).abs() < 0.05, "{boxes:?}");
    assert_eq!(doc.extract_text(0).expect("extract_text").trim(), "ABA");
}

/// The `ToUnicode` map of `F3`: `A`, `B` and `C` only, so its `D` is a glyph with a width and no text.
const TO_UNICODE_ABC: &[u8] = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
    /CMapName /ABC def\n1 begincodespacerange\n<00> <FF>\nendcodespacerange\n\
    3 beginbfchar\n<41> <0041>\n<42> <0042>\n<43> <0043>\nendbfchar\nendcmap\n\
    CMapName currentdict /CMap defineresource pop\nend\nend\n";

/// One page with three Helvetica-metric fonts: `F1` and `F2`, whose `A` and `B` are 600/1000 em wide
/// and whose `C` has no width, and `F3`, the same with a 600/1000 em `D` that `TO_UNICODE_ABC` gives
/// no text.
fn build(content: &[u8]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];
    let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding \
                /FirstChar 65 /LastChar 67 /Widths [600 600 0] >>";
    let objs = [
        ("<< /Type /Catalog /Pages 2 0 R >>".to_string(), None),
        ("<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(), None),
        (
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R /F2 6 0 R /F3 7 0 R >> >> >>"
                .to_string(),
            None,
        ),
        (String::new(), Some(content)),
        (font.to_string(), None),
        (font.replace("Helvetica", "Times-Roman"), None),
        (
            font.replace("/LastChar 67 /Widths [600 600 0]", "/LastChar 68 /Widths [600 600 0 600] /ToUnicode 8 0 R")
                .to_string(),
            None,
        ),
        (String::new(), Some(TO_UNICODE_ABC)),
    ];
    for (i, (dict, stream)) in objs.iter().enumerate() {
        offsets.push(pdf.len());
        if let Some(data) = stream {
            pdf.extend_from_slice(format!("{} 0 obj\n<< /Length {} >>\nstream\n", i + 1, data.len()).as_bytes());
            pdf.extend_from_slice(data);
            pdf.extend_from_slice(b"\nendstream\nendobj\n");
        } else {
            pdf.extend_from_slice(format!("{} 0 obj\n{dict}\nendobj\n", i + 1).as_bytes());
        }
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", offsets.len()).as_bytes());
    for off in &offsets[1..] {
        pdf.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    pdf
}
