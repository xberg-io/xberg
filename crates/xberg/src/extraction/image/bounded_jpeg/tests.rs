use super::{Process, ReducedScale, load_image_for_ocr_with_scale, read_frame, reduced_peak, reduced_scale};
use crate::extractors::security::SecurityLimits;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 256;

fn limits(max_content_size: usize) -> SecurityLimits {
    SecurityLimits {
        max_content_size,
        ..Default::default()
    }
}

fn jpeg(color: image::ExtendedColorType, channels: usize) -> Vec<u8> {
    let pixels: Vec<u8> = (0..WIDTH as usize * HEIGHT as usize * channels)
        .map(|index| (index % 251) as u8)
        .collect();
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90)
        .encode(&pixels, WIDTH, HEIGHT, color)
        .unwrap();
    bytes
}

fn rgb() -> Vec<u8> {
    jpeg(image::ExtendedColorType::Rgb8, 3)
}

fn peak(bytes: &[u8], denominator: u8) -> u64 {
    let frame = read_frame(bytes).unwrap();
    reduced_peak(&frame, reduced_scale(&frame, denominator), bytes.len() as u64).unwrap()
}

#[test]
fn should_read_the_frame_header_of_a_baseline_jpeg() {
    let frame = read_frame(&rgb()).unwrap();
    assert_eq!((frame.width, frame.height), (WIDTH, HEIGHT));
    assert!(frame.process == Process::Sequential);
    assert_eq!(frame.sampling.len(), 3);
}

#[test]
fn should_choose_the_finest_scale_at_an_inclusive_limit() {
    let bytes = rgb();
    let half = peak(&bytes, 2);
    let decoded = load_image_for_ocr_with_scale(&bytes, &limits(half as usize), |_| true).unwrap();
    assert_eq!(decoded.scale_denominator, 2);
    let decoded = load_image_for_ocr_with_scale(&bytes, &limits(half as usize - 1), |_| true).unwrap();
    assert_eq!(decoded.scale_denominator, 4);
    assert_eq!(decoded.image.dimensions(), (WIDTH / 4, HEIGHT / 4));
    assert_eq!((decoded.original_width, decoded.original_height), (WIDTH, HEIGHT));
}

#[test]
fn should_prefer_the_finest_accepted_scale_and_fall_back_to_the_finest_fitting_one() {
    let bytes = rgb();
    let budget = limits(peak(&bytes, 2) as usize);
    let quarter_only = |scale: ReducedScale| scale.denominator == 4;
    let decoded = load_image_for_ocr_with_scale(&bytes, &budget, quarter_only).unwrap();
    assert_eq!(decoded.scale_denominator, 4);
    let decoded = load_image_for_ocr_with_scale(&bytes, &budget, |_| false).unwrap();
    assert_eq!(decoded.scale_denominator, 2);
}

#[test]
fn should_keep_a_fitting_jpeg_at_its_original_size() {
    let bytes = rgb();
    let decoded = load_image_for_ocr_with_scale(&bytes, &limits(16 * 1024 * 1024), |_| false).unwrap();
    assert_eq!(decoded.scale_denominator, 1);
    assert_eq!(decoded.image.dimensions(), (WIDTH, HEIGHT));
}

#[test]
fn should_decode_an_oversized_grayscale_jpeg_at_a_reduced_size_as_rgb() {
    let bytes = jpeg(image::ExtendedColorType::L8, 1);
    assert_eq!(read_frame(&bytes).unwrap().sampling.len(), 1);
    // The full decoder needs the gray raster and its RGB copy: 4 bytes per pixel.
    let budget = WIDTH as usize * HEIGHT as usize * 4 - 1;
    let decoded = load_image_for_ocr_with_scale(&bytes, &limits(budget), |_| true).unwrap();
    assert_eq!(decoded.scale_denominator, 2);
    assert_eq!(decoded.image.dimensions(), (WIDTH / 2, HEIGHT / 2));
}

#[test]
fn should_count_full_resolution_coefficients_for_a_progressive_scan() {
    let mut bytes = rgb();
    let sof = bytes.windows(2).position(|marker| marker == [0xff, 0xc0]).unwrap();
    bytes[sof + 1] = 0xc2;
    let sequential = peak(&rgb(), 8);
    let progressive = peak(&bytes, 8);
    // Each full-resolution block keeps 64 two-byte coefficients.
    assert!(
        progressive >= sequential + u64::from(WIDTH * HEIGHT) * 2,
        "{progressive} vs {sequential}"
    );
}

#[test]
fn should_count_one_set_of_upsampler_line_buffers_per_rayon_thread() {
    let bytes = rgb();
    let frame = read_frame(&bytes).unwrap();
    let scale = reduced_scale(&frame, 2);
    let h_max = u64::from(frame.sampling.iter().map(|sampling| sampling.0).max().unwrap());
    let peak_on = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| reduced_peak(&frame, scale, bytes.len() as u64).unwrap())
    };
    // Three more threads hold three more sets of three line buffers, each one output row
    // times the largest horizontal sampling factor.
    let line_buffer = u64::from(scale.width) * h_max;
    assert_eq!(peak_on(4) - peak_on(1), 3 * 3 * line_buffer);
}

#[test]
fn should_not_count_upsampler_line_buffers_for_a_grayscale_scan() {
    let bytes = jpeg(image::ExtendedColorType::L8, 1);
    let frame = read_frame(&bytes).unwrap();
    let scale = reduced_scale(&frame, 2);
    let peak_on = |threads: usize| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| reduced_peak(&frame, scale, bytes.len() as u64).unwrap())
    };
    assert_eq!(peak_on(4), peak_on(1));
}
