//! Page boundary handling and page range calculation for chunked text.
//!
//! This module provides functions to track which pages text chunks span,
//! enabling accurate page-level metadata for document processing.

use crate::error::{Result, XbergError};
use crate::types::PageBoundary;

/// Validates the consistency and correctness of page boundaries.
///
/// # Validation Rules
///
/// 1. Boundaries must be sorted by byte_start (monotonically increasing)
/// 2. Boundaries must not overlap (byte_end[i] <= byte_start[i+1])
/// 3. Each boundary must have byte_start < byte_end
///
/// # Arguments
///
/// * `boundaries` - Page boundary markers to validate
///
/// # Returns
///
/// Returns `Ok(())` if all boundaries are valid.
/// Returns `XbergError::Validation` if any boundary is invalid.
pub(crate) fn validate_page_boundaries(boundaries: &[PageBoundary]) -> Result<()> {
    if boundaries.is_empty() {
        return Ok(());
    }

    for (idx, boundary) in boundaries.iter().enumerate() {
        if boundary.byte_start > boundary.byte_end {
            return Err(XbergError::validation(format!(
                "Invalid boundary range at index {}: byte_start ({}) must be <= byte_end ({})",
                idx, boundary.byte_start, boundary.byte_end
            )));
        }
    }

    for i in 0..boundaries.len() - 1 {
        let current = &boundaries[i];
        let next = &boundaries[i + 1];

        if current.byte_start > next.byte_start {
            return Err(XbergError::validation(format!(
                "Page boundaries not sorted: boundary at index {} (byte_start={}) comes after boundary at index {} (byte_start={})",
                i,
                current.byte_start,
                i + 1,
                next.byte_start
            )));
        }

        if current.byte_end > next.byte_start {
            return Err(XbergError::validation(format!(
                "Overlapping page boundaries: boundary {} ends at {} but boundary {} starts at {}",
                i,
                current.byte_end,
                i + 1,
                next.byte_start
            )));
        }
    }

    Ok(())
}

/// Calculate which pages a byte range spans.
///
/// # Arguments
///
/// * `byte_start` - Starting byte offset of the chunk
/// * `byte_end` - Ending byte offset of the chunk
/// * `boundaries` - Page boundary markers from the document
///
/// # Returns
///
/// A tuple of (first_page, last_page) where page numbers are 1-indexed.
/// Returns (None, None) if boundaries are empty or chunk doesn't overlap any page.
///
/// # Errors
///
/// Returns `XbergError::Validation` if boundaries are invalid.
///
/// # Examples
///
/// ```rust,ignore
/// use xberg::chunking::boundaries::calculate_page_range;
/// use xberg::types::PageBoundary;
///
/// let boundaries = vec![
///     PageBoundary { byte_start: 0, byte_end: 100, page_number: 1 },
///     PageBoundary { byte_start: 100, byte_end: 200, page_number: 2 },
/// ];
///
/// let (first, last) = calculate_page_range(50, 150, &boundaries)?;
/// assert_eq!(first, Some(1));
/// assert_eq!(last, Some(2));
/// # Ok::<(), xberg::Result<()>>(())
/// ```
pub(crate) fn calculate_page_range(
    byte_start: usize,
    byte_end: usize,
    boundaries: &[PageBoundary],
) -> Result<(Option<u32>, Option<u32>)> {
    if boundaries.is_empty() {
        return Ok((None, None));
    }

    validate_page_boundaries(boundaries)?;

    let mut first_page = None;
    let mut last_page = None;

    for boundary in boundaries {
        if byte_start < boundary.byte_end && byte_end > boundary.byte_start {
            if first_page.is_none() {
                first_page = Some(boundary.page_number);
            }
            last_page = Some(boundary.page_number);
        }
    }

    Ok((first_page, last_page))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_page_boundaries_valid() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 20,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 20,
                byte_end: 40,
                page_number: 2,
            },
            PageBoundary {
                byte_start: 40,
                byte_end: 60,
                page_number: 3,
            },
        ];

        let result = validate_page_boundaries(&boundaries);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_page_boundaries_empty() {
        let boundaries: Vec<PageBoundary> = vec![];
        let result = validate_page_boundaries(&boundaries);
        assert!(result.is_ok());
    }

    #[test]
    fn test_calculate_page_range_within_page() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
        ];

        let (first, last) = calculate_page_range(10, 50, &boundaries).unwrap();
        assert_eq!(first, Some(1));
        assert_eq!(last, Some(1));
    }

    #[test]
    fn test_calculate_page_range_spanning_pages() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
        ];

        let (first, last) = calculate_page_range(50, 150, &boundaries).unwrap();
        assert_eq!(first, Some(1));
        assert_eq!(last, Some(2));
    }

    #[test]
    fn test_calculate_page_range_empty_boundaries() {
        let boundaries: Vec<PageBoundary> = vec![];

        let (first, last) = calculate_page_range(0, 50, &boundaries).unwrap();
        assert_eq!(first, None);
        assert_eq!(last, None);
    }

    #[test]
    fn test_calculate_page_range_no_overlap() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
        ];

        let (first, last) = calculate_page_range(200, 250, &boundaries).unwrap();
        assert_eq!(first, None);
        assert_eq!(last, None);
    }

    #[test]
    fn test_calculate_page_range_three_pages() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 2,
            },
            PageBoundary {
                byte_start: 200,
                byte_end: 300,
                page_number: 3,
            },
        ];

        let (first, last) = calculate_page_range(50, 250, &boundaries).unwrap();
        assert_eq!(first, Some(1));
        assert_eq!(last, Some(3));
    }

    #[test]
    fn test_calculate_page_range_with_invalid_boundaries() {
        let boundaries = vec![PageBoundary {
            byte_start: 15,
            byte_end: 10,
            page_number: 1,
        }];

        let result = calculate_page_range(0, 20, &boundaries);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid boundary range"));
    }

    #[test]
    fn test_page_boundaries_with_gaps() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 10,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 15,
                byte_end: 25,
                page_number: 2,
            },
        ];

        let result = validate_page_boundaries(&boundaries);
        assert!(result.is_ok());
    }

    #[test]
    fn test_chunk_with_same_start_and_end() {
        // Zero-length boundaries are valid (empty pages)
        let boundaries = vec![PageBoundary {
            byte_start: 10,
            byte_end: 10,
            page_number: 1,
        }];

        let result = validate_page_boundaries(&boundaries);
        assert!(result.is_ok());
    }

    #[test]
    fn test_zero_length_boundary_is_valid() {
        let boundaries = vec![
            PageBoundary {
                byte_start: 0,
                byte_end: 100,
                page_number: 1,
            },
            PageBoundary {
                byte_start: 100,
                byte_end: 100,
                page_number: 2,
            }, // empty page
            PageBoundary {
                byte_start: 100,
                byte_end: 200,
                page_number: 3,
            },
        ];
        assert!(validate_page_boundaries(&boundaries).is_ok());
    }
}
