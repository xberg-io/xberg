#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng"
))]

mod helpers;

use sha2::{Digest, Sha256};

const FIRST: &str =
    "Lorem Ipsum is simply dummy text of the printing and typesetting industry. Lorem Ipsum has been the";
const SECOND: &str =
    "industry's standard dummy text ever since the 1500s, when an unknown printer took a galley of type and";
const THIRD: &str = "scrambled it to make a type specimen book";
const ORDINALS: [char; 6] = ['1', '2', '3', '1', '2', '3'];
const DOCUMENT_SHA: &str = "89b055edbdfb99a7cf707b5d15c1bf65c2460ddddf9c9a6bfb54bc79d786d1f7";
const PAGE_SHA: &str = "1d6b45f0322b7f2c3bc19ef3b0e0ad099112d14ad79e61c0109441ae7bd9900b";

fn sha(bytes: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn assert_projection(content: &str, original_len: usize, cut: usize, original_sha: &str) {
    assert!(content.len() > original_len, "right-column recovery absent");
    let end = cut + content.len() - original_len;
    let inserted = &content[cut..end];
    assert_eq!(
        sha([&content.as_bytes()[..cut], &content.as_bytes()[end..]].concat()),
        original_sha
    );
    let lines: Vec<_> = inserted
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(lines.len(), 18);
    for (row, number) in lines.chunks_exact(3).zip(ORDINALS) {
        let marked = row[0].strip_prefix(number).expect("exact source ordinal");
        let body = marked.strip_prefix(['.', ',']).unwrap_or(marked);
        assert_eq!(body.strip_prefix(' '), Some(FIRST));
        assert_eq!(row[1..], [SECOND, THIRD]);
    }
}

fn pinned_page() -> Vec<u8> {
    let path = helpers::get_test_documents_dir().join("pdf/sharable_web_guide.pdf");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(
        sha(&bytes),
        "04c76db01e20d7641e886b2403bea2df5757dfe9b4be7a283ee44d1861081a06"
    );
    let mut document = lopdf::Document::load_mem(&bytes).expect("read pinned corpus PDF");
    let pages = document.get_pages();
    let fourth = pages[&4];
    document.delete_pages(&pages.into_keys().filter(|page| *page != 4).collect::<Vec<_>>());
    assert_eq!(document.get_pages().into_values().collect::<Vec<_>>(), [fourth]);
    document.prune_objects();
    let mut isolated = Vec::new();
    document.save_to(&mut isolated).expect("serialize isolated source page");
    assert_source_image(&isolated);
    isolated
}

fn assert_source_image(bytes: &[u8]) {
    let page = lopdf::Document::load_mem(bytes).expect("reopen isolated PDF");
    let pages = page.get_pages();
    let page_id = pages[&1];
    let images = page.get_page_images(page_id).expect("read isolated image");
    let [image] = images.as_slice() else {
        panic!("one painted source image")
    };
    assert_eq!(
        (image.width, image.height, image.bits_per_component),
        (1856, 12303, Some(8))
    );
    assert_eq!(
        sha(image.content),
        "c6c2e5126ad245cb13541d615fdcc28cb950055fc28c08950c12a17f07251fa3"
    );
    let (_, indexed) = page.dereference(image.origin_dict.get(b"ColorSpace").unwrap()).unwrap();
    let palette = &indexed.as_array().unwrap()[3];
    let (_, palette) = page.dereference(palette).unwrap();
    assert_eq!(
        sha(palette.as_stream().unwrap().decompressed_content().unwrap()),
        "5e025eac07f934aece65431fc7309fa0248882b891845d62323537e1dfc4e503"
    );
    let paint = page.get_page_content(page_id);
    assert_eq!(
        std::str::from_utf8(&paint)
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>(),
        [
            "q",
            "1855.7700043",
            "0",
            "0",
            "12301.4600067",
            "0",
            "0",
            "cm",
            "/Im0",
            "Do",
            "Q"
        ]
    );
}

fn write_english_model(directory: &std::path::Path) {
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English data");
    assert_eq!(
        sha(english),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(directory.join("eng.traineddata"), english).expect("write private English data");
}

async fn read_pinned(use_cache: bool) -> xberg::types::ExtractedDocument {
    let bytes = pinned_page();
    let tessdata = tempfile::tempdir().expect("create private tessdata directory");
    write_english_model(tessdata.path());
    let mut config: xberg::ExtractionConfig = serde_json::from_str(
        r#"{"use_cache":false,"ocr":{"backend":"tesseract","language":["eng"]},"pages":{"extract_pages":true}}"#,
    )
    .unwrap();
    config.use_cache = use_cache;
    let ocr = config.ocr.as_mut().unwrap();
    ocr.tessdata_path = Some(tessdata.path().to_path_buf());
    // Keep the inner Tesseract cache out of this test. Only the outer switch controls admission.
    ocr.backend_options = Some(serde_json::json!({"use_cache": false}));
    let output = xberg::extract(
        xberg::ExtractInput::from_bytes(bytes, "application/pdf", Some("page4.pdf".into())),
        &config,
    )
    .await
    .unwrap();
    assert!(output.errors.is_empty());
    assert_eq!(output.results.len(), 1);
    output.results.into_iter().next().unwrap()
}

#[tokio::test]
async fn ordered_right_column_adds_only_source_lines_and_preserves_both_original_projections() {
    let doc = read_pinned(false).await;
    let [page] = doc.pages.as_ref().unwrap().as_slice() else {
        panic!("exactly one page")
    };
    assert_eq!(page.page_number, 1);
    let [table] = doc.tables.as_slice() else {
        panic!("the original table survives")
    };
    let [page_table] = page.tables.as_slice() else {
        panic!("the original page table survives")
    };
    assert_eq!(
        serde_json::to_value(table).unwrap(),
        serde_json::to_value(page_table.as_ref()).unwrap()
    );
    assert_eq!(
        table.cells[0],
        ["Palo alte Netware,", "the plbaleypersecurty lender", "gah ne"]
    );
    assert_projection(&doc.content, 2321, 2267, DOCUMENT_SHA);
    assert_projection(&page.content, 2316, 2266, PAGE_SHA);
}

#[tokio::test]
async fn cache_enabled_pdf_keeps_both_stock_projections() {
    let doc = read_pinned(true).await;
    let [page] = doc.pages.as_ref().unwrap().as_slice() else {
        panic!("exactly one page")
    };
    assert_eq!(sha(&doc.content), DOCUMENT_SHA);
    assert_eq!(sha(&page.content), PAGE_SHA);
    assert_eq!(doc.tables.len(), 1);
    assert_eq!(page.tables.len(), 1);
}
