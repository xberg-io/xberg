//! Layout analysis algorithms for PDF documents.
//!
//! This module provides algorithms for analyzing document layout:
//! - DBSCAN clustering (characters → words → lines)
//! - Reading order determination
//! - Font clustering and normalization
//! - Bounded text extraction

pub mod area_filter;
pub mod clustering;
pub mod document_analyzer;
pub mod font_name;
pub mod font_stats;
pub mod reading_order;
pub mod region_classifier;
pub mod text_block;

pub mod bold_validation;
pub mod font_normalization;

pub use area_filter::{LayoutObjectSpatial, RectFilterMode, SpatialCollectionFiltering};
pub use document_analyzer::{AdaptiveLayoutParams, DocumentProperties};
pub use font_name::FontName;
pub use font_stats::PageFontStats;
pub use reading_order::graph_based_reading_order;
pub use region_classifier::{RegionClass, classify_region};
pub use text_block::{Color, FontWeight, PageText, TextBlock, TextChar, TextLine, TextSpan, Word};

pub use bold_validation::{BoldGroup, BoldMarkerDecision, BoldMarkerValidator};
pub use font_normalization::{FontWeightNormalizer, NormalizedSpan, SpanType};
