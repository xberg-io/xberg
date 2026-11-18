use super::EncodedPage;

pub(in crate::extractors::pdf::ocr) fn try_render_page_with_owned_ocr(
    doc: &xberg_native_pdf::PdfDocument,
    security_limits: &crate::extractors::security::SecurityLimits,
    images_config: Option<&crate::core::config::ImageExtractionConfig>,
    preprocessing_dpi: Option<i32>,
) -> Option<(
    EncodedPage,
    crate::pdf::render::OcrRasterDpi,
    crate::ocr::processor::OwnedOcrRaster,
)> {
    let (canvas, dpi) = crate::pdf::render::render_pdf_ocr_raster(
        doc,
        0,
        images_config,
        security_limits,
        &mut false,
        crate::pdf::render::OcrRasterOptions {
            preprocessing_dpi,
            owned_rgba: true,
        },
    )
    .ok()?;
    // Existing page recovery consumes PNG bytes. Keep the stock encoding from this same canvas. ~keep
    let png = xberg_native_pdf::rendering::encode_opaque_ocr_canvas(&canvas).ok()??;
    let (width, height) = (canvas.width, canvas.height);
    tracing::debug!(
        target: "xberg::ocr::raster",
        width,
        height,
        encoded_bytes = png.len(),
        "Retained PNG for native PDF OCR recovery"
    );
    let mut rgb = canvas.data;
    for pixel in 0..rgb.len() / 4 {
        let source = pixel * 4;
        rgb.copy_within(source..source + 3, pixel * 3);
    }
    rgb.truncate(rgb.len() / 4 * 3);
    let page = (0, std::sync::Arc::new(png), width, height);
    let raster = crate::ocr::processor::OwnedOcrRaster {
        rgb,
        width,
        height,
        dpi: dpi.actual,
    };
    Some((page, dpi, raster))
}
