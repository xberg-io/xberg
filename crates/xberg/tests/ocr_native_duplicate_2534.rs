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
const ENGLISH_MODEL_SHA256: &str = "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2";

fn three_pages_around(page: u32) -> Vec<u8> {
    use lopdf::Object;

    let mut document = lopdf::Document::load(helpers::get_test_file_path(BOOK)).expect("open the corpus PDF");
    let pages = document.get_pages();
    let kept: Vec<Object> = (page - 1..=page + 1)
        .map(|number| Object::Reference(pages[&number]))
        .collect();
    let root = document
        .catalog()
        .unwrap()
        .get(b"Pages")
        .unwrap()
        .as_reference()
        .unwrap();
    for number in pages.keys().filter(|number| number.abs_diff(page) > 1) {
        document.objects.remove(&pages[number]);
    }
    let tree = document.get_object_mut(root).and_then(Object::as_dict_mut).unwrap();
    tree.set("Kids", kept);
    tree.set("Count", 3);
    for number in page - 1..=page + 1 {
        document
            .get_object_mut(pages[&number])
            .and_then(Object::as_dict_mut)
            .unwrap()
            .set("Parent", root);
    }
    document.prune_objects();
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("save the three-page excerpt");
    bytes
}

async fn pages_of(bytes: &[u8], config: &ExtractionConfig) -> (Vec<String>, Option<ExtractionMethod>) {
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
    assert_eq!(pages.len(), 3);
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
    let model = xberg_tesseract::bundled_eng_traineddata().expect("bundled English model");
    assert_eq!(hex::encode(Sha256::digest(model)), ENGLISH_MODEL_SHA256);
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
    let bytes = three_pages_around(LISTING_PAGE);
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
