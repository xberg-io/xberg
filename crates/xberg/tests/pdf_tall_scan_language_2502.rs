#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng",
    not(target_arch = "wasm32")
))]

//! A tall scan in a non-English language, read from the bundled English model directory plus
//! an installed German model. The qualification suite runs offline with English only, so the
//! test is ignored there and run by hand where `deu.traineddata` is installed.

use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig};

const TALL_DEU: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/strict-tall-deu.pdf");
const EXPECTED: &str = include_str!("../../../fixtures/repros/tall-scan-strips/expected-deu.txt");

fn german_model_dir() -> Option<tempfile::TempDir> {
    let candidates = std::env::var_os("TESSDATA_PREFIX")
        .map(std::path::PathBuf::from)
        .into_iter()
        .chain(
            [
                "/opt/homebrew/share/tessdata",
                "/usr/share/tesseract-ocr/5/tessdata",
                "/usr/share/tessdata",
            ]
            .map(Into::into),
        );
    let german = candidates
        .map(|dir| dir.join("deu.traineddata"))
        .find(|path| path.is_file())?;
    let models = tempfile::tempdir().unwrap();
    std::fs::copy(german, models.path().join("deu.traineddata")).unwrap();
    Some(models)
}

fn config(models: &std::path::Path) -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "ocr": {
            "backend": "tesseract",
            "language": ["deu"],
            "tessdata_path": models.to_string_lossy(),
        },
        "pages": {"extract_pages": true},
        "concurrency": {"max_threads": 1},
    }))
    .unwrap()
}

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .map(|word| word.trim_matches(|character: char| !character.is_alphabetic()))
        .filter(|word| !word.is_empty())
        .collect()
}

fn extract(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut result = runtime
        .block_on(xberg::extract(
            ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
            config,
        ))
        .unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    result.results.pop().unwrap()
}

#[test]
#[ignore = "needs deu.traineddata in TESSDATA_PREFIX or a system tessdata directory"]
#[allow(clippy::print_stderr)]
fn a_tall_german_scan_recovers_every_source_word_from_its_strips() {
    let Some(models) = german_model_dir() else {
        eprintln!("skipped: deu.traineddata is not installed");
        return;
    };
    let expected = words(EXPECTED);
    assert_eq!(expected.len(), 400);
    let config = config(models.path());
    let document = extract(TALL_DEU, &config);
    assert!(document.metadata.ocr_used);
    assert!(
        document
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("source-image strips.")),
        "{:?}",
        document.processing_warnings
    );
    assert_eq!(words(&document.content), expected);
    let pages = document.pages.as_ref().unwrap();
    assert_eq!(words(&pages[0].content), expected);

    let mut blocked = config;
    blocked.include_document_structure = true;
    let weak = extract(TALL_DEU, &blocked);
    assert!(
        !weak
            .processing_warnings
            .iter()
            .any(|warning| warning.message.contains("source-image strips."))
    );
    assert!(
        words(&weak.content).len() < expected.len() / 2,
        "the blocked read kept {} of {} words",
        words(&weak.content).len(),
        expected.len()
    );
}
