#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OcrConfig, PageConfig, TesseractConfig};

const PDF_SHA256: &str = "a80d659ab1fbf8cbefcd182799d5b2785da94d9ab3fb3d9a024ab4c4c6dc8eb4";
const DICTIONARY_SHA256: &str = "9f513f1ceadb6a01c5485b7dbdfd5118dc66cd70b59cae2851292112d4066a32";
const MODEL_SHA256: &str = "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2";

fn recipe_root() -> PathBuf {
    std::env::var_os("XBERG_2439_RECIPE_ROOT")
        .map(PathBuf::from)
        .expect("XBERG_2439_RECIPE_ROOT must contain the hash-pinned public recipe and truth")
}

fn read_pinned(path: &Path, expected: &str) -> Vec<u8> {
    let bytes = std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    assert_eq!(hex::encode(Sha256::digest(&bytes)), expected, "{}", path.display());
    bytes
}

fn words(text: &str) -> Vec<String> {
    text.split(|character: char| !character.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn expected_pages(root: &Path) -> Vec<Vec<String>> {
    let truth: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("generated/truth.json")).expect("independent generator truth"))
            .unwrap();
    let pages: Vec<Vec<String>> = truth["scan_first_half"]["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|page| {
            page["words"]
                .as_array()
                .unwrap()
                .iter()
                .map(|word| word.as_str().unwrap().to_ascii_lowercase())
                .collect()
        })
        .collect();
    assert_eq!(pages.len(), 8);
    assert!(pages.iter().all(|page| page.len() == 115));
    assert_eq!(
        pages.iter().flatten().collect::<BTreeSet<_>>().len(),
        920,
        "source words never cross pages"
    );
    pages
}

async fn run(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let mut result = xberg::extract(
        ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
        config,
    )
    .await
    .expect("real public API extraction");
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(result.results.len(), 1);
    result.results.remove(0)
}

fn page_metrics(document: &ExtractedDocument, expected: &[Vec<String>]) -> Vec<serde_json::Value> {
    let pages = document.pages.as_ref().expect("requested all per-page outputs");
    assert_eq!(pages.len(), 8);
    pages
        .iter()
        .enumerate()
        .map(|(index, page)| {
            assert_eq!(page.page_number as usize, index + 1);
            let actual = words(&page.content);
            let mut counts = BTreeMap::new();
            for word in &actual {
                *counts.entry(word.as_str()).or_insert(0usize) += 1;
            }
            let wanted: BTreeSet<_> = expected[index].iter().map(String::as_str).collect();
            let missing: Vec<_> = expected[index]
                .iter()
                .filter(|word| !counts.contains_key(word.as_str()))
                .collect();
            let duplicates: Vec<_> = counts
                .iter()
                .filter(|(word, count)| wanted.contains(**word) && **count > 1)
                .map(|(word, count)| (*word, *count))
                .collect();
            let extra: Vec<_> = actual.iter().filter(|word| !wanted.contains(word.as_str())).collect();
            serde_json::json!({"page":index + 1, "expected":115, "actual_tokens":actual.len(),
            "source_present":115 - missing.len(), "missing":missing, "duplicates":duplicates,
            "extra":extra, "exact_source_order":actual == expected[index],
            "ocr_confidence":page.ocr_confidence})
        })
        .collect()
}

struct RecipeObservation {
    native: ExtractedDocument,
    automatic: ExtractedDocument,
}

async fn observe_recipe() -> RecipeObservation {
    let root = recipe_root();
    read_pinned(&root.join("american-english"), DICTIONARY_SHA256);
    let bytes = read_pinned(&root.join("generated/scan_first_half.pdf"), PDF_SHA256);
    let expected = expected_pages(&root);
    let model = xberg_tesseract::bundled_eng_traineddata().expect("mandatory bundled English model");
    assert_eq!(hex::encode(Sha256::digest(model)), MODEL_SHA256);
    let tessdata = tempfile::tempdir().unwrap();
    std::fs::write(tessdata.path().join("eng.traineddata"), model).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            enabled: true,
            backend: "tesseract".into(),
            language: vec!["eng".into()],
            tessdata_path: Some(tessdata.path().to_path_buf()),
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let start = Instant::now();
    let native = run(
        &bytes,
        &ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        },
    )
    .await;
    let native_ms = start.elapsed().as_millis();
    let start = Instant::now();
    let automatic = run(&bytes, &config).await;
    let automatic_ms = start.elapsed().as_millis();
    let native_metrics = page_metrics(&native, &expected);
    let automatic_metrics = page_metrics(&automatic, &expected);
    for (name, document) in [("native", &native), ("automatic", &automatic)] {
        std::fs::write(
            root.join(format!("candidate-{name}.json")),
            serde_json::to_vec_pretty(document).unwrap(),
        )
        .unwrap();
    }
    let metrics = serde_json::json!({"jobs_expected":2,"jobs_executed":2,"native_ms":native_ms,
        "automatic_ms":automatic_ms,"native":native_metrics,"automatic":automatic_metrics,
        "model_sha256":MODEL_SHA256,"tesseract_version":xberg_tesseract::TesseractAPI::version(),
        "pdf_sha256":PDF_SHA256,"dictionary_sha256":DICTIONARY_SHA256,"config":config});
    std::fs::write(
        root.join("candidate-metrics.json"),
        serde_json::to_vec_pretty(&metrics).unwrap(),
    )
    .unwrap();
    assert!(automatic.metadata.ocr_used, "the scanned pages must execute real OCR");
    let native_pages = native.pages.as_ref().unwrap();
    assert!(native_pages[..4].iter().all(|page| words(&page.content).is_empty()));
    for index in 4..8 {
        assert_eq!(
            words(&native_pages[index].content),
            expected[index],
            "native page {}",
            index + 1
        );
        assert_eq!(
            automatic_metrics[index]["missing"],
            serde_json::json!([]),
            "automatic page {}",
            index + 1
        );
        assert_eq!(
            automatic_metrics[index]["duplicates"],
            serde_json::json!([]),
            "automatic page {}",
            index + 1
        );
    }
    RecipeObservation { native, automatic }
}

#[tokio::test]
#[ignore = "requires hash-pinned public recipe; run explicitly with XBERG_2439_RECIPE_ROOT"]
async fn should_retain_all_native_source_words_in_the_public_scan_first_half_recipe() {
    let _ = observe_recipe().await;
}

#[tokio::test]
#[ignore = "requires hash-pinned public recipe and real OCR"]
async fn should_preserve_exact_native_page8_content_and_order_in_the_public_recipe() {
    let observation = observe_recipe().await;
    let native = &observation.native.pages.as_ref().unwrap()[7];
    let automatic = &observation.automatic.pages.as_ref().unwrap()[7];
    assert_eq!(
        automatic.content, native.content,
        "page8 must retain complete native content and order"
    );
}
