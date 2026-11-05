#![cfg(feature = "pdf")]

mod helpers;

use helpers::{extract_bytes_document_blocking, get_test_file_path};
use sha2::{Digest, Sha256};
use xberg::core::config::{ExtractionConfig, OutputFormat, PageConfig};

#[test]
fn should_keep_form_values_and_file_headers_in_source_order_with_forcebold() {
    let path = get_test_file_path("pdf/pdfa_042.pdf");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("required public corpus PDF {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "517f4da2af87e1043e3be4e40c8523d671d361f79a2937fc00210b7bb9d7779b",
        "public source PDF identity"
    );
    let config = ExtractionConfig {
        output_format: OutputFormat::Plain,
        disable_ocr: true,
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    let pages = document.pages.expect("requested page extraction");
    let page = pages.iter().find(|page| page.page_number == 5).expect("page 5");
    let content = page.content.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(content.contains("Client Sample ID: CAMS 519"), "{content}");
    assert!(content.contains("File ID DF"), "{content}");
    let (before_footer, _) = content.split_once("RL = Reporting Limit").expect("source footer");
    for value in [
        "CAMS 519",
        "JD3794-1",
        "AIR - Ambient Air Comp. Summa ID: A990",
        "TO-15",
        "Valero-CAMS, Baton Rouge, LA",
    ] {
        assert!(before_footer.contains(value), "{value:?} appears below the footer");
    }
    assert_eq!(page.content.split_whitespace().count(), 505);
}
