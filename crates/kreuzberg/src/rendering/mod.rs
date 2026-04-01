//! Unified rendering of document content to output formats.
//!
//! - [`render_markdown`] — GFM Markdown (via comrak)
//! - [`render_html`] — HTML5 (via comrak)
//! - [`render_djot`] — Djot markup
//! - [`render_plain`] — Plain text (no formatting)

pub(crate) mod common;
mod comrak_bridge;
mod djot;
mod html;
mod json;
mod markdown;
mod plain;

pub use comrak_bridge::build_comrak_ast;
pub use djot::render_djot;
pub use html::render_html;
pub use json::render_json;
pub use markdown::render_markdown;
pub use plain::render_plain;
