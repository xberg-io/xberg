//! The OCR text of a picture on a PPT slide, in the content of that slide.
//!
//! The deck is `tests/fixtures/ppt/slide_with_picture.ppt`: one slide with the text
//! "Stock list" and one picture, saved as PPT by LibreOffice. A stub OCR backend returns
//! fixed words for the picture and counts its calls, so each test can see where the words
//! land in `content` and in the content of the slide in `pages`.

#![cfg(all(feature = "office", feature = "ocr"))]

mod helpers;

use async_trait::async_trait;
use helpers::extract_bytes_document_blocking;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg::ExtractedDocument;
use xberg::core::config::{ExtractionConfig, ImageExtractionConfig, OcrConfig, OutputFormat, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};

const POWERPOINT_97_MIME_TYPE: &str = "application/vnd.ms-powerpoint";
/// The words the stub reads from the picture.
const PICTURE_WORDS: &str = "Crate 17 holds forty blue lanterns";
/// The text that the author typed on the slide.
const SLIDE_TEXT: &str = "Stock list";

fn deck() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ppt/slide_with_picture.ppt"
    ))
    .expect("the fixture is in the repository")
}

struct StubBackend {
    name: &'static str,
    reply: &'static str,
    calls: Arc<AtomicUsize>,
}

impl Plugin for StubBackend {
    fn name(&self) -> &str {
        self.name
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
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut document = ExtractedDocument::default();
        document.content = self.reply.to_string();
        Ok(document)
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

struct BackendGuard(&'static str);

impl Drop for BackendGuard {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend(self.0);
    }
}

/// One extraction and the OCR calls it made.
struct Run {
    document: ExtractedDocument,
    ocr_calls: usize,
}

impl Run {
    /// The content of the only slide. The deck has one slide, so the result has one page.
    fn slide_content(&self) -> &str {
        let pages = self
            .document
            .pages
            .as_ref()
            .expect("the picture is on slide 1, so the result has pages");
        assert_eq!(pages.len(), 1, "the deck has one slide");
        assert_eq!(pages[0].page_number, 1);
        pages[0].content.as_str()
    }
}

/// How many times the picture's words are in `text`.
fn words_in(text: &str) -> usize {
    text.matches(PICTURE_WORDS).count()
}

/// Extract the deck with a stub backend registered as `backend` that reads `reply` from the
/// picture. The config has an `ocr` block for the stub, page extraction on, plain output and
/// no `images` block; `configure` changes it.
fn extract(backend: &'static str, reply: &'static str, configure: impl FnOnce(&mut ExtractionConfig)) -> Run {
    let calls = Arc::new(AtomicUsize::new(0));
    let _ = unregister_ocr_backend(backend);
    register_ocr_backend(Arc::new(StubBackend {
        name: backend,
        reply,
        calls: Arc::clone(&calls),
    }))
    .expect("the stub backend registers");
    let _guard = BackendGuard(backend);

    let mut config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: backend.to_string(),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        output_format: OutputFormat::Plain,
        use_cache: false,
        ..Default::default()
    };
    configure(&mut config);
    let document =
        extract_bytes_document_blocking(&deck(), POWERPOINT_97_MIME_TYPE, &config).expect("the deck extracts");
    Run {
        document,
        ocr_calls: calls.load(Ordering::SeqCst),
    }
}

#[test]
fn plain_slide_content_has_the_picture_words_once() {
    let run = extract("ppt-slide-picture-plain", PICTURE_WORDS, |_| {});

    assert_eq!(run.ocr_calls, 1);
    assert!(run.document.content.contains(SLIDE_TEXT), "{:?}", run.document.content);
    assert_eq!(words_in(&run.document.content), 1, "{:?}", run.document.content);
    assert_eq!(words_in(run.slide_content()), 1, "{:?}", run.slide_content());
}

#[test]
fn plain_slide_content_has_the_words_with_every_images_block_setting() {
    let settings: [(&str, &'static str, Option<ImageExtractionConfig>); 4] = [
        ("no images block", "ppt-slide-picture-images-absent", None),
        (
            "default images block",
            "ppt-slide-picture-images-default",
            Some(ImageExtractionConfig::default()),
        ),
        (
            "append_ocr_text off",
            "ppt-slide-picture-images-no-append",
            Some(ImageExtractionConfig {
                append_ocr_text: false,
                ..Default::default()
            }),
        ),
        (
            "ocr_text_only on",
            "ppt-slide-picture-images-text-only",
            Some(ImageExtractionConfig {
                ocr_text_only: true,
                ..Default::default()
            }),
        ),
    ];

    for (label, backend, images) in settings {
        let run = extract(backend, PICTURE_WORDS, |config| config.images = images);

        assert_eq!(run.ocr_calls, 1, "{label}");
        assert_eq!(
            words_in(&run.document.content),
            1,
            "{label}: {:?}",
            run.document.content
        );
        assert_eq!(words_in(run.slide_content()), 1, "{label}: {:?}", run.slide_content());
    }
}

#[test]
fn formatted_slide_content_has_the_words_once() {
    let formats = [
        ("ppt-slide-picture-markdown", OutputFormat::Markdown),
        ("ppt-slide-picture-djot", OutputFormat::Djot),
        ("ppt-slide-picture-html", OutputFormat::Html),
    ];

    for (backend, format) in formats {
        let label = format!("{format:?}");
        let run = extract(backend, PICTURE_WORDS, |config| config.output_format = format);

        assert_eq!(run.ocr_calls, 1, "{label}");
        assert!(
            run.document.content.contains(SLIDE_TEXT),
            "{label}: {:?}",
            run.document.content
        );
        assert_eq!(
            words_in(&run.document.content),
            1,
            "{label}: {:?}",
            run.document.content
        );
        assert_eq!(words_in(run.slide_content()), 1, "{label}: {:?}", run.slide_content());
    }
}
