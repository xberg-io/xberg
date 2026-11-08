#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "tokio-runtime",
    feature = "bundle-tessdata-eng",
    not(target_arch = "wasm32")
))]

use sha2::{Digest, Sha256};
use std::io::Write;
use xberg::{ExtractInput, ExtractedDocument, ExtractionConfig};

const NEUTRAL_RGB: &[u8] = include_bytes!("../../../fixtures/repros/tall-scan-strips/neutral-rgb-tall.pdf");
const ORACLE: &str = include_str!("../../../fixtures/repros/tall-scan-strips/expected.txt");

#[derive(Clone, Copy, Debug)]
enum PaintEncoding {
    Raw,
    Flate,
    Ascii85Flate,
}

fn ascii85(input: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::new();
    for chunk in input.chunks(4) {
        let mut padded = [0; 4];
        padded[..chunk.len()].copy_from_slice(chunk);
        let mut value = u32::from_be_bytes(padded);
        let mut digits = [b'!'; 5];
        for digit in digits.iter_mut().rev() {
            *digit += (value % 85) as u8;
            value /= 85;
        }
        encoded.extend_from_slice(&digits[..chunk.len() + 1]);
    }
    encoded.extend_from_slice(b"~>");
    encoded
}

fn wrap_paint(stream: &mut lopdf::Stream, encoding: PaintEncoding) {
    if matches!(encoding, PaintEncoding::Raw) {
        stream.set_content(stream.content.clone());
        return;
    }
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&stream.content).unwrap();
    let compressed = encoder.finish().unwrap();
    if matches!(encoding, PaintEncoding::Ascii85Flate) {
        stream.set_content(ascii85(&compressed));
        stream.dict.set(
            "Filter",
            vec![
                lopdf::Object::Name(b"ASCII85Decode".to_vec()),
                lopdf::Object::Name(b"FlateDecode".to_vec()),
            ],
        );
    } else {
        stream.set_content(compressed);
        stream.dict.set("Filter", "FlateDecode");
    }
}

fn page_paint(encoding: PaintEncoding, extra_paint: bool) -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(NEUTRAL_RGB).unwrap();
    let mut changed = 0;
    let mut expected_paint = Vec::new();
    for object in doc.objects.values_mut() {
        let Ok(stream) = object.as_stream_mut() else {
            continue;
        };
        if stream.dict.has(b"Subtype") {
            continue;
        }
        assert!(!stream.dict.has(b"Filter"));
        if extra_paint {
            stream.content.extend_from_slice(b"\n0 0 m 1 1 l S\n");
        }
        let expected = stream.content.clone();
        expected_paint = expected.clone();
        wrap_paint(stream, encoding);
        let decoded = if matches!(encoding, PaintEncoding::Raw) {
            stream.content.clone()
        } else {
            stream.decompressed_content().unwrap()
        };
        assert_eq!(decoded, expected, "lossless {encoding:?} page-paint wrapper");
        changed += 1;
    }
    assert_eq!(changed, 1, "change only the certified page-paint stream");
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    assert_serialized_paint(&bytes, &expected_paint, encoding);
    bytes
}

fn assert_serialized_paint(bytes: &[u8], expected: &[u8], encoding: PaintEncoding) {
    let reparsed = lopdf::Document::load_mem(bytes).unwrap();
    let contents = reparsed.get_page_contents(reparsed.get_pages()[&1]);
    assert_eq!(contents.len(), 1);
    let stream = reparsed.get_object(contents[0]).unwrap().as_stream().unwrap();
    assert_eq!(
        stream.dict.get(b"Length").unwrap().as_i64().unwrap(),
        stream.content.len() as i64
    );
    let decoded = if matches!(encoding, PaintEncoding::Raw) {
        stream.content.clone()
    } else {
        stream.decompressed_content().unwrap()
    };
    assert_eq!(
        decoded, expected,
        "serialized {encoding:?} artifact contains the intended paint"
    );
}

fn config() -> (tempfile::TempDir, ExtractionConfig) {
    let models = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), english).unwrap();
    let config = serde_json::from_value(serde_json::json!({
        "use_cache": false,
        "ocr": {"backend": "tesseract", "language": ["eng"], "tessdata_path": models.path()},
        "pages": {"extract_pages": true},
        "concurrency": {"max_threads": 1}
    }))
    .unwrap();
    (models, config)
}

fn extract(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let mut result = runtime
        .block_on(xberg::extract(
            ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
            config,
        ))
        .unwrap();
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(result.results.len(), 1);
    result.results.pop().unwrap()
}

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace()
        .map(|word| word.trim_matches(|character: char| !character.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect()
}

fn used_strips(document: &ExtractedDocument) -> bool {
    document
        .processing_warnings
        .iter()
        .any(|warning| warning.message.contains("source-image strips."))
}

#[test]
fn neutral_rgb_scan_recovers_the_same_words_after_lossless_page_paint_compression() {
    let (_models, config) = config();
    let expected = words(ORACLE);
    assert_eq!(expected.len(), 440);
    for encoding in [PaintEncoding::Raw, PaintEncoding::Flate, PaintEncoding::Ascii85Flate] {
        let result = extract(&page_paint(encoding, false), &config);
        assert!(result.metadata.ocr_used);
        assert!(used_strips(&result), "{:?}", result.processing_warnings);
        assert_eq!(words(&result.content), expected);
        let pages = result.pages.as_ref().unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(words(&pages[0].content), expected);
    }
}

#[test]
fn raw_and_filtered_extra_drawing_is_refused_as_uncertified() {
    let (_models, config) = config();
    for encoding in [PaintEncoding::Raw, PaintEncoding::Flate, PaintEncoding::Ascii85Flate] {
        let result = extract(&page_paint(encoding, true), &config);
        assert!(result.metadata.ocr_used);
        assert!(!used_strips(&result), "{encoding:?}: {:?}", result.processing_warnings);
        let reason = "source-image strips were not used: the page is not one scan image in a supported encoding and density within the memory limit, or a strip did not read as its source lines. The text may be incomplete.";
        assert!(
            result
                .processing_warnings
                .iter()
                .any(|warning| warning.source == "ocr" && warning.message.contains(reason)),
            "{encoding:?}: {:?}",
            result.processing_warnings
        );
    }
}

fn assert_large_text_recovery(input: &[u8], source_width: u64) {
    let (_models, config) = config();
    let oracle = include_str!("../../../fixtures/repros/tall-scan-strips/large-text-expected.txt");
    let expected = words(oracle);
    assert_eq!(oracle.lines().count(), 41);
    assert_eq!(expected.len(), 242);
    let result = extract(input, &config);
    assert!(used_strips(&result), "{:?}", result.processing_warnings);
    assert_eq!(words(&result.content), expected);
    let pages = result.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 1);
    assert_eq!(words(&pages[0].content), expected);
    let frame = &result.metadata.additional["ocr_source_strip_original_frame"];
    assert_eq!(frame["processed_width"], 154);
    assert_eq!(frame["processed_height"], 4096);
    let rendered = frame["rendered_width"].as_u64().unwrap();
    assert!(154 < rendered);
    assert!(frame["rendered_height"].as_u64().unwrap() > 4096);
    if source_width == 450 {
        assert_eq!(rendered, source_width, "only preprocessing shrinks the 150 DPI render");
    }
}

#[test]
fn tall_scan_shrunk_only_by_ocr_preprocessing_recovers_every_source_word() {
    assert_large_text_recovery(
        include_bytes!("../../../fixtures/repros/tall-scan-strips/large-text-150.pdf"),
        450,
    );
}

#[test]
fn same_physical_large_text_at_300_dpi_recovers_every_source_word() {
    assert_large_text_recovery(
        include_bytes!("../../../fixtures/repros/tall-scan-strips/large-text-300.pdf"),
        900,
    );
}
