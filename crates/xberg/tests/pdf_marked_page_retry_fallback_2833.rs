#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

//! The fallback conditions of the per-page retry that keeps a marked page's native text beside a
//! scan (the `RunFallback` arm of the OCR gate, #2812's follow-up #2833). The registered backend
//! tells pages apart by the dimensions of the raster it is handed, so a test can let the
//! whole-document read cover a page and fail or blank only the per-page retry that reads the same
//! page again. The backend replaces the built-in `tesseract` entry of this process's registry, so
//! every test here is serial and the file holds no other tests.

mod helpers;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use xberg::core::config::{OcrStrategy, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::{ExtractedDocument, ExtractionMethod};
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const THEN_SCANNED: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/khmer-then-scanned-english.pdf");
const ENGLISH: &str = "The quarterly report contains useful readable text about the country and its people.";
const WEAK: &str = "So be it.";
const KHMER_PAGE_LETTERS: usize = 222;
/// The rasters of this fixture set, by rendered dimensions: the Khmer page and the weak page
/// render at the preprocessing DPI, the scan page at its own higher source DPI.
const KHMER_DIMS: (u32, u32) = (1275, 1650);
const SCAN_DIMS: (u32, u32) = (2550, 3300);
const WEAK_DIMS: (u32, u32) = (1042, 1459);

/// The scan fixture #2812's tests read, with one unmarked page appended whose weak native text
/// fails the OCR gate's size thresholds. The appended page's MediaBox differs from the fixture's
/// own pages, so the registered backend tells its raster apart by dimensions alone.
fn khmer_scan_then_weak_native() -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};
    let mut document = Document::load_mem(THEN_SCANNED).unwrap();
    let pages_id = document
        .catalog_mut()
        .unwrap()
        .get(b"Pages")
        .unwrap()
        .as_reference()
        .unwrap();
    let font = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let content = document.add_object(Stream::new(
        dictionary! {},
        format!("BT /F1 11 Tf 50 600 Td ({WEAK}) Tj ET\n").into_bytes(),
    ));
    let page = document.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content,
        "MediaBox" => vec![0.into(), 0.into(), 500.into(), 700.into()],
        "Resources" => dictionary! {"Font" => dictionary! {"F1" => font}},
    });
    let count = {
        let pages = document.get_object_mut(pages_id).unwrap().as_dict_mut().unwrap();
        let kids = match pages.get_mut(b"Kids") {
            Ok(Object::Array(kids)) => kids,
            kids => panic!("the fixture's page tree holds a Kids array: {kids:?}"),
        };
        kids.push(page.into());
        kids.len()
    };
    document
        .get_object_mut(pages_id)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Count", Object::Integer(count as i64));
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

/// What a scripted raster's calls return, one per call in call order.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Read {
    /// The text a Latin-only first stage reads.
    Latin,
    /// A read that comes back with no text at all.
    Blank,
    /// A read that fails.
    Fails,
}

/// Reads pages apart by the dimensions of the raster it is handed: a raster script's calls
/// consume its outcomes in call order, and a script that runs out repeats its last outcome, so a
/// test can let the whole-document read cover a page and fail or blank only the per-page retry
/// that reads the same page again. The scan flag of the handed config (`known_full_page_scan`)
/// separates a page's own reads from the reads of its embedded image, whose dimensions the page
/// render repeats. A raster with no script reads Latin.
struct ByRaster {
    scripts: Mutex<HashMap<ScriptKey, Vec<Read>>>,
    next: Mutex<HashMap<ScriptKey, usize>>,
    calls: AtomicUsize,
}

/// A raster script's key: the raster's dimensions and whether the handed config marks the read as
/// a known full-page scan (the page's own read, not an embedded-image read).
type ScriptKey = ((u32, u32), bool);

impl ByRaster {
    fn new(scripts: HashMap<ScriptKey, Vec<Read>>) -> Self {
        Self {
            scripts: Mutex::new(scripts),
            next: Mutex::new(HashMap::new()),
            calls: AtomicUsize::new(0),
        }
    }
}

impl Plugin for ByRaster {
    fn name(&self) -> &str {
        "tesseract"
    }
    fn version(&self) -> String {
        "1.0.0".to_owned()
    }
    fn initialize(&self) -> xberg::Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> xberg::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl OcrBackend for ByRaster {
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
    fn supports_language(&self, _: &str) -> bool {
        true
    }
    async fn process_image(&self, image: &[u8], config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let dims = image::load_from_memory(image).map_or((0, 0), |decoded| (decoded.width(), decoded.height()));
        let scan_flag = config
            .backend_options
            .as_ref()
            .and_then(|options| options.get("known_full_page_scan"))
            .is_some_and(|value| value.as_bool().unwrap_or(false));
        let key = (dims, scan_flag);
        let outcome = {
            let scripts = self.scripts.lock().unwrap();
            let mut next = self.next.lock().unwrap();
            match scripts.get(&key) {
                None => Read::Latin,
                Some(outcomes) => {
                    let index = next.get(&key).copied().unwrap_or(0);
                    let outcome = outcomes.get(index).or(outcomes.last()).copied().unwrap_or(Read::Latin);
                    next.insert(key, (index + 1).min(outcomes.len().saturating_sub(1)));
                    outcome
                }
            }
        };
        let mut document = ExtractedDocument::default();
        match outcome {
            Read::Latin => document.content = ENGLISH.to_owned(),
            Read::Blank => {}
            Read::Fails => return Err(xberg::XbergError::ocr("the page raster was refused")),
        }
        Ok(document)
    }
}

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = unregister_ocr_backend("tesseract");
    }
}

fn khmer_count(text: &str) -> usize {
    text.chars().filter(|c| ('\u{1780}'..='\u{17ff}').contains(c)).count()
}

fn script(pages: &[(ScriptKey, &[Read])]) -> HashMap<ScriptKey, Vec<Read>> {
    pages.iter().map(|(key, reads)| (*key, reads.to_vec())).collect()
}

/// Runs the whole-document read and the marked-page retry of `source` with a Latin-only first
/// stage whose backend is `backend`, through the public API.
fn read(source: &[u8], backend: Arc<ByRaster>) -> ExtractedDocument {
    let _guard = Guard;
    register_ocr_backend(backend).unwrap();
    let pipeline = serde_json::json!({"stages": [
        {"backend": "tesseract", "priority": 100, "language": ["khm"]},
    ]});
    let config = ExtractionConfig {
        use_cache: false,
        output_format: OutputFormat::Plain,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            pipeline: Some(serde_json::from_value(pipeline).unwrap()),
            ..Default::default()
        }),
        ocr_strategy: OcrStrategy::Auto,
        ..Default::default()
    };
    helpers::extract_bytes_document_blocking(source, "application/pdf", &config).unwrap()
}

fn warnings_containing<'a>(document: &'a ExtractedDocument, needle: &str) -> Vec<&'a str> {
    document
        .processing_warnings
        .iter()
        .filter(|warning| warning.source == "ocr" && warning.message.contains(needle))
        .map(|warning| warning.message.as_ref())
        .collect()
}

/// Case A: a per-page read that fails but is recovered by the page's native text must not drop the
/// retry. The marked page keeps its native text, the weak page keeps its own, and no warning
/// claims the targeted read could not read a page the whole read had read.
#[test]
#[serial_test::serial]
fn a_recovered_per_page_read_failure_keeps_the_retry_and_the_marked_page_native_text() {
    let backend = Arc::new(ByRaster::new(script(&[
        // The weak page: the whole-document read covers it, the per-page retry fails on it. Its
        // failure is recovered by the page's own native text. A vector page's reads carry no scan
        // flag, and the page has no embedded image to re-read.
        ((WEAK_DIMS, false), &[Read::Latin, Read::Fails]),
        // The scan: both page reads read it.
        ((SCAN_DIMS, true), &[Read::Latin, Read::Latin]),
    ])));
    let source = khmer_scan_then_weak_native();
    let automatic = read(&source, backend.clone());
    assert_eq!(
        backend.calls.load(Ordering::SeqCst),
        6,
        "the whole read and the per-page retry each read every page once"
    );
    assert_eq!(
        khmer_count(&automatic.content),
        KHMER_PAGE_LETTERS,
        "the marked page must keep its native text: {:?}",
        automatic.content
    );
    assert!(
        automatic.content.contains(WEAK),
        "the weak page must keep its native text: {:?}",
        automatic.content
    );
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Mixed));
    assert!(
        warnings_containing(&automatic, "could not read every other page").is_empty(),
        "{:?}",
        automatic.processing_warnings
    );
    let failures: Vec<_> = automatic
        .ocr_page_failures
        .iter()
        .flatten()
        .filter(|failure| failure.page == 3)
        .collect();
    assert_eq!(failures.len(), 1, "{:?}", automatic.ocr_page_failures);
    assert!(
        failures[0].recovered,
        "the weak page's native text recovers its failed read: {:?}",
        failures[0]
    );
}

/// Case B: a scan the whole read read blank, whose per-page read fails, must not drop the retry
/// either: the marked page keeps its native text, the scan stays unread, and no warning claims a
/// page could not be read.
#[test]
#[serial_test::serial]
fn a_scan_the_whole_read_read_blank_does_not_drop_the_retry_nor_warn_of_an_unread_page() {
    let backend = Arc::new(ByRaster::new(script(&[
        // The scan: the whole read reads it blank, the whole read's embedded-image retry of it
        // reads blank too, the marked-page retry's read of it fails, and that retry's own
        // embedded-image retry reads blank again. A page read and the embedded-image reads share
        // the raster's dimensions; the scan flag separates each page read from the reads of the
        // page's picture.
        ((SCAN_DIMS, true), &[Read::Blank, Read::Blank, Read::Fails, Read::Blank]),
        ((SCAN_DIMS, false), &[Read::Blank]),
    ])));
    let automatic = read(THEN_SCANNED, backend.clone());
    assert_eq!(
        backend.calls.load(Ordering::SeqCst),
        7,
        "the whole read and the per-page retry each read every page once, and the scan's picture is \
         re-read after each blank page read"
    );
    assert_eq!(
        khmer_count(&automatic.content),
        KHMER_PAGE_LETTERS,
        "the marked page must keep its native text: {:?}",
        automatic.content
    );
    let pages = automatic.pages.as_ref().unwrap();
    assert!(
        pages[1].content.trim().is_empty(),
        "the scan stays unread: {:?}",
        pages[1].content
    );
    assert!(
        warnings_containing(&automatic, "could not read every other page").is_empty(),
        "{:?}",
        automatic.processing_warnings
    );
    let failures: Vec<_> = automatic.ocr_page_failures.iter().flatten().collect();
    assert_eq!(failures.len(), 1, "{:?}", automatic.ocr_page_failures);
    assert_eq!(failures[0].page, 2, "{failures:?}");
    assert!(!failures[0].recovered, "the scan has no native text: {failures:?}");
}
