use std::sync::{Arc, Mutex};

use image::{Rgb, RgbImage};
use ndarray::{Array, arr1, arr2};

use super::{INPUT_SIZE, PpDocLayoutV3Model};
use crate::inference::{InferenceError, InferenceSession, InferenceTensor};
use crate::layout::inference_timings;
use crate::layout::models::LayoutModel;
use crate::layout::types::{BBox, LayoutClass};

const INPUT_NAMES: [&str; 3] = ["im_shape", "image", "scale_factor"];
const OUTPUT_NAMES: [&str; 3] = ["fetch_name_2", "fetch_name_1", "fetch_name_0"];

#[derive(Debug)]
struct Forward {
    inputs: Vec<(String, Vec<usize>)>,
    marker: f32,
    im_shape: Vec<f32>,
    scale_factor: Vec<f32>,
    outputs: Vec<String>,
}

type Forwards = Arc<Mutex<Vec<Forward>>>;

struct SingletonSession {
    input_names: Vec<String>,
    forwards: Forwards,
    fail_at: Option<usize>,
}

fn input<'a>(inputs: &'a [(String, InferenceTensor)], name: &str) -> &'a ndarray::ArrayD<f32> {
    inputs
        .iter()
        .find(|(key, _)| key == name)
        .expect("all named inputs must be forwarded")
        .1
        .as_f32()
        .expect("input tensors must be f32")
}

fn forward(inputs: &[(String, InferenceTensor)]) -> Forward {
    Forward {
        inputs: inputs
            .iter()
            .map(|(name, tensor)| (name.clone(), tensor.as_f32().unwrap().shape().to_vec()))
            .collect(),
        marker: input(inputs, "image")[[0, 0, 0, 0]],
        im_shape: input(inputs, "im_shape").iter().copied().collect(),
        scale_factor: input(inputs, "scale_factor").iter().copied().collect(),
        outputs: Vec::new(),
    }
}

fn outputs(call: usize) -> Vec<(String, InferenceTensor)> {
    let class = if call == 1 { 3.0 } else { 22.0 };
    let left = call as f32;
    vec![
        (
            OUTPUT_NAMES[0].to_owned(),
            InferenceTensor::I32(Array::zeros((2, 200, 200)).into_dyn()),
        ),
        (OUTPUT_NAMES[1].to_owned(), InferenceTensor::I32(arr1(&[2]).into_dyn())),
        (
            OUTPUT_NAMES[2].to_owned(),
            InferenceTensor::F32(
                arr2(&[
                    [class, 0.75, left, 2.0, 20.0, 30.0, 0.0],
                    [21.0, 0.25, 3.0, 4.0, 15.0, 20.0, 0.0],
                ])
                .into_dyn(),
            ),
        ),
    ]
}

impl InferenceSession for SingletonSession {
    fn run(&self, inputs: Vec<(String, InferenceTensor)>) -> Result<Vec<(String, InferenceTensor)>, InferenceError> {
        let mut forwards = self.forwards.lock().unwrap();
        forwards.push(forward(&inputs));
        let call = forwards.len();
        if input(&inputs, "image").shape() != [1, 3, INPUT_SIZE as usize, INPUT_SIZE as usize] {
            return Err(InferenceError::Run(
                "masked V3 reference requires B1 forwards".to_owned(),
            ));
        }
        if self.fail_at == Some(call) {
            return Err(InferenceError::Run(format!("failure on singleton {call}")));
        }
        let outputs = outputs(call);
        forwards.last_mut().unwrap().outputs = outputs.iter().map(|(name, _)| name.clone()).collect();
        Ok(outputs)
    }

    fn input_names(&self) -> &[String] {
        &self.input_names
    }
}

fn model(fail_at: Option<usize>) -> (PpDocLayoutV3Model, Forwards) {
    let forwards = Arc::new(Mutex::new(Vec::new()));
    let input_names: Vec<_> = INPUT_NAMES.iter().map(|name| (*name).to_owned()).collect();
    let session = SingletonSession {
        input_names: input_names.clone(),
        forwards: forwards.clone(),
        fail_at,
    };
    let model = PpDocLayoutV3Model {
        session: Box::new(session),
        input_names,
    };
    (model, forwards)
}

fn images() -> [RgbImage; 2] {
    [
        RgbImage::from_pixel(37, 59, Rgb([51, 0, 0])),
        RgbImage::from_pixel(59, 37, Rgb([179, 0, 0])),
    ]
}

fn assert_forward(forward: &Forward, image: &RgbImage) {
    assert_eq!(
        forward.inputs,
        vec![
            ("im_shape".to_owned(), vec![1, 2]),
            ("image".to_owned(), vec![1, 3, 800, 800]),
            ("scale_factor".to_owned(), vec![1, 2]),
        ]
    );
    assert!((forward.marker - f32::from(image.get_pixel(0, 0).0[0]) / 255.0).abs() < 1e-6);
    assert_eq!(forward.im_shape, vec![800.0, 800.0]);
    assert_eq!(
        forward.scale_factor,
        vec![800.0 / image.height() as f32, 800.0 / image.width() as f32]
    );
    assert_eq!(forward.outputs, OUTPUT_NAMES.map(str::to_owned));
}

#[test]
fn should_dispatch_each_image_as_a_singleton_in_input_order() {
    let (mut model, forwards) = model(None);
    let images = images();
    let result = model.detect_batch(&[&images[0], &images[1]], Some(0.6)).unwrap();
    assert_eq!(result.len(), 2);
    for (index, class) in [LayoutClass::Chart, LayoutClass::Text].into_iter().enumerate() {
        assert_eq!(result[index].len(), 1);
        assert_eq!(result[index][0].class_name, class);
        assert_eq!(result[index][0].confidence, 0.75);
        assert_eq!(result[index][0].bbox, BBox::new(index as f32 + 1.0, 2.0, 20.0, 30.0));
    }
    let forwards = forwards.lock().unwrap();
    assert_eq!(forwards.len(), 2);
    assert_eq!(forwards.iter().map(|call| call.inputs.len()).sum::<usize>(), 6);
    assert_eq!(forwards.iter().map(|call| call.outputs.len()).sum::<usize>(), 6);
    for (call, image) in forwards.iter().zip(images.iter()) {
        assert_forward(call, image);
    }
    let (preprocess, inference) = inference_timings::take();
    assert!(preprocess.is_finite() && preprocess > 0.0);
    assert!(inference.is_finite() && inference > 0.0);
    assert_eq!(inference_timings::take(), (0.0, 0.0));
}

#[test]
fn should_return_empty_without_forwarding_or_stale_timings() {
    let (mut model, forwards) = model(None);
    inference_timings::set(17.0, 23.0);
    assert!(model.detect_batch(&[], None).unwrap().is_empty());
    assert!(forwards.lock().unwrap().is_empty());
    assert_eq!(inference_timings::take(), (0.0, 0.0));
}

#[test]
fn should_stop_at_the_first_singleton_error_without_leaking_timings() {
    let (mut model, forwards) = model(Some(2));
    let images = images();
    inference_timings::set(17.0, 23.0);
    let error = model
        .detect_batch(&[&images[0], &images[1], &images[0]], None)
        .unwrap_err();
    assert!(error.to_string().contains("failure on singleton 2"), "{error}");
    let forwards = forwards.lock().unwrap();
    assert_eq!(forwards.len(), 2, "the third image must never be forwarded");
    assert_eq!(forwards.iter().map(|call| call.outputs.len()).sum::<usize>(), 3);
    assert_eq!(inference_timings::take(), (0.0, 0.0));
}

#[test]
fn should_preserve_singleton_default_and_explicit_thresholds() {
    let (mut model, forwards) = model(None);
    let image = &images()[0];
    let default = model.detect_batch(&[image], None).unwrap();
    assert_eq!(default.len(), 1);
    assert_eq!(default[0].len(), 1);
    let lower_threshold = model.detect_batch(&[image], Some(0.2)).unwrap();
    assert_eq!(lower_threshold.len(), 1);
    assert_eq!(lower_threshold[0].len(), 2);
    assert_eq!(lower_threshold[0][1].class_name, LayoutClass::Table);
    assert_eq!(lower_threshold[0][1].confidence, 0.25);
    assert_eq!(forwards.lock().unwrap().len(), 2);
    inference_timings::take();
}
