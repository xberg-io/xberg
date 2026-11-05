use xberg_native_pdf::PdfDocument;

fn assemble_single_page_pdf(stream: &str) -> Vec<u8> {
    let mut pdf: Vec<u8> = Vec::new();

    macro_rules! push {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }

    push!("%PDF-1.4\n");

    let off1 = pdf.len();
    push!("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    let off2 = pdf.len();
    push!("2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off3 = pdf.len();
    push!(
        "3 0 obj\n\
         << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]\
         \n   /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\n\
         endobj\n"
    );

    let off4 = pdf.len();
    let stream_bytes = stream.as_bytes();
    push!(format!("4 0 obj\n<< /Length {} >>\nstream\n", stream_bytes.len()));
    pdf.extend_from_slice(stream_bytes);
    push!("\nendstream\nendobj\n");

    let off5 = pdf.len();
    push!("5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n");

    let xref_off = pdf.len();
    push!(format!(
        "xref\n0 6\n\
         0000000000 65535 f \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n",
        off1, off2, off3, off4, off5
    ));
    push!(format!(
        "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n"
    ));

    pdf
}

#[test]
fn adjacent_source_glyphs_do_not_gain_a_word_boundary_from_rounding() {
    let mut stream = String::new();
    for (character, x, y) in [
        ('s', 71.12, 374.57825),
        ('t', 76.12, 374.83823),
        ('u', 78.9, 374.9828),
        ('r', 84.46001, 375.2719),
        ('e', 87.79, 375.44507),
    ] {
        stream.push_str(&format!("BT /F1 10 Tf 1 0 0 1 {x} {y} Tm ({character}) Tj ET\n"));
    }
    let document = PdfDocument::from_bytes(assemble_single_page_pdf(&stream)).expect("parse adjacent source glyphs");
    let chars = document.extract_chars(0).expect("decode source advances");
    let previous = chars
        .iter()
        .find(|character| character.char == 'r')
        .expect("previous glyph");
    let next = chars
        .iter()
        .find(|character| character.char == 'e')
        .expect("following glyph");
    let gap = next.origin_x - (previous.origin_x + previous.advance_width);
    assert!(gap < 0.0 && gap.abs() < 0.00002, "one rounding unit of overlap: {gap}");
    let mut spans = document.extract_spans(0).expect("extract native spans");
    spans.sort_by(|a, b| a.bbox.x.total_cmp(&b.bbox.x));
    assert_eq!(spans.iter().map(|span| span.text.as_str()).collect::<String>(), "sture");
}
