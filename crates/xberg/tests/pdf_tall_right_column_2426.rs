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
    let original = [&content.as_bytes()[..cut], &content.as_bytes()[end..]].concat();
    assert_eq!(
        sha(&original),
        original_sha,
        "OCR text around the recovered lines changed:\n{}",
        String::from_utf8_lossy(&original)
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

fn pinned_config(use_cache: bool, use_ocr_cache: bool, tessdata: &std::path::Path) -> xberg::ExtractionConfig {
    let mut config: xberg::ExtractionConfig = serde_json::from_str(
        r#"{"use_cache":false,"ocr":{"backend":"tesseract","language":["eng"]},"pages":{"extract_pages":true}}"#,
    )
    .unwrap();
    config.use_cache = use_cache;
    let ocr = config.ocr.as_mut().unwrap();
    ocr.tessdata_path = Some(tessdata.to_path_buf());
    ocr.backend_options = Some(serde_json::json!({"use_cache": use_ocr_cache}));
    config
}

async fn read_pinned(
    engine: &xberg::engine::Engine,
    bytes: &[u8],
    config: &xberg::ExtractionConfig,
) -> xberg::types::ExtractedDocument {
    let output = engine
        .extract(
            xberg::ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", Some("page4.pdf".into())),
            config,
        )
        .await
        .unwrap();
    assert!(output.errors.is_empty());
    assert_eq!(output.results.len(), 1);
    output.results.into_iter().next().unwrap()
}

#[tokio::test]
async fn ordered_right_column_adds_only_source_lines_and_preserves_both_original_projections() {
    let tessdata = tempfile::tempdir().expect("private tessdata directory");
    write_english_model(tessdata.path());
    let config = pinned_config(false, false, tessdata.path());
    let doc = read_pinned(&xberg::engine::Engine::new_default(), &pinned_page(), &config).await;
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

#[derive(Default)]
struct CountedCache {
    entries: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
    hits: std::sync::atomic::AtomicUsize,
    writes: std::sync::atomic::AtomicUsize,
}

#[async_trait::async_trait]
impl xberg::engine::seams::CacheBackend for CountedCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let bytes = self.entries.lock().unwrap().get(key).cloned();
        if bytes.is_some() {
            self.hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        bytes
    }

    async fn put(&self, key: &str, bytes: Vec<u8>, _ttl: Option<std::time::Duration>) {
        self.entries.lock().unwrap().insert(key.into(), bytes);
        self.writes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[tokio::test]
async fn cached_pdf_recovers_the_same_certified_strips_on_cold_and_warm_reads() {
    use std::sync::{Arc, atomic::Ordering};

    let tessdata = tempfile::tempdir().expect("private tessdata directory");
    write_english_model(tessdata.path());
    let bytes = pinned_page();
    let config = pinned_config(true, false, tessdata.path());
    let cache = Arc::new(CountedCache::default());
    let engine = xberg::engine::Engine::builder()
        .with_cache_backend(cache.clone())
        .build();
    let cold = read_pinned(&engine, &bytes, &config).await;
    assert_projection(&cold.content, 2321, 2267, DOCUMENT_SHA);
    assert_projection(&cold.pages.as_deref().unwrap()[0].content, 2316, 2266, PAGE_SHA);
    assert_eq!(cache.hits.load(Ordering::SeqCst), 0);
    assert_eq!(cache.writes.load(Ordering::SeqCst), 1);

    let warm = read_pinned(&engine, &bytes, &config).await;
    assert_eq!(cache.hits.load(Ordering::SeqCst), 1);
    assert_eq!(cache.writes.load(Ordering::SeqCst), 1);
    assert_eq!(
        serde_json::to_value(&warm).unwrap(),
        serde_json::to_value(&cold).unwrap()
    );
}

#[tokio::test]
async fn explicit_inner_ocr_cache_keeps_both_stock_projections() {
    let tessdata = tempfile::tempdir().expect("private tessdata directory");
    write_english_model(tessdata.path());
    let config = pinned_config(true, true, tessdata.path());
    let doc = read_pinned(&xberg::engine::Engine::new_default(), &pinned_page(), &config).await;
    let [page] = doc.pages.as_ref().unwrap().as_slice() else {
        panic!("exactly one page")
    };
    assert_eq!(sha(&doc.content), DOCUMENT_SHA);
    assert_eq!(sha(&page.content), PAGE_SHA);
    assert_eq!(doc.tables.len(), 1);
    assert_eq!(page.tables.len(), 1);
}
