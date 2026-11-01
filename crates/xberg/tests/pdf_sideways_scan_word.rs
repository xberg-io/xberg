//! A word only the first OCR read of a sideways scan holds stays in the page (#2314).
#![cfg(all(feature = "pdf", feature = "ocr"))]

mod helpers;

use std::collections::HashSet;

use helpers::extract_bytes_document_blocking;
use xberg::ExtractionConfig;

/// One image-only page: sideways prose, with short upright words stamped beside the lines.
/// The sparse first read holds the stamps. The automatic layout read holds the prose and passes over the stamps.
const STAMPED_SCAN: &[u8] = include_bytes!("../../../fixtures/repros/sideways-scan/fixtures/stamped-sideways-scan.pdf");

fn config() -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({
        "use_cache": false, "enable_quality_processing": true, "concurrency": {"max_threads": 1},
        "ocr": {
            "enabled": true, "backend": "tesseract", "language": ["eng"], "auto_rotate": false,
            "vlm_fallback": {"mode": "disabled"}, "numeric_repair": false,
            "pipeline": {"stages": [{"backend": "tesseract", "priority": 100}]}
        }
    }))
    .unwrap()
}

#[test]
fn a_dictionary_word_only_the_first_read_holds_joins_the_kept_second_read() {
    let result = extract_bytes_document_blocking(STAMPED_SCAN, "application/pdf", &config()).unwrap();
    let words: HashSet<_> = result.content.split(|c: char| !c.is_alphanumeric()).collect();

    for word in ["copper", "thunder", "orchard"] {
        assert!(
            words.contains(word),
            "the kept second read must hold {word:?}: {:?}",
            result.content
        );
    }
    for word in ["box", "cat", "hat", "jam", "red", "sun"] {
        assert!(
            words.contains(word),
            "the first read must give {word:?}: {:?}",
            result.content
        );
    }
}
