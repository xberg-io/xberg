#![cfg(all(feature = "heuristics", feature = "pdf"))]

use std::io::Cursor;
use std::sync::Arc;

use lopdf::{Document, Object, Stream, dictionary};
use xberg::engine::parsed::ParsedDocument;
use xberg::engine::structured::rasterize::{RasterizeError, pages_for_call_with_security_limits};
use xberg::extractors::security::SecurityLimits;
use xberg::heuristics::StructuredCallMode;

const PDF_MIME: &str = "application/pdf";
const DPI: u32 = 72;
const SMALL_BUDGET: usize = 400_000;
const LARGE_BUDGET: usize = 4 * 1024 * 1024;

fn limits(max_content_size: usize) -> SecurityLimits {
    SecurityLimits {
        max_content_size,
        ..Default::default()
    }
}

fn mixed_size_pdf() -> Vec<u8> {
    pdf_with_second_media_box(480.into(), 320.into())
}

fn pdf_with_second_media_box(width: Object, height: Object) -> Vec<u8> {
    let mut pdf = Document::with_version("1.4");
    let pages = pdf.new_object_id();
    let content = pdf.add_object(Stream::new(dictionary! {}, Vec::new()));
    let first = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => content,
        "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Integer(72), Object::Integer(72)],
    });
    let second = pdf.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Contents" => content, "Rotate" => 90,
        "MediaBox" => vec![Object::Integer(0), Object::Integer(0), width, height],
        "CropBox" => vec![Object::Integer(0), Object::Integer(0), Object::Integer(10), Object::Integer(20)],
    });
    pdf.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![Object::Reference(first), Object::Reference(second)], "Count" => 2,
        }),
    );
    let root = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    pdf.trailer.set("Root", root);
    let mut bytes = Vec::new();
    pdf.save_to(&mut bytes).expect("serialize synthetic mixed-size PDF");
    bytes
}

fn assert_page_two_budget_error(error: RasterizeError, limit: usize) {
    assert!(matches!(error, RasterizeError::Pdf(_)), "{error}");
    let message = error.to_string();
    assert!(message.contains("page 2"), "{message}");
    assert!(
        message.contains(&format!("security_limits.max_content_size ({limit} bytes)")),
        "{message}"
    );
}

#[tokio::test]
async fn structured_pdf_vision_rejects_the_page_that_exceeds_the_callers_budget() {
    let error = pages_for_call_with_security_limits(
        &mixed_size_pdf(),
        PDF_MIME,
        StructuredCallMode::VisionOnly,
        DPI,
        &limits(SMALL_BUDGET),
    )
    .await
    .expect_err("the second page alone requires more RGBA bytes than the request budget");
    assert_page_two_budget_error(error, SMALL_BUDGET);
}

#[tokio::test]
async fn sufficient_budget_keeps_the_actual_rotated_media_box_raster() {
    let pages = pages_for_call_with_security_limits(
        &mixed_size_pdf(),
        PDF_MIME,
        StructuredCallMode::VisionOnly,
        DPI,
        &limits(LARGE_BUDGET),
    )
    .await
    .expect("both bounded pages render");
    let dimensions: Vec<_> = pages
        .iter()
        .map(|page| {
            let image = image::load_from_memory(&page.png_bytes).expect("decode real rendered PNG");
            (page.page_number, image.width(), image.height())
        })
        .collect();
    assert_eq!(dimensions, [(1, 72, 72), (2, 320, 480)]);
}

#[test]
fn parsed_pdf_memo_does_not_bypass_its_own_raster_budget() {
    let bytes: Arc<[u8]> = Arc::from(mixed_size_pdf());
    let permissive =
        ParsedDocument::from_bytes_with_security_limits(bytes.clone(), &limits(LARGE_BUDGET)).expect("parse PDF");
    let restricted =
        ParsedDocument::from_bytes_with_security_limits(bytes, &limits(SMALL_BUDGET)).expect("parse same PDF");
    assert_eq!(permissive.rendered_pages(DPI).expect("warm permissive render").len(), 2);
    let error = restricted
        .rendered_pages(DPI)
        .expect_err("restricted PDF must not render");
    assert_page_two_budget_error(error, SMALL_BUDGET);
}

#[tokio::test]
async fn nonvision_modes_do_not_parse_or_render_a_pdf_even_with_zero_budget() {
    for mode in [
        StructuredCallMode::Skip,
        StructuredCallMode::TextOnly,
        StructuredCallMode::TextOnlyWithVisionFallback,
    ] {
        let pages = pages_for_call_with_security_limits(b"not a PDF", PDF_MIME, mode, DPI, &limits(0))
            .await
            .expect("nonvision calls must not open the PDF");
        assert_eq!(pages.len(), 0);
    }
}

#[tokio::test]
async fn pdf_budget_admits_exact_raster_and_png_charge_but_refuses_one_byte_less() {
    const EXACT: usize = 480 * 320 * 12 + 256 * 1024;
    let pdf = mixed_size_pdf();
    let pages =
        pages_for_call_with_security_limits(&pdf, PDF_MIME, StructuredCallMode::VisionOnly, DPI, &limits(EXACT))
            .await
            .expect("the exact budget admits both actual rasters");
    assert_eq!(pages.len(), 2);
    let error =
        pages_for_call_with_security_limits(&pdf, PDF_MIME, StructuredCallMode::VisionOnly, DPI, &limits(EXACT - 1))
            .await
            .expect_err("one byte below the checked live raster and PNG charge must reject");
    assert_page_two_budget_error(error, EXACT - 1);
}

#[tokio::test]
async fn pdf_budget_uses_effective_dpi_and_rounds_fractional_rotated_dimensions_up() {
    const EXACT: usize = 20_001 * 2 * 12 + 256 * 1024;
    let pdf = pdf_with_second_media_box(Object::Real(20_000.25), Object::Real(1.25));
    let pages =
        pages_for_call_with_security_limits(&pdf, PDF_MIME, StructuredCallMode::VisionOnly, 150, &limits(EXACT))
            .await
            .expect("the native DPI reduction fits the caller budget");
    let image = image::load_from_memory(&pages[1].png_bytes).expect("decode actual second page");
    assert_eq!((pages[1].page_number, image.width(), image.height()), (2, 2, 20_001));
    let error =
        pages_for_call_with_security_limits(&pdf, PDF_MIME, StructuredCallMode::VisionOnly, 150, &limits(EXACT - 1))
            .await
            .expect_err("the exact effective size includes fractional pixels rounded up");
    assert_page_two_budget_error(error, EXACT - 1);
}

#[tokio::test]
async fn image_png_budget_retains_its_inclusive_threshold() {
    const EXACT: usize = 16 * 8 * 7 + 256 * 1024;
    let image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(16, 8, image::Rgb([12, 34, 56])));
    let mut png = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let pages =
        pages_for_call_with_security_limits(&png, "image/png", StructuredCallMode::VisionOnly, DPI, &limits(EXACT))
            .await
            .expect("the exact image encoding charge is admitted");
    let result = image::load_from_memory(&pages[0].png_bytes).unwrap().to_rgb8();
    assert_eq!(result.dimensions(), (16, 8));
    assert_eq!(result.get_pixel(0, 0), &image::Rgb([12, 34, 56]));
    let error = pages_for_call_with_security_limits(
        &png,
        "image/png",
        StructuredCallMode::VisionOnly,
        DPI,
        &limits(EXACT - 1),
    )
    .await
    .expect_err("one byte below the exact image charge must reject");
    assert!(matches!(error, RasterizeError::Image(_)));
    assert!(
        error
            .to_string()
            .contains(&format!("security_limits.max_content_size ({} bytes)", EXACT - 1))
    );
}

#[path = "structured_pdf_raster_budget/cmyk.rs"]
mod cmyk;
