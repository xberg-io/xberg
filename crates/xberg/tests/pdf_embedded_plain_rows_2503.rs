#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng"
))]

use sha2::{Digest, Sha256};
use xberg::core::config::{ExtractInput, ExtractionConfig};

const PDF: &[u8] = include_bytes!("fixtures/ocr/embedded_ledger_2503.pdf");
const EXPECTED: [&str; 10] = [
    "The table below lists the yearly service figures.",
    "Item Count Budget Change",
    "Snow removal......... 68 594,898 (8,684)",
    "Field lights ............ 95 985,652 141.97",
    "Parks and trails ........ 45 901,761 70,139",
    "Water service ......... 26 61,656 (3,409)",
    "Street repair.......... 16 181,811 126.32",
    "Library hours .......... 28 801,525 54,396",
    "Fleet fuel............. 16 863,842 (5,273)",
    "Bridge paint........... 39 756,157 176.66",
];

#[tokio::test]
async fn plain_keeps_embedded_ledger_rows_at_document_and_page_level() {
    let models = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), english).unwrap();
    assert_ledger(models.path(), None, None, false).await;
    for psm in [None, Some(6)] {
        for elements in [None, Some(false), Some(true)] {
            assert_ledger(models.path(), psm, elements, true).await;
        }
    }
}

async fn assert_ledger(models: &std::path::Path, psm: Option<u8>, elements: Option<bool>, retain_image: bool) {
    let mut config = serde_json::json!({
        "use_cache": false,
        "output_format": "plain",
        "ocr": {"enabled": true, "backend": "tesseract", "language": ["eng"], "tessdata_path": models},
        "pages": {"extract_pages": true},
    });
    if retain_image {
        config["images"] = serde_json::json!({"extract_images": true});
    }
    if let Some(psm) = psm {
        config["ocr"]["tesseract_config"] = serde_json::json!({"psm": psm});
    }
    if let Some(enabled) = elements {
        config["ocr"]["element_config"] = serde_json::json!({
            "include_elements": enabled, "min_level": "word", "min_confidence": 0.5, "build_hierarchy": true
        });
    }
    let config: ExtractionConfig = serde_json::from_value(config).unwrap();
    let batch = xberg::extract(ExtractInput::from_bytes(PDF.to_vec(), "application/pdf", None), &config)
        .await
        .unwrap();
    assert!(batch.errors.is_empty());
    let [result] = batch.results.as_slice() else {
        panic!("expected one document");
    };
    if retain_image {
        assert_nested_ocr(result, psm, elements);
    }
    let [page] = result.pages.as_deref().expect("requested pages") else {
        panic!("expected one page");
    };
    let mut expected = EXPECTED;
    if psm == Some(6) {
        expected[5] = "Water service......... 26 61,656 (3,409)";
    }
    for (surface, text) in [("document", &result.content), ("page", &page.content)] {
        let lines: Vec<_> = text.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
        assert_eq!(
            lines, expected,
            "{surface}, PSM {psm:?}, elements {elements:?}, retain image {retain_image}"
        );
    }
}

fn assert_nested_ocr(result: &xberg::types::ExtractedDocument, psm: Option<u8>, elements: Option<bool>) {
    use xberg::types::{FormatMetadata, OcrElementLevel};

    let [image] = result.images.as_deref().expect("requested images") else {
        panic!("expected one embedded image");
    };
    let nested = image.ocr_result.as_deref().expect("embedded OCR result");
    let Some(FormatMetadata::Ocr(metadata)) = &nested.metadata.format else {
        panic!("expected nested OCR metadata");
    };
    assert_eq!(metadata.language, "eng");
    assert_eq!(metadata.psm, i32::from(psm.unwrap_or(3)));
    if elements != Some(true) {
        assert!(nested.ocr_elements.is_none(), "{elements:?}");
        return;
    }
    let public = nested.ocr_elements.as_deref().expect("requested nested OCR elements");
    assert!(public.iter().all(|element| element.confidence.recognition >= 0.5));
    let words: Vec<_> = public
        .iter()
        .filter(|element| element.level == OcrElementLevel::Word)
        .collect();
    assert!(words.iter().any(|word| word.text == "594,898"));
    assert!(words.iter().any(|word| {
        word.parent_id.as_ref().is_some_and(|id| {
            public.iter().any(|parent| {
                parent.level != OcrElementLevel::Word
                    && parent
                        .backend_metadata
                        .get("element_id")
                        .and_then(serde_json::Value::as_str)
                        == Some(id.as_str())
            })
        })
    }));
}
