#![cfg(all(feature = "pdf", feature = "ocr", feature = "tokio-runtime"))]

//! Embedded-picture line recovery on pages other than the first (#2327). The picture is the
//! synthetic panel of `pdf_embedded_psm6_addition_2327`: strong first and last lines and a
//! faint middle line that the stock read of the stored raster drops.

use lopdf::{Dictionary, Document, Object, Stream, dictionary};
use xberg::core::config::{ExtractInput, ExtractionConfig, ImageExtractionConfig, OcrConfig, PageConfig};
use xberg::types::{ExtractedDocument, ExtractionMethod, TesseractConfig};

const IMAGE: &[u8] = include_bytes!("fixtures/ocr/embedded_psm6_panel_2327.png");
const MISSING: &str = "Relay service starts Monday";
const FIRST: &str = "Annual calibration records are available for review.";
const LAST: &str = "Deliver all equipment by Friday afternoon.";

fn native_page(label: &str) -> String {
    format!(
        "BT /F1 12 Tf 1 0 0 1 72 740 Tm (Archive inventory memorandum {label} for the branch office and municipal staff.) Tj ET\n\
         BT /F1 12 Tf 1 0 0 1 72 716 Tm (The original image and paper sources remain available in the reference room.) Tj ET\n\
         BT /F1 12 Tf 1 0 0 1 72 692 Tm (Readers may request a copy from the records desk during business hours.) Tj ET\n"
    )
}

/// `leading` text-only pages, then the panel page, then `trailing` text-only pages.
/// `after_panel` is appended to the panel page's content stream.
fn build(leading: usize, trailing: usize, with_logo: bool, after_panel: &str) -> Vec<u8> {
    let raster = image::load_from_memory(IMAGE).unwrap().into_rgb8();
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let font = pdf.add_object(
        dictionary! {"Type"=>"Font","Subtype"=>"Type1","BaseFont"=>"Helvetica","Encoding"=>"WinAnsiEncoding"},
    );
    let panel = pdf.add_object(Stream::new(
        dictionary! {"Type"=>"XObject","Subtype"=>"Image","ColorSpace"=>"DeviceRGB","BitsPerComponent"=>8,"Width"=>612,"Height"=>396},
        raster.into_raw(),
    ));
    let logo = pdf.add_object(Stream::new(
        dictionary! {"Type"=>"XObject","Subtype"=>"Image","ColorSpace"=>"DeviceGray","BitsPerComponent"=>8,"Width"=>64,"Height"=>64},
        (0..64 * 64)
            .map(|i| if (i / 64 + i % 64) % 16 < 8 { 40u8 } else { 215 })
            .collect::<Vec<u8>>(),
    ));
    let mut kids = Vec::new();
    let text_page = |pdf: &mut Document, kids: &mut Vec<Object>, label: &str| {
        let content = pdf.add_object(Stream::new(Dictionary::new(), native_page(label).into_bytes()));
        kids.push(Object::Reference(pdf.add_object(dictionary! {
        "Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()],
        "Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font}},"Contents"=>content})));
    };
    for index in 0..leading {
        text_page(&mut pdf, &mut kids, &format!("leading {index}"));
    }
    let mut drawing = native_page("panel");
    drawing.push_str("q 612 0 0 396 0 0 cm /Panel Do Q\n");
    if with_logo {
        drawing.push_str("q 48 0 0 48 520 730 cm /Logo Do Q\n");
    }
    drawing.push_str(after_panel);
    let content = pdf.add_object(Stream::new(Dictionary::new(), drawing.into_bytes()));
    kids.push(Object::Reference(pdf.add_object(dictionary! {
    "Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),612.into(),792.into()],
    "Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font},"XObject"=>dictionary!{"Panel"=>panel,"Logo"=>logo}},
    "Contents"=>content})));
    for index in 0..trailing {
        text_page(&mut pdf, &mut kids, &format!("trailing {index}"));
    }
    let count = kids.len() as i64;
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>count}),
    );
    let catalog = pdf.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    pdf.trailer.set("Root", catalog);
    pdf.compress();
    let mut out = Vec::new();
    pdf.save_to(&mut out).unwrap();
    out
}

fn config() -> ExtractionConfig {
    ExtractionConfig {
        use_cache: false,
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
                use_cache: false,
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

async fn extract(bytes: Vec<u8>) -> ExtractedDocument {
    let mut batch = xberg::extract(ExtractInput::from_bytes(bytes, "application/pdf", None), &config())
        .await
        .unwrap();
    assert!(batch.errors.is_empty());
    let document = batch.results.remove(0);
    assert_eq!(document.extraction_method, Some(ExtractionMethod::Native));
    document
}

fn nested(document: &ExtractedDocument) -> String {
    let images = document.images.as_deref().expect("retained images");
    images
        .iter()
        .filter_map(|image| image.ocr_result.as_ref())
        .map(|ocr| ocr.content.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn picture_on_a_later_page_gets_its_missing_line_once() {
    for (leading, trailing, panel_page) in [(1usize, 0usize, 2u32), (2, 1, 3)] {
        let result = extract(build(leading, trailing, false, "")).await;
        assert_eq!(result.content.matches(MISSING).count(), 1, "leading={leading}");
        let pages = result.pages.as_deref().expect("tracked pages");
        for page in pages {
            let expected = usize::from(page.page_number == panel_page);
            assert_eq!(
                page.content.matches(MISSING).count(),
                expected,
                "page {}",
                page.page_number
            );
        }
        let panel = pages.iter().find(|page| page.page_number == panel_page).unwrap();
        assert!(panel.content.contains(FIRST) && panel.content.contains(LAST));
        assert_eq!(
            nested(&result),
            format!("{FIRST}\n\n{LAST}"),
            "nested read is untouched"
        );
    }
}

#[tokio::test]
async fn later_paint_and_extra_pictures_on_the_page_add_nothing() {
    let overpainted = extract(build(
        1,
        0,
        false,
        "BT /F1 24 Tf 1 0 0 1 55 284 Tm (Inspection overlay) Tj ET\n",
    ))
    .await;
    assert!(overpainted.content.contains("Inspection overlay"));
    assert!(!overpainted.content.contains(MISSING));

    let with_logo = extract(build(1, 0, true, "")).await;
    assert!(!with_logo.content.contains(MISSING));
}
