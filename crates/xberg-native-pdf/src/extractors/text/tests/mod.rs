//! Unit tests for [`super`].
//!
//! Split out of `extractors/text.rs` for file size: the parent was 16,288 lines
//! (673 KiB), over the repository's 500 KiB file-safety limit. A child module sees
//! the parent's private items exactly as the inline module did. ~keep

mod advance_and_monospace;
mod artifact_and_dedup;
mod color_fallbacks_and_state;
mod color_spaces;
mod columns_and_ordering;
mod config_and_merging;
mod extraction_and_rotation;
mod ligature_decode;
