use xberg::{ExtractionConfig, types::ExtractionMethod};

const JUNK_SCAN: &[u8] = include_bytes!("../fixtures/ocr/hidden_layer_ccitt_2694/four-page-junk-layer.pdf");
const FORCED_SDM_WORDS: usize = 582;
const KHMER: &[u8] = include_bytes!("../fixtures/pdf/khmer-cluster-2536/genuinely-partial-mapped.pdf");

async fn read(bytes: &[u8], config: &ExtractionConfig) -> xberg::ExtractedDocument {
    super::helpers::extract_bytes_document(bytes, "application/pdf", config)
        .await
        .unwrap()
}

fn native_config(config: &ExtractionConfig) -> ExtractionConfig {
    let mut native = config.clone();
    native.disable_ocr = true;
    native
}

fn forced_config(config: &ExtractionConfig) -> ExtractionConfig {
    let mut forced = config.clone();
    forced.force_ocr = true;
    forced
}

fn fragmented_junk_layer() -> Vec<u8> {
    let mut document = lopdf::Document::load_mem(JUNK_SCAN).unwrap();
    let mut replacements = 0;
    for id in document.get_pages().into_values() {
        let bytes = document.get_page_content(id);
        assert!(!bytes.is_empty(), "every source page has a content stream");
        let mut content = lopdf::content::Content::decode(&bytes).unwrap();
        for operation in &mut content.operations {
            if operation.operator == "Tj" {
                operation.operands = vec![lopdf::Object::string_literal("x x x x x x x x")];
                replacements += 1;
            }
        }
        let stream = document.add_object(lopdf::Stream::new(lopdf::Dictionary::new(), content.encode().unwrap()));
        document.get_dictionary_mut(id).unwrap().set("Contents", stream);
    }
    assert_eq!(replacements, 12, "three hidden rows on each of four pages");
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

#[tokio::test]
async fn automatic_all_pages_replaces_a_disjoint_invisible_junk_layer() {
    let models = tempfile::tempdir().unwrap();
    let config = super::pinned_config(models.path());
    let bytes = fragmented_junk_layer();
    let native = read(&bytes, &native_config(&config)).await;
    assert_eq!(native.content.split_whitespace().collect::<Vec<_>>(), vec!["x"; 96]);
    let forced = read(&bytes, &forced_config(&config)).await;
    for word in ["lighthouse", "Volunteers", "Fishing", "evening"] {
        assert!(
            forced.content.contains(word),
            "the forced control reads {word}: {:?}",
            forced.processing_warnings
        );
    }
    let automatic = read(&bytes, &config).await;
    assert_eq!(automatic.pages.as_ref().unwrap().len(), 4);
    assert!(
        automatic.metadata.ocr_used,
        "automatic method {:?}, words {}, warnings {:?}",
        automatic.extraction_method,
        automatic.content.split_whitespace().count(),
        automatic.processing_warnings
    );
    assert!(!automatic.content.split_whitespace().any(|word| word == "x"));
    for word in ["lighthouse", "Volunteers", "Fishing", "evening"] {
        assert!(automatic.content.contains(word), "{word}");
    }
    assert_eq!(
        automatic.content.split_whitespace().collect::<Vec<_>>(),
        forced.content.split_whitespace().collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn automatic_all_pages_keeps_native_script_the_english_model_cannot_read() {
    let models = tempfile::tempdir().unwrap();
    let config = super::pinned_config(models.path());
    let native = read(KHMER, &native_config(&config)).await;
    let count = native
        .content
        .chars()
        .filter(|c| ('\u{1780}'..='\u{17ff}').contains(c))
        .count();
    assert_eq!(count, 28, "the partial native map preserves 28 Khmer characters");
    let automatic = read(KHMER, &config).await;
    assert_eq!(automatic.content, native.content);
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
}

#[tokio::test]
async fn forced_ocr_at_the_all_pages_seam_returns_the_full_recognized_text() {
    if super::helpers::skip_if_missing(super::SOURCE) {
        return;
    }
    let models = tempfile::tempdir().unwrap();
    let config = super::pinned_config(models.path());
    let bytes = super::excerpt(&[super::SOURCE_PAGE]);
    let automatic = read(&bytes, &config).await;
    let forced = read(&bytes, &forced_config(&config)).await;
    assert_eq!(automatic.content.split_whitespace().count(), super::SOURCE_WORDS);
    assert_eq!(forced.extraction_method, Some(ExtractionMethod::Ocr));
    assert_eq!(
        forced.content.split_whitespace().count(),
        FORCED_SDM_WORDS,
        "the pinned full-page OCR read"
    );
    assert!(
        forced
            .content
            .contains("Table 3-12. Encoding of CPUID Leaf 2 Descriptors")
    );
    assert!(forced.content.trim_end().ends_with("Vol. 2A 3-241"));
    assert_ne!(forced.content, automatic.content);
}

#[tokio::test]
async fn naca_2425_all_pages_replaces_the_junk_layer_with_the_confident_read() {
    const PAGE: &str = "pdf/regression_pages/nasa-naca-report-1135-p25.pdf";
    if super::helpers::skip_if_missing(PAGE) {
        return;
    }
    let bytes = std::fs::read(super::helpers::get_test_file_path(PAGE)).unwrap();
    use sha2::Digest;
    assert_eq!(
        hex::encode(sha2::Sha256::digest(&bytes)),
        "1696465613177c9059fea2edd8a109c753c649fdfb80cf33712d9d890f0a3ed0"
    );
    let models = tempfile::tempdir().unwrap();
    let config = super::pinned_config(models.path());
    let native = read(&bytes, &native_config(&config)).await;
    assert_eq!(native.content.split_whitespace().count(), 424);
    assert!(native.content.contains("COMMI2\"rEE"));
    let forced = read(&bytes, &forced_config(&config)).await;
    assert!(forced.content.split_whitespace().count() > 2000);
    let automatic = read(&bytes, &config).await;
    assert!(automatic.metadata.ocr_used);
    assert!(!automatic.content.contains("COMMI2\"rEE"));
    assert!(automatic.content.split_whitespace().any(|word| word == "COMMITTEE"));
    assert_eq!(
        automatic.content.split_whitespace().count(),
        forced.content.split_whitespace().count()
    );
    assert!(
        automatic.pages.as_ref().unwrap()[0]
            .ocr_confidence
            .as_ref()
            .unwrap()
            .score
            .is_some_and(|score| score >= 0.75)
    );
}
