//! Token-aware batching of pages for vision-LLM calls.
//!
//! Greedily packs pages into batches up to a max token budget. The first batch
//! includes the user text; subsequent batches omit it to avoid duplication.
//!
//! All tuning lives in [`ChunkerConfig`], which the caller supplies. The
//! mechanism reads no environment variables and decides no parallelism policy;
//! those concerns belong to the caller. [`ChunkerConfig::default`] exposes
//! conventional constants (4 chars/token, 1500 tokens/image, 800k max input
//! tokens) as plain overridable defaults.

use super::rasterize::PageImage;

/// Default characters-per-token divisor used to estimate text token counts.
const CHARS_PER_TOKEN: usize = 4;
/// Default per-image token estimate.
const IMAGE_TOKEN_ESTIMATE: u32 = 1500;
/// Default maximum input tokens per batch.
const DEFAULT_MAX_INPUT_TOKENS: u32 = 800_000;

/// Configuration for batch packing.
///
/// These are plain parameters with no environment or policy coupling.
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
pub struct ChunkerConfig {
    /// Maximum input tokens per batch.
    pub max_input_tokens: u32,
    /// Estimated tokens contributed by each image.
    pub avg_tokens_per_image: u32,
    /// Characters-per-token divisor used to estimate text/image byte token counts.
    pub chars_per_token: usize,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            max_input_tokens: DEFAULT_MAX_INPUT_TOKENS,
            avg_tokens_per_image: IMAGE_TOKEN_ESTIMATE,
            chars_per_token: CHARS_PER_TOKEN,
        }
    }
}

/// A batch of pages ready for a single vision-LLM call.
#[derive(Debug, Clone)]
pub struct Batch {
    /// Page images in this batch.
    pub pages: Vec<PageImage>,
    /// User text (context + excerpt). Only set for the first batch;
    /// subsequent batches have None to avoid duplication.
    pub user_text: Option<String>,
}

/// Greedily pack pages into batches up to a max token budget.
///
/// The first batch includes `user_text`; subsequent batches omit it.
/// Text tokens are counted only for the first batch.
///
/// If a single page exceeds the limit, it is emitted as its own batch
/// with a warning.
#[cfg_attr(alef, alef(skip))]
pub fn batch_pages(pages: Vec<PageImage>, user_text: Option<String>, config: &ChunkerConfig) -> Vec<Batch> {
    if pages.is_empty() {
        return vec![Batch {
            pages: vec![],
            user_text,
        }];
    }

    // Guard the divisor: `chars_per_token` is a caller-settable field, and a 0
    // would panic the integer divisions below. Clamp to at least 1.
    let chars_per_token = config.chars_per_token.max(1);

    let user_text_tokens = user_text
        .as_ref()
        .map(|t| (t.len() / chars_per_token).max(1) as u32)
        .unwrap_or(0);

    let mut batches = Vec::new();
    let mut current_batch = Vec::new();
    let mut current_text_tokens = user_text_tokens;
    let mut is_first_batch = true;

    for page in pages {
        let page_tokens = (page.png_bytes.len() / chars_per_token).max(1) as u32 + config.avg_tokens_per_image;
        let new_total = current_text_tokens + page_tokens;

        if !current_batch.is_empty() && new_total > config.max_input_tokens {
            let batch = Batch {
                pages: current_batch,
                user_text: if is_first_batch { user_text.clone() } else { None },
            };
            batches.push(batch);
            current_batch = Vec::new();
            current_text_tokens = 0;
            is_first_batch = false;
        }

        if current_batch.is_empty() && page_tokens > config.max_input_tokens {
            tracing::warn!(
                page_bytes = page.png_bytes.len(),
                page_number = page.page_number,
                max_tokens = config.max_input_tokens,
                "Page exceeds max token budget; emitting as single-page batch anyway"
            );

            let batch = Batch {
                pages: vec![page],
                user_text: if is_first_batch { user_text.clone() } else { None },
            };
            batches.push(batch);
            is_first_batch = false;
        } else {
            current_batch.push(page);
            // Update the running total incrementally rather than assigning
            // `new_total`. `new_total` was computed before the flush branch
            // above may have reset `current_text_tokens` to 0; reusing it here
            // would carry the just-flushed batch's tokens into the new batch and
            // over-split subsequent pages. Adding `page_tokens` yields
            // `page_tokens` after a flush and the correct cumulative total
            // otherwise.
            current_text_tokens += page_tokens;
        }
    }

    if !current_batch.is_empty() {
        let batch = Batch {
            pages: current_batch,
            user_text: if is_first_batch { user_text } else { None },
        };
        batches.push(batch);
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
    fn empty_pages_returns_single_batch() {
        let config = ChunkerConfig {
            max_input_tokens: 100,
            avg_tokens_per_image: 1500,
            chars_per_token: 4,
        };
        let batches = batch_pages(vec![], None, &config);
        assert_eq!(batches.len(), 1);
        assert!(batches[0].pages.is_empty());
        assert!(batches[0].user_text.is_none());
    }

    #[test]
    fn single_page_under_limit_returns_one_batch() {
        let config = ChunkerConfig {
            max_input_tokens: 100_000,
            avg_tokens_per_image: 1500,
            chars_per_token: 4,
        };
        let pages = vec![stub_page(1, 5000)];
        let batches = batch_pages(pages, Some("text".to_string()), &config);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].pages.len(), 1);
        assert!(batches[0].user_text.is_some());
    }

    #[test]
    fn multiple_pages_split_into_batches() {
        let config = ChunkerConfig {
            max_input_tokens: 3000,
            avg_tokens_per_image: 1500,
            chars_per_token: 4,
        };
        let pages = vec![stub_page(1, 5000), stub_page(2, 5000), stub_page(3, 5000)];
        let batches = batch_pages(pages, Some("text".to_string()), &config);
        assert!(batches.len() > 1);
        assert!(batches[0].user_text.is_some());
        if batches.len() > 1 {
            assert!(batches[1].user_text.is_none());
        }
    }

    #[test]
    fn oversized_single_page_emitted_with_warning() {
        let config = ChunkerConfig {
            max_input_tokens: 1000,
            avg_tokens_per_image: 1500,
            chars_per_token: 4,
        };
        let pages = vec![stub_page(1, 50_000)];
        let batches = batch_pages(pages, None, &config);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].pages.len(), 1);
    }

    #[test]
    fn user_text_only_on_first_batch() {
        let config = ChunkerConfig {
            max_input_tokens: 2000,
            avg_tokens_per_image: 1500,
            chars_per_token: 4,
        };
        let pages = vec![stub_page(1, 4000), stub_page(2, 4000), stub_page(3, 4000)];
        let user_text = Some("user context".to_string());
        let batches = batch_pages(pages, user_text, &config);

        assert!(!batches.is_empty());
        assert!(batches[0].user_text.is_some());
        if batches.len() > 1 {
            assert!(batches[1].user_text.is_none());
        }
        if batches.len() > 2 {
            assert!(batches[2].user_text.is_none());
        }
    }

    #[test]
    fn none_user_text_stays_none() {
        let config = ChunkerConfig {
            max_input_tokens: 100_000,
            avg_tokens_per_image: 1500,
            chars_per_token: 4,
        };
        let pages = vec![stub_page(1, 5000)];
        let batches = batch_pages(pages, None, &config);
        assert_eq!(batches.len(), 1);
        assert!(batches[0].user_text.is_none());
    }

    #[test]
    fn multiple_pages_all_under_limit_single_batch() {
        let config = ChunkerConfig {
            max_input_tokens: 500_000,
            avg_tokens_per_image: 1500,
            chars_per_token: 4,
        };
        let pages = vec![stub_page(1, 5000), stub_page(2, 5000), stub_page(3, 5000)];
        let batches = batch_pages(pages, None, &config);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].pages.len(), 3);
    }

    #[test]
    fn two_oversized_pages_emit_as_separate_batches() {
        let config = ChunkerConfig {
            max_input_tokens: 1000,
            avg_tokens_per_image: 1500,
            chars_per_token: 4,
        };
        let pages = vec![stub_page(1, 50_000), stub_page(2, 50_000)];
        let batches = batch_pages(pages, None, &config);
        assert!(batches.len() >= 2);
        assert_eq!(batches[0].pages.len(), 1);
        assert_eq!(batches[1].pages.len(), 1);
    }

    #[test]
    fn zero_chars_per_token_does_not_panic() {
        // `chars_per_token` is a public, caller-settable field; a 0 must not
        // panic the internal `len / chars_per_token` divisions.
        let config = ChunkerConfig {
            max_input_tokens: 100_000,
            avg_tokens_per_image: 1500,
            chars_per_token: 0,
        };
        let pages = vec![stub_page(1, 5000), stub_page(2, 5000)];
        let batches = batch_pages(pages, Some("user context".to_string()), &config);
        // The divisor clamps to 1, so packing proceeds normally rather than panicking.
        assert!(!batches.is_empty());
        assert!(batches[0].user_text.is_some());
    }

    #[test]
    fn post_flush_total_does_not_carry_flushed_batch_tokens() {
        // Regression test for the running-total bug in `batch_pages`. After a
        // flush resets `current_text_tokens` to 0, the post-flush page must
        // start the new batch's running total from `page_tokens` alone. The
        // pre-fix code reused the pre-flush `new_total`, inflating the new
        // batch's total by the just-flushed batch's tokens and over-splitting
        // later pages into extra batches (extra vision-LLM calls).
        //
        // Config note: this uses `avg_tokens_per_image = 1500` and
        // `chars_per_token = 4` (the historical defaults) but `max_input_tokens
        // = 4000` rather than 3000. With a 3000 budget the bug is unobservable:
        // every page costs `1500 + (len/4).max(1) >= 1501` tokens, so any two
        // pages total >= 3002 > 3000 and never share a batch in EITHER version
        // — each batch holds exactly one page and the counts are identical. A
        // 4000 budget gives just enough slack for two pages to pack, which is
        // the only situation in which the over-split is visible.
        let config = ChunkerConfig {
            max_input_tokens: 4000,
            avg_tokens_per_image: 1500,
            chars_per_token: 4,
        };

        // user_text: 4000 bytes / 4 = 1000 text tokens.
        let user_text = Some("u".repeat(4000));
        // Each page: png 4 bytes -> (4/4).max(1) = 1, + 1500 image = 1501 tokens.
        let pages = vec![stub_page(1, 4), stub_page(2, 4), stub_page(3, 4)];

        // Token arithmetic (budget 4000):
        //   page1: 1000 + 1501 = 2501  <= 4000  -> stays in batch1 (with user_text)
        //   page2: 2501 + 1501 = 4002  >  4000  -> flush batch1 = [page1]; reset to 0
        //          fixed: running total becomes 1501 (page2 alone)
        //          buggy: running total becomes 4002 (carries flushed batch1 + page2)
        //   page3: fixed: 1501 + 1501 = 3002 <= 4000 -> packs with page2 => batch2 = [page2, page3]
        //          buggy: 4002 + 1501 = 5503 >  4000 -> flush batch2 = [page2]; batch3 = [page3]
        // => fixed code: 2 batches; pre-fix code: 3 batches.
        let batches = batch_pages(pages, user_text, &config);

        assert_eq!(batches.len(), 2, "page2 and page3 must share one post-flush batch");
        assert_eq!(batches[0].pages.len(), 1, "batch1 holds only page1");
        assert_eq!(batches[0].pages[0].page_number, 1);
        assert!(batches[0].user_text.is_some(), "user_text rides on the first batch");
        assert_eq!(
            batches[1].pages.len(),
            2,
            "page2 and page3 pack together after the flush"
        );
        assert_eq!(batches[1].pages[0].page_number, 2);
        assert_eq!(batches[1].pages[1].page_number, 3);
        assert!(batches[1].user_text.is_none(), "subsequent batches omit user_text");
    }
}
