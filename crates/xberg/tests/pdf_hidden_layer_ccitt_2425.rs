#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

use sha2::{Digest, Sha256};
use xberg::{ExtractInput, ExtractionConfig, OcrConfig};

const SOURCE: &str = include_str!("fixtures/ocr/hidden_layer_ccitt_2425/source.txt");
const CCITT_PARTIAL: &[u8] = include_bytes!("fixtures/ocr/hidden_layer_ccitt_2425/ccitt-partial.pdf");
const CCITT_COMPLETE: &[u8] = include_bytes!("fixtures/ocr/hidden_layer_ccitt_2425/ccitt-complete.pdf");
const FLATE_PARTIAL: &[u8] = include_bytes!("fixtures/ocr/hidden_layer_ccitt_2425/flate8-partial.pdf");
const FLATE_COMPLETE: &[u8] = include_bytes!("fixtures/ocr/hidden_layer_ccitt_2425/flate8-complete.pdf");

async fn assert_read(pdf: &[u8], force_ocr: bool, expected_ocr: bool) {
    let config = ExtractionConfig {
        force_ocr,
        // The PDF route forwards this cache opt-out to Tesseract after stock routing.
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        pages: Some(xberg::core::config::PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let output = xberg::extract(ExtractInput::from_bytes(pdf.to_vec(), "application/pdf", None), &config)
        .await
        .expect("extract the synthetic scan");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let [document] = <[_; 1]>::try_from(output.results).unwrap();
    let expected = SOURCE.split_whitespace().collect::<Vec<_>>();
    assert_eq!(expected.len(), 81);
    let actual = document.content.split_whitespace().collect::<Vec<_>>();
    let [page] = document.pages.as_deref().expect("page output requested") else {
        panic!("one page");
    };
    assert_eq!(page.page_number, 1);
    assert_eq!(page.content.split_whitespace().collect::<Vec<_>>(), actual);
    assert_eq!(
        (document.metadata.ocr_used, actual.len()),
        (expected_ocr, expected.len()),
        "expected OCR={expected_ocr} with all source words, got {actual:?}"
    );
    assert_eq!(actual, expected);
    assert!(document.tables.is_empty());
}

fn assert_fixture(pdf: &[u8], sha: &str) {
    assert_eq!(hex::encode(Sha256::digest(pdf)), sha);
}

#[tokio::test]
async fn ccitt_partial_hidden_layer_recovers_all_raster_words_in_order() {
    assert_fixture(
        CCITT_PARTIAL,
        "6629d2d663fa7123042ce450d3be096133a80b43f74bc86cc62abb855501c8f9",
    );
    assert_read(CCITT_PARTIAL, false, true).await;
}

#[tokio::test]
async fn independently_decoded_raster_controls_preserve_words_and_native_coverage() {
    for (pdf, sha, force_ocr, expected_ocr) in [
        (
            FLATE_PARTIAL,
            "cc42e3932f1d65eac547c7279fe1ea2fbb1aeaee93e93cc06eb2bc3a41fdb13b",
            false,
            true,
        ),
        (
            CCITT_PARTIAL,
            "6629d2d663fa7123042ce450d3be096133a80b43f74bc86cc62abb855501c8f9",
            true,
            true,
        ),
        (
            CCITT_COMPLETE,
            "779f5a2908d7fdd839635009d93374a93f9ebd43d3f56deedc243cf425485f7a",
            false,
            false,
        ),
        (
            FLATE_COMPLETE,
            "66df812b7beed4db08e830d0f5540b5b806de0fc6382fdc49e0f2e09d57160a5",
            false,
            false,
        ),
    ] {
        assert_fixture(pdf, sha);
        assert_read(pdf, force_ocr, expected_ocr).await;
    }
}
