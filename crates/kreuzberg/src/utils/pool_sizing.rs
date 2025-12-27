//! Dynamic pool sizing heuristics based on document complexity.
//!
//! This module provides functions to estimate optimal pool sizes based on file size
//! and document format (MIME type). By sizing pools to match actual document complexity,
//! we reduce memory waste from pre-allocated but unused capacity.
//!
//! # Sizing Strategy
//!
//! Pool size is determined by a combination of:
//! 1. **Format-specific ratio**: Extraction overhead varies by format
//!    - PDF: 25% (binary, compression overhead)
//!    - DOCX/XLSX/PPTX: 40-45% (compressed, XML-heavy)
//!    - HTML: 65% (markup overhead)
//!    - Text/Markdown: 95% (minimal overhead)
//!    - Default: 50% (conservative)
//! 2. **File size scaling**: Larger documents benefit from more buffers
//!    - Small (< 100KB): Base allocation
//!    - Medium (100KB-1MB): +2 buffers
//!    - Large (1MB-10MB): +4 buffers
//!    - Huge (>10MB): +6 buffers
//!
//! # Example
//!
//! ```rust,ignore
//! use kreuzberg::utils::pool_sizing::estimate_pool_size;
//!
//! // 5MB PDF → pool sized at ~1.25MB (5MB * 0.25)
//! let hint = estimate_pool_size(5_000_000, "application/pdf");
//! assert_eq!(hint.estimated_total_size, 1_250_000);
//!
//! // 2MB HTML → pool sized at ~1.3MB (2MB * 0.65)
//! let hint = estimate_pool_size(2_000_000, "text/html");
//! assert_eq!(hint.estimated_total_size, 1_300_000);
//! ```

/// Hint for optimal pool sizing based on document characteristics.
///
/// This struct contains the estimated sizes for string and byte buffers
/// that should be allocated in the pool to handle extraction without
/// excessive reallocation.
#[derive(Debug, Clone, Copy)]
pub struct PoolSizeHint {
    /// Estimated total string buffer pool size in bytes
    pub estimated_total_size: usize,
    /// Recommended number of string buffers
    pub string_buffer_count: usize,
    /// Recommended capacity per string buffer in bytes
    pub string_buffer_capacity: usize,
    /// Recommended number of byte buffers
    pub byte_buffer_count: usize,
    /// Recommended capacity per byte buffer in bytes
    pub byte_buffer_capacity: usize,
}

impl PoolSizeHint {
    /// Calculate the estimated string pool memory in bytes.
    ///
    /// This is the total estimated memory for all string buffers.
    #[inline]
    pub fn estimated_string_pool_memory(&self) -> usize {
        self.string_buffer_count * self.string_buffer_capacity
    }

    /// Calculate the estimated byte pool memory in bytes.
    ///
    /// This is the total estimated memory for all byte buffers.
    #[inline]
    pub fn estimated_byte_pool_memory(&self) -> usize {
        self.byte_buffer_count * self.byte_buffer_capacity
    }

    /// Calculate the total estimated pool memory in bytes.
    ///
    /// This includes both string and byte buffer pools.
    #[inline]
    pub fn total_pool_memory(&self) -> usize {
        self.estimated_string_pool_memory() + self.estimated_byte_pool_memory()
    }
}

/// Get the format-specific extraction ratio.
///
/// This ratio represents the approximate size of extracted content
/// as a percentage of the original file size. Different formats have
/// different overhead due to compression, binary structures, markup, etc.
///
/// # Arguments
///
/// * `mime_type` - The MIME type of the document (e.g., "application/pdf")
///
/// # Returns
///
/// A ratio between 0.0 and 1.0 representing the expected extraction ratio
#[inline]
fn get_format_ratio(mime_type: &str) -> f64 {
    match mime_type {
        "text/plain" | "text/markdown" | "text/x-markdown" => 0.95,
        "text/csv" | "text/tab-separated-values" => 0.90,

        "text/html" | "text/html; charset=utf-8" => 0.65,
        "application/xml" | "text/xml" => 0.60,
        "image/svg+xml" => 0.55,

        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        | "application/vnd.openxmlformats-officedocument.wordprocessingml.macro-enabled.document"
        | "application/msword" => 0.45,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        | "application/vnd.openxmlformats-officedocument.spreadsheetml.macro-enabled.sheet"
        | "application/vnd.ms-excel" => 0.40,
        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        | "application/vnd.openxmlformats-officedocument.presentationml.macro-enabled.presentation"
        | "application/vnd.ms-powerpoint" => 0.35,

        "application/vnd.oasis.opendocument.text" => 0.45,
        "application/vnd.oasis.opendocument.spreadsheet" => 0.40,
        "application/vnd.oasis.opendocument.presentation" => 0.35,

        "application/pdf" => 0.25,

        "application/json" | "text/json" => 0.80,
        "application/x-yaml" | "text/yaml" | "text/x-yaml" | "application/yaml" => 0.85,

        "application/zip" | "application/x-zip-compressed" => 0.30,
        "application/gzip" | "application/x-gzip" => 0.25,
        "application/x-rar-compressed" => 0.30,
        "application/x-7z-compressed" => 0.25,

        _ => 0.50,
    }
}

/// Get base pool configuration for a format type.
///
/// The base configuration represents the minimum number of buffers
/// needed for typical documents of that format.
///
/// # Arguments
///
/// * `mime_type` - The MIME type of the document
///
/// # Returns
///
/// A tuple of (base_buffer_count, base_buffer_capacity)
#[inline]
fn get_format_base_config(mime_type: &str) -> (usize, usize) {
    match mime_type {
        "text/plain" | "text/markdown" | "text/x-markdown" => (2, 4096),
        "text/csv" | "text/tab-separated-values" => (3, 8192),

        "text/html" | "text/html; charset=utf-8" => (8, 16384),

        "application/xml" | "text/xml" => (5, 8192),
        "image/svg+xml" => (4, 8192),

        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        | "application/vnd.openxmlformats-officedocument.wordprocessingml.macro-enabled.document"
        | "application/msword" => (5, 8192),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        | "application/vnd.openxmlformats-officedocument.spreadsheetml.macro-enabled.sheet"
        | "application/vnd.ms-excel" => (4, 8192),
        "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        | "application/vnd.openxmlformats-officedocument.presentationml.macro-enabled.presentation"
        | "application/vnd.ms-powerpoint" => (4, 8192),

        "application/vnd.oasis.opendocument.text" => (5, 8192),
        "application/vnd.oasis.opendocument.spreadsheet" => (4, 8192),
        "application/vnd.oasis.opendocument.presentation" => (4, 8192),

        "application/pdf" => (6, 16384),

        "application/json" | "text/json" => (4, 8192),
        "application/x-yaml" | "text/yaml" | "text/x-yaml" | "application/yaml" => (4, 8192),

        _ => (3, 8192),
    }
}

/// Estimate optimal pool configuration based on document size.
///
/// Adjusts the base configuration up for larger documents to provide
/// adequate buffering for streaming extraction operations.
///
/// # Arguments
///
/// * `file_size` - Size of the file in bytes
/// * `base_count` - Base buffer count from format config
///
/// # Returns
///
/// Adjusted buffer count considering file size
#[inline]
fn adjust_for_file_size(file_size: u64, base_count: usize) -> usize {
    match file_size {
        0..=100_000 => base_count,
        100_001..=1_000_000 => base_count.saturating_add(2),
        1_000_001..=10_000_000 => base_count.saturating_add(4),
        _ => base_count.saturating_add(6),
    }
}

/// Estimate pool capacity based on file size.
///
/// Larger files benefit from larger buffers to reduce reallocation cycles
/// during extraction.
///
/// # Arguments
///
/// * `file_size` - Size of the file in bytes
///
/// # Returns
///
/// Recommended buffer capacity in bytes
#[inline]
fn estimate_buffer_capacity(file_size: u64) -> usize {
    match file_size {
        0..=10_000 => 1024,
        10_001..=100_000 => 4096,
        100_001..=1_000_000 => 16384,
        1_000_001..=10_000_000 => 65536,
        _ => 262144,
    }
}

/// Estimate optimal pool sizing based on file size and document type.
///
/// This function uses the file size and MIME type to estimate how many
/// buffers and what capacity they should have. The estimates are conservative
/// to avoid starving large document processing.
///
/// # Arguments
///
/// * `file_size` - Size of the file in bytes
/// * `mime_type` - MIME type of the document (e.g., "application/pdf")
///
/// # Returns
///
/// A `PoolSizeHint` with recommended pool configuration
///
/// # Example
///
/// ```rust,ignore
/// use kreuzberg::utils::pool_sizing::estimate_pool_size;
///
/// let hint = estimate_pool_size(5_000_000, "application/pdf");
/// // PDF at 5MB gets 10 string buffers (base 6 + 4 for size)
/// // of 65KB each (for 1-10MB files)
/// ```
#[inline]
pub fn estimate_pool_size(file_size: u64, mime_type: &str) -> PoolSizeHint {
    let format_ratio = get_format_ratio(mime_type);
    let (base_count, _base_capacity) = get_format_base_config(mime_type);

    let adjusted_string_buffer_count = adjust_for_file_size(file_size, base_count);

    let buffer_capacity = estimate_buffer_capacity(file_size);

    let estimated_total_size = (file_size as f64 * format_ratio).ceil() as usize;

    let byte_buffer_count = (adjusted_string_buffer_count / 2).max(1);
    let byte_buffer_capacity = buffer_capacity * 8;

    PoolSizeHint {
        estimated_total_size,
        string_buffer_count: adjusted_string_buffer_count,
        string_buffer_capacity: buffer_capacity,
        byte_buffer_count,
        byte_buffer_capacity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_ratio_pdf() {
        let ratio = get_format_ratio("application/pdf");
        assert_eq!(ratio, 0.25);
    }

    #[test]
    fn test_format_ratio_html() {
        let ratio = get_format_ratio("text/html");
        assert_eq!(ratio, 0.65);
    }

    #[test]
    fn test_format_ratio_docx() {
        let ratio = get_format_ratio("application/vnd.openxmlformats-officedocument.wordprocessingml.document");
        assert_eq!(ratio, 0.45);
    }

    #[test]
    fn test_format_ratio_default() {
        let ratio = get_format_ratio("application/unknown-format");
        assert_eq!(ratio, 0.50);
    }

    #[test]
    fn test_small_file_sizing() {
        let hint = estimate_pool_size(5_000, "application/pdf");
        assert_eq!(hint.string_buffer_count, 6);
        assert_eq!(hint.string_buffer_capacity, 1024);
    }

    #[test]
    fn test_medium_file_sizing() {
        let hint = estimate_pool_size(500_000, "application/pdf");
        assert_eq!(hint.string_buffer_count, 8);
        assert_eq!(hint.string_buffer_capacity, 16384);
    }

    #[test]
    fn test_large_file_sizing() {
        let hint = estimate_pool_size(5_000_000, "application/pdf");
        assert_eq!(hint.string_buffer_count, 10);
        assert_eq!(hint.string_buffer_capacity, 65536);
    }

    #[test]
    fn test_huge_file_sizing() {
        let hint = estimate_pool_size(50_000_000, "application/pdf");
        assert_eq!(hint.string_buffer_count, 12);
        assert_eq!(hint.string_buffer_capacity, 262144);
    }

    #[test]
    fn test_html_sizing() {
        let hint = estimate_pool_size(1_000_000, "text/html");
        assert_eq!(hint.string_buffer_count, 10);
        assert_eq!(hint.string_buffer_capacity, 16384);
        assert_eq!(hint.estimated_total_size, 650_000);
    }

    #[test]
    fn test_text_sizing() {
        let hint = estimate_pool_size(1_000_000, "text/plain");
        assert_eq!(hint.string_buffer_count, 4);
        assert_eq!(hint.estimated_total_size, 950_000);
    }

    #[test]
    fn test_byte_buffer_sizing() {
        let hint = estimate_pool_size(5_000_000, "application/pdf");
        assert!(hint.byte_buffer_count < hint.string_buffer_count);
        assert_eq!(hint.byte_buffer_capacity, hint.string_buffer_capacity * 8);
    }

    #[test]
    fn test_total_size_estimation() {
        let hint = estimate_pool_size(10_000_000, "application/pdf");
        assert_eq!(hint.estimated_total_size, 2_500_000);
    }

    #[test]
    fn test_xlsx_sizing() {
        let hint = estimate_pool_size(
            2_000_000,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        );
        assert_eq!(hint.estimated_total_size, 800_000);
        assert_eq!(hint.string_buffer_count, 8);
    }
}
