#![cfg(all(feature = "pdf", feature = "ocr"))]

use lopdf::{Document, Object, Stream, dictionary};
use xberg::core::config::ExtractInput;
use xberg::types::ProcessingWarning;
use xberg::{ExtractedDocument, ExtractionConfig, extract};

const WARNING: &str = "Page 1 rendered blank and a painted image could not be decoded for OCR.";

const PAINT_IMAGE: &[u8] = b"q 288 0 0 80 0 0 cm /Im0 Do Q";

fn synthetic_scan(commands: &[u8]) -> Vec<u8> {
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let image = pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => 864, "Height" => 240,
            "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8, "Filter" => "FlateDecode",
        },
        b"This is not zlib compressed data".to_vec(),
    ));
    let content = pdf.add_object(Stream::new(dictionary! {}, commands.to_vec()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Integer(288), Object::Integer(80)],
        "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => image } },
        "Contents" => content,
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1,
        }),
    );
    let root = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).unwrap();
    bytes
}

async fn public_pdf_result(commands: &[u8], force_ocr: bool) -> ExtractedDocument {
    let config = ExtractionConfig {
        use_cache: false,
        force_ocr,
        ..Default::default()
    };
    let input = ExtractInput::from_bytes(synthetic_scan(commands), "application/pdf", None);
    let mut result = extract(input, &config)
        .await
        .expect("extract synthetic PDF with real OCR");
    assert_eq!(result.results.len(), 1);
    result.results.remove(0)
}

fn assert_image_warning(document: &ExtractedDocument) {
    assert_eq!(document.content.trim(), "");
    let actual: Vec<_> = document
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "ocr")
        .map(|warning| warning.message.as_ref())
        .collect();
    assert_eq!(actual, [WARNING]);
}

#[tokio::test]
async fn default_pdf_ocr_warns_when_the_only_painted_image_cannot_decode() {
    assert_image_warning(&public_pdf_result(PAINT_IMAGE, false).await);
}

#[tokio::test]
async fn forced_pdf_ocr_warns_when_the_only_painted_image_cannot_decode() {
    assert_image_warning(&public_pdf_result(PAINT_IMAGE, true).await);
}

#[tokio::test]
async fn unused_broken_image_does_not_warn_on_a_blank_pdf_page() {
    let result = public_pdf_result(b"", false).await;
    assert_eq!(result.content.trim(), "");
    assert_no_image_decode_warning(&result.processing_warnings);
}

fn assert_no_image_decode_warning(warnings: &[ProcessingWarning]) {
    assert!(
        warnings.iter().all(|warning| warning.message != WARNING),
        "unexpected painted-image decode warning on blank PDF: {warnings:?}"
    );
}

#[test]
#[should_panic(expected = "unexpected painted-image decode warning on blank PDF")]
fn blank_page_warning_oracle_rejects_the_painted_image_message() {
    assert_no_image_decode_warning(&[ProcessingWarning {
        source: "ocr".into(),
        message: WARNING.into(),
    }]);
}

#[tokio::test]
async fn unresolved_image_operator_does_not_overstate_which_image_failed() {
    let commands = b"q 288 0 0 80 0 0 cm /Missing Do /Im0 Do Q";
    assert_image_warning(&public_pdf_result(commands, true).await);
}
