use crate::core::config::{ImageExtractionConfig, OcrConfig};
use crate::extractors::security::SecurityLimits;
use crate::ocr::tesseract_backend::read_counter;
use crate::{ExtractInput, ExtractionConfig};
use sha2::{Digest, Sha256};

const GUIDE: &str = "pdf_scanned/memo_prose_scanned.pdf";
const MIME: &str = "application/pdf";

fn config(models: &std::path::Path, case: usize) -> ExtractionConfig {
    let limits = SecurityLimits {
        max_archive_size: case,
        ..Default::default()
    };
    ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        security_limits: Some(limits.clone()),
        #[cfg(feature = "layout-types")]
        layout: None,
        images: Some(ImageExtractionConfig {
            extract_images: false,
            inject_placeholders: false,
            run_ocr_on_images: false,
            target_dpi: 150,
            min_dpi: 150,
            max_dpi: 150,
            auto_adjust_dpi: false,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            backend: "tesseract".into(),
            tessdata_path: Some(models.into()),
            security_limits: Some(limits),
            auto_rotate: Some(false),
            backend_options: Some(serde_json::json!({"use_cache": false})),
            tesseract_config: Some(crate::TesseractConfig {
                psm: Some(6),
                enable_table_detection: false,
                preprocessing: Some(crate::types::ImagePreprocessingConfig {
                    target_dpi: 150,
                    deskew: false,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[tokio::test]
#[ignore = "requires the pinned public scanned-PDF fixture"]
async fn public_single_page_uses_owned_tesseract_and_matches_encoded_fallback() {
    let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents");
    let pdf = corpus.join(GUIDE);
    let bytes = std::fs::read(&pdf).expect("pinned public scan must be provisioned");
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "9f75e84ec7e322b71715a626f06df1ce1ebe5bd7d921e6b046807d18a981b484"
    );
    let models = tempfile::tempdir().unwrap();
    let eng = xberg_tesseract::bundled_eng_traineddata().unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(eng)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), eng).unwrap();
    let path = pdf.to_str().unwrap();

    for (route, key) in [(false, 0x3171_0720), (true, 0x3171_0730)] {
        let input = || {
            if route {
                ExtractInput::from_uri(path)
            } else {
                ExtractInput::from_bytes(bytes.clone(), MIME, None)
            }
        };
        read_counter::force_encoded(key);
        let stock = crate::extract(input(), &config(models.path(), key)).await.unwrap();
        let owned = crate::extract(input(), &config(models.path(), key + 1)).await.unwrap();
        assert!(stock.errors.is_empty() && owned.errors.is_empty());
        let [stock] = stock.results.as_slice() else {
            panic!("one stock document")
        };
        let [owned] = owned.results.as_slice() else {
            panic!("one owned document")
        };
        assert!(stock.content.split_whitespace().count() > 20);
        assert_eq!(read_counter::reads(key), 1, "stock recognized once");
        assert_eq!(read_counter::reads(key + 1), 1, "owned recognized once");
        assert_eq!(read_counter::owned_successes(key), 0, "forced PNG control");
        assert_eq!(
            read_counter::owned_successes(key + 1),
            1,
            "public route reached the decoded worker"
        );
        let project = |document: &crate::types::ExtractedDocument| {
            let mut doc = document.clone();
            doc.metadata.extraction_duration_ms = None;
            serde_json::to_value(doc).unwrap()
        };
        assert!(
            project(stock) == project(owned),
            "public decoded and encoded results differ"
        );
    }

    const UNSUPPORTED: usize = 0x3171_0740;
    let mut unsupported = config(models.path(), UNSUPPORTED);
    unsupported
        .ocr
        .as_mut()
        .unwrap()
        .tesseract_config
        .as_mut()
        .unwrap()
        .preprocessing
        .as_mut()
        .unwrap()
        .binarization_method = "sauvola".into();
    let result = crate::extract(ExtractInput::from_bytes(bytes, MIME, None), &unsupported)
        .await
        .unwrap();
    assert!(result.errors.is_empty());
    assert_eq!(result.results.len(), 1);
    assert!(result.results[0].content.split_whitespace().count() > 20);
    assert_eq!(read_counter::reads(UNSUPPORTED), 1);
    assert_eq!(
        read_counter::owned_successes(UNSUPPORTED),
        0,
        "unsupported policy stays on PNG"
    );
}
