//! PDF-to-Markdown renderer using segment-level font analysis.
//!
//! Converts PDF documents into structured markdown by analyzing pdfium text segments
//! (pre-merged character runs sharing baseline + font settings) to reconstruct headings,
//! paragraphs, inline formatting, and list items.

pub(crate) mod adapters;
mod assembly;
mod bridge;
mod classify;
mod columns;
mod constants;
pub(crate) mod content;
mod content_convert;
pub(crate) mod geometry;
pub(crate) mod layout_classify;
mod lines;
mod paragraphs;
mod pipeline;
mod regions;
mod render;
pub(in crate::pdf) mod text_repair;
pub(crate) mod types;

#[allow(unused_imports)] // Used by extractors/pdf/ocr.rs when ocr feature is enabled
pub(crate) use content_convert::{content_to_paragraphs, reorder_elements_reading_order};
pub(crate) use pipeline::render_document_as_markdown_with_tables;
pub use render::inject_image_placeholders;
#[allow(unused_imports)] // Used by extractors/pdf/ocr.rs when ocr feature is enabled
pub(crate) use render::render_paragraphs_to_string;
