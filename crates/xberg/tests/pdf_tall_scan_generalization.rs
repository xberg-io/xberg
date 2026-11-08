#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng"
))]

use sha2::{Digest, Sha256};
use std::io::Write;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig};

const NEUTRAL_RGB: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/neutral-rgb-tall.pdf");
const ORACLE: &str = include_str!("../../../fixtures/repros/tall-scan-strips/expected.txt");

fn compressed_paint(extra_paint: bool) -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(NEUTRAL_RGB).unwrap();
    let mut changed = 0;
    for object in doc.objects.values_mut() {
        let Ok(stream) = object.as_stream_mut() else {
            continue;
        };
        if stream.dict.has(b"Subtype") {
            continue;
        }
        assert!(!stream.dict.has(b"Filter"));
        let original = stream.content.clone();
        if extra_paint {
            stream.content.extend_from_slice(b"\n0 0 m 1 1 l S\n");
        }
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&stream.content).unwrap();
        stream.set_content(encoder.finish().unwrap());
        stream.dict.set("Filter", "FlateDecode");
        assert!(stream.dict.has(b"Filter"));
        if !extra_paint {
            assert_eq!(stream.decompressed_content().unwrap(), original);
        }
        changed += 1;
    }
    assert_eq!(changed, 1, "change only the certified page-paint stream");
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

fn config() -> (tempfile::TempDir, ExtractionConfig) {
    let models = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), english).unwrap();
    let config = serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "ocr": {"backend": "tesseract", "language": ["eng"], "tessdata_path": models.path()},
        "pages": {"extract_pages": true},
        "concurrency": {"max_threads": 1}
    }))
    .unwrap();
    (models, config)
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
    assert_eq!(result.results.len(), 1);
    result.results.pop().unwrap()
}

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .map(|word| word.trim_matches(|character: char| !character.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect()
}

fn used_strips(document: &ExtractedDocument) -> bool {
    document
        .processing_warnings
        .iter()
        .any(|warning| warning.message.contains("source-image strips."))
}

#[test]
fn neutral_rgb_scan_recovers_the_same_words_after_lossless_page_paint_compression() {
    let (_models, config) = config();
    let expected = words(ORACLE);
    assert_eq!(expected.len(), 440);
    for input in [NEUTRAL_RGB.to_vec(), compressed_paint(false)] {
        let result = extract(&input, &config);
        assert!(result.metadata.ocr_used);
        assert!(used_strips(&result), "{:?}", result.processing_warnings);
        assert_eq!(words(&result.content), expected);
        let pages = result.pages.as_ref().unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(words(&pages[0].content), expected);
    }
}

#[test]
fn compressed_paint_with_an_additional_drawing_is_not_certified_as_one_scan() {
    let (_models, config) = config();
    let result = extract(&compressed_paint(true), &config);
    assert!(result.metadata.ocr_used);
    assert!(!used_strips(&result));
}

fn assert_large_text_recovery(input: &[u8]) {
    let (_models, config) = config();
    let oracle = include_str!("../../../fixtures/repros/tall-scan-strips/large-text-expected.txt");
    let expected = words(oracle);
    assert_eq!(oracle.lines().count(), 41);
    assert_eq!(expected.len(), 242);
    let result = extract(input, &config);
    assert!(used_strips(&result), "{:?}", result.processing_warnings);
    assert_eq!(words(&result.content), expected);
    let pages = result.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(words(&pages[0].content), expected);
}

#[test]
fn tall_scan_shrunk_only_by_ocr_preprocessing_recovers_every_source_word() {
    assert_large_text_recovery(include_bytes!(
        "../../../fixtures/repros/tall-scan-strips/large-text-150.pdf"
    ));
}

#[test]
fn same_physical_large_text_at_300_dpi_recovers_every_source_word() {
    assert_large_text_recovery(include_bytes!(
        "../../../fixtures/repros/tall-scan-strips/large-text-300.pdf"
    ));
}
