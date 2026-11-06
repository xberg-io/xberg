//! Source-raster veto for a word-gap caption certificate.

use std::io::Cursor;

use ahash::AHashSet;
use image::{ColorType, ImageDecoder, RgbImage};

use super::{Rect, SourceElement};
use crate::extractors::security::SecurityLimits;

const MAX_REFERENCE_WORDS: usize = 4;
const MAX_WORD_PIXELS: usize = 4096;
const MAX_CROP_PIXELS: usize = 512 * 1024;
const MAX_RASTER_PIXELS: usize = 10 * 1024 * 1024;
const MAX_DECODER_BYTES: usize = 64 * 1024 * 1024;
const SCRATCH_BYTES: usize = 2 * 1024 * 1024;
const CROP_HALO: u32 = 8;
// ~keep Test independent silhouettes at increasing contrast and require each fragment pixel
// to have support from the brighter aligned reference mask.
const SHADE_PAIRS: [(u8, u8); 3] = [(128, 208), (192, 224), (224, 248)];
const MIN_LITERAL_IOU_PERCENT: usize = 80;
const MIN_COMPONENT_IOU_PERCENT: usize = 85;

type Pixel = (u32, u32);
type Mask = AHashSet<Pixel>;

struct Edge {
    bounds: Rect,
    references: Option<Vec<Rect>>,
}

impl Edge {
    fn new(word: &SourceElement<'_>, words: &[SourceElement<'_>]) -> Self {
        let references: Vec<_> = words
            .iter()
            .filter(|other| other.text == word.text && !other.bounds.intersects(word.bounds))
            .map(|other| other.bounds)
            .take(MAX_REFERENCE_WORDS + 1)
            .collect();
        Self {
            bounds: word.bounds,
            references: (references.len() <= MAX_REFERENCE_WORDS).then_some(references),
        }
    }
}

struct Cut {
    area: Rect,
    left: Edge,
    right: Edge,
}

pub(crate) struct CaptionPaintProof {
    crop: Rect,
    min_line_height: u32,
    remaining_work: usize,
    cuts: Vec<Cut>,
}

impl CaptionPaintProof {
    pub(super) fn new(crop: Rect, min_line_height: u32, remaining_work: usize) -> Self {
        Self {
            crop,
            min_line_height,
            remaining_work,
            cuts: Vec::new(),
        }
    }

    pub(super) fn push(
        &mut self,
        line: Rect,
        left: &SourceElement<'_>,
        right: &SourceElement<'_>,
        words: &[SourceElement<'_>],
    ) {
        self.cuts.push(Cut {
            area: Rect {
                left: left.bounds.right,
                top: line.top,
                right: right.bounds.left,
                bottom: line.bottom,
            },
            left: Edge::new(left, words),
            right: Edge::new(right, words),
        });
    }

    pub(crate) fn certify(
        &self,
        bytes: &[u8],
        frame: (u32, u32),
        retained_bytes: usize,
        limits: &SecurityLimits,
    ) -> bool {
        let Some((crop, mut work)) = self.crop_work(frame, limits) else {
            return false;
        };
        let Some(raster) = decode_opaque_rgb(bytes, frame, retained_bytes, limits) else {
            return false;
        };
        self.no_ruling(&raster, crop) && self.cuts.iter().all(|cut| cut.certifies(&raster, &mut work))
    }

    pub(super) fn crop_work(&self, frame: (u32, u32), limits: &SecurityLimits) -> Option<(Rect, usize)> {
        let crop = self.halo_crop(frame)?;
        let pixels = area(crop).filter(|pixels| *pixels <= MAX_CROP_PIXELS)?;
        let work = self
            .remaining_work
            .min(limits.max_iterations)
            .checked_sub(pixels.checked_mul(5)?)?;
        Some((crop, work))
    }

    fn halo_crop(&self, frame: (u32, u32)) -> Option<Rect> {
        let crop = Rect {
            left: self.crop.left.checked_sub(CROP_HALO)?,
            top: self.crop.top.checked_sub(CROP_HALO)?,
            right: self.crop.right.checked_add(CROP_HALO)?,
            bottom: self.crop.bottom.checked_add(CROP_HALO)?,
        };
        crop.within_frame(frame).then_some(crop)
    }

    // ~keep Reject text-height dark strokes and longer low-contrast horizontal rules.
    fn no_ruling(&self, rgb: &RgbImage, crop: Rect) -> bool {
        for y in crop.top..crop.bottom {
            if (crop.left..crop.right).any(|x| {
                let [red, green, blue] = rgb.get_pixel(x, y).0;
                red != green || green != blue
            }) {
                return false;
            }
            if run_reaches(
                (crop.left..crop.right).map(|x| shade(rgb, x, y)),
                192,
                self.min_line_height,
            ) || run_reaches(
                (crop.left..crop.right).map(|x| shade(rgb, x, y)),
                248,
                self.min_line_height * 3,
            ) {
                return false;
            }
        }
        !(crop.left..crop.right).any(|x| {
            run_reaches(
                (crop.top..crop.bottom).map(|y| shade(rgb, x, y)),
                248,
                self.min_line_height,
            )
        })
    }
}

fn decode_opaque_rgb(
    bytes: &[u8],
    (width, height): (u32, u32),
    retained_bytes: usize,
    limits: &SecurityLimits,
) -> Option<RgbImage> {
    let pixels = (width as usize).checked_mul(height as usize)?;
    if pixels == 0 || pixels > MAX_RASTER_PIXELS || bytes.get(24..26)? != [8, 2] {
        return None;
    }
    let output = pixels.checked_mul(3)?;
    let decoder_bytes = limits
        .max_content_size
        .checked_sub(retained_bytes)?
        .checked_sub(output)?
        .checked_sub(SCRATCH_BYTES)?
        .min(MAX_DECODER_BYTES);
    let mut decode_limits = image::Limits::default();
    decode_limits.max_image_width = Some(width);
    decode_limits.max_image_height = Some(height);
    decode_limits.max_alloc = Some(decoder_bytes.try_into().ok()?);
    let decoder = image::codecs::png::PngDecoder::with_limits(Cursor::new(bytes), decode_limits).ok()?;
    if decoder.dimensions() != (width, height) || decoder.color_type() != ColorType::Rgb8 || decoder.is_apng().ok()? {
        return None;
    }
    let mut output = vec![0; output];
    decoder.read_image(&mut output).ok()?;
    RgbImage::from_raw(width, height, output)
}

fn shade(rgb: &RgbImage, x: u32, y: u32) -> u8 {
    rgb.get_pixel(x, y).0[0]
}

fn run_reaches(shades: impl Iterator<Item = u8>, cutoff: u8, limit: u32) -> bool {
    let mut run = 0;
    for pixel in shades {
        run = if pixel <= cutoff { run + 1 } else { 0 };
        if run >= limit {
            return true;
        }
    }
    false
}

fn area(rect: Rect) -> Option<usize> {
    (rect.right.checked_sub(rect.left)? as usize).checked_mul(rect.bottom.checked_sub(rect.top)? as usize)
}

fn mask(rgb: &RgbImage, rect: Rect, cutoff: u8, work: &mut usize) -> Option<Mask> {
    let pixels = area(rect).filter(|pixels| *pixels <= MAX_WORD_PIXELS)?;
    // ~keep Charge the scan, connected component walk, and a bounded number of mask comparisons.
    *work = work.checked_sub(pixels.checked_mul(16)?)?;
    Some(
        (rect.top..rect.bottom)
            .flat_map(|y| {
                (rect.left..rect.right)
                    .filter_map(move |x| (shade(rgb, x, y) <= cutoff).then_some((x - rect.left, y - rect.top)))
            })
            .collect(),
    )
}

fn components(mut ink: Mask) -> Vec<Mask> {
    let mut all = Vec::new();
    let starts: Vec<_> = ink.iter().copied().collect();
    for start in starts {
        if !ink.remove(&start) {
            continue;
        }
        let mut queue = vec![start];
        let mut group = Mask::default();
        while let Some((x, y)) = queue.pop() {
            group.insert((x, y));
            queue.extend(adjacent(x, y).filter(|pixel| ink.remove(pixel)));
        }
        all.push(group);
    }
    all
}

fn adjacent(x: u32, y: u32) -> impl Iterator<Item = Pixel> {
    (x.saturating_sub(1)..=x + 1).flat_map(move |x| (y.saturating_sub(1)..=y + 1).map(move |y| (x, y)))
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Assignment {
    neighbor: Rect,
    reference: Rect,
}

impl Cut {
    fn certifies(&self, rgb: &RgbImage, work: &mut usize) -> bool {
        let mut owner = None;
        let mut shades = 0;
        for (cutoff, support) in SHADE_PAIRS {
            match self.assign_at_shade(rgb, cutoff, support, work) {
                Some(None) => {}
                Some(Some(assignment)) if owner.is_none_or(|previous| previous == assignment) => {
                    owner = Some(assignment);
                    shades += 1;
                }
                _ => return false,
            }
        }
        owner.is_none() || shades >= 2
    }

    fn assign_at_shade(&self, rgb: &RgbImage, cutoff: u8, support: u8, work: &mut usize) -> Option<Option<Assignment>> {
        let mut unexplained = components(mask(rgb, self.area, cutoff, work)?)
            .into_iter()
            .filter(|group| !self.word_fringe(rgb, group, cutoff));
        let Some(fragment) = unexplained.next() else {
            return Some(None);
        };
        if unexplained.next().is_some() {
            return None;
        }
        let mut owner = None;
        for edge in [&self.left, &self.right] {
            for reference in edge.references.as_deref()? {
                if self.matches_reference(rgb, &fragment, edge.bounds, *reference, (cutoff, support), work)?
                    && owner
                        .replace(Assignment {
                            neighbor: edge.bounds,
                            reference: *reference,
                        })
                        .is_some()
                {
                    return None;
                }
            }
        }
        owner.map(Some)
    }

    fn word_fringe(&self, rgb: &RgbImage, group: &Mask, cutoff: u8) -> bool {
        let Rect { left, right, top, .. } = self.area;
        group.iter().all(|&(x, y)| {
            x == 0
                && (self.left.bounds.top..self.left.bounds.bottom).contains(&(top + y))
                && shade(rgb, left - 1, top + y) <= cutoff
        }) || group.iter().all(|&(x, y)| {
            x + 1 == right - left
                && (self.right.bounds.top..self.right.bounds.bottom).contains(&(top + y))
                && shade(rgb, right, top + y) <= cutoff
        })
    }

    fn matches_reference(
        &self,
        rgb: &RgbImage,
        fragment: &Mask,
        neighbor: Rect,
        reference: Rect,
        shades: (u8, u8),
        work: &mut usize,
    ) -> Option<bool> {
        let bounds = self.fragment_bounds(fragment)?;
        let reconstructed = neighbor.union(bounds);
        if reconstructed.top != neighbor.top
            || reconstructed.bottom != neighbor.bottom
            || reconstructed.right - reconstructed.left != reference.right - reference.left
            || reconstructed.bottom - reconstructed.top != reference.bottom - reference.top
        {
            return Some(false);
        }
        let literal_ink = mask(rgb, reconstructed, shades.0, work)?;
        let reference_ink = mask(rgb, reference, shades.0, work)?;
        if !iou_at_least(&literal_ink, &reference_ink, MIN_LITERAL_IOU_PERCENT) {
            return Some(false);
        }
        let support = mask(rgb, reference, shades.1, work)?;
        let translated: Mask = fragment
            .iter()
            .map(|&(x, y)| {
                (
                    self.area.left + x - reconstructed.left,
                    self.area.top + y - reconstructed.top,
                )
            })
            .collect();
        if !translated.is_subset(&support) {
            return Some(false);
        }
        Some(
            components(reference_ink)
                .iter()
                .filter(|component| iou_at_least(&translated, component, MIN_COMPONENT_IOU_PERCENT))
                .count()
                == 1,
        )
    }

    fn fragment_bounds(&self, fragment: &Mask) -> Option<Rect> {
        fragment
            .iter()
            .map(|&(x, y)| {
                let x = x + self.area.left;
                let y = y + self.area.top;
                Rect {
                    left: x,
                    top: y,
                    right: x + 1,
                    bottom: y + 1,
                }
            })
            .reduce(Rect::union)
    }
}

fn iou_at_least(a: &Mask, b: &Mask, percent: usize) -> bool {
    let intersection = a.intersection(b).count();
    intersection > 0 && intersection * 100 >= percent * (a.len() + b.len() - intersection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageEncoder, Rgb};

    fn rect(left: u32, top: u32, right: u32, bottom: u32) -> Rect {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    fn repeated_literal() -> (CaptionPaintProof, RgbImage) {
        let left = SourceElement {
            text: "an",
            bounds: rect(12, 16, 18, 21),
        };
        let right = SourceElement {
            text: "sun",
            bounds: rect(26, 16, 31, 21),
        };
        let reference = SourceElement {
            text: "sun",
            bounds: rect(20, 30, 29, 35),
        };
        let mut proof = CaptionPaintProof::new(rect(12, 16, 31, 35), 5, SecurityLimits::default().max_iterations);
        proof.push(rect(12, 16, 31, 21), &left, &right, &[reference]);
        let mut raster = RgbImage::from_pixel(48, 44, Rgb([255; 3]));
        for (x, y) in [(0, 1), (1, 1), (1, 2), (4, 1), (5, 1), (7, 1), (8, 1), (8, 2)] {
            raster.put_pixel(22 + x, 16 + y, Rgb([15; 3]));
            raster.put_pixel(20 + x, 30 + y, Rgb([15; 3]));
        }
        (proof, raster)
    }

    fn certify(proof: &CaptionPaintProof, raster: &RgbImage, limits: &SecurityLimits) -> bool {
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(raster.as_raw(), raster.width(), raster.height(), ColorType::Rgb8.into())
            .unwrap();
        proof.certify(&png, raster.dimensions(), png.capacity(), limits)
    }

    #[test]
    fn unique_repeated_word_can_explain_ink_but_extra_detached_or_touching_marks_cannot() {
        let (proof, raster) = repeated_literal();
        let limits = SecurityLimits::default();
        assert!(certify(&proof, &raster, &limits));
        for pixels in [&[(19, 18)][..], &[(22, 19), (23, 19)][..]] {
            let mut changed = raster.clone();
            for &(x, y) in pixels {
                changed.put_pixel(x, y, Rgb([176; 3]));
            }
            assert!(!certify(&proof, &changed, &limits), "unowned pixels {pixels:?}");
        }
    }

    #[test]
    fn refuses_a_verified_caption_under_a_nested_only_iteration_budget() {
        use super::super::super::caption_conversion::proof_limits;
        use crate::core::config::{ExtractionConfig, OcrConfig};

        let (proof, raster) = repeated_literal();
        let config = ExtractionConfig::default();
        assert!(certify(&proof, &raster, &proof_limits(&config, &OcrConfig::default())));
        let ocr = OcrConfig {
            security_limits: Some(SecurityLimits {
                max_iterations: 10,
                ..SecurityLimits::default()
            }),
            ..Default::default()
        };
        assert!(!certify(&proof, &raster, &proof_limits(&config, &ocr)));
    }

    #[test]
    fn refuses_independent_painted_rule_chromatic_ink_and_insufficient_budget() {
        let (proof, raster) = repeated_literal();
        let limits = SecurityLimits::default();
        let mut changed = raster.clone();
        for y in 15..20 {
            changed.put_pixel(34, y, Rgb([220; 3]));
        }
        assert!(!certify(&proof, &changed, &limits));

        let mut changed = raster.clone();
        changed.put_pixel(34, 15, Rgb([220, 220, 219]));
        assert!(!certify(&proof, &changed, &limits));

        let mut alpha = Vec::new();
        let rgba = image::DynamicImage::ImageRgb8(raster.clone()).into_rgba8();
        image::codecs::png::PngEncoder::new(&mut alpha)
            .write_image(rgba.as_raw(), rgba.width(), rgba.height(), ColorType::Rgba8.into())
            .unwrap();
        assert!(!proof.certify(&alpha, raster.dimensions(), alpha.capacity(), &limits));

        let low = SecurityLimits {
            max_content_size: SCRATCH_BYTES,
            ..limits
        };
        assert!(!certify(&proof, &raster, &low));
    }
}
