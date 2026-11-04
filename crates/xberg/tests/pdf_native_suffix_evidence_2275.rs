#![cfg(feature = "pdf")]

mod helpers;

use helpers::extract_bytes_document_blocking;
use xberg::core::config::{ExtractionConfig, OutputFormat, PdfBackend, PdfConfig};
use xberg::types::document_structure::AnnotationKind;

fn suffix_pdf() -> Vec<u8> {
    let stream = concat!(
        "BT /F1 12 Tf 1 0 0 1 100 700 Tm (Author) Tj ET\n",
        "BT /F1 8 Tf 1 0 0 1 135.352 704.5 Tm (1) Tj ET\n",
        "BT /F1 12 Tf 1 0 0 1 100 630 Tm (Independent paragraph.) Tj ET\n",
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_owned(),
        format!("<< /Length {} >>\nstream\n{stream}endstream", stream.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    bytes
}

fn assert_native_suffix(extract_tables: bool) {
    let bytes = suffix_pdf();
    assert!(bytes.starts_with(b"%PDF-1.4"));
    let config = ExtractionConfig {
        output_format: OutputFormat::Markdown,
        include_document_structure: true,
        disable_ocr: true,
        use_cache: false,
        enable_quality_processing: false,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result =
        extract_bytes_document_blocking(&bytes, "application/pdf", &config).expect("self-contained native PDF");
    assert_eq!(result.tables.len(), 0);
    let document = result.document.expect("requested document structure");
    let nodes: Vec<_> = document
        .nodes
        .iter()
        .filter(|node| node.content.text() == Some("Author1"))
        .collect();
    assert_eq!(
        nodes.len(),
        1,
        "author and raised suffix must share one structured node"
    );
    let scripts: Vec<_> = nodes[0]
        .annotations
        .iter()
        .filter(|annotation| matches!(annotation.kind, AnnotationKind::Superscript))
        .collect();
    assert_eq!(scripts.len(), 1);
    assert_eq!((scripts[0].start, scripts[0].end), (6, 7));
    assert_eq!(
        document
            .nodes
            .iter()
            .filter(|node| node.content.text() == Some("Independent paragraph."))
            .count(),
        1
    );
}

#[test]
fn should_preserve_native_suffix_in_direct_structure_extraction() {
    assert_native_suffix(false);
}

#[test]
fn should_preserve_native_suffix_in_preserved_hierarchy_after_table_extraction() {
    assert_native_suffix(true);
}
