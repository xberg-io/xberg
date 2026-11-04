use super::{ImageAnchor, anchor_images_as_ocr_text, image_anchor};
use crate::core::config::{
    ExtractionConfig, ImageExtractionConfig, OcrConfig, OutputFormat, PdfConfig, PostProcessorConfig,
};
use crate::core::pipeline::run_pipeline;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement};
use crate::types::{ExtractedDocument, ExtractedImage, ImageKind};
use serial_test::serial;

const BODY: &str = "The scanned PDF contains a body paragraph.";
const WORDS: &str = "Recognized picture words.";
const CAPTION: &str = "Original caption.";

fn config(images: Option<(bool, bool)>, legacy: bool, embedded_ocr: bool) -> ExtractionConfig {
    ExtractionConfig {
        output_format: OutputFormat::Markdown,
        use_cache: false,
        enable_quality_processing: false,
        postprocessor: Some(PostProcessorConfig {
            enabled: false,
            ..Default::default()
        }),
        ocr_embedded_images: Some(embedded_ocr),
        images: images.map(|(extract_images, inject_placeholders)| ImageExtractionConfig {
            extract_images,
            inject_placeholders,
            ..Default::default()
        }),
        pdf_options: Some(PdfConfig {
            extract_images: legacy,
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn placeholders_require_requested_images_and_an_explicit_placeholder_flag() {
    use ImageAnchor::{None as NoAnchor, OcrTextOnly, Placeholder};
    let cases = [
        (None, false, NoAnchor, OcrTextOnly),
        (None, true, NoAnchor, OcrTextOnly),
        (Some((false, true)), false, NoAnchor, OcrTextOnly),
        (Some((false, true)), true, Placeholder, Placeholder),
        (Some((true, true)), false, Placeholder, Placeholder),
        (Some((true, true)), true, Placeholder, Placeholder),
        (Some((true, false)), false, NoAnchor, OcrTextOnly),
        (Some((true, false)), true, NoAnchor, OcrTextOnly),
        (Some((false, false)), false, NoAnchor, OcrTextOnly),
        (Some((false, false)), true, NoAnchor, OcrTextOnly),
    ];
    let mut examined = 0;
    for (images, legacy, without_ocr, with_ocr) in cases {
        for (embedded_ocr, expected) in [(false, without_ocr), (true, with_ocr)] {
            let config = config(images, legacy, embedded_ocr);
            assert!(
                image_anchor(&config) == expected,
                "images={images:?}, legacy={legacy}, embedded_ocr={embedded_ocr}"
            );
            examined += 1;
        }
    }
    assert_eq!(examined, 20);
}

#[test]
fn disabling_image_output_keeps_the_embedded_ocr_byte_read_gate() {
    let config = config(Some((false, true)), false, true);
    assert!(config.needs_image_data());
    assert!(config.runs_ocr_on_embedded_images());
    assert!(image_anchor(&config) == ImageAnchor::OcrTextOnly);
}

#[test]
fn disabled_ocr_does_not_create_text_anchors_for_unrequested_images() {
    let base = config(Some((false, true)), false, true);
    let mut disabled = base.clone();
    disabled.disable_ocr = true;
    let mut disabled_backend = base.clone();
    disabled_backend.ocr = Some(OcrConfig {
        enabled: false,
        ..Default::default()
    });
    let mut disabled_images = base;
    disabled_images.images.as_mut().unwrap().run_ocr_on_images = false;
    for config in [disabled, disabled_backend, disabled_images] {
        assert!(!config.runs_ocr_on_embedded_images());
        assert!(image_anchor(&config) == ImageAnchor::None);
    }
}

fn pixel_png() -> bytes::Bytes {
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(1, 1)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    encoded.into_inner().into()
}

fn native_document(ocr_words: &str) -> InternalDocument {
    let mut doc = InternalDocument::new("pdf");
    doc.push_element(InternalElement::text(ElementKind::Paragraph, BODY, 0).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::Image { image_index: 0 }, "", 0).with_page(1));
    doc.push_element(InternalElement::text(ElementKind::Paragraph, CAPTION, 0).with_page(1));
    doc.images.push(ExtractedImage {
        data: pixel_png(),
        format: std::borrow::Cow::Borrowed("png"),
        page_number: Some(1),
        image_kind: Some(ImageKind::PageRaster),
        ocr_result: Some(Box::new(ExtractedDocument {
            content: ocr_words.to_owned(),
            ..Default::default()
        })),
        ..Default::default()
    });
    doc
}

async fn selected_native_document(config: &ExtractionConfig, ocr_words: &str) -> ExtractedDocument {
    let mut doc = native_document(ocr_words);
    assert_eq!(doc.elements.len(), 3);
    assert_eq!(doc.images.len(), 1);
    assert!(!doc.images[0].data.is_empty());
    if image_anchor(config) == ImageAnchor::OcrTextOnly {
        anchor_images_as_ocr_text(&mut doc);
    }
    run_pipeline(doc, config).await.unwrap()
}

fn assert_text_without_generated_link(result: &ExtractedDocument, words: &str) {
    assert_eq!(result.content.matches(BODY).count(), 1);
    assert_eq!(result.content.matches(CAPTION).count(), 1);
    assert_eq!(result.content.matches(WORDS).count(), usize::from(!words.is_empty()));
    assert!(!result.content.contains("image_0.png"), "{}", result.content);
    assert!(result.images.is_none());
    assert_eq!(result.counts.images, 1);
    let pages = result.pages.as_ref().expect("the tagged source page must be derived");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].content.matches(BODY).count(), 1);
    assert_eq!(pages[0].content.matches(CAPTION).count(), 1);
    assert_eq!(pages[0].content.matches(WORDS).count(), usize::from(!words.is_empty()));
    assert!(!pages[0].content.contains("image_0.png"), "{}", pages[0].content);
    assert!(pages[0].image_indices.is_empty());
}

#[tokio::test]
#[serial]
async fn unrequested_pdf_images_keep_ocr_words_without_document_or_page_links() {
    for images in [None, Some((false, true))] {
        let result = selected_native_document(&config(images, false, true), WORDS).await;
        assert_text_without_generated_link(&result, WORDS);
    }
}

#[tokio::test]
#[serial]
async fn unrequested_pdf_image_without_child_words_does_not_create_an_empty_placeholder() {
    let result = selected_native_document(&config(Some((false, true)), false, true), "").await;
    assert_text_without_generated_link(&result, "");
}

#[tokio::test]
#[serial]
async fn requested_pdf_images_keep_the_real_payload_and_generated_link() {
    let result = selected_native_document(&config(Some((true, true)), false, true), WORDS).await;
    assert_eq!(result.content.matches("image_0.png").count(), 1);
    assert_eq!(result.content.matches(WORDS).count(), 1);
    let images = result
        .images
        .as_ref()
        .expect("requested image payload must be returned");
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].data, pixel_png());
    let decoded = image::load_from_memory(&images[0].data).expect("the retained PNG must be valid");
    assert_eq!((decoded.width(), decoded.height()), (1, 1));
    let pages = result.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].content.matches("image_0.png").count(), 1);
    assert_eq!(pages[0].image_indices, vec![0]);
}

#[tokio::test]
#[serial]
async fn source_markdown_image_links_and_code_survive_the_pdf_policy() {
    use crate::extractors::markdown::MarkdownExtractor;
    use crate::plugins::InternalDocumentExtractor;
    use crate::types::uri::ExtractedUri;

    let source = "![remote](https://example.com/image.png)\n\n![relative](image_0.png)\n\n\
                  ![inline](data:image/svg+xml;base64,PHN2Zy8+)\n\n`![](image_0.png)`\n\nOriginal caption.";
    let mut config = config(Some((false, true)), false, false);
    config.disable_ocr = true;
    let doc = MarkdownExtractor::new()
        .extract_content(source.as_bytes(), "text/markdown", &config)
        .await
        .unwrap();
    assert_eq!(doc.images.len(), 1);
    assert_eq!(doc.images[0].data.as_ref(), b"<svg/>");
    assert_eq!(doc.images[0].format, "svg");
    let expected_uris = vec![
        ExtractedUri::image("https://example.com/image.png", Some("remote".to_owned())),
        ExtractedUri::image("image_0.png", Some("relative".to_owned())),
        ExtractedUri::image("data:image/svg+xml;base64,PHN2Zy8+", Some("inline".to_owned())),
    ];
    assert_eq!(doc.uris, expected_uris);
    for reference in [
        "[Image: remote (https://example.com/image.png)]",
        "[Image: relative (image_0.png)]",
    ] {
        assert_eq!(
            doc.elements
                .iter()
                .filter(|element| element.kind == ElementKind::Paragraph && element.text == reference)
                .count(),
            1
        );
    }
    let baseline_rendering = crate::rendering::render_markdown(&doc);
    let result = run_pipeline(doc, &config).await.unwrap();
    assert_eq!(result.content, baseline_rendering);
    assert_eq!(result.uris.as_ref(), Some(&expected_uris));
    for preserved in [
        "https://example.com/image.png",
        "![inline](image_0.svg)",
        "`![](image_0.png)`",
        CAPTION,
    ] {
        assert_eq!(result.content.matches(preserved).count(), 1, "{}", result.content);
    }
    assert_eq!(result.content.matches("image_0.png").count(), 2);
}
