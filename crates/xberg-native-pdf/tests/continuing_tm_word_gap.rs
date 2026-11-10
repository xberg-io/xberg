//! A producer that writes `Tm /F Tf (word) Tj` for each word continues the run at every `Tm`. When an
//! operator ends that run before the next glyph, the run must end at its last glyph, so that the word
//! gap stays visible to the span merge (#2675).

use xberg_native_pdf::document::PdfDocument;

const SIZE: f32 = 12.0;
const GLYPH: f32 = 7.2;

/// `A` at x = 100 and `B` one word gap after its advance, each with its own `Tm`; `between` sits after
/// the second `Tm` and before `(B) Tj`.
fn text(between: &str, gap_em: f32) -> String {
    let x_b = 100.0 + GLYPH + gap_em * SIZE;
    let content = format!(
        "BT /F1 {SIZE} Tf\n1 0 0 1 100 500 Tm 100 Tz (A) Tj\n1 0 0 1 {x_b} 500 Tm {between}(B) Tj\nET"
    );
    let doc = PdfDocument::from_bytes(build(content.as_bytes())).expect("open fixture");
    doc.extract_text(0).expect("extract_text").trim().to_string()
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

/// One page with two Helvetica-metric fonts, `F1` and `F2`, whose `A` and `B` are 600/1000 em wide.
fn build(content: &[u8]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];
    let font = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding \
                /FirstChar 65 /LastChar 66 /Widths [600 600] >>";
    let objs = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
         /Resources << /Font << /F1 5 0 R /F2 6 0 R >> >> >>"
            .to_string(),
        String::new(),
        font.to_string(),
        font.replace("Helvetica", "Times-Roman"),
    ];
    for (i, body) in objs.iter().enumerate() {
        offsets.push(pdf.len());
        if i == 3 {
            pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
            pdf.extend_from_slice(content);
            pdf.extend_from_slice(b"\nendstream\nendobj\n");
        } else {
            pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
        }
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", offsets.len()).as_bytes());
    for off in &offsets[1..] {
        pdf.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len()).as_bytes(),
    );
    pdf
}
