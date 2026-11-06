#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use std::fmt::Write as _;
use xberg::core::config::{ExtractInput, ExtractionConfig, ImageExtractionConfig, OcrConfig, PageConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod, TesseractConfig};

// Synthetic RGB, drawn with bundled DejaVu Sans using Pillow 12.3.0. At x=55,
// lines start at y=66,101,130 with sizes 17,8,17 on RGB(218,214,196).
// Strong ink is RGB(35,34,31); the middle line is RGB(112,112,112). ~keep
const IMAGE: &[u8] = include_bytes!("fixtures/ocr/embedded_psm6_panel_2327.png");
const NATIVE: [&str; 4] = [
    "Archive inventory memorandum for the branch office and municipal staff.",
    "The original image and paper sources remain available in the reference room.",
    "Readers may request a copy from the records desk during business hours.",
    "Page 7",
];
const FIRST: &str = "Annual calibration records are available for review.";
const MISSING: &str = "Relay service starts Monday";
const LAST: &str = "Deliver all equipment by Friday afternoon.";

fn panel_pdf(overpaint: bool, duplicate: bool) -> Vec<u8> {
    panel_pdf_with_decode(overpaint, duplicate, false)
}

fn panel_pdf_with_decode(overpaint: bool, duplicate: bool, unproved_decode: bool) -> Vec<u8> {
    let raster = image::load_from_memory(IMAGE)
        .expect("load synthetic panel")
        .into_rgb8();
    assert_eq!(raster.dimensions(), (612, 396));

    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let mut image_dictionary = dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
            "Width" => 612, "Height" => 396,
    };
    if unproved_decode {
        image_dictionary.set(
            "Decode",
            vec![0.into(), 1.into(), 0.into(), 1.into(), 0.into(), 1.into()],
        );
    }
    let image = pdf.add_object(Stream::new(image_dictionary, raster.into_raw()));
    let mut drawing = String::new();
    for (index, line) in NATIVE.into_iter().enumerate() {
        writeln!(
            drawing,
            "BT /F1 12 Tf 1 0 0 1 72 {} Tm ({line}) Tj ET",
            740 - index * 24
        )
        .expect("write native line");
    }
    drawing.push_str("q 612 0 0 396 0 0 cm /Panel Do Q\n");
    if duplicate {
        drawing.push_str("q 612 0 0 396 0 0 cm /Panel Do Q\n");
    }
    if overpaint {
        drawing.push_str("BT /F1 24 Tf 1 0 0 1 55 284 Tm (Inspection overlay) Tj ET\n");
    }
    let content = pdf.add_object(Stream::new(Dictionary::new(), drawing.into_bytes()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font }, "XObject" => dictionary! { "Panel" => image } },
        "Contents" => content,
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1 }),
    );
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", catalog);
    pdf.compress();
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("serialize PDF");
    bytes
}

fn config(psm: i32, use_cache: bool) -> ExtractionConfig {
    ExtractionConfig {
        use_cache,
        pages: Some(PageConfig {
            extract_pages: true,
            ..Default::default()
        }),
        images: Some(ImageExtractionConfig {
            extract_images: true,
            inject_placeholders: false,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            tesseract_config: Some(TesseractConfig {
                psm: Some(psm),
                use_cache: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn extract(bytes: Vec<u8>, config: &ExtractionConfig) -> ExtractedDocument {
    extract_input(ExtractInput::from_bytes(bytes, "application/pdf", None), config).await
}

async fn extract_input(input: ExtractInput, config: &ExtractionConfig) -> ExtractedDocument {
    let mut batch = xberg::extract(input, config).await.unwrap();
    assert!(batch.errors.is_empty());
    assert_eq!(batch.results.len(), 1);
    let document = batch.results.remove(0);
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Native));
    document
}

#[tokio::test]
async fn explicit_psm6_adds_only_the_missing_embedded_line() {
    use sha2::{Digest, Sha256};
    assert_eq!(
        hex::encode(Sha256::digest(IMAGE)),
        "04c680cdd7c95eb6ca7df11a60c93b07d6b902306d6b658c37c0b5a44b74821f"
    );
    let result = extract(panel_pdf(false, false), &config(6, false)).await;
    let [image] = result.images.as_deref().expect("retained images") else {
        panic!("expected exactly one retained image");
    };
    let original = image.ocr_result.as_ref().expect("original nested OCR");
    assert_eq!(original.content, format!("{FIRST}\n{LAST}"));
    assert_eq!(original.metadata.additional.get("psm"), Some(&serde_json::json!("6")));
    assert!(original.tables.is_empty());
    assert!(original.ocr_elements.is_none());
    let [page] = result.pages.as_deref().expect("tracked page") else {
        panic!("expected one page");
    };
    assert_eq!(page.page_number, 1);
    let expected = format!("{}\n\n{FIRST}\n{MISSING}\n{LAST}", NATIVE.join("\n\n"));
    assert_eq!(result.content, expected);
    assert_eq!(page.content, expected);
    let mut automatic = config(6, false);
    automatic.images = None;
    let ordinary = extract(panel_pdf(false, false), &automatic).await;
    assert_eq!(ordinary.content, expected);
    assert_eq!(ordinary.pages.as_deref().unwrap()[0].content, expected);

    let mut placeholders = config(6, false);
    placeholders.images.as_mut().unwrap().inject_placeholders = true;
    let untouched = extract(panel_pdf(false, false), &placeholders).await;
    assert!(!untouched.content.contains(MISSING));
    assert_eq!(
        serde_json::to_value(original).unwrap(),
        serde_json::to_value(untouched.images.as_deref().unwrap()[0].ocr_result.as_ref().unwrap()).unwrap()
    );
}

#[tokio::test]
async fn embedded_line_recovery_leaves_other_modes_and_unproved_placements_untouched() {
    for (psm, late, duplicate) in [(11, false, false), (6, true, false), (6, false, true)] {
        let result = extract(panel_pdf(late, duplicate), &config(psm, false)).await;
        assert_eq!(result.images.as_ref().unwrap().len(), if duplicate { 2 } else { 1 });
        assert!(
            !result.content.contains(MISSING),
            "psm={psm} late={late} duplicate={duplicate}"
        );
        if late {
            assert!(result.content.contains("Inspection overlay"));
        }
    }
}

#[tokio::test]
async fn uri_recovery_keeps_the_nested_result_and_respects_cache_opt_in() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("embedded-panel.pdf");
    std::fs::write(&path, panel_pdf(false, false)).unwrap();
    let uri = ExtractInput::from_uri(path.to_string_lossy().into_owned());
    let enabled = extract_input(uri.clone(), &config(6, false)).await;
    let cached = extract_input(uri, &config(6, true)).await;
    assert_eq!(enabled.content.matches(MISSING).count(), 1);
    assert_eq!(enabled.pages.as_ref().unwrap()[0].content.matches(MISSING).count(), 1);
    assert!(!cached.content.contains(MISSING));
    assert_eq!(
        serde_json::to_value(enabled.images.as_deref().unwrap()[0].ocr_result.as_ref().unwrap()).unwrap(),
        serde_json::to_value(cached.images.as_deref().unwrap()[0].ocr_result.as_ref().unwrap()).unwrap()
    );
}

#[tokio::test]
async fn explicit_decode_remains_outside_the_pixel_proof() {
    let result = extract(panel_pdf_with_decode(false, false, true), &config(6, false)).await;
    assert!(!result.content.contains(MISSING));
    let image = &result.images.as_deref().unwrap()[0];
    assert_eq!(image.ocr_result.as_ref().unwrap().content, format!("{FIRST}\n{LAST}"));
}
