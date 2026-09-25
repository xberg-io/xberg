//! Flatten shaded table rows before recognition (xberg-io/xberg#1785).
//!
//! Financial statements mark subtotal and total rows with a fill: dark text on a light grey
//! band, or white text on a mid grey or dark band. Tesseract thresholds the whole page once,
//! and a band whose fill sits between paper and ink does not survive that threshold: the
//! label and every value of the row are dropped together. Nothing page-wide can fix this,
//! because the page's polarity and threshold are right for every other row.
//!
//! This pass works per horizontal band. Dark bands (light text on a dark fill) are inverted
//! first. Then every band whose median grey lies between paper and ink is stretched so its
//! fill becomes white and its ink black, after deciding the band's own polarity from the
//! pixels far above and far below the fill. Rows outside such bands are not touched, so a
//! page without shaded bands comes out byte-identical.

/// A pixel darker than this counts as ink when looking for dark bands.
const DARK_PIXEL_MAX: u8 = 110;
/// A row is part of a dark band when at least this fraction of its pixels is dark.
const DARK_BAND_MIN_FRACTION: f64 = 0.40;
/// A dark band's columns: those whose pixels inside the band are dark at least this often.
const DARK_BAND_COLUMN_MIN_FRACTION: f64 = 0.5;
/// Rows and columns added around a dark band before inverting it, so anti-aliased edges flip too.
const DARK_BAND_PAD: usize = 2;
/// A shaded band's median grey lies strictly between these two values: darker is a rule or a
/// dark band, lighter is paper.
const SHADED_MEDIAN_MIN: u8 = 60;
const SHADED_MEDIAN_MAX: u8 = 225;
/// Two adjacent rows belong to the same band when their medians differ by at most this much.
const SHADED_MEDIAN_STEP_MAX: i32 = 25;
/// Rows skipped at the top and bottom of a band when judging its polarity: they hold the rules.
const BAND_EDGE_ROWS: usize = 4;
/// A pixel this far from the fill, on either side, votes for the band's polarity.
const POLARITY_MARGIN: i32 = 40;
/// Below this fill-to-ink distance the band carries no text worth stretching.
const MIN_FILL_INK_CONTRAST: i32 = 30;
/// The ink level of a band is this percentile of its pixels.
const INK_PERCENTILE: f64 = 0.02;
/// Bands shorter than `max(MIN_BAND_HEIGHT_PX, height / MIN_BAND_HEIGHT_DIVISOR)` are noise.
const MIN_BAND_HEIGHT_PX: usize = 8;
const MIN_BAND_HEIGHT_DIVISOR: usize = 400;
/// Medians and polarity votes use the middle of the page, between these fractions of the width.
const CORE_LEFT_EIGHTHS: usize = 1;
const CORE_RIGHT_EIGHTHS: usize = 7;

/// Flatten shaded rows in an 8-bit RGB buffer of `width` x `height` pixels.
///
/// Returns a grayscale image encoded as RGB (equal channels), the form the OCR preprocessor
/// and Tesseract both accept. A buffer whose length does not match the dimensions is returned
/// unchanged.
pub(crate) fn flatten_shaded_rows_rgb(rgb: &[u8], width: u32, height: u32) -> Vec<u8> {
    let (width, height) = (width as usize, height as usize);
    if width == 0 || height == 0 || rgb.len() != width * height * 3 {
        return rgb.to_vec();
    }
    let mut gray: Vec<u8> = rgb.chunks_exact(3).map(|px| luma(px[0], px[1], px[2])).collect();
    flatten_shaded_rows(&mut gray, width, height);
    gray.iter().flat_map(|&value| [value, value, value]).collect()
}

/// Rec. 601 luma, the same weights Leptonica's RGB-to-gray conversion uses.
fn luma(r: u8, g: u8, b: u8) -> u8 {
    (0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b)).round() as u8
}

/// Flatten shaded rows in place in an 8-bit grayscale buffer.
pub(crate) fn flatten_shaded_rows(gray: &mut [u8], width: usize, height: usize) {
    if width == 0 || height == 0 || gray.len() != width * height {
        return;
    }
    for band in find_dark_bands(gray, width, height) {
        invert_region(gray, width, band);
    }
    let core = core_columns(width);
    for (y0, y1) in find_shaded_bands(gray, width, height, core) {
        flatten_band(gray, width, y0, y1, core);
    }
}

/// A rectangle of rows `y0..y1` and columns `x0..x1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Region {
    y0: usize,
    y1: usize,
    x0: usize,
    x1: usize,
}

fn min_band_height(height: usize) -> usize {
    MIN_BAND_HEIGHT_PX.max(height / MIN_BAND_HEIGHT_DIVISOR)
}

fn core_columns(width: usize) -> (usize, usize) {
    (width * CORE_LEFT_EIGHTHS / 8, width * CORE_RIGHT_EIGHTHS / 8)
}

/// Runs of rows where at least `DARK_BAND_MIN_FRACTION` of the pixels are dark, each with the
/// column span that is dark inside the run, padded by `DARK_BAND_PAD`.
fn find_dark_bands(gray: &[u8], width: usize, height: usize) -> Vec<Region> {
    let min_height = min_band_height(height);
    let row_is_dark: Vec<bool> = (0..height)
        .map(|y| {
            let row = &gray[y * width..(y + 1) * width];
            let dark = row.iter().filter(|&&v| v < DARK_PIXEL_MAX).count();
            dark as f64 >= DARK_BAND_MIN_FRACTION * width as f64
        })
        .collect();

    let mut bands = Vec::new();
    let mut y = 0;
    while y < height {
        if !row_is_dark[y] {
            y += 1;
            continue;
        }
        let y0 = y;
        while y < height && row_is_dark[y] {
            y += 1;
        }
        if y - y0 < min_height {
            continue;
        }
        let rows = y - y0;
        let dark_columns: Vec<usize> = (0..width)
            .filter(|&x| {
                let dark = (y0..y).filter(|&yy| gray[yy * width + x] < DARK_PIXEL_MAX).count();
                dark as f64 > DARK_BAND_COLUMN_MIN_FRACTION * rows as f64
            })
            .collect();
        let (x0, x1) = match (dark_columns.first(), dark_columns.last()) {
            (Some(&first), Some(&last)) => (first, last + 1),
            _ => (0, width),
        };
        bands.push(Region {
            y0: y0.saturating_sub(DARK_BAND_PAD),
            y1: (y + DARK_BAND_PAD).min(height),
            x0: x0.saturating_sub(DARK_BAND_PAD),
            x1: (x1 + DARK_BAND_PAD).min(width),
        });
    }
    bands
}

fn invert_region(gray: &mut [u8], width: usize, region: Region) {
    for y in region.y0..region.y1 {
        for value in &mut gray[y * width + region.x0..y * width + region.x1] {
            *value = 255 - *value;
        }
    }
}

/// Runs of rows whose median grey across the page core lies between paper and ink, split
/// where the fill changes between two adjacent rows.
fn find_shaded_bands(gray: &[u8], width: usize, height: usize, core: (usize, usize)) -> Vec<(usize, usize)> {
    let (x0, x1) = core;
    if x1 <= x0 {
        return Vec::new();
    }
    let min_height = min_band_height(height);
    let medians: Vec<u8> = (0..height)
        .map(|y| median(gray[y * width + x0..y * width + x1].iter().copied()))
        .collect();
    let shaded = |y: usize| medians[y] > SHADED_MEDIAN_MIN && medians[y] < SHADED_MEDIAN_MAX;

    let mut bands = Vec::new();
    let mut y = 0;
    while y < height {
        if !shaded(y) {
            y += 1;
            continue;
        }
        let y0 = y;
        y += 1;
        while y < height
            && shaded(y)
            && (i32::from(medians[y]) - i32::from(medians[y - 1])).abs() <= SHADED_MEDIAN_STEP_MAX
        {
            y += 1;
        }
        if y - y0 >= min_height {
            bands.push((y0, y));
        }
    }
    bands
}

/// Stretch one band so its fill becomes white and its ink black, inverting it first when the
/// text is lighter than the fill.
fn flatten_band(gray: &mut [u8], width: usize, y0: usize, y1: usize, core: (usize, usize)) {
    let span = y0 * width..y1 * width;
    let mut fill = i32::from(median(gray[span.clone()].iter().copied()));

    let (cx0, cx1) = core;
    let core_rows = (y0 + BAND_EDGE_ROWS)..(y1.saturating_sub(BAND_EDGE_ROWS));
    let (mut lighter, mut darker) = (0usize, 0usize);
    for y in core_rows {
        for &value in &gray[y * width + cx0..y * width + cx1] {
            let value = i32::from(value);
            if value > fill + POLARITY_MARGIN {
                lighter += 1;
            } else if value < fill - POLARITY_MARGIN {
                darker += 1;
            }
        }
    }
    let inverted = lighter > darker;
    if inverted {
        for value in &mut gray[span.clone()] {
            *value = 255 - *value;
        }
        fill = i32::from(median(gray[span.clone()].iter().copied()));
    }

    let ink = i32::from(percentile(gray[span.clone()].iter().copied(), INK_PERCENTILE));
    if fill - ink < MIN_FILL_INK_CONTRAST {
        if inverted {
            for value in &mut gray[span] {
                *value = 255 - *value;
            }
        }
        return;
    }
    let scale = 255.0 / (fill - ink) as f32;
    for value in &mut gray[span] {
        let stretched = ((i32::from(*value) - ink) as f32 * scale).round();
        *value = stretched.clamp(0.0, 255.0) as u8;
    }
}

/// Exact median of 8-bit values through a histogram; 255 for an empty input.
fn median(values: impl Iterator<Item = u8>) -> u8 {
    percentile(values, 0.5)
}

/// The value at `fraction` of the sorted input (0.0 is the minimum, 1.0 the maximum),
/// through a histogram; 255 for an empty input.
fn percentile(values: impl Iterator<Item = u8>, fraction: f64) -> u8 {
    let mut histogram = [0usize; 256];
    let mut count = 0usize;
    for value in values {
        histogram[usize::from(value)] += 1;
        count += 1;
    }
    if count == 0 {
        return 255;
    }
    let target = ((count - 1) as f64 * fraction).floor() as usize;
    let mut seen = 0usize;
    for (value, &bucket) in histogram.iter().enumerate() {
        seen += bucket;
        if seen > target {
            return value as u8;
        }
    }
    255
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: usize = 400;
    const HEIGHT: usize = 300;

    /// A white page with two shaded bands the way a table draws them: a light grey band
    /// (fill 168) with dark bold glyph blobs, and a dark band (fill 77) with white glyph blobs.
    /// The blobs are 12 px squares, one every 40 px across the core of the page.
    fn shaded_page() -> Vec<u8> {
        let mut gray = vec![255u8; WIDTH * HEIGHT];
        paint_band(&mut gray, 100, 140, 168, 30);
        paint_band(&mut gray, 200, 240, 77, 255);
        gray
    }

    fn paint_band(gray: &mut [u8], y0: usize, y1: usize, fill: u8, ink: u8) {
        for y in y0..y1 {
            for x in 0..WIDTH {
                gray[y * WIDTH + x] = fill;
            }
        }
        for x in (60..WIDTH - 60).step_by(40) {
            for y in y0 + 14..y0 + 26 {
                for xx in x..x + 12 {
                    gray[y * WIDTH + xx] = ink;
                }
            }
        }
    }

    fn band_stats(gray: &[u8], y0: usize, y1: usize) -> (u8, u8) {
        let band = gray[y0 * WIDTH..y1 * WIDTH].iter().copied();
        (median(band.clone()), percentile(band, INK_PERCENTILE))
    }

    #[test]
    fn a_light_fill_with_dark_text_becomes_dark_text_on_white() {
        let mut gray = shaded_page();
        flatten_shaded_rows(&mut gray, WIDTH, HEIGHT);

        let (fill, ink) = band_stats(&gray, 100, 140);
        assert!(fill >= 250, "the light fill must become paper, got median {fill}");
        assert!(ink <= 5, "the dark glyphs must stay ink, got second percentile {ink}");
    }

    #[test]
    fn a_dark_fill_with_white_text_becomes_dark_text_on_white() {
        let mut gray = shaded_page();
        flatten_shaded_rows(&mut gray, WIDTH, HEIGHT);

        let (fill, ink) = band_stats(&gray, 200, 240);
        assert!(fill >= 250, "the dark fill must become paper, got median {fill}");
        assert!(
            ink <= 5,
            "the white glyphs must become ink, got second percentile {ink}"
        );
    }

    #[test]
    fn rows_outside_the_bands_are_untouched_and_a_plain_page_is_byte_identical() {
        let original = shaded_page();
        let mut gray = original.clone();
        flatten_shaded_rows(&mut gray, WIDTH, HEIGHT);
        assert_eq!(
            &gray[..96 * WIDTH],
            &original[..96 * WIDTH],
            "rows above the first band"
        );
        assert_eq!(
            &gray[144 * WIDTH..196 * WIDTH],
            &original[144 * WIDTH..196 * WIDTH],
            "rows between the bands"
        );
        assert_eq!(
            &gray[244 * WIDTH..],
            &original[244 * WIDTH..],
            "rows below the second band"
        );

        let mut plain = vec![255u8; WIDTH * HEIGHT];
        for x in (60..WIDTH - 60).step_by(40) {
            for y in 100..112 {
                plain[y * WIDTH + x] = 0;
            }
        }
        let before = plain.clone();
        flatten_shaded_rows(&mut plain, WIDTH, HEIGHT);
        assert_eq!(plain, before, "dark text on white paper has no band to flatten");
    }

    #[test]
    fn the_rgb_entry_point_returns_gray_as_rgb_and_rejects_a_mismatched_buffer() {
        let gray = shaded_page();
        let rgb: Vec<u8> = gray.iter().flat_map(|&v| [v, v, v]).collect();
        let out = flatten_shaded_rows_rgb(&rgb, WIDTH as u32, HEIGHT as u32);
        assert_eq!(out.len(), rgb.len());
        assert!(out.chunks_exact(3).all(|px| px[0] == px[1] && px[1] == px[2]));
        assert!(
            out[(120 * WIDTH + 200) * 3] >= 250,
            "the light band's fill is paper in the RGB output"
        );

        let short = vec![0u8; 10];
        assert_eq!(flatten_shaded_rows_rgb(&short, WIDTH as u32, HEIGHT as u32), short);
    }

    #[test]
    fn percentile_and_median_are_exact_on_small_inputs() {
        assert_eq!(median([5u8, 1, 9].into_iter()), 5);
        assert_eq!(median([4u8, 8].into_iter()), 4);
        assert_eq!(percentile(0..=255u8, 0.02), 5);
        assert_eq!(percentile(std::iter::empty(), 0.5), 255);
    }
}
