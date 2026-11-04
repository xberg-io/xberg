#![cfg(all(
    feature = "html",
    feature = "office",
    feature = "pdf",
    feature = "qr-codes",
    feature = "redaction",
    feature = "tokio-runtime"
))]

use base64::Engine as _;
use image::{ExtendedColorType, ImageEncoder, Luma};
use lopdf::{Document, Object, Stream, dictionary};
use std::io::{Cursor, Write};
use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::core::config::{ImageExtractionConfig, PdfConfig};
use xberg::types::redaction::RedactionStrategy;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, extract};

const DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const PERSON: &str = "Jane Doe";
const MASK: &str = "[REDACTED]";
const QR: &str = "Jane Doe payload";
const SAFE_QR: &str = "safe payload";
const NOTE: &str = "Safe annotation note";

fn config(format: OutputFormat, redact: bool) -> ExtractionConfig {
    ExtractionConfig {
        output_format: format,
        use_cache: false,
        redaction: redact.then(|| RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: vec![RedactionTerm::labeled("person", PERSON)],
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn public(bytes: &[u8], mime: &str, config: &ExtractionConfig) -> ExtractedDocument {
    let result = extract(ExtractInput::from_bytes(bytes, mime, None), config)
        .await
        .unwrap();
    assert_eq!(result.results.len(), 1);
    result.results.into_iter().next().unwrap()
}

fn annotated_pdf(author: &str) -> Vec<u8> {
    let mut doc = Document::with_version("1.4");
    let pages = doc.new_object_id();
    let font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 14 Tf 72 700 Td (Ordinary PDF body) Tj ET".to_vec(),
    ));
    let annotation = doc.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Text", "Rect" => vec![100.into(), 620.into(), 120.into(), 642.into()],
        "T" => Object::string_literal(author),
        "Contents" => Object::string_literal(NOTE),
    });
    let page = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => content,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Annots" => vec![Object::Reference(annotation)],
    });
    doc.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1,
        }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    doc.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

async fn pdf_author(author: &str, redact: bool) -> ExtractedDocument {
    let mut options = config(OutputFormat::Markdown, redact);
    options.pdf_options = Some(PdfConfig {
        extract_annotations: true,
        ..Default::default()
    });
    public(&annotated_pdf(author), "application/pdf", &options).await
}

fn annotation_author(doc: &ExtractedDocument) -> &str {
    let annotations = doc.annotations.as_ref().expect("actual extracted PDF annotation");
    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations[0].content.as_deref(), Some(NOTE));
    assert_eq!(annotations[0].page_number, 1);
    annotations[0].author.as_deref().expect("actual PDF annotation author")
}

#[tokio::test]
async fn should_redact_native_pdf_annotation_author() {
    let baseline = pdf_author(PERSON, false).await;
    assert_eq!(annotation_author(&baseline), PERSON);
    assert!(baseline.content.contains("Ordinary PDF body"));
    let result = pdf_author(PERSON, true).await;
    assert!(result.content.contains(MASK));
    assert!(!result.content.contains(PERSON));
    assert_eq!(annotation_author(&result), MASK);
}

#[tokio::test]
async fn should_keep_safe_native_pdf_annotation_author() {
    let baseline = pdf_author("Safe Author", false).await;
    let result = pdf_author("Safe Author", true).await;
    assert_eq!(annotation_author(&result), "Safe Author");
    assert_eq!(result.content, baseline.content);
    assert_eq!(result.redaction_report.unwrap().total_redacted, 0);
}

fn qr_png(payload: &str) -> Vec<u8> {
    let image = qrcode::QrCode::new(payload)
        .unwrap()
        .render::<Luma<u8>>()
        .min_dimensions(256, 256)
        .build();
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(image.as_raw(), image.width(), image.height(), ExtendedColorType::L8)
        .unwrap();
    png
}

fn qr_html(payload: &str) -> String {
    let encoded = base64::engine::general_purpose::STANDARD.encode(qr_png(payload));
    format!("<p>Safe body</p><img src='data:image/png;base64,{encoded}' alt='Safe image'>")
}

fn qr_config(format: OutputFormat, redact: bool) -> ExtractionConfig {
    let mut options = config(format, redact);
    options.qr_codes = Some(true);
    options.images = Some(ImageExtractionConfig {
        inject_placeholders: false,
        ..Default::default()
    });
    options
}

fn payload(doc: &ExtractedDocument) -> &str {
    let images = doc.images.as_ref().expect("actual embedded image");
    assert_eq!(images.len(), 1);
    let codes = images[0].qr_codes.as_ref().expect("actual QR detection");
    assert_eq!(codes.len(), 1);
    assert_eq!(codes[0].confidence, Some(1.0));
    assert!(codes[0].bbox.is_some());
    &codes[0].payload
}

async fn html_qr(payload: &str, redact: bool) -> ExtractedDocument {
    let html = qr_html(payload);
    public(html.as_bytes(), "text/html", &qr_config(OutputFormat::Plain, redact)).await
}

#[tokio::test]
async fn should_redact_decoded_qr_payload_from_html() {
    let baseline = html_qr(QR, false).await;
    assert_eq!(payload(&baseline), QR);
    let result = html_qr(QR, true).await;
    assert!(result.content.contains("[REDACTED] payload"));
    assert!(!result.content.contains(PERSON));
    assert_eq!(payload(&result), "[REDACTED] payload");
}

#[tokio::test]
async fn should_keep_safe_decoded_qr_payload() {
    let baseline = html_qr(SAFE_QR, false).await;
    let result = html_qr(SAFE_QR, true).await;
    assert_eq!(payload(&result), SAFE_QR);
    assert_eq!(result.content, baseline.content);
    assert_eq!(result.redaction_report.unwrap().total_redacted, 0);
}

fn qr_docx(payload: &str) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let parts = [
        (
            "[Content_Types].xml",
            include_str!("redaction_structured_fields_2363/content-types.xml"),
        ),
        (
            "_rels/.rels",
            include_str!("redaction_structured_fields_2363/package-relationships.xml"),
        ),
        (
            "word/document.xml",
            include_str!("redaction_structured_fields_2363/document.xml"),
        ),
        (
            "word/_rels/document.xml.rels",
            include_str!("redaction_structured_fields_2363/document-relationships.xml"),
        ),
    ];
    for (name, xml) in parts {
        zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(xml.as_bytes()).unwrap();
    }
    zip.start_file("word/media/qr.png", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(&qr_png(payload)).unwrap();
    zip.finish().unwrap().into_inner()
}

#[tokio::test]
async fn should_redact_decoded_qr_payload_from_docx() {
    let bytes = qr_docx(QR);
    let baseline = public(&bytes, DOCX, &qr_config(OutputFormat::Markdown, false)).await;
    assert!(baseline.content.contains("Safe body"));
    assert_eq!(payload(&baseline), QR);
    let result = public(&bytes, DOCX, &qr_config(OutputFormat::Markdown, true)).await;
    assert!(result.content.contains("[REDACTED] payload"));
    assert!(!result.content.contains(PERSON));
    assert_eq!(payload(&result), "[REDACTED] payload");
}
