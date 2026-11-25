use super::*;
use crate::extractors::images::{ColorSpace, ImageData, PdfImage};
use crate::rendering::page_renderer::image_budget::{
    Blit, ImageBudget, JpegDecode, JpegPaint, Placement, TINY_SKIA_DRAW_BYTES, resize_working_bytes,
};
use crate::rendering::render_png_page_with_budget_evidence;

/// A 1200 px square painted over 1000 px: `render_image` pre-resizes it to 1000 x 1000.
const SOURCE: u32 = 1200;
const PAINTED: &[u8] = b"q 1000 0 0 1000 0 0 cm /Im0 Do Q\n";
/// The 1000 x 1000 output (4 000 000), the vertical-first intermediate of 1200 x 1000 pixels plus
/// one alignment pixel (4 800 004), and two coefficient tables of 1000 outputs with a 5-slot
/// window (1000 * (5 * 10 + 64) = 114 000 each).
const RESIZE: u64 = 9_028_004;
/// The 1200 px frame padded to whole 32-column MCUs, three components, 1024 row bytes per column.
const ROW_BYTES: u64 = 1216 * 3 * 1024;

fn square() -> PdfImage {
    PdfImage::new(
        SOURCE,
        SOURCE,
        ColorSpace::DeviceRGB,
        8,
        ImageData::Jpeg(source::jpeg_of(SOURCE, SOURCE)),
    )
}

fn encoded(image: &PdfImage) -> u64 {
    let ImageData::Jpeg(bytes) = image.data() else {
        unreachable!()
    };
    (bytes.capacity() + bytes.len()) as u64
}

fn paint(decode: JpegDecode, blit: Blit) -> JpegPaint {
    JpegPaint { decode, blit }
}

#[test]
fn resize_working_bytes_counts_the_output_intermediate_and_coefficients() {
    assert_eq!(resize_working_bytes((1200, 1200), (1000, 1000)), RESIZE);
    // One resized axis: no intermediate, and only that axis's table (window 2 * 6 + 1 = 13).
    assert_eq!(resize_working_bytes((1, 512), (1, 100)), 400 + 100 * (13 * 10 + 64));
    assert_eq!(resize_working_bytes((32, 32), (32, 32)), 32 * 32 * 4);
    assert_eq!(resize_working_bytes((1, 1), (u32::MAX, u32::MAX)), u64::MAX);
}

#[test]
fn the_pre_resize_is_kept_whenever_it_fits_and_the_direct_draw_is_taken_only_below_it() {
    let image = square();
    let canvas = 1000;
    let pixels = u64::from(SOURCE * SOURCE);
    let shrunk = Placement::extent(1000.0, 1000.0);
    let admit = |limit: u64| ImageBudget::new(limit as usize).admit_jpeg_decode(&image, canvas, shrunk, true);
    // The pre-resize outweighs the RGB decode it follows: the RGBA raster plus its working set.
    let pre_resize = pixels * 4 + RESIZE + encoded(&image) + canvas;
    assert_eq!(admit(pre_resize).unwrap(), paint(JpegDecode::Full, Blit::PreResize));
    // One byte under, the full decode is drawn directly: decode and RGBA copy, nothing resized.
    let full_direct = pixels * (3 + 4) + encoded(&image) + canvas;
    assert!(full_direct < pre_resize - 1);
    assert_eq!(admit(pre_resize - 1).unwrap(), paint(JpegDecode::Full, Blit::Direct));
    assert_eq!(admit(full_direct).unwrap(), paint(JpegDecode::Full, Blit::Direct));
    // Below that, the in-place decode is drawn directly; it holds the decoder's row buffers, which
    // outweigh tiny-skia's draw, and a second copy of the encoded bytes.
    let in_place_direct = pixels * 4
        + ROW_BYTES.max(TINY_SKIA_DRAW_BYTES)
        + encoded(&image)
        + {
            let ImageData::Jpeg(bytes) = image.data() else {
                unreachable!()
            };
            bytes.len() as u64
        }
        + canvas;
    assert_eq!(
        admit(full_direct - 1).unwrap(),
        paint(JpegDecode::InPlace, Blit::Direct)
    );
    assert_eq!(
        admit(in_place_direct).unwrap(),
        paint(JpegDecode::InPlace, Blit::Direct)
    );
    let mut budget = ImageBudget::new(in_place_direct as usize - 1);
    assert!(budget.admit_jpeg_decode(&image, canvas, shrunk, true).is_err());
    assert!(budget.exceeded(), "the refusal is the budget class");
}

#[test]
fn an_image_that_is_not_pre_resized_is_never_admitted_for_a_direct_draw() {
    let image = square();
    let own_size = Placement::extent(SOURCE as f32, SOURCE as f32);
    let full = u64::from(SOURCE * SOURCE) * (3 + 4) + encoded(&image);
    let admitted = ImageBudget::new(full as usize)
        .admit_jpeg_decode(&image, 0, own_size, true)
        .unwrap();
    assert_eq!(admitted, paint(JpegDecode::Full, Blit::PreResize));
}

#[test]
fn a_page_refused_for_its_pre_resize_renders_through_the_direct_draw() {
    let document = source::document_painting(&source::jpeg_of(SOURCE, SOURCE), SOURCE, PAINTED, "");
    let budget = |mib: usize| {
        Some(PngRenderBudget {
            max_working_bytes: mib * 1024 * 1024,
            minimum_dpi: 72,
        })
    };
    // 12 MiB holds the 10.1 MB decode and RGBA copy but not the 14.8 MB pre-resize peak, which the
    // previous accounting refused.
    let (rendered, _) = renderer()
        .render_page_with_budget(0, &document, budget(12))
        .expect("the direct draw holds no resized copy");
    assert!(is_black_somewhere(&rendered), "the direct draw must paint the image");
    // 9 MiB holds not even the 9.5 MB in-place decode drawn directly.
    let (error, exceeded) = render_png_page_with_budget_evidence(&document, 0, 72, 72, 9 * 1024 * 1024)
        .err()
        .expect("no decode fits 9 MiB");
    assert!(exceeded, "{error}");
}

/// Deterministic stroke pattern: 1-3 px strokes, 3-11 px long, on white, like dense small text.
fn text_like(size: u32) -> image::RgbaImage {
    let mut state: u32 = 0x2545_f491;
    let mut next = |bound: u32| {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (state >> 8) % bound
    };
    let mut image = image::RgbaImage::from_pixel(size, size, image::Rgba([255, 255, 255, 255]));
    for _ in 0..size * size / 60 {
        let (x, y) = (next(size), next(size));
        let (length, thickness, horizontal) = (3 + next(9), 1 + next(3), next(2) == 0);
        for along in 0..length {
            for across in 0..thickness {
                let (px, py) = if horizontal {
                    (x + along, y + across)
                } else {
                    (x + across, y + along)
                };
                if px < size && py < size {
                    image.put_pixel(px, py, image::Rgba([0, 0, 0, 255]));
                }
            }
        }
    }
    image
}

fn gradient(size: u32) -> image::RgbaImage {
    image::RgbaImage::from_fn(size, size, |x, y| {
        let value = (x * 153 / (size - 1) + y * 102 / (size - 1)) as u8;
        image::Rgba([value, value, value, 255])
    })
}

/// Mean absolute RGB difference between `image` drawn at `scale` with each [`Blit`].
fn direct_draw_error(image: &image::RgbaImage, scale: f32) -> f64 {
    let image_transform = Transform::from_scale(scale, scale);
    let target = fast_resize_target(image_transform, image.dimensions()).expect("a pre-resized downscale");
    let draw = |blit| {
        let mut canvas = Pixmap::new(target.0, target.1).unwrap();
        canvas.fill(tiny_skia::Color::WHITE);
        blit_rgba(
            &mut canvas,
            image.clone(),
            image_transform,
            PixmapPaint::default(),
            None,
            blit,
        );
        canvas
    };
    let (resized, direct) = (draw(Blit::PreResize), draw(Blit::Direct));
    let differences: Vec<f64> = resized
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .zip(direct.data().as_chunks::<4>().0)
        .flat_map(|(a, b)| (0..3).map(move |channel| f64::from(a[channel].abs_diff(b[channel]))))
        .collect();
    differences.iter().sum::<f64>() / differences.len() as f64
}

#[test]
fn the_direct_draw_stays_within_a_few_levels_of_the_pre_resize() {
    // 500 px at the 0.854 scale a one-page scan is painted at: exactly 427 px either way.
    let text = direct_draw_error(&text_like(500), 0.854);
    assert!(text > 0.0, "the two draws must take different paths");
    assert!(text < 6.0, "text-like strokes differ by {text} levels on average");
    let gradient = direct_draw_error(&gradient(500), 0.854);
    assert!(gradient < 1.0, "a gradient differs by {gradient} levels on average");
}
