use lopdf::{Document, Object};
use xberg::{ExtractedDocument, ExtractionConfig};

fn inherited_page(document: &Document, id: lopdf::ObjectId) -> lopdf::Dictionary {
    let mut page = document.get_dictionary(id).unwrap().clone();
    let mut parent = page.get(b"Parent").unwrap().as_reference().unwrap();
    loop {
        let ancestor = document.get_dictionary(parent).unwrap();
        for key in [b"Resources".as_slice(), b"MediaBox", b"CropBox", b"Rotate"] {
            if page.get(key).is_err()
                && let Ok(value) = ancestor.get(key)
            {
                page.set(key.to_vec(), value.clone());
            }
        }
        let Ok(next) = ancestor.get(b"Parent") else {
            break;
        };
        parent = next.as_reference().unwrap();
    }
    page
}

fn with_scanned_table(source_pages: &[u32], stamp: bool) -> Vec<u8> {
    let mut native = Document::load_mem(&super::excerpt(source_pages)).unwrap();
    let scan_bytes = if stamp {
        super::structured_output::stamped_table_pages()
    } else {
        include_bytes!("../fixtures/ocr/shaded_table_scan.pdf").to_vec()
    };
    let mut scan = Document::load_mem(&scan_bytes).unwrap();
    scan.renumber_objects_with(native.max_id + 1);
    let scan_id = *scan.get_pages().values().next().unwrap();
    let mut scan_page = inherited_page(&scan, scan_id);
    let root = native.catalog().unwrap().get(b"Pages").unwrap().as_reference().unwrap();
    let mut children: Vec<_> = native.get_pages().into_values().map(Object::Reference).collect();
    children.push(Object::Reference(scan_id));
    scan_page.set("Parent", root);
    native.max_id = scan.max_id.max(native.max_id);
    native.objects.extend(scan.objects);
    native.objects.insert(scan_id, Object::Dictionary(scan_page));
    let tree = native.get_dictionary_mut(root).unwrap();
    tree.set("Count", i64::try_from(children.len()).unwrap());
    tree.set("Kids", children);
    let mut bytes = Vec::new();
    native.save_to(&mut bytes).unwrap();
    assert_eq!(
        Document::load_mem(&bytes).unwrap().get_pages().len(),
        source_pages.len() + 1
    );
    bytes
}

async fn read(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    super::helpers::extract_bytes_document(bytes, "application/pdf", config)
        .await
        .unwrap()
}

fn assert_mixed_tables(actual: &ExtractedDocument, native: &ExtractedDocument, forced: &ExtractedDocument) {
    let pages = actual.pages.as_ref().unwrap();
    let last = pages.len() - 1;
    let source = native.pages.as_ref().unwrap();
    let recognized = forced.pages.as_ref().unwrap();
    assert_eq!(actual.extraction_method, Some(xberg::types::ExtractionMethod::Mixed));
    assert!(!source[last - 1].tables.is_empty(), "the SDM control has native tables");
    assert!(source[last].tables.is_empty(), "the raster control has no native table");
    assert_eq!(recognized[last].tables.len(), 1, "the scanned table is recognized");
    let mut expected = Vec::new();
    for (index, page) in pages.iter().enumerate() {
        let reference = if index == last {
            &recognized[index]
        } else {
            &source[index]
        };
        assert_eq!(page.content, reference.content);
        assert_eq!(super::page_tables_json(page), super::page_tables_json(reference));
        expected.extend(reference.tables.iter().map(|table| table.as_ref()));
    }
    assert_eq!(
        serde_json::to_value(&actual.tables).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert_ne!(
        pages[last].content, source[last].content,
        "the raster page was replaced"
    );
}

async fn check_mixed_tables(source_pages: &[u32], stamp: bool, force_scan: bool) {
    let models = tempfile::tempdir().unwrap();
    let mut config = super::pinned_config(models.path());
    let bytes = with_scanned_table(source_pages, stamp);
    let native = read(
        &bytes,
        &ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        },
    )
    .await;
    let forced = read(
        &bytes,
        &ExtractionConfig {
            force_ocr: true,
            ..config.clone()
        },
    )
    .await;
    if stamp && !force_scan {
        let whole = read(
            &bytes,
            &ExtractionConfig {
                include_document_structure: true,
                ..config.clone()
            },
        )
        .await;
        assert_eq!(
            whole.extraction_method,
            Some(xberg::types::ExtractionMethod::Ocr),
            "an unforced request that excludes the adapter takes the whole-document gate"
        );
    }
    if force_scan {
        config.force_ocr_pages = Some(vec![u32::try_from(source_pages.len() + 1).unwrap()]);
    }
    let actual = read(&bytes, &config).await;
    assert_mixed_tables(&actual, &native, &forced);
}

#[tokio::test]
async fn automatic_all_pages_keeps_native_tables_and_replaces_the_stamped_scan() {
    check_mixed_tables(&[super::SOURCE_PAGE], true, false).await;
}

#[tokio::test]
async fn forced_page_ocr_attaches_only_the_replaced_pages_tables() {
    check_mixed_tables(&[super::SOURCE_PAGE], false, true).await;
}

#[tokio::test]
async fn detected_scanned_page_attaches_ocr_tables_and_keeps_native_tables() {
    check_mixed_tables(&[super::SOURCE_PAGE - 1, super::SOURCE_PAGE], false, false).await;
}
