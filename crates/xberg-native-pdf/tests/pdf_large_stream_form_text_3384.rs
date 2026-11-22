//! Text painted inside a Form XObject is lost when the page content stream is
//! large enough to take the prescan path.
//!
//! Over 256 KiB, `parse_and_execute_text_only` locates text-bearing regions and
//! reparses only those. A region for a `Do` starts at the operator, so its
//! `/Name` operand — which sits before it — is outside the region; the region
//! then parses as a bare `Do` with no name, the Form XObject is never
//! executed, and the page extracts empty. Pages like these are vector art with
//! an invisible OCR text layer in a form (PDF24 Tools, arXiv 2308.04470):
//! `q q 1 0 0 1 0 0.12 cm /OL2 Do Q Q` behind megabytes of glyph outlines
//! (xberg-io/xberg#3384).

use xberg_native_pdf::converters::ConversionOptions;
use xberg_native_pdf::document::PdfDocument;

const FORM_TEXT: &str = "Hello world test sentence";

/// One page whose content stream is `padding` plus `q /X1 Do Q`, where X1 is a
/// form that paints [`FORM_TEXT`].
fn pdf_with_padded_form(padding: usize) -> Vec<u8> {
    let font = "6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n";
    let form_stream = format!("BT /F1 12 Tf 72 720 Td ({FORM_TEXT}) Tj ET");
    let form = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 612 792]\n\
         /Resources << /Font << /F1 6 0 R >> >>\n\
         /Length {} >>\nstream\n{}\nendstream\nendobj\n",
        form_stream.len(),
        form_stream
    );

    let mut content = Vec::with_capacity(padding + 32);
    while content.len() < padding {
        content.extend_from_slice(b"q 1 0 0 1 0 0 cm Q\n");
    }
    content.extend_from_slice(b"q /X1 Do Q\n");

    let page = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]\n\
                /Contents 4 0 R\n\
                /Resources << /Font << /F1 6 0 R >> /XObject << /X1 5 0 R >> >>\n\
                >>\nendobj\n";

    let mut pdf = Vec::new();
    let mut offsets = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(page.as_bytes());
    offsets.push(pdf.len());
    let header = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    pdf.extend_from_slice(header.as_bytes());
    pdf.extend_from_slice(&content);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(pdf.len());
    pdf.extend_from_slice(form.as_bytes());
    offsets.push(pdf.len());
    pdf.extend_from_slice(font.as_bytes());

    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in &offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len() + 1,
            xref
        )
        .as_bytes(),
    );
    pdf
}

fn text_of(pdf: Vec<u8>) -> String {
    let document = PdfDocument::from_bytes(pdf).expect("the fixture parses");
    document
        .extract_text_with_options(
            0,
            &ConversionOptions {
                extract_tables: false,
                ..Default::default()
            },
        )
        .expect("extraction succeeds")
}

#[test]
fn a_large_stream_prescan_keeps_text_painted_by_a_form_xobject() {
    let pdf = pdf_with_padded_form(285 * 1024);
    assert!(
        pdf.len() > 256 * 1024,
        "the fixture must exceed the prescan threshold"
    );

    let text = text_of(pdf);
    assert!(
        text.contains(FORM_TEXT),
        "the form's text is missing from the prescan path, got {text:?}"
    );
}

#[test]
fn the_same_page_below_the_prescan_threshold_still_extracts_its_form_text() {
    // Control: the sequential parser reaches the form through `/X1 Do`.
    let text = text_of(pdf_with_padded_form(1024));

    assert!(
        text.contains(FORM_TEXT),
        "the sequential path regressed, got {text:?}"
    );
}
