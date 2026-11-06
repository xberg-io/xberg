#![cfg(all(
    feature = "ocr",
    feature = "pdf",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]

use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig, ExtractionMethod, extract as extract_public};

const TALL_JPEG: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/jpeg-tall.pdf");
const TALL: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/strict-tall.pdf");
const BILEVEL: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/bilevel-tall.pdf");
const TALL_PREDICTOR: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/predictor-mixed.pdf");
const NEUTRAL_RGB: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/neutral-rgb-tall.pdf");
const MIXED: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/strict-mixed.pdf");
const SHORT: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/strict-short.pdf");
const ORACLE: &str = include_str!("../../../fixtures/repros/tall-scan-strips/expected.txt");
const STRIP_WARNING: &str = "source-image strips";

fn extraction_config() -> ExtractionConfig {
    serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "ocr": {"backend": "tesseract", "language": ["eng"]},
        "pages": {"extract_pages": true},
        "concurrency": {"max_threads": 1},
    }))
    .expect("one-thread Tesseract configuration")
}

fn extract(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("integration runtime");
    let mut result = runtime
        .block_on(extract_public(
            ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
            config,
        ))
        .expect("extract with the real Tesseract backend");
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(result.results.len(), 1);
    result.results.pop().expect("one extracted document")
}

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .map(|word| word.trim_matches(|character: char| !character.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect()
}

fn strip_warning(document: &ExtractedDocument) -> bool {
    document
        .processing_warnings
        .iter()
        .any(|warning| warning.message.contains(STRIP_WARNING))
}

fn assert_image_text(document: &ExtractedDocument, used_strips: bool) {
    let expected_words = words(ORACLE);
    assert_eq!(expected_words.len(), 440);
    assert_eq!(ORACLE.lines().count(), 40);
    assert_eq!(ORACLE.lines().collect::<std::collections::HashSet<_>>().len(), 40);
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Ocr));
    assert!(document.metadata.ocr_used);
    assert_eq!(strip_warning(document), used_strips);
    assert_eq!(words(&document.content), expected_words);
    let pages = document.pages.as_ref().expect("one original PDF page");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    assert_eq!(words(&pages[0].content), expected_words);
    if used_strips {
        let confidence = pages[0].ocr_confidence.as_ref().expect("one page confidence summary");
        assert_eq!(confidence.word_count, 440);
        assert_eq!(confidence.backend, "tesseract");
    }
}

#[test]
fn tall_scan_recovers_every_source_line_once_in_order() {
    let document = extract(TALL, &extraction_config());
    assert_image_text(&document, true);
    assert_eq!(
        document.metadata.additional.get("ocr_page_segmentation_modes"),
        Some(&serde_json::json!([{ "page_number": 1, "psm": 11 }]))
    );
}

#[test]
fn bilevel_tall_scan_recovers_every_source_line_once_in_order() {
    let document = extract(BILEVEL, &extraction_config());
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Ocr));
    assert!(strip_warning(&document));
    let pages = document.pages.as_ref().unwrap();
    let [page] = pages.as_slice() else {
        panic!("expected one page, got {}", pages.len());
    };
    assert_eq!(page.page_number, 1);
    assert_eq!(page.ocr_confidence.as_ref().unwrap().word_count, 440);
    let expected = words(ORACLE);
    for actual in [words(&document.content), words(&page.content)] {
        assert_eq!(actual.len(), expected.len());
        let (rows, remainder) = actual.as_chunks::<11>();
        assert!(remainder.is_empty());
        assert_eq!(rows.len(), 40);
        for (row, expected_row) in rows.iter().zip(expected.chunks(11)) {
            assert_eq!(&row[..10], &expected_row[..10]);
        }
        let exact_tags = rows
            .iter()
            .zip(expected.chunks(11))
            .filter(|(row, expected_row)| row[10] == expected_row[10])
            .count();
        assert!(exact_tags >= 36, "only {exact_tags} of 40 distinct row tags recognized");
    }
}

#[test]
fn in_cap_scan_keeps_original_ocr_route_and_text() {
    assert_image_text(&extract(SHORT, &extraction_config()), false);
}

#[test]
fn neutral_rgb_tall_scan_recovers_every_source_line_once_in_order() {
    assert_image_text(&extract(NEUTRAL_RGB, &extraction_config()), true);
}

#[test]
fn mixed_pdf_preserves_the_native_page_and_recovers_the_tall_scan() {
    let document = extract(MIXED, &extraction_config());
    let native = "The native page remains unchanged.\n".repeat(10);
    let pages = document.pages.as_ref().expect("two original PDF pages");
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Mixed));
    assert!(document.metadata.ocr_used);
    assert!(strip_warning(&document));
    assert_eq!(pages.len(), 2);
    assert_eq!((pages[0].page_number, pages[1].page_number), (1, 2));
    assert_eq!(words(&pages[0].content), words(&native));
    assert_eq!(words(&pages[1].content), words(ORACLE));
    assert_eq!(words(&document.content), words(&format!("{native}{ORACLE}")));
    let confidence = pages[1].ocr_confidence.as_ref().expect("mixed OCR page confidence");
    assert_eq!(confidence.word_count, 440);
    assert_eq!(confidence.backend, "tesseract");
}

#[test]
fn disabling_ocr_leaves_the_tall_scan_unrecognized() {
    let result = extract(
        TALL,
        &ExtractionConfig {
            disable_ocr: true,
            ..extraction_config()
        },
    );
    assert!(result.content.is_empty());
    assert!(!result.metadata.ocr_used);
    assert!(!strip_warning(&result));
}

#[test]
fn low_per_page_content_limit_keeps_existing_render_fallback() {
    let mut config = extraction_config();
    config
        .security_limits
        .get_or_insert_with(Default::default)
        .max_content_size = 32 * 1024 * 1024;
    let document = extract(TALL, &config);
    assert!(!strip_warning(&document));
    assert_eq!(document.pages.as_ref().expect("original page record").len(), 1);
}

#[test]
fn tall_gray_jpeg_recovers_every_source_word_after_render_budget_refusal() {
    assert_image_text(&extract(TALL_JPEG, &extraction_config()), true);
}

#[test]
fn tall_gray_jpeg_keeps_the_existing_refusal_for_forced_or_very_low_budget_ocr() {
    let forced = ExtractionConfig {
        force_ocr: true,
        ..extraction_config()
    };
    let mut low = extraction_config();
    low.security_limits
        .get_or_insert_with(Default::default)
        .max_content_size = 16 * 1024 * 1024;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    for config in [forced, low] {
        let input = ExtractInput::from_bytes(TALL_JPEG.to_vec(), "application/pdf", None);
        let error = runtime
            .block_on(extract_public(input, &config))
            .expect_err("existing render refusal");
        let xberg::XbergError::Parsing {
            source: Some(source), ..
        } = error
        else {
            panic!("expected typed Native render error, got {error:?}");
        };
        let native = source.downcast_ref::<xberg_native_pdf::Error>().or_else(|| {
            source
                .source()
                .and_then(|cause| cause.downcast_ref::<xberg_native_pdf::Error>())
        });
        assert!(matches!(native, Some(xberg_native_pdf::Error::Image(_))));
    }
}

fn assert_exact_tall_source_text(source: &[u8]) {
    let document = extract(source, &extraction_config());
    assert_image_text(&document, true);
    assert!(document.content.split_whitespace().eq(ORACLE.split_whitespace()));
    let pages = document.pages.as_ref().unwrap();
    assert!(pages[0].content.split_whitespace().eq(ORACLE.split_whitespace()));
}

#[test]
fn tall_png_predictor_recovers_all_source_words_in_order() {
    assert_exact_tall_source_text(TALL_PREDICTOR);
}

#[test]
fn tall_png_predictor_recovers_all_source_words_with_default_parameters() {
    let full = b"/DecodeParms << /Predictor 12 /Columns 1275 /Colors 1 /BitsPerComponent 8 >>";
    let short = b"/DecodeParms << /Predictor 12 /Columns 1275 >>";
    let positions = TALL_PREDICTOR
        .windows(full.len())
        .enumerate()
        .filter_map(|(index, window)| (window == full).then_some(index))
        .collect::<Vec<_>>();
    let [start] = positions.as_slice() else {
        panic!("the predictor fixture must have exactly one matching DecodeParms dictionary");
    };
    let mut abbreviated = TALL_PREDICTOR.to_vec();
    abbreviated[*start..*start + short.len()].copy_from_slice(short);
    abbreviated[*start + short.len()..*start + full.len()].fill(b' ');
    assert_exact_tall_source_text(&abbreviated);
}

#[test]
fn tall_lower_density_png_predictor_recovers_all_source_words_in_order() {
    assert_exact_tall_source_text(&lower_density_predictor());
}

fn lower_density_predictor() -> Vec<u8> {
    // Keep the exact encoded image and xref offsets; only the page and paint size change.
    let swaps: [(&[u8], &[u8]); 2] = [
        (b"/MediaBox [0 0 612 5760]", b"/MediaBox [0 0 765 7200]"),
        (b"q 612 0 0 5760 0 0 cm", b"q 765 0 0 7200 0 0 cm"),
    ];
    let mut lower_density = TALL_PREDICTOR.to_vec();
    for (old, new) in swaps {
        assert_eq!(old.len(), new.len());
        let mut positions = TALL_PREDICTOR
            .windows(old.len())
            .enumerate()
            .filter_map(|(index, window)| (window == old).then_some(index));
        let start = positions.next().expect("exact source geometry");
        assert!(positions.next().is_none());
        lower_density[start..start + new.len()].copy_from_slice(new);
    }
    lower_density
}

#[test]
fn tall_lower_density_plain_gray_flate_recovers_all_source_words_in_order() {
    const SOURCE_PIXELS: usize = 1275 * 12000;
    let mut pdf = lopdf::Document::load_mem(&lower_density_predictor()).expect("read exact synthetic source");
    let stream = pdf
        .get_object_mut((5, 0))
        .and_then(lopdf::Object::as_stream_mut)
        .expect("the source image stream");
    assert_eq!(stream.dict.get(b"Subtype").unwrap().as_name().unwrap(), b"Image");
    let samples = stream
        .get_plain_content_with_limit(SOURCE_PIXELS + 12000)
        .expect("decode the exact predictor source samples");
    assert_eq!(samples.len(), SOURCE_PIXELS);
    stream.set_plain_content(samples);
    stream.compress().expect("losslessly encode without prediction");
    assert_eq!(stream.dict.get(b"Filter").unwrap().as_name().unwrap(), b"FlateDecode");
    assert!(stream.dict.get(b"DecodeParms").is_err());
    let mut plain = Vec::new();
    pdf.save_to(&mut plain).expect("serialize the unpredicted source");
    assert_exact_tall_source_text(&plain);
}

#[test]
fn tall_inverted_indexed_scan_recovers_all_source_words_in_order() {
    let source = include_bytes!("../../../fixtures/repros/tall-scan-strips/indexed-inverted.pdf");
    assert_exact_tall_source_text(source);
}
