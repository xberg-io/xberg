#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::core::config::{PageConfig, PdfBackend, PdfConfig};
use xberg::{ExtractionConfig, OutputFormat};

fn source_bytes() -> Vec<u8> {
    let path = std::env::var_os("XBERG_2376_NOUGAT_SOURCE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| helpers::get_test_documents_dir().join("pdf/nougat_014.pdf"));
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "05015d789a281dc7737a853d8bf468f0bd3002cf899cebd428b0f4d14d4e6817"
    );
    bytes
}

fn assert_painted_exhibit_table(bytes: &[u8], last_identifier: &str) {
    let config = ExtractionConfig {
        use_cache: false,
        disable_ocr: true,
        include_document_structure: true,
        output_format: OutputFormat::Markdown,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            extract_tables: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = helpers::extract_bytes_document_blocking(bytes, "application/pdf", &config).unwrap();
    assert_eq!(result.pages.as_ref().unwrap().len(), 105);
    let p91: Vec<_> = result.tables.iter().filter(|table| table.page_number == 91).collect();
    assert_eq!(p91.len(), 1);
    assert_eq!(
        p91[0].cells.iter().map(|row| row[0].as_str()).collect::<Vec<_>>(),
        [
            "3.1",
            "3.2",
            "4.1",
            "5.1*",
            "10.1",
            "10.2",
            "10.3",
            "10.4",
            "10.5",
            "10.6",
            "10.7",
            "10.8",
            "10.9",
            last_identifier
        ]
    );

    let document = result.document.as_ref().unwrap();
    for expected in ["Wei Y", "A LLP"] {
        assert_eq!(
            document
                .nodes
                .iter()
                .filter(|node| { node.page == Some(92) && node.content.text() == Some(expected) })
                .count(),
            1,
            "page 92 retains {expected}"
        );
    }
    assert!(
        document.nodes.iter().all(|node| {
            node.page != Some(91)
                || node.content.text() != Some("0")
                || !node
                    .bbox
                    .is_some_and(|bbox| bbox.x0 == 32.36532974243164 && bbox.y0 == 392.796875)
        }),
        "page 91 emits the last digit of {last_identifier} only in its table cell"
    );
}

#[test]
fn painted_exhibit_table_owns_the_exact_last_identifier_without_duplicate_prose() {
    assert_painted_exhibit_table(&source_bytes(), "10.10");
}

#[test]
fn painted_exhibit_cell_owns_an_equal_width_decimal_comma_without_duplicate_prose() {
    let mut pdf = lopdf::Document::load_mem(&source_bytes()).expect("pinned source PDF");
    let stream = pdf.get_object_mut((232, 0)).unwrap().as_stream_mut().unwrap();
    let content = String::from_utf8(stream.decompressed_content().unwrap()).expect("ASCII page stream");
    let cell = concat!(
        "1 0 0 -1 8.328125 111175 Tm\n",
        "<0014> Tj\n6.6640625 0 Td <0013> Tj\n6.6640625 0 Td <0011> Tj\n",
        "3.3320313 0 Td <0014> Tj\n6.6640625 0 Td <0013> Tj"
    );
    assert_eq!(content.matches(cell).count(), 1);
    // In the embedded font, the period and comma CIDs have the same advance width. ~keep
    stream.set_plain_content(content.replace(cell, &cell.replace("<0011>", "<000F>")).into_bytes());
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    assert_painted_exhibit_table(&bytes, "10,10");
}
