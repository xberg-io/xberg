//! PDF Logical Structure (Tagged PDF) support.
//!
//! This module implements parsing and traversal of PDF logical structure trees
//! according to ISO 32000-1:2008 Section 14.7.
//!
//! ## Overview
//!
//! Tagged PDFs contain explicit document structure that defines reading order,
//! semantic meaning, and accessibility information. This is the PDF-spec-compliant
//! way to determine reading order, as opposed to heuristic layout analysis.
//!
//! ## Structure Tree
//!
//! A structure tree consists of:
//! - **StructTreeRoot**: The root of the structure hierarchy
//! - **StructElem**: Structure elements (paragraphs, headings, sections, etc.)
//! - **ParentTree**: Maps marked content IDs to structure elements
//! - **Marked Content**: Tagged content in page streams (BMC/BDC/EMC operators)
//!
//! ## Reading Order
//!
//! Reading order is determined by pre-order traversal of the structure tree:
//! 1. Visit structure element
//! 2. Extract associated marked content
//! 3. Recursively visit children in order
//!
//! ## Example
//!
//! ```ignore
//! use xberg_native_pdf::structure::StructureTreeRoot;
//!
//! // Parse structure tree from document
//! if let Some(struct_tree) = document.structure_tree()? {
//!     // Traverse in document order
//!     for page_num in 0..document.page_count() {
//!         let ordered_content = struct_tree.extract_ordered_content(page_num)?;
//!         // Content is now in correct reading order!
//!     }
//! }
//! ```

pub mod article_threads;
pub mod converter;
mod parser;
pub mod spatial_table_detector;
pub mod table_extractor;
pub mod traversal;
pub mod types;

pub use article_threads::{ArticleThread, Bead, parse_article_threads};
pub use converter::StructureConverter;
pub(crate) use parser::checked_mcid;
#[cfg(test)]
pub(crate) use parser::with_structure_parse_cutoff;
pub use parser::{parse_structure_tree, parse_structure_tree_with_budget};
pub use spatial_table_detector::{DetectedTable, SpatialTableDetector, TableDetectionConfig, detect_tables_from_spans};
pub use table_extractor::{
    Table, TableCell, TableRow, extract_table, extract_table_from_spans, find_table_elements,
    find_table_elements_all_pages,
};
pub use traversal::{
    ListRole, OrderedContent, extract_reading_order, traverse_structure_tree, traverse_structure_tree_all_pages,
};
pub use types::{
    ActualTextIndex, MarkInfo, McidScope, ParentTree, StructChild, StructElem, StructTreeRoot, StructType,
};
