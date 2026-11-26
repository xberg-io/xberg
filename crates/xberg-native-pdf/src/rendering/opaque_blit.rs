//! A direct copy of an opaque, pre-resized gray or RGB image onto the canvas, replacing the
//! tiny-skia pattern blit only where it computes the same bytes.
//!
//! `draw_pixmap` always samples through a pattern, which runs tiny-skia's f32 (`highp`) pipeline
//! for every covered pixel: two thirds of rendering a full-page scan. For an opaque source drawn
//! with nearest sampling, normal blending and full opacity at a translation within a quarter
//! pixel of whole pixels, each covered pixel takes the source pixel at the same integer offset,
//! and the clip mask blends it with the arithmetic reproduced here. The samples are widened to
//! RGBA as they are copied, so no RGBA copy of the image is held beside the canvas.

use tiny_skia::{BlendMode, FilterQuality, Mask, Pixmap, PixmapPaint, Transform};

/// The largest distance from a whole pixel at which a translation still covers and samples the
/// same pixels as the rounded one: pixel centres sit half a pixel from either rounding boundary.
const MAX_SUBPIXEL_OFFSET: f32 = 0.25;

/// tiny-skia's `highp` 8-bit to float factor (`load_8888`).
const UNIT_FACTOR: f32 = 1.0 / 255.0;

/// Copy `samples`, gray (`components` 1) or RGB (3) pixels of `size`, onto `pixmap` at
/// `transform` as `draw_pixmap(0, 0, .., paint, transform, clip_mask)` draws their opaque RGBA
/// widening. `false`, leaving `pixmap` untouched, when the draw is not one this copy reproduces.
pub(super) fn blit_opaque_translated(
    pixmap: &mut Pixmap,
    (samples, components): (&[u8], u8),
    (width, height): (u32, u32),
    paint: &PixmapPaint,
    transform: Transform,
    clip_mask: Option<&Mask>,
) -> bool {
    let Some((left, top)) = whole_pixel_offset(paint, transform) else {
        return false;
    };
    let channels = usize::from(components);
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(channels));
    if !matches!(components, 1 | 3) || expected != Some(samples.len()) {
        return false;
    }
    if clip_mask.is_some_and(|mask| (mask.width(), mask.height()) != (pixmap.width(), pixmap.height())) {
        return false;
    }
    let canvas_width = i64::from(pixmap.width());
    let x0 = left.max(0);
    let x1 = (left + i64::from(width)).min(canvas_width);
    let y0 = top.max(0);
    let y1 = (top + i64::from(height)).min(i64::from(pixmap.height()));
    if x0 >= x1 || y0 >= y1 {
        return true;
    }
    let mask = clip_mask.map(Mask::data);
    let data = pixmap.data_mut();
    for y in y0..y1 {
        let source_row = ((y - top) as usize * width as usize + (x0 - left) as usize) * channels;
        let target_row = (y * canvas_width + x0) as usize;
        let span = (x1 - x0) as usize;
        let source = &samples[source_row..source_row + span * channels];
        let target = &mut data[target_row * 4..(target_row + span) * 4];
        let coverage = mask.map(|mask| &mask[target_row..target_row + span]);
        blit_span(target, source, channels, coverage);
    }
    true
}

/// One row of the draw: full coverage (or no clip) takes the widened source, none keeps the
/// canvas, and a partial clip edge blends as tiny-skia does.
fn blit_span(target: &mut [u8], source: &[u8], channels: usize, coverage: Option<&[u8]>) {
    for (index, (dst, src)) in target
        .chunks_exact_mut(4)
        .zip(source.chunks_exact(channels))
        .enumerate()
    {
        let pixel = if channels == 1 {
            [src[0], src[0], src[0], u8::MAX]
        } else {
            [src[0], src[1], src[2], u8::MAX]
        };
        match coverage.map_or(u8::MAX, |coverage| coverage[index]) {
            0 => {}
            u8::MAX => dst.copy_from_slice(&pixel),
            cover => {
                for channel in 0..4 {
                    dst[channel] = masked_source_over(pixel[channel], u8::MAX, dst[channel], cover);
                }
            }
        }
    }
}

/// The whole-pixel offset a nearest, opaque, normal-blend, full-opacity pure translation draws
/// at, or `None` for any other draw.
fn whole_pixel_offset(paint: &PixmapPaint, transform: Transform) -> Option<(i64, i64)> {
    if paint.opacity != 1.0
        || paint.blend_mode != BlendMode::SourceOver
        || paint.quality != FilterQuality::Nearest
        || transform.sx != 1.0
        || transform.sy != 1.0
        || transform.kx != 0.0
        || transform.ky != 0.0
    {
        return None;
    }
    let whole = |offset: f32| {
        let rounded = offset.round();
        (offset.is_finite() && (offset - rounded).abs() < MAX_SUBPIXEL_OFFSET && rounded.abs() < 1.0e7)
            .then_some(rounded as i64)
    };
    Some((whole(transform.tx)?, whole(transform.ty)?))
}

/// One channel of tiny-skia's `highp` `mask_u8` then `source_over` then `store`: the source
/// scaled by the coverage, over the destination, rounded half to even.
fn masked_source_over(source: u8, source_alpha: u8, destination: u8, coverage: u8) -> u8 {
    let cover = f32::from(coverage) / 255.0;
    let s = f32::from(source) * UNIT_FACTOR * cover;
    let sa = f32::from(source_alpha) * UNIT_FACTOR * cover;
    let d = f32::from(destination) * UNIT_FACTOR;
    let blended = d * (1.0 - sa) + s;
    (blended.clamp(0.0, 1.0) * 255.0).round_ties_even() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::{FillRule, IntSize, PathBuilder};

    fn samples(width: u32, height: u32, components: u8, seed: u32) -> Vec<u8> {
        let mut state = seed;
        (0..width * height * u32::from(components))
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state >> 24) as u8
            })
            .collect()
    }

    fn widened(samples: &[u8], components: u8) -> Vec<u8> {
        samples
            .chunks_exact(usize::from(components))
            .flat_map(|pixel| match pixel {
                [gray] => [*gray, *gray, *gray, u8::MAX],
                [r, g, b] => [*r, *g, *b, u8::MAX],
                _ => unreachable!(),
            })
            .collect()
    }

    fn canvas(width: u32, height: u32) -> Pixmap {
        let mut pixmap = Pixmap::new(width, height).unwrap();
        for (index, pixel) in pixmap.data_mut().chunks_exact_mut(4).enumerate() {
            let value = (index * 37 % 251) as u8;
            pixel.copy_from_slice(&[value / 2, value / 3, value / 4, value]);
        }
        pixmap
    }

    fn clip(width: u32, height: u32, rect: (f32, f32, f32, f32)) -> Mask {
        let mut mask = Mask::new(width, height).unwrap();
        let path = PathBuilder::from_rect(tiny_skia::Rect::from_ltrb(rect.0, rect.1, rect.2, rect.3).unwrap());
        mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
        mask
    }

    fn nearest_paint() -> PixmapPaint {
        PixmapPaint {
            quality: FilterQuality::Nearest,
            ..PixmapPaint::default()
        }
    }

    #[test]
    fn full_and_empty_coverage_match_tiny_skia_for_every_byte_pair() {
        for source in 0..=u8::MAX {
            for destination in 0..=u8::MAX {
                assert_eq!(masked_source_over(source, u8::MAX, destination, u8::MAX), source);
                assert_eq!(masked_source_over(source, u8::MAX, destination, 0), destination);
            }
        }
    }

    #[test]
    fn direct_copy_matches_draw_pixmap_bit_for_bit() {
        let (width, height) = (61, 37);
        let masks = [
            None,
            Some(clip(80, 50, (2.3, 1.7, 57.6, 33.2))),
            Some(clip(80, 50, (-4.0, -3.0, 90.0, 60.0))),
        ];
        for components in [1, 3] {
            let source = samples(width, height, components, 7);
            for translate in [
                (0.0, 0.0),
                (3.0, 2.0),
                (5.2, 7.9),
                (-2.1, -1.0),
                (30.0, 20.0),
                (0.0, 3.3e-5),
            ] {
                for mask in &masks {
                    let mut expected = canvas(80, 50);
                    let rgba = widened(&source, components);
                    let image = Pixmap::from_vec(rgba, IntSize::from_wh(width, height).unwrap()).unwrap();
                    let transform = Transform::from_translate(translate.0, translate.1);
                    expected.draw_pixmap(0, 0, image.as_ref(), &nearest_paint(), transform, mask.as_ref());
                    let mut actual = canvas(80, 50);
                    assert!(blit_opaque_translated(
                        &mut actual,
                        (&source, components),
                        (width, height),
                        &nearest_paint(),
                        transform,
                        mask.as_ref(),
                    ));
                    assert_eq!(
                        actual.data(),
                        expected.data(),
                        "{components} channels, translate {translate:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn draws_it_cannot_reproduce_are_declined() {
        let source = samples(4, 4, 3, 3);
        let mut pixmap = canvas(8, 8);
        let untouched = pixmap.data().to_vec();
        let half = Transform::from_translate(0.5, 0.0);
        assert!(!blit_opaque_translated(
            &mut pixmap,
            (&source, 3),
            (4, 4),
            &nearest_paint(),
            half,
            None
        ));
        let bilinear = PixmapPaint {
            quality: FilterQuality::Bilinear,
            ..PixmapPaint::default()
        };
        let identity = Transform::identity();
        assert!(!blit_opaque_translated(
            &mut pixmap,
            (&source, 3),
            (4, 4),
            &bilinear,
            identity,
            None
        ));
        let translucent = PixmapPaint {
            opacity: 0.5,
            ..nearest_paint()
        };
        assert!(!blit_opaque_translated(
            &mut pixmap,
            (&source, 3),
            (4, 4),
            &translucent,
            identity,
            None
        ));
        assert!(!blit_opaque_translated(
            &mut pixmap,
            (&source, 4),
            (3, 4),
            &nearest_paint(),
            identity,
            None
        ));
        assert_eq!(pixmap.data(), untouched.as_slice());
    }
}
