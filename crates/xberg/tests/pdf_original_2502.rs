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

#[tokio::test]
#[ignore = "requires the pinned public guide"]
async fn public_original_multipage_recovery() {
    let path = std::env::var_os("XBERG_2502_GUIDE").unwrap();
    let bytes = std::fs::read(path).unwrap();
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
        use_cache: false,
        ocr: Some(OcrConfig {
            tessdata_path: Some(d.path().to_path_buf()),
            tesseract_config: Some(xberg::TesseractConfig {
                preprocessing: Some(xberg::ImagePreprocessingConfig {
                    deskew: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            backend_options: Some(serde_json::json!({"use_cache": false})),
            ..Default::default()
        }),
        pages: Some(serde_json::from_value(serde_json::json!({"extract_pages": true})).unwrap()),
        ..Default::default()
    };
    let traces = TraceBuffer::default();
    let writer = traces.clone();
    tracing_subscriber::fmt()
        .with_env_filter("xberg::extractors::pdf::ocr::tall_source::region_probe=trace")
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .try_init()
        .unwrap();
    let mut results = Vec::new();
    for enabled in [false, true] {
        config.ocr.as_mut().unwrap().backend_options.as_mut().unwrap()["source_region_recovery"] = enabled.into();
        let output = xberg::extract(
            ExtractInput::from_bytes(bytes.clone(), "application/pdf", None),
            &config,
        )
        .await
        .unwrap();
        assert!(output.errors.is_empty());
        let [document] = output.results.try_into().ok().unwrap();
        results.push(document);
    }
    let full = String::from_utf8(traces.0.lock().unwrap().clone()).unwrap();
    let full_calls = full.matches("optional region OCR backend call").count();
    assert!((4..=16).contains(&full_calls), "{full_calls} document crop reads");
    traces.0.lock().unwrap().clear();
    config.ocr.as_mut().unwrap().backend_options.as_mut().unwrap()["source_region_max_calls"] = 2.into();
    let low_budget = xberg::extract(ExtractInput::from_bytes(bytes, "application/pdf", None), &config)
        .await
        .unwrap();
    assert!(low_budget.errors.is_empty());
    let low = String::from_utf8(traces.0.lock().unwrap().clone()).unwrap();
    let low_calls = low.matches("optional region OCR backend call").count();
    assert_eq!(low_calls, 2);
    assert!(low.contains("optional region OCR document budget exhausted"));
    println!("actual optional backend calls: default {full_calls}, low budget {low_calls}");
    let [stock, live]: [xberg::ExtractedDocument; 2] = results.try_into().ok().unwrap();
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
    let low = &low_budget.results[0];
    assert_eq!(low.pages.as_ref().unwrap()[12].content, old_pages[12].content);
    let old = serde_json::to_value(&stock).unwrap();
    let new = serde_json::to_value(&live).unwrap();
    assert_eq!(old.get("tables"), new.get("tables"));
    assert_eq!(old.get("annotations"), new.get("annotations"));
}
