#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::types::ExtractionMethod;
use xberg::{ExtractedDocument, ExtractionConfig, OutputFormat, PageConfig};

const SOURCE: &[u8] = include_bytes!("fixtures/pdf/ligature-source-space-2322/dejavu-source-space.pdf");
const LITERAL: &str = "diﬀerent deﬁnes ﬂow; staﬀ only, waﬄe ﬂoor";
const COMPATIBILITY_TEXT: &str = "different defines flow; staff only, waffle floor";

#[tokio::test]
async fn native_default_text_keeps_the_words_and_the_source_space() {
    assert_eq!(
        hex::encode(Sha256::digest(SOURCE)),
        "d49ee26b226d6196de84bb12b5c15751c77196ba9d1f502252bad099e48e8413"
    );
    let native = xberg_native_pdf::PdfDocument::from_bytes(SOURCE.to_vec()).unwrap();
    assert_eq!(native.page_count().unwrap(), 1);
    assert_words_and_source_space(native.extract_text(0).unwrap().trim_end());
    let default = extract(&ExtractionConfig::default()).await;
    assert_words_and_source_space(default.content.trim_end());
}

#[tokio::test]
async fn rich_plain_expands_ligatures_and_preserves_the_source_space() {
    let document = extract(&rich_config(OutputFormat::Plain)).await;
    assert_words_and_source_space(document.pages.as_ref().unwrap()[0].content.trim_end());
    assert_eq!(document.content.trim_end(), COMPATIBILITY_TEXT);
    assert_paragraph(&document);
}

#[tokio::test]
async fn rich_markdown_expands_ligatures_and_preserves_the_source_space() {
    let document = extract(&rich_config(OutputFormat::Markdown)).await;
    assert_eq!(document.content.trim_end(), COMPATIBILITY_TEXT);
    assert_eq!(
        document.pages.as_ref().unwrap()[0].content.trim_end(),
        COMPATIBILITY_TEXT
    );
    assert_paragraph(&document);
}

/// The native text is either the author's ligature code points or their letters, never a mix of the
/// two, and the source space after "staff" stays in both forms.
fn assert_words_and_source_space(actual: &str) {
    assert!(
        actual == LITERAL || actual == COMPATIBILITY_TEXT,
        "expected the ligature code points or their letters with the source space, got: {actual:?}"
    );
}

async fn extract(config: &ExtractionConfig) -> ExtractedDocument {
    let document = helpers::extract_bytes_document(SOURCE, "application/pdf", config)
        .await
        .unwrap();
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Native));
    assert!(document.tables.is_empty());
    document
}

fn rich_config(output_format: OutputFormat) -> ExtractionConfig {
    ExtractionConfig {
        output_format,
        include_document_structure: true,
        use_cache: false,
        disable_ocr: true,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn assert_paragraph(document: &ExtractedDocument) {
    let [node] = document.document.as_ref().unwrap().nodes.as_slice() else {
        panic!("expected a single source paragraph");
    };
    assert_eq!(node.content.text(), Some(COMPATIBILITY_TEXT));
}
