use crate::geometry::Rect;
use tiny_skia::{Pixmap, Point, Transform};

pub(super) const MAX_PROBE_BYTES: usize = 1 << 20;
pub(super) const MAX_PROBE_PAGE_PIXELS: f32 = (1 << 20) as f32;
const MAX_GLYPHS: usize = 128;
const MAX_SAMPLES: usize = 64 * 1024;
const MIN_VISIBLE_CONTRAST: u8 = 51;
const MIN_CHANGED_PIXELS: usize = 2;

pub(super) struct TextContrastProbe {
    literal: Vec<u8>,
    glyphs: Vec<Rect>,
    seen: bool,
    visible: bool,
}

impl TextContrastProbe {
    pub(super) fn new(literal: &[u8], glyphs: &[Rect]) -> Option<Self> {
        if literal.len() > MAX_GLYPHS || glyphs.is_empty() || glyphs.len() > MAX_GLYPHS {
            return None;
        }
        Some(Self {
            literal: literal.to_vec(),
            glyphs: glyphs.to_vec(),
            seen: false,
            visible: false,
        })
    }

    pub(super) fn capture(&mut self, pixmap: &Pixmap, text: &[u8], base: Transform) -> Option<Vec<GlyphSample>> {
        if self.literal != text {
            return None;
        }
        if std::mem::replace(&mut self.seen, true) {
            self.visible = false;
            return None;
        }
        // The audited page has only one image before this show. A blank canvas cannot witness its background.
        if !pixmap
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| luminance(pixel) <= u8::MAX - MIN_VISIBLE_CONTRAST)
        {
            return None;
        }
        let mut samples = Vec::with_capacity(self.glyphs.len());
        let mut remaining = MAX_SAMPLES;
        let mut previous_right = 0;
        for glyph in &self.glyphs {
            let rect = glyph_pixels(*glyph, pixmap, base)?;
            if rect.x < previous_right {
                return None;
            }
            previous_right = rect.x + rect.width;
            let count = rect.width.checked_mul(rect.height)?;
            remaining = remaining.checked_sub(count)?;
            let before = luminance_samples(pixmap, rect).collect();
            samples.push(GlyphSample { rect, before });
        }
        Some(samples)
    }

    pub(super) fn observe(&mut self, pixmap: &Pixmap, samples: Vec<GlyphSample>) {
        self.visible = samples.into_iter().all(|sample| {
            sample
                .before
                .into_iter()
                .zip(luminance_samples(pixmap, sample.rect))
                .filter(|&(before, after)| before.abs_diff(after) >= MIN_VISIBLE_CONTRAST)
                .take(MIN_CHANGED_PIXELS)
                .count()
                == MIN_CHANGED_PIXELS
        });
    }

    pub(super) fn visible(&self) -> bool {
        self.seen && self.visible
    }
}

#[derive(Clone, Copy)]
struct PixelRect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

pub(super) struct GlyphSample {
    rect: PixelRect,
    before: Vec<u8>,
}

fn glyph_pixels(bounds: Rect, pixmap: &Pixmap, base: Transform) -> Option<PixelRect> {
    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return None;
    }
    // Keep the sample inside the glyph's horizontal advance, away from adjacent glyph ink.
    let inset = bounds.width / 8.0;
    let mut first = Point::from_xy(bounds.left() + inset, bounds.top());
    let mut last = Point::from_xy(bounds.right() - inset, bounds.bottom());
    base.map_point(&mut first);
    base.map_point(&mut last);
    let x0 = first.x.min(last.x).ceil();
    let x1 = first.x.max(last.x).floor();
    let y0 = first.y.min(last.y).ceil();
    let y1 = first.y.max(last.y).floor();
    if ![x0, x1, y0, y1].into_iter().all(f32::is_finite)
        || x0 < 0.0
        || y0 < 0.0
        || x1 > pixmap.width() as f32
        || y1 > pixmap.height() as f32
        || x1 <= x0
        || y1 <= y0
    {
        return None;
    }
    Some(PixelRect {
        x: x0 as usize,
        y: y0 as usize,
        width: (x1 - x0) as usize,
        height: (y1 - y0) as usize,
    })
}

fn luminance_samples(pixmap: &Pixmap, rect: PixelRect) -> impl Iterator<Item = u8> + '_ {
    let stride = pixmap.width() as usize * 4;
    (rect.y..rect.y + rect.height).flat_map(move |row| {
        let start = row * stride + rect.x * 4;
        pixmap.data()[start..start + rect.width * 4]
            .as_chunks::<4>()
            .0
            .iter()
            .map(luminance)
    })
}

fn luminance(rgb: &[u8; 4]) -> u8 {
    ((54 * u16::from(rgb[0]) + 183 * u16::from(rgb[1]) + 19 * u16::from(rgb[2])) >> 8) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::{Color, Paint};

    #[test]
    fn a_text_show_needs_visible_change_in_each_disjoint_glyph() {
        let glyphs = [Rect::new(2.0, 3.0, 6.0, 8.0), Rect::new(12.0, 3.0, 6.0, 8.0)];
        for paint_count in 0..=2 {
            let mut raster = Pixmap::new(20, 16).unwrap();
            raster.fill(Color::BLACK);
            let mut probe = TextContrastProbe::new(b"AB", &glyphs).unwrap();
            let before = probe.capture(&raster, b"AB", Transform::identity()).unwrap();
            let mut paint = Paint::default();
            paint.set_color(Color::WHITE);
            for glyph in &glyphs[..paint_count] {
                let region = tiny_skia::Rect::from_xywh(glyph.x, glyph.y, glyph.width, glyph.height).unwrap();
                raster.fill_rect(region, &paint, Transform::identity(), None);
            }
            probe.observe(&raster, before);
            assert_eq!(probe.visible(), paint_count == 2);
            assert!(probe.capture(&raster, b"AB", Transform::identity()).is_none());
            assert!(!probe.visible(), "an ambiguous second show cannot carry the proof");
        }
    }
}
