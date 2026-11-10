use std::sync::Arc;

use super::*;
use crate::extractors::security::SecurityLimits;
use crate::ocr::tesseract_backend::read_counter;

const ARABIC: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/pdf/wrong-script-tounicode-2830/arabic.pdf"
));

/// `security_limits.max_archive_size` values that key `read_counter`, one per case.
const NORMALIZATION_CASE: usize = 0x2862_5101;
const ROUTE_CASE: usize = 0x2862_5102;

/// Words the fixture prints; its `ToUnicode` map spells them in Arabic code points.
const SOURCE_WORDS: [&str; 4] = ["committee", "architect", "residents", "library"];

const HARBOR: [&str; 4] = [
    "the harbor ferry leaves the north pier at seven every summer morning",
    "fishermen unload their crates while gulls circle above the wooden market stalls",
    "visitors buy smoked mackerel and fresh bread before walking along the seawall",
    "by noon the tide turns and the small boats return to harbor",
];
const ORCHARD: [&str; 4] = [
    "orchard workers gather apples from long rows of trees beside the river",
    "baskets are carried to the barn where cider presses run until evening",
    "children sort the bruised fruit into crates for the village bakery today",
    "at dusk the farmer counts the harvest and closes the wooden gate",
];

struct Stock {
    _models: tempfile::TempDir,
    backend: Arc<dyn OcrBackend>,
    config: OcrConfig,
}

fn limits(case: usize) -> SecurityLimits {
    SecurityLimits {
        max_archive_size: case,
        ..Default::default()
    }
}

fn stock(case: usize, backend_options: serde_json::Value) -> Stock {
    crate::plugins::ensure_ocr_backends_initialized();
    let registry = crate::plugins::registry::get_ocr_backend_registry();
    let backend = registry.read().get("tesseract").unwrap();
    assert!(registry.read().is_builtin_tesseract(&backend));
    let models = tempfile::tempdir().unwrap();
    std::fs::write(
        models.path().join("eng.traineddata"),
        xberg_tesseract::bundled_eng_traineddata().expect("verified bundled English model"),
    )
    .unwrap();
    let config = OcrConfig {
        backend: "tesseract".to_string(),
        tessdata_path: Some(models.path().into()),
        auto_rotate: Some(false),
        backend_options: Some(backend_options),
        security_limits: Some(limits(case)),
        ..Default::default()
    };
    Stock {
        _models: models,
        backend,
        config,
    }
}

async fn stock_read(
    stock: &Stock,
    reuse: &Arc<ProbeReads>,
    config: &OcrConfig,
    pixels: &Arc<Vec<u8>>,
) -> crate::Result<ExtractedDocument> {
    let identity = ReadIdentity {
        page: 1,
        backend: &stock.backend,
        configs: (config, config),
        whole_page_raster: false,
        correction_degrees: 0,
    };
    let engine = Arc::clone(&stock.backend);
    read(Some(reuse), identity, Arc::clone(pixels), None, |bytes| async move {
        engine.process_image_owned(bytes, config).await
    })
    .await
}

// Compare maps as JSON values while retaining the private OCR payload. ~keep
fn whole_read(document: &ExtractedDocument) -> serde_json::Value {
    serde_json::to_value((
        document,
        &document.formatted_content,
        internal_read(&document.ocr_internal_document),
        internal_read(&document.internal_document),
        &document.raw_ocr_source_frame,
    ))
    .unwrap()
}

fn internal_read(document: &Option<crate::types::internal::InternalDocument>) -> serde_json::Value {
    serde_json::to_value((
        document,
        document.as_ref().map(|document| {
            (
                document.uris_dropped,
                &document.recorded_formulas,
                document.ocr_text_only,
                document.append_ocr_text,
                document.escape_markdown,
                document.include_watermarks,
                &document.page_marker_format,
                document.table_anchors,
                format!("{:?}", document.ocr_coordinate_frame),
                document.ocr_text_claimed_by_tables,
            )
        }),
    ))
    .unwrap()
}

fn assert_source_prose(text: &str) {
    let lower = text.to_lowercase();
    for word in SOURCE_WORDS {
        assert!(lower.contains(word), "{word} missing from {text:?}");
    }
    assert!(
        !text
            .chars()
            .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c) || c == '\u{FFFD}'),
        "{text:?}"
    );
}

#[tokio::test]
async fn stock_raw_read_of_a_text_raster_matches_an_uncached_final_read() {
    let pixels = Arc::new(crate::pdf::render::render_pdf_page_to_png(ARABIC, 0, Some(150), None).unwrap());
    let stock = stock(
        NORMALIZATION_CASE,
        serde_json::json!({"source_dpi": 150.0, "use_cache": false}),
    );
    let final_config = stock.config.clone();
    let mut probe_config = final_config.clone();
    let mut thresholds = probe_config.effective_thresholds();
    thresholds.discard_suspected_ocr_noise = true;
    probe_config.quality_thresholds = Some(thresholds);

    let mut normalized = probe_config.clone();
    normalized
        .quality_thresholds
        .as_mut()
        .unwrap()
        .discard_suspected_ocr_noise = false;
    let mut expected = final_config.clone();
    expected.quality_thresholds = Some(final_config.effective_thresholds());
    assert_eq!(
        serde_json::to_value(&normalized).unwrap(),
        serde_json::to_value(&expected).unwrap(),
        "the probe config differs from the final one in the discard flag alone"
    );
    assert_eq!(
        format!("{:?}", probe_config.security_limits),
        format!("{:?}", final_config.security_limits)
    );

    let reuse = Arc::new(ProbeReads::new(SecurityLimits::default().max_content_size));
    let probe = stock_read(&stock, &reuse, &probe_config, &pixels).await.unwrap();
    assert_eq!(read_counter::reads(NORMALIZATION_CASE), 1, "the probe reads once");
    assert_source_prose(&probe.content);
    assert!(reuse.finish(&[1]), "the text read was retained");

    let reused = stock_read(&stock, &reuse, &final_config, &pixels).await.unwrap();
    assert_eq!(
        read_counter::reads(NORMALIZATION_CASE),
        1,
        "the final read is the probe read"
    );
    let independent = stock
        .backend
        .process_image_owned(Arc::clone(&pixels), &final_config)
        .await
        .unwrap();
    assert_eq!(read_counter::reads(NORMALIZATION_CASE), 2, "the control read runs");
    assert_source_prose(&independent.content);
    assert_eq!(whole_read(&reused), whole_read(&probe));
    assert_eq!(
        whole_read(&reused),
        whole_read(&independent),
        "a read under discard_suspected_ocr_noise = true is the read the final config makes"
    );
}

fn with_healthy_pages(source: &[u8]) -> Vec<u8> {
    use lopdf::content::{Content, Operation};
    use lopdf::{Object, Stream, dictionary};

    let mut document = lopdf::Document::load_mem(source).unwrap();
    let pages = document.get_pages();
    assert_eq!(pages.len(), 1);
    let original = *pages.values().next().unwrap();
    let parent = document
        .get_dictionary(original)
        .unwrap()
        .get(b"Parent")
        .unwrap()
        .as_reference()
        .unwrap();
    let font = document.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let mut kids = vec![Object::Reference(original)];
    for lines in [HARBOR, ORCHARD] {
        let mut operations = vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), Object::Integer(11)]),
            Operation::new("TL", vec![Object::Integer(14)]),
            Operation::new("Td", vec![Object::Integer(50), Object::Integer(730)]),
        ];
        for line in lines.iter().cycle().take(lines.len() * 4) {
            operations.push(Operation::new("Tj", vec![Object::string_literal(*line)]));
            operations.push(Operation::new("T*", vec![]));
        }
        operations.push(Operation::new("ET", vec![]));
        let stream = Stream::new(dictionary! {}, Content { operations }.encode().unwrap());
        let contents = document.add_object(stream);
        let page = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => parent,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
            "Contents" => contents,
        });
        kids.push(Object::Reference(page));
    }
    let tree = document.get_object_mut(parent).unwrap().as_dict_mut().unwrap();
    tree.set("Kids", kids);
    tree.set("Count", Object::Integer(3));
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

#[tokio::test]
async fn stock_public_mixed_route_reads_the_wrong_script_page_once() {
    let pdf = with_healthy_pages(ARABIC);
    let stock = stock(ROUTE_CASE, serde_json::json!({"use_cache": false}));
    let config = crate::ExtractionConfig {
        use_cache: false,
        ocr: Some(stock.config.clone()),
        pages: Some(crate::core::config::PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        result_format: crate::types::ResultFormat::ElementBased,
        security_limits: Some(limits(ROUTE_CASE)),
        ..Default::default()
    };
    let result = crate::core::extractor::extract_bytes(&pdf, "application/pdf", &config)
        .await
        .unwrap();
    assert_eq!(
        result.extraction_method,
        Some(crate::types::ExtractionMethod::Mixed),
        "{:?}",
        result.processing_warnings
    );
    let pages = result.pages.as_ref().expect("pages were requested");
    assert_eq!(pages.len(), 3);
    assert_source_prose(&pages[0].content);
    for (page, words) in [(&pages[1], ["mackerel", "seawall"]), (&pages[2], ["orchard", "cider"])] {
        for word in words {
            assert!(page.content.contains(word), "{word} missing from {:?}", page.content);
        }
    }
    assert_eq!(
        read_counter::reads(ROUTE_CASE),
        1,
        "the targeted route takes the probe's read of page 1: {:?}",
        result.processing_warnings
    );
    assert_eq!(
        read_counter::probe_reuses(ROUTE_CASE),
        1,
        "the final route must consume a retained probe read, not merely run OCR once"
    );
}
