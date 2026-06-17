//! Token-aware batch packing of rendered pages into vision calls.
//!
//! Greedily packs pages into batches up to a configurable token budget. The first batch
//! includes the user text; subsequent batches omit it to avoid duplication.

use super::PageImage;

/// Characters-per-token estimate used to convert text lengths to token counts.
const CHARS_PER_TOKEN: usize = 4;

/// Configuration for batch packing.
#[derive(Debug, Clone)]
pub struct ChunkerConfig {
    /// Maximum input tokens allowed per batch.
    pub max_input_tokens: u32,
    /// Estimated tokens consumed by a single image.
    pub avg_tokens_per_image: u32,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            max_input_tokens: 800_000,
            avg_tokens_per_image: 1_500,
        }
    }
}

/// A batch of pages ready for a single vision-LLM call.
#[derive(Debug, Clone)]
pub struct Batch {
    /// Page images in this batch.
    pub pages: Vec<PageImage>,
    /// User text (context + excerpt). Only set for the first batch;
    /// subsequent batches have `None` to avoid duplication.
    pub user_text: Option<String>,
}

/// Greedily pack `pages` into batches up to the token budget in `config`.
///
/// The `user_text` rides only the first batch; subsequent batches set
/// `user_text` to `None`. Text tokens are counted only when building the
/// first batch.
///
/// If a single page exceeds the limit on its own it is still emitted as a
/// one-page batch (never dropped).
pub fn batch_pages(
    pages: Vec<PageImage>,
    user_text: Option<String>,
    config: &ChunkerConfig,
) -> Vec<Batch> {
    if pages.is_empty() {
        return vec![Batch {
            pages: vec![],
            user_text,
        }];
    }

    let user_text_tokens = user_text
        .as_ref()
        .map(|t| (t.len() / CHARS_PER_TOKEN).max(1) as u32)
        .unwrap_or(0);

    let mut batches: Vec<Batch> = Vec::new();
    let mut current_pages: Vec<PageImage> = Vec::new();
    let mut current_tokens: u32 = user_text_tokens;
    let mut is_first_batch = true;

    for page in pages {
        let page_tokens =
            (page.png_bytes.len() / CHARS_PER_TOKEN).max(1) as u32 + config.avg_tokens_per_image;
        let new_total = current_tokens + page_tokens;

        if !current_pages.is_empty() && new_total > config.max_input_tokens {
            batches.push(Batch {
                pages: current_pages,
                user_text: if is_first_batch {
                    user_text.clone()
                } else {
                    None
                },
            });
            current_pages = Vec::new();
            current_tokens = 0;
            is_first_batch = false;
        }

        if current_pages.is_empty() && page_tokens > config.max_input_tokens {
            tracing::warn!(
                page_bytes = page.png_bytes.len(),
                page_number = page.page_number,
                max_tokens = config.max_input_tokens,
                "page exceeds max token budget; emitting as single-page batch anyway"
            );
            batches.push(Batch {
                pages: vec![page],
                user_text: if is_first_batch {
                    user_text.clone()
                } else {
                    None
                },
            });
            is_first_batch = false;
        } else {
            current_pages.push(page);
            current_tokens = current_tokens.saturating_add(page_tokens);
        }
    }

    if !current_pages.is_empty() {
        batches.push(Batch {
            pages: current_pages,
            user_text: if is_first_batch { user_text } else { None },
        });
    }

    batches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_page(number: u32, size: usize) -> PageImage {
        PageImage {
            page_number: number,
            png_bytes: vec![0u8; size],
        }
    }

    #[test]
    fn empty_pages_returns_single_empty_batch() {
        let config = ChunkerConfig {
            max_input_tokens: 100,
            avg_tokens_per_image: 1_500,
        };
        let batches = batch_pages(vec![], None, &config);
        assert_eq!(batches.len(), 1);
        assert!(batches[0].pages.is_empty());
        assert!(batches[0].user_text.is_none());
    }

    #[test]
    fn single_page_under_limit_returns_one_batch_with_user_text() {
        let config = ChunkerConfig {
            max_input_tokens: 100_000,
            avg_tokens_per_image: 1_500,
        };
        let pages = vec![stub_page(1, 5_000)];
        let batches = batch_pages(pages, Some("text".to_string()), &config);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].pages.len(), 1);
        assert!(batches[0].user_text.is_some());
    }

    #[test]
    fn multiple_pages_split_into_batches_user_text_only_on_first() {
        let config = ChunkerConfig {
            max_input_tokens: 3_000,
            avg_tokens_per_image: 1_500,
        };
        let pages = vec![stub_page(1, 5_000), stub_page(2, 5_000), stub_page(3, 5_000)];
        let batches = batch_pages(pages, Some("text".to_string()), &config);
        assert!(batches.len() > 1);
        assert!(batches[0].user_text.is_some());
        assert!(batches[1].user_text.is_none());
    }

    #[test]
    fn oversized_single_page_emitted_as_own_batch() {
        let config = ChunkerConfig {
            max_input_tokens: 1_000,
            avg_tokens_per_image: 1_500,
        };
        let pages = vec![stub_page(1, 50_000)];
        let batches = batch_pages(pages, None, &config);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].pages.len(), 1);
    }

    #[test]
    fn two_oversized_pages_emit_as_separate_batches() {
        let config = ChunkerConfig {
            max_input_tokens: 1_000,
            avg_tokens_per_image: 1_500,
        };
        let pages = vec![stub_page(1, 50_000), stub_page(2, 50_000)];
        let batches = batch_pages(pages, None, &config);
        assert!(batches.len() >= 2);
        assert_eq!(batches[0].pages.len(), 1);
        assert_eq!(batches[1].pages.len(), 1);
    }

    #[test]
    fn user_text_only_on_first_batch_across_three_batches() {
        let config = ChunkerConfig {
            max_input_tokens: 2_000,
            avg_tokens_per_image: 1_500,
        };
        let pages = vec![stub_page(1, 4_000), stub_page(2, 4_000), stub_page(3, 4_000)];
        let batches = batch_pages(pages, Some("user context".to_string()), &config);
        assert!(!batches.is_empty());
        assert!(batches[0].user_text.is_some());
        for batch in batches.iter().skip(1) {
            assert!(batch.user_text.is_none());
        }
    }

    #[test]
    fn none_user_text_stays_none_in_all_batches() {
        let config = ChunkerConfig {
            max_input_tokens: 100_000,
            avg_tokens_per_image: 1_500,
        };
        let pages = vec![stub_page(1, 5_000)];
        let batches = batch_pages(pages, None, &config);
        assert_eq!(batches.len(), 1);
        assert!(batches[0].user_text.is_none());
    }

    #[test]
    fn multiple_pages_all_under_limit_single_batch() {
        let config = ChunkerConfig {
            max_input_tokens: 500_000,
            avg_tokens_per_image: 1_500,
        };
        let pages = vec![stub_page(1, 5_000), stub_page(2, 5_000), stub_page(3, 5_000)];
        let batches = batch_pages(pages, None, &config);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].pages.len(), 3);
    }

    #[test]
    fn default_config_has_correct_values() {
        let config = ChunkerConfig::default();
        assert_eq!(config.max_input_tokens, 800_000);
        assert_eq!(config.avg_tokens_per_image, 1_500);
    }
}
