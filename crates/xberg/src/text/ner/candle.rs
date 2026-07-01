//! NER backend backed by `xberg-gliner-candle` (GLiNER2 safetensors + optional LoRA).

use std::path::Path;
use std::sync::Mutex;

use async_trait::async_trait;
use xberg_gliner_candle::Gliner2Candle;

use crate::Result;
use crate::text::ner::NerBackend;
use crate::types::entity::{Entity, EntityCategory};

const DEFAULT_THRESHOLD: f32 = 0.5;

/// Wraps [`Gliner2Candle`] behind the [`NerBackend`] trait.
///
/// `Gliner2Candle` holds candle tensors which are not `Send`, so we wrap it
/// in a `Mutex` to satisfy the `Send + Sync` requirement of [`NerBackend`].
pub struct CandleBackend {
    model: Mutex<Gliner2Candle>,
}

impl CandleBackend {
    /// Load from a local model directory. Applies `lora_adapter_dir` if provided.
    ///
    /// `model_dir` must contain `tokenizer.json` and `model.safetensors`.
    /// `lora_adapter_dir`, when set, must contain `adapter_config.json` and
    /// `adapter_model.safetensors` — merged into the base weights at load time.
    pub fn from_local(model_dir: &Path, lora_adapter_dir: Option<&Path>) -> crate::Result<Self> {
        let mut model = Gliner2Candle::from_local(model_dir)
            .map_err(|e| crate::XbergError::Other(format!("CandleBackend load: {e}")))?;
        if let Some(adapter_dir) = lora_adapter_dir {
            let adapter_name = adapter_dir.file_name().and_then(|n| n.to_str()).unwrap_or("adapter");
            model
                .load_adapter(adapter_name, adapter_dir)
                .map_err(|e| crate::XbergError::Other(format!("CandleBackend load_adapter: {e}")))?;
        }
        Ok(Self {
            model: Mutex::new(model),
        })
    }
}

fn spans_to_entities(spans: Vec<xberg_gliner_candle::Span>) -> Vec<Entity> {
    spans
        .into_iter()
        .map(|span| {
            let (start, end) = span.offsets();
            Entity {
                category: EntityCategory::from(span.class().to_string()),
                text: span.text().to_string(),
                start: start as u32,
                end: end as u32,
                confidence: Some(span.probability()),
            }
        })
        .collect()
}

#[async_trait]
impl NerBackend for CandleBackend {
    async fn detect(&self, text: &str, categories: &[EntityCategory]) -> Result<Vec<Entity>> {
        let labels: Vec<&str> = if categories.is_empty() {
            vec!["person", "organization", "location", "email", "phone"]
        } else {
            categories.iter().map(category_to_label).collect()
        };

        let model = self
            .model
            .lock()
            .map_err(|_| crate::XbergError::Other("CandleBackend: model mutex poisoned".into()))?;

        // extract_ner is CPU-bound (tensor inference). block_in_place signals tokio to
        // move other tasks off this thread for the duration without requiring Send.
        let spans = tokio::task::block_in_place(|| model.extract_ner(text, &labels, DEFAULT_THRESHOLD))
            .map_err(|e| crate::XbergError::Other(format!("CandleBackend inference: {e}")))?;

        Ok(spans_to_entities(spans))
    }
}

fn category_to_label(cat: &EntityCategory) -> &str {
    match cat {
        EntityCategory::Person => "person",
        EntityCategory::Organization => "organization",
        EntityCategory::Location => "location",
        EntityCategory::Email => "email",
        EntityCategory::Phone => "phone",
        EntityCategory::Date => "date",
        EntityCategory::Time => "time",
        EntityCategory::Money => "money",
        EntityCategory::Percent => "percent",
        EntityCategory::Url => "url",
        EntityCategory::Custom(s) => s.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_to_label_maps_known_categories() {
        assert_eq!(category_to_label(&EntityCategory::Person), "person");
        assert_eq!(category_to_label(&EntityCategory::Organization), "organization");
        assert_eq!(category_to_label(&EntityCategory::Location), "location");
        assert_eq!(category_to_label(&EntityCategory::Email), "email");
        assert_eq!(category_to_label(&EntityCategory::Phone), "phone");
        assert_eq!(category_to_label(&EntityCategory::Date), "date");
        assert_eq!(category_to_label(&EntityCategory::Time), "time");
        assert_eq!(category_to_label(&EntityCategory::Money), "money");
        assert_eq!(category_to_label(&EntityCategory::Percent), "percent");
        assert_eq!(category_to_label(&EntityCategory::Url), "url");
        assert_eq!(
            category_to_label(&EntityCategory::Custom("product".to_string())),
            "product"
        );
    }

    #[test]
    fn spans_to_entities_is_empty_for_no_spans() {
        let entities = spans_to_entities(vec![]);
        assert!(entities.is_empty());
    }

    #[test]
    fn spans_to_entities_converts_fields_correctly() {
        let span = xberg_gliner_candle::Span::new(0, 0, 5, "Alice".to_string(), "person".to_string(), 0.92)
            .expect("valid span");
        let entities = spans_to_entities(vec![span]);

        assert_eq!(entities.len(), 1);
        let e = &entities[0];
        assert_eq!(e.text, "Alice");
        assert_eq!(e.category, EntityCategory::Person);
        assert_eq!(e.start, 0);
        assert_eq!(e.end, 5);
        assert!((e.confidence.unwrap() - 0.92).abs() < 1e-5);
    }
}
