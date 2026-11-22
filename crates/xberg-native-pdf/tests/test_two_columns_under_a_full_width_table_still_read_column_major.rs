//! Two columns of prose do not stop being two columns because a table crosses
//! the page beneath them.
//!
//! ISO 32000-1:2008 §9.4.4 (`docs/spec/pdf.md`:17398) puts a horizontal show
//! string on one unbroken interval of the X axis, and NOTE 6 to 9.4.3
//! (`docs/spec/pdf.md`:17389) asks extraction to return natural reading order.
//! An untagged page carries no logical structure, so that order has to come
//! from the printed geometry — and the geometry here is unambiguous: two column
//! blocks of body text, each thirty lines tall, with an empty channel between
//! them for their whole height.
//!
//! The channel is empty only over the body. A full-measure ruled table sits
//! below the columns, and its cells cross the channel's X range, so a coverage
//! sweep taken over the whole page finds no empty corridor anywhere. The page
//! then falls to the guard that asks whether the content outside the detected
//! tables is single-column prose, and that guard has to answer "no".
//!
//! Read row-major, the page comes out with each left-column line glued to the
//! right-column line printed beside it, which tears the wrapped word in the
//! right column in half.

use xberg_native_pdf::PdfDocument;

const LEFT_X: f32 = 72.0;
const RIGHT_X: f32 = 316.0;
const TOP: f32 = 700.0;
const LEADING: f32 = 12.0;
const ROWS: usize = 30;

/// Wide enough that each column line reaches within a few points of the
/// channel: the columns must look like columns, not like an indent.
const LEFT_BODY: &str = "the left column carries an ordinary line of text here";
const RIGHT_BODY: &str = "the right column carries an ordinary line";

/// The wrapped word the splice tears. It is printed as two runs on two
/// consecutive lines of the RIGHT column, so it survives only if the right
/// column is read as one run of lines.
const WRAP_ROW: usize = 9;

fn show(c: &mut String, x: f32, y: f32, size: f32, t: &str) {
    c.push_str(&format!("BT /F1 {size} Tf {x} {y} Td ({t}) Tj ET\n"));
}

fn hline(c: &mut String, x0: f32, x1: f32, y: f32) {
    c.push_str(&format!("{x0} {y} m {x1} {y} l 0.6 w S\n"));
}

fn vline(c: &mut String, x: f32, y0: f32, y1: f32) {
    c.push_str(&format!("{x} {y0} m {x} {y1} l 0.6 w S\n"));
}

fn pdf_with(content: String) -> Vec<u8> {
    let content = content.into_bytes();
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
           /Resources << /Font << /F1 5 0 R >> >> >>"
            .to_vec(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content,
            b"\nendstream".to_vec(),
        ]
        .concat(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    let n = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {n}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    pdf
}

/// A full-measure UNRULED figure under the columns — a bit chart, an alphabet
/// table, a listing. Its cells stop short of the channel between the two body
/// columns on the left and resume beyond it on the right, exactly as a printed
/// figure's own columns do; what crosses the channel is the caption and the
/// folio. The figure's cell count is what makes the two body columns a
/// minority of the page's spans, which is the state the reported page is in.
fn full_measure_figure(c: &mut String) {
    const CELL_X: [f32; 8] = [78.0, 135.0, 192.0, 249.0, 330.0, 387.0, 444.0, 501.0];
    const FIG_TOP: f32 = 300.0;
    for r in 0..8 {
        let y = FIG_TOP - 14.0 * r as f32;
        for (i, x) in CELL_X.iter().enumerate() {
            show(c, *x, y, 9.0, &format!("{}{}0{}", r % 2, i % 2, r % 2));
        }
    }
    // The caption: a label under the left column and a title that starts in the
    // channel and runs across it. One run reaching across is what a page-wide
    // coverage sweep sees; it is not the body.
    show(c, 192.0, 188.0, 9.0, "Figure 3:");
    show(c, 302.0, 188.0, 9.0, "Hidden in the Idle Bits");
}

/// A small RULED key/value table in the bottom margin — the one grid on the
/// page the line-driven detector recognises, so the guard under test is
/// consulted at all. It sits well clear of both the columns and the figure.
fn ruled_settings_table(c: &mut String) {
    const BASE: f32 = 160.0;
    for (i, (k, v)) in [
        ("Sampler steps", "fifty"),
        ("Guidance scale", "seven"),
        ("Resolution", "1024"),
        ("Precision", "bf16"),
    ]
    .iter()
    .enumerate()
    {
        let y = BASE - 16.0 * i as f32;
        show(c, 96.0, y, 9.0, k);
        show(c, 210.0, y, 9.0, v);
        hline(c, 90.0, 260.0, y + 12.0);
    }
    hline(c, 90.0, 260.0, BASE - 52.0);
    for x in [90.0, 200.0, 260.0] {
        vline(c, x, BASE + 12.0, BASE - 52.0);
    }
}

fn two_column_page_over_a_table() -> Vec<u8> {
    let mut c = String::new();
    for r in 0..ROWS {
        let y = TOP - LEADING * r as f32;
        show(&mut c, LEFT_X, y, 9.0, &format!("LR{:02} {LEFT_BODY}", r + 1));
        let right = if r == WRAP_ROW {
            format!("RL{:02} the artwork came from the Septem-", r + 1)
        } else if r == WRAP_ROW + 1 {
            // No tag: the continuation has to close the wrap, so nothing may
            // sit between the two halves of the word.
            "ber 1977 issue of the magazine".to_string()
        } else {
            format!("RL{:02} {RIGHT_BODY}", r + 1)
        };
        show(&mut c, RIGHT_X, y, 9.0, &right);
    }
    full_measure_figure(&mut c);
    ruled_settings_table(&mut c);
    // The centred folio, printed in the channel, as a running page carries it.
    show(&mut c, 301.0, 72.0, 9.0, "15");
    pdf_with(c)
}

/// The fixture only tests what it claims to if the page really carries a
/// detected table, a channel between the columns over the body, and no empty
/// corridor when the whole page is swept at once.
#[test]
fn test_fixture_is_two_columns_over_a_table_that_blankets_the_channel() {
    let doc = PdfDocument::from_bytes(two_column_page_over_a_table()).expect("open");
    assert!(
        !doc.extract_tables(0).expect("tables").is_empty(),
        "the page must carry a table the detector recognises, else the guard \
         under test is never consulted"
    );
    let spans = doc.extract_spans(0).expect("spans");
    let body_bottom = TOP - LEADING * (ROWS as f32 - 1.0);
    let body: Vec<&xberg_native_pdf::layout::TextSpan> =
        spans.iter().filter(|s| s.bbox.y >= body_bottom - 1.0).collect();
    assert!(body.len() >= 2 * ROWS, "expected both columns, got {}", body.len());
    let left_right_edge = body
        .iter()
        .filter(|s| s.bbox.x < RIGHT_X - 1.0)
        .map(|s| s.bbox.x + s.bbox.width)
        .fold(f32::MIN, f32::max);
    assert!(
        RIGHT_X - left_right_edge > 12.0,
        "the channel over the body measures {}pt; it has to read as a gutter",
        RIGHT_X - left_right_edge
    );
    // Swept over the whole page, the same channel is not empty: the table's
    // middle cells and the folio cross it.
    let mut boxes: Vec<(f32, f32)> = spans
        .iter()
        .filter(|s| !s.text.trim().is_empty() && s.bbox.width > 0.0)
        .map(|s| (s.bbox.x, s.bbox.x + s.bbox.width))
        .collect();
    boxes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut cover = boxes[0].1;
    let mut corridors = 0usize;
    for &(l, r) in &boxes[1..] {
        if l - cover >= 12.0 {
            corridors += 1;
        }
        cover = cover.max(r);
    }
    assert_eq!(
        corridors, 0,
        "a page-wide sweep must find no corridor here, else the page never \
         reaches the guard under test"
    );
}

/// The page reads column-major: the whole left column, then the right one.
#[test]
fn test_body_reads_column_major_under_the_table() {
    let doc = PdfDocument::from_bytes(two_column_page_over_a_table()).expect("open");
    let out = doc.extract_text(0).expect("text");
    let at = |tag: &str| {
        out.find(tag)
            .unwrap_or_else(|| panic!("`{tag}` missing from output:\n{out}"))
    };
    assert!(
        at(&format!("LR{ROWS:02}")) < at("RL01"),
        "the right column is spliced into the left column line by line. Two \
         columns of body text with an empty channel between them read one \
         column at a time (ISO 32000-1 §9.4.3 NOTE 6); a table crossing the \
         page beneath them does not make them one column. Got:\n{out}"
    );
}

/// The reader-visible damage: the splice lands between the two halves of a
/// wrapped word in the right column.
#[test]
fn test_wrapped_word_in_the_right_column_survives() {
    let doc = PdfDocument::from_bytes(two_column_page_over_a_table()).expect("open");
    let out = doc.extract_text(0).expect("text");
    assert!(
        out.contains("September 1977"),
        "`Septem-`/`ber` was torn apart by a row-major splice. Got:\n{out}"
    );
}
