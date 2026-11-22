use xberg_native_pdf::document::PdfDocument;

fn page(media_box: &str, crop_box: Option<&str>, content: &str) -> Vec<u8> {
    let content = content.as_bytes().to_vec();
    let crop = crop_box.map(|c| format!(" /CropBox [{c}]")).unwrap_or_default();
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [{media_box}]{crop} /Contents 4 0 R \
             /Resources << /Font << /F1 5 0 R >> >> >>"
        )
        .into_bytes(),
        [
            format!("<< /Length {} >>\nstream\n", content.len()).into_bytes(),
            content,
            b"\nendstream".to_vec(),
        ]
        .concat(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman >>".to_vec(),
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

fn text_of(pdf: Vec<u8>) -> String {
    let doc = PdfDocument::from_bytes(pdf).expect("open");
    doc.extract_text(0).expect("text")
}

/// Three body lines with a line number in the cropped-off margin beside each.
const BODY_WITH_A_RAIL: &str = "BT /F1 10.5 Tf\n\
     54 340 Td (a math department, anyone would be free to tinker with a proof that) Tj\n\
     402 0 Td (17) Tj\n\
     -402 -14.5 Td (someone offered. If you thought you had a better way to prove a theo-) Tj\n\
     402 0 Td (18) Tj\n\
     -402 -14.5 Td (rem, you could take what someone else did and change it. In a classics) Tj\n\
     402 0 Td (19) Tj\n\
     ET\n";

#[test]
fn test_line_number_cropped_off_the_page_does_not_break_the_word_beside_it() {
    let text = text_of(page("0 0 480 678", Some("41.76 41.76 438.24 636.24"), BODY_WITH_A_RAIL));
    assert!(
        text.contains("prove a theorem, you could"),
        "the wrap hyphen must rejoin its word once the cropped margin is gone; got:\n{text}"
    );
    for rail in ["17", "18", "19"] {
        assert!(
            !text.contains(rail),
            "the line number at x=456 lies outside the CropBox and is not on the page; got:\n{text}"
        );
    }
}

#[test]
fn without_a_crop_box_the_whole_medium_is_the_page() {
    let text = text_of(page("0 0 480 678", None, BODY_WITH_A_RAIL));
    assert!(
        text.contains("17") && text.contains("18") && text.contains("19"),
        "with no CropBox the margin is visible and its numbers are text; got:\n{text}"
    );
}

#[test]
fn test_crop_box_that_misses_the_medium_does_not_blank_the_page() {
    let text = text_of(page("0 0 480 678", Some("1000 1000 1200 1200"), BODY_WITH_A_RAIL));
    assert!(
        text.contains("a math department"),
        "a CropBox with no intersection with the MediaBox describes nothing; the medium stands in; got:\n{text}"
    );
}

#[test]
fn test_run_that_straddles_the_crop_edge_is_kept() {
    // The run starts inside the crop box and overhangs the edge at x=438.
    let text = text_of(page(
        "0 0 480 678",
        Some("41.76 41.76 438.24 636.24"),
        "BT /F1 10.5 Tf 400 340 Td (overhang) Tj ET\n",
    ));
    assert!(
        text.contains("overhang"),
        "a partially visible run is on the page and stays; got:\n{text}"
    );
}

#[test]
fn test_media_box_on_the_opposite_diagonal_is_normalised() {
    let doc = PdfDocument::from_bytes(page("612 792 0 0", None, "BT /F1 10 Tf 100 100 Td (x) Tj ET\n")).expect("open");
    assert_eq!(doc.get_page_media_box(0).expect("media box"), (0.0, 0.0, 612.0, 792.0));
}

#[test]
fn test_annotation_text_in_the_cropped_margin_is_not_on_the_page() {
    let objects: Vec<&str> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 480 678] /CropBox [40 40 440 640] /Annots [4 0 R 5 0 R] >>",
        "<< /Type /Annot /Subtype /FreeText /Rect [100 300 200 320] /Contents (inside note) >>",
        "<< /Type /Annot /Subtype /FreeText /Rect [450 300 475 320] /Contents (margin note) >>",
    ];
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref = pdf.len();
    let n = objects.len() + 1;
    pdf.extend_from_slice(format!("xref\n0 {n}\n0000000000 65535 f \n").as_bytes());
    for off in &offsets {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(format!("trailer\n<< /Size {n} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    let text = text_of(pdf);
    assert!(
        text.contains("inside note"),
        "visible annotation text stays; got:\n{text}"
    );
    assert!(
        !text.contains("margin note"),
        "annotation text outside the CropBox is not on the page; got:\n{text}"
    );
}
