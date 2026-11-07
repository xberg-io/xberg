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

#[derive(Clone, Default)]
struct Layout {
    leading: usize,
    trailing: usize,
    with_logo: bool,
    after_panel: &'static str,
    inherit_media_box: bool,
    inherited_crop_box: Option<[i64; 4]>,
    second_panel_page: bool,
    filler_lines: usize,
}

/// `leading` text-only pages, the panel page, then `trailing` text-only pages.
fn build(layout: &Layout) -> Vec<u8> {
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
    let media_box: Vec<Object> = vec![0.into(), 0.into(), 612.into(), 792.into()];
    let mut kids = Vec::new();
    let page = |pdf: &mut Document, kids: &mut Vec<Object>, content: String, xobjects: Dictionary| {
        let content = pdf.add_object(Stream::new(Dictionary::new(), content.into_bytes()));
        let mut dictionary = dictionary! {"Type"=>"Page","Parent"=>pages,
        "Resources"=>dictionary!{"Font"=>dictionary!{"F1"=>font},"XObject"=>xobjects},"Contents"=>content};
        if !layout.inherit_media_box {
            dictionary.set("MediaBox", media_box.clone());
        }
        kids.push(Object::Reference(pdf.add_object(dictionary)));
    };
    let mut leading_text = String::new();
    for line in 0..layout.filler_lines {
        leading_text.push_str(&format!(
            "BT /F1 8 Tf 1 0 0 1 20 {} Tm (Filler line {line} of ordinary running text that pads the document well past the native text budget.) Tj ET\n",
            20 + (line % 700)
        ));
    }
    for index in 0..layout.leading {
        let text = format!("{}{leading_text}", native_page(&format!("leading {index}")));
        page(&mut pdf, &mut kids, text, Dictionary::new());
    }
    let panel_page = |label: &str| {
        let mut drawing = native_page(label);
        drawing.push_str("q 612 0 0 396 0 0 cm /Panel Do Q\n");
        if layout.with_logo {
            drawing.push_str("q 48 0 0 48 520 730 cm /Logo Do Q\n");
        }
        drawing.push_str(layout.after_panel);
        drawing
    };
    let xobjects = dictionary! {"Panel"=>panel,"Logo"=>logo};
    page(&mut pdf, &mut kids, panel_page("panel"), xobjects.clone());
    if layout.second_panel_page {
        page(&mut pdf, &mut kids, panel_page("second panel"), xobjects);
    }
    for index in 0..layout.trailing {
        page(
            &mut pdf,
            &mut kids,
            native_page(&format!("trailing {index}")),
            Dictionary::new(),
        );
    }
    let count = kids.len() as i64;
    let mut tree = dictionary! {"Type"=>"Pages","Kids"=>kids,"Count"=>count};
    if layout.inherit_media_box {
        tree.set("MediaBox", media_box);
    }
    if let Some(crop) = layout.inherited_crop_box {
        tree.set(
            "CropBox",
            crop.iter().map(|value| Object::from(*value)).collect::<Vec<_>>(),
        );
    }
    pdf.objects.insert(pages, Object::Dictionary(tree));
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
    for (leading, trailing, panel_page) in [(1usize, 0usize, 2u32), (2, 1, 3), (3, 0, 4)] {
        let result = extract(build(&Layout {
            leading,
            trailing,
            ..Default::default()
        }))
        .await;
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
async fn recovered_line_stays_with_its_page_in_document_order() {
    let result = extract(build(&Layout {
        leading: 2,
        trailing: 1,
        ..Default::default()
    }))
    .await;
    let at = |needle: &str| {
        result
            .content
            .find(needle)
            .unwrap_or_else(|| panic!("{needle} missing"))
    };
    assert!(at("leading 1") < at(MISSING), "after the preceding page");
    assert!(at(MISSING) < at("trailing 0"), "before the following page");
}

#[tokio::test]
async fn page_attributes_inherited_from_the_page_tree_are_honoured() {
    let inherited = extract(build(&Layout {
        leading: 1,
        inherit_media_box: true,
        ..Default::default()
    }))
    .await;
    assert_eq!(inherited.content.matches(MISSING).count(), 1);

    // An inherited CropBox that differs from the MediaBox changes the visible frame, which the
    // panel proof does not measure, so the page is left to the ordinary read.
    let cropped = extract(build(&Layout {
        leading: 1,
        inherit_media_box: true,
        inherited_crop_box: Some([0, 0, 612, 700]),
        ..Default::default()
    }))
    .await;
    assert!(!cropped.content.contains(MISSING));
}

#[tokio::test]
async fn a_long_document_still_recovers_the_picture_line() {
    let result = extract(build(&Layout {
        leading: 1,
        filler_lines: 900,
        ..Default::default()
    }))
    .await;
    assert!(
        result.content.len() > 64 * 1024,
        "document exceeds the native text budget"
    );
    assert_eq!(result.content.matches(MISSING).count(), 1);
}

#[tokio::test]
async fn two_pictures_on_different_pages_recover_nothing() {
    let result = extract(build(&Layout {
        leading: 1,
        second_panel_page: true,
        ..Default::default()
    }))
    .await;
    assert!(!result.content.contains(MISSING));
}

#[tokio::test]
async fn later_paint_and_extra_pictures_on_the_page_add_nothing() {
    let overpainted = extract(build(&Layout {
        leading: 1,
        after_panel: "BT /F1 24 Tf 1 0 0 1 55 284 Tm (Inspection overlay) Tj ET\n",
        ..Default::default()
    }))
    .await;
    assert!(overpainted.content.contains("Inspection overlay"));
    assert!(!overpainted.content.contains(MISSING));

    let with_logo = extract(build(&Layout {
        leading: 1,
        with_logo: true,
        ..Default::default()
    }))
    .await;
    assert!(!with_logo.content.contains(MISSING));
}
