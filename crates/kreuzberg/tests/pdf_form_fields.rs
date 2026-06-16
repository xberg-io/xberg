#![cfg(feature = "pdf")]

use kreuzberg::{ExtractionConfig, extract_bytes_sync};

/// Build a minimal PDF whose form field values live ONLY in Widget annotations.
///
/// Page content stream: labels "Name:" and "Email:" as plain text.
/// AcroForm: two Widget annotations with `/V (John Smith)` and `/V (john@example.com)`.
/// The values are intentionally absent from the content stream to replicate the
/// interactive (non-flattened) PDF pattern that issue #1120 reports.
fn make_interactive_form_pdf() -> Vec<u8> {
    // Page content: field labels only (no values)
    let content_stream = b"BT /Helvetica 12 Tf 72 700 Td (Name:) Tj 0 -30 Td (Email:) Tj ET";

    let mut pdf: Vec<u8> = Vec::new();

    macro_rules! push_bytes {
        ($s:expr) => {
            pdf.extend_from_slice($s)
        };
    }
    macro_rules! push_str {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }

    push_bytes!(b"%PDF-1.4\n");

    let off1 = pdf.len();
    push_bytes!(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 5 0 R >>\nendobj\n");

    let off2 = pdf.len();
    push_bytes!(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    // Obj 4: content stream (defined before page obj so length is known)
    let off4 = pdf.len();
    push_str!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content_stream.len()));
    push_bytes!(content_stream);
    push_bytes!(b"\nendstream\nendobj\n");

    // Obj 3: page — references content + both Widget annotations
    let off3 = pdf.len();
    push_bytes!(
        b"3 0 obj\n\
         << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]\n\
            /Contents 4 0 R\n\
            /Resources << /Font << /Helvetica 8 0 R >> >>\n\
            /Annots [6 0 R 7 0 R] >>\n\
         endobj\n"
    );

    // Obj 5: AcroForm
    let off5 = pdf.len();
    push_bytes!(
        b"5 0 obj\n\
         << /Type /AcroForm /Fields [6 0 R 7 0 R]\n\
            /DA (/Helvetica 12 Tf 0 g) >>\n\
         endobj\n"
    );

    // Obj 6: Widget — "name" field, value "John Smith", at y=680..700 (PDF coords)
    let off6 = pdf.len();
    push_bytes!(
        b"6 0 obj\n\
         << /Type /Annot /Subtype /Widget /FT /Tx\n\
            /T (name) /V (John Smith)\n\
            /Rect [140 680 400 700] /P 3 0 R\n\
            /DA (/Helvetica 12 Tf 0 g) >>\n\
         endobj\n"
    );

    // Obj 7: Widget — "email" field, value "john@example.com", at y=650..670
    let off7 = pdf.len();
    push_bytes!(
        b"7 0 obj\n\
         << /Type /Annot /Subtype /Widget /FT /Tx\n\
            /T (email) /V (john@example.com)\n\
            /Rect [140 650 400 670] /P 3 0 R\n\
            /DA (/Helvetica 12 Tf 0 g) >>\n\
         endobj\n"
    );

    // Obj 8: Font
    let off8 = pdf.len();
    push_bytes!(
        b"8 0 obj\n\
         << /Type /Font /Subtype /Type1 /BaseFont /Helvetica\n\
            /Encoding /WinAnsiEncoding >>\n\
         endobj\n"
    );

    let xref_off = pdf.len();
    push_str!(format!(
        "xref\n0 9\n\
         0000000000 65535 f \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n",
        off1, off2, off3, off4, off5, off6, off7, off8
    ));
    push_str!(format!(
        "trailer\n<< /Size 9 /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n"
    ));

    pdf
}

/// Build a PDF where the form value is ALSO written into the content stream (flattened).
///
/// This replicates PDFs that have been "flattened" — the form field appearance has been
/// rendered into the page content stream AND the Widget annotation still exists with /V.
/// After the fix, the value must appear exactly once (not duplicated).
fn make_flattened_form_pdf() -> Vec<u8> {
    // Content stream includes the value "Jane Doe" alongside the label
    let content_stream = b"BT /Helvetica 12 Tf 72 700 Td (Name: Jane Doe) Tj ET";

    let mut pdf: Vec<u8> = Vec::new();

    macro_rules! push_bytes {
        ($s:expr) => {
            pdf.extend_from_slice($s)
        };
    }
    macro_rules! push_str {
        ($s:expr) => {
            pdf.extend_from_slice($s.as_bytes())
        };
    }

    push_bytes!(b"%PDF-1.4\n");

    let off1 = pdf.len();
    push_bytes!(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R /AcroForm 5 0 R >>\nendobj\n");

    let off2 = pdf.len();
    push_bytes!(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    let off4 = pdf.len();
    push_str!(format!("4 0 obj\n<< /Length {} >>\nstream\n", content_stream.len()));
    push_bytes!(content_stream);
    push_bytes!(b"\nendstream\nendobj\n");

    let off3 = pdf.len();
    push_bytes!(
        b"3 0 obj\n\
         << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]\n\
            /Contents 4 0 R\n\
            /Resources << /Font << /Helvetica 8 0 R >> >>\n\
            /Annots [6 0 R] >>\n\
         endobj\n"
    );

    let off5 = pdf.len();
    push_bytes!(b"5 0 obj\n<< /Type /AcroForm /Fields [6 0 R] /DA (/Helvetica 12 Tf 0 g) >>\nendobj\n");

    // Widget with the same value as in the content stream
    let off6 = pdf.len();
    push_bytes!(
        b"6 0 obj\n\
         << /Type /Annot /Subtype /Widget /FT /Tx\n\
            /T (name) /V (Jane Doe)\n\
            /Rect [140 680 400 700] /P 3 0 R\n\
            /DA (/Helvetica 12 Tf 0 g) >>\n\
         endobj\n"
    );

    let off7 = pdf.len();
    push_bytes!(
        b"7 0 obj\n\
         << /Type /Font /Subtype /Type1 /BaseFont /Helvetica\n\
            /Encoding /WinAnsiEncoding >>\n\
         endobj\n"
    );

    let xref_off = pdf.len();
    push_str!(format!(
        "xref\n0 8\n\
         0000000000 65535 f \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n\
         {:010} 00000 n \r\n",
        off1, off2, off3, off4, off5, off6, off7
    ));
    push_str!(format!(
        "trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF\n"
    ));

    pdf
}

/// Widget field values absent from the content stream must appear in the extracted text.
///
/// Regression for issue #1120: interactive PDFs store form values only in Widget
/// `/V` entries; they were previously absent from kreuzberg's extraction output.
#[test]
fn test_interactive_form_field_values_extracted() {
    let pdf = make_interactive_form_pdf();
    let config = ExtractionConfig::default();
    let result =
        extract_bytes_sync(&pdf, "application/pdf", &config).expect("interactive form PDF must extract without error");

    let content = &result.content;
    assert!(
        content.contains("John Smith"),
        "extracted text must include Widget field value 'John Smith'; got: {content:?}"
    );
    assert!(
        content.contains("john@example.com"),
        "extracted text must include Widget field value 'john@example.com'; got: {content:?}"
    );
}

/// Field labels in the content stream must still be present alongside field values.
#[test]
fn test_content_stream_labels_preserved_alongside_field_values() {
    let pdf = make_interactive_form_pdf();
    let config = ExtractionConfig::default();
    let result =
        extract_bytes_sync(&pdf, "application/pdf", &config).expect("interactive form PDF must extract without error");

    let content = &result.content;
    assert!(
        content.contains("Name:"),
        "label 'Name:' from content stream must be preserved; got: {content:?}"
    );
    assert!(
        content.contains("Email:"),
        "label 'Email:' from content stream must be preserved; got: {content:?}"
    );
}

/// Widget values already present in the content stream (flattened PDFs) must not be duplicated.
///
/// When a PDF is flattened, the form appearance is rendered into the content stream AND
/// the Widget annotation still carries `/V`. The value must appear exactly once.
#[test]
fn test_flattened_form_value_not_duplicated() {
    let pdf = make_flattened_form_pdf();
    let config = ExtractionConfig::default();
    let result =
        extract_bytes_sync(&pdf, "application/pdf", &config).expect("flattened form PDF must extract without error");

    let content = &result.content;
    let count = content.matches("Jane Doe").count();
    assert_eq!(
        count, 1,
        "flattened Widget value 'Jane Doe' must appear exactly once, not duplicated; got: {content:?}"
    );
}
