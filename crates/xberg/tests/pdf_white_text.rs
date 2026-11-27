//! White text on no painted content is dropped with a warning, or kept with `keep_white_text`.

#![cfg(all(feature = "pdf", feature = "tokio-runtime"))]

mod helpers;

use helpers::extract_bytes_document_blocking;
use lopdf::{Document, Object, Stream, dictionary};
use xberg::core::config::{PdfBackend, PdfConfig};
use xberg::{ExtractedDocument, ExtractionConfig};

const VISIBLE: &str = "BT /F1 12 Tf 72 720 Td (Visible) Tj ET\n";
const HIDDEN: &str = "BT /F1 12 Tf 1 g 72 600 Td (Hidden) Tj ET\n";

fn pdf(content: &str) -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let pages = doc.new_object_id();
    let font = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
    let stream = doc.add_object(Stream::new(dictionary! {}, content.as_bytes().to_vec()));
    let page = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => stream,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
    });
    doc.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    doc.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).expect("serialize fixture PDF");
    bytes
}

fn extract(bytes: &[u8], keep_white_text: bool) -> ExtractedDocument {
    let config = ExtractionConfig {
        use_cache: false,
        disable_ocr: true,
        pdf_options: Some(PdfConfig {
            backend: PdfBackend::Native,
            keep_white_text,
            ..Default::default()
        }),
        ..Default::default()
    };
    extract_bytes_document_blocking(bytes, "application/pdf", &config).expect("native PDF extraction")
}

fn white_text_warnings(document: &ExtractedDocument) -> Vec<String> {
    document
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "pdf_white_text")
        .map(|warning| warning.message.to_string())
        .collect()
}

#[test]
fn dropped_white_text_is_reported_in_a_warning() {
    let document = extract(&pdf(&format!("{VISIBLE}{HIDDEN}")), false);

    assert!(document.content.contains("Visible"), "{:?}", document.content);
    assert!(!document.content.contains("Hidden"), "{:?}", document.content);
    let warnings = white_text_warnings(&document);
    assert_eq!(warnings.len(), 1, "{:?}", document.processing_warnings);
    assert!(warnings[0].contains("Dropped 1 white text span(s)"), "{}", warnings[0]);
    assert!(warnings[0].contains("by page (spans): 1 (1)."), "{}", warnings[0]);
}

#[test]
fn keep_white_text_keeps_it_without_a_warning() {
    let document = extract(&pdf(&format!("{VISIBLE}{HIDDEN}")), true);

    assert!(document.content.contains("Visible"), "{:?}", document.content);
    assert!(document.content.contains("Hidden"), "{:?}", document.content);
    assert!(
        white_text_warnings(&document).is_empty(),
        "{:?}",
        document.processing_warnings
    );
}

#[test]
fn a_document_without_white_text_has_no_warning() {
    let document = extract(&pdf(VISIBLE), false);

    assert!(document.content.contains("Visible"), "{:?}", document.content);
    assert!(
        white_text_warnings(&document).is_empty(),
        "{:?}",
        document.processing_warnings
    );
}
