//! The OCR text of an image file when the caller also asks for its images.
//!
//! The image extractor reads the file with OCR, and that read is the document. An `images` block
//! that extracts images returns the file as an image, and image OCR must not read that picture
//! again when the first read gave text or a table. Each test sends PNG bytes through the public
//! extraction entry point with a stub OCR backend that counts its calls and answers each call
//! from a list, so a test sees how many reads were made and which answer reached `content`.

#![cfg(feature = "ocr")]

mod helpers;

use async_trait::async_trait;
use helpers::extract_bytes_document_blocking;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg::ExtractedDocument;
use xberg::core::config::{ExtractionConfig, ImageExtractionConfig, OcrConfig, OutputFormat};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::Table;

const FORMATS: [(OutputFormat, &str); 4] = [
    (OutputFormat::Plain, "plain"),
    (OutputFormat::Markdown, "markdown"),
    (OutputFormat::Djot, "djot"),
    (OutputFormat::Html, "html"),
];

/// What one OCR call returns.
#[derive(Clone)]
enum Read {
    Words(&'static str),
    /// A table with these rows and no other text.
    TableOnly(&'static [&'static [&'static str]]),
}

impl Read {
    fn document(&self) -> ExtractedDocument {
        let mut document = ExtractedDocument::default();
        match self {
            Read::Words(words) => document.content = (*words).to_string(),
            Read::TableOnly(rows) => {
                let mut lines: Vec<String> = rows.iter().map(|row| format!("| {} |", row.join(" | "))).collect();
                lines.insert(1, format!("|{}", " --- |".repeat(rows[0].len())));
                document.tables = vec![Table {
                    cells: rows
                        .iter()
                        .map(|row| row.iter().map(|cell| (*cell).to_string()).collect())
                        .collect(),
                    markdown: lines.join("\n"),
                    page_number: 1,
                    ..Default::default()
                }];
            }
        }
        document
    }
}

/// Answers call N with read N. A call after the last read gets the last read again.
struct ReadsInOrder {
    name: &'static str,
    reads: Vec<Read>,
    calls: Arc<AtomicUsize>,
}

impl Plugin for ReadsInOrder {
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
impl OcrBackend for ReadsInOrder {
    async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.reads[call.min(self.reads.len() - 1)].document())
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

/// A grey PNG. The stub does not look at the pixels.
fn png() -> Vec<u8> {
    let picture = image::RgbImage::from_pixel(300, 120, image::Rgb([160, 160, 160]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    picture
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("the PNG encodes");
    bytes.into_inner()
}

/// One extraction of the PNG and the number of OCR calls it made.
struct Run {
    document: ExtractedDocument,
    ocr_calls: usize,
}

impl Run {
    fn count(&self, words: &str) -> usize {
        self.document.content.matches(words).count()
    }

    /// The OCR result of the returned image, and whether the image has its bytes.
    fn returned_image(&self) -> (Option<&str>, bool) {
        let image = &self.document.images.as_ref().expect("the image is returned")[0];
        (
            image.ocr_result.as_ref().map(|result| result.content.as_str()),
            !image.data.is_empty(),
        )
    }
}

/// Extract the PNG with a stub registered as `backend` that gives `first` and then `second`.
fn extract(
    backend: &'static str,
    first: Read,
    second: Read,
    format: OutputFormat,
    images: Option<ImageExtractionConfig>,
) -> Run {
    let calls = Arc::new(AtomicUsize::new(0));
    let _ = unregister_ocr_backend(backend);
    register_ocr_backend(Arc::new(ReadsInOrder {
        name: backend,
        reads: vec![first, second],
        calls: Arc::clone(&calls),
    }))
    .expect("the stub backend registers");
    let _guard = BackendGuard(backend);

    let config = ExtractionConfig {
        ocr: Some(OcrConfig {
            backend: backend.to_string(),
            ..Default::default()
        }),
        images,
        output_format: format,
        use_cache: false,
        ..Default::default()
    };
    let document = extract_bytes_document_blocking(&png(), "image/png", &config).expect("the PNG extracts");
    Run {
        document,
        ocr_calls: calls.load(Ordering::SeqCst),
    }
}

/// Makes the `images` block of one setting.
type ImagesBlock = fn() -> ImageExtractionConfig;

fn default_images() -> ImageExtractionConfig {
    ImageExtractionConfig::default()
}

fn ocr_text_only() -> ImageExtractionConfig {
    ImageExtractionConfig {
        ocr_text_only: true,
        ..Default::default()
    }
}

fn no_appended_ocr_text() -> ImageExtractionConfig {
    ImageExtractionConfig {
        append_ocr_text: false,
        ..Default::default()
    }
}

const SETTINGS: [(ImagesBlock, &str); 3] = [
    (default_images, "default"),
    (ocr_text_only, "ocr_text_only"),
    (no_appended_ocr_text, "append_ocr_text=false"),
];

const WORDS: &str = "CRATE 17 KEEP DRY";

#[test]
fn an_image_file_with_text_is_read_one_time() {
    for (images, setting) in SETTINGS {
        for (format, format_name) in FORMATS {
            let run = extract(
                "image-file-read-one-time",
                Read::Words(WORDS),
                Read::Words(WORDS),
                format,
                Some(images()),
            );

            assert_eq!(run.ocr_calls, 1, "{setting}, {format_name}: OCR calls");
            assert_eq!(
                run.count(WORDS),
                1,
                "{setting}, {format_name}: {:?}",
                run.document.content
            );
            assert_eq!(
                run.returned_image(),
                (None, true),
                "{setting}, {format_name}: the returned image has its bytes and no OCR result of its own"
            );
        }
    }
}

/// The `images` block returns the picture. It does not change the text of the file: `content`
/// is the same as with image OCR off, and plain `content` is the same as with no block.
#[test]
fn the_images_block_does_not_change_the_text_of_an_image_file() {
    for (images, setting) in SETTINGS {
        for (format, format_name) in FORMATS {
            let read = |images: Option<ImageExtractionConfig>| -> Run {
                extract(
                    "image-file-images-block",
                    Read::Words(WORDS),
                    Read::Words("SECOND READ"),
                    format.clone(),
                    images,
                )
            };
            let with_block = read(Some(images()));
            let image_ocr_off = read(Some(ImageExtractionConfig {
                run_ocr_on_images: false,
                ..images()
            }));

            assert_eq!(with_block.count(WORDS), 1, "{setting}, {format_name}");
            assert_eq!(
                with_block.document.content, image_ocr_off.document.content,
                "{setting}, {format_name}: against image OCR off"
            );
            if format == OutputFormat::Plain {
                assert_eq!(
                    with_block.document.content,
                    read(None).document.content,
                    "{setting}: against no images block"
                );
            }
        }
    }
}

/// The first read has a part of the words. A second read would give more words or other words.
/// That second read is not requested, so its answer is in no part of the result.
#[test]
fn a_second_answer_is_not_requested_after_a_partial_first_read() {
    for second in ["CRATE 17 KEEP DRY", "KEEP DRY"] {
        for (format, format_name) in FORMATS {
            let run = extract(
                "image-file-partial-first-read",
                Read::Words("CRATE 17"),
                Read::Words(second),
                format,
                Some(default_images()),
            );

            assert_eq!(run.ocr_calls, 1, "{second:?}, {format_name}: OCR calls");
            assert_eq!(
                (run.count("CRATE 17"), run.count("KEEP DRY")),
                (1, 0),
                "{second:?}, {format_name}: {:?}",
                run.document.content
            );
            assert_eq!(run.returned_image(), (None, true), "{second:?}, {format_name}");
        }
    }
}

/// The first read gives a table and no other text. The table is the text of the file, so the
/// picture is not read again.
#[test]
fn a_table_only_first_read_is_read_one_time() {
    const TABLE_ROWS: &[&[&str]] = &[&["Item", "Bay"], &["PALLET", "NORTH"]];
    for (format, format_name) in FORMATS {
        let run = extract(
            "image-file-table-only-first-read",
            Read::TableOnly(TABLE_ROWS),
            Read::Words("PALLET NORTH SEALED"),
            format,
            Some(default_images()),
        );

        assert_eq!(run.ocr_calls, 1, "{format_name}: OCR calls");
        assert_eq!(run.document.tables.len(), 1, "{format_name}: the table is returned");
        assert_eq!(
            (run.count("PALLET"), run.count("NORTH"), run.count("SEALED")),
            (1, 1, 0),
            "{format_name}: {:?}",
            run.document.content
        );
        assert_eq!(run.returned_image(), (None, true), "{format_name}");
    }
}

/// The first read gives no text and no table, so the file has no text of its own. Image OCR
/// reads the picture, and its words are the only text.
#[test]
fn an_empty_first_read_keeps_the_words_of_the_second_read() {
    let settings: [(ImagesBlock, &str); 2] = [(default_images, "default"), (ocr_text_only, "ocr_text_only")];
    for (images, setting) in settings {
        for (format, format_name) in FORMATS {
            let run = extract(
                "image-file-empty-first-read",
                Read::Words(""),
                Read::Words(WORDS),
                format,
                Some(images()),
            );

            assert_eq!(run.ocr_calls, 2, "{setting}, {format_name}: OCR calls");
            assert_eq!(
                run.count(WORDS),
                1,
                "{setting}, {format_name}: {:?}",
                run.document.content
            );
            assert_eq!(run.returned_image(), (Some(WORDS), true), "{setting}, {format_name}");
        }
    }
}
