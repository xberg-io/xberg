//! Unit tests for [`super::super`].
//!
//! Split out of `document.rs` purely for file size: the parent was 28,713 lines
//! (1.2 MiB) and tripped the repository's 500 KiB file-safety limit. A child
//! module sees the parent's private items exactly as an inline `mod tests` did.
//! Further split into per-topic submodules here for the same reason: the flat
//! `tests.rs` grew to 8,362 lines and tripped `file-too-long`/`function-too-long`.
//! Shared fixtures and helpers used by more than one topic live in `common`. ~keep

mod common;
mod pdf_fixtures;

mod annotations;
mod catalog;
mod columns;
mod core;
mod extract_api;
mod fonts;
mod images;
mod key_value_rows;
mod objects;
mod open;
mod pages;
mod paths;
mod reading_order;
mod redaction;
mod span_postprocess;
mod spans_text;
mod spans_text_rtl;
mod tables;
mod text_assembly;
