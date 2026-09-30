//! Unified rendering of document content to output formats.
//!
//! - `render_markdown` — GFM Markdown (via comrak)
//! - `render_html` — HTML5 (via comrak)
//! - `render_djot` — Djot markup
//! - `render_doctags` — Docling DocTags (tables as OTSL)
//! - `render_dot` — Graphviz DOT (diagrams recovered from vector sources)
//! - `render_docx` — Office Open XML package, built from rendered Markdown
//! - `render_pdf` — PDF document, laid out from rendered Markdown
//! - `render_plain` — Plain text (no formatting)

pub(crate) mod common;
mod comrak_bridge;
mod djot;
mod doctags;
#[cfg(feature = "office")]
mod docx;
mod dot;
mod html;
#[cfg(feature = "html")]
pub mod html_styled;
mod json;
mod markdown;
#[cfg(feature = "pdf")]
mod pdf;
mod plain;

pub(crate) use djot::render_djot;
pub(crate) use doctags::render_doctags;
#[cfg(feature = "office")]
pub(crate) use docx::render_docx;
pub(crate) use dot::render_dot;
pub(crate) use html::render_html;
#[cfg(feature = "html")]
pub use html_styled::StyledHtmlRenderer;
pub use json::render_json;
pub(crate) use markdown::render_markdown;
#[cfg(feature = "pdf")]
pub(crate) use pdf::render_pdf;
pub(crate) use plain::render_plain;
