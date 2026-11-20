use super::xberg;

use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use std::fmt::Write as _;
use xberg::core::config::{ExtractInput, ExtractionConfig, ImageExtractionConfig, OcrConfig, PageConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod, TesseractConfig};

// Synthetic RGB, drawn with bundled DejaVu Sans using Pillow 12.3.0. At x=55,
// lines start at y=66,101,130 with sizes 17,8,17 on RGB(218,214,196).
// Strong ink is RGB(35,34,31); the middle line is RGB(112,112,112). ~keep
pub(super) const IMAGE: &[u8] = include_bytes!("../fixtures/ocr/embedded_psm6_panel_2327.png");
pub(super) const NATIVE: [&str; 4] = [
    "Archive inventory memorandum for the branch office and municipal staff.",
    "The original image and paper sources remain available in the reference room.",
    "Readers may request a copy from the records desk during business hours.",
    "Page 7",
];
pub(super) const FIRST: &str = "Annual calibration records are available for review.";
pub(super) const MISSING: &str = "Relay service starts Monday";
pub(super) const LAST: &str = "Deliver all equipment by Friday afternoon.";

pub(super) fn panel_pdf(overpaint: bool, duplicate: bool) -> Vec<u8> {
    panel_pdf_with_decode(overpaint, duplicate, false)
}

pub(super) fn panel_pdf_with_decode(overpaint: bool, duplicate: bool, unproved_decode: bool) -> Vec<u8> {
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

pub(super) fn config(psm: impl Into<Option<i32>>, use_cache: bool) -> ExtractionConfig {
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
                psm: psm.into(),
                use_cache: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub(super) async fn extract(bytes: Vec<u8>, config: &ExtractionConfig) -> ExtractedDocument {
    extract_input(ExtractInput::from_bytes(bytes, "application/pdf", None), config).await
}

pub(super) async fn extract_input(input: ExtractInput, config: &ExtractionConfig) -> ExtractedDocument {
    let mut batch = xberg::extract(input, config).await.unwrap();
    assert!(batch.errors.is_empty());
    assert_eq!(batch.results.len(), 1);
    let document = batch.results.remove(0);
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Native));
    document
}
