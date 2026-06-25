//! Backend trait shared by every NER implementation.

use crate::Result;
use crate::types::entity::{Entity, EntityCategory};
use async_trait::async_trait;

/// One-method trait that every NER backend implements.
///
/// The redaction engine and the NER post-processor both consume backends through
/// this trait so they can be swapped without rewriting consumer code.
#[async_trait]
#[cfg_attr(alef, alef(skip))]
pub trait NerBackend: Send + Sync {
    /// Identify entities in `text` belonging to any of the `categories`.
    ///
    /// Implementations must return entities in source byte-offset order. Byte offsets
    /// are 0-indexed and refer to UTF-8 byte positions in `text`. When `categories`
    /// is empty the backend returns every entity it can identify.
    async fn detect(&self, text: &str, categories: &[EntityCategory]) -> Result<Vec<Entity>>;

    /// Identify entities in `text`, including user-supplied custom labels for
    /// zero-shot detection.
    ///
    /// Backends should treat each label in `custom_labels` as if the caller had
    /// passed `EntityCategory::Custom(label)` in `categories`. The default
    /// implementation forwards to [`detect`](Self::detect) after appending each
    /// custom label as a `Custom` category — backends that can do something
    /// smarter (e.g. `xberg-gliner` native multi-label zero-shot inference) should
    /// override this method.
    async fn detect_with_custom(
        &self,
        text: &str,
        categories: &[EntityCategory],
        custom_labels: &[String],
    ) -> Result<Vec<Entity>> {
        if custom_labels.is_empty() {
            return self.detect(text, categories).await;
        }
        let mut all: Vec<EntityCategory> = categories.to_vec();
        for label in custom_labels {
            all.push(EntityCategory::Custom(label.clone()));
        }
        self.detect(text, &all).await
    }
}
