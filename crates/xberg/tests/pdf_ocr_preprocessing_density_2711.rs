#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]

#[path = "ocr_pixel_phase_2711/fixture.rs"]
mod fixture;

use sha2::{Digest, Sha256};
use xberg::types::{ExtractionMethod, ImagePreprocessingConfig};
use xberg::{
    ExtractInput, ExtractedDocument, ExtractionConfig, ImageExtractionConfig, OcrConfig, PageConfig, TesseractConfig,
};

fn expected_words() -> Vec<String> {
    words(&fixture::LINES.join(" "))
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .collect()
}

async fn read(
    bytes: Vec<u8>,
    images: Option<ImageExtractionConfig>,
    preprocessing: Option<ImagePreprocessingConfig>,
) -> ExtractedDocument {
    let models = tempfile::tempdir().unwrap();
    let model = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    assert_eq!(
        hex::encode(Sha256::digest(model)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), model).unwrap();
    let config = ExtractionConfig {
        force_ocr: true,
        use_cache: false,
        images,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            tessdata_path: Some(models.path().to_path_buf()),
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                preprocessing,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let native = xberg_native_pdf::PdfDocument::from_bytes(bytes.clone()).unwrap();
    assert_eq!(words(&native.extract_text(0).unwrap()), expected_words());
    let output = xberg::extract(ExtractInput::from_bytes(bytes, "application/pdf", None), &config)
        .await
        .unwrap();
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let [document]: [ExtractedDocument; 1] = output.results.try_into().expect("one document");
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Ocr));
    assert_eq!(document.pages.as_ref().unwrap().len(), 1);
    document
}

fn original_dimensions(document: &ExtractedDocument) -> (usize, usize) {
    let metadata = document.pages.as_ref().unwrap()[0]
        .image_preprocessing
        .as_ref()
        .expect("render provenance");
    (metadata.original_dimensions.width, metadata.original_dimensions.height)
}

#[tokio::test]
async fn explicit_preprocessing_density_preserves_source_words_before_resampling() {
    let bytes = fixture::localized_pdf("gap-left");
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "b260b82cb305f91c23ba5575bdaf4bfb3943324b82389467773162fdc15b0937"
    );
    let document = read(bytes, None, Some(ImagePreprocessingConfig::default())).await;
    println!(
        "render={:?} words={}",
        original_dimensions(&document),
        words(&document.content).len()
    );
    assert_eq!(
        words(&document.content),
        expected_words(),
        "render original detail instead of interpolating a lost word boundary"
    );
    assert_eq!(original_dimensions(&document), (1800, 900));
}

#[tokio::test]
async fn explicit_image_density_keeps_precedence_over_preprocessing() {
    let images = ImageExtractionConfig {
        target_dpi: 150,
        min_dpi: 150,
        max_dpi: 150,
        auto_adjust_dpi: false,
        ..Default::default()
    };
    let document = read(
        fixture::localized_pdf("gap-left"),
        Some(images),
        Some(ImagePreprocessingConfig::default()),
    )
    .await;
    assert_eq!(original_dimensions(&document), (900, 450));
}

#[tokio::test]
async fn implicit_preprocessing_keeps_the_existing_render_density() {
    let document = read(fixture::localized_pdf("gap-left"), None, None).await;
    assert_eq!(original_dimensions(&document), (900, 450));
}

#[tokio::test]
async fn promoted_density_preserves_origin_and_glyph_phase_controls() {
    for bytes in [
        fixture::pdf(7, 0),
        fixture::pdf(7, 1),
        fixture::localized_pdf("I-right"),
    ] {
        let document = read(bytes, None, Some(ImagePreprocessingConfig::default())).await;
        assert_eq!(words(&document.content), expected_words());
        assert_eq!(original_dimensions(&document), (1800, 900));
    }
}
