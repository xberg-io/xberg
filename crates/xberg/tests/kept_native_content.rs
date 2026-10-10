//! The text layer of a PDF page whose `content` OCR output replaces:
//! `PageConfig::keep_native_content` and `PageContent::native_content`
//! (see <https://github.com/xberg-io/xberg/issues/2069>).
//!
//! A stub OCR backend returns the same fixed words for every image, so each test knows the OCR
//! text of a page without an OCR engine. The words differ from the text layer of every fixture
//! page. All values are invented.

#![cfg(all(feature = "pdf", feature = "ocr"))]

mod helpers;

use async_trait::async_trait;
use helpers::{BytesInput, extract_bytes_document_blocking, extract_bytes_documents_blocking};
use std::sync::Arc;
use xberg::core::config::{ExtractionConfig, OcrConfig, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::PageContent;
use xberg::{ExtractInput, ExtractedDocument, FileExtractionConfig, XbergError};

const PDF_MIME_TYPE: &str = "application/pdf";
const STUB_BACKEND: &str = "kept-native-content-stub";

/// The words the stub reads from every image.
const OCR_WORDS: &str = "Stub reading of the page render with enough plain words to stand as the \
                         recognized text of one whole page in every route that these tests use";
/// A text layer of one short line, below the size at which a page is sent to OCR.
const SHORT_LINE: &str = "Ref KX-204 Qty 7";
/// A text layer long enough to stay native text on the automatic routes.
const LONG_LINE: &str = "The harbour ledger lists forty crates of dried apricots that arrived on the \
                         morning ferry and were stored in the second warehouse before noon.";
/// The text a stamp adds on top of a scanned page.
const STAMP: &str = "COPY 17";

struct StubBackend;

impl Plugin for StubBackend {
    fn name(&self) -> &str {
        STUB_BACKEND
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for StubBackend {
    async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        let mut document = ExtractedDocument::default();
        document.content = OCR_WORDS.to_string();
        Ok(document)
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

/// What one fixture page holds.
#[derive(Clone, Copy)]
enum FixturePage {
    /// One line of real text.
    Text(&'static str),
    /// A raster over the whole page and no text layer.
    Scan,
    /// A raster over the whole page and one line of real text on top.
    StampedScan(&'static str),
}

/// A PDF with one Letter page for each entry of `pages`.
fn pdf_with_pages(pages: &[FixturePage]) -> Vec<u8> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    let mut document = Document::with_version("1.5");
    let pages_id = document.new_object_id();
    let font_id = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let image_id = document.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => 1,
            "Height" => 1,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 8,
        },
        vec![0xA0],
    ));

    let mut kids: Vec<Object> = Vec::new();
    for page in pages {
        let (text, has_raster) = match *page {
            FixturePage::Text(text) => (Some(text), false),
            FixturePage::Scan => (None, true),
            FixturePage::StampedScan(text) => (Some(text), true),
        };
        let mut operations = Vec::new();
        if has_raster {
            operations.extend([
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![612.into(), 0.into(), 0.into(), 792.into(), 0.into(), 0.into()],
                ),
                Operation::new("Do", vec![Object::Name(b"Scan".to_vec())]),
                Operation::new("Q", vec![]),
            ]);
        }
        if let Some(text) = text {
            operations.extend([
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 9.into()]),
                Operation::new("Td", vec![72.into(), 720.into()]),
                Operation::new("Tj", vec![Object::string_literal(text)]),
                Operation::new("ET", vec![]),
            ]);
        }
        let content_id = document.add_object(Stream::new(
            dictionary! {},
            Content { operations }.encode().expect("the page content encodes"),
        ));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => dictionary! {
                "Font" => dictionary! { "F1" => font_id },
                "XObject" => dictionary! { "Scan" => image_id },
            },
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        kids.push(page_id.into());
    }

    document.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => pages.len() as i64,
            "Kids" => kids,
        }),
    );
    let catalog_id = document.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    document.trailer.set("Root", catalog_id);

    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("the fixture serializes");
    bytes
}

/// A config with an `ocr` block for the stub backend and pages on.
fn ocr_config(keep_native_content: bool) -> ExtractionConfig {
    ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: STUB_BACKEND.to_string(),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            keep_native_content,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    }
}

/// A config that reads the text layer only, with pages on.
fn text_layer_config() -> ExtractionConfig {
    ExtractionConfig {
        disable_ocr: true,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    }
}

/// Extract `pdf` with the stub backend registered and return the pages.
fn extract_pages(pdf: &[u8], config: &ExtractionConfig) -> Vec<PageContent> {
    let _ = unregister_ocr_backend(STUB_BACKEND);
    register_ocr_backend(Arc::new(StubBackend)).expect("the stub backend registers");
    let result = extract_bytes_document_blocking(pdf, PDF_MIME_TYPE, config);
    unregister_ocr_backend(STUB_BACKEND).expect("the stub backend unregisters");
    result
        .expect("extraction succeeds")
        .pages
        .expect("extract_pages gives pages")
}

/// The text layer of each page of `pdf`, as page `content` with OCR disabled.
fn text_layer_of_pages(pdf: &[u8]) -> Vec<String> {
    extract_pages(pdf, &text_layer_config())
        .into_iter()
        .map(|page| page.content)
        .collect()
}

fn page_json(page: &PageContent) -> serde_json::Map<String, serde_json::Value> {
    match serde_json::to_value(page).expect("the page serializes") {
        serde_json::Value::Object(map) => map,
        other => panic!("a page serializes as an object, got {other}"),
    }
}

fn assert_ocr_content(page: &PageContent) {
    assert_eq!(
        page.content.trim(),
        OCR_WORDS,
        "the content of page {} must be the OCR text",
        page.page_number
    );
}

#[test]
#[serial_test::serial]
fn a_short_text_page_sent_to_ocr_keeps_its_text_layer_when_asked() {
    let pdf = pdf_with_pages(&[FixturePage::Text(SHORT_LINE)]);
    let text_layer = text_layer_of_pages(&pdf);
    assert!(
        text_layer[0].contains(SHORT_LINE),
        "the fixture has a text layer: {text_layer:?}"
    );

    let pages = extract_pages(&pdf, &ocr_config(true));

    assert_eq!(pages.len(), 1);
    assert_ocr_content(&pages[0]);
    assert_eq!(pages[0].native_content.as_deref(), Some(text_layer[0].as_str()));
}

#[test]
#[serial_test::serial]
fn the_default_result_does_not_carry_the_text_layer() {
    let pdf = pdf_with_pages(&[FixturePage::Text(SHORT_LINE)]);

    let default_pages = extract_pages(&pdf, &ocr_config(false));
    let kept_pages = extract_pages(&pdf, &ocr_config(true));

    assert_ocr_content(&default_pages[0]);
    assert_eq!(default_pages[0].native_content, None);
    let default_json = page_json(&default_pages[0]);
    assert!(
        !default_json.contains_key("native_content"),
        "the default page must not serialize the key: {default_json:?}"
    );

    let mut kept_json = page_json(&kept_pages[0]);
    assert!(
        kept_json.remove("native_content").is_some(),
        "with the setting on the page serializes the key"
    );
    assert_eq!(
        kept_json, default_json,
        "the setting must change nothing else on the page"
    );
}

#[test]
#[serial_test::serial]
fn a_page_not_sent_to_ocr_carries_no_native_content() {
    let pdf = pdf_with_pages(&[FixturePage::Text(LONG_LINE), FixturePage::Text(SHORT_LINE)]);
    let text_layer = text_layer_of_pages(&pdf);
    let config = ExtractionConfig {
        force_ocr_pages: Some(vec![2]),
        ..ocr_config(true)
    };

    let pages = extract_pages(&pdf, &config);

    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].content, text_layer[0], "page 1 keeps its own text");
    assert!(pages[0].content.contains(LONG_LINE));
    assert_eq!(pages[0].native_content, None);
    assert!(!page_json(&pages[0]).contains_key("native_content"));

    assert_ocr_content(&pages[1]);
    assert_eq!(pages[1].native_content.as_deref(), Some(text_layer[1].as_str()));
    assert!(text_layer[1].contains(SHORT_LINE));
}

#[test]
#[serial_test::serial]
fn a_blank_text_layer_sent_to_ocr_carries_no_native_content() {
    let pdf = pdf_with_pages(&[FixturePage::Scan]);

    let pages = extract_pages(&pdf, &ocr_config(true));

    assert_eq!(pages.len(), 1);
    assert_ocr_content(&pages[0]);
    assert_eq!(pages[0].native_content, None);
    assert!(!page_json(&pages[0]).contains_key("native_content"));
}

#[test]
#[serial_test::serial]
fn a_stamped_scan_keeps_ocr_content_and_returns_the_stamp_when_asked() {
    let pdf = pdf_with_pages(&[FixturePage::StampedScan(STAMP)]);
    let text_layer = text_layer_of_pages(&pdf);
    assert!(text_layer[0].contains(STAMP), "the stamp is real text: {text_layer:?}");

    let default_pages = extract_pages(&pdf, &ocr_config(false));
    assert_ocr_content(&default_pages[0]);
    assert!(!page_json(&default_pages[0]).contains_key("native_content"));

    let kept_pages = extract_pages(&pdf, &ocr_config(true));
    assert_eq!(kept_pages[0].content, default_pages[0].content);
    assert_eq!(kept_pages[0].native_content.as_deref(), Some(text_layer[0].as_str()));
}

#[test]
#[serial_test::serial]
fn forced_ocr_keeps_the_text_layer_it_replaces() {
    let pdf = pdf_with_pages(&[FixturePage::Text(LONG_LINE)]);
    let text_layer = text_layer_of_pages(&pdf);
    assert!(text_layer[0].contains(LONG_LINE));
    let forced = |keep_native_content: bool| ExtractionConfig {
        force_ocr: true,
        ..ocr_config(keep_native_content)
    };

    let kept_pages = extract_pages(&pdf, &forced(true));
    assert_ocr_content(&kept_pages[0]);
    assert_eq!(kept_pages[0].native_content.as_deref(), Some(text_layer[0].as_str()));

    let default_pages = extract_pages(&pdf, &forced(false));
    assert_ocr_content(&default_pages[0]);
    assert_eq!(default_pages[0].native_content, None);
}

fn assert_needs_extract_pages(error: &str) {
    assert!(
        error.contains("`pages.keep_native_content` needs `pages.extract_pages = true`"),
        "the error must name both settings; got: {error}"
    );
}

#[test]
fn extraction_rejects_the_setting_without_extract_pages() {
    let pdf = pdf_with_pages(&[FixturePage::Text(SHORT_LINE)]);
    let invalid = ExtractionConfig {
        pages: Some(PageConfig {
            keep_native_content: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };

    let error = extract_bytes_document_blocking(&pdf, PDF_MIME_TYPE, &invalid)
        .expect_err("the setting without extract_pages must be rejected");
    assert!(matches!(error, XbergError::Validation { .. }), "got: {error:?}");
    assert_needs_extract_pages(&error.to_string());

    let valid = ExtractionConfig {
        pages: Some(PageConfig {
            extract_pages: true,
            keep_native_content: true,
            ..Default::default()
        }),
        use_cache: false,
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(&pdf, PDF_MIME_TYPE, &valid).expect("a valid config extracts");
    let pages = document.pages.expect("extract_pages gives pages");
    assert!(pages[0].content.contains(SHORT_LINE));
    assert_eq!(pages[0].native_content, None, "without OCR no content is replaced");
}

#[test]
fn a_per_input_override_is_rejected_without_extract_pages() {
    let pdf = pdf_with_pages(&[FixturePage::Text(SHORT_LINE)]);
    let input = |pages: PageConfig| BytesInput {
        content: pdf.clone(),
        mime_type: PDF_MIME_TYPE.to_string(),
        config: Some(FileExtractionConfig {
            pages: Some(pages),
            ..Default::default()
        }),
    };
    let base = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };

    let error = extract_bytes_documents_blocking(
        vec![input(PageConfig {
            keep_native_content: true,
            ..Default::default()
        })],
        &base,
    )
    .expect_err("an override with the setting and without extract_pages must be rejected");
    assert_needs_extract_pages(&error.to_string());

    let documents = extract_bytes_documents_blocking(
        vec![input(PageConfig {
            extract_pages: true,
            keep_native_content: true,
            ..Default::default()
        })],
        &base,
    )
    .expect("a valid override extracts");
    assert!(
        documents[0].pages.as_ref().expect("the override turns pages on")[0]
            .content
            .contains(SHORT_LINE)
    );
}

#[tokio::test]
async fn a_per_input_override_on_one_input_is_rejected_without_extract_pages() {
    let pdf = pdf_with_pages(&[FixturePage::Text(SHORT_LINE)]);
    let input = |pages: PageConfig| {
        let mut input = ExtractInput::from_bytes(pdf.clone(), PDF_MIME_TYPE, None);
        input.config = Some(FileExtractionConfig {
            pages: Some(pages),
            ..Default::default()
        });
        input
    };
    let base = ExtractionConfig {
        use_cache: false,
        ..Default::default()
    };

    let rejected = input(PageConfig {
        keep_native_content: true,
        ..Default::default()
    });
    let error = xberg::extract(rejected, &base)
        .await
        .expect_err("an override with the setting and without extract_pages must be rejected");
    assert_needs_extract_pages(&error.to_string());

    let accepted = input(PageConfig {
        extract_pages: true,
        keep_native_content: true,
        ..Default::default()
    });
    let output = xberg::extract(accepted, &base)
        .await
        .expect("a valid override extracts");
    assert!(
        output.results[0].pages.as_ref().expect("the override turns pages on")[0]
            .content
            .contains(SHORT_LINE)
    );
}

#[test]
fn a_config_file_and_a_json_override_are_rejected_without_extract_pages() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let invalid_path = directory.path().join("invalid.toml");
    std::fs::write(&invalid_path, "[pages]\nkeep_native_content = true\n").expect("the config file is written");
    let error = ExtractionConfig::from_toml_file(&invalid_path).expect_err("the file loader must reject the config");
    assert_needs_extract_pages(&error.to_string());

    let valid_path = directory.path().join("valid.toml");
    std::fs::write(
        &valid_path,
        "[pages]\nextract_pages = true\nkeep_native_content = true\n",
    )
    .expect("the config file is written");
    let loaded = ExtractionConfig::from_toml_file(&valid_path).expect("a valid config file loads");
    assert!(loaded.pages.expect("the file sets pages").keep_native_content);

    let base = ExtractionConfig::default();
    let error = xberg::core::config::merge::merge_config_json(&base, r#"{"pages": {"keep_native_content": true}}"#)
        .expect_err("the JSON override must be rejected");
    assert_needs_extract_pages(&error);

    let merged = xberg::core::config::merge::merge_config_json(
        &base,
        r#"{"pages": {"extract_pages": true, "keep_native_content": true}}"#,
    )
    .expect("a valid JSON override merges");
    assert!(merged.pages.expect("the override sets pages").keep_native_content);
}
