use lopdf::{Document, Object, dictionary};
use xberg::{ExtractedDocument, ExtractionConfig};

fn resources(document: &Document, page: &lopdf::Dictionary) -> lopdf::Dictionary {
    match page.get(b"Resources").unwrap() {
        Object::Reference(id) => document.get_dictionary(*id).unwrap().clone(),
        Object::Dictionary(value) => value.clone(),
        value => panic!("unexpected page resources: {value:?}"),
    }
}

fn stamped_table_pages() -> Vec<u8> {
    let source = include_bytes!("../fixtures/ocr/shaded_table_scan.pdf");
    let mut document = Document::load_mem(source).unwrap();
    let page_id = *document.get_pages().values().next().unwrap();
    let mut page = document.get_dictionary(page_id).unwrap().clone();
    let root = page.get(b"Parent").unwrap().as_reference().unwrap();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut page_resources = resources(&document, &page);
    page_resources.set("Font", dictionary! {"ReviewFont" => font});
    page.set("Resources", page_resources);
    let second = document.add_object(page.clone());
    document.objects.insert(page_id, Object::Dictionary(page));
    for id in [page_id, second] {
        document
            .add_page_contents(id, b"BT /ReviewFont 10 Tf 3 Tr 40 40 Td (REVIEW COPY) Tj ET".to_vec())
            .unwrap();
    }
    let tree = document.get_dictionary_mut(root).unwrap();
    tree.set("Kids", vec![Object::Reference(page_id), Object::Reference(second)]);
    tree.set("Count", 2);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

async fn read(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    super::helpers::extract_bytes_document(bytes, "application/pdf", config)
        .await
        .expect("extract stamped table pages")
}

fn assert_same_page_payload(actual: &ExtractedDocument, forced: &ExtractedDocument) {
    assert_eq!(actual.tables.len(), 2, "all accepted OCR tables survive");
    let actual_pages = actual.pages.as_ref().unwrap();
    let forced_pages = forced.pages.as_ref().unwrap();
    assert_eq!(actual_pages.len(), 2);
    for (actual_page, forced_page) in actual_pages.iter().zip(forced_pages) {
        assert_eq!(actual_page.page_number, forced_page.page_number);
        assert_eq!(actual_page.tables.len(), 1, "the OCR table belongs to its page");
        assert_eq!(forced_page.tables.len(), 1, "the control has one table per page");
        assert_eq!(
            serde_json::to_value(actual_page.tables[0].as_ref()).unwrap(),
            serde_json::to_value(forced_page.tables[0].as_ref()).unwrap()
        );
        assert_eq!(actual_page.content, forced_page.content, "per-page OCR text survives");
    }
    assert_eq!(
        serde_json::to_value(&actual.tables).unwrap(),
        serde_json::to_value(&forced.tables).unwrap()
    );
}

async fn whole_document_control(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let forced = read(
        bytes,
        &ExtractionConfig {
            force_ocr: true,
            ..config.clone()
        },
    )
    .await;
    assert_eq!(forced.tables.len(), 2, "the control recognizes both tables");
    forced
}

#[tokio::test]
async fn automatic_all_pages_keeps_the_ocr_tables_and_page_texts() {
    let models = tempfile::tempdir().unwrap();
    let automatic = super::pinned_config(models.path());
    let bytes = stamped_table_pages();
    let native = read(
        &bytes,
        &ExtractionConfig {
            disable_ocr: true,
            ..automatic.clone()
        },
    )
    .await;
    assert_eq!(native.pages.as_ref().unwrap().len(), 2);
    assert!(
        native
            .pages
            .as_ref()
            .unwrap()
            .iter()
            .all(|page| page.content.contains("REVIEW COPY"))
    );
    let forced = whole_document_control(&bytes, &automatic).await;
    let actual = read(&bytes, &automatic).await;
    assert_eq!(actual.extraction_method, Some(xberg::types::ExtractionMethod::Mixed));
    assert_same_page_payload(&actual, &forced);
}

#[tokio::test]
async fn requested_document_structure_and_semantic_elements_keep_the_whole_document_payload() {
    let models = tempfile::tempdir().unwrap();
    let mut automatic = super::pinned_config(models.path());
    automatic.include_document_structure = true;
    automatic.result_format = xberg::types::ResultFormat::ElementBased;
    let bytes = stamped_table_pages();
    let forced = whole_document_control(&bytes, &automatic).await;
    assert!(!forced.document.as_ref().unwrap().nodes.is_empty());
    assert!(!forced.elements.as_ref().unwrap().is_empty());
    let actual = read(&bytes, &automatic).await;
    assert_same_page_payload(&actual, &forced);
    assert_eq!(
        serde_json::to_value(&actual.document).unwrap(),
        serde_json::to_value(&forced.document).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&actual.elements).unwrap(),
        serde_json::to_value(&forced.elements).unwrap()
    );
    assert_eq!(actual.extraction_method, Some(xberg::types::ExtractionMethod::Ocr));
}

#[tokio::test]
async fn requested_ocr_elements_keep_the_whole_document_payload() {
    let models = tempfile::tempdir().unwrap();
    let mut automatic = super::pinned_config(models.path());
    automatic.ocr.as_mut().unwrap().element_config = Some(xberg::types::OcrElementConfig {
        include_elements: true,
        ..Default::default()
    });
    let bytes = stamped_table_pages();
    let forced = whole_document_control(&bytes, &automatic).await;
    let actual = read(&bytes, &automatic).await;
    assert_eq!(actual.extraction_method, Some(xberg::types::ExtractionMethod::Ocr));
    assert!(!forced.ocr_elements.as_ref().unwrap().is_empty());
    assert_eq!(
        serde_json::to_value(&actual.ocr_elements).unwrap(),
        serde_json::to_value(&forced.ocr_elements).unwrap()
    );
    assert_same_page_payload(&actual, &forced);
}
