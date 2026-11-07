//! Proof that a page's invisible text layer was drawn on the scan beneath it.
//!
//! A layer that no OCR read confirms may still be the true text of the page: the OCR language
//! cannot read the script. The proof renders the page twice, once as painted and once with the
//! invisible glyphs revealed in red, and requires every revealed glyph shape to lie on the scan's
//! ink and the ink within it to belong to the glyph. A layer that names a different character at
//! the same place draws a different shape and fails.

use crate::ocr::processor::light_page_foreground_mode;
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::{RenderOptions, render_page};

const PROOF_DPI: u32 = 150;
const MAX_PROOF_PIXELS: f64 = 16.0 * 1024.0 * 1024.0;
const POINTS_PER_INCH: f64 = 72.0;
/// A region of disagreement smaller than this is speckle: a 4 by 4 pixel square, 0.7 mm at the
/// proof resolution. Two rasterizations of one outline differ along its edges by a pixel; a
/// different character differs over a stroke. ~keep
const MAX_SPECKLE_PIXELS: usize = 16;
/// The red channel must exceed the green one by half its range for a pixel to count as revealed. ~keep
const MIN_RED_EXCESS: i16 = 128;

struct Raster {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

fn render(doc: &PdfDocument, page_index: usize, reveal: bool) -> Option<Raster> {
    let mut options = RenderOptions::with_dpi(PROOF_DPI).as_raw();
    options.reveal_invisible_text = reveal;
    let image = render_page(doc, page_index, &options).ok()?;
    let (width, height) = (image.width as usize, image.height as usize);
    (image.data.len() == width * height * 4).then_some(Raster {
        width,
        height,
        rgba: image.data,
    })
}

fn red_excess(rgba: &[u8; 4]) -> i16 {
    i16::from(rgba[0]) - i16::from(rgba[1])
}

fn luminance(rgba: &[u8; 4]) -> u8 {
    ((54 * u16::from(rgba[0]) + 183 * u16::from(rgba[1]) + 19 * u16::from(rgba[2])) >> 8) as u8
}

/// Whether the hidden glyphs of the page lie on matching scan ink. `false` when the page has no
/// hidden glyph, the render is refused, or the glyphs and the ink beneath them disagree over an
/// area larger than speckle.
pub(super) fn hidden_layer_matches_scan(doc: &PdfDocument, page_index: usize) -> bool {
    let Ok(info) = doc.get_page_info(page_index) else {
        return false;
    };
    let scale = f64::from(PROOF_DPI) / POINTS_PER_INCH;
    let pixels = f64::from(info.media_box.width) * f64::from(info.media_box.height) * scale * scale;
    if !pixels.is_finite() || pixels > MAX_PROOF_PIXELS {
        return false;
    }
    let (Some(plain), Some(revealed)) = (render(doc, page_index, false), render(doc, page_index, true)) else {
        return false;
    };
    (plain.width, plain.height) == (revealed.width, revealed.height)
        && largest_disagreement(&plain, &revealed).is_some_and(|area| area < MAX_SPECKLE_PIXELS)
}

/// The area of the largest connected region where the revealed glyphs and the scan ink disagree
/// by more than one pixel, or `None` when the page reveals no glyph or has no ink to measure.
/// Ink counts only inside the bounding box of a revealed glyph, so unrelated page content
/// elsewhere is not held against the layer.
fn largest_disagreement(plain: &Raster, revealed: &Raster) -> Option<usize> {
    let (width, height) = (plain.width, plain.height);
    let mut histogram = [0usize; 256];
    for pixel in plain.rgba.as_chunks::<4>().0 {
        histogram[usize::from(luminance(pixel))] += 1;
    }
    let threshold = light_page_foreground_mode(&histogram)?;
    let ink: Vec<bool> = plain
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| luminance(p) <= threshold)
        .collect();
    let glyph: Vec<bool> = plain
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(revealed.rgba.as_chunks::<4>().0)
        .map(|(before, after)| red_excess(after) - red_excess(before) >= MIN_RED_EXCESS)
        .collect();
    let near_ink = dilate(&ink, width, height);
    let near_glyph = dilate(&glyph, width, height);

    let mut seen = vec![false; glyph.len()];
    let mut within_glyph_box = vec![false; glyph.len()];
    let mut any_glyph = false;
    for start in 0..glyph.len() {
        if !glyph[start] || seen[start] {
            continue;
        }
        any_glyph = true;
        let component = flood(&glyph, &mut seen, width, height, start);
        let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
        for &at in &component {
            let (x, y) = (at % width, at / width);
            left = left.min(x);
            right = right.max(x);
            top = top.min(y);
            bottom = bottom.max(y);
        }
        for y in top..=bottom {
            within_glyph_box[y * width + left..=y * width + right].fill(true);
        }
    }
    if !any_glyph {
        return None;
    }
    let disagreement: Vec<bool> = (0..glyph.len())
        .map(|at| (glyph[at] && !near_ink[at]) || (within_glyph_box[at] && ink[at] && !near_glyph[at]))
        .collect();
    let mut seen = vec![false; glyph.len()];
    let mut largest = 0;
    for start in 0..disagreement.len() {
        if disagreement[start] && !seen[start] {
            largest = largest.max(flood(&disagreement, &mut seen, width, height, start).len());
        }
    }
    Some(largest)
}

fn flood(mask: &[bool], seen: &mut [bool], width: usize, height: usize, start: usize) -> Vec<usize> {
    let mut component = vec![start];
    seen[start] = true;
    let mut next = 0;
    while next < component.len() {
        let at = component[next];
        next += 1;
        let (x, y) = (at % width, at / width);
        for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
            for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                let neighbour = ny * width + nx;
                if mask[neighbour] && !seen[neighbour] {
                    seen[neighbour] = true;
                    component.push(neighbour);
                }
            }
        }
    }
    component
}

fn dilate(mask: &[bool], width: usize, height: usize) -> Vec<bool> {
    let mut out = vec![false; mask.len()];
    for at in (0..mask.len()).filter(|&at| mask[at]) {
        let (x, y) = (at % width, at / width);
        let (left, right) = (x.saturating_sub(1), (x + 1).min(width - 1));
        for row in y.saturating_sub(1)..=(y + 1).min(height - 1) {
            out[row * width + left..=row * width + right].fill(true);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MATCHING: &[u8] = include_bytes!("../../../../tests/fixtures/pdf_khmer_number_2408/matching.pdf");
    const MISLEADING: &[u8] = include_bytes!("../../../../tests/fixtures/pdf_khmer_number_2408/misleading.pdf");

    const VISIBLE_ONLY: &[u8] = include_bytes!("../../../../tests/fixtures/ocr/raster_prose_stubs_2313.pdf");

    fn proven(bytes: &[u8]) -> bool {
        hidden_layer_matches_scan(&PdfDocument::from_bytes(bytes.to_vec()).unwrap(), 0)
    }

    fn disagreement(bytes: &[u8]) -> Option<usize> {
        let doc = PdfDocument::from_bytes(bytes.to_vec()).unwrap();
        largest_disagreement(&render(&doc, 0, false).unwrap(), &render(&doc, 0, true).unwrap())
    }

    #[test]
    fn a_layer_drawn_on_its_scan_is_proven_and_a_different_digit_is_not() {
        assert_eq!(disagreement(MATCHING), Some(0));
        assert!(proven(MATCHING));
        let wrong = disagreement(MISLEADING).unwrap();
        assert!(wrong >= MAX_SPECKLE_PIXELS, "{wrong}");
        assert!(!proven(MISLEADING));
    }

    #[test]
    fn a_page_without_invisible_text_has_nothing_to_prove() {
        assert_eq!(disagreement(VISIBLE_ONLY), None);
        assert!(!proven(VISIBLE_ONLY));
    }
}
