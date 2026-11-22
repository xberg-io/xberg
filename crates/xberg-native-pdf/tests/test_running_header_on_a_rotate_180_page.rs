//! A running header is recorded from the spans as the content stream wrote
//! them, and matched against spans after `/Rotate` has mapped them into the
//! displayed frame. On a 180 degree page a header away from the page centre
//! sits in two different places in those two frames, so the position check
//! never agreed and the header stayed in the body.

use xberg_native_pdf::PdfDocument;

fn pdf(rotate: i32, page_count: usize) -> Vec<u8> {
    let mut buf: Vec<u8> = b"%PDF-1.7\n".to_vec();
    let mut off = vec![0usize; 4 + page_count * 2];
    fn obj(buf: &mut Vec<u8>, off: &mut [usize], id: usize, body: &str) {
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    obj(&mut buf, &mut off, 1, "<< /Type /Catalog /Pages 2 0 R >>");
    let kids: Vec<String> = (0..page_count).map(|i| format!("{} 0 R", 5 + i * 2)).collect();
    obj(
        &mut buf,
        &mut off,
        2,
        &format!("<< /Type /Pages /Kids [{}] /Count {page_count} >>", kids.join(" ")),
    );
    obj(
        &mut buf,
        &mut off,
        3,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    for i in 0..page_count {
        let content = format!(
            "BT /F1 10 Tf 1 0 0 1 72 750 Tm (Annual Report {}) Tj ET\n\
             BT /F1 12 Tf 1 0 0 1 72 400 Tm (Body text placeholder) Tj ET\n",
            i + 1
        );
        let content_id = 4 + i * 2;
        let id = content_id;
        off[id] = buf.len();
        buf.extend_from_slice(format!("{id} 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
        buf.extend_from_slice(content.as_bytes());
        buf.extend_from_slice(b"\nendstream\nendobj\n");
        obj(
            &mut buf,
            &mut off,
            content_id + 1,
            &format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate {rotate} \
                 /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>"
            ),
        );
    }
    let xref = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", off.len()).as_bytes());
    for offset in &off[1..] {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            off.len()
        )
        .as_bytes(),
    );
    buf
}

fn header_is_artifact(rotate: i32, page: usize) -> bool {
    let doc = PdfDocument::from_bytes(pdf(rotate, 4)).expect("parse");
    let spans = doc.extract_spans(page).expect("spans");
    spans
        .iter()
        .find(|s| s.text.contains("Annual Report"))
        .unwrap_or_else(|| panic!("no header span on page {page}"))
        .artifact_type
        .is_some()
}

#[test]
fn test_running_header_is_an_artifact_on_an_unrotated_page() {
    assert!(header_is_artifact(0, 2), "control: the upright header is an artifact");
}

#[test]
fn test_running_header_is_an_artifact_on_a_rotate_180_page() {
    assert!(header_is_artifact(180, 2));
}
