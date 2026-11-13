#![cfg(all(feature = "ocr", feature = "bundle-tessdata-eng"))]

use xberg::{ExtractInput, ExtractionConfig, OcrConfig, TesseractConfig};

async fn read(image: &[u8], tesseract_config: Option<TesseractConfig>) -> String {
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            tesseract_config,
            ..Default::default()
        }),
        ..Default::default()
    };
    let extracted = xberg::extract(ExtractInput::from_bytes(image.to_vec(), "image/png", None), &config)
        .await
        .unwrap();
    assert!(extracted.errors.is_empty(), "{:?}", extracted.errors);
    let [document] = extracted.results.as_slice() else {
        panic!("expected one extracted image");
    };
    document.content.clone()
}

#[tokio::test]
async fn standalone_calendar_retains_words_inside_colored_events() {
    // Public synthetic calendar from #2959, rendered with the issue's Arial font. ~keep
    let image = include_bytes!("fixtures/ocr_color_calendar_2959.png");
    let explicit_otsu = read(image, Some(TesseractConfig::default())).await;
    assert!(!explicit_otsu.to_ascii_lowercase().contains("harbor dredge"));
    let content = read(image, None).await.to_ascii_lowercase();
    let recovered = ["dredge", "quarry", "orchard", "outreach", "library", "roadmap"]
        .into_iter()
        .filter(|word| {
            content
                .split(|ch: char| !ch.is_ascii_alphabetic())
                .any(|token| token == *word)
        })
        .count();
    assert!(recovered >= 5, "recovered {recovered}/6 event words: {content}");
}

#[tokio::test]
async fn standalone_clean_table_keeps_its_ordinary_read() {
    let image = include_bytes!("fixtures/ocr_dot_leader_2503/clean_table.png");
    let ordinary = read(image, None).await;
    let explicit_otsu = read(image, Some(TesseractConfig::default())).await;
    assert!(ordinary.split_whitespace().count() >= 5, "{ordinary}");
    assert_eq!(ordinary, explicit_otsu);
}
