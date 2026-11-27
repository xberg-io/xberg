use super::*;
use crate::extractors::images::{ColorSpace, ImageData, PdfImage, PixelFormat};
use crate::rendering::page_renderer::image_budget::{native_downscale_peak_bytes, resize_working_bytes};
use std::io::Write;

/// Deterministic samples with every byte value and no long runs, so a rounding difference in any
/// resize kernel shows.
fn samples(length: usize) -> Vec<u8> {
    let mut state: u32 = 0x9e37_79b9;
    (0..length)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        })
        .collect()
}

/// The canvas `render_image` drew before the native downscale: the samples widened to RGBA at full
/// size by `to_rgba8`, premultiplied, then pre-resized and drawn by `blit_rgba`.
fn drawn_through_rgba(
    pixels: &[u8],
    components: u8,
    source: (u32, u32),
    image_transform: Transform,
    paint: PixmapPaint,
    canvas: &Pixmap,
) -> Pixmap {
    let decoded = match components {
        1 => image::DynamicImage::ImageLuma8(image::GrayImage::from_raw(source.0, source.1, pixels.to_vec()).unwrap()),
        _ => image::DynamicImage::ImageRgb8(image::RgbImage::from_raw(source.0, source.1, pixels.to_vec()).unwrap()),
    };
    let mut rgba = decoded.to_rgba8();
    premultiply_rgba(&mut rgba);
    let mut canvas = canvas.clone();
    blit_rgba(
        &mut canvas,
        rgba,
        image_transform,
        paint,
        None,
        image_budget::Blit::PreResize,
    );
    canvas
}

#[test]
fn a_native_downscale_draws_the_same_pixels_as_the_rgba_pre_resize() {
    let mut canvas = Pixmap::new(400, 300).unwrap();
    canvas.fill(tiny_skia::Color::from_rgba8(40, 90, 200, 255));
    let paint = PixmapPaint {
        opacity: 0.7,
        ..PixmapPaint::default()
    };
    // Odd widths and heights reach every SIMD remainder (16, 8, 4, 2 and 1 coefficients, four-row
    // and one-row passes); 0.1 gives windows over 16 coefficients; the 1.0 axes resize one axis only.
    let cases: [((u32, u32), (f32, f32), (f32, f32)); 7] = [
        ((257, 131), (0.5, 0.5), (3.25, 7.5)),
        ((64, 64), (0.3, 0.3), (0.0, 0.0)),
        ((1001, 37), (0.25, 0.6), (11.0, 2.75)),
        ((400, 300), (0.1, 0.1), (1.5, 1.5)),
        ((500, 500), (0.854, 0.854), (0.0, 0.0)),
        ((300, 90), (0.5, 1.0), (4.0, 0.0)),
        ((90, 300), (1.0, 0.45), (0.0, 4.0)),
    ];
    let mut compared = 0;
    for components in [1u8, 3] {
        for (source, (sx, sy), (tx, ty)) in cases {
            let pixels = samples(source.0 as usize * source.1 as usize * usize::from(components));
            let painted = (source.0 as f32 * sx, source.1 as f32 * sy);
            let transform = Transform::from_row(painted.0, 0.0, 0.0, -painted.1, tx, ty + painted.1);
            let image_transform = image_unit_square_transform(transform, source.0, source.1);
            let resized = native_downscale(&pixels, components, source, image_transform)
                .unwrap_or_else(|| panic!("{components}-byte {source:?} at {sx}x{sy} must take the native path"));
            assert!(resized.0.0 <= source.0 && resized.0.1 <= source.1);
            let mut native = canvas.clone();
            blit_native_resized(&mut native, resized, components, image_transform, paint, None);
            let expected = drawn_through_rgba(&pixels, components, source, image_transform, paint, &canvas);
            assert!(
                native.data() == expected.data(),
                "{components}-byte {source:?} at {sx}x{sy}: the native downscale changed pixels"
            );
            assert!(native.data() != canvas.data(), "{source:?}: the image must be drawn");
            compared += 1;
        }
    }
    assert_eq!(compared, 14);
}

#[test]
fn only_an_axis_aligned_downscale_takes_the_native_path() {
    let pixels = samples(100 * 100);
    let native = |transform: Transform| {
        let image_transform = image_unit_square_transform(transform, 100, 100);
        native_downscale(&pixels, 1, (100, 100), image_transform).is_some()
    };
    assert!(native(Transform::from_row(50.0, 0.0, 0.0, -50.0, 0.0, 50.0)));
    // At its own size, enlarged, rotated, or shrunk on one axis while enlarged on the other.
    assert!(!native(Transform::from_row(100.0, 0.0, 0.0, -100.0, 0.0, 100.0)));
    assert!(!native(Transform::from_row(300.0, 0.0, 0.0, -300.0, 0.0, 300.0)));
    assert!(!native(Transform::from_row(0.0, 50.0, 50.0, 0.0, 0.0, 0.0)));
    assert!(!native(Transform::from_row(50.0, 0.0, 0.0, -300.0, 0.0, 300.0)));
}

#[test]
fn native_samples_borrow_only_what_to_dynamic_image_widens_unchanged() {
    let raw = |color_space, bits, pixels: Vec<u8>, format| {
        PdfImage::new(4, 2, color_space, bits, ImageData::Raw { pixels, format })
    };
    let gray = raw(ColorSpace::DeviceGray, 8, vec![7; 8], PixelFormat::Grayscale);
    assert_eq!(native_samples(&gray).map(|(_, components)| components), Some(1));
    let rgb = raw(ColorSpace::DeviceRGB, 8, vec![7; 24], PixelFormat::RGB);
    assert_eq!(native_samples(&rgb).map(|(_, components)| components), Some(3));
    // A 1-bit DeviceGray image unpacks its own way, CMYK is converted, and a buffer of another
    // length is cut or refused by `to_dynamic_image`.
    let one_bit = raw(ColorSpace::DeviceGray, 1, vec![7; 8], PixelFormat::Grayscale);
    assert!(native_samples(&one_bit).is_none());
    let cmyk = raw(ColorSpace::DeviceCMYK, 8, vec![7; 32], PixelFormat::CMYK);
    assert!(native_samples(&cmyk).is_none());
    let long = raw(ColorSpace::DeviceGray, 8, vec![7; 9], PixelFormat::Grayscale);
    assert!(native_samples(&long).is_none());
    let jpeg = PdfImage::new(4, 2, ColorSpace::DeviceRGB, 8, ImageData::Jpeg(source::jpeg_of(4, 2)));
    assert!(native_samples(&jpeg).is_none());
}

fn zlib(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

/// A 32 pt page painting a Flate-coded `color_space` image of `size` over `placement`, with an
/// all-opaque raw `/SMask` when `opaque_mask`.
fn flate_page(color_space: &str, size: (u32, u32), pixels: &[u8], placement: &str, opaque_mask: bool) -> PdfDocument {
    let content = format!("q {placement} cm /Im0 Do Q\n");
    let smask = if opaque_mask { "/SMask 6 0 R" } else { "" };
    let mask = vec![255; size.0 as usize * size.1 as usize];
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 32 32] /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>".to_vec(),
        source::stream("", content.as_bytes()),
        source::stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /{color_space} \
                 /BitsPerComponent 8 /Filter /FlateDecode {smask}",
                size.0, size.1
            ),
            &zlib(pixels),
        ),
        source::stream(
            &format!(
                "/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8",
                size.0, size.1
            ),
            &mask,
        ),
    ];
    source::from_objects(&objects)
}

#[test]
fn a_flate_page_renders_the_same_through_the_native_path_as_through_the_masked_rgba_path() {
    // An all-opaque /SMask keeps the image on the RGBA path without changing a pixel, so the two
    // pages differ only in which path draws the image.
    let size = (97, 61);
    let mut rendered = 0;
    for (color_space, components) in [("DeviceGray", 1), ("DeviceRGB", 3)] {
        let pixels = samples(size.0 as usize * size.1 as usize * components);
        for placement in ["32 0 0 32 0 0", "20 0 0 13 5.5 7.25"] {
            let render = |opaque_mask| {
                renderer()
                    .render_page_with_budget(0, &flate_page(color_space, size, &pixels, placement, opaque_mask), None)
                    .expect("the page renders")
                    .0
            };
            let (native, masked) = (render(false), render(true));
            assert!(
                native
                    .data
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[..3] != [255, 255, 255]),
                "{color_space} at {placement}: the image must be drawn"
            );
            assert!(
                native.data == masked.data,
                "{color_space} at {placement}: the native path changed the page"
            );
            rendered += 1;
        }
    }
    assert_eq!(rendered, 4);
}

#[test]
fn the_native_downscale_peak_counts_its_source_resize_and_widened_raster() {
    // 1200 x 1200 to 1000 x 1000 in gray: the source (1 440 000) beside the 1-byte resize (output
    // 1 000 000, intermediate 1 200 001, two 114 000-byte coefficient tables) is 3 868 001, below
    // the 1 000 000 resized samples beside their 4 000 000-byte RGBA widening.
    assert_eq!(native_downscale_peak_bytes((1200, 1200), (1000, 1000), 1), 5_000_000);
    // In RGB the source (4 320 000) and the 3-byte resize (3 000 000 + 3 600 003 + 228 000) lead.
    assert_eq!(native_downscale_peak_bytes((1200, 1200), (1000, 1000), 3), 11_148_003);
    // Both stay below the RGBA path: the 5 760 000-byte RGBA raster beside its pre-resize.
    let rgba_path = 1200 * 1200 * 4 + resize_working_bytes((1200, 1200), (1000, 1000));
    assert_eq!(rgba_path, 14_788_004);
    for components in [1, 3] {
        assert!(native_downscale_peak_bytes((1200, 1200), (1000, 1000), components) < rgba_path);
    }
}
