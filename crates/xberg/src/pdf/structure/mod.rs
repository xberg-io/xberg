//! PDF-to-structure renderer using segment-level font analysis.
//!
//! Converts PDF documents into structured `InternalDocument` by analyzing pdf_oxide
//! text segments to reconstruct headings, paragraphs, inline formatting, and list items.

pub(crate) mod adapters;
mod assembly;
mod classify;
pub(crate) mod constants;
pub(crate) mod geometry;
pub(crate) mod layout_classify;
pub(crate) mod layout_debug;
mod lines;
mod list_marker;
mod paragraphs;
mod pipeline;
pub(crate) mod regions;
mod text_repair;
pub(crate) mod types;

#[allow(unused_imports)]
pub(crate) use assembly::assemble_internal_document;
pub(crate) use pipeline::{SegmentStructureConfig, extract_document_structure_from_segments};
