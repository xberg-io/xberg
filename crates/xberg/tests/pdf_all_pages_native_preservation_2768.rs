#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]

#[path = "pdf_all_pages_native_preservation_2768/document_scope.rs"]
mod document_scope;
mod helpers;

use lopdf::Object;
use sha2::{Digest, Sha256};
use xberg::{ExtractionConfig, OcrConfig, PageConfig, TesseractConfig};

const SOURCE: &str = "pdf/intel_64_and_ia_32_architectures_software_developer_s_manual_combined_volumes_1_4_june_2021_325462_sdm_vol_1_2abcd_3abcd.pdf";
const SOURCE_SHA256: &str = "ef129f8b18963a55deb190e2fbff40f7e3bacbee4afc248c9c9087093d08be48";
const MODEL_SHA256: &str = "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2";
const SOURCE_PAGE: u32 = 815;
const SOURCE_WORDS: usize = 550;

fn excerpt(wanted: &[u32]) -> Vec<u8> {
    let bytes = std::fs::read(helpers::get_test_file_path(SOURCE)).expect("required public SDM corpus fixture");
    assert_eq!(hex::encode(Sha256::digest(&bytes)), SOURCE_SHA256);
    let mut document = lopdf::Document::load_mem(&bytes).expect("open the pinned source");
    let pages = document.get_pages();
    let root = document
        .catalog()
        .unwrap()
        .get(b"Pages")
        .unwrap()
        .as_reference()
        .unwrap();
    let kept: Vec<Object> = wanted.iter().map(|number| Object::Reference(pages[number])).collect();
    for number in pages.keys().filter(|number| !wanted.contains(number)) {
        document.objects.remove(&pages[number]);
    }
    let tree = document.get_dictionary_mut(root).unwrap();
    tree.set("Kids", kept);
    tree.set("Count", i64::try_from(wanted.len()).unwrap());
    for number in wanted {
        document.get_dictionary_mut(pages[number]).unwrap().set("Parent", root);
    }
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("save the excerpt");
    bytes
}

fn pinned_config(tessdata: &std::path::Path) -> ExtractionConfig {
    let model = xberg_tesseract::bundled_eng_traineddata().expect("bundled English model");
    assert_eq!(hex::encode(Sha256::digest(model)), MODEL_SHA256);
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

async fn page_texts(bytes: &[u8], config: &ExtractionConfig, expected_pages: usize) -> Vec<String> {
    let document = helpers::extract_bytes_document(bytes, "application/pdf", config)
        .await
        .expect("extract the public excerpt");
    let pages = document.pages.expect("requested page text");
    assert_eq!(pages.len(), expected_pages);
    pages.into_iter().map(|page| page.content).collect()
}

#[tokio::test]
async fn automatic_all_pages_ocr_preserves_the_same_native_table_as_the_mixed_route() {
    let tessdata = tempfile::tempdir().unwrap();
    let automatic = pinned_config(tessdata.path());
    let native = ExtractionConfig {
        disable_ocr: true,
        ..automatic.clone()
    };
    let mixed = excerpt(&[SOURCE_PAGE - 1, SOURCE_PAGE, SOURCE_PAGE + 1]);
    let mixed_native = page_texts(&mixed, &native, 3).await;
    let mixed_automatic = page_texts(&mixed, &automatic, 3).await;
    assert_eq!(mixed_native[1].split_whitespace().count(), SOURCE_WORDS);
    assert_eq!(
        mixed_automatic[1], mixed_native[1],
        "the existing mixed route keeps native text once"
    );

    let single = excerpt(&[SOURCE_PAGE]);
    let single_native = page_texts(&single, &native, 1).await;
    let single_automatic = page_texts(&single, &automatic, 1).await;
    assert_eq!(single_native[0].split_whitespace().count(), SOURCE_WORDS);
    assert_eq!(
        single_automatic[0].split_whitespace().count(),
        SOURCE_WORDS,
        "automatic all-pages OCR must preserve the same 550 source words as the mixed route"
    );
    assert_eq!(single_automatic[0], single_native[0]);
}
