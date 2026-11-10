#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

use sha2::{Digest, Sha256};
use xberg::{ExtractInput, ExtractionConfig, OcrConfig};

const SOURCE: &str = include_str!("fixtures/ocr/hidden_layer_ccitt_2425/source.txt");
const CCITT_PARTIAL: &[u8] = include_bytes!("fixtures/ocr/hidden_layer_ccitt_2425/ccitt-partial.pdf");
const CCITT_COMPLETE: &[u8] = include_bytes!("fixtures/ocr/hidden_layer_ccitt_2425/ccitt-complete.pdf");
const FLATE_PARTIAL: &[u8] = include_bytes!("fixtures/ocr/hidden_layer_ccitt_2425/flate8-partial.pdf");
const FLATE_COMPLETE: &[u8] = include_bytes!("fixtures/ocr/hidden_layer_ccitt_2425/flate8-complete.pdf");

fn config(force_ocr: bool) -> ExtractionConfig {
    ExtractionConfig {
        force_ocr,
        // The PDF route forwards this cache opt-out to Tesseract after stock routing.
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        pages: Some(xberg::core::config::PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn read(pdf: &[u8], config: &ExtractionConfig) -> xberg::ExtractedDocument {
    let output = xberg::extract(ExtractInput::from_bytes(pdf.to_vec(), "application/pdf", None), config)
        .await
        .expect("extract the synthetic scan");
    assert!(output.errors.is_empty(), "{:?}", output.errors);
    let [document] = <[_; 1]>::try_from(output.results).unwrap();
    document
}

async fn assert_read(pdf: &[u8], force_ocr: bool, expected_ocr: bool) {
    assert_read_with_config(pdf, &config(force_ocr), expected_ocr).await;
}

async fn assert_read_with_config(pdf: &[u8], config: &ExtractionConfig, expected_ocr: bool) {
    let document = read(pdf, config).await;
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
async fn ccitt_hidden_layer_in_separate_content_stream_preserves_exact_words() {
    let partial = include_bytes!("fixtures/ocr/hidden_layer_ccitt_2425/ccitt-partial-split.pdf");
    let complete = include_bytes!("fixtures/ocr/hidden_layer_ccitt_2425/ccitt-complete-split.pdf");
    assert_fixture(
        partial,
        "f15c24652fe73b2f430254ce56d5702e0dfca2e3f78f6b94b0999dfa9b5d2d6d",
    );
    assert_fixture(
        complete,
        "d3f95ed8eec386a4f6c87025db43fa356d7da934f448a1c7cccdb4d543090402",
    );
    assert_read(complete, false, false).await;
    assert_read(partial, true, true).await;
    assert_read(partial, false, true).await;
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

#[tokio::test]
async fn tight_ccitt_work_limit_keeps_the_partial_native_text() {
    let mut config = config(false);
    config.security_limits = Some(xberg::extractors::security::SecurityLimits {
        max_iterations: 500_000,
        ..Default::default()
    });
    let ccitt = read(CCITT_PARTIAL, &config).await;
    let expected = SOURCE
        .lines()
        .enumerate()
        .filter(|(index, _)| [0, 1, 2, 3, 10].contains(index))
        .flat_map(|(_, line)| line.split_whitespace())
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 29);
    assert!(!ccitt.metadata.ocr_used);
    assert_eq!(ccitt.content.split_whitespace().collect::<Vec<_>>(), expected);
    assert_eq!(
        ccitt.pages.unwrap()[0].content.split_whitespace().collect::<Vec<_>>(),
        expected
    );

    let flate = read(FLATE_PARTIAL, &config).await;
    assert!(flate.metadata.ocr_used);
    assert_eq!(
        flate.content.split_whitespace().collect::<Vec<_>>(),
        SOURCE.split_whitespace().collect::<Vec<_>>()
    );
}

#[cfg(feature = "bundle-tessdata-eng")]
fn encode_up_rows(pixels: &[u8], width: usize) -> Vec<u8> {
    use std::io::Write;

    let mut tagged = Vec::with_capacity(pixels.len() + pixels.len() / width);
    for (y, row) in pixels.chunks_exact(width).enumerate() {
        tagged.push(2);
        tagged.extend(
            row.iter()
                .enumerate()
                .map(|(x, sample)| sample.wrapping_sub(if y == 0 { 0 } else { pixels[(y - 1) * width + x] })),
        );
    }
    let params = xberg_native_pdf::decoders::DecodeParams {
        predictor: 12,
        columns: width,
        colors: 1,
        bits_per_component: 8,
    };
    assert_eq!(
        xberg_native_pdf::decoders::decode_predictor(&tagged, &params).unwrap(),
        pixels
    );
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&tagged).unwrap();
    encoder.finish().unwrap()
}

#[cfg(feature = "bundle-tessdata-eng")]
fn predictor_image(pdf: &[u8], singleton: bool) -> Vec<u8> {
    use lopdf::{Object, dictionary};

    let mut document = lopdf::Document::load_mem(pdf).unwrap();
    let mut changed = 0;
    for image in document
        .objects
        .values_mut()
        .filter_map(|object| object.as_stream_mut().ok())
    {
        if image.dict.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"Image") {
            continue;
        }
        assert_eq!(image.dict.get(b"Filter").unwrap().as_name().unwrap(), b"FlateDecode");
        let width = usize::try_from(image.dict.get(b"Width").unwrap().as_i64().unwrap()).unwrap();
        let height = usize::try_from(image.dict.get(b"Height").unwrap().as_i64().unwrap()).unwrap();
        let pixels = image.decompressed_content().unwrap();
        assert_eq!(pixels.len(), width * height);
        image.set_content(encode_up_rows(&pixels, width));
        image.dict.set(
            "DecodeParms",
            dictionary! {"Predictor" => 12, "Columns" => width as i64,
            "Colors" => 1, "BitsPerComponent" => 8},
        );
        if singleton {
            image.dict.set("Filter", vec![Object::Name(b"FlateDecode".to_vec())]);
        }
        changed += 1;
    }
    assert_eq!(changed, 1);
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).unwrap();
    bytes
}

#[cfg(feature = "bundle-tessdata-eng")]
#[tokio::test]
async fn singleton_flate_predictor_recovers_partial_layer_and_keeps_complete_layer_native() {
    let tessdata = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("compiled English fixture, no download");
    std::fs::write(tessdata.path().join("eng.traineddata"), english).unwrap();
    let mut config = config(false);
    config.ocr.as_mut().unwrap().tessdata_path = Some(tessdata.path().to_path_buf());
    assert_fixture(
        FLATE_PARTIAL,
        "cc42e3932f1d65eac547c7279fe1ea2fbb1aeaee93e93cc06eb2bc3a41fdb13b",
    );
    assert_fixture(
        FLATE_COMPLETE,
        "66df812b7beed4db08e830d0f5540b5b806de0fc6382fdc49e0f2e09d57160a5",
    );
    for singleton in [false, true] {
        assert_read_with_config(&predictor_image(FLATE_PARTIAL, singleton), &config, true).await;
        assert_read_with_config(&predictor_image(FLATE_COMPLETE, singleton), &config, false).await;
    }
}
