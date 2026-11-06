#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, PageConfig};
use xberg_native_pdf::{PdfDocument, rendering};

const MATCHING: &[u8] = include_bytes!("fixtures/pdf_khmer_number_2408/matching.pdf");
const MISLEADING: &[u8] = include_bytes!("fixtures/pdf_khmer_number_2408/misleading.pdf");
const PROSE: [&str; 6] = [
    "The office keeps the current monthly service records.",
    "Readers can compare each line with its source record.",
    "The committee will meet on Monday to discuss the schedule",
    "Please retain the signed copy for your own records.",
    "The annual report is available at the information desk.",
    "The public document includes the relevant details.",
];

fn extract(input: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let result = helpers::extract_bytes_document_blocking(input, "application/pdf", config).unwrap();
    assert_eq!(result.pages.as_ref().unwrap().len(), 1);
    result
}

fn rendered(input: &[u8]) -> Vec<u8> {
    let doc = PdfDocument::from_bytes(input.to_vec()).unwrap();
    rendering::render_page(&doc, 0, &rendering::RenderOptions::with_dpi(150))
        .unwrap()
        .data
}

#[test]
fn preserves_the_drawn_khmer_number_but_rejects_a_misleading_layer() {
    let model_dir = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(model_dir.path().join("eng.traineddata"), english).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            tessdata_path: Some(model_dir.path().to_owned()),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let native = ExtractionConfig {
        disable_ocr: true,
        ..config.clone()
    };
    assert!(rendered(MATCHING) == rendered(MISLEADING), "same painted raster");
    assert_eq!(extract(MATCHING, &native).content.trim(), "ប០ប");
    assert_eq!(extract(MISLEADING, &native).content.trim(), "ប១ប");

    let misleading = extract(MISLEADING, &config);
    assert!(misleading.metadata.ocr_used);
    for text in [&misleading.content, &misleading.pages.as_ref().unwrap()[0].content] {
        assert!(text.lines().any(|line| line.trim() == "VOU"), "{text}");
        assert!(!text.contains('១'), "the scan does not paint the claimed digit");
        assert!(PROSE.iter().all(|line| text.contains(line)), "{text}");
    }

    let matching = extract(MATCHING, &config);
    assert!(matching.metadata.ocr_used);
    for text in [&matching.content, &matching.pages.as_ref().unwrap()[0].content] {
        assert!(PROSE.iter().all(|line| text.contains(line)), "{text}");
        let number = text.find('០').expect("the scan paints the matching layer's digit");
        assert_eq!(text.matches('០').count(), 1, "one painted source occurrence");
        assert!(
            number < text.find(PROSE[0]).unwrap(),
            "source number stays above the prose"
        );
    }
}
