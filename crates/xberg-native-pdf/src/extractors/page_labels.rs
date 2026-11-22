//! Page label extraction from PDF documents.
//!
//! Extracts page labeling information from PDF documents that define custom
//! page numbering schemes. See ISO 32000-1:2008, Section 12.4.2 - Page Labels.
//!
//! PDF page labels allow documents to have different numbering styles for
//! different sections. For example, a document might have:
//! - Roman numerals (i, ii, iii, iv) for the preface
//! - Arabic numerals (1, 2, 3) for the main content
//! - Prefixed numerals (A-1, A-2) for appendices

use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::{Object, ObjectRef};
use std::collections::HashSet;

const MAX_NUMBER_TREE_DEPTH: usize = 100;

/// Page numbering style as defined in PDF specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLabelStyle {
    /// Decimal Arabic numerals (1, 2, 3, ...)
    Decimal,
    /// Uppercase Roman numerals (I, II, III, IV, ...)
    RomanUpper,
    /// Lowercase Roman numerals (i, ii, iii, iv, ...)
    RomanLower,
    /// Uppercase letters (A, B, C, ... Z, AA, AB, ...)
    AlphaUpper,
    /// Lowercase letters (a, b, c, ... z, aa, ab, ...)
    AlphaLower,
    /// No numbering style (only prefix is used, if any)
    None,
}

impl PageLabelStyle {
    /// Convert from PDF name value to PageLabelStyle.
    fn from_name(name: &str) -> Self {
        match name {
            "D" => PageLabelStyle::Decimal,
            "R" => PageLabelStyle::RomanUpper,
            "r" => PageLabelStyle::RomanLower,
            "A" => PageLabelStyle::AlphaUpper,
            "a" => PageLabelStyle::AlphaLower,
            _ => PageLabelStyle::None,
        }
    }

    /// Convert to PDF name value.
    pub fn to_name(&self) -> Option<&'static str> {
        match self {
            PageLabelStyle::Decimal => Some("D"),
            PageLabelStyle::RomanUpper => Some("R"),
            PageLabelStyle::RomanLower => Some("r"),
            PageLabelStyle::AlphaUpper => Some("A"),
            PageLabelStyle::AlphaLower => Some("a"),
            PageLabelStyle::None => None,
        }
    }
}

/// A page label range definition.
///
/// Each range defines the labeling scheme for a contiguous sequence of pages
/// starting at `start_page` and continuing until the next range begins.
#[derive(Debug, Clone, PartialEq)]
pub struct PageLabelRange {
    /// The zero-based page index where this labeling range begins.
    pub start_page: usize,
    /// The numbering style for this range.
    pub style: PageLabelStyle,
    /// Optional prefix string to prepend to page numbers.
    pub prefix: Option<String>,
    /// The value of the numeric portion for the first page in this range.
    /// Default is 1 if not specified.
    pub start_value: u32,
}

impl Default for PageLabelRange {
    fn default() -> Self {
        Self {
            start_page: 0,
            style: PageLabelStyle::Decimal,
            prefix: None,
            start_value: 1,
        }
    }
}

impl PageLabelRange {
    /// Create a new page label range.
    pub fn new(start_page: usize) -> Self {
        Self {
            start_page,
            ..Default::default()
        }
    }

    /// Set the numbering style.
    pub fn with_style(mut self, style: PageLabelStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the prefix string.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Set the starting value for page numbering.
    pub fn with_start_value(mut self, start_value: u32) -> Self {
        self.start_value = start_value;
        self
    }

    /// Generate the label string for a given page within this range.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index (must be >= start_page)
    ///
    /// # Returns
    ///
    /// The formatted page label string.
    pub fn format_label(&self, page_index: usize) -> String {
        let offset = (page_index - self.start_page) as u32;
        let number = self.start_value + offset;

        let number_str = match self.style {
            PageLabelStyle::Decimal => number.to_string(),
            PageLabelStyle::RomanUpper => to_roman(number, true),
            PageLabelStyle::RomanLower => to_roman(number, false),
            PageLabelStyle::AlphaUpper => to_alpha(number, true),
            PageLabelStyle::AlphaLower => to_alpha(number, false),
            PageLabelStyle::None => String::new(),
        };

        match &self.prefix {
            Some(prefix) => format!("{}{}", prefix, number_str),
            None => number_str,
        }
    }
}

/// Page labels extractor.
pub struct PageLabelExtractor;

impl PageLabelExtractor {
    /// Helper function to resolve an Object (handles indirect references).
    fn resolve_object(doc: &PdfDocument, obj: &Object) -> Result<Object> {
        if let Some(ref_val) = obj.as_reference() {
            doc.load_object(ref_val)
        } else {
            Ok(obj.clone())
        }
    }

    /// Decode a PDF string that may be UTF-16BE (with BOM) or PDFDocEncoding.
    fn decode_text_string(bytes: &[u8]) -> Option<String> {
        if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
            // UTF-16BE with BOM ~keep
            let utf16_bytes = &bytes[2..];
            let utf16_pairs: Vec<u16> = utf16_bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                .collect();
            String::from_utf16(&utf16_pairs).ok()
        } else {
            Some(
                bytes
                    .iter()
                    .filter_map(|&b| crate::fonts::font_dict::pdfdoc_encoding_lookup(b))
                    .collect(),
            )
        }
    }

    /// Parse a page label dictionary.
    fn parse_label_dict(doc: &PdfDocument, dict_obj: &Object) -> Result<PageLabelRange> {
        let dict_resolved = Self::resolve_object(doc, dict_obj)?;
        let dict = dict_resolved
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Page label entry is not a dictionary".to_string()))?;

        let mut range = PageLabelRange::default();

        // /S - numbering style (optional) ~keep
        if let Some(style_obj) = dict.get("S") {
            if let Some(style_name) = style_obj.as_name() {
                range.style = PageLabelStyle::from_name(style_name);
            }
        } else {
            range.style = PageLabelStyle::None;
        }

        // /P - prefix string (optional) ~keep
        if let Some(prefix_obj) = dict.get("P")
            && let Some(prefix_bytes) = prefix_obj.as_string()
            && let Some(prefix_str) = Self::decode_text_string(prefix_bytes)
        {
            range.prefix = Some(prefix_str);
        }

        // /St - starting value (optional, default 1) ~keep
        if let Some(st_obj) = dict.get("St")
            && let Some(st_val) = st_obj.as_integer()
            && st_val > 0
        {
            range.start_value = st_val as u32;
        }

        Ok(range)
    }

    /// Parse a number tree to extract page label ranges.
    ///
    /// Number trees can have:
    /// - /Nums array: direct array of [key, value, key, value, ...]
    /// - /Kids array: array of intermediate nodes
    fn parse_number_tree(doc: &PdfDocument, tree_obj: &Object) -> Result<Vec<PageLabelRange>> {
        let mut visited = HashSet::new();
        Self::parse_number_tree_inner(doc, tree_obj, &mut visited, 0)
    }

    fn parse_number_tree_inner(
        doc: &PdfDocument,
        tree_obj: &Object,
        visited: &mut HashSet<ObjectRef>,
        depth: usize,
    ) -> Result<Vec<PageLabelRange>> {
        // A /Kids entry may name an ancestor; recursion without these guards overflows the stack. ~keep
        if depth > MAX_NUMBER_TREE_DEPTH {
            return Ok(Vec::new());
        }
        if let Some(node_ref) = tree_obj.as_reference()
            && !visited.insert(node_ref)
        {
            return Ok(Vec::new());
        }
        let tree_resolved = Self::resolve_object(doc, tree_obj)?;
        let tree_dict = tree_resolved
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("PageLabels is not a dictionary".to_string()))?;

        let mut ranges = Vec::new();

        if let Some(nums_obj) = tree_dict.get("Nums") {
            let nums_resolved = Self::resolve_object(doc, nums_obj)?;
            if let Some(nums_arr) = nums_resolved.as_array() {
                // Parse pairs: [page_index, label_dict, page_index, label_dict, ...] ~keep
                let mut i = 0;
                while i + 1 < nums_arr.len() {
                    let page_index_obj = &nums_arr[i];
                    let label_dict_obj = &nums_arr[i + 1];

                    if let Some(page_index) = page_index_obj.as_integer()
                        && page_index >= 0
                    {
                        let mut label_range = Self::parse_label_dict(doc, label_dict_obj)?;
                        label_range.start_page = page_index as usize;
                        ranges.push(label_range);
                    }

                    i += 2;
                }
            }
        }

        if let Some(kids_obj) = tree_dict.get("Kids") {
            let kids_resolved = Self::resolve_object(doc, kids_obj)?;
            if let Some(kids_arr) = kids_resolved.as_array() {
                for kid_obj in kids_arr {
                    let kid_ranges = Self::parse_number_tree_inner(doc, kid_obj, visited, depth + 1)?;
                    ranges.extend(kid_ranges);
                }
            }
        }

        ranges.sort_by_key(|r| r.start_page);

        Ok(ranges)
    }

    /// Extract all page label ranges from a PDF document.
    ///
    /// # Arguments
    ///
    /// * `doc` - The PDF document to extract page labels from
    ///
    /// # Returns
    ///
    /// A vector of page label ranges, sorted by start_page.
    /// Returns an empty vector if no page labels are defined.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use xberg_native_pdf::document::PdfDocument;
    /// use xberg_native_pdf::extractors::page_labels::PageLabelExtractor;
    ///
    /// let mut doc = PdfDocument::open("book.pdf")?;
    /// let labels = PageLabelExtractor::extract(&mut doc)?;
    ///
    /// for range in &labels {
    ///     println!("Page {} starts with style {:?}", range.start_page, range.style);
    /// }
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn extract(doc: &PdfDocument) -> Result<Vec<PageLabelRange>> {
        let catalog = doc.catalog()?;
        let catalog_dict = catalog
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Catalog is not a dictionary".to_string()))?;

        let page_labels_obj = match catalog_dict.get("PageLabels") {
            Some(obj) => obj.clone(),
            None => {
                return Ok(Vec::new());
            }
        };

        Self::parse_number_tree(doc, &page_labels_obj)
    }

    /// Get the label for a specific page.
    ///
    /// # Arguments
    ///
    /// * `ranges` - The page label ranges (from `extract`)
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    ///
    /// The formatted page label, or a default decimal label if no
    /// label range covers the page.
    pub fn get_label(ranges: &[PageLabelRange], page_index: usize) -> String {
        // Find the range that applies to this page
        // (the last range with start_page <= page_index) ~keep
        let range = ranges.iter().rev().find(|r| r.start_page <= page_index);

        match range {
            Some(r) => r.format_label(page_index),
            None => (page_index + 1).to_string(),
        }
    }

    /// Get labels for all pages in a document.
    ///
    /// # Arguments
    ///
    /// * `ranges` - The page label ranges (from `extract`)
    /// * `page_count` - Total number of pages in the document
    ///
    /// # Returns
    ///
    /// A vector of page labels, one for each page.
    pub fn get_all_labels(ranges: &[PageLabelRange], page_count: usize) -> Vec<String> {
        (0..page_count).map(|i| Self::get_label(ranges, i)).collect()
    }
}

/// Convert a number to Roman numerals.
fn to_roman(mut n: u32, uppercase: bool) -> String {
    if n == 0 {
        return String::new();
    }

    let numerals = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];

    let mut result = String::new();

    for (value, numeral) in numerals.iter() {
        while n >= *value {
            result.push_str(numeral);
            n -= value;
        }
    }

    if uppercase { result.to_uppercase() } else { result }
}

/// Convert a number to alphabetic representation.
/// 1=A, 2=B, ..., 26=Z, 27=AA, 28=AB, ...
fn to_alpha(mut n: u32, uppercase: bool) -> String {
    if n == 0 {
        return String::new();
    }

    let mut result = String::new();
    let base = if uppercase { b'A' } else { b'a' };

    while n > 0 {
        n -= 1;
        let c = (base + (n % 26) as u8) as char;
        result.insert(0, c);
        n /= 26;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_roman() {
        assert_eq!(to_roman(1, false), "i");
        assert_eq!(to_roman(4, false), "iv");
        assert_eq!(to_roman(9, false), "ix");
        assert_eq!(to_roman(42, false), "xlii");
        assert_eq!(to_roman(100, false), "c");
        assert_eq!(to_roman(1994, false), "mcmxciv");
        assert_eq!(to_roman(1, true), "I");
        assert_eq!(to_roman(4, true), "IV");
    }

    #[test]
    fn test_to_alpha() {
        assert_eq!(to_alpha(1, true), "A");
        assert_eq!(to_alpha(2, true), "B");
        assert_eq!(to_alpha(26, true), "Z");
        assert_eq!(to_alpha(27, true), "AA");
        assert_eq!(to_alpha(28, true), "AB");
        assert_eq!(to_alpha(52, true), "AZ");
        assert_eq!(to_alpha(53, true), "BA");
        assert_eq!(to_alpha(1, false), "a");
    }

    #[test]
    fn test_format_label() {
        let range = PageLabelRange::new(0)
            .with_style(PageLabelStyle::RomanLower)
            .with_start_value(1);

        assert_eq!(range.format_label(0), "i");
        assert_eq!(range.format_label(1), "ii");
        assert_eq!(range.format_label(2), "iii");
        assert_eq!(range.format_label(3), "iv");
    }

    #[test]
    fn test_format_label_with_prefix() {
        let range = PageLabelRange::new(0)
            .with_style(PageLabelStyle::Decimal)
            .with_prefix("A-")
            .with_start_value(8);

        assert_eq!(range.format_label(0), "A-8");
        assert_eq!(range.format_label(1), "A-9");
        assert_eq!(range.format_label(2), "A-10");
    }

    #[test]
    fn test_get_label() {
        let ranges = vec![
            PageLabelRange::new(0).with_style(PageLabelStyle::RomanLower),
            PageLabelRange::new(4).with_style(PageLabelStyle::Decimal),
            PageLabelRange::new(7)
                .with_style(PageLabelStyle::Decimal)
                .with_prefix("A-")
                .with_start_value(8),
        ];

        assert_eq!(PageLabelExtractor::get_label(&ranges, 0), "i");
        assert_eq!(PageLabelExtractor::get_label(&ranges, 1), "ii");
        assert_eq!(PageLabelExtractor::get_label(&ranges, 3), "iv");

        assert_eq!(PageLabelExtractor::get_label(&ranges, 4), "1");
        assert_eq!(PageLabelExtractor::get_label(&ranges, 5), "2");
        assert_eq!(PageLabelExtractor::get_label(&ranges, 6), "3");

        assert_eq!(PageLabelExtractor::get_label(&ranges, 7), "A-8");
        assert_eq!(PageLabelExtractor::get_label(&ranges, 8), "A-9");
    }

    #[test]
    fn test_page_label_style_from_name() {
        assert_eq!(PageLabelStyle::from_name("D"), PageLabelStyle::Decimal);
        assert_eq!(PageLabelStyle::from_name("R"), PageLabelStyle::RomanUpper);
        assert_eq!(PageLabelStyle::from_name("r"), PageLabelStyle::RomanLower);
        assert_eq!(PageLabelStyle::from_name("A"), PageLabelStyle::AlphaUpper);
        assert_eq!(PageLabelStyle::from_name("a"), PageLabelStyle::AlphaLower);
        assert_eq!(PageLabelStyle::from_name("X"), PageLabelStyle::None);
    }

    #[test]
    fn test_page_label_style_to_name() {
        assert_eq!(PageLabelStyle::Decimal.to_name(), Some("D"));
        assert_eq!(PageLabelStyle::RomanUpper.to_name(), Some("R"));
        assert_eq!(PageLabelStyle::RomanLower.to_name(), Some("r"));
        assert_eq!(PageLabelStyle::AlphaUpper.to_name(), Some("A"));
        assert_eq!(PageLabelStyle::AlphaLower.to_name(), Some("a"));
        assert_eq!(PageLabelStyle::None.to_name(), None);
    }

    #[test]
    fn test_page_label_style_roundtrip() {
        for style in [
            PageLabelStyle::Decimal,
            PageLabelStyle::RomanUpper,
            PageLabelStyle::RomanLower,
            PageLabelStyle::AlphaUpper,
            PageLabelStyle::AlphaLower,
        ] {
            let name = style.to_name().unwrap();
            assert_eq!(PageLabelStyle::from_name(name), style);
        }
    }

    #[test]
    fn test_page_label_range_default() {
        let range = PageLabelRange::default();
        assert_eq!(range.start_page, 0);
        assert_eq!(range.style, PageLabelStyle::Decimal);
        assert!(range.prefix.is_none());
        assert_eq!(range.start_value, 1);
    }

    #[test]
    fn test_page_label_range_new() {
        let range = PageLabelRange::new(5);
        assert_eq!(range.start_page, 5);
        assert_eq!(range.style, PageLabelStyle::Decimal);
        assert_eq!(range.start_value, 1);
    }

    #[test]
    fn test_page_label_range_builder() {
        let range = PageLabelRange::new(10)
            .with_style(PageLabelStyle::AlphaUpper)
            .with_prefix("App-")
            .with_start_value(3);
        assert_eq!(range.start_page, 10);
        assert_eq!(range.style, PageLabelStyle::AlphaUpper);
        assert_eq!(range.prefix.as_deref(), Some("App-"));
        assert_eq!(range.start_value, 3);
    }

    #[test]
    fn test_format_label_decimal() {
        let range = PageLabelRange::new(0).with_style(PageLabelStyle::Decimal);
        assert_eq!(range.format_label(0), "1");
        assert_eq!(range.format_label(9), "10");
    }

    #[test]
    fn test_format_label_roman_upper() {
        let range = PageLabelRange::new(0).with_style(PageLabelStyle::RomanUpper);
        assert_eq!(range.format_label(0), "I");
        assert_eq!(range.format_label(3), "IV");
        assert_eq!(range.format_label(8), "IX");
    }

    #[test]
    fn test_format_label_alpha_lower() {
        let range = PageLabelRange::new(0).with_style(PageLabelStyle::AlphaLower);
        assert_eq!(range.format_label(0), "a");
        assert_eq!(range.format_label(1), "b");
        assert_eq!(range.format_label(25), "z");
        assert_eq!(range.format_label(26), "aa");
    }

    #[test]
    fn test_format_label_alpha_upper() {
        let range = PageLabelRange::new(0).with_style(PageLabelStyle::AlphaUpper);
        assert_eq!(range.format_label(0), "A");
        assert_eq!(range.format_label(25), "Z");
        assert_eq!(range.format_label(26), "AA");
    }

    #[test]
    fn test_format_label_none_style() {
        let range = PageLabelRange::new(0).with_style(PageLabelStyle::None);
        assert_eq!(range.format_label(0), "");
        assert_eq!(range.format_label(5), "");
    }

    #[test]
    fn test_format_label_none_style_with_prefix() {
        let range = PageLabelRange::new(0)
            .with_style(PageLabelStyle::None)
            .with_prefix("Cover");
        assert_eq!(range.format_label(0), "Cover");
    }

    #[test]
    fn test_format_label_with_nonzero_start_page() {
        let range = PageLabelRange::new(5)
            .with_style(PageLabelStyle::Decimal)
            .with_start_value(10);
        assert_eq!(range.format_label(5), "10");
        assert_eq!(range.format_label(6), "11");
        assert_eq!(range.format_label(10), "15");
    }

    #[test]
    fn test_get_label_no_ranges() {
        assert_eq!(PageLabelExtractor::get_label(&[], 0), "1");
        assert_eq!(PageLabelExtractor::get_label(&[], 4), "5");
    }

    #[test]
    fn test_get_all_labels() {
        let ranges = vec![
            PageLabelRange::new(0).with_style(PageLabelStyle::RomanLower),
            PageLabelRange::new(3).with_style(PageLabelStyle::Decimal),
        ];
        let labels = PageLabelExtractor::get_all_labels(&ranges, 6);
        assert_eq!(labels, vec!["i", "ii", "iii", "1", "2", "3"]);
    }

    #[test]
    fn test_get_all_labels_empty() {
        let labels = PageLabelExtractor::get_all_labels(&[], 3);
        assert_eq!(labels, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_to_roman_zero() {
        assert_eq!(to_roman(0, false), "");
        assert_eq!(to_roman(0, true), "");
    }

    #[test]
    fn test_to_roman_large_numbers() {
        assert_eq!(to_roman(3999, false), "mmmcmxcix");
        assert_eq!(to_roman(3999, true), "MMMCMXCIX");
    }

    #[test]
    fn test_to_roman_all_subtractive() {
        assert_eq!(to_roman(4, false), "iv");
        assert_eq!(to_roman(9, false), "ix");
        assert_eq!(to_roman(40, false), "xl");
        assert_eq!(to_roman(90, false), "xc");
        assert_eq!(to_roman(400, false), "cd");
        assert_eq!(to_roman(900, false), "cm");
    }

    #[test]
    fn test_to_alpha_zero() {
        assert_eq!(to_alpha(0, true), "");
        assert_eq!(to_alpha(0, false), "");
    }

    #[test]
    fn test_to_alpha_multi_digit() {
        assert_eq!(to_alpha(702, true), "ZZ");
        assert_eq!(to_alpha(703, true), "AAA");
    }

    #[test]
    fn test_page_label_style_eq_and_copy() {
        let s1 = PageLabelStyle::Decimal;
        let s2 = s1;
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_page_label_range_clone_eq() {
        let r1 = PageLabelRange::new(0).with_prefix("X-");
        let r2 = r1.clone();
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_page_label_style_debug() {
        let debug = format!("{:?}", PageLabelStyle::RomanUpper);
        assert!(debug.contains("RomanUpper"));
    }

    #[test]
    fn test_decode_text_string_utf16be() {
        let bytes = vec![0xFE, 0xFF, 0x00, 0x41];
        let result = PageLabelExtractor::decode_text_string(&bytes);
        assert_eq!(result, Some("A".to_string()));
    }

    #[test]
    fn test_decode_text_string_pdfdoc() {
        let bytes = b"Hello";
        let result = PageLabelExtractor::decode_text_string(bytes);
        assert_eq!(result, Some("Hello".to_string()));
    }
}
