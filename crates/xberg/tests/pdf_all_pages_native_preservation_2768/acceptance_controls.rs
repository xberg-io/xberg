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
async fn unreadable_native_script_control_keeps_the_preexisting_native_route() {
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

fn token_digest(text: &str) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(
        text.split_whitespace().collect::<Vec<_>>().join(" "),
    ))
}

fn token_surplus<'a>(text: &'a str, reference: &str) -> std::collections::BTreeMap<&'a str, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0_usize) += 1;
    }
    for word in reference.split_whitespace() {
        if let Some(count) = counts.get_mut(word) {
            *count = count.saturating_sub(1);
        }
    }
    counts.retain(|_, count| *count > 0);
    counts
}

fn expected_forced_surplus() -> std::collections::BTreeMap<&'static str, usize> {
    std::collections::BTreeMap::from([
        ("|", 32),
        ("OOH", 1),
        ("O5H", 1),
        ("OGH", 1),
        ("OAH", 1),
        ("OBH", 1),
        ("OCH", 1),
        ("ODH", 1),
        ("OEH", 1),
        ("end-level", 2),
        ("OFH,", 1),
        ("O6H);", 1),
        ("4€H", 1),
        ("GMByte,", 1),
    ])
}

fn expected_native_surplus() -> std::collections::BTreeMap<&'static str, usize> {
    std::collections::BTreeMap::from([
        ("00H", 1),
        ("05H", 1),
        ("06H", 1),
        ("0AH", 1),
        ("0BH", 1),
        ("0CH", 1),
        ("0DH", 1),
        ("0EH", 1),
        ("2nd-level", 2),
        ("0FH,", 1),
        ("06H);", 1),
        ("4EH", 1),
        ("6MByte,", 1),
    ])
}

fn assert_sdm_word_difference(native: &str, forced: &str) {
    assert_eq!(token_surplus(forced, native), expected_forced_surplus());
    assert_eq!(token_surplus(native, forced), expected_native_surplus());
    assert_eq!(
        token_digest(native),
        "491415e6e0ae19ade9640896167add05c42771bf160a9d83743a6cc1fd2ae8e9"
    );
    assert_eq!(
        token_digest(forced),
        "e7b627f8c2767b9db1a4e32d6838ac40f1db9e9cab33960e5b9d5b894df81fb3"
    );
}

#[tokio::test]
async fn forced_ocr_at_the_all_pages_seam_returns_the_full_recognized_text() {
    let models = tempfile::tempdir().unwrap();
    let config = super::pinned_config(models.path());
    let bytes = super::excerpt(&[super::SOURCE_PAGE]);
    let automatic = read(&bytes, &config).await;
    let forced = read(&bytes, &forced_config(&config)).await;
    let native = read(&bytes, &native_config(&config)).await;
    assert!(
        automatic
            .content
            .split_whitespace()
            .eq(native.content.split_whitespace())
    );
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
    assert_sdm_word_difference(&native.content, &forced.content);
}

#[tokio::test]
async fn explicit_tesseract_options_keep_the_original_native_table_once() {
    let models = tempfile::tempdir().unwrap();
    let mut config = super::pinned_config(models.path());
    config.ocr.as_mut().unwrap().tesseract_config = Some(xberg::TesseractConfig {
        use_cache: false,
        ..Default::default()
    });
    let bytes = super::excerpt(&[super::SOURCE_PAGE]);
    let native = read(&bytes, &native_config(&config)).await;
    let automatic = read(&bytes, &config).await;
    let count = automatic.content.split_whitespace().count();
    assert_eq!(count, super::SOURCE_WORDS);
    assert!(
        automatic
            .content
            .split_whitespace()
            .eq(native.content.split_whitespace())
    );
    assert_eq!(
        token_digest(&automatic.content),
        "491415e6e0ae19ade9640896167add05c42771bf160a9d83743a6cc1fd2ae8e9"
    );
}

#[tokio::test]
async fn naca_2425_all_pages_replaces_the_junk_layer_with_the_confident_read() {
    const PAGE: &str = "pdf/regression_pages/nasa-naca-report-1135-p25.pdf";
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
