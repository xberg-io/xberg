//! Proof that a page's invisible text layer was drawn on the scan beneath it.
//!
//! A layer that no OCR read confirms may still be the true text of the page: the OCR language
//! cannot read the script. The proof renders the page as drawn and with the invisible glyphs
//! revealed, then checks every character of the layer against the scan ink beneath its own box.
//! It proves shape, not Unicode: a layer whose font maps the right shape to the wrong code point
//! passes, so a caller must first restrict it to a layer with no unmapped glyph.
//!
//! The proof refuses rather than guesses. A character whose strokes are too thin at the proof
//! resolution to tell shapes apart, a box that is mostly ink, a character that reveals no glyph
//! (render mode 7, Type 3, clipped, off-page) and any layer whose characters do not match the
//! page's characters one for one are all "not proven".

use crate::ocr::processor::light_page_foreground_mode;
use xberg_native_pdf::PdfDocument;
use xberg_native_pdf::rendering::render_revealed_layer;

const PROOF_DPI: u32 = 300;
const POINTS_PER_INCH: f64 = 72.0;
const MAX_PAGE_PIXELS: f64 = 16.0 * 1024.0 * 1024.0;
const MAX_DOCUMENT_PIXELS: f64 = 64.0 * 1024.0 * 1024.0;
const MAX_DOCUMENT_PAGES: usize = 4;
/// Thinner strokes than this leave a matching glyph with a noise floor near the disagreement of a
/// wrong one: at 300 DPI an 11 pt stroke (about 4 px) disagrees with its own scan by 10 percent,
/// a wrong digit by 21. Measured strokes: 24 pt about 9 px, 30 pt about 10 px. ~keep
const MIN_STROKE_PIXELS: f64 = 8.0;
/// The share of a glyph's pixels and of the ink beside it that may disagree. Above the stroke
/// minimum, matching glyphs measured 0.0 to 0.006 and wrong ones 0.067 to 0.144. ~keep
const MAX_DISAGREEMENT: f64 = 0.03;
/// Beyond this share of ink in the box around a glyph, the background is no evidence of anything. ~keep
const MAX_BACKGROUND_INK: f64 = 0.5;

/// Pages and pixels one document may spend on proofs.
pub(super) struct ProofBudget {
    pages_left: usize,
    pixels_left: f64,
    byte_limit: usize,
}

impl ProofBudget {
    pub(super) fn new(byte_limit: usize) -> Self {
        Self {
            pages_left: MAX_DOCUMENT_PAGES,
            pixels_left: MAX_DOCUMENT_PIXELS,
            byte_limit,
        }
    }

    fn charge(&mut self, pixels: f64) -> bool {
        if self.pages_left == 0 || !pixels.is_finite() || pixels > MAX_PAGE_PIXELS || pixels > self.pixels_left {
            return false;
        }
        self.pages_left -= 1;
        self.pixels_left -= pixels;
        true
    }
}

struct Window {
    left: usize,
    top: usize,
    width: usize,
    height: usize,
}

/// Whether every character of the page's hidden layer lies on matching scan ink.
pub(super) fn hidden_layer_matches_scan(
    doc: &PdfDocument,
    page_index: usize,
    layer: &str,
    budget: &mut ProofBudget,
    cancelled: impl Fn() -> bool,
) -> bool {
    prove(doc, page_index, layer, budget, cancelled).is_some_and(|verdict| verdict.proven())
}

#[derive(Debug, Default)]
struct Verdict {
    /// Worst disagreement share of any one glyph.
    worst: f64,
    miss_and_extra: usize,
    denominator: usize,
}

impl Verdict {
    fn proven(&self) -> bool {
        self.worst <= MAX_DISAGREEMENT && self.page_share() <= MAX_DISAGREEMENT
    }

    fn page_share(&self) -> f64 {
        self.miss_and_extra as f64 / self.denominator.max(1) as f64
    }
}

fn prove(
    doc: &PdfDocument,
    page_index: usize,
    layer: &str,
    budget: &mut ProofBudget,
    cancelled: impl Fn() -> bool,
) -> Option<Verdict> {
    if layer.contains(super::super::unmapped_glyphs::LOST_GLYPH_MARKER) {
        return None;
    }
    let info = doc.get_page_info(page_index).ok()?;
    let media = info.media_box;
    if info.rotation != 0 {
        return None;
    }
    let chars = doc.extract_chars(page_index).ok()?;
    let chars: Vec<_> = chars.into_iter().filter(|c| !c.char.is_whitespace()).collect();
    if chars.is_empty()
        || !chars
            .iter()
            .map(|c| c.char)
            .eq(layer.chars().filter(|c| !c.is_whitespace()))
        || chars.iter().any(|c| c.rotation_degrees != 0.0)
    {
        return None;
    }
    let scale = f64::from(PROOF_DPI) / POINTS_PER_INCH;
    if !budget.charge(f64::from(media.width) * f64::from(media.height) * scale * scale) {
        return None;
    }
    let raster = render_revealed_layer(doc, page_index, PROOF_DPI, budget.byte_limit, &cancelled).ok()??;
    let (width, height) = (raster.width as usize, raster.height as usize);
    if raster.luminance.len() != width * height || raster.revealed.len() != width * height {
        return None;
    }
    let mut histogram = [0usize; 256];
    for &sample in &raster.luminance {
        histogram[usize::from(sample)] += 1;
    }
    let threshold = light_page_foreground_mode(&histogram)?;
    let ink: Vec<bool> = raster.luminance.iter().map(|&sample| sample <= threshold).collect();

    let scale = scale as f32;
    let mut verdict = Verdict::default();
    for character in &chars {
        if cancelled() {
            return None;
        }
        let b = character.bbox;
        let left = ((b.x - media.x) * scale).floor();
        let right = ((b.x + b.width - media.x) * scale).ceil();
        let top = ((media.y + media.height - b.y - b.height) * scale).floor();
        let bottom = ((media.y + media.height - b.y) * scale).ceil();
        if ![left, right, top, bottom].iter().all(|v| v.is_finite())
            || left < 0.0
            || top < 0.0
            || right > width as f32
            || bottom > height as f32
            || right <= left
            || bottom <= top
        {
            return None;
        }
        let window = Window {
            left: left as usize,
            top: top as usize,
            width: (right - left) as usize,
            height: (bottom - top) as usize,
        };
        let (misses, denominator) = score_glyph(
            &crop(&ink, width, &window),
            &crop(&raster.revealed, width, &window),
            window.width,
            window.height,
        )?;
        verdict.worst = verdict.worst.max(misses as f64 / denominator as f64);
        verdict.miss_and_extra += misses;
        verdict.denominator += denominator;
    }
    Some(verdict)
}

fn crop(mask: &[bool], stride: usize, window: &Window) -> Vec<bool> {
    (window.top..window.top + window.height)
        .flat_map(|row| {
            mask[row * stride + window.left..row * stride + window.left + window.width]
                .iter()
                .copied()
        })
        .collect()
}

/// Disagreeing pixels and the pixels compared for one glyph's box, or `None` when the box cannot
/// carry the proof.
fn score_glyph(ink: &[bool], glyph: &[bool], width: usize, height: usize) -> Option<(usize, usize)> {
    let area = glyph.iter().filter(|&&on| on).count();
    if area == 0 {
        return None;
    }
    let perimeter = (0..glyph.len())
        .filter(|&at| glyph[at] && !interior(glyph, width, height, at))
        .count();
    let stroke = 2.0 * area as f64 / perimeter.max(1) as f64;

    if stroke < MIN_STROKE_PIXELS {
        return None;
    }
    let tolerance = (stroke / 4.0) as usize;
    let margin = dilate(glyph, width, height, stroke.round() as usize);
    let (background, background_ink) = margin
        .iter()
        .zip(ink)
        .filter(|(near, _)| !**near)
        .fold((0usize, 0usize), |(all, inked), (_, &on)| {
            (all + 1, inked + usize::from(on))
        });
    if background == 0 || background_ink as f64 > MAX_BACKGROUND_INK * background as f64 {
        return None;
    }
    let near_ink = dilate(ink, width, height, tolerance);
    let near_glyph = dilate(glyph, width, height, tolerance);
    let misses = (0..glyph.len()).filter(|&at| glyph[at] && !near_ink[at]).count();
    let extras = (0..glyph.len())
        .filter(|&at| ink[at] && margin[at] && !near_glyph[at])
        .count();
    let beside = (0..glyph.len()).filter(|&at| ink[at] && margin[at]).count();
    Some((misses + extras, area + beside))
}

fn interior(mask: &[bool], width: usize, height: usize, at: usize) -> bool {
    let (x, y) = (at % width, at / width);
    x > 0
        && y > 0
        && x + 1 < width
        && y + 1 < height
        && mask[at - 1]
        && mask[at + 1]
        && mask[at - width]
        && mask[at + width]
}

fn dilate(mask: &[bool], width: usize, height: usize, radius: usize) -> Vec<bool> {
    let mut out = vec![false; mask.len()];
    for at in (0..mask.len()).filter(|&at| mask[at]) {
        let (x, y) = (at % width, at / width);
        let (left, right) = (x.saturating_sub(radius), (x + radius).min(width - 1));
        for row in y.saturating_sub(radius)..=(y + radius).min(height - 1) {
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

    macro_rules! fixture {
        ($name:literal) => {
            include_bytes!(concat!("../../../../tests/fixtures/pdf_layer_proof_2408/", $name))
        };
    }

    fn proven(bytes: &[u8], layer: &str) -> bool {
        verdict(bytes, layer).is_some_and(|verdict| verdict.proven())
    }

    fn worst(bytes: &[u8], layer: &str) -> f64 {
        verdict(bytes, layer)
            .expect("the page has glyphs large enough to compare")
            .worst
    }

    fn verdict(bytes: &[u8], layer: &str) -> Option<Verdict> {
        let doc = PdfDocument::from_bytes(bytes.to_vec()).unwrap();
        prove(&doc, 0, layer, &mut ProofBudget::new(usize::MAX / 4), || false)
    }

    #[test]
    fn a_layer_drawn_on_its_scan_is_proven_and_a_different_digit_is_not() {
        assert!(proven(MATCHING, "ប០ប"));
        assert!(!proven(MISLEADING, "ប១ប"));
        assert!(worst(MATCHING, "ប០ប") < MAX_DISAGREEMENT);
        assert!(worst(MISLEADING, "ប១ប") > MAX_DISAGREEMENT);
        assert!(proven(fixture!("large-matching.pdf"), "1234"));
        assert!(!proven(fixture!("large-wrong.pdf"), "1284"));
        assert!(worst(fixture!("large-wrong.pdf"), "1284") > 2.0 * MAX_DISAGREEMENT);
    }

    #[test]
    fn text_too_small_to_tell_digits_apart_is_not_proven_even_when_it_matches() {
        assert!(verdict(fixture!("small-matching.pdf"), "1234").is_none());
        assert!(verdict(fixture!("small-wrong.pdf"), "1284").is_none());
    }

    #[test]
    fn a_blurred_skewed_scan_is_not_proven() {
        let measured = worst(fixture!("blurred-skewed.pdf"), "1234");
        assert!(measured > 10.0 * MAX_DISAGREEMENT, "{measured}");
    }

    #[test]
    fn text_over_a_flat_dark_band_proves_nothing() {
        assert!(verdict(fixture!("band.pdf"), "1234").is_none());
    }

    #[test]
    fn a_layer_with_glyphs_that_reveal_nothing_is_not_proven() {
        assert!(verdict(fixture!("partly-mode7.pdf"), "1234").is_none());
        assert!(
            verdict(fixture!("large-matching.pdf"), "12345").is_none(),
            "layer text and page characters differ"
        );
    }

    #[test]
    fn a_page_without_invisible_text_has_nothing_to_prove() {
        assert!(verdict(VISIBLE_ONLY, "x").is_none());
    }

    #[test]
    fn a_layer_with_an_unmapped_glyph_is_refused_before_any_render() {
        assert!(verdict(MATCHING, "ប\u{FFFD}ប").is_none());
    }

    #[test]
    fn the_budget_caps_pages_and_pixels() {
        let mut budget = ProofBudget::new(usize::MAX / 4);
        assert!(!budget.charge(MAX_PAGE_PIXELS + 1.0));
        assert!(!budget.charge(f64::NAN));
        for _ in 0..MAX_DOCUMENT_PAGES {
            assert!(budget.charge(1.0));
        }
        assert!(!budget.charge(1.0));
    }
}
