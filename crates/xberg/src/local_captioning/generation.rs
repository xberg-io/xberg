use std::collections::BTreeSet;
use std::sync::Arc;

use ort::session::{RunOptions, SessionInputValue};
use ort::value::Tensor;

use super::control::WorkerControl;
use super::policy::invalid;
use super::preprocess::{ImageBatch, PreprocessConfig};
use super::types::LocalCaptioningFormat;

/// Key/value cache geometry read from the pinned text config.
#[derive(Debug, Clone, Copy)]
pub(super) struct KvShape {
    pub(super) layers: usize,
    pub(super) heads: usize,
    pub(super) head_dim: usize,
}

struct ActiveRun<'a>(&'a super::control::RequestState);

impl Drop for ActiveRun<'_> {
    fn drop(&mut self) {
        self.0.clear_run_options();
    }
}

fn run_options(control: &WorkerControl) -> crate::Result<Arc<RunOptions>> {
    let options =
        Arc::new(RunOptions::new().map_err(|error| invalid(format!("Local captioning run options: {error}")))?);
    control.state.register_run_options(Arc::clone(&options))?;
    Ok(options)
}

#[derive(Clone)]
pub(super) struct KvTensor {
    pub(super) suffix: String,
    pub(super) shape: Vec<i64>,
    pub(super) data: Vec<f32>,
}

pub(super) struct PreparedCaptioner {
    pub(super) format: LocalCaptioningFormat,
    pub(super) preprocess: PreprocessConfig,
    pub(super) tokenizer: tokenizers::Tokenizer,
    pub(super) vision: ort::session::Session,
    pub(super) embed: ort::session::Session,
    pub(super) decoder: ort::session::Session,
    pub(super) kv: KvShape,
    /// Cache input suffixes (`0.key` for SmolVLM, `0.decoder.key`/`0.encoder.key` for Florence-2).
    pub(super) past_suffixes: Vec<String>,
    pub(super) image_token_id: i64,
    pub(super) eos_token_ids: BTreeSet<i64>,
    pub(super) hidden: usize,
}

fn input_names(session: &ort::session::Session) -> Vec<String> {
    session.inputs().iter().map(|input| input.name().to_owned()).collect()
}

fn output_names(session: &ort::session::Session) -> Vec<String> {
    session
        .outputs()
        .iter()
        .map(|output| output.name().to_owned())
        .collect()
}

fn f32_tensor(shape: impl Into<Vec<i64>>, data: Vec<f32>) -> crate::Result<Tensor<f32>> {
    Tensor::from_array((shape.into(), data)).map_err(|error| invalid(format!("Local captioning tensor: {error}")))
}

fn i64_tensor(shape: impl Into<Vec<i64>>, data: Vec<i64>) -> crate::Result<Tensor<i64>> {
    Tensor::from_array((shape.into(), data)).map_err(|error| invalid(format!("Local captioning tensor: {error}")))
}

fn extract_f32<'a>(
    outputs: &'a ort::session::SessionOutputs<'_>,
    name: &str,
    label: &str,
) -> crate::Result<(Vec<i64>, &'a [f32])> {
    let output = outputs
        .get(name)
        .ok_or_else(|| invalid(format!("Local captioning {label} output {name} is missing")))?;
    let (shape, values) = output
        .try_extract_tensor::<f32>()
        .map_err(|error| invalid(format!("Local captioning {label} tensor: {error}")))?;
    Ok((shape.iter().copied().collect(), values))
}

fn first_output(session: &ort::session::Session, prefer: &str) -> String {
    let names = output_names(session);
    if names.iter().any(|name| name == prefer) {
        return prefer.to_owned();
    }
    names.first().cloned().unwrap_or_default()
}

/// Runs the vision encoder and returns `[pixels, hidden]` flattened features plus the token count.
fn run_vision(
    prepared: &mut PreparedCaptioner,
    batch: &ImageBatch,
    control: &WorkerControl,
) -> crate::Result<(Vec<f32>, usize, usize)> {
    control.check()?;
    // Idefics3 emits `[batch, num_images, 3, height, width]`; Florence-2 emits `[batch, 3, h, w]`.
    let shape = match prepared.format {
        LocalCaptioningFormat::SmolVlm => vec![
            1,
            i64::try_from(batch.num_images).unwrap_or(i64::MAX),
            3,
            i64::from(batch.edge),
            i64::from(batch.edge),
        ],
        LocalCaptioningFormat::Florence2 => vec![1, 3, i64::from(batch.edge), i64::from(batch.edge)],
    };
    let mut values: Vec<(String, SessionInputValue<'static>)> = Vec::new();
    for name in input_names(&prepared.vision) {
        let value: SessionInputValue<'static> = match name.as_str() {
            "pixel_values" => f32_tensor(shape.clone(), batch.pixels.clone())?.into(),
            "pixel_attention_mask" => {
                let plane = (batch.edge as usize) * (batch.edge as usize);
                let mask = vec![true; batch.num_images * plane];
                Tensor::from_array((
                    vec![1usize, batch.num_images, batch.edge as usize, batch.edge as usize],
                    mask,
                ))
                .map_err(|error| invalid(format!("Local captioning pixel mask: {error}")))?
                .into()
            }
            other => {
                return Err(invalid(format!(
                    "Local captioning vision encoder has an unsupported input {other}"
                )));
            }
        };
        values.push((name, value));
    }
    let preferred = first_output(&prepared.vision, "image_hidden_states");
    let options = run_options(control)?;
    let _active = ActiveRun(&control.state);
    let outputs = prepared
        .vision
        .run_with_options(values, options.as_ref())
        .map_err(|error| invalid(format!("Local captioning vision inference: {error}")))?;
    control.check()?;
    let (shape, values) = extract_f32(&outputs, &preferred, "vision")?;
    if shape.len() != 3 && shape.len() != 2 {
        return Err(invalid("Local captioning vision output must be a feature tensor"));
    }
    let hidden = usize::try_from(*shape.last().unwrap_or(&0))
        .map_err(|_| invalid("Local captioning vision output has no hidden size"))?;
    let tokens: usize = shape[..shape.len() - 1]
        .iter()
        .try_fold(1usize, |total, dimension| {
            usize::try_from(*dimension)
                .ok()
                .and_then(|value| total.checked_mul(value))
        })
        .ok_or_else(|| invalid("Local captioning vision output has an invalid token count"))?;
    if values.len() != tokens * hidden || values.iter().any(|value| !value.is_finite()) {
        return Err(invalid("Local captioning vision output is not a finite feature tensor"));
    }
    Ok((values.to_vec(), tokens, hidden))
}

fn run_embed(prepared: &mut PreparedCaptioner, ids: &[i64], control: &WorkerControl) -> crate::Result<Vec<f32>> {
    control.check()?;
    if !input_names(&prepared.embed).iter().any(|name| name == "input_ids") {
        return Err(invalid("Local captioning embedding graph has no input_ids"));
    }
    let tensor = i64_tensor(
        vec![1, i64::try_from(ids.len()).map_err(|_| invalid("prompt too long"))?],
        ids.to_vec(),
    )?;
    let preferred = first_output(&prepared.embed, "inputs_embeds");
    let options = run_options(control)?;
    let _active = ActiveRun(&control.state);
    let outputs = prepared
        .embed
        .run_with_options(
            vec![("input_ids".to_owned(), SessionInputValue::from(tensor))],
            options.as_ref(),
        )
        .map_err(|error| invalid(format!("Local captioning embedding inference: {error}")))?;
    control.check()?;
    let (shape, values) = extract_f32(&outputs, &preferred, "embedding")?;
    if shape.len() != 3 || values.len() != ids.len() * prepared.hidden {
        return Err(invalid("Local captioning embedding output must be [1, tokens, hidden]"));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(invalid("Local captioning embedding output is not finite"));
    }
    Ok(values.to_vec())
}

fn zero_past(suffixes: &[String], kv: KvShape) -> Vec<KvTensor> {
    let shape = vec![1, kv.heads as i64, 0, kv.head_dim as i64];
    suffixes
        .iter()
        .map(|suffix| KvTensor {
            suffix: suffix.clone(),
            shape: shape.clone(),
            data: Vec::new(),
        })
        .collect()
}

fn extract_present(outputs: &ort::session::SessionOutputs<'_>, suffix: &str) -> crate::Result<KvTensor> {
    let expected = format!("present.{suffix}");
    let found = if outputs.get(&expected).is_some() {
        Some(expected.clone())
    } else {
        // Some exports prefix cache outputs for the decoder or encoder stack.
        outputs
            .keys()
            .find(|candidate| candidate.ends_with(&format!(".{suffix}")))
            .map(str::to_owned)
    };
    let name = found.ok_or_else(|| invalid(format!("Local captioning decoder output {expected} is missing")))?;
    let (shape, data) = extract_f32(outputs, &name, "decoder cache")?;
    Ok(KvTensor {
        suffix: suffix.to_owned(),
        shape,
        data: data.to_vec(),
    })
}

struct DecoderStep {
    embeds: Vec<f32>,
    sequence: usize,
    positions: Vec<i64>,
    attention: Vec<i64>,
}

fn decoder_value(
    prepared: &PreparedCaptioner,
    step: &DecoderStep,
    past: &[KvTensor],
    use_cache: bool,
    name: &str,
) -> crate::Result<SessionInputValue<'static>> {
    let value: SessionInputValue<'static> = match name {
        "inputs_embeds" => {
            let shape = vec![
                1,
                i64::try_from(step.sequence).map_err(|_| invalid("prompt too long"))?,
                i64::try_from(prepared.hidden).map_err(|_| invalid("hidden size too large"))?,
            ];
            f32_tensor(shape, step.embeds.clone())?.into()
        }
        "attention_mask" => i64_tensor(vec![1, step.attention.len() as i64], step.attention.clone())?.into(),
        "position_ids" => i64_tensor(vec![1, step.positions.len() as i64], step.positions.clone())?.into(),
        "use_cache_branch" => Tensor::from_array((vec![1usize], vec![use_cache]))
            .map_err(|error| invalid(format!("Local captioning cache branch: {error}")))?
            .into(),
        other if other.starts_with("past_key_values.") => {
            let suffix = other.trim_start_matches("past_key_values.");
            let tensor = past
                .iter()
                .find(|tensor| tensor.suffix == suffix)
                .ok_or_else(|| invalid(format!("Local captioning decoder cache input {other} is missing")))?;
            f32_tensor(tensor.shape.clone(), tensor.data.clone())?.into()
        }
        other => {
            return Err(invalid(format!(
                "Local captioning decoder has an unsupported input {other}"
            )));
        }
    };
    Ok(value)
}

fn run_decoder(
    prepared: &mut PreparedCaptioner,
    step: &DecoderStep,
    past: &[KvTensor],
    use_cache: bool,
    control: &WorkerControl,
) -> crate::Result<(Vec<f32>, Vec<KvTensor>)> {
    control.check()?;
    let names = input_names(&prepared.decoder);
    let values = names
        .iter()
        .map(|name| {
            let value = decoder_value(prepared, step, past, use_cache, name)?;
            Ok((name.clone(), value))
        })
        .collect::<crate::Result<Vec<_>>>()?;
    let preferred = first_output(&prepared.decoder, "logits");
    let options = run_options(control)?;
    let _active = ActiveRun(&control.state);
    let outputs = prepared
        .decoder
        .run_with_options(values, options.as_ref())
        .map_err(|error| invalid(format!("Local captioning decoder inference: {error}")))?;
    control.check()?;
    let mut present = Vec::with_capacity(prepared.past_suffixes.len());
    for suffix in &prepared.past_suffixes {
        present.push(extract_present(&outputs, suffix)?);
    }
    let (shape, logits) = extract_f32(&outputs, &preferred, "decoder")?;
    if shape.len() != 3 || logits.iter().any(|value| !value.is_finite()) {
        return Err(invalid(
            "Local captioning decoder logits must be finite [1, tokens, vocab]",
        ));
    }
    Ok((logits.to_vec(), present))
}

fn argmax(values: &[f32]) -> usize {
    values.iter().enumerate().fold(
        0,
        |best, (index, value)| if *value > values[best] { index } else { best },
    )
}

/// Greedy decode from a prepared prompt. The `prompt_ids` include every image token; SmolVLM
/// substitutes the encoder features at those positions, Florence-2 prepends them to the encoder.
pub(super) fn generate(
    prepared: &mut PreparedCaptioner,
    batch: &ImageBatch,
    prompt: Option<&str>,
    max_new_tokens: u32,
    control: &WorkerControl,
) -> crate::Result<String> {
    let (features, feature_tokens, hidden) = run_vision(prepared, batch, control)?;
    if hidden != prepared.hidden {
        return Err(invalid(format!(
            "Local captioning vision hidden size {hidden} does not match the text hidden size {}",
            prepared.hidden
        )));
    }
    let expected_tokens = prepared.preprocess.image_seq_len as usize * batch.num_images;
    if feature_tokens != expected_tokens {
        return Err(invalid(format!(
            "Local captioning vision output has {feature_tokens} image tokens, expected {expected_tokens} from the pinned processor"
        )));
    }
    match prepared.format {
        LocalCaptioningFormat::SmolVlm => {
            generate_smolvlm(prepared, &features, batch.num_images, prompt, max_new_tokens, control)
        }
        LocalCaptioningFormat::Florence2 => Err(invalid(
            "Florence-2 captioning is not enabled by this adapter; only SmolVLM is wired",
        )),
    }
}

fn smolvlm_prompt(prepared: &PreparedCaptioner, num_images: usize, prompt: Option<&str>) -> crate::Result<Vec<i64>> {
    let text = smolvlm_prompt_text(num_images, prompt, prepared.preprocess.image_seq_len)?;
    let ids: Vec<i64> = prepared
        .tokenizer
        .encode(text.as_str(), false)
        .map_err(|error| invalid(format!("Local captioning prompt tokenization: {error}")))?
        .get_ids()
        .iter()
        .map(|id| i64::from(*id))
        .collect();
    Ok(ids)
}

/// The pinned Idefics3 chat expansion for one global frame. The image block is opened by
/// `<fake_token_around_image><global-img>`, holds `image_seq_len * num_images` repeated image
/// tokens, and is closed by `<fake_token_around_image>` before the question. This adapter
/// deliberately uses one global frame instead of the export's tile splitting, so no
/// `<row_N_col_M>` tags are inserted; that choice is a documented qualification gap.
pub(super) fn smolvlm_prompt_text(
    num_images: usize,
    prompt: Option<&str>,
    image_seq_len: u32,
) -> crate::Result<String> {
    if num_images != 1 {
        return Err(invalid(
            "Local captioning SmolVLM prompt supports one global image frame",
        ));
    }
    let question = prompt.unwrap_or("Can you describe this image?");
    let image_tokens = image_seq_len as usize * num_images;
    Ok(format!(
        "<|im_start|>User:<fake_token_around_image><global-img>{images}<fake_token_around_image>{question}<end_of_utterance>\nAssistant:",
        images = "<image>".repeat(image_tokens)
    ))
}

fn generate_smolvlm(
    prepared: &mut PreparedCaptioner,
    features: &[f32],
    num_images: usize,
    prompt: Option<&str>,
    max_new_tokens: u32,
    control: &WorkerControl,
) -> crate::Result<String> {
    let ids = smolvlm_prompt(prepared, num_images, prompt)?;
    let mut embeds = run_embed(prepared, &ids, control)?;
    merge_image_features(prepared.image_token_id, prepared.hidden, &ids, features, &mut embeds)?;
    let sequence = ids.len();
    let mut past = zero_past(&prepared.past_suffixes, prepared.kv);
    let mut generated = Vec::new();
    let mut all_positions: Vec<i64> = (0..sequence as i64).collect();
    let mut attention = vec![1i64; sequence];
    let mut step = DecoderStep {
        embeds,
        sequence,
        positions: all_positions.clone(),
        attention: attention.clone(),
    };
    let mut use_cache = false;
    for _ in 0..max_new_tokens {
        control.check()?;
        let (logits, next_past) = run_decoder(prepared, &step, &past, use_cache, control)?;
        past = next_past;
        use_cache = true;
        let tokens = step.sequence.max(1);
        let vocab = logits.len() / tokens;
        if vocab == 0 {
            return Err(invalid("Local captioning decoder returned empty logits"));
        }
        // Read the logits of the last fed position, not of the prompt length captured before
        // the loop: a cached step feeds one embedding but the prompt may be long.
        let last = &logits[logits.len() - vocab..];
        let token = i64::try_from(argmax(last)).map_err(|_| invalid("Local captioning token id overflow"))?;
        if prepared.eos_token_ids.contains(&token) {
            break;
        }
        generated.push(token);
        if generated.len() as u32 >= max_new_tokens {
            break;
        }
        let next = run_embed(prepared, &[token], control)?;
        let position = i64::try_from(all_positions.len()).map_err(|_| invalid("prompt too long"))?;
        all_positions.push(position);
        // The merged decoder needs the full, growing attention prefix even when a cached step
        // feeds one embedding; masking to length one makes it ignore the prompt and repeat.
        attention.push(1);
        step = DecoderStep {
            embeds: next,
            sequence: 1,
            positions: vec![position],
            attention: attention.clone(),
        };
    }
    decode(prepared, &generated)
}

fn merge_image_features(
    image_token_id: i64,
    hidden: usize,
    ids: &[i64],
    features: &[f32],
    embeds: &mut [f32],
) -> crate::Result<()> {
    let positions: Vec<usize> = ids
        .iter()
        .enumerate()
        .filter(|(_, id)| **id == image_token_id)
        .map(|(index, _)| index)
        .collect();
    if positions.len() != features.len() / hidden {
        return Err(invalid(format!(
            "Local captioning prompt has {} image tokens but the vision encoder produced {}",
            positions.len(),
            features.len() / hidden
        )));
    }
    for (slot, position) in positions.iter().enumerate() {
        let source = &features[slot * hidden..(slot + 1) * hidden];
        let target = &mut embeds[position * hidden..(position + 1) * hidden];
        target.copy_from_slice(source);
    }
    Ok(())
}

fn decode(prepared: &PreparedCaptioner, generated: &[i64]) -> crate::Result<String> {
    let ids: Vec<u32> = generated
        .iter()
        .map(|id| u32::try_from(*id).map_err(|_| invalid("Local captioning token id is invalid")))
        .collect::<crate::Result<Vec<_>>>()?;
    let caption = prepared
        .tokenizer
        .decode(&ids, true)
        .map_err(|error| invalid(format!("Local captioning decode: {error}")))?;
    Ok(caption.trim().to_owned())
}
