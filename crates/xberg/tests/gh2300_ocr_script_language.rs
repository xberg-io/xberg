//! Regression coverage for GH#2300: a page that goes to OCR reads with the configured languages
//! and also with the language of the script of its document's text layer.
//!
//! A stub registered as `tesseract` records the languages of each OCR call. The fixture has two
//! pages. Page 1 holds a text layer, set through a font whose map to Unicode gives Khmer or Latin
//! letters. Page 2 is a full-page picture with no text, which sends the page to OCR on its own.
//! The stub returns no text, so only the languages of the calls are under test.

#![allow(deprecated)]
#![cfg(all(feature = "pdf", feature = "ocr"))]

mod helpers;

use async_trait::async_trait;
use helpers::extract_bytes_document_blocking;
use std::sync::{Arc, Mutex};
use xberg::ExtractedDocument;
use xberg::core::config::{ExtractionConfig, OcrConfig};
use xberg::plugins::{OcrBackend, OcrBackendType, Plugin, register_ocr_backend, unregister_ocr_backend};
use xberg::types::TesseractConfig;

const PAGE_PT: u32 = 200;
const RASTER_PX: u32 = 64;
/// First code point of the Khmer consonants. Codes 1 to `LETTER_COUNT` map to consecutive ones.
const KHMER_FIRST: u32 = 0x1780;
/// First code point of the Latin small letters.
const LATIN_FIRST: u32 = 0x61;
const LETTER_COUNT: u32 = 26;
/// Times the letter run repeats in the text layer. The run is long enough to count as evidence.
const REPEATS: usize = 6;

/// The languages one OCR call carried, in both language fields.
#[derive(Clone, Debug)]
struct RecordedLanguages {
    ocr: Vec<String>,
    tesseract: Option<Vec<String>>,
}

struct RecordingBackend {
    calls: Arc<Mutex<Vec<RecordedLanguages>>>,
}

impl Plugin for RecordingBackend {
    fn name(&self) -> &str {
        "tesseract"
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
impl OcrBackend for RecordingBackend {
    async fn process_image(&self, _image_bytes: &[u8], config: &OcrConfig) -> xberg::Result<ExtractedDocument> {
        self.calls.lock().expect("calls lock").push(RecordedLanguages {
            ocr: config.language.clone(),
            tesseract: config.tesseract_config.as_ref().map(|t| t.language.clone()),
        });
        Ok(ExtractedDocument::default())
    }
    fn supports_language(&self, _language: &str) -> bool {
        true
    }
    fn backend_type(&self) -> OcrBackendType {
        OcrBackendType::Custom
    }
}

/// A two-page PDF. Page 1 shows `first_code_point + 0..LETTER_COUNT` through a font whose
/// `ToUnicode` map sends code `n` to that code point. Page 2 is one grey raster over the page.
fn two_page_pdf(first_code_point: u32) -> Vec<u8> {
    let codes: String = (1..=LETTER_COUNT).map(|code| format!("{code:02X}")).collect();
    let shown = codes.repeat(REPEATS);
    let text = format!("BT /F1 10 Tf 10 150 Td <{shown}> Tj ET\n");
    let picture = format!("q {PAGE_PT} 0 0 {PAGE_PT} 0 0 cm /Im0 Do Q\n");
    let mut to_unicode = String::from(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap\n\
         1 begincodespacerange <00> <FF> endcodespacerange\n",
    );
    to_unicode.push_str(&format!("{LETTER_COUNT} beginbfchar\n"));
    for code in 1..=LETTER_COUNT {
        to_unicode.push_str(&format!("<{code:02X}> <{:04X}>\n", first_code_point + code - 1));
    }
    to_unicode.push_str("endbfchar\nendcmap end end\n");
    let raster = vec![0xA0u8; (RASTER_PX * RASTER_PX) as usize];

    let mut buf = Vec::<u8>::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    let mut object = |buf: &mut Vec<u8>, body: &[u8]| {
        offsets.push(buf.len());
        buf.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
        buf.extend_from_slice(body);
        buf.extend_from_slice(b"\nendobj\n");
    };
    object(&mut buf, b"<</Type /Catalog /Pages 2 0 R>>");
    object(&mut buf, b"<</Type /Pages /Kids [3 0 R 4 0 R] /Count 2>>");
    object(
        &mut buf,
        format!(
            "<</Type /Page /MediaBox [0 0 {PAGE_PT} {PAGE_PT}] /Parent 2 0 R /Contents 5 0 R \
             /Resources <</Font <</F1 7 0 R>> >> >>"
        )
        .as_bytes(),
    );
    object(
        &mut buf,
        format!(
            "<</Type /Page /MediaBox [0 0 {PAGE_PT} {PAGE_PT}] /Parent 2 0 R /Contents 6 0 R \
             /Resources <</XObject <</Im0 8 0 R>> >> >>"
        )
        .as_bytes(),
    );
    object(
        &mut buf,
        format!("<</Length {}>>\nstream\n{text}\nendstream", text.len() + 1).as_bytes(),
    );
    object(
        &mut buf,
        format!("<</Length {}>>\nstream\n{picture}\nendstream", picture.len() + 1).as_bytes(),
    );
    object(
        &mut buf,
        b"<</Type /Font /Subtype /Type1 /BaseFont /Helvetica /ToUnicode 9 0 R>>",
    );
    let mut image = format!(
        "<</Type /XObject /Subtype /Image /Width {RASTER_PX} /Height {RASTER_PX} /ColorSpace /DeviceGray \
         /BitsPerComponent 8 /Length {}>>\nstream\n",
        raster.len()
    )
    .into_bytes();
    image.extend_from_slice(&raster);
    image.extend_from_slice(b"\nendstream");
    object(&mut buf, &image);
    object(
        &mut buf,
        format!("<</Length {}>>\nstream\n{to_unicode}\nendstream", to_unicode.len() + 1).as_bytes(),
    );

    let xref_offset = buf.len();
    buf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
    for offset in &offsets {
        buf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    buf.extend_from_slice(format!("trailer\n<</Size {} /Root 1 0 R>>\n", offsets.len() + 1).as_bytes());
    buf.extend_from_slice(format!("startxref\n{xref_offset}\n%%EOF\n").as_bytes());
    buf
}

/// A tessdata directory that holds the languages of these tests as empty files. The stub
/// backend never reads them. The availability check that guards the added language does.
fn fake_tessdata() -> std::path::PathBuf {
    static DIR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("gh2300-fake-tessdata-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create the fake tessdata directory");
        for language in ["eng", "deu", "fra", "khm", "chi_sim"] {
            std::fs::write(dir.join(format!("{language}.traineddata")), b"").expect("write a fake model");
        }
        dir
    })
    .clone()
}

fn config_with(mut ocr: OcrConfig) -> ExtractionConfig {
    ocr.tessdata_path = Some(fake_tessdata());
    ExtractionConfig {
        ocr: Some(ocr),
        ocr_embedded_images: Some(false),
        use_cache: false,
        ..Default::default()
    }
}

/// Extract `pdf` with the stub backend and return the languages of every OCR call.
fn record_calls(pdf: &[u8], config: &ExtractionConfig) -> Vec<RecordedLanguages> {
    assert!(
        std::str::from_utf8(pdf).is_err(),
        "the fixture must carry the binary raster stream"
    );
    let calls = Arc::new(Mutex::new(Vec::new()));
    let _ = unregister_ocr_backend("tesseract");
    register_ocr_backend(Arc::new(RecordingBackend { calls: calls.clone() })).expect("the stub backend registers");
    let result = extract_bytes_document_blocking(pdf, "application/pdf", config);
    unregister_ocr_backend("tesseract").expect("the stub backend unregisters");
    result.expect("extraction of the fixture must not error");
    let calls = calls.lock().expect("calls lock").clone();
    assert!(
        !calls.is_empty(),
        "the picture page must reach the OCR backend; the fixture no longer routes a page to OCR"
    );
    calls
}

fn text_layer_of(pdf: &[u8], config: &ExtractionConfig) -> String {
    let _ = unregister_ocr_backend("tesseract");
    let no_ocr = ExtractionConfig {
        ocr: None,
        ..config.clone()
    };
    extract_bytes_document_blocking(pdf, "application/pdf", &no_ocr)
        .expect("native extraction must not error")
        .content
}

/// The Khmer text layer must be read as Khmer, or the tests below prove nothing.
#[test]
#[serial_test::serial]
fn the_fixture_text_layer_is_khmer() {
    let text = text_layer_of(&two_page_pdf(KHMER_FIRST), &config_with(OcrConfig::default()));
    let khmer = text.chars().filter(|c| ('\u{1780}'..='\u{17FF}').contains(c)).count();
    assert!(
        khmer >= LETTER_COUNT as usize,
        "the fixture's text layer must decode to Khmer, got {text:?}"
    );
}

#[test]
#[serial_test::serial]
fn a_page_sent_to_ocr_also_reads_the_language_of_the_documents_script() {
    let calls = record_calls(&two_page_pdf(KHMER_FIRST), &config_with(OcrConfig::default()));
    for call in &calls {
        assert_eq!(
            call.ocr,
            ["eng", "khm"],
            "the Khmer text layer adds khm to the default eng"
        );
    }
}

#[test]
#[serial_test::serial]
fn a_caller_language_keeps_its_place_when_the_script_language_joins() {
    let ocr = OcrConfig {
        language: vec!["deu".to_string()],
        ..Default::default()
    };
    let calls = record_calls(&two_page_pdf(KHMER_FIRST), &config_with(ocr));
    for call in &calls {
        assert_eq!(call.ocr, ["deu", "khm"]);
    }
}

/// The losing field: the tesseract language wins while the OCR language is the default, and it
/// is the one that gains the script language.
#[test]
#[serial_test::serial]
fn the_script_language_joins_the_field_that_wins() {
    let ocr = OcrConfig {
        tesseract_config: Some(TesseractConfig {
            language: vec!["fra".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    };
    let calls = record_calls(&two_page_pdf(KHMER_FIRST), &config_with(ocr));
    for call in &calls {
        assert_eq!(
            call.tesseract.as_deref(),
            Some(&["fra".to_string(), "khm".to_string()][..])
        );
        assert_eq!(call.ocr, ["eng"], "the losing field is not touched");
    }
}

#[test]
#[serial_test::serial]
fn a_caller_language_that_reads_the_script_is_left_alone() {
    let ocr = OcrConfig {
        language: vec!["khm".to_string(), "eng".to_string()],
        ..Default::default()
    };
    let calls = record_calls(&two_page_pdf(KHMER_FIRST), &config_with(ocr));
    for call in &calls {
        assert_eq!(call.ocr, ["khm", "eng"]);
    }
}

#[test]
#[serial_test::serial]
fn a_latin_text_layer_adds_no_language() {
    let calls = record_calls(&two_page_pdf(LATIN_FIRST), &config_with(OcrConfig::default()));
    for call in &calls {
        assert_eq!(call.ocr, ["eng"]);
    }
}

/// An explicit `force_ocr` request is not re-decided: it reads with the configured languages.
#[test]
#[serial_test::serial]
fn a_forced_ocr_request_keeps_the_configured_languages() {
    let mut config = config_with(OcrConfig::default());
    config.force_ocr = true;
    let calls = record_calls(&two_page_pdf(KHMER_FIRST), &config);
    for call in &calls {
        assert_eq!(call.ocr, ["eng"]);
    }
}

/// Extract the file `path` with the stub backend and return the languages of every OCR call.
fn record_file_calls(path: &std::path::Path, config: &ExtractionConfig) -> Vec<RecordedLanguages> {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let _ = unregister_ocr_backend("tesseract");
    register_ocr_backend(Arc::new(RecordingBackend { calls: calls.clone() })).expect("the stub backend registers");
    let result = helpers::extract_uri_document_blocking(path, Some("application/pdf"), config);
    unregister_ocr_backend("tesseract").expect("the stub backend unregisters");
    result.expect("extraction of the fixture must not error");
    calls.lock().expect("calls lock").clone()
}

/// Sets `TESSDATA_PREFIX` for the life of the guard, and restores the old value on drop.
struct TessdataPrefix(Option<std::ffi::OsString>);

impl TessdataPrefix {
    #[allow(unsafe_code)]
    fn set(path: &std::path::Path) -> Self {
        let old = std::env::var_os("TESSDATA_PREFIX");
        // SAFETY: the test is serial, so no other thread of this binary reads the environment now.
        unsafe { std::env::set_var("TESSDATA_PREFIX", path) };
        Self(old)
    }
}

impl Drop for TessdataPrefix {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: as in `set`.
        unsafe {
            match &self.0 {
                Some(old) => std::env::set_var("TESSDATA_PREFIX", old),
                None => std::env::remove_var("TESSDATA_PREFIX"),
            }
        }
    }
}

/// The result of an extraction depends on the models on disk: a Khmer layer adds `khm` only when
/// a directory of the search chain holds it with the configured languages. A cached result must
/// not be returned after that answer or its directory changes, with the same file and the same
/// configuration.
///
/// The search chain is hermetic. The configured language is a valid code that Tesseract ships no
/// model for, which only the two test directories hold, so no directory outside the test can hold
/// the whole list. The two test
/// directories are the first two of the chain: the configured tessdata path, then
/// `TESSDATA_PREFIX`.
#[test]
#[serial_test::serial]
fn a_cached_result_is_not_returned_after_the_script_model_arrives_or_moves() {
    const LANGUAGE: &str = "abq";
    let install = |dir: &std::path::Path, model: &str| {
        std::fs::write(dir.join(format!("{model}.traineddata")), b"").expect("write a fake model");
    };
    let configured = tempfile::tempdir().expect("configured tessdata directory");
    let prefix = tempfile::tempdir().expect("prefix tessdata directory");
    install(configured.path(), LANGUAGE);
    install(prefix.path(), LANGUAGE);
    let _prefix = TessdataPrefix::set(prefix.path());

    let files = tempfile::tempdir().expect("fixture directory");
    let pdf_path = files.path().join("khmer-layer.pdf");
    std::fs::write(&pdf_path, two_page_pdf(KHMER_FIRST)).expect("write the fixture");
    let ocr = OcrConfig {
        language: vec![LANGUAGE.to_string()],
        tessdata_path: Some(configured.path().to_path_buf()),
        ..Default::default()
    };
    let config = ExtractionConfig {
        ocr: Some(ocr),
        ocr_embedded_images: Some(false),
        use_cache: true,
        cache_namespace: Some(format!("gh2300-cache-transition-{}", std::process::id())),
        ..Default::default()
    };

    let without_model = record_file_calls(&pdf_path, &config);
    assert!(!without_model.is_empty(), "the picture page must reach the OCR backend");
    for call in &without_model {
        assert_eq!(call.ocr, [LANGUAGE], "no directory holds the Khmer model yet");
    }

    install(prefix.path(), "khm");
    let model_arrived = record_file_calls(&pdf_path, &config);
    assert!(
        !model_arrived.is_empty(),
        "the cached result of the run without the model was returned after the model arrived"
    );
    for call in &model_arrived {
        assert_eq!(call.ocr, [LANGUAGE, "khm"]);
    }

    install(configured.path(), "khm");
    let model_moved = record_file_calls(&pdf_path, &config);
    assert!(
        !model_moved.is_empty(),
        "the cached result of the prefix directory was returned after the model moved to the configured one"
    );
    for call in &model_moved {
        assert_eq!(call.ocr, [LANGUAGE, "khm"]);
    }
}
