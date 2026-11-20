//! Issue #3260: the automatic OCR route must not pay an OCR read per occurrence of the same
//! embedded image, and must still read genuinely mixed documents.
//!
//! The native control paints one image XObject on every page and counts backend reads: the
//! shared bytes are one read, so the count is the number of distinct images, not pages. The
//! mixed control pairs a native page with a scanned page and asserts both readers still run.

#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

mod helpers;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use xberg::core::config::{ExtractionConfig, OcrConfig, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::ExtractedDocument;
use xberg_native_pdf::{PdfDocument, rendering};

const PAGE_WIDTH: f32 = 612.0;
const PAGE_HEIGHT: f32 = 792.0;
const RASTER_DPI: u32 = 150;
const IMAGE_TEXT: &str = "shared image read";
const NATIVE_SENTENCES: [&str; 3] = [
    "The first native page keeps its original sentences.",
    "The second native page keeps its original sentences.",
    "The third native page keeps its original sentences.",
];
const SCANNED_LINES: [&str; 3] = [
    "The coastal archive opened its wooden shutters before sunrise.",
    "Volunteers carried maps into the reading room and recorded each label.",
    "Beyond the courtyard a narrow pathway led toward the lighthouse.",
];

struct CountingBackend {
    name: &'static str,
    calls: Arc<AtomicUsize>,
}

impl Plugin for CountingBackend {
    fn name(&self) -> &str {
        self.name
    }

    fn version(&self) -> String {
        "0.0.0".to_string()
    }

    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl OcrBackend for CountingBackend {
    async fn process_image(&self, _image_bytes: &[u8], _config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut document = ExtractedDocument::default();
        document.content = IMAGE_TEXT.to_string();
        Ok(document)
    }

    fn supports_language(&self, _lang: &str) -> bool {
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

fn native_bytes() -> Vec<u8> {
    let mut document = Document::with_version("1.4");
    let pages = document.new_object_id();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let image = document.add_object(image_stream());
    let mut children = Vec::new();
    for text in NATIVE_SENTENCES {
        let content = format!("BT /F1 12 Tf 72 700 Td ({text}) Tj ET\nq 24 0 0 24 72 600 cm /Im0 Do Q\n");
        let content = document.add_object(Stream::new(Dictionary::new(), content.into_bytes()));
        let page = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages,
            "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
            "Resources" => dictionary! {
                "Font" => dictionary! { "F1" => font },
                "XObject" => dictionary! { "Im0" => image },
            },
            "Contents" => content,
        });
        children.push(Object::Reference(page));
    }
    finish(document, pages, children)
}

/// One RGB image XObject, added to the document once and painted on every page.
fn image_stream() -> Stream {
    let pixels: Vec<u8> = (0..8 * 8).flat_map(|_| [0x20u8, 0x40, 0x60]).collect();
    Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
            "Width" => 8, "Height" => 8,
        },
        pixels,
    )
}

fn finish(mut document: Document, pages: lopdf::ObjectId, children: Vec<Object>) -> Vec<u8> {
    let count = children.len() as i64;
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => children, "Count" => count }),
    );
    let root = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    document.trailer.set("Root", root);
    document.compress();
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("serialize synthetic PDF");
    bytes
}

/// Native pages with one shared image XObject, preceded by a scanned page rendered from vector
/// text, for the mixed control.
fn mixed_bytes() -> Vec<u8> {
    let mut document = Document::with_version("1.4");
    let pages = document.new_object_id();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let mut children = Vec::new();

    let native = "BT /F1 12 Tf 72 700 Td (Native page keeps its extracted prose.) Tj ET\n";
    let content = document.add_object(Stream::new(Dictionary::new(), native.as_bytes().to_vec()));
    let page = document.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
        "Contents" => content,
    });
    children.push(Object::Reference(page));

    let raster = document.add_object(render_scanned_page());
    let content = document.add_object(Stream::new(
        Dictionary::new(),
        format!("q {PAGE_WIDTH} 0 0 {PAGE_HEIGHT} 0 0 cm /Scan Do Q\n").into_bytes(),
    ));
    let page = document.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
        "Resources" => dictionary! { "XObject" => dictionary! { "Scan" => raster } },
        "Contents" => content,
    });
    children.push(Object::Reference(page));

    finish(document, pages, children)
}

/// A full-page RGB raster of [`SCANNED_LINES`], rendered from a temporary vector page.
fn render_scanned_page() -> Stream {
    let mut operators = String::from("BT /F1 14 Tf\n");
    for (line, text) in SCANNED_LINES.iter().enumerate() {
        let y = 700.0 - 24.0 * line as f32;
        operators.push_str(&format!("1 0 0 1 72 {y} Tm ({text}) Tj\n"));
    }
    operators.push_str("ET\n");

    let mut source = Document::with_version("1.4");
    let pages = source.new_object_id();
    let font = source.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let content = source.add_object(Stream::new(Dictionary::new(), operators.into_bytes()));
    let page = source.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
        "Contents" => content,
    });
    let count = 1i64;
    source.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => count,
        }),
    );
    let root = source.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    source.trailer.set("Root", root);
    let mut source_bytes = Vec::new();
    source.save_to(&mut source_bytes).expect("serialize vector source");

    let vector = PdfDocument::from_bytes(source_bytes).expect("open vector source");
    let rendered = rendering::render_page(&vector, 0, &rendering::RenderOptions::with_dpi(RASTER_DPI))
        .expect("render the scanned page");
    let image = image::load_from_memory(&rendered.data)
        .expect("decode the rendered page")
        .into_rgb8();
    Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
            "Width" => image.width(), "Height" => image.height(),
        },
        image.into_raw(),
    )
}

fn config(ocr: OcrConfig) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ocr: Some(ocr),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn extract(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    helpers::extract_bytes_document(bytes, "application/pdf", config)
        .await
        .expect("extract the synthetic PDF")
}

#[tokio::test]
async fn automatic_ocr_reads_a_repeated_native_image_once() {
    const BACKEND: &str = "repeated-native-image-count-3260";
    let calls = Arc::new(AtomicUsize::new(0));
    register_ocr_backend(Arc::new(CountingBackend {
        name: BACKEND,
        calls: Arc::clone(&calls),
    }))
    .expect("register counting OCR backend");
    let _guard = BackendGuard(BACKEND);

    let document = extract(
        &native_bytes(),
        &config(OcrConfig {
            backend: BACKEND.to_string(),
            ..Default::default()
        }),
    )
    .await;

    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "one image XObject painted on three pages is one OCR read, not three"
    );
    let pages = document.pages.as_deref().expect("page text requested");
    assert_eq!(pages.len(), NATIVE_SENTENCES.len());
    for (page, native) in pages.iter().zip(NATIVE_SENTENCES) {
        assert!(
            page.content.contains(native),
            "page {} lost its native sentence: {}",
            page.page_number,
            page.content
        );
    }
}

#[tokio::test]
async fn automatic_ocr_still_reads_a_scanned_page_in_a_mixed_document() {
    let document = extract(&mixed_bytes(), &config(OcrConfig::default())).await;

    let pages = document.pages.as_deref().expect("page text requested");
    assert_eq!(pages.len(), 2);
    assert!(
        pages[0].content.contains("Native page keeps its extracted prose."),
        "native page text changed: {}",
        pages[0].content
    );
    let scanned = pages[1].content.to_lowercase();
    for words in [
        "coastal archive opened",
        "volunteers carried maps",
        "narrow pathway led",
    ] {
        assert!(
            scanned.contains(words),
            "scanned page lost {words:?}: {}",
            pages[1].content
        );
    }
}
