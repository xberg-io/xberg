use crate::{GlinerError, Result};

/// Maximum batch size accepted by the span-mode runtime.
pub const MAX_BATCH_SIZE: usize = 32;
/// Maximum number of entity labels accepted per inference call.
pub const MAX_ENTITY_LABELS: usize = 256;
/// Maximum label length in Unicode scalar values.
pub const MAX_ENTITY_LABEL_CHARS: usize = 128;
/// Maximum number of split words per sequence.
pub const MAX_WORDS_PER_SEQUENCE: usize = 4096;
/// Maximum span width in words.
pub const MAX_SPAN_WIDTH: usize = 128;
/// Maximum span candidates per sequence.
pub const MAX_SPANS_PER_SEQUENCE: usize = 262_144;

/// Processing parameters for GLiNER span-mode inference.
#[derive(Debug, Clone)]
pub struct Parameters {
    /// Probability threshold. Defaults to `0.5`.
    pub threshold: f32,
    /// No entity may overlap another entity when enabled. Defaults to `true`.
    pub flat_ner: bool,
    /// Overlapping spans may share the same label when enabled and `flat_ner` is disabled.
    pub dup_label: bool,
    /// Overlapping spans may use different labels when enabled and `flat_ner` is disabled.
    pub multi_label: bool,
    /// Maximum span width in words. Defaults to `12`.
    pub max_width: usize,
    /// Maximum number of words per input sequence. Defaults to `512`.
    pub max_length: Option<usize>,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            flat_ner: true,
            dup_label: false,
            multi_label: false,
            max_width: 12,
            max_length: Some(512),
        }
    }
}

impl Parameters {
    pub(crate) fn validate(&self) -> Result<()> {
        if !(0.0..=1.0).contains(&self.threshold) || !self.threshold.is_finite() {
            return Err(GlinerError::InvalidInput(format!(
                "threshold must be finite and between 0.0 and 1.0, got {}",
                self.threshold
            )));
        }
        if self.max_width == 0 {
            return Err(GlinerError::InvalidInput("max_width must be at least 1".to_string()));
        }
        if self.max_width > MAX_SPAN_WIDTH {
            return Err(GlinerError::InvalidInput(format!(
                "max_width must be at most {MAX_SPAN_WIDTH}, got {}",
                self.max_width
            )));
        }
        if self.max_length == Some(0) {
            return Err(GlinerError::InvalidInput(
                "max_length must be at least 1 when set".to_string(),
            ));
        }
        if self
            .max_length
            .is_some_and(|max_length| max_length > MAX_WORDS_PER_SEQUENCE)
        {
            return Err(GlinerError::InvalidInput(format!(
                "max_length must be at most {MAX_WORDS_PER_SEQUENCE} when set"
            )));
        }
        Ok(())
    }

    /// Set the probability threshold.
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set the maximum span width.
    pub fn with_max_width(mut self, max_width: usize) -> Self {
        self.max_width = max_width;
        self
    }

    /// Set the maximum input length.
    pub fn with_max_length(mut self, max_length: Option<usize>) -> Self {
        self.max_length = max_length;
        self
    }

    /// Configure flat NER overlap filtering.
    pub fn with_flat_ner(mut self, flat_ner: bool) -> Self {
        self.flat_ner = flat_ner;
        self
    }

    /// Configure duplicate-label overlap filtering.
    pub fn with_dup_label(mut self, dup_label: bool) -> Self {
        self.dup_label = dup_label;
        self
    }

    /// Configure multi-label overlap filtering.
    pub fn with_multi_label(mut self, multi_label: bool) -> Self {
        self.multi_label = multi_label;
        self
    }
}

/// ONNX Runtime session configuration.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Intra-op thread count passed to ONNX Runtime.
    pub intra_threads: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self { intra_threads: 4 }
    }
}

impl RuntimeConfig {
    /// Set the ONNX Runtime intra-op thread count.
    pub fn with_intra_threads(mut self, intra_threads: usize) -> Self {
        self.intra_threads = intra_threads.max(1);
        self
    }
}
