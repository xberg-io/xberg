use std::sync::Arc;

use ort::session::SessionInputValue;
use ort::value::Tensor;

use super::control::{RequestState, WorkerControl};
use super::decoding::decode_logits;
use super::policy::{NLI_CLASS_COUNT, invalid};
use super::preparation::PreparedModel;
use super::{LocalClassificationConfig, LocalClassificationResult};

struct ActiveRun<'a>(&'a RequestState);

impl Drop for ActiveRun<'_> {
    fn drop(&mut self) {
        self.0.clear_run_options();
    }
}

fn named_inputs(
    encoding: &tokenizers::Encoding,
    has_type_ids: bool,
) -> crate::Result<Vec<(String, SessionInputValue<'static>)>> {
    let mut inputs = Vec::with_capacity(3);
    for (name, values) in [
        ("input_ids", encoding.get_ids()),
        ("attention_mask", encoding.get_attention_mask()),
        ("token_type_ids", encoding.get_type_ids()),
    ] {
        if name == "token_type_ids" && !has_type_ids {
            continue;
        }
        let values = values.iter().map(|value| i64::from(*value)).collect::<Vec<_>>();
        let tensor = Tensor::from_array(([1usize, encoding.len()], values))
            .map_err(|error| invalid(format!("Local NLI token tensor: {error}")))?;
        inputs.push((name.to_owned(), tensor.into()));
    }
    Ok(inputs)
}

fn run_pair(
    model: &mut PreparedModel,
    encoding: &tokenizers::Encoding,
    output_name: &str,
    control: &WorkerControl,
) -> crate::Result<[f32; NLI_CLASS_COUNT]> {
    control.check()?;
    let inputs = named_inputs(encoding, model.has_type_ids)?;
    let options =
        Arc::new(ort::session::RunOptions::new().map_err(|error| invalid(format!("Local NLI run options: {error}")))?);
    control.state.register_run_options(Arc::clone(&options))?;
    let _active = ActiveRun(&control.state);
    let outputs = model
        .session
        .run_with_options(inputs, options.as_ref())
        .map_err(|error| invalid(format!("Local NLI CPU inference: {error}")))?;
    control.check()?;
    let output = outputs
        .get(output_name)
        .ok_or_else(|| invalid("Local NLI selected logits output is missing"))?;
    let (shape, values) = output
        .try_extract_tensor::<f32>()
        .map_err(|error| invalid(format!("Local NLI logits tensor: {error}")))?;
    if shape[..] != [1, NLI_CLASS_COUNT as i64]
        || values.len() != NLI_CLASS_COUNT
        || values.iter().any(|value| !value.is_finite())
    {
        return Err(invalid("Local NLI logits must be finite [1, 3] scores"));
    }
    Ok([values[0], values[1], values[2]])
}

pub(super) fn classify_prepared(
    model: &mut PreparedModel,
    config: &LocalClassificationConfig,
    text: &str,
    labels: &[String],
    control: &WorkerControl,
) -> crate::Result<LocalClassificationResult> {
    let mut bounded_config = config.clone();
    bounded_config.limits.max_tokens = model.max_tokens;
    let verifier = super::tokenization::source_tokenizer(&model.tokenizer);
    let mut logits = Vec::with_capacity(labels.len() * NLI_CLASS_COUNT);
    for label in labels {
        control.check()?;
        let encoding = super::tokenization::encode_pair(&model.tokenizer, &verifier, text, label, &bounded_config)?;
        control.check()?;
        logits.extend_from_slice(&run_pair(model, &encoding, &config.model.output_name, control)?);
    }
    let logits = ndarray::ArrayD::from_shape_vec(ndarray::IxDyn(&[labels.len(), NLI_CLASS_COUNT]), logits)
        .map_err(|error| invalid(format!("Local NLI logits shape: {error}")))?;
    control.check()?;
    Ok(LocalClassificationResult {
        labels: decode_logits(labels, &logits, config)?,
        model: model.info.clone(),
    })
}
