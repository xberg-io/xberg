#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

//! The fallback conditions of the per-page retry that keeps a marked page's native text beside a
//! scan (the `RunFallback` arm of the OCR gate, #2812's follow-up #2833). The registered backend
//! tells pages apart by the dimensions of the raster it is handed, so a test can let the
//! whole-document read cover a page and fail or blank only the per-page retry that reads the same
//! page again. The backend replaces the built-in `tesseract` entry of this process's registry, so
//! every test here is serial and the file holds no other tests.
//!
//! The pinned fallback conditions of `loses_whole_read` (crates/xberg/src/extractors/pdf/mod.rs)
//! are `!failure.recovered` and the non-empty whole-read page clause: cases A, B, C and D decide
//! them through the public API. The `!dropped_pages.contains(&failure.page)` clause is not pinned
//! here: a page in `dropped_pages` always has native text, which recovers its failure first, so it
//! never decides alone through the public API — it is unit-tested as a redundant conjunct of the
//! extracted pure function.
//!
//! CI collects these tests on the unit-test legs (`mise run rust:test:ci`, which runs
//! `scripts/ci/rust/run-unit-tests.sh`; its `xberg_test_features` include `bundle-tessdata-eng`
//! on every platform, with the `full-no-heic` substitution on Linux aarch64).

mod helpers;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use xberg::core::config::{OcrStrategy, PageConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::{ExtractedDocument, ExtractionMethod};
use xberg::{ExtractionConfig, OcrConfig, OutputFormat};

const THEN_SCANNED: &[u8] = include_bytes!("fixtures/pdf/khmer-cluster-2536/khmer-then-scanned-english.pdf");
const ENGLISH: &str = "The quarterly report contains useful readable text about the country and its people.";
const WEAK: &str = "So be it.";
const KHMER_READ: &str = "ខ្ញុំស្រលាញ់ប្រទេសកម្ពុជា សួស្តីភ្នំពេញ";
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
    /// A read in the page's own script, which the whole read keeps: nothing drops, so no retry
    /// is armed at all.
    Khmer,
    /// A read that stays Latin until the per-page retry is executing — marked by the arming
    /// key's raster being read a second time, its whole read and then the retry's read of it —
    /// and fails from then on: however many re-reads the whole-document read makes of the same
    /// raster, none of them is the failing one.
    FailsAfterRetryArms,
}

/// Reads pages apart by the dimensions of the raster it is handed: a raster script's calls
/// consume its outcomes in call order, and a script that runs out repeats its last outcome, so a
/// test can let the whole-document read cover a page and fail or blank only the per-page retry
/// that reads the same page again. `FailsAfterRetryArms` is the exception: it reads Latin until
/// the arming key's raster has been read a second time — the page's whole read, then the
/// per-page retry's read of it — and fails from then on, so a whole read that re-reads the
/// raster any number of times never takes the failure. The scan flag of the handed config
/// (`known_full_page_scan`) separates a page's own reads from the reads of its embedded image,
/// whose dimensions the page render repeats. A raster with no script reads Latin.
struct ByRaster {
    scripts: Mutex<HashMap<ScriptKey, Vec<Read>>>,
    next: Mutex<HashMap<ScriptKey, usize>>,
    /// The key whose second read marks the per-page retry as executing. `None` keeps every
    /// script call-ordered.
    arming_key: Option<ScriptKey>,
    /// Every call per key, counted only while an arming key is set.
    calls: Mutex<HashMap<ScriptKey, usize>>,
    /// Whether the per-page retry has been observed executing.
    retry_armed: AtomicBool,
}

/// A raster script's key: the raster's dimensions and whether the handed config marks the read as
/// a known full-page scan (the page's own read, not an embedded-image read).
type ScriptKey = ((u32, u32), bool);

impl ByRaster {
    fn new(scripts: HashMap<ScriptKey, Vec<Read>>) -> Self {
        Self::new_arming_after(scripts, None)
    }

    fn new_arming_after(scripts: HashMap<ScriptKey, Vec<Read>>, arming_key: Option<ScriptKey>) -> Self {
        Self {
            scripts: Mutex::new(scripts),
            next: Mutex::new(HashMap::new()),
            arming_key,
            calls: Mutex::new(HashMap::new()),
            retry_armed: AtomicBool::new(false),
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
        let dims = image::load_from_memory(image).map_or((0, 0), |decoded| (decoded.width(), decoded.height()));
        let scan_flag = config
            .backend_options
            .as_ref()
            .and_then(|options| options.get("known_full_page_scan"))
            .is_some_and(|value| value.as_bool().unwrap_or(false));
        let key = (dims, scan_flag);
        if self.arming_key.is_some() {
            let mut calls = self.calls.lock().unwrap();
            let count = calls.entry(key).or_default();
            *count += 1;
            if *count == 2 && Some(key) == self.arming_key {
                self.retry_armed.store(true, Ordering::SeqCst);
            }
        }
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
            Read::Khmer => document.content = KHMER_READ.to_owned(),
            Read::FailsAfterRetryArms => {
                if self.retry_armed.load(Ordering::SeqCst) {
                    return Err(xberg::XbergError::ocr("the page raster was refused"));
                }
                document.content = ENGLISH.to_owned();
            }
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

/// Case C: a per-page read that fails unrecovered on a page the whole-document read read with text
/// discards the retry for the whole read: the fallback warning appears, and the marked page's
/// native text is replaced by the whole read's OCR text. This pins the fallback firing itself and
/// the whole-read page-to-index mapping, which cases A and B cannot see: both keep the retry. The
/// failing read is keyed on the phase, not on call order: the whole read and the retry share the
/// scan's script key, and the whole read can re-read the scan through its embedded-image retry
/// (case B shows that re-read), so the scan reads Latin until the marked page's raster has been
/// read a second time — the whole read's read of it, then the per-page retry's read of it — and
/// fails only from then on.
#[test]
#[serial_test::serial]
fn a_retry_failure_unrecovered_on_a_page_the_whole_read_read_drops_the_retry_for_the_whole_read() {
    let backend = Arc::new(ByRaster::new_arming_after(
        script(&[
            // The scan reads Latin for as long as the whole read is running — however many
            // calls it makes, including its embedded-image re-reads — and fails the per-page
            // retry's read of it. The scan has no native text to recover that failure, so the
            // retry loses to the whole read.
            ((SCAN_DIMS, true), &[Read::FailsAfterRetryArms]),
        ]),
        // The marked page: its whole read is unscripted, so it reads Latin, which drops its
        // script and arms the retry; the retry's read of it is the key's second call.
        Some((KHMER_DIMS, false)),
    ));
    let automatic = read(THEN_SCANNED, backend.clone());
    assert_eq!(
        warnings_containing(&automatic, "could not read every other page").len(),
        1,
        "{:?}",
        automatic.processing_warnings
    );
    assert_eq!(
        warnings_containing(&automatic, "OCR of page 2 failed").len(),
        0,
        "the discarded retry's per-page warnings are dropped with it: {:?}",
        automatic.processing_warnings
    );
    assert_eq!(
        khmer_count(&automatic.content),
        0,
        "the marked page's native text was replaced by the whole read: {:?}",
        automatic.content
    );
    assert!(
        automatic.content.contains(ENGLISH),
        "the whole read's text is the result: {:?}",
        automatic.content
    );
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Ocr));
}

/// Case D: a per-page read that fails on the marked page itself — the page the retry exists to
/// keep — must not drop the retry: the marked page keeps its native text and no warning claims a
/// page could not be read. The failure's record is recovered by the page's native text, so the
/// `!failure.recovered` conjunct of `loses_whole_read` decides before the dropped-pages clause
/// can: a page in `dropped_pages` always has native text, so through the public API the clause
/// never decides alone. This case pins the two conjuncts' combined guarantee — a marked page's
/// failed read never discards the retry.
#[test]
#[serial_test::serial]
fn a_retry_read_failure_on_the_marked_page_itself_keeps_the_retry_and_the_native_text() {
    let backend = Arc::new(ByRaster::new(script(&[
        // The scan reads in both runs. The marked page: the whole read reads it (Latin, which
        // drops its script and arms the retry), and the retry's read of it fails unrecovered by
        // any OCR output — only the page's native text recovers it.
        ((SCAN_DIMS, true), &[Read::Latin, Read::Latin]),
        ((KHMER_DIMS, false), &[Read::Latin, Read::Fails]),
    ])));
    let automatic = read(THEN_SCANNED, backend.clone());
    assert_eq!(
        khmer_count(&automatic.content),
        KHMER_PAGE_LETTERS,
        "the marked page must keep its native text: {:?}",
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
        .filter(|failure| failure.page == 1)
        .collect();
    assert_eq!(failures.len(), 1, "{:?}", automatic.ocr_page_failures);
    assert!(
        failures[0].recovered,
        "the marked page's native text recovers its failed read: {:?}",
        failures[0]
    );
}

/// Case E: a whole read that keeps the marked page's script arms no retry — nothing dropped — and
/// its own result stands (the `None` arm of the retry, the no-partial-drop path). The sibling
/// `Some(Err)` arm of the retry — the run failing as a whole — is unreachable with an observable
/// effect through the public API: a page task's backend error, however classified, is converted to
/// a per-page failure, and a run-level cancellation error is re-raised by the cancellation check
/// inside that arm before its fallback applies. The arm itself is a fallback, not bare error
/// propagation: it warns that the targeted OCR failed, records that warning, and hands the
/// document back to the whole-document read.
#[test]
#[serial_test::serial]
fn a_whole_read_that_keeps_the_marked_script_arms_no_retry_and_stands_on_its_own() {
    let backend = Arc::new(ByRaster::new(script(&[
        // The marked page's whole read comes back in its own script, so no page drops its script
        // and the retry is never armed. The scan reads Latin.
        ((KHMER_DIMS, false), &[Read::Khmer]),
    ])));
    let automatic = read(THEN_SCANNED, backend.clone());
    assert!(
        automatic.content.contains(KHMER_READ),
        "the whole read kept the marked page's script: {:?}",
        automatic.content
    );
    assert!(
        automatic.content.contains(ENGLISH),
        "the scan's read is in the result: {:?}",
        automatic.content
    );
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Ocr));
    assert!(
        warnings_containing(&automatic, "could not read every other page").is_empty()
            && warnings_containing(&automatic, "the targeted OCR that keeps their native text failed").is_empty(),
        "no retry ran, so no retry warning appears: {:?}",
        automatic.processing_warnings
    );
    assert!(
        automatic.ocr_page_failures.iter().flatten().count() == 0,
        "every read succeeded: {:?}",
        automatic.ocr_page_failures
    );
}
