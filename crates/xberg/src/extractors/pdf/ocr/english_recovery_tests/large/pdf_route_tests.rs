#[tokio::test]
#[ignore = "large model-backed OCR; covered by rust:test:jpeg-recovery"]
async fn large_english_pdf() {
    super::isolated("pdf", concat!(module_path!(), "::large_english_pdf")).await;
}
