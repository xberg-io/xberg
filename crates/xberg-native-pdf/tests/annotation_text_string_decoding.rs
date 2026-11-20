//! Annotation `/Contents`, `/T`, and `/Subj` are PDF text strings
//! (ISO 32000-1 §7.9.2.2): PDFDocEncoding, UTF-16BE with a `FE FF` BOM, or
//! UTF-8 with an `EF BB BF` BOM in PDF 2.0. They must not be decoded as UTF-8,
//! which leaves U+0000 between UTF-16BE characters and U+FFFD for
//! PDFDocEncoding bytes at or above 0x80.

use xberg_native_pdf::document::PdfDocument;

/// Assemble a PDF with a correct xref from raw object bodies.
/// `objects[i]` is the body of object i+1 (no "N 0 obj"/"endobj" wrapper).
fn build_pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref_pos = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_pos
        )
        .as_bytes(),
    );
    out
}

fn with_annotations(annotations: &[&str]) -> Vec<u8> {
    let annots = annotations
        .iter()
        .enumerate()
        .map(|(i, _)| format!("{} 0 R", i + 5))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [{annots}] >>").into_bytes(),
        b"<< /Length 0 >>\nstream\n\nendstream".to_vec(),
    ];
    objects.extend(annotations.iter().map(|body| body.as_bytes().to_vec()));
    build_pdf(&objects)
}

#[test]
fn annotation_utf16be_contents_author_and_subject_decode() {
    let pdf = with_annotations(&["<< /Type /Annot /Subtype /Text /Rect [10 10 40 40] \
         /Contents <FEFF0031002E00200041> /T <FEFF004A00F60068006E> /Subj <FEFF00520065007600690065> >>"]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse PDF");
    let annotations = doc.get_annotations(0).expect("read annotations");

    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations[0].contents.as_deref(), Some("1. A"));
    assert_eq!(annotations[0].author.as_deref(), Some("J\u{00F6}hn"));
    assert_eq!(annotations[0].subject.as_deref(), Some("Revie"));
}

#[test]
fn annotation_pdfdocencoding_contents_strips_trailing_nul() {
    // "What’s New" with 0x90 (right single quotation mark) and a producer's
    // trailing NUL terminator.
    let pdf =
        with_annotations(&["<< /Type /Annot /Subtype /Link /Rect [10 10 40 40] /Contents <576861749073204E657700> >>"]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse PDF");
    let annotations = doc.get_annotations(0).expect("read annotations");

    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations[0].contents.as_deref(), Some("What\u{2019}s New"));
}

#[test]
fn annotation_utf8_bom_contents_decode() {
    let pdf = with_annotations(&[
        "<< /Type /Annot /Subtype /Text /Rect [10 10 40 40] /Contents <EFBBBF57686174E2809973204E6577> >>",
    ]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse PDF");
    let annotations = doc.get_annotations(0).expect("read annotations");

    assert_eq!(annotations[0].contents.as_deref(), Some("What\u{2019}s New"));
}

#[test]
fn annotation_plain_ascii_contents_decode() {
    let pdf = with_annotations(&["<< /Type /Annot /Subtype /Text /Rect [10 10 40 40] /Contents (Plain ASCII) >>"]);
    let doc = PdfDocument::from_bytes(pdf).expect("parse PDF");
    let annotations = doc.get_annotations(0).expect("read annotations");

    assert_eq!(annotations[0].contents.as_deref(), Some("Plain ASCII"));
}
