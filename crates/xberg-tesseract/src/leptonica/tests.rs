use std::cell::Cell;

use super::*;

const TEST_X_RESOLUTION: i32 = 240;
const TEST_Y_RESOLUTION: i32 = 180;

thread_local! {
    static GEOMETRY_DISPATCH_BLOCKED: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn check_geometry_dispatch() -> Result<()> {
    if GEOMETRY_DISPATCH_BLOCKED.get() {
        return Err(TesseractError::OcrError);
    }
    Ok(())
}

fn without_geometry_dispatch(check: impl FnOnce()) {
    struct RestoreDispatch<'a>(&'a Cell<bool>, bool);
    impl Drop for RestoreDispatch<'_> {
        fn drop(&mut self) {
            self.0.set(self.1);
        }
    }

    GEOMETRY_DISPATCH_BLOCKED.with(|blocked| {
        let _restore = RestoreDispatch(blocked, blocked.replace(true));
        check();
    });
}

fn make_rgb_pix(width: u32, height: u32, fill: u8) -> Pix {
    let data = vec![fill; (width * height * 3) as usize];
    Pix::from_raw_rgb(&data, width, height).expect("from_raw_rgb failed")
}

fn gray_pixel(pix: &Pix, x: i32, y: i32) -> f64 {
    pix.clip_rectangle(x, y, 1, 1)
        .expect("clip failed")
        .mean_gray_value(1)
        .expect("pixel read failed")
}

fn gray_fixture(width: u32, height: u32) -> Pix {
    let mut data = Vec::with_capacity((width * height * 3) as usize);
    for y in 0..height {
        for x in 0..width {
            let value = 48 + ((x * 160) / width) as u8;
            let value = if (x + y * 17).is_multiple_of(53) { 255 } else { value };
            data.extend_from_slice(&[value, value, value]);
        }
    }
    make_gray_pix_with_resolution(&data, width, height)
}

fn make_gray_pix_with_resolution(data: &[u8], width: u32, height: u32) -> Pix {
    let mut pix = Pix::from_raw_rgb(data, width, height)
        .expect("from_raw_rgb failed")
        .to_grayscale()
        .expect("to_grayscale failed");
    pix.set_resolution(TEST_X_RESOLUTION, TEST_Y_RESOLUTION)
        .expect("set_resolution failed");
    pix
}

#[test]
fn test_from_raw_rgb_dimensions() {
    let pix = make_rgb_pix(16, 8, 200);
    assert_eq!(pix.width(), 16);
    assert_eq!(pix.height(), 8);
    assert_eq!(pix.depth(), 32);
}

#[test]
fn test_from_raw_rgb_wrong_length() {
    let data = vec![0u8; 10];
    let err = Pix::from_raw_rgb(&data, 4, 4).unwrap_err();
    assert!(matches!(err, TesseractError::InvalidImageData));
}

#[test]
fn test_from_raw_rgb_zero_dimensions() {
    let err = Pix::from_raw_rgb(&[], 0, 4).unwrap_err();
    assert!(matches!(err, TesseractError::InvalidImageData));

    let err = Pix::from_raw_rgb(&[], 4, 0).unwrap_err();
    assert!(matches!(err, TesseractError::InvalidImageData));
}

#[test]
fn test_as_ptr_is_non_null() {
    let pix = make_rgb_pix(8, 8, 128);
    assert!(!pix.as_ptr().is_null());
}

#[test]
fn test_to_grayscale() {
    let pix = make_rgb_pix(32, 32, 150);
    let gray = pix.to_grayscale().expect("to_grayscale failed");
    assert_eq!(gray.width(), 32);
    assert_eq!(gray.height(), 32);
    assert_eq!(gray.depth(), 8);
}

#[test]
fn test_scale_up() {
    let pix = make_rgb_pix(20, 10, 100);
    let scaled = pix.scale(2.0, 2.0).expect("scale failed");
    assert_eq!(scaled.width(), 40);
    assert_eq!(scaled.height(), 20);

    let rounded = make_rgb_pix(5, 7, 100).scale(1.5, 1.5).expect("rounded scale");
    assert_eq!((rounded.width(), rounded.height()), (8, 11));
}

#[test]
fn test_unsharp_mask_returns_same_dimensions() {
    let pix = make_rgb_pix(32, 32, 200);
    let sharpened = pix.unsharp_mask(2, 0.4).expect("unsharp_mask failed");
    assert_eq!(sharpened.width(), 32);
    assert_eq!(sharpened.height(), 32);
}

#[test]
fn no_op_transforms_return_independent_pixels_and_resolution() {
    let input = Pix::from_grayscale_bytes(&[220; 64 * 64], 64, 64).expect("make a flat grayscale image");
    let unsharp = input.unsharp_mask(0, 0.0).expect("no-op sharpening");
    let (deskewed, rotated) = input.deskew_with_rotation().expect("deskew a flat image");
    assert!(!rotated);
    for mut output in [unsharp, deskewed] {
        assert_ne!(
            output.as_ptr(),
            input.as_ptr(),
            "transform result must own different pixels"
        );
        assert_eq!(output.grayscale_bytes().unwrap(), [220; 64 * 64]);
        assert_eq!(output.get_resolution().unwrap(), input.get_resolution().unwrap());
        output.set_resolution(72, 96).expect("change result resolution");
        assert_eq!(input.get_resolution().unwrap(), (300, 300));
    }
}

#[test]
fn grayscale_bytes_refuses_images_that_are_not_8_bpp() {
    let rgb = make_rgb_pix(8, 8, 200);
    assert_eq!(rgb.depth(), 32);
    assert!(matches!(
        rgb.grayscale_bytes(),
        Err(TesseractError::InvalidParameterError)
    ));

    let gray = rgb.to_grayscale().expect("to_grayscale");
    assert_eq!(gray.grayscale_bytes().expect("8 bpp read").len(), 64);
}

ffi_extern! {
    fn pixcmapCreate(depth: i32) -> *mut c_void;
    fn pixSetColormap(pix: *mut c_void, colormap: *mut c_void) -> i32;
}

#[test]
fn grayscale_bytes_refuses_a_colormapped_8_bpp_image() {
    let pix = Pix::from_grayscale_bytes(&[7; 16], 4, 4).expect("8 bpp image");
    assert!(pix.grayscale_bytes().is_ok());
    // SAFETY: the new colormap is transferred to the live Pix, which owns and frees it. ~keep
    let status = unsafe { pixSetColormap(pix.as_ptr(), pixcmapCreate(8)) };
    assert_eq!(status, 0);
    assert!(matches!(
        pix.grayscale_bytes(),
        Err(TesseractError::InvalidParameterError)
    ));
}

#[test]
fn binary_bytes_reports_set_bits_as_ink_and_refuses_grayscale_images() {
    const WIDTH: u32 = 16;
    const HEIGHT: u32 = 8;
    let mut data = Vec::with_capacity((WIDTH * HEIGHT) as usize);
    for _ in 0..HEIGHT {
        data.extend(std::iter::repeat_n(40u8, (WIDTH / 2) as usize));
        data.extend(std::iter::repeat_n(210u8, (WIDTH / 2) as usize));
    }
    let gray = Pix::from_grayscale_bytes(&data, WIDTH, HEIGHT).expect("make grayscale image");
    let binary = gray.otsu_threshold().expect("one-bit raster");
    let row_width = WIDTH as usize;
    let expected: Vec<u8> = (0..row_width * HEIGHT as usize)
        .map(|index| u8::from(index % row_width < row_width / 2))
        .collect();

    assert_eq!(binary.binary_bytes().expect("read binary pixels"), expected);
    assert!(matches!(
        gray.binary_bytes(),
        Err(TesseractError::InvalidParameterError)
    ));
}

#[test]
fn deskew_reports_rotation_for_slanted_rows() {
    const WIDTH: usize = 384;
    const HEIGHT: usize = 256;
    let mut pixels = vec![255; WIDTH * HEIGHT];
    for top in (18..HEIGHT - 45).step_by(25) {
        for left in (24..WIDTH - 24).step_by(13) {
            for row in top..top + 8 {
                for x in left..left + 7 {
                    pixels[(row + x / 12) * WIDTH + x] = 0;
                }
            }
        }
    }
    let input = Pix::from_grayscale_bytes(&pixels, WIDTH as u32, HEIGHT as u32).expect("make slanted rows");
    let (output, rotated) = input.deskew_with_rotation().expect("deskew slanted rows");
    assert!(rotated);
    assert_ne!(output.grayscale_bytes().unwrap(), pixels);
}

#[test]
fn typed_thresholded_images_do_not_share_engine_metadata() {
    let api = crate::TesseractAPI::new().expect("create engine");
    api.init_for_analyse_page()
        .expect("initialize without recognition data");
    api.set_image(&[128; 64 * 64], 64, 64, 1, 64)
        .expect("set grayscale image");
    let mut first = api.thresholded_image().expect("copy thresholded image");
    let source_resolution = first.get_resolution().expect("read thresholded resolution");
    let changed_resolution = (source_resolution.0 + 1, source_resolution.1 + 1);
    first
        .set_resolution(changed_resolution.0, changed_resolution.1)
        .expect("change only the copied image");
    let second = api.thresholded_image().expect("copy the same engine image again");
    assert_ne!(first.as_ptr(), second.as_ptr());
    assert_eq!(second.get_resolution().unwrap(), source_resolution);
    assert_eq!((second.width(), second.height(), second.depth()), (64, 64, 1));
}

#[test]
fn scale_wrapper_rejects_invalid_geometry_before_native_dispatch() {
    let pix = make_rgb_pix(4, 4, 128);
    without_geometry_dispatch(|| {
        // A valid request establishes that the interlock catches the dispatch path. ~keep
        assert!(matches!(pix.scale(1.5, 1.5), Err(TesseractError::OcrError)));
        assert!(matches!(
            pix.scale(0.0, 1.0),
            Err(TesseractError::InvalidParameterError)
        ));
        assert!(matches!(
            pix.scale(1.0, f32::NAN),
            Err(TesseractError::InvalidParameterError)
        ));
    });
    let output = pix.scale(1.5, 1.5).expect("valid scaling after the interlock");
    assert_eq!((output.width(), output.height()), (6, 6));
}

#[test]
fn clip_wrapper_rejects_overflow_before_native_dispatch() {
    let pix = make_rgb_pix(4, 4, 128);
    without_geometry_dispatch(|| {
        // A valid request establishes that the interlock catches the dispatch path. ~keep
        assert!(matches!(pix.clip_rectangle(1, 1, 2, 2), Err(TesseractError::OcrError)));
        assert!(matches!(
            pix.clip_rectangle(1, 0, i32::MAX, 1),
            Err(TesseractError::InvalidParameterError)
        ));
        assert!(matches!(
            pix.clip_rectangle(0, 1, 1, i32::MAX),
            Err(TesseractError::InvalidParameterError)
        ));
    });
    let output = pix.clip_rectangle(1, 1, 2, 2).expect("valid crop after the interlock");
    assert_eq!((output.width(), output.height()), (2, 2));
}

#[test]
fn scale_validation_rejects_nonfinite_and_out_of_range_native_rounding() {
    for factor in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0, 0.0] {
        assert!(matches!(
            validate_scale(20, 20, 8, factor, 1.0),
            Err(TesseractError::InvalidParameterError)
        ));
        assert!(matches!(
            validate_scale(20, 20, 8, 1.0, factor),
            Err(TesseractError::InvalidParameterError)
        ));
    }
    let first_unrepresentable = i32::MAX as f32;
    let last_representable_product = first_unrepresentable.next_down();
    assert!(validate_scale(1, 1, 8, last_representable_product, 1.0).is_ok());
    assert!(matches!(
        validate_scale(1, 1, 8, first_unrepresentable, 1.0),
        Err(TesseractError::InvalidParameterError)
    ));
    assert!(matches!(
        validate_scale(20, 20, 8, f32::MAX, 1.0),
        Err(TesseractError::InvalidParameterError)
    ));
    assert!(matches!(
        validate_scale(20, 20, 8, f32::MIN_POSITIVE, 0.01),
        Err(TesseractError::InvalidParameterError)
    ));
}

#[test]
fn clip_endpoint_validation_preserves_outside_clipping_and_rejects_overflow() {
    assert!(validate_clip_endpoints(i32::MAX - 1, 0, 1, 1).is_ok());
    assert!(validate_clip_endpoints(-2, 0, 3, 1).is_ok());
    assert!(matches!(
        validate_clip_endpoints(1, 0, i32::MAX, 1),
        Err(TesseractError::InvalidParameterError)
    ));
    assert!(matches!(
        validate_clip_endpoints(0, 1, 1, i32::MAX),
        Err(TesseractError::InvalidParameterError)
    ));
    let clipped = make_rgb_pix(4, 4, 128)
        .clip_rectangle(-2, 0, 3, 1)
        .expect("clip to positive quadrant");
    assert_eq!((clipped.width(), clipped.height()), (1, 1));
}

#[test]
fn test_adaptive_threshold_produces_1bpp() {
    let pix = make_rgb_pix(64, 64, 180);
    let gray = pix.to_grayscale().expect("to_grayscale failed");
    let binary = gray.adaptive_threshold(32, 32).expect("adaptive_threshold failed");
    assert_eq!(binary.depth(), 1);
}

#[test]
fn median_filter_removes_single_pixel_noise_and_preserves_resolution() {
    const WIDTH: u32 = 9;
    const HEIGHT: u32 = 9;
    let mut data = vec![200; (WIDTH * HEIGHT * 3) as usize];
    let center = ((HEIGHT / 2 * WIDTH + WIDTH / 2) * 3) as usize;
    data[center..center + 3].fill(0);
    let gray = make_gray_pix_with_resolution(&data, WIDTH, HEIGHT);

    let filtered = gray.median_filter(3, 3).expect("median_filter failed");

    assert_eq!(gray_pixel(&filtered, 4, 4), 200.0);
    assert_eq!(
        filtered.get_resolution().unwrap(),
        (TEST_X_RESOLUTION, TEST_Y_RESOLUTION)
    );
}

#[test]
fn contrast_stretch_changes_dynamic_range_and_preserves_resolution() {
    let width = 3;
    let data = [50, 50, 50, 125, 125, 125, 200, 200, 200];
    let gray = make_gray_pix_with_resolution(&data, width, 1);

    let stretched = gray.contrast_stretch(1.0, 50, 200).expect("contrast_stretch failed");

    assert_eq!(gray_pixel(&stretched, 0, 0), 0.0);
    assert_eq!(gray_pixel(&stretched, 2, 0), 255.0);
    assert_eq!(
        stretched.get_resolution().unwrap(),
        (TEST_X_RESOLUTION, TEST_Y_RESOLUTION)
    );
}

#[test]
fn global_otsu_threshold_produces_binary_and_preserves_resolution() {
    let gray = gray_fixture(96, 64);

    let binary = gray.otsu_threshold().expect("otsu_threshold failed");

    assert_eq!(binary.depth(), 1);
    assert_eq!(binary.get_resolution().unwrap(), (TEST_X_RESOLUTION, TEST_Y_RESOLUTION));
}

fn leptonica_full_tile_otsu(gray: &Pix) -> Pix {
    gray.otsu_threshold_with_options(
        gray.width().max(Pix::MINIMUM_OTSU_TILE_DIMENSION),
        gray.height().max(Pix::MINIMUM_OTSU_TILE_DIMENSION),
        0,
        0,
        Pix::STANDARD_OTSU_SCORE_FRACTION,
    )
    .expect("leptonica otsu")
}

fn two_level_gray(width: u32, height: u32, ink: impl Fn(u32, u32) -> u8) -> Pix {
    let mut data = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            data.push(ink(x, y));
        }
    }
    Pix::from_grayscale_bytes(&data, width, height).expect("gray pix")
}

#[test]
fn global_otsu_matches_leptonica_full_tile_otsu_bit_for_bit() {
    for (width, height) in [(96, 64), (97, 61), (13, 7), (4, 4), (101, 3)] {
        let gray = gray_fixture(width, height);
        let expected = leptonica_full_tile_otsu(&gray);
        let actual = gray.otsu_threshold().expect("global otsu");
        assert_eq!(
            actual.binary_bytes().unwrap(),
            expected.binary_bytes().unwrap(),
            "{width}x{height}"
        );
        assert_eq!(actual.get_resolution().unwrap(), expected.get_resolution().unwrap());
        let skewed = two_level_gray(width, height, |x, y| [12, 40, 200, 230][((x * 7 + y * 3) % 4) as usize]);
        assert_eq!(
            skewed.otsu_threshold().unwrap().binary_bytes().unwrap(),
            leptonica_full_tile_otsu(&skewed).binary_bytes().unwrap(),
            "skewed {width}x{height}"
        );
    }
}

#[test]
fn global_otsu_saturates_bin_counts_past_two_to_the_twenty_four_like_leptonica() {
    let side = 4400;
    let value = |x: u32, y: u32| match (x % 97, y % 89) {
        (0..=2, _) => 0,
        (_, 0..=1) => 90,
        (_, 2..=4) => 170,
        _ => 255,
    };
    let paper = (0..side)
        .flat_map(|y| (0..side).map(move |x| (x, y)))
        .filter(|&(x, y)| value(x, y) == 255)
        .count();
    assert!(paper > 1 << 24, "the paper bin must pass the f32 exact-count limit");
    let gray = two_level_gray(side, side, value);
    assert_eq!(
        gray.otsu_threshold().unwrap().binary_bytes().unwrap(),
        leptonica_full_tile_otsu(&gray).binary_bytes().unwrap()
    );
}

#[test]
fn adaptive_otsu_threshold_produces_binary_and_preserves_resolution() {
    let gray = gray_fixture(96, 64);

    let binary = gray.adaptive_threshold(32, 32).expect("adaptive_threshold failed");

    assert_eq!(binary.depth(), 1);
    assert_eq!(binary.get_resolution().unwrap(), (TEST_X_RESOLUTION, TEST_Y_RESOLUTION));
}

#[test]
fn tiled_sauvola_threshold_produces_binary_and_preserves_resolution() {
    let gray = gray_fixture(96, 64);

    let binary = gray.sauvola_threshold(7, 0.35, 2, 2).expect("sauvola_threshold failed");

    assert_eq!(binary.depth(), 1);
    assert_eq!(binary.get_resolution().unwrap(), (TEST_X_RESOLUTION, TEST_Y_RESOLUTION));
}

#[test]
fn preprocessing_wrappers_reject_invalid_parameters_before_ffi() {
    let rgb = make_rgb_pix(32, 32, 128);
    let gray = rgb.to_grayscale().expect("to_grayscale failed");

    assert!(matches!(
        rgb.median_filter(3, 3),
        Err(TesseractError::InvalidParameterError)
    ));
    assert!(matches!(
        gray.median_filter(0, 3),
        Err(TesseractError::InvalidParameterError)
    ));
    assert!(matches!(
        gray.contrast_stretch(f32::NAN, 0, 255),
        Err(TesseractError::InvalidParameterError)
    ));
    assert!(matches!(
        gray.contrast_stretch(1.0, 200, 100),
        Err(TesseractError::InvalidParameterError)
    ));
    assert!(matches!(
        gray.adaptive_threshold(8, 32),
        Err(TesseractError::InvalidParameterError)
    ));
    assert!(matches!(
        gray.sauvola_threshold(1, 0.35, 1, 1),
        Err(TesseractError::InvalidParameterError)
    ));
    assert!(matches!(
        gray.sauvola_threshold(7, -0.1, 1, 1),
        Err(TesseractError::InvalidParameterError)
    ));
    assert!(matches!(
        gray.sauvola_threshold(7, 0.35, 0, 1),
        Err(TesseractError::InvalidParameterError)
    ));
}

#[test]
fn test_invert_flips_uniform_gray_value() {
    let pix = make_rgb_pix(16, 16, 30);
    let gray = pix.to_grayscale().expect("to_grayscale failed");
    let inverted = gray.invert().expect("invert failed");

    assert_eq!(inverted.width(), gray.width());
    assert_eq!(inverted.height(), gray.height());
    assert_eq!(inverted.depth(), 8);

    let original_mean = gray.mean_gray_value(1).expect("mean_gray_value failed");
    let inverted_mean = inverted.mean_gray_value(1).expect("mean_gray_value failed");
    assert!((original_mean + inverted_mean - 255.0).abs() < 1.0);
}

#[test]
fn test_invert_twice_round_trips() {
    let pix = make_rgb_pix(16, 16, 90);
    let gray = pix.to_grayscale().expect("to_grayscale failed");

    let once = gray.invert().expect("first invert failed");
    let twice = once.invert().expect("second invert failed");

    let original_mean = gray.mean_gray_value(1).expect("mean_gray_value failed");
    let round_tripped_mean = twice.mean_gray_value(1).expect("mean_gray_value failed");
    assert!((original_mean - round_tripped_mean).abs() < 1.0);
}

#[test]
fn test_mean_gray_value_light_vs_dark() {
    let light = make_rgb_pix(8, 8, 220).to_grayscale().expect("to_grayscale failed");
    let dark = make_rgb_pix(8, 8, 20).to_grayscale().expect("to_grayscale failed");

    let light_mean = light.mean_gray_value(1).expect("mean_gray_value failed");
    let dark_mean = dark.mean_gray_value(1).expect("mean_gray_value failed");

    assert!(light_mean > 200.0, "light_mean was {light_mean}");
    assert!(dark_mean < 40.0, "dark_mean was {dark_mean}");
}

fn coloured_rgb(width: u32, height: u32) -> Vec<u8> {
    let mut state = 0x9e37_79b9_u32;
    (0..width * height * 3)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 24) as u8
        })
        .collect()
}

fn assert_gray_matches_full_page_conversion(rgb: &[u8], width: u32, height: u32) {
    let expected = Pix::from_raw_rgb(rgb, width, height)
        .expect("from_raw_rgb failed")
        .to_grayscale()
        .expect("to_grayscale failed");
    let banded = Pix::gray_from_raw_rgb(rgb, width, height).expect("gray_from_raw_rgb failed");
    assert_eq!(
        (banded.width(), banded.height(), banded.depth()),
        (expected.width(), expected.height(), expected.depth()),
        "{width}x{height}"
    );
    assert_eq!(banded.depth(), 8);
    assert_eq!(
        banded.get_resolution().unwrap(),
        expected.get_resolution().unwrap(),
        "{width}x{height}"
    );
    assert_eq!(
        banded.grayscale_bytes().unwrap(),
        expected.grayscale_bytes().unwrap(),
        "{width}x{height}"
    );
}

#[test]
fn gray_from_raw_rgb_equals_the_full_page_conversion_across_band_edges() {
    let band = Pix::GRAY_CONVERSION_STRIP_ROWS as u32;
    for width in [1, 3, 37, 64, 129] {
        for height in [1, band - 1, band, band + 1, 3 * band + 5] {
            assert_gray_matches_full_page_conversion(&coloured_rgb(width, height), width, height);
        }
    }
}

#[test]
fn gray_from_raw_rgb_equals_the_full_page_conversion_for_every_red_green_pair() {
    let mut rgb = Vec::with_capacity(256 * 256 * 3);
    for green in 0..=255u8 {
        for red in 0..=255u8 {
            rgb.extend([red, green, red ^ green]);
        }
    }
    assert_gray_matches_full_page_conversion(&rgb, 256, 256);
}

#[test]
fn gray_from_raw_rgb_rejects_what_from_raw_rgb_rejects() {
    for (data, width, height) in [(vec![0u8; 10], 4, 4), (Vec::new(), 0, 4), (Vec::new(), 4, 0)] {
        let err = Pix::gray_from_raw_rgb(&data, width, height).unwrap_err();
        assert!(matches!(err, TesseractError::InvalidImageData), "{width}x{height}");
    }
}
