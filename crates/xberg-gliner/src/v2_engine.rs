use std::path::Path;

use ndarray::{Array1, Array2};
use ort::session::Session;
use ort::value::Tensor;
use parking_lot::Mutex;

use crate::session::build_session;
use crate::v2_decode::decode_span_scores;
use crate::v2_preprocess::encode_v2;
use crate::v2_session::{
    TENSOR_V2_ATTENTION_MASK, TENSOR_V2_INPUT_IDS, TENSOR_V2_SCHEMA_POSITIONS, TENSOR_V2_SPAN_IDX,
    TENSOR_V2_SPAN_SCORES, TENSOR_V2_TEXT_POSITIONS, validate_session_schema_v2,
};
use crate::v2_splitter::V2Splitter;
use crate::v2_tensor::build_span_idx;
use crate::v2_tokenizer::V2Tokenizer;
use crate::{GlinerError, Parameters, Result, RuntimeConfig, SpanOutput, TextInput};

/// GLiNER2 schema-prompt inference engine.
///
/// Unlike [`crate::Gliner`] (span-mode, batched), GLiNER2's published ONNX
/// exports hardcode a batch dimension of 1 — `inference` accepts exactly one
/// text per call.
pub struct Gliner2 {
    params: Parameters,
    splitter: V2Splitter,
    tokenizer: V2Tokenizer,
    session: Mutex<Session>,
}

impl Gliner2 {
    /// Load a GLiNER2 schema-prompt ONNX model and tokenizer from local files.
    pub fn new<PT, PM>(params: Parameters, tokenizer_path: PT, model_path: PM) -> Result<Self>
    where
        PT: AsRef<Path>,
        PM: AsRef<Path>,
    {
        Self::with_runtime(params, RuntimeConfig::default(), tokenizer_path, model_path)
    }

    /// Load a GLiNER2 schema-prompt ONNX model and tokenizer from local files with runtime options.
    pub fn with_runtime<PT, PM>(
        params: Parameters,
        runtime: RuntimeConfig,
        tokenizer_path: PT,
        model_path: PM,
    ) -> Result<Self>
    where
        PT: AsRef<Path>,
        PM: AsRef<Path>,
    {
        params.validate()?;
        let tokenizer = V2Tokenizer::from_file(tokenizer_path)?;
        let session = build_session(model_path, &runtime)?;
        validate_session_schema_v2(&session)?;
        Ok(Self {
            params,
            splitter: V2Splitter::new()?,
            tokenizer,
            session: Mutex::new(session),
        })
    }

    /// Run schema-prompt inference. `input` must contain exactly one text.
    pub fn inference(&self, input: TextInput) -> Result<SpanOutput> {
        if input.texts.len() != 1 {
            return Err(GlinerError::InvalidInput(format!(
                "Gliner2::inference accepts exactly one text per call, got {}",
                input.texts.len()
            )));
        }
        let text = input.texts[0].clone();
        let labels = input.entities.clone();

        let encoded = encode_v2(&text, &labels, &self.tokenizer, &self.splitter)?;
        let seq_len = encoded.input_ids.len();
        let num_words = encoded.words.len();

        let input_ids = Array2::from_shape_vec((1, seq_len), encoded.input_ids)
            .map_err(|error| GlinerError::InvalidInput(format!("failed to build GLiNER2 input_ids tensor: {error}")))?;
        let attention_mask = Array2::from_shape_vec((1, seq_len), vec![1i64; seq_len]).map_err(|error| {
            GlinerError::InvalidInput(format!("failed to build GLiNER2 attention_mask tensor: {error}"))
        })?;
        let text_positions = Array1::from_vec(encoded.text_positions);
        let schema_positions = Array1::from_vec(encoded.schema_positions);
        let span_idx = build_span_idx(num_words, self.params.max_width)?;

        let input_ids = Tensor::from_array(input_ids)?;
        let attention_mask = Tensor::from_array(attention_mask)?;
        let text_positions = Tensor::from_array(text_positions)?;
        let schema_positions = Tensor::from_array(schema_positions)?;
        let span_idx = Tensor::from_array(span_idx)?;

        let span_scores = {
            let mut session = self.session.lock();
            let outputs = session.run(ort::inputs![
                TENSOR_V2_INPUT_IDS => input_ids,
                TENSOR_V2_ATTENTION_MASK => attention_mask,
                TENSOR_V2_TEXT_POSITIONS => text_positions,
                TENSOR_V2_SCHEMA_POSITIONS => schema_positions,
                TENSOR_V2_SPAN_IDX => span_idx,
            ])?;
            outputs
                .get(TENSOR_V2_SPAN_SCORES)
                .ok_or(GlinerError::MissingOutput(TENSOR_V2_SPAN_SCORES))?
                .try_extract_array::<f32>()?
                .to_owned()
        };

        decode_span_scores(
            span_scores.view(),
            &text,
            &encoded.words,
            &labels,
            self.params.threshold,
            self.params.max_width,
            self.params.flat_ner,
            self.params.dup_label,
            self.params.multi_label,
        )
    }
}
