use image::{Rgb, RgbImage};
use ndarray::{ArrayView3, Axis};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{INPUT_SIZE, PpDocLayoutV3Model};
use crate::layout::preprocessing;

const RGB_CHANNELS: usize = 3;
const FIXTURE_COUNT: usize = 5;
const U8_MAX: f32 = 255.0;
const NORMALIZATION_ROUNDING_ERROR: f32 = 2.0 * f32::EPSILON * U8_MAX;
const MANIFEST: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/layout/pp_doclayout_v3_preprocessing/manifest.json"
));

struct Fixture {
    name: &'static str,
    width: u32,
    height: u32,
    rotate: bool,
    edges: bool,
    png: &'static [u8],
}

macro_rules! fixture {
    ($name:literal, $width:literal, $height:literal, $rotate:literal, $edges:literal) => {
        Fixture {
            name: $name,
            width: $width,
            height: $height,
            rotate: $rotate,
            edges: $edges,
            png: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/layout/pp_doclayout_v3_preprocessing/",
                $name,
                ".png"
            )),
        }
    };
}

const FIXTURES: &[Fixture] = &[
    fixture!("odd_portrait", 37, 59, false, false),
    fixture!("odd_landscape", 1603, 901, false, false),
    fixture!("rotated_portrait", 37, 59, true, false),
    fixture!("saturated_edges", 13, 11, false, true),
    fixture!("identity", 800, 800, false, false),
];

#[derive(Deserialize)]
struct Manifest {
    input_size: u32,
    generator_sha256: String,
    max_error_u8: u8,
    sources: Vec<serde_json::Value>,
    cases: Vec<Reference>,
}

#[derive(Deserialize)]
struct Reference {
    name: String,
    width: u32,
    height: u32,
    clockwise_quarter_turns: u8,
    input_rgb_sha256: String,
    output_rgb_sha256: String,
    png_sha256: String,
    im_shape: [f32; 2],
    scale_factor: [f32; 2],
    linear_negative_control_max_error_u8: u8,
}

fn synthetic_rgb(fixture: &Fixture) -> RgbImage {
    let image = RgbImage::from_fn(fixture.width, fixture.height, |x, y| {
        if fixture.edges {
            Rgb([
                if x == 0 { 255 } else { 0 },
                if y == fixture.height - 1 { 255 } else { 0 },
                if (x + y) % 2 == 0 { 255 } else { 0 },
            ])
        } else {
            Rgb([
                if x % 29 < 7 { 255 } else { 0 },
                ((17 * x + 31 * y) % 256) as u8,
                if (x / 13 + y / 11) % 2 == 0 { 255 } else { 0 },
            ])
        }
    });
    if fixture.rotate {
        image::imageops::rotate90(&image)
    } else {
        image
    }
}

fn fixture(name: &str) -> &'static Fixture {
    FIXTURES
        .iter()
        .find(|fixture| fixture.name == name)
        .expect("known fixture")
}

fn read_reference(fixture: &Fixture) -> (Reference, RgbImage) {
    let manifest: Manifest = serde_json::from_slice(MANIFEST).expect("frozen reference manifest");
    assert_eq!(manifest.input_size, INPUT_SIZE);
    assert_eq!(manifest.max_error_u8, 1, "the preregistered bound is one u8 level");
    let reference = manifest
        .cases
        .into_iter()
        .find(|reference| reference.name == fixture.name)
        .expect("every fixture has an independent reference");
    assert_eq!(hex::encode(Sha256::digest(fixture.png)), reference.png_sha256);
    let expected = image::load_from_memory(fixture.png)
        .expect("OpenCV-generated PNG")
        .to_rgb8();
    assert_eq!(expected.dimensions(), (INPUT_SIZE, INPUT_SIZE));
    assert_eq!(
        hex::encode(Sha256::digest(expected.as_raw())),
        reference.output_rgb_sha256
    );
    let input = synthetic_rgb(fixture);
    assert_eq!(input.dimensions(), (reference.width, reference.height));
    assert_eq!(u8::from(fixture.rotate), reference.clockwise_quarter_turns);
    assert_eq!(hex::encode(Sha256::digest(input.as_raw())), reference.input_rgb_sha256);
    (reference, expected)
}

fn max_pixel_error(actual: ArrayView3<'_, f32>, expected: &RgbImage) -> f32 {
    actual
        .indexed_iter()
        .map(|((channel, y, x), &value)| {
            (value * U8_MAX - f32::from(expected.get_pixel(x as u32, y as u32).0[channel])).abs()
        })
        .fold(0.0, f32::max)
}

fn assert_tensor_matches(name: &str, actual: ArrayView3<'_, f32>, expected: &RgbImage) {
    assert_eq!(
        actual.shape(),
        &[RGB_CHANNELS, INPUT_SIZE as usize, INPUT_SIZE as usize]
    );
    assert_eq!(actual.len(), RGB_CHANNELS * INPUT_SIZE as usize * INPUT_SIZE as usize);
    for &value in &actual {
        assert!(
            value.is_finite() && (0.0..=1.0).contains(&value),
            "{name}: invalid normalized RGB value"
        );
        let raw = value * U8_MAX;
        assert!(
            (raw - raw.round()).abs() <= NORMALIZATION_ROUNDING_ERROR,
            "{name}: resizing must round and clamp once before normalization"
        );
    }
    let maximum = max_pixel_error(actual, expected);
    assert!(
        maximum <= 1.0 + NORMALIZATION_ROUNDING_ERROR,
        "{name}: maximum OpenCV pixel error {maximum}"
    );
}

fn assert_single(name: &str) {
    let fixture = fixture(name);
    let (reference, expected) = read_reference(fixture);
    let (actual, shape, scale) = PpDocLayoutV3Model::preprocess_single(&synthetic_rgb(fixture)).unwrap();
    assert_eq!(
        actual.shape(),
        &[1, RGB_CHANNELS, INPUT_SIZE as usize, INPUT_SIZE as usize]
    );
    assert_eq!(shape.as_slice().expect("contiguous shape"), reference.im_shape);
    assert_eq!(scale.as_slice().expect("contiguous scale"), reference.scale_factor);
    assert_tensor_matches(name, actual.index_axis(Axis(0), 0), &expected);
}

#[test]
fn opencv_reference_manifest_covers_every_frozen_synthetic_case() {
    let manifest: Manifest = serde_json::from_slice(MANIFEST).unwrap();
    assert_eq!(FIXTURES.len(), FIXTURE_COUNT);
    assert_eq!(manifest.cases.len(), FIXTURE_COUNT);
    assert_eq!(manifest.sources.len(), 7);
    let generator = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/layout/pp_doclayout_v3_preprocessing/generate.py"
    ));
    assert_eq!(hex::encode(Sha256::digest(generator)), manifest.generator_sha256);
    for fixture in FIXTURES {
        let (reference, _) = read_reference(fixture);
        if fixture.name == "identity" {
            assert_eq!(reference.linear_negative_control_max_error_u8, 0);
        } else {
            assert!(reference.linear_negative_control_max_error_u8 > 1);
        }
    }
}

#[test]
fn odd_portrait_pixels_match_opencv_cubic_including_borders() {
    assert_single("odd_portrait");
}

#[test]
fn odd_landscape_downsample_matches_fixed_support_opencv_cubic() {
    assert_single("odd_landscape");
}

#[test]
fn rotated_input_is_not_rotated_again_and_preserves_rgb_channel_order() {
    assert_single("rotated_portrait");
}

#[test]
fn saturated_edges_match_opencv_without_intermediate_clipping() {
    assert_single("saturated_edges");
}

#[test]
fn identity_resize_keeps_the_existing_normalization_contract() {
    assert_single("identity");
}

#[test]
fn empty_images_fail_before_constructing_invalid_scale_factors() {
    for (width, height) in [(0, INPUT_SIZE), (INPUT_SIZE, 0)] {
        let error = PpDocLayoutV3Model::preprocess_single(&RgbImage::new(width, height)).unwrap_err();
        assert!(error.to_string().contains("empty image"));
    }
}

#[test]
fn singleton_tensor_construction_preserves_each_original_image_frame() {
    let fixtures = [fixture("odd_portrait"), fixture("rotated_portrait")];
    let images = fixtures.map(synthetic_rgb);
    let dimensions: Vec<_> = images.iter().map(|image| image.dimensions()).collect();
    assert_eq!(dimensions, vec![(37, 59), (59, 37)]);
    for (image, fixture) in images.iter().zip(fixtures.iter()) {
        let (pixels, shapes, scales) =
            PpDocLayoutV3Model::preprocess_single(image).expect("model-free singleton tensor construction");
        assert_eq!(
            pixels.shape(),
            &[1, RGB_CHANNELS, INPUT_SIZE as usize, INPUT_SIZE as usize]
        );
        let (reference, expected) = read_reference(fixture);
        assert_eq!(shapes.row(0).as_slice().unwrap(), reference.im_shape);
        assert_eq!(scales.row(0).as_slice().unwrap(), reference.scale_factor);
        assert_tensor_matches(fixture.name, pixels.index_axis(Axis(0), 0), &expected);
    }
}

#[test]
fn triangle_negative_control_is_rejected_on_every_resized_reference() {
    for fixture in FIXTURES.iter().filter(|fixture| fixture.name != "identity") {
        let (_, expected) = read_reference(fixture);
        let wrong = preprocessing::preprocess_rescale(&synthetic_rgb(fixture), INPUT_SIZE);
        assert!(
            max_pixel_error(wrong.index_axis(Axis(0), 0), &expected) > 1.0,
            "{}: Triangle control",
            fixture.name
        );
    }
}

#[test]
fn image_catmull_rom_negative_control_is_not_opencv_cubic() {
    for fixture in FIXTURES.iter().filter(|fixture| fixture.name != "identity") {
        let (_, expected) = read_reference(fixture);
        let wrong = image::imageops::resize(
            &synthetic_rgb(fixture),
            INPUT_SIZE,
            INPUT_SIZE,
            image::imageops::FilterType::CatmullRom,
        );
        let maximum = wrong
            .pixels()
            .zip(expected.pixels())
            .flat_map(|(actual, expected)| actual.0.into_iter().zip(expected.0))
            .map(|(actual, expected)| actual.abs_diff(expected))
            .max()
            .unwrap();
        assert!(maximum > 1, "{}: Catmull-Rom control", fixture.name);
    }
}
