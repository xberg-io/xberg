//! Evidence for the Florence-2 refusal: the pinned graphs are loaded with the bundled ONNX Runtime
//! and the merged decoder is run on both its uncached and cached branches. These tests are ignored
//! by default because they need the staged multi-hundred-megabyte graphs; they are a diagnostic,
//! not a gate. On ONNX Runtime 1.31.0 (`ort` 2.0.0-rc.15 `download-binaries`) every pinned graph
//! loads and the decoder runs both branches, so the family stays refused for pipeline scope rather
//! than a demonstrated runtime failure.

use std::sync::Arc;

use ort::session::SessionInputValue;
use ort::value::Tensor;

use super::artifacts::read_artifacts;
use super::control::{RequestState, WorkerControl};
use super::preprocess::{PreprocessConfig, preprocess};
use super::{LocalCaptioningFormat, LocalCaptioningModel, LocalCaptioningPrecision};

fn control() -> WorkerControl {
    WorkerControl {
        state: Arc::new(RequestState::default()),
        options: Default::default(),
        started: std::time::Instant::now(),
    }
}

fn f32_value(shape: Vec<i64>, data: Vec<f32>) -> SessionInputValue<'static> {
    Tensor::from_array((shape, data)).unwrap().into()
}

fn i64_value(shape: Vec<i64>, data: Vec<i64>) -> SessionInputValue<'static> {
    Tensor::from_array((shape, data)).unwrap().into()
}

/// One merged-decoder input set. `past` is either empty or the present tensors of the prior step.
fn decoder_inputs(
    names: &[String],
    embeds: (Vec<i64>, Vec<f32>),
    attention: Vec<i64>,
    position: Vec<i64>,
    context_tokens: usize,
    use_cache: bool,
    past: &[(String, (Vec<i64>, Vec<f32>))],
) -> Vec<(String, SessionInputValue<'static>)> {
    names
        .iter()
        .map(|name| {
            let value: SessionInputValue<'static> = if let Some((_, tensor)) =
                past.iter().find(|(input, _)| input == name)
            {
                f32_value(tensor.0.clone(), tensor.1.clone())
            } else {
                match name.as_str() {
                    "inputs_embeds" => f32_value(embeds.0.clone(), embeds.1.clone()),
                    "attention_mask" => i64_value(vec![1, attention.len() as i64], attention.clone()),
                    "position_ids" => i64_value(vec![1, position.len() as i64], position.clone()),
                    "encoder_attention_mask" => i64_value(vec![1, context_tokens as i64], vec![1i64; context_tokens]),
                    "encoder_hidden_states" => {
                        f32_value(vec![1, context_tokens as i64, 768], vec![0.0f32; context_tokens * 768])
                    }
                    "use_cache_branch" => Tensor::from_array((vec![1usize], vec![use_cache])).unwrap().into(),
                    other => panic!("unexpected decoder input {other}"),
                }
            };
            (name.clone(), value)
        })
        .collect()
}

#[test]
#[ignore = "diagnostic: requires staged Florence-2 artifacts"]
fn florence_merged_decoder_diagnostic() {
    for precision in [LocalCaptioningPrecision::Quantized, LocalCaptioningPrecision::Fp32] {
        println!("== {precision:?}");
        run_decoder_branches(precision);
    }
}

fn run_decoder_branches(precision: LocalCaptioningPrecision) {
    let model = LocalCaptioningModel::staged("florence-2-base", precision).unwrap();
    let config = super::tests::config(model);
    let control = control();
    let artifacts = read_artifacts(&config, &control).unwrap();
    let mut decoder = match super::runtime::build_session(&artifacts.decoder, "decoder") {
        Ok(session) => session,
        Err(error) => {
            println!("decoder load FAILED: {error}");
            return;
        }
    };
    for (label, bytes) in [("vision", &artifacts.vision), ("embed", &artifacts.embed)] {
        if let Err(error) = super::runtime::build_session(bytes, label) {
            println!("{label} load FAILED: {error}");
        }
    }
    let names: Vec<String> = decoder.inputs().iter().map(|input| input.name().to_owned()).collect();
    let contexts = 583;
    let values = decoder_inputs(
        &names,
        (vec![1, 2, 768], vec![0.0f32; 2 * 768]),
        vec![1, 1],
        vec![0, 1],
        contexts,
        false,
        &[],
    );
    let present: Vec<(String, (Vec<i64>, Vec<f32>))> = {
        let options = ort::session::RunOptions::new().unwrap();
        let outputs = match decoder.run_with_options(values, &options) {
            Ok(outputs) => outputs,
            Err(error) => {
                println!("uncached step FAILED: {error}");
                return;
            }
        };
        outputs
            .keys()
            .filter_map(|name| {
                let suffix = name.strip_prefix("present.")?;
                let (shape, data) = outputs.get(name)?.try_extract_tensor::<f32>().ok()?;
                Some((
                    format!("past_key_values.{suffix}"),
                    (shape.iter().copied().collect(), data.to_vec()),
                ))
            })
            .collect()
    };
    let values = decoder_inputs(
        &names,
        (vec![1, 1, 768], vec![0.0f32; 768]),
        vec![1, 1, 1],
        vec![2],
        contexts,
        true,
        &present,
    );
    let options = ort::session::RunOptions::new().unwrap();
    match decoder.run_with_options(values, &options) {
        Ok(_) => println!("cached step: ok ({} present tensors)", present.len()),
        Err(error) => println!("cached step FAILED: {error}"),
    }
}

/// Runs the full path the adapter would need: vision encoder, text encoder, token embedding and a
/// merged decoder step over the real concatenated encoder context.
#[test]
#[ignore = "diagnostic: requires staged Florence-2 artifacts"]
fn florence_pipeline_diagnostic() {
    let model = LocalCaptioningModel::staged("florence-2-base", LocalCaptioningPrecision::Quantized).unwrap();
    let config = super::tests::config(model.clone());
    let control = control();
    let artifacts = read_artifacts(&config, &control).unwrap();
    let preprocessor: serde_json::Value =
        serde_json::from_slice(artifacts.companions.get("preprocessor_config.json").unwrap()).unwrap();
    let processor: serde_json::Value =
        serde_json::from_slice(artifacts.companions.get("processor_config.json").unwrap()).unwrap();
    let preprocess_config =
        PreprocessConfig::from_json(LocalCaptioningFormat::Florence2, &preprocessor, &processor).unwrap();
    let image = image::RgbImage::from_pixel(800, 600, image::Rgb([128, 128, 128]));
    let batch = preprocess(&config, LocalCaptioningFormat::Florence2, &preprocess_config, &image).unwrap();
    println!("pixels: {} frames at {}", batch.num_images, batch.edge);

    let mut vision = super::runtime::build_session(&artifacts.vision, "vision").unwrap();
    let mut embed = super::runtime::build_session(&artifacts.embed, "embed").unwrap();
    let mut encoder = super::runtime::build_session(&read_text_encoder(&model), "text encoder").unwrap();
    let mut decoder = super::runtime::build_session(&artifacts.decoder, "decoder").unwrap();

    let values: Vec<(String, SessionInputValue<'static>)> = vision
        .inputs()
        .iter()
        .map(|input| {
            let value: SessionInputValue<'static> = match input.name() {
                "pixel_values" => f32_value(
                    vec![1, 3, i64::from(batch.edge), i64::from(batch.edge)],
                    batch.pixels.clone(),
                ),
                other => panic!("vision input {other}"),
            };
            (input.name().to_owned(), value)
        })
        .collect();
    let options = ort::session::RunOptions::new().unwrap();
    let outputs = vision.run_with_options(values, &options).unwrap();
    let (image_shape, image_features) = first_f32(&outputs);
    println!("vision features: {image_shape:?}");
    drop(outputs);

    let tokenizer = tokenizers::Tokenizer::from_bytes(artifacts.companions.get("tokenizer.json").unwrap()).unwrap();
    let prompt_ids: Vec<i64> = tokenizer
        .encode("<CAPTION>", false)
        .unwrap()
        .get_ids()
        .iter()
        .map(|id| i64::from(*id))
        .collect();
    let (prompt_shape, prompt_embeds) = embed_ids(&mut embed, &prompt_ids);
    let encoder_values: Vec<(String, SessionInputValue<'static>)> = encoder
        .inputs()
        .iter()
        .map(|input| {
            let value: SessionInputValue<'static> = match input.name() {
                "inputs_embeds" => f32_value(prompt_shape.clone(), prompt_embeds.clone()),
                "attention_mask" => i64_value(vec![1, prompt_ids.len() as i64], vec![1i64; prompt_ids.len()]),
                other => panic!("text encoder input {other}"),
            };
            (input.name().to_owned(), value)
        })
        .collect();
    let options = ort::session::RunOptions::new().unwrap();
    let (text_shape, text_data) = match encoder.run_with_options(encoder_values, &options) {
        Ok(outputs) => first_f32(&outputs),
        Err(error) => {
            println!("text encoder FAILED: {error}");
            return;
        }
    };
    println!("text encoder: {text_shape:?}");
    let mut context = image_features;
    context.extend_from_slice(&text_data);
    let context_tokens = text_shape[1] as usize + image_shape[1] as usize;

    let start = tokenizer.token_to_id("</s>").map(i64::from).unwrap_or(2);
    let (embed_shape, embeds) = embed_ids(&mut embed, &[start]);
    let names: Vec<String> = decoder.inputs().iter().map(|input| input.name().to_owned()).collect();
    let values = decoder_inputs(
        &names,
        (embed_shape, embeds),
        vec![1],
        vec![0],
        context_tokens,
        false,
        &[],
    );
    let values: Vec<(String, SessionInputValue<'static>)> = values
        .into_iter()
        .map(|(name, value)| {
            if name == "encoder_hidden_states" {
                (name, f32_value(vec![1, context_tokens as i64, 768], context.clone()))
            } else {
                (name, value)
            }
        })
        .collect();
    let options = ort::session::RunOptions::new().unwrap();
    match decoder.run_with_options(values, &options) {
        Ok(outputs) => {
            let (shape, _) = first_f32(&outputs);
            println!("decoder full-context step: ok, logits shape {shape:?}");
        }
        Err(error) => println!("decoder full-context step FAILED: {error}"),
    }
}

fn first_f32(outputs: &ort::session::SessionOutputs<'_>) -> (Vec<i64>, Vec<f32>) {
    let value = outputs
        .get("last_hidden_state")
        .or_else(|| outputs.get("image_features"))
        .or_else(|| outputs.get("inputs_embeds"))
        .or_else(|| outputs.get("logits"))
        .or_else(|| outputs.keys().next().and_then(|name| outputs.get(name)))
        .unwrap();
    let (shape, data) = value.try_extract_tensor::<f32>().unwrap();
    (shape.iter().copied().collect(), data.to_vec())
}

fn embed_ids(embed: &mut ort::session::Session, ids: &[i64]) -> (Vec<i64>, Vec<f32>) {
    let values: Vec<(String, SessionInputValue<'static>)> = embed
        .inputs()
        .iter()
        .map(|input| {
            let value: SessionInputValue<'static> = match input.name() {
                "input_ids" => i64_value(vec![1, ids.len() as i64], ids.to_vec()),
                other => panic!("embed input {other}"),
            };
            (input.name().to_owned(), value)
        })
        .collect();
    let options = ort::session::RunOptions::new().unwrap();
    let outputs = embed.run_with_options(values, &options).unwrap();
    first_f32(&outputs)
}

fn read_text_encoder(model: &LocalCaptioningModel) -> Vec<u8> {
    let path = model
        .text_encoder_file
        .as_deref()
        .expect("Florence-2 pins a text encoder");
    let pin = model.files.iter().find(|file| file.path == path).unwrap();
    let client = crate::model_download::hf_client(None).unwrap();
    let cached = crate::model_download::hf_cached_revision_with_client(
        &client,
        &model.repository,
        &pin.path,
        Some(&model.revision),
    )
    .unwrap()
    .unwrap();
    std::fs::read(cached).unwrap()
}
