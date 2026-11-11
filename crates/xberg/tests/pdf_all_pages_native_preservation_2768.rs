#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]

#[path = "pdf_all_pages_native_preservation_2768/acceptance_controls.rs"]
mod acceptance_controls;
#[path = "pdf_all_pages_native_preservation_2768/document_contracts.rs"]
mod document_contracts;
#[path = "pdf_all_pages_native_preservation_2768/document_scope.rs"]
mod document_scope;
mod helpers;
#[path = "pdf_all_pages_native_preservation_2768/mixed_tables.rs"]
mod mixed_tables;
#[path = "pdf_all_pages_native_preservation_2768/plain_contract.rs"]
mod plain_contract;
#[path = "pdf_all_pages_native_preservation_2768/structured_output.rs"]
mod structured_output;

use lopdf::Object;
use sha2::{Digest, Sha256};
use xberg::{ExtractionConfig, OcrConfig, PageConfig};

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
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn read_document(bytes: &[u8], config: &ExtractionConfig, expected_pages: usize) -> xberg::ExtractedDocument {
    let document = helpers::extract_bytes_document(bytes, "application/pdf", config)
        .await
        .expect("extract the public excerpt");
    assert_eq!(
        document.pages.as_ref().expect("requested page text").len(),
        expected_pages
    );
    document
}

fn page_tables_json(page: &xberg::types::PageContent) -> serde_json::Value {
    serde_json::to_value(page.tables.iter().map(|table| table.as_ref()).collect::<Vec<_>>()).unwrap()
}

fn assert_kept_native_page(actual: &xberg::ExtractedDocument, native: &xberg::ExtractedDocument, index: usize) {
    let actual_page = &actual.pages.as_ref().unwrap()[index];
    let native_page = &native.pages.as_ref().unwrap()[index];
    assert_eq!(native_page.content.split_whitespace().count(), SOURCE_WORDS);
    assert_eq!(actual_page.content, native_page.content);
    assert!(
        !native_page.tables.is_empty(),
        "the source control detects a native table"
    );
    assert_eq!(page_tables_json(actual_page), page_tables_json(native_page));
    let tables = |document: &xberg::ExtractedDocument| {
        serde_json::to_value(
            document
                .tables
                .iter()
                .filter(|table| table.page_number == native_page.page_number)
                .collect::<Vec<_>>(),
        )
        .unwrap()
    };
    assert_eq!(tables(actual), tables(native));
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
    let mixed_native = read_document(&mixed, &native, 3).await;
    let mixed_automatic = read_document(&mixed, &automatic, 3).await;
    assert_kept_native_page(&mixed_automatic, &mixed_native, 1);

    let single = excerpt(&[SOURCE_PAGE]);
    let single_native = read_document(&single, &native, 1).await;
    let single_automatic = read_document(&single, &automatic, 1).await;
    assert_kept_native_page(&single_automatic, &single_native, 0);
}
