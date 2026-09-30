//! Numeric footnotes must stay separate through the public PDF extractor (#1771).
#![cfg(feature = "pdf")]

mod helpers;
use helpers::extract_bytes_document_blocking;
use xberg::{ExtractionConfig, NodeContent, OutputFormat, PdfConfig};

fn build_pdf_with_content(content: &str) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
          /Contents 4 0 R /Resources << /Font << /Helvetica 5 0 R >> >> >>\nendobj\n",
    );

    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len()).as_bytes());
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"\nendstream\nendobj\n");

    offsets.push(pdf.len());
    pdf.extend_from_slice(
        b"5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica \
          /Encoding /WinAnsiEncoding >>\nendobj\n",
    );

    let xref_pos = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for &off in &offsets[1..] {
        pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len(),
            xref_pos
        )
        .as_bytes(),
    );
    pdf
}

#[test]
fn numeric_footnote_survives_public_extraction() {
    let pdf = build_pdf_with_content(
        "BT /Helvetica 12 Tf 72 700 Td (Il successivo comma 3) Tj \
         /Helvetica 8 Tf (5) Tj /Helvetica 12 Tf ( stabilisce la regola.) Tj ET \
         BT /Helvetica 6 Tf 72 100 Td (5) Tj /Helvetica 8 Tf ( A supporting note.) Tj ET",
    );
    let config = ExtractionConfig {
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        pdf_options: Some(PdfConfig {
            extract_tables: false,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = extract_bytes_document_blocking(&pdf, "application/pdf", &config).unwrap();
    assert!(result.content.contains("comma 3 5"), "{}", result.content);
    assert!(!result.content.contains("comma 35"), "{}", result.content);
}

#[test]
fn numeric_script_without_matching_note_keeps_its_join() {
    for note in [
        "",
        "BT /Helvetica 6 Tf 72 100 Td (6) Tj /Helvetica 8 Tf ( A different note.) Tj ET",
    ] {
        let pdf = build_pdf_with_content(&format!(
            "BT /Helvetica 12 Tf 72 700 Td (There are 2) Tj /Helvetica 8 Tf (5) Tj /Helvetica 12 Tf ( combinations.) Tj ET {note}"
        ));
        let config = ExtractionConfig {
            disable_ocr: true,
            use_cache: false,
            enable_quality_processing: false,
            pdf_options: Some(PdfConfig {
                extract_tables: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = extract_bytes_document_blocking(&pdf, "application/pdf", &config).unwrap();
        assert!(result.content.contains("There are 25"), "{}", result.content);
    }
}

fn body_paragraph(result: &xberg::ExtractedDocument) -> &str {
    result
        .document
        .as_ref()
        .expect("requested document structure")
        .nodes
        .iter()
        .find_map(|node| match &node.content {
            NodeContent::Paragraph { text } if text.contains("Il successivo") || text.contains("There are") => {
                Some(text.as_str())
            }
            _ => None,
        })
        .expect("body paragraph")
}

#[test]
fn numeric_footnote_survives_document_structure() {
    for rise in [0, 3] {
        for tables in [false, true] {
            for output_format in [OutputFormat::Plain, OutputFormat::Markdown] {
                let pdf = build_pdf_with_content(&format!(
                    "BT /Helvetica 12 Tf 72 700 Td (Il successivo comma 3) Tj \
                     /Helvetica 8 Tf {rise} Ts (5) Tj 0 Ts /Helvetica 12 Tf ( stabilisce la regola.) Tj ET \
                     BT /Helvetica 6 Tf 72 100 Td (5) Tj /Helvetica 8 Tf ( A supporting note.) Tj ET"
                ));
                let config = ExtractionConfig {
                    disable_ocr: true,
                    use_cache: false,
                    enable_quality_processing: false,
                    include_document_structure: true,
                    output_format,
                    pdf_options: Some(PdfConfig {
                        extract_tables: tables,
                        ..Default::default()
                    }),
                    ..Default::default()
                };
                let result = extract_bytes_document_blocking(&pdf, "application/pdf", &config).unwrap();
                let paragraph = body_paragraph(&result);
                assert!(
                    paragraph.contains("comma 3 5"),
                    "rise={rise}, tables={tables}: {paragraph}"
                );
                assert!(!paragraph.contains("comma 35"), "{paragraph}");
                assert!(!result.content.contains("comma 35"), "{}", result.content);
            }
        }
    }
}

#[test]
fn structured_numeric_scripts_without_note_evidence_keep_their_join() {
    for (note, suffix) in [
        ("", " combinations."),
        (
            "BT /Helvetica 6 Tf 72 100 Td (6) Tj /Helvetica 8 Tf ( A different note.) Tj ET",
            " combinations.",
        ),
        (
            "BT /Helvetica 6 Tf 72 100 Td (5) Tj /Helvetica 8 Tf ( An unrelated note.) Tj ET",
            " + 1 combinations.",
        ),
    ] {
        let pdf = build_pdf_with_content(&format!(
            "BT /Helvetica 12 Tf 72 700 Td (There are 2) Tj /Helvetica 8 Tf (5) Tj /Helvetica 12 Tf ({suffix}) Tj ET {note}"
        ));
        let config = ExtractionConfig {
            disable_ocr: true,
            use_cache: false,
            enable_quality_processing: false,
            include_document_structure: true,
            pdf_options: Some(PdfConfig {
                extract_tables: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = extract_bytes_document_blocking(&pdf, "application/pdf", &config).unwrap();
        let paragraph = body_paragraph(&result);
        assert!(paragraph.contains("There are 25"), "{paragraph}");
    }
}
