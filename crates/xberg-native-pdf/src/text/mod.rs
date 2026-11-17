//! Text processing and analysis module.
//!
//! This module provides tools for working with extracted text from PDF documents,
//! including word boundary detection per ISO 32000-1:2008 specification.

pub mod bidi;
pub mod cjk_punctuation;
pub(crate) mod cjk_ranges;
pub mod complex_script_detector;
pub mod document_classifier;
pub mod hyphenation;
pub mod justification;
pub mod kangxi;
pub mod ligature_processor;
pub mod rtl_detector;
pub mod script_detector;
pub mod word_boundary;

pub use complex_script_detector::{ComplexScript, detect_complex_script, is_complex_script};
pub use document_classifier::{DocumentClassifier, DocumentStats};
pub use hyphenation::HyphenationHandler;
pub use justification::{JustificationDetector, JustificationMode};
pub use rtl_detector::{RTLScript, detect_rtl_script, is_arabic_diacritic, is_hebrew_diacritic, is_rtl_text};
pub use script_detector::{CJKScript, DocumentLanguage};
pub use word_boundary::{BoundaryContext, CharacterInfo, DocumentScript, WordBoundaryDetector, detect_word_boundaries};
