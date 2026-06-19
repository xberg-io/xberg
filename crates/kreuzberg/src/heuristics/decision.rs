//! Chunking decision types and chunk planning.
//!
//! This module defines the core data structures for representing
//! chunking decisions and document processing plans.

use crate::heuristics::config::HeuristicsConfig;
use serde::{Deserialize, Serialize};

/// The chunking decision made by the analyzer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChunkingDecision {
    /// Process without chunking (small file, text layer detected, etc.)
    NoChunking {
        /// Reason why chunking is not needed.
        reason: NoChunkingReason,
    },
    /// Chunk according to plan.
    Chunk(ChunkPlan),
    /// Use user-provided chunk overrides.
    UseOverrides {
        /// User-specified page ranges.
        user_chunks: Vec<PageRange>,
    },
}

/// Reason for not chunking a document.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum NoChunkingReason {
    /// File is below size threshold.
    SmallFile {
        /// Actual size in bytes.
        size_bytes: u64,
        /// Threshold in bytes.
        threshold_bytes: u64,
    },
    /// Document has fewer pages than threshold.
    FewPages {
        /// Actual page count.
        page_count: u32,
        /// Threshold page count.
        threshold: u32,
    },
    /// PDF has substantial text layer (OCR not needed).
    TextLayerDetected {
        /// Percentage of pages with text (0.0 to 1.0).
        text_coverage: f32,
        /// Average characters per page.
        avg_chars_per_page: u32,
    },
    /// Document format does not support chunking.
    FormatNotChunkable {
        /// MIME type of the document.
        mime_type: String,
    },
    /// Chunking is disabled by configuration.
    #[default]
    ChunkingDisabled,
    /// Force OCR is disabled and text extraction is fast.
    FastTextExtraction,
}

/// Reason for chunking a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChunkingReason {
    /// File exceeds size threshold.
    LargeFile {
        /// Actual size in bytes.
        size_bytes: u64,
        /// Threshold in bytes.
        threshold_bytes: u64,
    },
    /// Document has many pages.
    ManyPages {
        /// Actual page count.
        page_count: u32,
        /// Threshold page count.
        threshold: u32,
    },
    /// PDF requires OCR and is large.
    OcrRequired {
        /// Page count.
        page_count: u32,
        /// Whether OCR is forced.
        force_ocr: bool,
    },
    /// Both size and page count exceed thresholds.
    LargeAndManyPages {
        /// Actual size in bytes.
        size_bytes: u64,
        /// Actual page count.
        page_count: u32,
    },
}

/// Complete chunking plan for a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkPlan {
    /// Total number of chunks.
    pub total_chunks: u32,
    /// Individual chunk information.
    pub chunks: Vec<ChunkInfo>,
    /// Estimated total processing time in milliseconds.
    pub total_estimated_time_ms: u64,
    /// Whether to use disk-based processing for large files.
    pub use_disk_processing: bool,
    /// Reason for chunking.
    pub reason: ChunkingReason,
}

impl Default for ChunkPlan {
    /// An empty plan (no chunks). The `reason` is a placeholder since an empty plan
    /// has no chunking rationale; callers always overwrite it when a real plan is built.
    fn default() -> Self {
        Self {
            total_chunks: 0,
            chunks: Vec::new(),
            total_estimated_time_ms: 0,
            use_disk_processing: false,
            reason: ChunkingReason::LargeFile {
                size_bytes: 0,
                threshold_bytes: 0,
            },
        }
    }
}

/// Information about a single chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkInfo {
    /// Zero-based chunk index.
    pub index: u32,
    /// Page range for this chunk.
    pub pages: PageRange,
    /// Estimated processing time for this chunk in milliseconds.
    pub estimated_time_ms: u64,
}

/// Page range for a chunk (0-indexed, inclusive).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageRange {
    /// Start page (0-indexed, inclusive).
    pub start: u32,
    /// End page (0-indexed, inclusive).
    pub end: u32,
}

impl PageRange {
    /// Create a new page range.
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    /// Get the number of pages in this range.
    pub fn page_count(&self) -> u32 {
        self.end - self.start + 1
    }
}

impl ChunkPlan {
    /// Get the total number of pages across all chunks.
    pub fn total_pages(&self) -> u32 {
        self.chunks.iter().map(|c| c.pages.page_count()).sum()
    }
}

impl ChunkingDecision {
    /// Check if this decision requires chunking.
    pub fn requires_chunking(&self) -> bool {
        matches!(self, ChunkingDecision::Chunk(_) | ChunkingDecision::UseOverrides { .. })
    }

    /// Get the chunk plan if chunking is required.
    pub fn chunk_plan(&self) -> Option<&ChunkPlan> {
        match self {
            ChunkingDecision::Chunk(plan) => Some(plan),
            _ => None,
        }
    }

    /// Get user overrides if present.
    pub fn user_chunks(&self) -> Option<&[PageRange]> {
        match self {
            ChunkingDecision::UseOverrides { user_chunks } => Some(user_chunks),
            _ => None,
        }
    }
}

/// Decision returned for pre-extraction rejection based on XLSX/PPTX-specific
/// resource bounds. Returns `Some(reason)` to reject; `None` to proceed.
///
/// Callers must provide counts from a pre-extraction peek (e.g. parsing
/// `xl/workbook.xml` for sheet count).
pub fn check_format_limits(
    mime_type: &str,
    sheet_count: Option<u32>,
    workbook_cells: Option<u64>,
    embedded_count: Option<u32>,
    config: &HeuristicsConfig,
) -> Option<String> {
    let is_xlsx = mime_type == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        || mime_type == "application/vnd.ms-excel";
    let is_pptx_or_docx = mime_type == "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        || mime_type == "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

    if is_xlsx
        && let Some(n) = sheet_count
        && n > config.max_xlsx_sheet_count
    {
        return Some(format!(
            "XLSX sheet count {} exceeds cap of {}",
            n, config.max_xlsx_sheet_count
        ));
    }
    if is_xlsx
        && let Some(c) = workbook_cells
        && c > config.max_xlsx_workbook_cells
    {
        return Some(format!(
            "XLSX workbook cell count {} exceeds cap of {}",
            c, config.max_xlsx_workbook_cells
        ));
    }
    if is_pptx_or_docx
        && let Some(e) = embedded_count
        && e > config.max_pptx_embedded_count
    {
        return Some(format!(
            "Embedded object count {} exceeds cap of {}",
            e, config.max_pptx_embedded_count
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_range_count() {
        let range = PageRange::new(0, 9);
        assert_eq!(range.page_count(), 10);

        let single = PageRange::new(5, 5);
        assert_eq!(single.page_count(), 1);
    }

    #[test]
    fn test_chunk_plan_total_pages() {
        let plan = ChunkPlan {
            total_chunks: 3,
            chunks: vec![
                ChunkInfo {
                    index: 0,
                    pages: PageRange::new(0, 9),
                    estimated_time_ms: 1000,
                },
                ChunkInfo {
                    index: 1,
                    pages: PageRange::new(10, 19),
                    estimated_time_ms: 1000,
                },
                ChunkInfo {
                    index: 2,
                    pages: PageRange::new(20, 24),
                    estimated_time_ms: 500,
                },
            ],
            total_estimated_time_ms: 2500,
            use_disk_processing: false,
            reason: ChunkingReason::ManyPages {
                page_count: 25,
                threshold: 10,
            },
        };

        assert_eq!(plan.total_pages(), 25);
    }

    #[test]
    fn test_chunking_decision_requires_chunking() {
        let no_chunk = ChunkingDecision::NoChunking {
            reason: NoChunkingReason::SmallFile {
                size_bytes: 1000,
                threshold_bytes: 10000,
            },
        };
        assert!(!no_chunk.requires_chunking());

        let chunk = ChunkingDecision::Chunk(ChunkPlan {
            total_chunks: 1,
            chunks: vec![],
            total_estimated_time_ms: 0,
            use_disk_processing: false,
            reason: ChunkingReason::LargeFile {
                size_bytes: 100000,
                threshold_bytes: 10000,
            },
        });
        assert!(chunk.requires_chunking());

        let overrides = ChunkingDecision::UseOverrides {
            user_chunks: vec![PageRange::new(0, 5)],
        };
        assert!(overrides.requires_chunking());
    }

    #[test]
    fn test_chunk_plan_returns_some_for_chunk_variant() {
        let plan = ChunkPlan {
            total_chunks: 2,
            chunks: vec![
                ChunkInfo {
                    index: 0,
                    pages: PageRange::new(0, 4),
                    estimated_time_ms: 500,
                },
                ChunkInfo {
                    index: 1,
                    pages: PageRange::new(5, 9),
                    estimated_time_ms: 500,
                },
            ],
            total_estimated_time_ms: 1000,
            use_disk_processing: true,
            reason: ChunkingReason::LargeFile {
                size_bytes: 50_000_000,
                threshold_bytes: 10_000_000,
            },
        };
        let decision = ChunkingDecision::Chunk(plan.clone());

        let retrieved_plan = decision.chunk_plan();
        assert!(retrieved_plan.is_some());
        let retrieved_plan = retrieved_plan.unwrap();
        assert_eq!(retrieved_plan.total_chunks, 2);
        assert_eq!(retrieved_plan.chunks.len(), 2);
        assert_eq!(retrieved_plan.total_estimated_time_ms, 1000);
        assert!(retrieved_plan.use_disk_processing);
    }

    #[test]
    fn test_chunk_plan_returns_none_for_no_chunking() {
        let decision = ChunkingDecision::NoChunking {
            reason: NoChunkingReason::SmallFile {
                size_bytes: 1000,
                threshold_bytes: 10000,
            },
        };
        assert!(decision.chunk_plan().is_none());
    }

    #[test]
    fn test_user_chunks_returns_some_for_use_overrides() {
        let chunks = vec![PageRange::new(0, 5), PageRange::new(10, 15)];
        let decision = ChunkingDecision::UseOverrides {
            user_chunks: chunks.clone(),
        };

        let retrieved_chunks = decision.user_chunks();
        assert!(retrieved_chunks.is_some());
        let retrieved_chunks = retrieved_chunks.unwrap();
        assert_eq!(retrieved_chunks.len(), 2);
        assert_eq!(retrieved_chunks[0], PageRange::new(0, 5));
        assert_eq!(retrieved_chunks[1], PageRange::new(10, 15));
    }

    #[test]
    fn test_user_chunks_returns_none_for_no_chunking() {
        let decision = ChunkingDecision::NoChunking {
            reason: NoChunkingReason::ChunkingDisabled,
        };
        assert!(decision.user_chunks().is_none());
    }

    #[test]
    fn test_all_no_chunking_reasons_in_decision() {
        let reasons = vec![
            NoChunkingReason::SmallFile {
                size_bytes: 100,
                threshold_bytes: 1000,
            },
            NoChunkingReason::FewPages {
                page_count: 2,
                threshold: 10,
            },
            NoChunkingReason::TextLayerDetected {
                text_coverage: 0.9,
                avg_chars_per_page: 2000,
            },
            NoChunkingReason::FormatNotChunkable {
                mime_type: "image/png".to_string(),
            },
            NoChunkingReason::ChunkingDisabled,
            NoChunkingReason::FastTextExtraction,
        ];

        for reason in reasons {
            let decision = ChunkingDecision::NoChunking { reason: reason.clone() };
            assert!(!decision.requires_chunking());
            assert!(decision.chunk_plan().is_none());
            assert!(decision.user_chunks().is_none());
        }
    }

    #[test]
    fn check_format_limits_passes_under_caps() {
        let config = HeuristicsConfig::default();
        let result = check_format_limits(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Some(50),
            Some(100_000),
            None,
            &config,
        );
        assert_eq!(result, None);
    }

    #[test]
    fn check_format_limits_rejects_too_many_sheets() {
        let config = HeuristicsConfig::default();
        let result = check_format_limits(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Some(500),
            None,
            None,
            &config,
        );
        assert!(result.is_some());
        let msg = result.unwrap();
        assert!(msg.contains("sheet count"));
        assert!(msg.contains("500"));
        assert!(msg.contains("200"));
    }

    #[test]
    fn check_format_limits_rejects_too_many_cells() {
        let config = HeuristicsConfig::default();
        let result = check_format_limits(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            None,
            Some(10_000_000),
            None,
            &config,
        );
        assert!(result.is_some());
        let msg = result.unwrap();
        assert!(msg.contains("cell count"));
        assert!(msg.contains("10000000"));
        assert!(msg.contains("5000000"));
    }

    #[test]
    fn check_format_limits_rejects_too_many_embedded_in_pptx() {
        let config = HeuristicsConfig::default();
        let result = check_format_limits(
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            None,
            None,
            Some(60),
            &config,
        );
        assert!(result.is_some());
        let msg = result.unwrap();
        assert!(msg.contains("Embedded object"));
        assert!(msg.contains("60"));
        assert!(msg.contains("50"));
    }

    #[test]
    fn check_format_limits_ignores_unrelated_mime() {
        let config = HeuristicsConfig::default();
        let result = check_format_limits("text/plain", Some(500), Some(10_000_000), Some(100), &config);
        assert_eq!(result, None);
    }

    #[test]
    fn check_format_limits_handles_none_counts() {
        let config = HeuristicsConfig::default();
        let result = check_format_limits(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            None,
            None,
            None,
            &config,
        );
        assert_eq!(result, None);
    }
}
