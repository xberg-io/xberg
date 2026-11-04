#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use std::fmt::Write;
use xberg::core::config::{ExtractInput, ExtractionConfig, OcrConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod};
use xberg_native_pdf::{PdfDocument, rendering};

const PAGE_WIDTH: f32 = 612.0;
const PAGE_HEIGHT: f32 = 792.0;
const RASTER_WIDTH: u32 = 1275;
const RASTER_HEIGHT: u32 = 1650;
const RASTER_DPI: u32 = 150;
const PARTIAL: &[usize] = &[0, 1, 2, 3, 10];
const SPARSE: &[usize] = &[0, 10];
const COMPLETE: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

const LINES: &[(f32, &str)] = &[
    (135.0, "Heading"),
    (
        245.0,
        "The coastal archive opened its wooden shutters before sunrise. Volunteers",
    ),
    (
        279.0,
        "carried maps into the reading room and carefully recorded each original",
    ),
    (313.0, "label beside the central register."),
    (
        435.0,
        "Beyond the courtyard, a narrow pathway led toward the lighthouse. During",
    ),
    (
        469.0,
        "the afternoon, the keeper inspected a brass compass and repaired a cracked",
    ),
    (503.0, "window before supper."),
    (
        625.0,
        "Across the quiet harbor, fishing boats returned with silver baskets.",
    ),
    (
        659.0,
        "Neighbors delivered warm bread to the dock and counted lanterns while the",
    ),
    (693.0, "evening tide rose gently."),
    (1215.0, "Page 7"),
];

fn page_pdf(operators: String, raster: Option<Stream>) -> Vec<u8> {
    let mut document = Document::with_version("1.4");
    let pages = document.new_object_id();
    let font = document.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let mut resources = dictionary! { "Font" => dictionary! { "F1" => font } };
    if let Some(raster) = raster {
        resources.set("XObject", dictionary! { "Scan" => document.add_object(raster) });
    }
    let content = document.add_object(Stream::new(Dictionary::new(), operators.into_bytes()));
    let page = document.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
        "Resources" => resources, "Contents" => content,
    });
    document.objects.insert(
        pages,
        Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1 }),
    );
    let root = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    document.trailer.set("Root", root);
    document.compress();
    let mut bytes = Vec::new();
    document.save_to(&mut bytes).expect("serialize synthetic PDF");
    bytes
}

fn add_text(operators: &mut String, rows: &[usize], scale: f32, bottom: f32, render_mode: u8, extra: Option<&str>) {
    let font_size = 12.0 * scale;
    let horizontal_scale = 100.0 / scale;
    writeln!(
        operators,
        "BT /F1 {font_size} Tf {horizontal_scale} Tz {render_mode} Tr"
    )
    .expect("write text state");
    for &index in rows {
        let (y, text) = LINES[index];
        let y = bottom + scale * (PAGE_HEIGHT - y * 72.0 / RASTER_DPI as f32);
        writeln!(operators, "1 0 0 1 67.2 {y} Tm ({text}) Tj").expect("write text operation");
    }
    if let Some(extra) = extra {
        writeln!(operators, "1 0 0 1 67.2 143 Tm ({extra}) Tj").expect("write independent text");
    }
    operators.push_str("ET\n");
}

fn make_raster() -> Stream {
    let mut operators = String::new();
    add_text(&mut operators, COMPLETE, 1.0, 0.0, 0, None);
    let vector = PdfDocument::from_bytes(page_pdf(operators, None)).expect("open synthetic source page");
    let rendered = rendering::render_page(&vector, 0, &rendering::RenderOptions::with_dpi(RASTER_DPI))
        .expect("render independent vector source text");
    assert_eq!((rendered.width, rendered.height), (RASTER_WIDTH, RASTER_HEIGHT));
    let image = image::load_from_memory(&rendered.data)
        .expect("decode rendered PNG")
        .into_rgb8();
    Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
            "Width" => RASTER_WIDTH, "Height" => RASTER_HEIGHT,
        },
        image.into_raw(),
    )
}

fn scanned_page(raster: &Stream, percent: u8, rows: &[usize], render_mode: u8, extra_native: Option<&str>) -> Vec<u8> {
    let scale = f64::from(percent) / 100.0;
    let image_height = f64::from(PAGE_HEIGHT) * scale;
    let bottom = (f64::from(PAGE_HEIGHT) - image_height) / 2.0;
    let mut operators = format!("q {PAGE_WIDTH} 0 0 {image_height} 0 {bottom} cm /Scan Do Q\n");
    add_text(
        &mut operators,
        rows,
        scale as f32,
        bottom as f32,
        render_mode,
        extra_native,
    );
    let bytes = page_pdf(operators, Some(raster.clone()));
    let native = PdfDocument::from_bytes(bytes.clone()).expect("open scan");
    let handles = native.page_image_handles(0).expect("read painted images");
    assert_eq!(handles.len(), 1);
    let image = &handles[0];
    assert_eq!((image.width, image.height), (RASTER_WIDTH, RASTER_HEIGHT));
    let area = f64::from(image.bbox.width) * f64::from(image.bbox.height);
    let coverage = area / f64::from(PAGE_WIDTH * PAGE_HEIGHT);
    assert!((coverage - f64::from(percent) / 100.0).abs() < 1e-6, "{coverage}");
    assert!(coverage >= 0.85);
    let signals = native.classify_page(0).expect("classify scan").signals;
    assert_eq!(signals.invisible_text_ratio, if render_mode == 3 { 1.0 } else { 0.0 },);
    bytes
}

fn default_tesseract() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        ocr: Some(OcrConfig::default()),
        ..Default::default()
    }
}

async fn extract(bytes: &[u8], config: &ExtractionConfig) -> ExtractedDocument {
    let mut batch = xberg::extract(
        ExtractInput::from_bytes(bytes.to_vec(), "application/pdf", None),
        config,
    )
    .await
    .expect("extract synthetic scan");
    assert!(batch.errors.is_empty());
    assert_eq!(batch.results.len(), 1);
    batch.results.remove(0)
}

fn words(doc: &ExtractedDocument) -> Vec<&str> {
    let content = doc.content.split_whitespace().collect::<Vec<_>>();
    let pages = doc.pages.as_deref().expect("one extracted page");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].page_number, 1);
    assert_eq!(pages[0].content.split_whitespace().collect::<Vec<_>>(), content);
    content
}

fn expected(rows: &[usize]) -> Vec<&'static str> {
    rows.iter()
        .flat_map(|&index| LINES[index].1.split_whitespace())
        .collect()
}

fn assert_exact_word_inventory(doc: &ExtractedDocument) {
    let mut actual = doc.content.split_whitespace().collect::<Vec<_>>();
    let mut source = expected(COMPLETE);
    assert_eq!(source.len(), 81);
    actual.sort_unstable();
    source.sort_unstable();
    assert_eq!(actual, source);
}

#[tokio::test]
async fn plain_auto_recovers_words_missing_from_a_partial_hidden_layer() {
    let raster = make_raster();
    let mut missing = Vec::new();
    for percent in [85, 100] {
        let pdf = scanned_page(&raster, percent, PARTIAL, 3, None);
        let native = extract(
            &pdf,
            &ExtractionConfig {
                disable_ocr: true,
                ..default_tesseract()
            },
        )
        .await;
        assert_eq!(native.extraction_method, Some(ExtractionMethod::Native));
        assert_eq!(words(&native), expected(PARTIAL));
        let forced = extract(
            &pdf,
            &ExtractionConfig {
                force_ocr: true,
                ..default_tesseract()
            },
        )
        .await;
        assert_exact_word_inventory(&forced);
        let automatic = extract(&pdf, &default_tesseract()).await;
        if words(&automatic) != expected(COMPLETE) {
            missing.push((percent, words(&automatic).len()));
        } else {
            assert!(automatic.metadata.ocr_used);
            assert!(automatic.tables.is_empty());
        }
    }
    assert!(
        missing.is_empty(),
        "automatic OCR lost raster words at these coverage/word counts: {missing:?}"
    );
}

#[tokio::test]
async fn complete_hidden_layer_keeps_its_exact_native_text() {
    let raster = make_raster();
    for percent in [85, 100] {
        let pdf = scanned_page(&raster, percent, COMPLETE, 3, None);
        let native = extract(
            &pdf,
            &ExtractionConfig {
                disable_ocr: true,
                ..default_tesseract()
            },
        )
        .await;
        let automatic = extract(&pdf, &default_tesseract()).await;
        assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
        assert!(!automatic.metadata.ocr_used);
        assert_eq!(automatic.content, native.content);
        assert_eq!(words(&automatic), expected(COMPLETE));
    }
}

#[tokio::test]
async fn sparse_visible_or_hidden_layers_still_use_page_ocr_without_duplication() {
    let raster = make_raster();
    for render_mode in [0, 3] {
        let pdf = scanned_page(&raster, 100, SPARSE, render_mode, None);
        let automatic = extract(&pdf, &default_tesseract()).await;
        assert!(automatic.metadata.ocr_used);
        assert_exact_word_inventory(&automatic);
    }
}

#[tokio::test]
async fn automatic_ocr_keeps_hidden_words_that_the_raster_does_not_contain() {
    let raster = make_raster();
    let pdf = scanned_page(&raster, 100, PARTIAL, 3, Some("Sealed voucher X512"));
    let native = extract(
        &pdf,
        &ExtractionConfig {
            disable_ocr: true,
            ..default_tesseract()
        },
    )
    .await;
    let automatic = extract(&pdf, &default_tesseract()).await;
    assert!(words(&native).contains(&"X512"));
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
    assert!(!automatic.metadata.ocr_used);
    assert_eq!(words(&automatic), words(&native));
}

#[tokio::test]
async fn explicit_page_ocr_opt_out_keeps_the_partial_native_layer() {
    let raster = make_raster();
    let pdf = scanned_page(&raster, 100, PARTIAL, 3, None);
    let automatic = extract(
        &pdf,
        &ExtractionConfig {
            ocr_near_empty_fallback: Some(false),
            ..default_tesseract()
        },
    )
    .await;
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
    assert!(!automatic.metadata.ocr_used);
    assert_eq!(words(&automatic), expected(PARTIAL));
}

#[tokio::test]
async fn unknown_hidden_font_keeps_the_existing_native_result() {
    let mut operators = "q 612 0 0 792 0 0 cm /Scan Do Q\n".to_owned();
    add_text(&mut operators, PARTIAL, 1.0, 0.0, 3, None);
    assert_eq!(operators.matches("/F1 ").count(), 1);
    let pdf = page_pdf(operators.replace("/F1 ", "/Z9 "), Some(make_raster()));
    let native = extract(
        &pdf,
        &ExtractionConfig {
            disable_ocr: true,
            ..default_tesseract()
        },
    )
    .await;
    assert_eq!(words(&native), expected(PARTIAL));
    let automatic = extract(&pdf, &default_tesseract()).await;
    assert_eq!(automatic.extraction_method, Some(ExtractionMethod::Native));
    assert!(!automatic.metadata.ocr_used);
    assert_eq!(automatic.content, native.content);
    assert_eq!(words(&automatic), words(&native));
}
