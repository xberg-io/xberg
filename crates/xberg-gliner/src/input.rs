use crate::config::{MAX_BATCH_SIZE, MAX_ENTITY_LABEL_CHARS, MAX_ENTITY_LABELS};
use crate::{GlinerError, Result};

const ENTITY_TOKEN: &str = "<<ENT>>";
const SEP_TOKEN: &str = "<<SEP>>";

/// Raw text input and zero-shot entity labels.
#[derive(Debug, Clone)]
pub struct TextInput {
    pub(crate) texts: Vec<String>,
    pub(crate) entities: Vec<String>,
}

impl TextInput {
    /// Construct a text batch.
    pub fn new(texts: Vec<String>, entities: Vec<String>) -> Result<Self> {
        if texts.is_empty() {
            return Err(GlinerError::InvalidInput("texts must not be empty".to_string()));
        }
        if entities.is_empty() {
            return Err(GlinerError::InvalidInput("entity labels must not be empty".to_string()));
        }
        if texts.len() > MAX_BATCH_SIZE {
            return Err(GlinerError::InvalidInput(format!(
                "batch size must be at most {MAX_BATCH_SIZE}, got {}",
                texts.len()
            )));
        }
        if entities.len() > MAX_ENTITY_LABELS {
            return Err(GlinerError::InvalidInput(format!(
                "entity label count must be at most {MAX_ENTITY_LABELS}, got {}",
                entities.len()
            )));
        }
        if let Some(index) = texts.iter().position(|text| text.trim().is_empty()) {
            return Err(GlinerError::InvalidInput(format!("texts[{index}] must not be empty")));
        }
        if let Some(index) = entities.iter().position(|label| label.trim().is_empty()) {
            return Err(GlinerError::InvalidInput(format!(
                "entity labels[{index}] must not be empty"
            )));
        }
        if let Some((index, label)) = entities
            .iter()
            .enumerate()
            .find(|(_, label)| label.chars().count() > MAX_ENTITY_LABEL_CHARS)
        {
            return Err(GlinerError::InvalidInput(format!(
                "entity labels[{index}] must be at most {MAX_ENTITY_LABEL_CHARS} characters, got {}",
                label.chars().count()
            )));
        }
        if let Some(index) = entities
            .iter()
            .position(|label| label.contains(ENTITY_TOKEN) || label.contains(SEP_TOKEN))
        {
            return Err(GlinerError::InvalidInput(format!(
                "entity labels[{index}] must not contain reserved GLiNER prompt markers"
            )));
        }
        Ok(Self { texts, entities })
    }

    /// Construct a text batch from borrowed string slices.
    pub fn from_str(texts: &[&str], entities: &[&str]) -> Result<Self> {
        Self::new(
            texts.iter().map(|text| (*text).to_string()).collect(),
            entities.iter().map(|entity| (*entity).to_string()).collect(),
        )
    }
}

/// A word token with byte offsets in the source text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    start: usize,
    end: usize,
    text: String,
}

impl Token {
    pub fn new(start: usize, end: usize, text: &str) -> Self {
        Self {
            start,
            end,
            text: text.to_string(),
        }
    }

    /// Start byte offset.
    pub fn start(&self) -> usize {
        self.start
    }

    /// End byte offset.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Token text.
    pub fn text(&self) -> &str {
        &self.text
    }
}
