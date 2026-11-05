#![cfg(all(feature = "pdf", feature = "ocr", feature = "bundle-tessdata-eng"))]

mod helpers;

use image::{ImageEncoder, RgbImage};
use lopdf::{Document, Object, Stream, dictionary};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use xberg::core::config::{ExtractionConfig, OcrConfig, PageConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod, TesseractConfig};
use xberg_native_pdf::{PdfDocument, rendering};

const WIDTH: u32 = 600;
const HEIGHT: u32 = 800;
const SOURCE_DPI: u32 = 150;
const PAGE_WIDTH: f32 = 288.0;
const PAGE_HEIGHT: f32 = 384.0;
const TEXT_ROWS: [&str; 4] = [
    "harbor meadow lantern orchard compass",
    "thimble granite willow ferry quarry",
    "pasture beacon cobble saddle marble",
    "timber anchor bridge cellar dune",
];
const NATIVE_CONTROL_ROWS: [&str; 12] = [
    "alpha beta gamma delta epsilon",
    "zeta eta theta iota kappa",
    "lambda omicron sigma tau upsilon",
    "phi chi psi omega zero",
    "one two three four five",
    "six seven eight nine copper",
    "silver gold platinum nickel cobalt",
    "oxygen hydrogen carbon nitrogen helium",
    "neon argon xenon krypton radon",
    "spring summer autumn winter morning",
    "evening noon midnight dawn dusk",
    "red green blue orange violet",
];
const ENGLISH_SHA256: &str = "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2";

fn source_pdf(rows: &[&str], text_gray: u8, panel: bool) -> Vec<u8> {
    let mut operators = if panel {
        String::from("q 0 0 0.882353 rg 14.4 300 259.2 67.2 re f Q\n")
    } else {
        String::new()
    };
    writeln!(operators, "{} g BT /F1 12 Tf", f32::from(text_gray) / 255.0).unwrap();
    for (index, row) in rows.iter().enumerate() {
        let y = 268.8 - index as f32 * 21.6;
        writeln!(operators, "1 0 0 1 14.4 {y} Tm ({row}) Tj").unwrap();
    }
    operators.push_str("ET\n");
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let stream = pdf.add_object(Stream::new(dictionary! {}, operators.into_bytes()));
    let page = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => stream,
        "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
        }),
    );
    let root = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes)
        .expect("serialize known synthetic words and colored panel");
    bytes
}

fn dark_panel_raster(text_gray: u8) -> RgbImage {
    let native = PdfDocument::from_bytes(source_pdf(&TEXT_ROWS, text_gray, true)).expect("open synthetic source");
    assert_eq!(native.page_count().unwrap(), 1);
    let source = native
        .extract_page_text_with_options(0, xberg_native_pdf::document::ReadingOrder::TopToBottom)
        .expect("extract source operators independently of OCR");
    let source_words: Vec<_> = source
        .spans
        .iter()
        .flat_map(|span| span.text.split_whitespace())
        .collect();
    assert_eq!(
        source_words,
        TEXT_ROWS
            .iter()
            .flat_map(|row| row.split_whitespace())
            .collect::<Vec<_>>()
    );
    let raster = rendering::render_page(&native, 0, &rendering::RenderOptions::with_dpi(SOURCE_DPI))
        .expect("render source words into a raster");
    let image = image::load_from_memory(&raster.data)
        .expect("decode rendered source")
        .to_rgb8();
    assert_eq!(image.dimensions(), (WIDTH, HEIGHT));
    let sum: u64 = image.as_raw().iter().map(|&value| u64::from(value)).sum();
    let mean = sum as f64 / (f64::from(WIDTH) * f64::from(HEIGHT) * 3.0 * 255.0);
    let light = image
        .pixels()
        .filter(|pixel| pixel.0.iter().map(|&v| u16::from(v)).sum::<u16>() >= 689)
        .count();
    let light_fraction = light as f64 / (f64::from(WIDTH) * f64::from(HEIGHT));
    assert!(mean < 0.90, "the non-text panel must lower the page mean: {mean}");
    assert!(
        light_fraction >= 0.80,
        "retain the page-like light fraction: {light_fraction}"
    );
    image
}

fn encode_png(image: &RgbImage) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgb8,
        )
        .expect("encode source raster");
    bytes
}

fn pinned_tessdata() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("private pinned English data");
    let english = xberg_tesseract::bundled_eng_traineddata().expect("bundled English model");
    assert_eq!(hex::encode(Sha256::digest(english)), ENGLISH_SHA256);
    std::fs::write(directory.path().join("eng.traineddata"), english).expect("write pinned English model");
    directory
}

fn configuration(tessdata: &std::path::Path, known_dpi: Option<u32>) -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
        pages: Some(PageConfig {
            extract_pages: true,
            keep_native_content: true,
            ..Default::default()
        }),
        ocr: Some(OcrConfig {
            tessdata_path: Some(tessdata.to_path_buf()),
            backend_options: known_dpi.map(|dpi| serde_json::json!({ "source_dpi": dpi })),
            tesseract_config: Some(TesseractConfig {
                use_cache: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn extract_raster(name: &str, image: &RgbImage, known_dpi: Option<u32>) -> ExtractedDocument {
    let tessdata = pinned_tessdata();
    let config = configuration(tessdata.path(), known_dpi);
    let png = encode_png(image);
    let result = helpers::extract_bytes_document_blocking(&png, "image/png", &config)
        .expect("extract through the real public image API");
    if let Some(root) = std::env::var_os("XBERG_DARK_MEAN_TEST_RECEIPTS") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).expect("receipt directory");
        std::fs::write(root.join(format!("{name}.png")), png).expect("source raster receipt");
        std::fs::write(
            root.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .expect("actual public result receipt");
    }
    result
}

fn assert_source_words(result: &ExtractedDocument) {
    let expected: Vec<_> = TEXT_ROWS.iter().flat_map(|row| row.split_whitespace()).collect();
    assert_eq!(expected.len(), 20);
    assert_eq!(result.content.split_whitespace().collect::<Vec<_>>(), expected);
    let pages = result.pages.as_ref().expect("requested image page");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].content.split_whitespace().collect::<Vec<_>>(), expected);
}

#[test]
fn should_retain_every_source_word_beside_a_dark_colored_panel() {
    let image = dark_panel_raster(156);
    assert_source_words(&extract_raster("gray-text-panel", &image, None));
}

#[test]
fn should_retain_every_source_word_beside_three_dark_panels() {
    let mut image = dark_panel_raster(156);
    for top in [79, 127] {
        for y in top..top + 4 {
            for x in 30..570 {
                image.put_pixel(x, y, image::Rgb([255; 3]));
            }
        }
    }
    let result = extract_raster("three-gray-text-panels", &image, None);
    // The pre-#2475 RGB control emits this graphic prefix; its removal belongs to #2312. ~keep
    let mut legacy_words = vec!["po", "po", "ee"];
    legacy_words.extend(TEXT_ROWS.iter().flat_map(|row| row.split_whitespace()));
    assert_eq!(legacy_words.len(), 23);
    assert_eq!(result.content.split_whitespace().collect::<Vec<_>>(), legacy_words);
    let pages = result.pages.as_ref().expect("requested image page");
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].content.split_whitespace().collect::<Vec<_>>(), legacy_words);
}

#[test]
fn should_retain_black_text_beside_the_same_dark_panel() {
    let image = dark_panel_raster(0);
    assert_source_words(&extract_raster("black-text-panel", &image, None));
}

#[test]
fn should_retain_all_aligned_dense_lines_targeted_by_2475() {
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/images/test_hello_world.png");
    let bytes = std::fs::read(source).expect("mandatory pinned corpus fixture");
    assert_eq!(
        hex::encode(Sha256::digest(&bytes)),
        "a3b088cbd13c0e88faf503ffc6b7c670f0d7a553f3d4c342e5811c9f7b7a3bd2"
    );
    let source = image::load_from_memory(&bytes).unwrap().to_rgb8();
    assert_eq!(source.dimensions(), (800, 200));
    let line = image::imageops::crop_imm(&source, 219, 36, 400, 132).to_image();
    let mut image = RgbImage::from_pixel(400, 1320, image::Rgb([255; 3]));
    for index in 0..10 {
        image::imageops::replace(&mut image, &line, 0, index * 132);
    }
    let result = extract_raster("dense-aligned-2475", &image, Some(SOURCE_DPI));
    let expected = vec!["Hello World"; 10];
    assert_eq!(
        result
            .content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>(),
        expected
    );
    assert!(
        result.metadata.image_preprocessing.is_some(),
        "retain the measured default preprocessing path"
    );
}

fn mixed_partial_raster_pdf() -> Vec<u8> {
    let mut pdf = Document::load_mem(&source_pdf(&NATIVE_CONTROL_ROWS, 0, false)).expect("native control PDF");
    let first = pdf.get_pages()[&1];
    let pages = pdf
        .get_dictionary(first)
        .unwrap()
        .get(b"Parent")
        .unwrap()
        .as_reference()
        .unwrap();
    let mut resources = pdf
        .get_dictionary(first)
        .unwrap()
        .get(b"Resources")
        .unwrap()
        .as_dict()
        .unwrap()
        .clone();
    let body = dark_panel_raster(156);
    let image = pdf.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
            "Width" => body.width(), "Height" => body.height(),
        },
        body.into_raw(),
    ));
    resources.set("XObject", dictionary! { "Scan" => image });
    let scale = 0.225_f64.sqrt();
    let width = f64::from(PAGE_WIDTH) * scale;
    let height = f64::from(PAGE_HEIGHT) * scale;
    let left = (f64::from(PAGE_WIDTH) - width) / 2.0;
    let bottom = (f64::from(PAGE_HEIGHT) - height) / 2.0;
    let operators = format!(
        "q {width:.6} 0 0 {height:.6} {left:.6} {bottom:.6} cm /Scan Do Q\n\
         BT /F1 12 Tf 3 Tr 1 0 0 1 14.4 268.8 Tm (routing marker) Tj ET\n"
    );
    let content = pdf.add_object(Stream::new(dictionary! {}, operators.into_bytes()));
    let second = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => content, "Resources" => resources,
        "MediaBox" => vec![0.into(), 0.into(), PAGE_WIDTH.into(), PAGE_HEIGHT.into()],
    });
    pdf.get_object_mut(pages)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Kids", vec![first.into(), second.into()]);
    pdf.get_object_mut(pages)
        .unwrap()
        .as_dict_mut()
        .unwrap()
        .set("Count", 2);
    pdf.compress();
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("serialize the mixed partial-raster PDF");
    bytes
}

fn assert_partial_raster_authority(bytes: &[u8]) {
    let native = PdfDocument::from_bytes(bytes.to_vec()).expect("inspect actual PDF");
    assert_eq!(native.page_count().unwrap(), 2);
    assert!(native.page_image_handles(0).unwrap().is_empty());
    let images = native.page_image_handles(1).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!((images[0].width, images[0].height), (WIDTH, HEIGHT));
    let coverage = images[0].bbox.width * images[0].bbox.height / (PAGE_WIDTH * PAGE_HEIGHT);
    assert!(
        (coverage - 0.225).abs() < 0.000001,
        "exact partial raster coverage: {coverage}"
    );
    assert!(
        coverage < 0.25,
        "the PDF scan-density hint must not force preprocessing"
    );
    let layer = native
        .extract_page_text_with_options(1, xberg_native_pdf::document::ReadingOrder::TopToBottom)
        .unwrap();
    assert_eq!(
        layer
            .spans
            .iter()
            .flat_map(|span| span.text.split_whitespace())
            .collect::<Vec<_>>(),
        ["routing", "marker"]
    );
}

#[test]
fn should_retain_raster_words_on_a_mixed_partial_raster_pdf() {
    let bytes = mixed_partial_raster_pdf();
    assert_partial_raster_authority(&bytes);
    let tessdata = pinned_tessdata();
    let config = configuration(tessdata.path(), None);
    let native = helpers::extract_bytes_document_blocking(
        &bytes,
        "application/pdf",
        &ExtractionConfig {
            disable_ocr: true,
            ..config.clone()
        },
    )
    .expect("native page and actual text-layer control");
    assert_eq!(native.extraction_method, Some(ExtractionMethod::Native));
    let native_pages = native.pages.as_ref().unwrap();
    assert_eq!(native_pages.len(), 2);
    assert_eq!(native_pages[0].content.split_whitespace().count(), 60);
    assert_eq!(
        native_pages[1].content.split_whitespace().collect::<Vec<_>>(),
        ["routing", "marker"]
    );
    for word in native_pages.iter().flat_map(|page| page.content.split_whitespace()) {
        assert!(
            !TEXT_ROWS
                .iter()
                .flat_map(|row| row.split_whitespace())
                .any(|target| target == word),
            "no raster word may be borrowed from either native page: {word}"
        );
    }
    let result = helpers::extract_bytes_document_blocking(&bytes, "application/pdf", &config)
        .expect("default mixed PDF extraction");
    if let Some(root) = std::env::var_os("XBERG_DARK_MEAN_TEST_RECEIPTS") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("mixed-partial-raster.pdf"), bytes).unwrap();
        std::fs::write(
            root.join("mixed-partial-raster.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.join("mixed-partial-native.json"),
            serde_json::to_vec_pretty(&native).unwrap(),
        )
        .unwrap();
    }
    assert!(result.metadata.ocr_used);
    assert_eq!(result.extraction_method, Some(ExtractionMethod::Mixed));
    let pages = result.pages.as_ref().expect("both actual pages");
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].content, native_pages[0].content);
    assert_eq!(pages[1].native_content, Some(native_pages[1].content.clone()));
    assert_eq!(
        pages[1].content.split_whitespace().collect::<Vec<_>>(),
        TEXT_ROWS
            .iter()
            .flat_map(|row| row.split_whitespace())
            .collect::<Vec<_>>()
    );
}
