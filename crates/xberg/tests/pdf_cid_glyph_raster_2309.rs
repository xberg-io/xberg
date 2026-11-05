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
        include_bytes!("../../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/unicode_nul_mapping.pdf"),
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
    let tessdata = type0_fixture_models();
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

fn type0_fallback_config(tessdata: &std::path::Path, output_format: OutputFormat) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        output_format,
        ocr: Some(OcrConfig {
            tessdata_path: Some(tessdata.to_path_buf()),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn type0_fixture_models() -> tempfile::TempDir {
    let tessdata = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English data");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(tessdata.path().join("eng.traineddata"), english).unwrap();
    tessdata
}

#[test]
fn single_page_automatic_ocr_keeps_native_text_when_type0_ink_is_missing() {
    let bytes = include_bytes!("../../xberg-native-pdf/tests/fixtures/cid_glyph_raster_2309/custom.pdf");
    assert_eq!(
        hex::encode(Sha256::digest(bytes)),
        "129cfb0836d0b9bd97150acd4d37b45a6bd000be9bc8069bec2166ca1fe0b60a"
    );
    let models = type0_fixture_models();
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let config = type0_fallback_config(models.path(), format);
        let extract = |config: &ExtractionConfig| {
            helpers::extract_bytes_document_blocking(bytes, "application/pdf", config).unwrap()
        };
        let native = extract(&ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        });
        let automatic = extract(&config);
        let forced = extract(&ExtractionConfig {
            force_ocr: true,
            ..config
        });
        assert_eq!(
            automatic.extraction_method,
            Some(xberg::types::ExtractionMethod::Native)
        );
        assert_eq!(automatic.content, native.content);
        assert_eq!(automatic.pages.as_ref().unwrap().len(), 1);
        assert_eq!(automatic.pages.unwrap()[0].content, native.pages.unwrap()[0].content);
        for word in ["budget", "building", "design"] {
            assert_eq!(
                words(&automatic.content).iter().filter(|token| *token == word).count(),
                5
            );
        }
        assert!(
            automatic
                .processing_warnings
                .iter()
                .any(|warning| warning.message.contains("Type0 glyph ink"))
        );
        assert!(!automatic.content.contains('\u{FFFD}'));
        assert_eq!(forced.extraction_method, Some(xberg::types::ExtractionMethod::Ocr));
        assert!(forced.metadata.ocr_used);
        assert!(!forced.content.trim().is_empty());
        assert_ne!(forced.content, automatic.content);
    }
}

#[test]
fn mixed_automatic_ocr_keeps_the_original_redp5110_page_after_type0_ink_loss() {
    let path = helpers::get_test_file_path("vendored/docling/pdf/redp5110_sampled.pdf");
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("required source {}: {error}", path.display()));
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "2f6bc92861997d1434c069040dc63447c3e415e8e9eadbd1a8129ba2327ddba5"
    );
    let models = type0_fixture_models();
    for format in [OutputFormat::Plain, OutputFormat::Markdown] {
        let config = type0_fallback_config(models.path(), format);
        let native = helpers::extract_bytes_document_blocking(
            &bytes,
            "application/pdf",
            &ExtractionConfig {
                disable_ocr: true,
                ..config.clone()
            },
        )
        .unwrap();
        let automatic = helpers::extract_bytes_document_blocking(&bytes, "application/pdf", &config).unwrap();
        let pages = automatic.pages.unwrap();
        assert_eq!(pages.len(), 18);
        assert_eq!(pages[2].page_number, 3);
        assert!(pages[2].content.split_whitespace().count() >= 185);
        assert_eq!(native.pages.unwrap()[2].content, pages[2].content);
        assert!(
            automatic
                .processing_warnings
                .iter()
                .any(|warning| { warning.message.contains("Page 3 retained native text") })
        );
        assert_eq!(
            automatic.extraction_method,
            Some(xberg::types::ExtractionMethod::Native)
        );
    }
}
