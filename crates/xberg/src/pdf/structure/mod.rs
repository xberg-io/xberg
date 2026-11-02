//! PDF-to-structure renderer using segment-level font analysis.
//!
//! Converts PDF documents into structured `InternalDocument` by analyzing xberg_native_pdf
//! text segments to reconstruct headings, paragraphs, inline formatting, and list items.

pub(crate) mod adapters;
mod assembly;
mod classify;
pub(crate) mod constants;
pub(crate) mod geometry;
pub(crate) mod layout_classify;
pub(crate) mod layout_debug;
pub(crate) mod lines;
mod list_marker;
pub(crate) mod page_number;
mod paragraphs;
pub(crate) mod pipeline;
pub(crate) mod regions;
pub(crate) mod table_grid;
mod text_repair;
pub(crate) mod types;

#[allow(unused_imports)]
pub(crate) use assembly::assemble_internal_document;
pub(crate) use list_marker::is_bullet_glyph;
pub(crate) use pipeline::{SegmentStructureConfig, extract_document_structure_from_segments};
