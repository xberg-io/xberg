//! The ink a text layer drawn over a full-page picture leaves, in the page and on blank paper (#2826).

use crate::core::config::ExtractionConfig;

/// Resolution of the renders that look for the layer's ink.
const PICTURE_LAYER_DPI: u32 = 150;

/// The layer's glyphs ink a pixel of the page render when they change it by more than this many
/// grey levels.
const MIN_GLYPH_CONTRAST: u8 = 40;

/// Luminance under which a pixel of the render without pictures is ink.
const MAX_INK_LUMINANCE: u8 = 127;

/// Ink pixels per glyph, at [`PICTURE_LAYER_DPI`], that a layer a reader sees leaves in each render.
/// A layer printed on a picture leaves several times this; a layer drawn invisibly by render mode,
/// opacity 0, an empty clip, a fill the colour of the picture or under the picture leaves none in
/// one of the two. ~keep
const MIN_INK_PER_GLYPH: f64 = 5.0;

/// Whether the text layer of a page with a full-page picture is drawn over it where a reader sees
/// it: the glyphs drawn visibly after the picture (see
/// [`crate::pdf::scan_detect::glyph_share_over_full_page_picture`]) leave [`MIN_INK_PER_GLYPH`]
/// both where they change the page render and in a render without pictures. `layer` is the page's
/// native text; its characters, scaled by the share of the page's glyphs drawn over the picture,
/// stand for those glyphs. A render that fails or is cancelled sees nothing.
pub(super) fn layer_is_visible_over_picture(
    document: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    layer: &str,
    config: &ExtractionConfig,
) -> bool {
    let Some(share) = crate::pdf::scan_detect::glyph_share_over_full_page_picture(document, page_index) else {
        return false;
    };
    let glyphs = layer.chars().filter(|c| !c.is_whitespace()).count() as f64 * share;
    if glyphs <= 0.0 {
        return false;
    }
    let budget = super::ocr::preview_byte_limit(config);
    let cancelled = || config.cancel_token.as_ref().is_some_and(|token| token.is_cancelled());
    let Ok(Some(rasters)) = xberg_native_pdf::rendering::render_text_ink_rasters(
        document,
        page_index,
        PICTURE_LAYER_DPI,
        budget,
        cancelled,
    ) else {
        return false;
    };
    let glyph_ink = rasters
        .drawn
        .iter()
        .zip(&rasters.without_glyphs)
        .filter(|(drawn, without)| drawn.abs_diff(**without) > MIN_GLYPH_CONTRAST)
        .count();
    if (glyph_ink as f64) < MIN_INK_PER_GLYPH * glyphs || cancelled() {
        return false;
    }
    let Ok((_, _, without_pictures)) =
        xberg_native_pdf::rendering::render_page_without_images(document, page_index, PICTURE_LAYER_DPI, budget)
    else {
        return false;
    };
    let ink = without_pictures
        .iter()
        .filter(|luminance| **luminance <= MAX_INK_LUMINANCE)
        .count();
    ink as f64 >= MIN_INK_PER_GLYPH * glyphs
}

/// The page rendered without its pictures, as an image to read: the text and vector art on blank
/// paper. `None` when the render fails.
pub(super) fn page_without_pictures(
    document: &xberg_native_pdf::PdfDocument,
    page_index: usize,
    config: &ExtractionConfig,
) -> Option<image::DynamicImage> {
    let (width, height, luminance) = xberg_native_pdf::rendering::render_page_without_images(
        document,
        page_index,
        PICTURE_LAYER_DPI,
        super::ocr::preview_byte_limit(config),
    )
    .ok()?;
    image::GrayImage::from_raw(width, height, luminance).map(image::DynamicImage::ImageLuma8)
}
