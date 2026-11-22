//! A title faked into boldface by drawing every glyph twice must be read once.
//!
//! Word processors simulate a heavier weight by painting each glyph a second
//! time a fraction of a point away — here a light grey copy on one baseline and
//! a black copy 0.48 pt above and 0.48 pt to the left, each glyph in its own
//! `BT`/`ET` with its own `Tm`. ISO 32000-1:2008 §9.4.4 (docs/spec/pdf.md:17438)
//! updates the text matrix by the glyph displacement along the writing axis and
//! sets the other component to zero, so a run does not move vertically as it is
//! painted: the two passes are two independent runs that happen to be painted
//! over one another, and nothing in the file says they are one piece of text.
//!
//! Geometry says it instead: two runs that occupy the same horizontal span on
//! the same line cannot both be text a reader sees. Poppler, MuPDF and PDFium
//! all collapse the pair to a single copy.
//!
//! The 0.48 pt vertical offset is load-bearing. Span assembly rounds the
//! baseline to whole points, so the two passes land in different rounded rows
//! and each is folded into a whole word; the row assignment that runs
//! afterwards then puts both words back on one row and orders it by left edge.
//! That is why the damage is doubled *words* — `CHECKLISTCHECKLIST` — rather
//! than doubled letters.
//!
//! The font carries an explicit `/Widths` array so each glyph's advance is the
//! one the placement uses; without it the fallback-width correction inflates
//! every gap and scatters spaces through the word.

use xberg_native_pdf::PdfDocument;

/// Helvetica advance widths, in thousandths of an em. The same table fills the
/// font's `/Widths` array and drives the glyph placement, so the extracted
/// geometry is exactly the geometry the file declares.
fn advance_mille(c: char) -> u32 {
    match c {
        'C' | 'H' => 722,
        'E' | 'K' | 'S' => 667,
        'T' => 611,
        'L' => 556,
        'I' => 278,
        ' ' => 278,
        _ => 556,
    }
}

fn widths_array() -> String {
    (32u8..=122)
        .map(|c| advance_mille(c as char).to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_pdf(content: Vec<u8>) -> Vec<u8> {
    let font = format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
         /FirstChar 32 /LastChar 122 /Widths [{}] >>",
        widths_array()
    );
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
        font.into_bytes(),
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

/// Draw `word` glyph by glyph at `size`, starting at `x0` on `baseline`. When
/// `double_struck`, each glyph is painted twice — a grey pass on the baseline
/// and a black pass 0.48 pt up and 0.48 pt left, which is how a word processor
/// emits simulated bold.
fn title_run(word: &str, size: f32, x0: f32, baseline: f32, double_struck: bool) -> String {
    let mut out = String::new();
    let mut x = x0;
    for c in word.chars() {
        out.push_str(&format!(
            "BT /F1 {size} Tf 1 0 0 1 {x:.2} {baseline:.2} Tm 0.753 g ({c}) Tj ET\n"
        ));
        if double_struck {
            out.push_str(&format!(
                "BT /F1 {size} Tf 1 0 0 1 {:.2} {:.2} Tm 0 g ({c}) Tj ET\n",
                x - 0.48,
                baseline + 0.48,
            ));
        }
        x += advance_mille(c) as f32 / 1000.0 * size;
    }
    out
}

/// Two lines of body text below the title, so the page is a document rather
/// than a lone heading and the converters take their ordinary flow path.
fn body_text(top_baseline: f32) -> String {
    [
        "The first line of body text under the heading.",
        "The second line of body text under the heading.",
    ]
    .iter()
    .enumerate()
    .map(|(i, line)| {
        format!(
            "BT /F1 10 Tf 1 0 0 1 72 {:.2} Tm 0 g ({line}) Tj ET\n",
            top_baseline - 14.0 * i as f32
        )
    })
    .collect()
}

fn surfaces(pdf: &[u8]) -> Vec<(&'static str, String)> {
    let doc = PdfDocument::from_bytes(pdf.to_vec()).expect("open");
    let spans = doc.extract_spans(0).expect("spans");
    let joined = spans.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
    vec![("text", doc.extract_text(0).expect("text")), ("spans", joined)]
}

#[test]
fn test_double_struck_title_reads_once_not_twice() {
    // The real geometry: 11.04 pt glyphs, the grey pass on baseline 785.40 and
    // the black pass on 785.88.
    let mut content = title_run("CHECKLIST", 11.04, 170.06, 785.40, true);
    content.push_str(&body_text(745.40));
    let pdf = build_pdf(content.into_bytes());

    for (surface, out) in surfaces(&pdf) {
        assert!(
            !out.contains("CHECKLISTCHECKLIST"),
            "{surface}: a glyph drawn twice for weight must be read once, not \
             doubled into two words; got:\n{out}"
        );
        assert_eq!(
            out.matches("CHECKLIST").count(),
            1,
            "{surface}: the title itself must survive the collapse, exactly once; got:\n{out}"
        );
    }
}

/// The counter-case that stops the fix from being "drop any repeated word".
/// Two copies of the same word set a line apart are two real lines of text and
/// both must survive.
#[test]
fn test_same_word_on_two_real_lines_keeps_both_copies() {
    let mut content = title_run("CHECKLIST", 11.04, 170.0, 700.0, false);
    content.push_str(&title_run("CHECKLIST", 11.04, 170.0, 686.0, false));
    let pdf = build_pdf(content.into_bytes());

    let doc = PdfDocument::from_bytes(pdf).expect("open");
    let text = doc.extract_text(0).expect("text");
    assert_eq!(
        text.matches("CHECKLIST").count(),
        2,
        "two copies a line apart are two lines of text; got:\n{text}"
    );
}
