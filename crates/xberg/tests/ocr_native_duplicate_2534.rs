#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::types::ExtractionMethod;
use xberg::types::OcrElementConfig;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, OcrConfig, PageConfig, TesseractConfig};

const BOOK: &str = "pdf/assembly_language_for_beginners_al4_b_en.pdf";
const LISTING_PAGE: u32 = 591;
const SDM: &str = "pdf/intel_64_and_ia_32_architectures_software_developer_s_manual_combined_volumes_1_4_june_2021_325462_sdm_vol_1_2abcd_3abcd.pdf";
const DESCRIPTOR_TABLE_PAGE: u32 = 815;
const ESCAPED_CELL_TABLE_PAGE: u32 = 4311;
const ENGLISH_MODEL_SHA256: &str = "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2";

fn three_pages_around(book: &str, page: u32) -> Vec<u8> {
    pages_excerpt(book, &[page - 1, page, page + 1])
}

fn pages_excerpt(book: &str, wanted: &[u32]) -> Vec<u8> {
    use lopdf::Object;

    let mut document = lopdf::Document::load(helpers::get_test_file_path(book)).expect("open the corpus PDF");
    let pages = document.get_pages();
    let kept: Vec<Object> = wanted.iter().map(|number| Object::Reference(pages[number])).collect();
    let root = document
        .catalog()
        .unwrap()
        .get(b"Pages")
        .unwrap()
        .as_reference()
        .unwrap();
    for number in pages.keys().filter(|number| !wanted.contains(number)) {
        document.objects.remove(&pages[number]);
    }
    let tree = document.get_object_mut(root).and_then(Object::as_dict_mut).unwrap();
    let count = i64::try_from(kept.len()).unwrap();
    tree.set("Kids", kept);
    tree.set("Count", count);
    for number in wanted {
        document
            .get_object_mut(pages[number])
            .and_then(Object::as_dict_mut)
            .unwrap()
            .set("Parent", root);
    }
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("save the page excerpt");
    bytes
}

fn pinned_english_config(tessdata: &std::path::Path) -> ExtractionConfig {
    let model = xberg_tesseract::bundled_eng_traineddata().expect("bundled English model");
    assert_eq!(hex::encode(Sha256::digest(model)), ENGLISH_MODEL_SHA256);
    std::fs::write(tessdata.join("eng.traineddata"), model).unwrap();
    ExtractionConfig {
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            enabled: true,
            backend: "tesseract".into(),
            language: vec!["eng".into()],
            tessdata_path: Some(tessdata.to_path_buf()),
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn pages_of(bytes: &[u8], config: &ExtractionConfig) -> (Vec<String>, Option<ExtractionMethod>) {
    pages_of_excerpt(bytes, config, 3).await
}

async fn pages_of_excerpt(
    bytes: &[u8],
    config: &ExtractionConfig,
    expected: usize,
) -> (Vec<String>, Option<ExtractionMethod>) {
    let output = xberg::extract(
        ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
        config,
    )
    .await
    .expect("extract the excerpt");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let [document]: [ExtractedDocument; 1] = output.results.try_into().expect("one document");
    let method = document.extraction_method;
    let pages = document.pages.expect("per-page output requested");
    assert_eq!(pages.len(), expected);
    (pages.into_iter().map(|page| page.content).collect(), method)
}

#[tokio::test]
async fn automatic_ocr_returns_a_code_listing_page_once_and_keeps_its_repeated_lines() {
    if helpers::skip_if_missing(BOOK) {
        assert!(
            std::env::var_os("CI").is_none(),
            "the corpus book {BOOK} is required in CI"
        );
        return;
    }
    let tessdata = tempfile::tempdir().unwrap();
    let config = pinned_english_config(tessdata.path());
    let bytes = three_pages_around(BOOK, LISTING_PAGE);
    let (native, native_method) = pages_of(
        &bytes,
        &ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        },
    )
    .await;
    let (automatic, _) = pages_of(&bytes, &config).await;
    assert_eq!(native_method, Some(ExtractionMethod::Native));

    // Requesting OCR elements switches native preservation off, which shows that automatic
    // routing sends this page to OCR and that the read is not a copy of the native text.
    let mut with_elements = config.clone();
    with_elements.ocr.as_mut().unwrap().element_config = Some(OcrElementConfig {
        include_elements: true,
        ..Default::default()
    });
    let (read, _) = pages_of(&bytes, &with_elements).await;
    assert!(
        read[1].split_whitespace().count() > native[1].split_whitespace().count(),
        "automatic OCR ran on the listing page"
    );

    let words = |page: &str| page.split_whitespace().count();
    assert!(words(&native[1]) > 300, "the listing page holds its native words");
    assert_eq!(
        automatic[1], native[1],
        "native text that owns every inked word is returned once, unchanged"
    );
    let call_puts = |page: &str| page.lines().filter(|line| line.trim() == "call puts").count();
    assert!(call_puts(&native[1]) >= 3, "the listing repeats `call puts`");
    assert_eq!(call_puts(&automatic[1]), call_puts(&native[1]));
}

#[tokio::test]
async fn automatic_ocr_returns_a_table_page_once_when_native_text_holds_every_word() {
    assert!(
        !helpers::skip_if_missing(SDM),
        "the corpus manual {SDM} is missing; fetch test_documents with the test-corpus skill"
    );
    let tessdata = tempfile::tempdir().unwrap();
    let config = pinned_english_config(tessdata.path());
    let bytes = three_pages_around(SDM, DESCRIPTOR_TABLE_PAGE);
    let (native, _) = pages_of(
        &bytes,
        &ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        },
    )
    .await;
    let (automatic, _) = pages_of(&bytes, &config).await;
    let words = |page: &str| page.split_whitespace().count();
    assert!(
        words(&native[1]) > 500,
        "the descriptor table page holds its native words"
    );
    assert_eq!(
        words(&automatic[1]),
        words(&native[1]),
        "the OCR table of words the native text already holds is not appended"
    );
    assert_eq!(automatic[1], native[1]);
}

#[tokio::test]
async fn automatic_ocr_returns_a_table_page_once_when_a_cell_starts_with_a_pipe() {
    assert!(
        !helpers::skip_if_missing(SDM),
        "the corpus manual {SDM} is missing; fetch test_documents with the test-corpus skill"
    );
    let tessdata = tempfile::tempdir().unwrap();
    let config = pinned_english_config(tessdata.path());
    let bytes = three_pages_around(SDM, ESCAPED_CELL_TABLE_PAGE);
    let (native, _) = pages_of(
        &bytes,
        &ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        },
    )
    .await;
    let (automatic, _) = pages_of(&bytes, &config).await;
    let words = |page: &str| page.split_whitespace().count();
    assert!(
        words(&native[1]) > 250,
        "the register table page holds its native words"
    );
    assert_eq!(
        words(&automatic[1]),
        words(&native[1]),
        "the OCR table of words the native text already holds is not appended"
    );
    assert_eq!(automatic[1], native[1]);
}

const GLUED_TOKEN_TABLE_PAGE: u32 = 4443;
const CLEAN_TABLE_PAGE: u32 = 4444;

#[tokio::test]
async fn automatic_ocr_keeps_words_that_a_glued_native_token_hides_on_a_table_page() {
    assert!(
        !helpers::skip_if_missing(SDM),
        "the corpus manual {SDM} is missing; fetch test_documents with the test-corpus skill"
    );
    let tessdata = tempfile::tempdir().unwrap();
    let config = pinned_english_config(tessdata.path());
    let bytes = three_pages_around(SDM, GLUED_TOKEN_TABLE_PAGE);
    let (native, _) = pages_of(
        &bytes,
        &ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        },
    )
    .await;
    let (automatic, _) = pages_of(&bytes, &config).await;
    for word in ["Ovf_PMC1", "Ovf_PMC2", "Ovf_PMC3"] {
        let held = |page: &str| page.split_whitespace().any(|token| token == word);
        assert!(!held(&native[1]), "the native text glues `{word}` into a longer token");
        assert!(held(&automatic[1]), "automatic OCR keeps `{word}`");
    }
    for word in ["Ovf_PMC4", "Ovf_PMC6"] {
        assert!(
            automatic[1].split_whitespace().any(|token| token == word),
            "automatic OCR keeps the clean native token `{word}`"
        );
    }
}

#[tokio::test]
async fn automatic_ocr_returns_a_neighbouring_clean_table_page_once() {
    assert!(
        !helpers::skip_if_missing(SDM),
        "the corpus manual {SDM} is missing; fetch test_documents with the test-corpus skill"
    );
    let tessdata = tempfile::tempdir().unwrap();
    let config = pinned_english_config(tessdata.path());
    let bytes = three_pages_around(SDM, CLEAN_TABLE_PAGE);
    let (native, _) = pages_of(
        &bytes,
        &ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        },
    )
    .await;
    let (automatic, _) = pages_of(&bytes, &config).await;
    assert!(
        native[1].split_whitespace().count() > 100,
        "the clean table page holds its native words"
    );
    assert_eq!(automatic[1], native[1], "a table page with no glued token stays native");
}

const UNDERSCORE_PAGES: [u32; 3] = [2383, 2486, 2645];
const NON_TEXT_BLOCK_PAGES: [u32; 2] = [4420, 4422];
// A page that needs no OCR beside the table page, which keeps the excerpt on the route that mixes
// native and OCR pages, as in the whole manual: an excerpt of OCR pages only is read as a scan.
const NATIVE_COMPANION_PAGE: u32 = 4410;

#[tokio::test]
async fn automatic_ocr_returns_a_page_once_when_its_underscores_hang_below_their_lines() {
    assert!(
        !helpers::skip_if_missing(SDM),
        "the corpus manual {SDM} is missing; fetch test_documents with the test-corpus skill"
    );
    for page in UNDERSCORE_PAGES {
        let tessdata = tempfile::tempdir().unwrap();
        let config = pinned_english_config(tessdata.path());
        let bytes = three_pages_around(SDM, page);
        let (native, _) = pages_of(
            &bytes,
            &ExtractionConfig {
                disable_ocr: true,
                ..config.clone()
            },
        )
        .await;
        let (automatic, _) = pages_of(&bytes, &config).await;
        assert!(
            native[1].split_whitespace().count() > 250,
            "page {page} holds its native words"
        );
        assert_eq!(
            automatic[1], native[1],
            "page {page} is returned once, from its native text"
        );
    }
}

#[tokio::test]
async fn automatic_ocr_returns_a_table_page_once_when_tesseract_places_words_in_non_text_blocks() {
    assert!(
        !helpers::skip_if_missing(SDM),
        "the corpus manual {SDM} is missing; fetch test_documents with the test-corpus skill"
    );
    for page in NON_TEXT_BLOCK_PAGES {
        let tessdata = tempfile::tempdir().unwrap();
        let config = pinned_english_config(tessdata.path());
        let bytes = pages_excerpt(SDM, &[NATIVE_COMPANION_PAGE, page]);
        let (native, _) = pages_of_excerpt(
            &bytes,
            &ExtractionConfig {
                disable_ocr: true,
                ..config.clone()
            },
            2,
        )
        .await;
        let (automatic, _) = pages_of_excerpt(&bytes, &config, 2).await;
        assert!(
            native[1].split_whitespace().count() > 250,
            "page {page} holds its native words"
        );
        assert_eq!(
            automatic[1], native[1],
            "page {page} is returned once, from its native text"
        );
    }
}
