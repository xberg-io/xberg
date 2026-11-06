#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng",
    paddle_ocr
))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::types::formats::OcrTable;
use xberg::types::{NodeContent, OcrElement, OcrElementConfig, OcrElementLevel};
use xberg::{ExtractionConfig, OcrConfig, PageConfig};

const SCAN_SHA256: &str = "874bada67b7bf700d42df25265823799955ae37ba3ec0e283747d1f2cd5d94f7";
const ENGLISH_SHA256: &str = "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2";
const PAGE_PREFIX_SHA256: &str = "31c5f60c88e8a7dbdb271039dc3e9d253ce8ca6f243fb32ed04a02bcafeaec63";
const CAPTION_START: &str = "Figure 4:";

#[derive(serde::Deserialize)]
struct SourceCaption {
    table: OcrTable,
    elements: Vec<OcrElement>,
}

#[test]
fn preserves_the_full_docling_caption_as_prose_on_page_nine_and_in_structure() {
    let path = helpers::get_test_file_path("pdf_scanned/docling_scanned.pdf");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required scan {}: {error}", path.display()));
    assert_eq!(hex::encode(Sha256::digest(&bytes)), SCAN_SHA256);
    let source: SourceCaption = serde_json::from_str(include_str!("fixtures/ocr_caption_2530/p9-iterator.json"))
        .expect("same-attempt iterator line evidence");
    let models = tempfile::tempdir().expect("private test model directory");
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    assert_eq!(hex::encode(Sha256::digest(english)), ENGLISH_SHA256);
    std::fs::write(models.path().join("eng.traineddata"), english).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        include_document_structure: true,
        ocr: Some(OcrConfig {
            tessdata_path: Some(models.path().to_path_buf()),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let document = helpers::extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
    let pages = document.pages.as_deref().expect("requested page output");
    assert_eq!(pages.len(), 9);
    let page = &pages[8];
    assert_eq!(page.page_number, 9);
    let confidence = page.ocr_confidence.as_ref().expect("accepted page OCR");
    assert_eq!(confidence.backend, "tesseract");
    assert_eq!(confidence.word_count, 507);
    assert!(document.ocr_elements.is_none(), "internal evidence must stay private");

    let plain_caption = source
        .table
        .cells
        .iter()
        .map(|row| row.join(" "))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        document.content.ends_with(plain_caption.trim_end()),
        "retain existing Plain rendering bytes"
    );
    assert_eq!(document.content.matches(CAPTION_START).count(), 1);
    assert_eq!(page.content.matches(CAPTION_START).count(), 1);
    assert!(
        !page.content.contains(source.table.markdown.trim()),
        "Figure 4 is still emitted as the exact false caption grid on page nine"
    );

    let line_text = source
        .elements
        .iter()
        .filter(|element| element.level == OcrElementLevel::Line)
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let preserved_prefix = page
        .content
        .strip_suffix(&line_text)
        .expect("render the six same-attempt lines as prose on page nine");
    assert_eq!(hex::encode(Sha256::digest(preserved_prefix)), PAGE_PREFIX_SHA256);

    assert!(page.tables.is_empty());
    assert_eq!(document.tables.len(), 1, "leave the unrelated page-one table present");
    assert_eq!(document.tables[0].page_number, 1);
    assert_eq!(document.tables[0].cells[0][0], "This technical report");

    let mut control_config = config;
    control_config.ocr.as_mut().unwrap().element_config = Some(OcrElementConfig::default());
    let control = helpers::extract_bytes_document_blocking(&bytes, "application/pdf", &control_config).unwrap();
    let original_pages = control.pages.as_deref().unwrap();
    assert_eq!(original_pages.len(), 9);
    assert_eq!(control.content, document.content, "Plain document text is unchanged");
    assert_eq!(
        serde_json::to_value(&original_pages[..8]).unwrap(),
        serde_json::to_value(&pages[..8]).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&control.tables[0]).unwrap(),
        serde_json::to_value(&document.tables[0]).unwrap()
    );
    assert_eq!(control.tables.len(), 2, "the separate control keeps its original grid");
    assert!(original_pages[8].content.contains(source.table.markdown.trim()));
    assert_eq!(
        original_pages[8]
            .content
            .replace(source.table.markdown.trim(), &line_text),
        page.content,
    );

    let structure = document.document.expect("requested document structure");
    let node = structure.nodes.last().expect("the caption is the last node");
    assert_eq!(node.page, Some(9));
    assert!(matches!(&node.content, NodeContent::Paragraph { text } if text == &plain_caption));
}
