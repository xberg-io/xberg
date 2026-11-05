#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::core::config::{ExtractionConfig, OcrConfig, OutputFormat, PageConfig};
use xberg::types::TesseractConfig;

const SENTENCES: [&str; 4] = [
    "The committee met on Tuesday to review the plan for the new library.",
    "Members asked about the schedule and the budget for the parking area.",
    "The architect said that building work would start after the winter.",
    "A vote on the final design is planned for the next regular meeting.",
];
const FIXTURES: [(&str, &[u8]); 4] = [
    (
        "nul",
        include_bytes!("../../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/nul.pdf"),
    ),
    (
        "missing",
        include_bytes!("../../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/missing.pdf"),
    ),
    (
        "partial",
        include_bytes!("../../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/partial.pdf"),
    ),
    (
        "good",
        include_bytes!("../../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/good.pdf"),
    ),
];

fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
        .filter(|word| !word.is_empty())
        .collect()
}

fn assert_real_ocr_returns_the_source_words(fixtures: &[(&str, &[u8])]) {
    let tessdata = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English data");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(tessdata.path().join("eng.traineddata"), english).unwrap();
    let expected = words(&SENTENCES.repeat(5).join(" "));
    let mut jobs = 0;
    for &(name, bytes) in fixtures {
        for force_ocr in [false, true] {
            let config = ExtractionConfig {
                use_cache: false,
                force_ocr,
                output_format: OutputFormat::Plain,
                pages: Some(PageConfig {
                    extract_pages: true,
                    ..Default::default()
                }),
                ocr: Some(OcrConfig {
                    tessdata_path: Some(tessdata.path().to_path_buf()),
                    tesseract_config: Some(TesseractConfig {
                        use_cache: false,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let result = helpers::extract_bytes_document_blocking(bytes, "application/pdf", &config).unwrap();
            assert_eq!(words(&result.content), expected, "{name} force_ocr={force_ocr}");
            let pages = result.pages.as_ref().expect("requested page results");
            assert_eq!(pages.len(), 1);
            assert_eq!(words(&pages[0].content), expected, "page {name} force_ocr={force_ocr}");
            jobs += 1;
        }
    }
    assert_eq!(jobs, fixtures.len() * 2);
}

#[test]
fn should_read_cid_glyphs_with_lost_unicode_in_automatic_and_forced_ocr() {
    assert_eq!(FIXTURES[..3].len(), 3);
    assert_real_ocr_returns_the_source_words(&FIXTURES[..3]);
}

#[test]
fn should_preserve_good_tounicode_with_tiny_cmap_in_automatic_and_forced_ocr() {
    assert_real_ocr_returns_the_source_words(&FIXTURES[3..]);
}
