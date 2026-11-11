use lopdf::{Document, Object, dictionary};
use xberg::{ExtractedDocument, ExtractionConfig, types::ExtractionMethod};

fn save(document: &mut Document, pages: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    assert_eq!(Document::load_mem(&bytes).unwrap().get_pages().len(), pages);
    bytes
}

fn digit_font(document: &mut Document) -> lopdf::ObjectId {
    let cmap = document.add_object(lopdf::Stream::new(
        lopdf::Dictionary::new(),
        br#"
/CIDInit /ProcSet findresource begin 12 dict begin begincmap
/CIDSystemInfo << /Registry (Test) /Ordering (Unicode) /Supplement 0 >> def
/CMapName /KhmerDigitHint def /CMapType 2 def
1 begincodespacerange <00> <FF> endcodespacerange
1 beginbfchar <30> <17E0> endbfchar
endcmap CMapName currentdict /CMap defineresource pop end end
"#
        .to_vec(),
    ));
    document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "ToUnicode" => cmap,
    })
}

fn english_page_with_one_khmer_digit() -> Vec<u8> {
    let mut document = Document::load_mem(&super::excerpt(&[super::SOURCE_PAGE])).unwrap();
    let id = *document.get_pages().values().next().unwrap();
    let page = document.get_dictionary(id).unwrap().clone();
    let mut resources = super::structured_output::resources(&document, &page);
    let mut fonts = match resources.get(b"Font").unwrap() {
        Object::Reference(id) => document.get_dictionary(*id).unwrap().clone(),
        Object::Dictionary(value) => value.clone(),
        value => panic!("unexpected fonts: {value:?}"),
    };
    assert!(fonts.get(b"KhmerDigitHint").is_err());
    fonts.set("KhmerDigitHint", digit_font(&mut document));
    resources.set("Font", fonts);
    document.get_dictionary_mut(id).unwrap().set("Resources", resources);
    document
        .add_page_contents(id, b"BT /KhmerDigitHint 10 Tf 3 Tr 40 20 Td (0) Tj ET".to_vec())
        .unwrap();
    save(&mut document, 1)
}

fn two_visible_native_tables() -> Vec<u8> {
    let mut document = Document::load_mem(&super::excerpt(&[super::SOURCE_PAGE])).unwrap();
    let first = *document.get_pages().values().next().unwrap();
    let page = document.get_dictionary(first).unwrap().clone();
    let root = page.get(b"Parent").unwrap().as_reference().unwrap();
    let second = document.add_object(page);
    assert_ne!(first, second);
    let tree = document.get_dictionary_mut(root).unwrap();
    tree.set("Kids", vec![Object::Reference(first), Object::Reference(second)]);
    tree.set("Count", 2);
    save(&mut document, 2)
}

fn png(bytes: &[u8], page: usize) -> Vec<u8> {
    let document = xberg_native_pdf::PdfDocument::from_bytes(bytes.to_vec()).unwrap();
    xberg_native_pdf::rendering::render_page(
        &document,
        page,
        &xberg_native_pdf::rendering::RenderOptions::with_dpi(150),
    )
    .unwrap()
    .data
}

async fn read(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    super::helpers::extract_bytes_document(bytes, "application/pdf", config)
        .await
        .unwrap()
}

#[tokio::test]
async fn one_native_khmer_digit_keeps_the_entire_english_document_on_its_original_route() {
    let models = tempfile::tempdir().unwrap();
    let mut config = super::pinned_config(models.path());
    config.ocr.as_mut().unwrap().tesseract_config = Some(xberg::TesseractConfig {
        use_cache: false,
        ..Default::default()
    });
    let plain = super::excerpt(&[super::SOURCE_PAGE]);
    let bytes = english_page_with_one_khmer_digit();
    assert_eq!(
        png(&plain, 0),
        png(&bytes, 0),
        "the hint changes no painted English source"
    );
    let native = read(
        &bytes,
        &ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        },
    )
    .await;
    assert_eq!(native.content.chars().filter(|c| *c == '\u{17e0}').count(), 1);
    assert!(native.content.split_whitespace().count() >= super::SOURCE_WORDS);
    let baseline = read(&plain, &config).await;
    assert_eq!(baseline.extraction_method, Some(ExtractionMethod::Native));
    assert_eq!(baseline.content.split_whitespace().count(), super::SOURCE_WORDS);
    assert_original_document_route(&bytes, &config).await;
}

async fn assert_original_document_route(bytes: &[u8], config: &ExtractionConfig) {
    let original_route = read(
        bytes,
        &ExtractionConfig {
            include_document_structure: true,
            ..config.clone()
        },
    )
    .await;
    assert_eq!(original_route.extraction_method, Some(ExtractionMethod::Ocr));
    let actual = read(bytes, config).await;
    assert_eq!(
        actual.extraction_method,
        Some(ExtractionMethod::Ocr),
        "one native Khmer digit conservatively withholds the adapter for the whole English document"
    );
    assert!(
        actual
            .content
            .split_whitespace()
            .eq(original_route.content.split_whitespace())
    );
    assert_eq!(
        super::page_tables_json(&actual.pages.as_ref().unwrap()[0]),
        super::page_tables_json(&original_route.pages.as_ref().unwrap()[0])
    );
}

async fn assert_image_ocr_has_no_table(bytes: &[u8], config: &ExtractionConfig) {
    let raster = png(bytes, 1);
    let image = super::helpers::extract_bytes_document(&raster, "image/png", config)
        .await
        .unwrap();
    assert!(!image.content.trim().is_empty());
    assert!(image.tables.is_empty(), "OCR table detection was deliberately disabled");
}

#[tokio::test]
async fn replaced_text_without_an_ocr_table_keeps_native_tables_on_both_pages() {
    let models = tempfile::tempdir().unwrap();
    let mut config = super::pinned_config(models.path());
    config.ocr.as_mut().unwrap().tesseract_config = Some(xberg::TesseractConfig {
        enable_table_detection: false,
        use_cache: false,
        ..Default::default()
    });
    let bytes = two_visible_native_tables();
    let native = read(
        &bytes,
        &ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
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
            .all(|page| !page.tables.is_empty())
    );
    assert_image_ocr_has_no_table(&bytes, &config).await;
    let forced = read(
        &bytes,
        &ExtractionConfig {
            force_ocr: true,
            ..config.clone()
        },
    )
    .await;
    config.force_ocr_pages = Some(vec![2]);
    let actual = read(&bytes, &config).await;
    assert_eq!(actual.extraction_method, Some(ExtractionMethod::Mixed));
    assert_retained_tables(&actual, &native, &forced);
}

fn assert_retained_tables(actual: &ExtractedDocument, native: &ExtractedDocument, forced: &ExtractedDocument) {
    let pages = actual.pages.as_ref().unwrap();
    let source = native.pages.as_ref().unwrap();
    assert_eq!(pages[0].content, source[0].content);
    super::plain_contract::assert_page_ranges(actual);
    super::plain_contract::assert_page_ranges(forced);
    assert_eq!(
        pages[1]
            .content
            .lines()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>(),
        forced.pages.as_ref().unwrap()[1]
            .content
            .lines()
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>(),
        "Plain paragraph separators may differ; every nonempty line stays byte-exact"
    );
    assert!(
        !pages[1]
            .content
            .split_whitespace()
            .eq(source[1].content.split_whitespace()),
        "page 2 text is replaced"
    );
    for (page, original) in pages.iter().zip(source) {
        assert_eq!(super::page_tables_json(page), super::page_tables_json(original));
    }
    assert_eq!(
        serde_json::to_value(&actual.tables).unwrap(),
        serde_json::to_value(&native.tables).unwrap()
    );
}
