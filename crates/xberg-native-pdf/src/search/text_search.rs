//! Text search implementation with regex support.
//!
//! Provides text search functionality that tracks positions, allowing
//! matches to be highlighted or processed with their bounding boxes.

use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::geometry::Rect;
use crate::layout::TextSpan;
use regex::{Regex, RegexBuilder};

/// A search result with position information.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    /// Page number (0-indexed) where the match was found
    pub page: usize,
    /// The matched text
    pub text: String,
    /// Bounding box of the match on the page
    pub bbox: Rect,
    /// Start index in the extracted text
    pub start_index: usize,
    /// End index in the extracted text
    pub end_index: usize,
    /// Individual bounding boxes for each span that makes up the match
    /// (useful for matches spanning multiple lines)
    pub span_boxes: Vec<Rect>,
}

/// Options for text search.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct SearchOptions {
    /// Case insensitive search
    pub case_insensitive: bool,
    /// Treat pattern as literal text (not regex)
    pub literal: bool,
    /// Match whole words only
    pub whole_word: bool,
    /// Maximum number of results (0 = unlimited)
    pub max_results: usize,
    /// Page range to search (None = all pages)
    pub page_range: Option<(usize, usize)>,
}

impl SearchOptions {
    /// Create new default search options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable case-insensitive search.
    pub fn case_insensitive() -> Self {
        Self {
            case_insensitive: true,
            ..Default::default()
        }
    }

    /// Set case sensitivity.
    pub fn with_case_insensitive(mut self, value: bool) -> Self {
        self.case_insensitive = value;
        self
    }

    /// Treat pattern as literal text (escape regex special characters).
    pub fn with_literal(mut self, value: bool) -> Self {
        self.literal = value;
        self
    }

    /// Match whole words only.
    pub fn with_whole_word(mut self, value: bool) -> Self {
        self.whole_word = value;
        self
    }

    /// Limit the number of results.
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Search only within a page range (inclusive).
    pub fn with_page_range(mut self, start: usize, end: usize) -> Self {
        self.page_range = Some((start, end));
        self
    }
}

/// Lightweight per-page search index: page text plus per-span bounding
/// boxes, no font names or per-character widths.
///
/// `compute_match_bbox` only ever needs whole-span boxes, so search never
/// touches the rest of a [`TextSpan`] — caching this projection instead of
/// the full spans is cheap enough to retain for every page across repeated
/// `search()` calls on the same document (see `PdfDocument::search_index`).
pub(crate) struct SearchPageIndex {
    full_text: String,
    span_positions: Vec<(usize, usize, usize)>,
    span_boxes: Vec<Rect>,
}

impl SearchPageIndex {
    pub(crate) fn from_spans(spans: &[TextSpan]) -> Self {
        let (full_text, span_positions) = TextSearcher::build_text_with_positions(spans);
        let span_boxes = spans.iter().map(|s| s.bbox).collect();
        Self {
            full_text,
            span_positions,
            span_boxes,
        }
    }
}

/// Text searcher for PDF documents.
pub struct TextSearcher;

impl TextSearcher {
    /// Search for text in a PDF document.
    ///
    /// # Arguments
    ///
    /// * `doc` - The PDF document to search
    /// * `pattern` - The regex pattern to search for
    /// * `options` - Search options
    ///
    /// # Returns
    ///
    /// Vector of search results with positions.
    pub fn search(doc: &PdfDocument, pattern: &str, options: &SearchOptions) -> Result<Vec<SearchResult>> {
        let regex = Self::build_regex(pattern, options)?;

        let page_count = doc.page_count()?;
        let (start_page, end_page) = options.page_range.unwrap_or((0, page_count.saturating_sub(1)));

        let end_page = end_page.min(page_count.saturating_sub(1));

        let mut results = Vec::new();

        for page in start_page..=end_page {
            let page_results = Self::search_page(doc, page, &regex, options)?;
            results.extend(page_results);

            if options.max_results > 0 && results.len() >= options.max_results {
                results.truncate(options.max_results);
                break;
            }
        }

        Ok(results)
    }

    /// Search for text on a specific page.
    pub fn search_page(
        doc: &PdfDocument,
        page: usize,
        regex: &Regex,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>> {
        // Reuse the page's cached search index (page text + span boxes) if a
        // prior search() built it, instead of re-extracting the page. ~keep
        let index = doc.search_page_index(page)?;

        let mut results = Vec::new();

        for mat in regex.find_iter(&index.full_text) {
            let start = mat.start();
            let end = mat.end();
            let matched_text = mat.as_str().to_string();

            let (bbox, span_boxes) = Self::compute_match_bbox(start, end, &index.span_boxes, &index.span_positions);

            results.push(SearchResult {
                page,
                text: matched_text,
                bbox,
                start_index: start,
                end_index: end,
                span_boxes,
            });

            if options.max_results > 0 && results.len() >= options.max_results {
                break;
            }
        }

        Ok(results)
    }

    /// Build regex from pattern and options.
    fn build_regex(pattern: &str, options: &SearchOptions) -> Result<Regex> {
        let mut pattern_str = if options.literal {
            regex::escape(pattern)
        } else {
            pattern.to_string()
        };

        if options.whole_word {
            pattern_str = format!(r"\b{}\b", pattern_str);
        }

        RegexBuilder::new(&pattern_str)
            .case_insensitive(options.case_insensitive)
            .build()
            .map_err(|e| Error::InvalidPdf(format!("Invalid regex pattern: {}", e)))
    }

    /// Build concatenated text with position tracking.
    ///
    /// Returns the full text and a vector of (start_pos, end_pos, span_index)
    /// for each span.
    fn build_text_with_positions(spans: &[TextSpan]) -> (String, Vec<(usize, usize, usize)>) {
        let mut full_text = String::new();
        let mut positions = Vec::new();

        for (idx, span) in spans.iter().enumerate() {
            let start = full_text.len();
            full_text.push_str(&span.text);
            let end = full_text.len();
            positions.push((start, end, idx));

            if idx < spans.len() - 1 && !span.text.ends_with(' ') {
                full_text.push(' ');
            }
        }

        (full_text, positions)
    }

    /// Compute the bounding box for a match spanning potentially multiple spans.
    fn compute_match_bbox(
        match_start: usize,
        match_end: usize,
        span_boxes: &[Rect],
        span_positions: &[(usize, usize, usize)],
    ) -> (Rect, Vec<Rect>) {
        let mut matched_boxes = Vec::new();
        let mut combined_bbox: Option<Rect> = None;

        for &(span_start, span_end, span_idx) in span_positions {
            if span_start < match_end && span_end > match_start {
                let bbox = span_boxes[span_idx];

                // For simplicity, use the whole span's bbox
                // A more sophisticated implementation would compute character-level boxes ~keep
                matched_boxes.push(bbox);

                if let Some(ref mut combined) = combined_bbox {
                    *combined = combined.union(&bbox);
                } else {
                    combined_bbox = Some(bbox);
                }
            }
        }

        (
            combined_bbox.unwrap_or_else(|| Rect::new(0.0, 0.0, 0.0, 0.0)),
            matched_boxes,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_options_default() {
        let opts = SearchOptions::default();
        assert!(!opts.case_insensitive);
        assert!(!opts.literal);
        assert!(!opts.whole_word);
        assert_eq!(opts.max_results, 0);
        assert!(opts.page_range.is_none());
    }

    #[test]
    fn test_search_options_builder() {
        let opts = SearchOptions::new()
            .with_case_insensitive(true)
            .with_literal(true)
            .with_whole_word(true)
            .with_max_results(10)
            .with_page_range(0, 5);

        assert!(opts.case_insensitive);
        assert!(opts.literal);
        assert!(opts.whole_word);
        assert_eq!(opts.max_results, 10);
        assert_eq!(opts.page_range, Some((0, 5)));
    }

    #[test]
    fn test_build_regex_simple() {
        let opts = SearchOptions::default();
        let regex = TextSearcher::build_regex("hello", &opts).unwrap();
        assert!(regex.is_match("hello world"));
        assert!(!regex.is_match("HELLO world"));
    }

    #[test]
    fn test_build_regex_case_insensitive() {
        let opts = SearchOptions::case_insensitive();
        let regex = TextSearcher::build_regex("hello", &opts).unwrap();
        assert!(regex.is_match("hello world"));
        assert!(regex.is_match("HELLO world"));
        assert!(regex.is_match("HeLLo world"));
    }

    #[test]
    fn test_build_regex_literal() {
        let opts = SearchOptions::new().with_literal(true);
        let regex = TextSearcher::build_regex("a.b", &opts).unwrap();
        assert!(regex.is_match("a.b"));
        assert!(!regex.is_match("axb"));
    }

    #[test]
    fn test_build_regex_whole_word() {
        let opts = SearchOptions::new().with_whole_word(true);
        let regex = TextSearcher::build_regex("cat", &opts).unwrap();
        assert!(regex.is_match("the cat sat"));
        assert!(!regex.is_match("category"));
        assert!(!regex.is_match("concatenate"));
    }

    #[test]
    fn test_build_text_with_positions() {
        let spans = vec![
            TextSpan {
                provenance: None,
                text_rise: 0.0,
                artifact_type: None,
                text: "Hello".to_string(),
                bbox: Rect::new(0.0, 0.0, 50.0, 12.0),
                font_name: "Arial".to_string(),
                font_size: 12.0,
                font_weight: crate::layout::FontWeight::Normal,
                is_italic: false,
                is_monospace: false,
                color: crate::layout::Color { r: 0.0, g: 0.0, b: 0.0 },
                mcid: None,
                mcid_scope: None,
                sequence: 0,
                split_boundary_before: false,
                offset_semantic: false,
                char_spacing: 0.0,
                word_spacing: 0.0,
                horizontal_scaling: 100.0,
                primary_detected: false,
                char_widths: vec![],
                char_x_offsets: Vec::new(),
                heading_level: None,
                rotation_degrees: 0.0,
                wmode: 0,
                rtl_draw_logical: false,
                ligature_glyph: false,
                mirrored: false,
                page_rotation_applied: 0,
            },
            TextSpan {
                provenance: None,
                text_rise: 0.0,
                artifact_type: None,
                text: "World".to_string(),
                bbox: Rect::new(55.0, 0.0, 105.0, 12.0),
                font_name: "Arial".to_string(),
                font_size: 12.0,
                font_weight: crate::layout::FontWeight::Normal,
                is_italic: false,
                is_monospace: false,
                color: crate::layout::Color { r: 0.0, g: 0.0, b: 0.0 },
                mcid: None,
                mcid_scope: None,
                sequence: 1,
                split_boundary_before: false,
                offset_semantic: false,
                char_spacing: 0.0,
                word_spacing: 0.0,
                horizontal_scaling: 100.0,
                primary_detected: false,
                char_widths: vec![],
                char_x_offsets: Vec::new(),
                heading_level: None,
                rotation_degrees: 0.0,
                wmode: 0,
                rtl_draw_logical: false,
                ligature_glyph: false,
                mirrored: false,
                page_rotation_applied: 0,
            },
        ];

        let (text, positions) = TextSearcher::build_text_with_positions(&spans);

        assert_eq!(text, "Hello World");
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0], (0, 5, 0));
        assert_eq!(positions[1], (6, 11, 1));
    }
}
