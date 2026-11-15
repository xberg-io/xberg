#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng"
))]

use sha2::{Digest, Sha256};
use xberg::{ExtractInput, ExtractionConfig, OcrConfig};
#[derive(Clone, Default)]
struct TraceBuffer(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for TraceBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

async fn extract_arm(
    guide: &std::path::Path,
    config: &ExtractionConfig,
    active: &std::sync::Mutex<TraceBuffer>,
) -> (xberg::ExtractedDocument, String) {
    let arm = TraceBuffer::default();
    *active.lock().unwrap() = arm.clone();
    let result = xberg::extract(ExtractInput::from_uri(guide.to_str().unwrap()), config)
        .await
        .unwrap();
    assert!(result.errors.is_empty());
    let [document] = result.results.try_into().ok().unwrap();
    let trace = String::from_utf8(arm.0.lock().unwrap().clone()).unwrap();
    (document, trace)
}

fn assert_cache_arm(trace: &str, warm: bool) {
    let writes = trace.matches("Extraction cache write").count();
    let hits = trace.matches("Extraction cache hit").count();
    assert_eq!((writes, hits), if warm { (0, 1) } else { (1, 0) }, "{trace}");
    if warm {
        assert!(!trace.contains("optional region OCR backend call"));
        assert!(!trace.contains("hOCR region capture requested"));
    }
}

#[tokio::test]
#[ignore = "requires the pinned public guide"]
async fn public_original_multipage_recovery() {
    let path = std::env::var_os("XBERG_2502_GUIDE").unwrap();
    let guide = std::path::PathBuf::from(path);
    let bytes = std::fs::read(&guide).unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "04c76db01e20d7641e886b2403bea2df5757dfe9b4be7a283ee44d1861081a06"
    );
    let d = tempfile::tempdir().unwrap();
    let model = xberg_tesseract::bundled_eng_traineddata().unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(model)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(d.path().join("eng.traineddata"), model).unwrap();
    let mut config = ExtractionConfig {
        ocr: Some(OcrConfig {
            tessdata_path: Some(d.path().to_path_buf()),
            tesseract_config: Some(xberg::TesseractConfig {
                preprocessing: Some(xberg::ImagePreprocessingConfig {
                    deskew: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            backend_options: Some(serde_json::json!({})),
            ..Default::default()
        }),
        pages: Some(serde_json::from_value(serde_json::json!({"extract_pages": true})).unwrap()),
        ..Default::default()
    };
    let active = std::sync::Arc::new(std::sync::Mutex::new(TraceBuffer::default()));
    let writer = active.clone();
    tracing_subscriber::fmt()
        .with_env_filter(concat!(
            "xberg::extractors::pdf::ocr::tall_source::region_probe=trace,",
            "xberg::ocr::processor::execution=trace,xberg::core::extractor::file=debug",
        ))
        .with_ansi(false)
        .with_writer(move || writer.lock().unwrap().clone())
        .try_init()
        .unwrap();
    let (stock, stock_trace) = extract_arm(&guide, &config, &active).await;
    assert_cache_arm(&stock_trace, false);
    config.ocr.as_mut().unwrap().backend_options.as_mut().unwrap()["source_region_recovery"] = true.into();
    let (live, full) = extract_arm(&guide, &config, &active).await;
    assert_cache_arm(&full, false);
    let full_calls = full.matches("optional region OCR backend call").count();
    assert!((4..=16).contains(&full_calls), "{full_calls} document crop reads");
    assert!(full.contains("hOCR region capture requested"));

    let (warm, warm_trace) = extract_arm(&guide, &config, &active).await;
    assert_cache_arm(&warm_trace, true);
    assert_eq!(
        serde_json::to_value(&warm).unwrap(),
        serde_json::to_value(&live).unwrap()
    );

    config.ocr.as_mut().unwrap().backend_options.as_mut().unwrap()["source_region_max_calls"] = 2.into();
    let (low, low_trace) = extract_arm(&guide, &config, &active).await;
    assert_cache_arm(&low_trace, false);
    let low_calls = low_trace.matches("optional region OCR backend call").count();
    assert_eq!(low_calls, 2);
    assert!(low_trace.contains("optional region OCR document budget exhausted"));
    let (low_warm, low_warm_trace) = extract_arm(&guide, &config, &active).await;
    assert_cache_arm(&low_warm_trace, true);
    assert_eq!(
        serde_json::to_value(&low_warm).unwrap(),
        serde_json::to_value(&low).unwrap()
    );
    println!(
        "file cache fresh writes and warm hits verified; optional backend calls: cold {full_calls}, warm 0, low-budget cold {low_calls}, low-budget warm 0"
    );
    let expected = |text: &str, page| match page {
        8 => {
            assert_eq!(text.matches("POP WAcES:").count(), 1);
            text.replacen("POP WAcES:", "PDP IMAGES", 1)
        }
        13 => {
            let before = "\n\nwi\n\nthe blog";
            assert_eq!(text.matches(before).count(), 1);
            text.replacen(before, "\n\nView\n\nthe blog", 1)
        }
        _ => text.to_owned(),
    };
    assert_eq!(live.content, expected(&expected(&stock.content, 8), 13));
    let (old_pages, new_pages) = (stock.pages.as_ref().unwrap(), live.pages.as_ref().unwrap());
    assert_eq!((old_pages.len(), new_pages.len()), (13, 13));
    assert_eq!(stock.tables.len(), 10);
    for (before, after) in old_pages.iter().zip(new_pages) {
        assert_eq!(after.content, expected(&before.content, before.page_number));
        let mut expected_page = before.clone();
        expected_page.content.clone_from(&after.content);
        assert_eq!(
            serde_json::to_value(expected_page).unwrap(),
            serde_json::to_value(after).unwrap()
        );
    }
    assert_eq!(low.pages.as_ref().unwrap()[12].content, old_pages[12].content);
    let old = serde_json::to_value(&stock).unwrap();
    let new = serde_json::to_value(&live).unwrap();
    assert_eq!(old.get("tables"), new.get("tables"));
    assert_eq!(old.get("annotations"), new.get("annotations"));
}
