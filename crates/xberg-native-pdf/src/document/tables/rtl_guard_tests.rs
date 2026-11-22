//! A predominantly-RTL page with too little ruling to bound a grid is not read
//! as a table. Aligned Arabic/Hebrew runs look like columns to the clustering,
//! and the RTL guard that follows detection only ever gated the retry, so a
//! table produced by the first pass was handed straight back and its runs were
//! emitted twice, once as the table and once as prose.
//!
//! Drives `extract_page_tables` with `text_fallback` on, the only entry that reaches the guard.

use crate::converters::ConversionOptions;
use crate::document::PdfDocument;

/// Minimal untagged one-page PDF: a simple TrueType font (`/FirstChar 0`,
/// `/Widths`, `/ToUnicode`) and a plain content stream. `content_ops` is
/// the full `BT ... ET` body.
fn build_pdf(tounicode_bfchars: &str, widths: &str, last_char: usize, content_ops: &str) -> Vec<u8> {
    let tounicode = format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin begincmap\n\
         1 begincodespacerange <00> <FF> endcodespacerange\n\
         {} beginbfchar\n{}endbfchar\nendcmap CMapName currentdict /CMap defineresource pop end end",
        tounicode_bfchars.lines().filter(|l| !l.trim().is_empty()).count(),
        tounicode_bfchars,
    );

    let content_bytes = content_ops.as_bytes();

    let mut buf: Vec<u8> = Vec::new();
    let mut off = vec![0usize; 7];
    let obj = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, body: &str| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    };
    let stream = |buf: &mut Vec<u8>, off: &mut Vec<usize>, id: usize, dict: &str, data: &[u8]| {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n<< {dict} /Length {} >>\nstream\n", data.len()).as_bytes());
        buf.extend_from_slice(data);
        buf.extend_from_slice(b"\nendstream\nendobj\n");
    };

    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    obj(&mut buf, &mut off, 2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] \
         /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>",
    );
    stream(&mut buf, &mut off, 4, "", content_bytes);
    obj(
        &mut buf,
        &mut off,
        5,
        &format!(
            "<< /Type /Font /Subtype /TrueType /BaseFont /Synthetic \
             /FirstChar 0 /LastChar {last_char} /Widths [{widths}] /ToUnicode 6 0 R >>"
        ),
    );
    stream(&mut buf, &mut off, 6, "", tounicode.as_bytes());

    let xref_off = buf.len();
    buf.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for &offset in &off[1..=6] {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(b"trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n");
    buf.extend_from_slice(format!("{xref_off}\n%%EOF\n").as_bytes());
    buf
}

const TOUNICODE: &str = "\
<01> <0031>
<02> <0032>
<03> <0033>
<04> <0034>
<05> <0035>
<06> <0036>
<07> <0037>
<08> <0038>
<09> <0039>
<0A> <05D1>
<0B> <05D9>
<0C> <05E6>
<0D> <05D4>
<20> <0020>
";
const WIDTHS: &str = "600 600 600 600 600 600 600 600 600 600 600 600 600 600";
const LAST_CHAR: usize = 13;

/// Six rows of `cols` columns drawn at 90 pt pitch: the first column is a
/// Hebrew word, the others are digit runs, so the page reads as a table to the
/// clustering but carries no ruling.
fn grid(cols: usize) -> String {
    let mut ops = String::from("BT /F1 12 Tf\n");
    for row in 0..6usize {
        let y = 190 - row * 20;
        for col in 0..cols {
            let x = 20 + col * 90;
            let code = if col == 0 {
                format!("0{:X}0{:X}0{:X}", 10 + row % 4, 10 + (row + 1) % 4, 10 + (row + 2) % 4)
            } else {
                let (a, b) = (1 + (row * 3 + col) % 9, 1 + (row * 5 + col * 2 + 1) % 9);
                format!("{a:02X}{b:02X}{a:02X}")
            };
            ops.push_str(&format!("1 0 0 1 {x} {y} Tm [<{code}>] TJ\n"));
        }
    }
    ops.push_str("ET");
    ops
}

fn tables_of(cols: usize) -> (usize, usize) {
    let doc = PdfDocument::from_bytes(build_pdf(TOUNICODE, WIDTHS, LAST_CHAR, &grid(cols))).expect("open");
    let spans = doc.extract_spans(0).expect("spans");
    let rtl = spans.iter().filter(|s| crate::text::bidi::looks_rtl(&s.text)).count();
    let tables = doc.extract_page_tables(0, &spans, &ConversionOptions::default(), true);
    (rtl * 100 / spans.len(), tables.len())
}

#[test]
fn aligned_rtl_words_without_ruling_are_not_emitted_as_a_table() {
    let (rtl_percent, tables) = tables_of(3);
    assert!(
        rtl_percent > 30,
        "fixture is only {rtl_percent}% RTL, below the guard threshold"
    );
    assert_eq!(
        tables, 0,
        "an unruled predominantly-RTL page was read as {tables} table(s)"
    );
}

/// The same grid with the RTL share under the threshold is still a table, so
/// the guard above is what declines the page and not the detector.
#[test]
fn the_same_grid_below_the_rtl_threshold_is_still_a_table() {
    let (rtl_percent, tables) = tables_of(4);
    assert!(
        rtl_percent <= 30,
        "fixture is {rtl_percent}% RTL, above the guard threshold"
    );
    assert_eq!(tables, 1);
}

/// Two column rectangles and one divider are three primitives but six rules,
/// enough to bound a 2x2 grid, so the guard must not decline them.
#[test]
fn a_ruled_rtl_grid_drawn_with_three_primitives_is_still_a_table() {
    let ops = "0.5 w\n20 140 90 60 re S\n110 140 90 60 re S\n20 170 m 200 170 l S\n\
               BT /F1 12 Tf\n\
               1 0 0 1 30 180 Tm [<0A0B0C>] TJ\n\
               1 0 0 1 120 180 Tm [<010201>] TJ\n\
               1 0 0 1 30 150 Tm [<0B0C0D>] TJ\n\
               1 0 0 1 120 150 Tm [<030403>] TJ\nET";
    let doc = PdfDocument::from_bytes(build_pdf(TOUNICODE, WIDTHS, LAST_CHAR, ops)).expect("open");
    let spans = doc.extract_spans(0).expect("spans");
    let rtl = spans.iter().filter(|s| crate::text::bidi::looks_rtl(&s.text)).count();
    assert!(rtl * 100 / spans.len() > 30, "fixture is not predominantly RTL");
    let tables = doc.extract_page_tables(0, &spans, &ConversionOptions::default(), true);
    assert_eq!(tables.len(), 1, "a ruled RTL grid was declined");
}

/// A Hebrew run drawn in visual order is reversed to logical order, so the
/// per-character widths read off the stream no longer describe the emitted
/// characters unless they are dropped or reordered with them.
#[test]
fn char_widths_describe_the_emitted_text_after_an_rtl_reversal() {
    // codes 0x0A, 0x0B, 0x0C -> U+05D1, U+05D9, U+05E6 at widths 500, 600, 700.
    let widths = "600 600 600 600 600 600 600 600 600 600 500 600 700 600";
    let draw = |show: &str| {
        let ops = format!("BT /F1 12 Tf 1 0 0 1 20 100 Tm {show} ET");
        let doc = PdfDocument::from_bytes(build_pdf(TOUNICODE, widths, LAST_CHAR, &ops)).expect("open");
        doc.extract_spans(0).expect("spans")
    };
    for show in ["<0A0B0C> Tj", "[<0A0B0C>] TJ"] {
        let spans = draw(show);
        let span = spans
            .iter()
            .find(|s| s.text.chars().any(|c| c == '\u{5D1}'))
            .expect("hebrew span");
        assert_eq!(
            span.text, "\u{5E6}\u{5D9}\u{5D1}",
            "{show}: fixture must be reversed to logical order"
        );
        if span.char_widths.is_empty() {
            continue;
        }
        assert_eq!(span.char_widths.len(), 3, "{show}");
        let ratio = span.char_widths[0] / span.char_widths[2];
        assert!(
            (ratio - 700.0 / 500.0).abs() < 0.01,
            "{show}: widths {:?} are not in the order of the text",
            span.char_widths
        );
    }
}
