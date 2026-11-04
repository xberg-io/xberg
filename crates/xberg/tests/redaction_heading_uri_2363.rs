#![cfg(all(feature = "html", feature = "pdf", feature = "redaction", feature = "tokio-runtime"))]

use lopdf::{Document, Object, Stream, dictionary};
use xberg::core::config::PdfConfig;
use xberg::core::config::redaction::{RedactionConfig, RedactionTerm};
use xberg::types::metadata::HeaderMetadata;
use xberg::types::redaction::RedactionStrategy;
use xberg::types::uri::UriKind;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OutputFormat, extract};

const PERSON: &str = "Jane Doe";
const MASK: &str = "[REDACTED]";
const TARGET: &str = "https://example.test/safe";

fn config(redact: bool) -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        redaction: redact.then(|| RedactionConfig {
            strategy: RedactionStrategy::Mask,
            custom_terms: vec![RedactionTerm::labeled("person", PERSON)],
            ..Default::default()
        }),
        pdf_options: Some(PdfConfig {
            extract_annotations: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn public_extract(bytes: &[u8], mime: &str, redact: bool) -> ExtractedDocument {
    let result = extract(ExtractInput::from_bytes(bytes, mime, None), &config(redact))
        .await
        .expect("public extraction");
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(result.results.len(), 1);
    result.results.into_iter().next().unwrap()
}

fn headings(doc: &ExtractedDocument) -> &[HeaderMetadata] {
    &doc.metadata
        .format
        .as_ref()
        .and_then(|format| format.html())
        .expect("HTML format metadata")
        .headers
}

fn assert_no_term_in_serialized_result(doc: &ExtractedDocument) {
    let json = serde_json::to_string(doc).expect("serialize complete extraction result");
    assert!(!json.contains("Jane"));
    assert!(!json.contains("Doe"));
}

#[tokio::test]
async fn html_heading_redacts_terms_split_by_inline_markup() {
    let input = b"<html><body><h2>Jane <sup>Doe</sup></h2><h2 id='safe'>Safe heading</h2></body></html>";
    let baseline = public_extract(input, "text/html", false).await;
    let baseline_headers = headings(&baseline);
    assert_eq!(baseline_headers.len(), 2);
    assert_eq!(baseline_headers[0].text, "Jane ^Doe^");
    assert!(baseline.content.contains("## Jane ^Doe^"));
    assert_eq!(baseline_headers[1].text, "Safe heading");
    assert_eq!(baseline_headers[1].id.as_deref(), Some("safe"));

    let redacted = public_extract(input, "text/html", true).await;
    assert!(redacted.content.contains(MASK));
    let headers = headings(&redacted);
    assert_eq!(headers.len(), 2);
    assert_eq!(headers[0].text, MASK);
    assert_eq!(headers[1].text, "Safe heading");
    assert_eq!(headers[1].id.as_deref(), Some("safe"));
    assert_no_term_in_serialized_result(&redacted);
    assert_plain_fallback(&redacted);
}

#[tokio::test]
async fn html_heading_redacts_a_term_in_the_source_id_attribute() {
    let input = b"<html><body><h2 id='Jane Doe'>Safe heading</h2></body></html>";
    let baseline = public_extract(input, "text/html", false).await;
    let baseline_headers = headings(&baseline);
    assert_eq!(baseline_headers.len(), 1);
    assert_eq!(baseline_headers[0].id.as_deref(), Some(PERSON));
    assert_eq!(baseline_headers[0].text, "Safe heading");

    let redacted = public_extract(input, "text/html", true).await;
    let headers = headings(&redacted);
    assert_eq!(headers.len(), 1);
    assert_eq!(headers[0].text, "Safe heading");
    assert_eq!(headers[0].id.as_deref(), Some(MASK));
    assert_no_term_in_serialized_result(&redacted);
}

#[tokio::test]
async fn literal_caret_heading_cannot_borrow_the_superscript_proof() {
    let input = b"<h2>Jane ^Doe^</h2>";
    let baseline = public_extract(input, "text/html", false).await;
    assert_eq!(headings(&baseline)[0].text, "Jane ^Doe^");

    let error = extract(ExtractInput::from_bytes(input, "text/html", None), &config(true))
        .await
        .expect_err("unproved styled header must fail closed");
    assert!(
        error
            .to_string()
            .contains("Cannot prove complete generated Markdown redaction coverage")
    );
}

fn assert_plain_fallback(doc: &ExtractedDocument) {
    assert_eq!(doc.metadata.output_format.as_deref(), Some("plain"));
    assert!(
        doc.processing_warnings.iter().any(|warning| {
            warning.source == "output_format" && warning.message.contains("redacted plain text is returned")
        }),
        "cross-format redaction must disclose its plain fallback"
    );
}

fn linked_pdf() -> Vec<u8> {
    let mut doc = Document::with_version("1.4");
    let pages = doc.new_object_id();
    let regular = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica" });
    let bold = doc.add_object(dictionary! { "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica-Bold" });
    let content = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 14 Tf 72 700 Td (Jane ) Tj /F2 14 Tf (Doe) Tj ET".to_vec(),
    ));
    let link = doc.add_object(dictionary! {
        "Type" => "Annot",
        "Subtype" => "Link",
        "Rect" => vec![70.into(), 696.into(), 142.into(), 716.into()],
        "A" => dictionary! { "S" => "URI", "URI" => Object::string_literal(TARGET) },
    });
    let page = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => content,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => regular, "F2" => bold } },
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Annots" => vec![Object::Reference(link)],
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
    doc.save_to(&mut bytes).expect("serialize synthetic PDF link");
    bytes
}

#[tokio::test]
async fn native_pdf_uri_uses_the_target_across_mixed_font_runs() {
    let pdf = linked_pdf();
    let baseline = public_extract(&pdf, "application/pdf", false).await;
    assert!(baseline.content.contains("Jane **Doe**"));
    assert!(baseline.content.contains("## Annotations"));
    let redacted = public_extract(&pdf, "application/pdf", true).await;

    for document in [&baseline, &redacted] {
        let uris = document.uris.as_ref().expect("PDF link URI extracted");
        assert_eq!(uris.len(), 1);
        assert_eq!(uris[0].kind, UriKind::Hyperlink);
        assert_eq!(uris[0].url, TARGET);
        assert_eq!(uris[0].label.as_deref(), Some(TARGET));
        assert_eq!(uris[0].page, Some(1));
    }
    assert!(redacted.content.contains(MASK));
    assert_no_term_in_serialized_result(&redacted);
    assert_plain_fallback(&redacted);
}

#[tokio::test]
async fn unproved_formatted_heading_cannot_leak_through_plain_fallback() {
    let input = b"<h2>Jane <sup>Doe</sup></h2>";
    let mut options = config(false);
    options.html_options = Some(html_to_markdown_rs::ConversionOptions {
        extract_metadata: true,
        sup_symbol: "**".to_string(),
        ..Default::default()
    });
    let source = ExtractInput::from_bytes(input, "text/html", None);
    let baseline = extract(source, &options).await.expect("unredacted Markdown extraction");
    assert_eq!(headings(&baseline.results[0])[0].text, "Jane **Doe**");

    options.redaction = config(true).redaction;
    let result = extract(ExtractInput::from_bytes(input, "text/html", None), &options).await;
    let error = result.expect_err("the structured caret proof does not certify a bold-delimited heading");
    assert!(
        error
            .to_string()
            .contains("Cannot prove complete styled HTML heading redaction")
    );
}
