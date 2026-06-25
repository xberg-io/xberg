use std::path::Path;

use ort::session::Session;
use ort::value::Tensor;
use parking_lot::Mutex;

use crate::session::{
    TENSOR_ATTENTION_MASK, TENSOR_INPUT_IDS, TENSOR_LOGITS, TENSOR_SPAN_IDX, TENSOR_SPAN_MASK, TENSOR_TEXT_LENGTHS,
    TENSOR_WORD_MASK, build_session, validate_session_schema,
};
use crate::splitter::RegexSplitter;
use crate::tensor::SpanTensors;
use crate::tokenizer::HFTokenizer;
use crate::{
    EncodedInput, GlinerError, Parameters, Result, RuntimeConfig, SpanOutput, TextInput, decode::decode_logits,
    preprocess::PromptInput, preprocess::TokenizedInput, splitter::DEFAULT_SPLITTER_REGEX,
};

/// GLiNER span-mode inference engine.
pub struct Gliner {
    params: Parameters,
    splitter: RegexSplitter,
    tokenizer: HFTokenizer,
    session: Mutex<Session>,
}

impl Gliner {
    /// Load a GLiNER span-mode ONNX model and tokenizer from local files.
    pub fn new<PT, PM>(params: Parameters, tokenizer_path: PT, model_path: PM) -> Result<Self>
    where
        PT: AsRef<Path>,
        PM: AsRef<Path>,
    {
        Self::with_runtime(params, RuntimeConfig::default(), tokenizer_path, model_path)
    }

    /// Load a GLiNER span-mode ONNX model and tokenizer from local files with runtime options.
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
        let tokenizer = HFTokenizer::from_file(tokenizer_path)?;
        let session = build_session(model_path, &runtime)?;
        validate_session_schema(&session)?;
        Ok(Self {
            params,
            splitter: RegexSplitter::new(DEFAULT_SPLITTER_REGEX)?,
            tokenizer,
            session: Mutex::new(session),
        })
    }

    /// Run span-mode inference.
    pub fn inference(&self, input: TextInput) -> Result<SpanOutput> {
        let tokenized = TokenizedInput::from(input, &self.splitter, self.params.max_length)?;
        let prompt = PromptInput::from(tokenized);
        let encoded = EncodedInput::from(prompt, &self.tokenizer)?;
        let tensors = SpanTensors::from(encoded, self.params.max_width)?;
        let context = tensors.context;

        let input_ids = Tensor::from_array(tensors.input_ids)?;
        let attention_masks = Tensor::from_array(tensors.attention_masks)?;
        let word_masks = Tensor::from_array(tensors.word_masks)?;
        let text_lengths = Tensor::from_array(tensors.text_lengths)?;
        let span_idx = Tensor::from_array(tensors.span_idx)?;
        let span_mask = Tensor::from_array(tensors.span_mask)?;
        let logits = {
            let mut session = self.session.lock();
            let outputs = session.run(ort::inputs![
                TENSOR_INPUT_IDS => input_ids,
                TENSOR_ATTENTION_MASK => attention_masks,
                TENSOR_WORD_MASK => word_masks,
                TENSOR_TEXT_LENGTHS => text_lengths,
                TENSOR_SPAN_IDX => span_idx,
                TENSOR_SPAN_MASK => span_mask,
            ])?;
            outputs
                .get(TENSOR_LOGITS)
                .ok_or(GlinerError::MissingOutput(TENSOR_LOGITS))?
                .try_extract_array::<f32>()?
                .to_owned()
        };

        decode_logits(
            logits.view(),
            context,
            self.params.threshold,
            self.params.max_width,
            self.params.flat_ner,
            self.params.dup_label,
            self.params.multi_label,
        )
    }
}
