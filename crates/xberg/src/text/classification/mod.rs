//! Per-page LLM classification.
//!
//! Walks the rendered `content`, slices it on the page boundary metadata produced
//! during extraction, and asks the configured LLM to assign one or more labels
//! from a fixed vocabulary to each page. Results land on
//! [`ExtractedDocument::page_classifications`](crate::types::ExtractedDocument::page_classifications).
//!
//! Triggered by [`ExtractionConfig::page_classification`](crate::core::config::ExtractionConfig::page_classification);
//! invoked by the Middle-stage post-processor in
//! [`crate::plugins::processor::builtin::classification`].

pub mod chunk_classifier;
pub mod page_classifier;

pub use chunk_classifier::classify_chunks;
pub use page_classifier::{classify_pages, classify_text};

/// Classify a single document (as multiple pages or a single text block).
///
/// Aggregates classifications across all pages in the provided text, returning
/// a combined label set that represents the document as a whole.
///
/// # Arguments
///
/// * `pages` - Slice of page texts to classify. Each page is classified independently
///   using the configured LLM, and results are aggregated.
/// * `config` - Classification configuration including labels and LLM settings.
///
/// # Returns
///
/// A vector of `ClassificationLabel` entries representing the document's overall classification.
///
/// # Errors
///
/// Returns an error if `config.labels` is empty or if LLM calls fail.
///
/// # Example
///
/// ```rust,no_run
/// use xberg::text::classification::classify_document;
/// use xberg::core::config::PageClassificationConfig;
/// use xberg::core::config::LlmConfig;
///
/// # async fn example() -> xberg::Result<()> {
/// let config = PageClassificationConfig {
///     labels: vec!["invoice".to_string(), "memo".to_string()],
///     llm: LlmConfig::default(),
///     prompt_template: None,
///     multi_label: false,
/// };
///
/// let pages = vec!["Page 1 content", "Page 2 content"];
/// let labels = classify_document(&pages, &config).await?;
/// # Ok(())
/// # }
/// ```
pub async fn classify_document(
    pages: &[&str],
    config: &crate::core::config::PageClassificationConfig,
) -> crate::Result<Vec<crate::ClassificationLabel>> {
    if config.labels.is_empty() {
        return Err(crate::XbergError::validation(
            "PageClassificationConfig.labels must contain at least one entry",
        ));
    }

    if pages.is_empty() {
        return Ok(Vec::new());
    }

    let ctx = page_classifier::ClassifyContext::new(config);
    let mut all_labels: Vec<crate::ClassificationLabel> = Vec::new();
    let mut label_counts: std::collections::HashMap<String, (f32, u32)> = std::collections::HashMap::new();

    for page_text in pages {
        if page_text.is_empty() {
            continue;
        }
        let (labels, _usage) = page_classifier::classify_one(page_text, &ctx, config).await?;
        for label in labels {
            all_labels.push(label.clone());
            let count = label_counts.entry(label.label).or_insert((0.0, 0));
            if let Some(conf) = label.confidence {
                count.0 += conf;
            }
            count.1 += 1;
        }
    }

    if config.multi_label {
        all_labels.sort_by(|a, b| a.label.cmp(&b.label));
        all_labels.dedup_by(|a, b| a.label == b.label);
        Ok(all_labels)
    } else {
        if all_labels.is_empty() {
            return Ok(Vec::new());
        }
        let best = all_labels.into_iter().max_by(|a, b| {
            let a_score = a.confidence.unwrap_or(0.0);
            let b_score = b.confidence.unwrap_or(0.0);
            a_score.partial_cmp(&b_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(best.into_iter().collect())
    }
}
