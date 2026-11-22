//! Text and content extraction from PDF documents.
//!
//! Provides high-performance extraction of text, images, paths, and layout analysis.

pub mod auto;
pub mod ccitt_bilevel;
pub mod forms;
pub mod gap_statistics;
pub mod geometric_spacing;
pub mod hierarchical;
pub mod images;
pub mod page_labels;
pub mod paths;
pub mod pattern_detector;
pub mod recovery_tally;
pub mod structured;
pub mod synthetic_structure;
pub mod text;
pub mod warnings;
pub mod xmp;

pub use auto::{
    AutoExtractOptions, AutoExtractOptionsBuilder, AutoExtractor, DocumentClassification, DocumentExtraction,
    DocumentSummary, ExtractMode, ExtractSource, ExtractionStatus, ImageCodecClass, PageClassification, PageExtraction,
    PageKind, PageSignals, ProducerPrior, Quad, ReasonCode, Region, RegionKind, TableData,
};
pub use forms::{FieldType, FieldValue, FormExtractor, FormField};
pub use gap_statistics::{
    AdaptiveThresholdConfig, AdaptiveThresholdResult, GapStatistics, analyze_document_gaps, calculate_statistics,
    determine_adaptive_threshold, extract_gaps,
};
pub use geometric_spacing::{SpaceInsertion, SpacingConfig, should_insert_space};
pub use hierarchical::HierarchicalExtractor;
pub use images::{
    ColorSpace, ImageData, PdfImage, PixelFormat, expand_inline_image_dict, extract_image_from_xobject,
    extract_image_from_xobject_at,
};
pub use page_labels::{PageLabelExtractor, PageLabelRange, PageLabelStyle};
pub use paths::{FillRule, PathExtractor};
pub use pattern_detector::{PatternDetector, PatternPreservationConfig};
pub use structured::{
    BoundingBox, DocumentElement, DocumentMetadata, ExtractorConfig, ListItem, StructuredDocument, StructuredExtractor,
    TextAlignment, TextStyle,
};
pub use synthetic_structure::{SyntheticStructureConfig, SyntheticStructureGenerator};
pub use text::{SpanMergingConfig, TextExtractionConfig, TextExtractor};
pub use warnings::{Warning, WarningCategory, WarningSink};
pub use xmp::{XmpExtractor, XmpMetadata};
