//! Split-and-extract for multi-document PDFs.
//!
//! A single PDF often concatenates several logical documents (a scanned batch of
//! invoices, letters, or forms). [`split_and_extract`] detects sub-document
//! boundaries and returns one [`ExtractedDocument`] per segment from a **single
//! parse** — the caller does not have to slice the PDF externally and re-parse
//! each slice.
//!
//! Two strategies are supported (see [`SplitStrategy`]):
//!
//! - [`SplitStrategy::PageRanges`] — caller-supplied 1-indexed inclusive page
//!   ranges. Deterministic; use when the boundaries are already known.
//! - [`SplitStrategy::Auto`] — heuristic boundary detection via
//!   [`crate::heuristics::multidoc`] (blank/letterhead/density/page-one signals).
//!   Requires the `heuristics` feature.
//!
//! This module is **not** part of the language-binding surface; it is core-Rust
//! only (see `split_and_extract` in `alef.toml` `[crates.exclude]`).

use std::ops::RangeInclusive;

use crate::core::config::{ExtractInput, ExtractionConfig};
use crate::error::XbergError;
use crate::types::{DocumentCounts, ExtractedDocument};
use crate::{Result, core};

/// How to partition a multi-document PDF before extraction.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum SplitStrategy {
    /// Heuristic boundary detection (page-one markers, letterhead resets, text
    /// density shifts). Requires the `heuristics` feature.
    #[default]
    Auto,
    /// Caller-supplied 1-indexed, inclusive page ranges.
    PageRanges(Vec<RangeInclusive<u32>>),
}

/// Configuration for [`split_and_extract`].
///
/// Construct with struct-update syntax, e.g.
/// `SplitConfig { strategy: SplitStrategy::PageRanges(vec![1..=3]), ..Default::default() }`,
/// or take the `Auto` default via `SplitConfig::default()`.
#[derive(Debug, Clone, Default)]
pub struct SplitConfig {
    /// Boundary-detection strategy.
    pub strategy: SplitStrategy,
    /// Extraction config applied to every segment. Per-page content extraction
    /// is forced on internally regardless of this value.
    pub extraction: ExtractionConfig,
}

/// One extracted sub-document plus the page span it came from.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SplitSegment {
    /// 1-indexed inclusive page range in the original document.
    pub page_range: RangeInclusive<u32>,
    /// The sub-document extracted from `page_range`.
    pub document: ExtractedDocument,
}

/// Detect boundaries in a single PDF and extract each segment from one parse.
///
/// The PDF is parsed once (with per-page content extraction forced on), then
/// partitioned into segments according to `config.strategy`. Page/table/image
/// attribution is preserved in the original page coordinate system.
///
/// # Errors
///
/// - [`XbergError::UnsupportedFormat`] if `bytes` is not a PDF (PDF is the only
///   page-addressable format supported today).
/// - [`XbergError::Validation`] for an empty document, invalid page ranges, or
///   `Auto` requested without the `heuristics` feature.
/// - Any error propagated from the underlying single-document extraction.
pub async fn split_and_extract(bytes: &[u8], config: &SplitConfig) -> Result<Vec<SplitSegment>> {
    let mime = core::mime::detect_mime_type_from_bytes(bytes)?;
    if mime != "application/pdf" {
        return Err(XbergError::UnsupportedFormat(format!(
            "split_and_extract currently supports PDF only, got {mime}"
        )));
    }

    // Parse once, forcing per-page content extraction so segments can be
    // partitioned by page attribution without re-parsing.
    let mut extraction = config.extraction.clone();
    extraction
        .pages
        .get_or_insert_with(core::config::PageConfig::default)
        .extract_pages = true;

    let input = ExtractInput::from_bytes(bytes.to_vec(), mime, None);
    let result = crate::extract(input, &extraction).await?;
    let doc = result
        .results
        .into_iter()
        .next()
        .ok_or_else(|| XbergError::validation("split_and_extract: extraction produced no document"))?;

    let total_pages = doc.pages.as_ref().map_or(0, Vec::len) as u32;
    if total_pages == 0 {
        return Err(XbergError::validation(
            "split_and_extract requires a page-addressable PDF with at least one page",
        ));
    }

    let ranges = match &config.strategy {
        SplitStrategy::PageRanges(ranges) => ranges_from_page_ranges(ranges, total_pages)?,
        SplitStrategy::Auto => auto_ranges(&doc, total_pages)?,
    };

    Ok(ranges.iter().map(|range| sub_document_for_range(&doc, range)).collect())
}

/// Validate and normalize caller-supplied page ranges (1-indexed, inclusive).
fn ranges_from_page_ranges(ranges: &[RangeInclusive<u32>], total_pages: u32) -> Result<Vec<RangeInclusive<u32>>> {
    if ranges.is_empty() {
        return Err(XbergError::validation(
            "SplitStrategy::PageRanges requires at least one page range",
        ));
    }
    let mut out = Vec::with_capacity(ranges.len());
    for range in ranges {
        let (start, end) = (*range.start(), *range.end());
        if start < 1 || start > end || end > total_pages {
            return Err(XbergError::validation(format!(
                "invalid page range {start}..={end} for a {total_pages}-page document \
                 (expected 1 <= start <= end <= {total_pages})"
            )));
        }
        out.push(start..=end);
    }
    Ok(out)
}

/// Derive segment ranges from heuristic boundary detection.
#[cfg(feature = "heuristics")]
fn auto_ranges(doc: &ExtractedDocument, total_pages: u32) -> Result<Vec<RangeInclusive<u32>>> {
    use crate::heuristics::multidoc::{MultidocThresholds, boundaries_from_extraction_result};

    let boundaries = boundaries_from_extraction_result(doc, &MultidocThresholds::default());
    Ok(fold_boundaries_into_ranges(&boundaries, total_pages))
}

/// Fold detected boundaries into contiguous, non-overlapping page ranges.
///
/// Each real boundary's `start_page` is the page where a new sub-document
/// begins; `Start`/`End` are sentinels and the leading `1` seeds the first
/// segment.
#[cfg(feature = "heuristics")]
fn fold_boundaries_into_ranges(
    boundaries: &[crate::heuristics::multidoc::DocumentBoundary],
    total_pages: u32,
) -> Vec<RangeInclusive<u32>> {
    use crate::heuristics::multidoc::BoundaryReason;

    let mut starts: Vec<u32> = boundaries
        .iter()
        .filter(|b| {
            matches!(
                b.reason,
                BoundaryReason::PageOneMarker | BoundaryReason::LetterheadReset | BoundaryReason::DensityShift
            )
        })
        .map(|b| b.start_page)
        .filter(|&p| (1..=total_pages).contains(&p))
        .collect();
    starts.push(1);
    starts.sort_unstable();
    starts.dedup();

    let mut ranges = Vec::with_capacity(starts.len());
    for (index, &start) in starts.iter().enumerate() {
        let end = starts.get(index + 1).map_or(total_pages, |&next| next - 1);
        if start <= end {
            ranges.push(start..=end);
        }
    }
    ranges
}

/// `Auto` requires the `heuristics` feature; fail clearly when it is absent.
#[cfg(not(feature = "heuristics"))]
fn auto_ranges(_doc: &ExtractedDocument, _total_pages: u32) -> Result<Vec<RangeInclusive<u32>>> {
    Err(XbergError::validation(
        "SplitStrategy::Auto requires the 'heuristics' feature; use SplitStrategy::PageRanges instead",
    ))
}

/// Build one sub-document for `range` by partitioning the already-parsed data by
/// page attribution. No re-parse: pages, tables, and images are filtered from
/// the single whole-document parse.
fn sub_document_for_range(source: &ExtractedDocument, range: &RangeInclusive<u32>) -> SplitSegment {
    let (start, end) = (*range.start(), *range.end());
    let in_range = |page: u32| (start..=end).contains(&page);

    let pages: Vec<crate::types::PageContent> = source
        .pages
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter(|page| in_range(page.page_number))
        .cloned()
        .collect();

    let content = pages
        .iter()
        .map(|page| page.content.as_str())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    let tables: Vec<crate::types::Table> = source
        .tables
        .iter()
        .filter(|table| in_range(table.page_number))
        .cloned()
        .collect();

    let images: Option<Vec<crate::types::ExtractedImage>> = source.images.as_ref().map(|images| {
        images
            .iter()
            .filter(|image| image.page_number.is_some_and(in_range))
            .cloned()
            .collect()
    });

    let mut metadata = source.metadata.clone();
    if let Some(page_structure) = metadata.pages.as_mut() {
        page_structure.total_count = pages.len() as u32;
    }

    let counts = DocumentCounts {
        pages: pages.len(),
        tables: tables.len(),
        images: images.as_ref().map_or(0, Vec::len),
    };

    let document = ExtractedDocument {
        content,
        mime_type: source.mime_type.clone(),
        metadata,
        extraction_method: source.extraction_method,
        tables,
        counts,
        detected_languages: source.detected_languages.clone(),
        images,
        pages: source.pages.is_some().then_some(pages),
        ..Default::default()
    };

    SplitSegment {
        page_range: range.clone(),
        document,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::page::{PageContent, PageStructure, PageUnitType};
    use crate::types::{ExtractedImage, Metadata, Table};

    fn page(page_number: u32, content: &str) -> PageContent {
        PageContent {
            page_number,
            content: content.to_string(),
            tables: Vec::new(),
            image_indices: Vec::new(),
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
        }
    }

    fn table(page_number: u32) -> Table {
        Table {
            page_number,
            ..Default::default()
        }
    }

    fn image(page_number: u32) -> ExtractedImage {
        ExtractedImage {
            page_number: Some(page_number),
            ..Default::default()
        }
    }

    /// A 5-page document: pages carry text; tables on pages 1 and 4; images on
    /// pages 2 and 5.
    fn sample_doc() -> ExtractedDocument {
        ExtractedDocument {
            content: "whole".to_string(),
            mime_type: "application/pdf".into(),
            metadata: Metadata {
                pages: Some(PageStructure {
                    total_count: 5,
                    unit_type: PageUnitType::Page,
                    boundaries: None,
                    pages: None,
                }),
                ..Default::default()
            },
            tables: vec![table(1), table(4)],
            images: Some(vec![image(2), image(5)]),
            pages: Some((1..=5).map(|n| page(n, &format!("page {n} text"))).collect()),
            ..Default::default()
        }
    }

    #[test]
    // The `3..=2` literal is a deliberately-reversed range exercising the
    // start-after-end rejection path; clippy would otherwise flag it.
    #[allow(clippy::reversed_empty_ranges)]
    fn page_ranges_validation_rejects_out_of_bounds() {
        assert!(ranges_from_page_ranges(&[1..=6], 5).is_err(), "end beyond page count");
        assert!(ranges_from_page_ranges(&[0..=2], 5).is_err(), "start below 1");
        assert!(ranges_from_page_ranges(&[3..=2], 5).is_err(), "start after end");
        assert!(ranges_from_page_ranges(&[], 5).is_err(), "empty range list");
        assert!(ranges_from_page_ranges(&[1..=3, 4..=5], 5).is_ok());
    }

    #[test]
    fn sub_document_partitions_pages_tables_images_by_range() {
        let doc = sample_doc();
        let segment = sub_document_for_range(&doc, &(1..=3));

        assert_eq!(segment.page_range, 1..=3);
        let out = &segment.document;
        // Pages 1..=3 kept, original page numbers preserved.
        let page_numbers: Vec<u32> = out.pages.as_ref().unwrap().iter().map(|p| p.page_number).collect();
        assert_eq!(page_numbers, vec![1, 2, 3]);
        // Only the page-1 table falls in range; page-4 table excluded.
        assert_eq!(out.tables.len(), 1);
        assert_eq!(out.tables[0].page_number, 1);
        // Only the page-2 image falls in range; page-5 image excluded.
        assert_eq!(out.images.as_ref().unwrap().len(), 1);
        assert_eq!(out.images.as_ref().unwrap()[0].page_number, Some(2));
        // Content is reassembled from the segment's pages, not the whole doc.
        assert_eq!(out.content, "page 1 text\n\npage 2 text\n\npage 3 text");
        // Counts recomputed for the segment; metadata page count updated.
        assert_eq!(out.counts.pages, 3);
        assert_eq!(out.counts.tables, 1);
        assert_eq!(out.counts.images, 1);
        assert_eq!(out.metadata.pages.as_ref().unwrap().total_count, 3);
    }

    #[test]
    fn sub_document_second_segment_covers_remaining_pages() {
        let doc = sample_doc();
        let segment = sub_document_for_range(&doc, &(4..=5));
        let out = &segment.document;
        assert_eq!(out.counts.pages, 2);
        assert_eq!(out.tables[0].page_number, 4);
        assert_eq!(out.images.as_ref().unwrap()[0].page_number, Some(5));
    }

    #[cfg(feature = "heuristics")]
    #[test]
    fn auto_ranges_single_document_yields_one_full_range() {
        // A cohesive doc with no boundary signals must produce a single segment
        // spanning all pages.
        let doc = sample_doc();
        let ranges = auto_ranges(&doc, 5).unwrap();
        assert_eq!(ranges, vec![1..=5]);
    }

    #[cfg(feature = "heuristics")]
    #[test]
    fn auto_ranges_folds_boundaries_into_contiguous_ranges() {
        use crate::heuristics::multidoc::{BoundaryReason, DocumentBoundary};

        // Directly exercise the boundary→range folding: new documents start at
        // pages 3 and 5 within an 8-page PDF.
        let boundaries = vec![
            DocumentBoundary {
                start_page: 1,
                end_page: 1,
                confidence: 1.0,
                reason: BoundaryReason::Start,
            },
            DocumentBoundary {
                start_page: 3,
                end_page: 3,
                confidence: 0.9,
                reason: BoundaryReason::PageOneMarker,
            },
            DocumentBoundary {
                start_page: 5,
                end_page: 5,
                confidence: 0.85,
                reason: BoundaryReason::LetterheadReset,
            },
            DocumentBoundary {
                start_page: 8,
                end_page: 8,
                confidence: 1.0,
                reason: BoundaryReason::End,
            },
        ];
        let ranges = fold_boundaries_into_ranges(&boundaries, 8);
        assert_eq!(ranges, vec![1..=2, 3..=4, 5..=8]);
    }
}
