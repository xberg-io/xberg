use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
use ndarray::{Array, ArrayD, arr1, arr2};
use serde_json::{Value, json};

use super::super::INPUT_SIZE;
use super::{PpDocLayoutV3Model, bytes_sha256, capture_page, record_session};
use crate::inference::{InferenceError, InferenceSession, InferenceTensor};

const WIDTH: u32 = 7;
const HEIGHT: u32 = 5;
const ORIGINAL: [u8; 3] = [51, 0, 0];
const REPLACEMENT: [u8; 3] = [0, 0, 179];
const PAGE: &str = "synthetic-page";
const INPUT_NAMES: [&str; 3] = ["im_shape", "image", "scale_factor"];

struct Fixture {
    _directory: tempfile::TempDir,
    path: PathBuf,
    output: PathBuf,
    original: Vec<u8>,
    replacement: Vec<u8>,
    input: Value,
}

fn png(color: [u8; 3]) -> Vec<u8> {
    let image = RgbImage::from_pixel(WIDTH, HEIGHT, Rgb(color));
    let mut bytes = Vec::new();
    PngEncoder::new_with_quality(&mut bytes, CompressionType::Uncompressed, FilterType::NoFilter)
        .write_image(image.as_raw(), WIDTH, HEIGHT, ExtendedColorType::Rgb8)
        .expect("genuine uncompressed RGB PNG");
    bytes
}

fn assert_png(bytes: &[u8], color: [u8; 3]) {
    let image = image::load_from_memory(bytes)
        .expect("independently decoded PNG")
        .to_rgb8();
    assert_eq!(image.dimensions(), (WIDTH, HEIGHT));
    assert!(image.pixels().all(|pixel| pixel.0 == color));
}

fn fixture() -> Fixture {
    let directory = tempfile::tempdir().expect("owned PNG fixture");
    let original = png(ORIGINAL);
    let replacement = png(REPLACEMENT);
    assert_eq!(original.len(), replacement.len(), "equal-length genuine PNGs");
    assert_ne!(bytes_sha256(&original), bytes_sha256(&replacement));
    assert_png(&original, ORIGINAL);
    assert_png(&replacement, REPLACEMENT);
    let path = directory.path().join("input.png");
    fs::write(&path, &original).expect("original frozen PNG");
    let output = directory.path().join("capture");
    fs::create_dir(&output).expect("owned capture root");
    let input = json!({"name": PAGE, "path": path, "sha256": bytes_sha256(&original),
        "shape_hwc": [HEIGHT, WIDTH, 3]});
    Fixture {
        _directory: directory,
        path,
        output,
        original,
        replacement,
        input,
    }
}

#[derive(Clone, Default)]
struct Callback {
    count: Arc<AtomicUsize>,
    original_tensor_verified: Arc<AtomicBool>,
}

impl Callback {
    fn assert_one_original_forward(&self) {
        assert_eq!(self.count.load(Ordering::SeqCst), 1, "actual session callback");
        assert!(self.original_tensor_verified.load(Ordering::SeqCst));
    }
}

struct PngSession {
    names: Vec<String>,
    path: PathBuf,
    replacement: Option<Vec<u8>>,
    callback: Callback,
}

fn tensor<'a>(inputs: &'a [(String, InferenceTensor)], name: &str) -> &'a ArrayD<f32> {
    inputs
        .iter()
        .find(|(key, _)| key == name)
        .expect("actual named input")
        .1
        .as_f32()
        .expect("actual f32 input")
}

fn assert_original_tensor(values: impl Iterator<Item = f32>) {
    let plane = INPUT_SIZE as usize * INPUT_SIZE as usize;
    let mut count = 0;
    for (index, value) in values.enumerate() {
        let expected = f32::from(ORIGINAL[index / plane]) / 255.0;
        assert!((value - expected).abs() < 1e-6, "original uniform PNG channel");
        count += 1;
    }
    assert_eq!(count, plane * 3);
}

fn assert_original_inputs(inputs: &[(String, InferenceTensor)]) {
    assert_eq!(
        inputs.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(),
        INPUT_NAMES
    );
    let image = tensor(inputs, "image");
    assert_eq!(image.shape(), [1, 3, INPUT_SIZE as usize, INPUT_SIZE as usize]);
    assert_original_tensor(image.iter().copied());
    let shape = tensor(inputs, "im_shape");
    assert_eq!(shape.shape(), [1, 2]);
    assert_eq!(shape.iter().copied().collect::<Vec<_>>(), [800.0, 800.0]);
    let scale = tensor(inputs, "scale_factor");
    assert_eq!(scale.shape(), [1, 2]);
    assert_eq!(
        scale.iter().copied().collect::<Vec<_>>(),
        [INPUT_SIZE as f32 / HEIGHT as f32, INPUT_SIZE as f32 / WIDTH as f32]
    );
}

fn outputs() -> Vec<(String, InferenceTensor)> {
    vec![
        (
            "fetch_name_0".into(),
            InferenceTensor::F32(arr2(&[[3.0, 0.75, 0.0, 0.0, 4.0, 4.0, 0.0]]).into_dyn()),
        ),
        ("fetch_name_1".into(), InferenceTensor::I32(arr1(&[1]).into_dyn())),
        (
            "fetch_name_2".into(),
            InferenceTensor::I32(Array::zeros((1, 200, 200)).into_dyn()),
        ),
    ]
}

impl InferenceSession for PngSession {
    fn run(&self, inputs: Vec<(String, InferenceTensor)>) -> Result<Vec<(String, InferenceTensor)>, InferenceError> {
        self.callback.count.fetch_add(1, Ordering::SeqCst);
        assert_original_inputs(&inputs);
        self.callback.original_tensor_verified.store(true, Ordering::SeqCst);
        if let Some(replacement) = &self.replacement {
            fs::write(&self.path, replacement).expect("replace owned PNG during actual forward");
        }
        Ok(outputs())
    }

    fn input_names(&self) -> &[String] {
        &self.names
    }
}

fn model(fixture: &Fixture, mutate: bool) -> (PpDocLayoutV3Model, super::Captures, Callback) {
    let callback = Callback::default();
    let names = INPUT_NAMES.map(str::to_owned).to_vec();
    let session = PngSession {
        names: names.clone(),
        path: fixture.path.clone(),
        replacement: mutate.then(|| fixture.replacement.clone()),
        callback: callback.clone(),
    };
    let (session, captures) = record_session(Box::new(session));
    (
        PpDocLayoutV3Model {
            session,
            input_names: names,
        },
        captures,
        callback,
    )
}

fn assert_artifacts(fixture: &Fixture, row: &Value) {
    let mut count = 0;
    for group in ["inputs", "outputs"] {
        let records = row[group].as_array().expect("actual tensor artifact records");
        assert_eq!(records.len(), 3);
        let mut names = records
            .iter()
            .map(|record| record["name"].as_str().unwrap())
            .collect::<Vec<_>>();
        names.sort_unstable();
        let expected = if group == "inputs" {
            INPUT_NAMES
        } else {
            ["fetch_name_0", "fetch_name_1", "fetch_name_2"]
        };
        assert_eq!(names, expected);
        for record in records {
            let file = record["file"].as_str().expect("tensor artifact name");
            let bytes = fs::read(fixture.output.join(PAGE).join(group).join(file)).expect("actual tensor bytes");
            assert!(!bytes.is_empty());
            assert_eq!(record["bytes"], bytes.len());
            assert_eq!(record["sha256"], bytes_sha256(&bytes));
            count += 1;
        }
    }
    assert_eq!(count, 6);
    let pixels = fs::read(fixture.output.join(PAGE).join("inputs/image.bin")).expect("recorded input tensor");
    assert_eq!(
        pixels.len(),
        INPUT_SIZE as usize * INPUT_SIZE as usize * ORIGINAL.len() * std::mem::size_of::<f32>()
    );
    assert_original_tensor(
        pixels
            .chunks_exact(4)
            .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap())),
    );
}

#[test]
fn should_capture_an_unchanged_png_from_the_recorded_forward() {
    let fixture = fixture();
    let (mut model, captures, callback) = model(&fixture, false);
    let row = capture_page(&mut model, &captures, &fixture.output, &fixture.input);
    callback.assert_one_original_forward();
    assert!(captures.lock().unwrap().is_empty());
    assert_eq!(fs::read(&fixture.path).unwrap(), fixture.original);
    assert_eq!(row["input_sha256"], fixture.input["sha256"]);
    assert_eq!(row["processed_detections"].as_array().unwrap().len(), 1);
    assert_artifacts(&fixture, &row);
}

#[test]
fn should_reject_a_png_changed_by_the_forward_before_recording_artifacts() {
    let fixture = fixture();
    let (mut model, captures, callback) = model(&fixture, true);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        capture_page(&mut model, &captures, &fixture.output, &fixture.input)
    }));
    callback.assert_one_original_forward();
    let changed = fs::read(&fixture.path).expect("actual post-forward PNG");
    assert_eq!(changed, fixture.replacement);
    assert_eq!(changed.len(), fixture.original.len());
    assert_ne!(bytes_sha256(&changed), fixture.input["sha256"].as_str().unwrap());
    assert_png(&changed, REPLACEMENT);
    let error = result.expect_err("PNG changed during actual forward must reject before artifact recording");
    let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .expect("specific PNG identity rejection");
    assert!(message.contains("pinned PNG identity after forward"), "{message}");
    assert!(!fixture.output.join(PAGE).exists(), "no successful page artifacts");
}
