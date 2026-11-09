#![cfg(all(
    feature = "pdf",
    feature = "ocr",
    feature = "bundle-tessdata-eng",
    feature = "tokio-runtime"
))]

mod helpers;

use sha2::{Digest, Sha256};
use xberg::{ExtractedDocument, ExtractionConfig, OcrConfig, PageConfig};
use xberg_native_pdf::{PdfDocument, rendering};

const MATCHING: &[u8] = include_bytes!("fixtures/pdf_khmer_number_2408/matching.pdf");
const MISLEADING: &[u8] = include_bytes!("fixtures/pdf_khmer_number_2408/misleading.pdf");
const PROSE: [&str; 6] = [
    "The office keeps the current monthly service records.",
    "Readers can compare each line with its source record.",
    "The committee will meet on Monday to discuss the schedule",
    "Please retain the signed copy for your own records.",
    "The annual report is available at the information desk.",
    "The public document includes the relevant details.",
];

fn config() -> (tempfile::TempDir, ExtractionConfig) {
    let models = tempfile::tempdir().unwrap();
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English");
    assert_eq!(
        hex::encode(Sha256::digest(english)),
        "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
    );
    std::fs::write(models.path().join("eng.traineddata"), english).unwrap();
    let config = ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig {
            tessdata_path: Some(models.path().to_owned()),
            ..Default::default()
        }),
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    (models, config)
}

fn extract(input: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let result = helpers::extract_bytes_document_blocking(input, "application/pdf", config).unwrap();
    assert_eq!(result.pages.as_ref().unwrap().len(), 1);
    result
}

fn rendered(input: &[u8]) -> Vec<u8> {
    let doc = PdfDocument::from_bytes(input.to_vec()).unwrap();
    rendering::render_page(&doc, 0, &rendering::RenderOptions::with_dpi(150))
        .unwrap()
        .data
}

fn assert_ocr_retains_prose(result: &ExtractedDocument) {
    assert!(result.metadata.ocr_used);
    for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
        assert!(PROSE.iter().all(|line| text.contains(line)), "{text}");
    }
}

fn wrong_unicode() -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(MATCHING).unwrap();
    let mut changed = 0;
    for object in doc.objects.values_mut() {
        let Ok(stream) = object.as_stream_mut() else {
            continue;
        };
        let Ok(bytes) = stream.decompressed_content() else {
            continue;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        if text.contains("<02> <17E0>") {
            assert_eq!(text.matches("<02> <17E0>").count(), 1);
            stream.set_plain_content(text.replace("<02> <17E0>", "<02> <17E1>").into_bytes());
            changed += 1;
        }
    }
    assert_eq!(changed, 1, "mutate exactly one ToUnicode mapping");
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

fn revealed(input: &[u8]) -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(input).unwrap();
    let mut changed = 0;
    for object in doc.objects.values_mut() {
        let Ok(stream) = object.as_stream_mut() else {
            continue;
        };
        let Ok(bytes) = stream.decompressed_content() else {
            continue;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        if text.contains("3 Tr") {
            assert_eq!(text.matches("3 Tr").count(), 1);
            stream.set_plain_content(text.replace("3 Tr", "0 Tr").into_bytes());
            changed += 1;
        }
    }
    assert_eq!(changed, 1, "reveal exactly one hidden text show");
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    rendered(&bytes)
}

#[test]
fn same_scan_has_different_hidden_digits() {
    assert_eq!(
        hex::encode(Sha256::digest(MATCHING)),
        "9a623dd9a4cf726111c470d869ec32b2b5ec1b19f660bfe5caca62e7582ce239"
    );
    assert_eq!(
        hex::encode(Sha256::digest(MISLEADING)),
        "440539ba7870a7bcf42fbfd00fb4eceab296745ca971c9327465c55cf7e0d58d"
    );
    assert!(rendered(MATCHING) == rendered(MISLEADING), "same painted raster");
    let (_models, config) = config();
    let native = ExtractionConfig {
        disable_ocr: true,
        ..config
    };
    assert_eq!(extract(MATCHING, &native).content.trim(), "ប០ប");
    assert_eq!(extract(MISLEADING, &native).content.trim(), "ប១ប");
}

#[test]
fn refuses_a_hidden_digit_not_painted_by_the_scan() {
    let (_models, config) = config();
    let result = extract(MISLEADING, &config);
    assert_ocr_retains_prose(&result);
    for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
        assert!(!text.contains('១'), "the scan does not paint the claimed digit: {text}");
    }
}

#[test]
fn refuses_wrong_unicode_even_when_font_glyph_and_scan_are_unchanged() {
    let bad = wrong_unicode();
    assert!(rendered(MATCHING) == rendered(&bad), "same painted raster");
    assert!(revealed(MATCHING) == revealed(&bad), "same revealed glyph raster");
    assert!(
        rendered(MATCHING) != revealed(MATCHING),
        "revealing the glyphs changes ink"
    );
    let (_models, config) = config();
    let native = ExtractionConfig {
        disable_ocr: true,
        ..config.clone()
    };
    assert_eq!(extract(&bad, &native).content.trim(), "ប១ប");
    let result = extract(&bad, &config);
    assert_ocr_retains_prose(&result);
    for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
        assert!(
            !text.contains('១'),
            "ToUnicode alone cannot prove the source digit: {text}"
        );
    }
}

#[test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn recovers_the_painted_zero_once_at_its_source_position() {
    let (_models, config) = provisioned_config();
    let result = extract(MATCHING, &config);
    assert_ocr_retains_prose(&result);
    for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
        assert_eq!(text.matches('០').count(), 1, "one painted source occurrence: {text}");
        assert!(text.find('០').unwrap() < text.find(PROSE[0]).unwrap(), "{text}");
    }
}

fn provisioned_config() -> (tempfile::TempDir, ExtractionConfig) {
    let (models, config) = config();
    provision_khmer(models.path());
    (models, config)
}

fn provision_khmer(directory: &std::path::Path) {
    let model =
        std::fs::read(std::env::var_os("XBERG_2408_KHM_MODEL").expect("provide the pinned Khmer model")).unwrap();
    assert_eq!(model.len(), 1_446_926);
    assert_eq!(
        hex::encode(Sha256::digest(&model)),
        "47f110575341b322052f3becbefee61a3ecf1ef549352b5d4d33d28afe30d099"
    );
    std::fs::write(directory.join("khm.traineddata"), model).unwrap();
}

#[test]
fn english_only_keeps_the_unverified_ocr_read() {
    let (models, config) = config();
    assert!(!models.path().join("khm.traineddata").exists());
    let result = extract(MATCHING, &config);
    assert_ocr_retains_prose(&result);
    for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
        assert_eq!(
            text.matches('០').count(),
            0,
            "English cannot independently verify Khmer zero"
        );
    }
}

#[test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn provisioned_recognition_still_refuses_false_native_digits() {
    let (_models, config) = provisioned_config();
    for input in [MISLEADING.to_vec(), wrong_unicode()] {
        let result = extract(&input, &config);
        assert_ocr_retains_prose(&result);
        for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
            assert_eq!(text.matches('១').count(), 0, "untrusted Unicode must not win: {text}");
            assert_eq!(
                text.matches('០').count(),
                0,
                "do not invent numbers absent from the layer: {text}"
            );
        }
    }
}

fn moved_number(move_scan: bool) -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(MATCHING).unwrap();
    let mut changed = 0;
    for object in doc.objects.values_mut() {
        let Ok(stream) = object.as_stream_mut() else { continue };
        let Ok(data) = stream.decompressed_content() else {
            continue;
        };
        if let Ok(text) = String::from_utf8(data)
            && text.contains("100 650 Tm")
        {
            assert_eq!(text.matches("100 650 Tm").count(), 1);
            stream.set_plain_content(text.replace("100 650 Tm", "100 350 Tm").into_bytes());
            changed += 1;
        }
    }
    assert_eq!(changed, 1, "move one hidden text show");
    if move_scan {
        move_scan_word(&mut doc);
    }
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

fn move_scan_word(doc: &mut lopdf::Document) {
    let mut raster = image::load_from_memory(&rendered(MATCHING)).unwrap().into_rgb8();
    let word = image::imageops::crop_imm(&raster, 200, 220, 135, 100).to_image();
    for y in 220..320 {
        for x in 200..335 {
            raster.put_pixel(x, y, image::Rgb([255; 3]));
        }
    }
    image::imageops::replace(&mut raster, &word, 200, 845);
    replace_scan(doc, raster);
}

fn replace_scan(doc: &mut lopdf::Document, raster: image::RgbImage) {
    let mut changed = 0;
    for object in doc.objects.values_mut() {
        let Ok(stream) = object.as_stream_mut() else { continue };
        if stream.dict.get(b"Subtype").ok().and_then(|value| value.as_name().ok()) == Some(b"Image") {
            assert_eq!(stream.dict.get(b"Width").unwrap().as_i64().unwrap(), 1275);
            assert_eq!(stream.dict.get(b"Height").unwrap().as_i64().unwrap(), 1650);
            stream.set_plain_content(raster.clone().into_raw());
            changed += 1;
        }
    }
    assert_eq!(changed, 1, "move ink in the single scan");
}

#[test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn recovery_keeps_a_bottom_number_after_the_prose_in_plain_and_markdown() {
    let (_models, config) = provisioned_config();
    let moved = moved_number(true);
    for format in [xberg::OutputFormat::Plain, xberg::OutputFormat::Markdown] {
        let config = ExtractionConfig {
            output_format: format,
            include_document_structure: true,
            ..config.clone()
        };
        let result = extract(&moved, &config);
        assert_ocr_retains_prose(&result);
        for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
            assert_eq!(text.matches('០').count(), 1, "one source occurrence: {text}");
            assert!(
                text.find(PROSE[5]).unwrap() < text.find('០').unwrap(),
                "recover in position: {text}"
            );
        }
        assert!(
            serde_json::to_string(result.document.as_ref().unwrap())
                .unwrap()
                .contains('០')
        );
    }
}

#[test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn a_number_in_a_blank_source_crop_is_not_recovered() {
    let (_models, config) = provisioned_config();
    let shifted = moved_number(false);
    let result = extract(&shifted, &config);
    assert_ocr_retains_prose(&result);
    for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
        assert_eq!(
            text.matches('០').count(),
            0,
            "no painted number at the claimed position: {text}"
        );
    }
}

#[test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn installing_a_model_does_not_reuse_an_english_only_cached_result() {
    let (models, mut config) = config();
    config.use_cache = true;
    config.cache_namespace = Some(format!(
        "native-number-{}",
        models.path().file_name().unwrap().to_string_lossy()
    ));
    let before = extract(MATCHING, &config);
    assert_ocr_retains_prose(&before);
    assert_eq!(before.content.matches('០').count(), 0);
    provision_khmer(models.path());
    let after = extract(MATCHING, &config);
    assert_ocr_retains_prose(&after);
    assert_eq!(
        after.content.matches('០').count(),
        1,
        "newly available independent evidence must take effect"
    );
}

#[test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn forced_english_ocr_does_not_promote_native_numbers() {
    let (_models, mut config) = provisioned_config();
    config.force_ocr = true;
    let result = extract(MATCHING, &config);
    assert_ocr_retains_prose(&result);
    for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
        assert_eq!(text.matches('០').count(), 0, "forced OCR retains its own reading");
    }
}

fn overlapping_claims() -> Vec<u8> {
    let mut doc = lopdf::Document::load_mem(MATCHING).unwrap();
    let mut changed = 0;
    for object in doc.objects.values_mut() {
        let Ok(stream) = object.as_stream_mut() else { continue };
        let Ok(data) = stream.decompressed_content() else {
            continue;
        };
        if let Ok(text) = String::from_utf8(data)
            && text.contains("100 650 Tm")
        {
            assert_eq!(text.matches("100 650 Tm").count(), 1);
            let shifted = text.replace("100 650 Tm", "100 650.5 Tm");
            stream.set_plain_content(format!("{text}\n{shifted}").into_bytes());
            changed += 1;
        }
    }
    assert_eq!(changed, 1, "duplicate one invisible claim with a shifted baseline");
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    bytes
}

#[test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn overlapping_native_claims_cannot_multiply_one_painted_number() {
    let input = overlapping_claims();
    assert!(rendered(MATCHING) == rendered(&input), "same single painted number");
    let native = PdfDocument::from_bytes(input.clone()).unwrap();
    assert_eq!(
        native
            .extract_chars(0)
            .unwrap()
            .iter()
            .filter(|item| item.char == '០')
            .count(),
        2,
        "two native claims survive character extraction"
    );
    let (_models, config) = provisioned_config();
    let result = extract(&input, &config);
    assert_ocr_retains_prose(&result);
    for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
        assert_eq!(
            text.matches('០').count(),
            0,
            "ambiguous claims must not promote: {text}"
        );
    }
}

#[test]
#[ignore = "Run mise run rust:test:khmer-number-recovery with the pinned Khmer model"]
fn deskewed_primary_does_not_promote_a_number_in_the_original_source_frame() {
    let source = image::load_from_memory(&rendered(MATCHING)).unwrap().into_rgb8();
    let mut skewed = image::RgbImage::from_pixel(source.width(), source.height(), image::Rgb([255; 3]));
    for (x, y, pixel) in source.enumerate_pixels() {
        let shifted = i64::from(y) + ((f64::from(x) - 200.0) * 0.02).round() as i64;
        if let Ok(row) = u32::try_from(shifted)
            && row < source.height()
        {
            skewed.put_pixel(x, row, *pixel);
        }
    }
    let mut doc = lopdf::Document::load_mem(MATCHING).unwrap();
    replace_scan(&mut doc, skewed);
    let mut bytes = Vec::new();
    doc.save_to(&mut bytes).unwrap();
    let (_models, config) = provisioned_config();
    let result = extract(&bytes, &config);
    assert_ocr_retains_prose(&result);
    for text in [&result.content, &result.pages.as_ref().unwrap()[0].content] {
        assert_eq!(
            text.matches('០').count(),
            0,
            "a deskewed frame needs its transform: {text}"
        );
    }
}
