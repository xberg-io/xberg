use xberg_native_pdf::document::PdfDocument;

fn damaged_pdf(objects: &[(u64, &str)]) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    for (num, body) in objects {
        buf.extend_from_slice(format!("{num} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    buf.extend_from_slice(b"startxref\n999999999\n%%EOF\n");
    buf
}

#[test]
fn test_object_numbered_at_the_top_of_the_range_does_not_panic() {
    let pdf = damaged_pdf(&[
        (3, "<< /Type /Page /MediaBox [0 0 200 200] /Contents 4 0 R >>"),
        (4, "<< /Length 0 >>\nstream\n\nendstream"),
        (4294967295, "<< /Type /Whatever >>"),
    ]);
    if let Ok(doc) = PdfDocument::from_bytes(pdf) {
        let _ = doc.page_count();
    }
}

#[test]
fn test_ordinary_damaged_file_still_recovers_its_pages() {
    let pdf = damaged_pdf(&[
        (3, "<< /Type /Page /MediaBox [0 0 200 200] /Contents 4 0 R >>"),
        (4, "<< /Length 0 >>\nstream\n\nendstream"),
    ]);
    let doc = PdfDocument::from_bytes(pdf).expect("a recoverable file should open");
    assert_eq!(doc.page_count().expect("page count"), 1);
    assert_eq!(doc.get_page_media_box(0).expect("media box"), (0.0, 0.0, 200.0, 200.0));
}
