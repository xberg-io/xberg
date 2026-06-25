use std::path::Path;

use crate::{GlinerError, Result};

pub(crate) trait Tokenizer {
    fn encode(&self, input: &str) -> Result<Vec<u32>>;
}

pub(crate) struct HFTokenizer {
    inner: tokenizers::Tokenizer,
}

impl HFTokenizer {
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let inner = tokenizers::Tokenizer::from_file(path)
            .map_err(|error| GlinerError::Tokenizer(format!("failed to load tokenizer from file: {error}")))?;
        Ok(Self { inner })
    }
}

impl Tokenizer for HFTokenizer {
    fn encode(&self, input: &str) -> Result<Vec<u32>> {
        let encoding = self
            .inner
            .encode(input, false)
            .map_err(|error| GlinerError::Tokenizer(format!("failed to encode '{input}': {error}")))?;
        Ok(encoding.get_ids().to_vec())
    }
}
