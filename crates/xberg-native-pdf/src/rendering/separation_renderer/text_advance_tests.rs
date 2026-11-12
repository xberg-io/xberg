//! Skipped text on a separation plate still advances the text matrix by the rasterizer's measure.
//! Each test shows a glyph that paints nothing, then a visible one, and requires the plate to equal
//! the plate of the visible glyph placed at the advance by hand. It runs through the walker, so it
//! fails when either call site of `measure_text_advance` stops using it.

use super::*;

/// One glyph, `H`, 1000 units wide: at 20 pt and Tz 50 the advance is 10 pt.
const FONT: &str = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
    /FirstChar 72 /LastChar 72 /Widths [1000] >>";

fn plate_of(content: &str, ext_gstate: &str) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut offs: Vec<usize> = Vec::new();
    let mut push = |buf: &mut Vec<u8>, body: &str| {
        offs.push(buf.len());
        buf.extend_from_slice(body.as_bytes());
    };
    push(&mut buf, "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    push(&mut buf, "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    push(
        &mut buf,
        &format!(
            "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> /ExtGState << /T << {ext_gstate} >> >> >> >>\nendobj\n"
        ),
    );
    push(
        &mut buf,
        &format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n",
            content.len()
        ),
    );
    push(&mut buf, &format!("5 0 obj\n{FONT}\nendobj\n"));
    let xref_off = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offs.len() + 1).as_bytes());
    for off in &offs {
        buf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n",
            offs.len() + 1
        )
        .as_bytes(),
    );
    let doc = PdfDocument::from_bytes(buf).expect("synthetic PDF parses");
    let inks = vec!["Cyan".to_string()];
    let plates = render_plates_for_inks(&doc, 0, 72, &inks, &inks).expect("plate renders");
    assert_eq!(plates.len(), 1);
    plates.into_iter().next().unwrap().data
}

fn inked_columns(data: &[u8]) -> Vec<usize> {
    let mut columns: Vec<usize> = data
        .iter()
        .enumerate()
        .filter(|&(_, &v)| v > 0)
        .map(|(i, _)| i % 100)
        .collect();
    columns.sort_unstable();
    columns.dedup();
    columns
}

/// The reference: the visible glyph alone, placed where the skipped one's advance puts it.
fn reference() -> Vec<u8> {
    let plate = plate_of("1 0 0 0 k BT /F1 20 Tf 50 Tz 20 10 Td (H) Tj ET", "");
    assert!(
        !inked_columns(&plate).is_empty(),
        "the reference glyph must ink the plate"
    );
    plate
}

#[test]
fn invisible_text_advances_the_plate_by_the_rasterizer_measure() {
    let plate = plate_of("1 0 0 0 k BT /F1 20 Tf 50 Tz 10 10 Td 3 Tr (H) Tj 0 Tr (H) Tj ET", "");
    assert_eq!(plate, reference());
}

#[test]
fn unpainted_text_advances_the_plate_by_the_rasterizer_measure() {
    // With overprint and OPM 1 a zero Cyan component leaves the Cyan plate alone, so the first
    // glyph reaches no plate; the second, in nonzero Cyan, paints.
    let plate = plate_of(
        "/T gs 0 0 0 0 k BT /F1 20 Tf 50 Tz 10 10 Td (H) Tj 1 0 0 0 k (H) Tj ET",
        "/op true /OP true /OPM 1",
    );
    assert_eq!(plate, reference());
}
