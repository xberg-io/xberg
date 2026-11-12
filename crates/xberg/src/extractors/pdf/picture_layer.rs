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

/// Read the picture on pages the script gate kept for their visible native text. ~keep
pub(super) async fn read_retained_picture_pages(
    content: &[u8],
    text: &str,
    boundaries: &[crate::types::PageBoundary],
    candidates: &[u32],
    replacements: Option<&ahash::AHashMap<u32, String>>,
    config: &ExtractionConfig,
) -> crate::Result<ahash::AHashMap<u32, String>> {
    let mut additions = ahash::AHashMap::new();
    let mut eligible = candidates
        .iter()
        .copied()
        .filter(|&page| {
            super::ocr::native_preservation_is_automatic(config, page)
                && replacements.is_none_or(|replaced| !replaced.contains_key(&page))
        })
        .peekable();
    if eligible.peek().is_none() {
        return Ok(additions);
    }
    let document = match xberg_native_pdf::PdfDocument::from_bytes(content.to_vec()) {
        Ok(document) => document,
        Err(error) => {
            tracing::debug!(%error, "could not open PDF for picture-layer OCR");
            return Ok(additions);
        }
    };
    for page in eligible {
        super::ensure_pdf_not_cancelled(config)?;
        let Some(layer) = super::unreadable_script::page_text(text, boundaries, page) else {
            continue;
        };
        let Some(index) = page.checked_sub(1).map(|index| index as usize) else {
            continue;
        };
        if !layer_is_visible_over_picture(&document, index, layer, config) {
            continue;
        }
        if let Some(combined) = read_picture_with_native_overlay(&document, index, config).await? {
            additions.insert(page, combined);
        }
    }
    Ok(additions)
}

/// Keep native text from painting operators and read the picture with those glyphs suppressed. ~keep
async fn read_picture_with_native_overlay(
    document: &xberg_native_pdf::PdfDocument,
    index: usize,
    config: &ExtractionConfig,
) -> crate::Result<Option<String>> {
    let budget = super::ocr::preview_byte_limit(config);
    let raster = xberg_native_pdf::rendering::render_page_without_glyphs(document, index, PICTURE_LAYER_DPI, budget);
    let (width, height, luminance) = match raster {
        Ok(raster) => raster,
        Err(error) => {
            tracing::debug!(%error, index, "could not render the picture without its native glyphs");
            return Ok(None);
        }
    };
    let Some(image) = image::GrayImage::from_raw(width, height, luminance) else {
        return Ok(None);
    };
    super::ensure_pdf_not_cancelled(config)?;
    let read = super::ocr::read_rendered_page(image::DynamicImage::ImageLuma8(image), config).await;
    super::ensure_pdf_not_cancelled(config)?;
    let (read, confidence) = match read {
        Ok(read) => read,
        Err(error) => {
            tracing::debug!(%error, index, "could not read the picture beneath the native overlay");
            return Ok(None);
        }
    };
    let thresholds = config
        .ocr
        .as_ref()
        .map(|ocr| ocr.effective_thresholds())
        .unwrap_or_default();
    if read.trim().is_empty()
        || !confidence.is_some_and(|confidence| {
            confidence.word_count as usize >= thresholds.min_meaningful_words
                && confidence
                    .score
                    .is_some_and(|score| score >= thresholds.min_ocr_mean_confidence / 100.0)
        })
    {
        return Ok(None);
    }
    match document.extract_text_without_nonpainting_glyphs(index) {
        Ok(mut visible) => {
            super::unmapped_glyphs::strip_native_fragment(&mut visible);
            Ok((!visible.trim().is_empty()).then(|| format!("{}\n\n{}", visible.trim(), read.trim())))
        }
        Err(error) => {
            tracing::debug!(%error, index, "could not extract the native overlay separately");
            Ok(None)
        }
    }
}
