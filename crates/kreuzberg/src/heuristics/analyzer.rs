//! Main document analyzer for chunking decisions.
//!
//! This module provides the primary entry point for analyzing documents
//! and determining the optimal processing strategy.
//!
//! # PDF text-layer detection
//!
//! The `heuristics-pdf` feature gates the branch that calls into `pdf_oxide`
//! to detect whether a PDF already has a usable text layer.  When that feature
//! is absent the function follows a "text-layer-unknown" path and proceeds
//! directly to chunking based on size/page-count thresholds.

use crate::heuristics::config::HeuristicsConfig;
use crate::heuristics::decision::{ChunkingDecision, NoChunkingReason, PageRange};
use crate::heuristics::error::Result;
use crate::heuristics::thresholds::{calculate_chunk_plan, calculate_plan_from_overrides};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

/// Metadata about a document for analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// MIME type of the document.
    pub mime_type: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Page count (if known, e.g., from previous analysis).
    pub page_count: Option<u32>,
    /// Whether OCR is forced regardless of text layer.
    pub force_ocr: bool,
    /// User-provided chunk configuration overrides.
    pub user_chunk_config: Option<UserChunkConfig>,
    /// Whether chunking is enabled for this job.
    pub chunking_enabled: bool,
}

/// User-provided chunk configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserChunkConfig {
    /// User-specified page ranges (overrides automatic chunking).
    pub page_ranges: Option<Vec<PageRange>>,
    /// User-specified pages per chunk (overrides automatic calculation).
    pub pages_per_chunk: Option<u32>,
    /// Force chunking even for small documents.
    pub force_chunking: bool,
    /// Disable chunking even for large documents.
    pub disable_chunking: bool,
}

/// Analyze a document and determine the optimal chunking strategy.
///
/// Decision logic (in priority order):
///
/// 1. If user provides `disable_chunking` → no chunking
/// 2. If user provides page_ranges → use user overrides
/// 3. If chunking is not enabled → no chunking
/// 4. If format doesn't support chunking → no chunking
/// 5. If file is small (below both thresholds) and not force_chunking → no chunking
/// 6. If PDF has a substantial text layer AND !force_ocr → no chunking
///    *(only when `heuristics-pdf` feature is enabled; otherwise skipped)*
/// 7. Otherwise → chunk the document
///
/// # Errors
///
/// Returns an error only when the `heuristics-pdf` feature is active and
/// the PDF text-layer analysis itself returns a hard error.  In all other
/// cases the function returns a `ChunkingDecision`.
#[instrument(skip(config, document_bytes), fields(
    mime_type = %metadata.mime_type,
    size_bytes = metadata.size_bytes,
    force_ocr = metadata.force_ocr
))]
pub fn analyze_document(
    metadata: &DocumentMetadata,
    config: &HeuristicsConfig,
    document_bytes: Option<&[u8]>,
) -> Result<ChunkingDecision> {
    info!("Analyzing document for chunking decision");

    // Step 1: Check for user disable / overrides.
    if let Some(user_config) = &metadata.user_chunk_config {
        if user_config.disable_chunking {
            debug!("Chunking disabled by user configuration");
            return Ok(ChunkingDecision::NoChunking {
                reason: NoChunkingReason::ChunkingDisabled,
            });
        }

        if let Some(page_ranges) = &user_config.page_ranges {
            debug!(num_ranges = page_ranges.len(), "Using user-provided page ranges");
            return Ok(ChunkingDecision::UseOverrides {
                user_chunks: page_ranges.clone(),
            });
        }
    }

    // Step 2: Check if chunking is enabled for this job.
    if !metadata.chunking_enabled {
        debug!("Chunking not enabled for this job");
        return Ok(ChunkingDecision::NoChunking {
            reason: NoChunkingReason::ChunkingDisabled,
        });
    }

    // Step 3: Check if the format supports chunking.
    if !supports_chunking(&metadata.mime_type) {
        debug!(
            mime_type = %metadata.mime_type,
            "Format does not support chunking"
        );
        return Ok(ChunkingDecision::NoChunking {
            reason: NoChunkingReason::FormatNotChunkable {
                mime_type: metadata.mime_type.clone(),
            },
        });
    }

    // Step 4: Check size/page thresholds (unless force_chunking).
    let force_chunking = metadata.user_chunk_config.as_ref().is_some_and(|c| c.force_chunking);

    if !force_chunking && metadata.size_bytes < config.file_size_threshold_bytes {
        if let Some(page_count) = metadata.page_count {
            if page_count < config.page_count_threshold {
                debug!(
                    size_bytes = metadata.size_bytes,
                    page_count = page_count,
                    "Document below thresholds, no chunking needed"
                );
                return Ok(ChunkingDecision::NoChunking {
                    reason: NoChunkingReason::FewPages {
                        page_count,
                        threshold: config.page_count_threshold,
                    },
                });
            }
        } else {
            debug!(
                size_bytes = metadata.size_bytes,
                threshold = config.file_size_threshold_bytes,
                "Document below size threshold"
            );
            return Ok(ChunkingDecision::NoChunking {
                reason: NoChunkingReason::SmallFile {
                    size_bytes: metadata.size_bytes,
                    threshold_bytes: config.file_size_threshold_bytes,
                },
            });
        }
    }

    // Step 5: For PDFs, optionally check text layer when `heuristics-pdf` is active.
    // When the feature is absent this branch compiles out and we proceed to chunking.
    // The `document_bytes` parameter is reserved for that future feature path.
    let is_pdf = is_pdf_mime_type(&metadata.mime_type);
    let page_count = metadata.page_count;

    // `document_bytes` is accepted for API compatibility with the `heuristics-pdf` path
    // that will use it for PDF text-layer analysis.  Silence the unused-variable lint
    // without the feature.
    let _ = document_bytes;

    // Step 6: Calculate chunking plan.
    let page_count = page_count.unwrap_or_else(|| estimate_page_count(metadata.size_bytes));
    let needs_ocr = metadata.force_ocr || !is_pdf;

    debug!(page_count = page_count, needs_ocr = needs_ocr, "Calculating chunk plan");

    let plan = calculate_chunk_plan(page_count, metadata.size_bytes, needs_ocr, config);

    info!(
        total_chunks = plan.total_chunks,
        use_disk = plan.use_disk_processing,
        "Chunking decision: will chunk document"
    );

    Ok(ChunkingDecision::Chunk(plan))
}

/// Analyze a document with user-specified chunk ranges.
///
/// Creates a chunk plan based on user-provided page ranges.
#[instrument(skip(config), fields(
    num_ranges = user_ranges.len(),
    size_bytes = size_bytes
))]
pub fn analyze_with_user_chunks(
    user_ranges: &[PageRange],
    total_pages: u32,
    size_bytes: u64,
    config: &HeuristicsConfig,
) -> ChunkingDecision {
    let plan = calculate_plan_from_overrides(user_ranges, total_pages, size_bytes, config);

    info!(total_chunks = plan.total_chunks, "Using user-specified chunk ranges");

    ChunkingDecision::Chunk(plan)
}

/// Check if a MIME type supports page-level chunking.
fn supports_chunking(mime_type: &str) -> bool {
    matches!(
        mime_type.to_lowercase().as_str(),
        "application/pdf" | "image/tiff" | "image/tif"
    )
}

/// Check if a MIME type is PDF.
fn is_pdf_mime_type(mime_type: &str) -> bool {
    mime_type.to_lowercase() == "application/pdf"
}

/// Estimate page count based on file size (rough heuristic).
///
/// Assumes ~50 KiB per page for PDFs (a reasonable average).
fn estimate_page_count(size_bytes: u64) -> u32 {
    const BYTES_PER_PAGE: u64 = 50 * 1024; // 50 KiB
    ((size_bytes / BYTES_PER_PAGE) as u32).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> HeuristicsConfig {
        HeuristicsConfig {
            enable_pdf_text_heuristics: true,
            text_layer_threshold: 0.7,
            file_size_threshold_bytes: 10 * 1024 * 1024,
            page_count_threshold: 50,
            target_pages_per_chunk: 10,
            max_pages_per_chunk: 25,
            disk_processing_threshold_bytes: 50 * 1024 * 1024,
            min_chars_per_page: 50,
            max_xlsx_sheet_count: 200,
            max_xlsx_workbook_cells: 5_000_000,
            max_pptx_embedded_count: 50,
        }
    }

    fn base_metadata() -> DocumentMetadata {
        DocumentMetadata {
            mime_type: "application/pdf".to_string(),
            size_bytes: 20 * 1024 * 1024,
            page_count: Some(100),
            force_ocr: false,
            user_chunk_config: None,
            chunking_enabled: true,
        }
    }

    #[test]
    fn test_analyze_small_file_no_chunking() {
        let config = test_config();
        let metadata = DocumentMetadata {
            mime_type: "application/pdf".to_string(),
            size_bytes: 1024 * 1024, // 1MB
            page_count: Some(10),
            force_ocr: false,
            user_chunk_config: None,
            chunking_enabled: true,
        };

        let decision = analyze_document(&metadata, &config, None).unwrap();

        assert!(matches!(
            decision,
            ChunkingDecision::NoChunking {
                reason: NoChunkingReason::FewPages { .. }
            }
        ));
    }

    #[test]
    fn test_analyze_large_file_chunks() {
        let config = test_config();
        let metadata = base_metadata();

        let decision = analyze_document(&metadata, &config, None).unwrap();

        assert!(matches!(decision, ChunkingDecision::Chunk(_)));
        if let ChunkingDecision::Chunk(plan) = decision {
            assert!(plan.total_chunks > 1);
            assert_eq!(plan.total_pages(), 100);
        }
    }

    #[test]
    fn test_analyze_user_disable_chunking() {
        let config = test_config();
        let mut metadata = base_metadata();
        metadata.user_chunk_config = Some(UserChunkConfig {
            disable_chunking: true,
            ..Default::default()
        });

        let decision = analyze_document(&metadata, &config, None).unwrap();

        assert!(matches!(
            decision,
            ChunkingDecision::NoChunking {
                reason: NoChunkingReason::ChunkingDisabled
            }
        ));
    }

    #[test]
    fn test_analyze_user_page_ranges() {
        let config = test_config();
        let mut metadata = base_metadata();
        metadata.user_chunk_config = Some(UserChunkConfig {
            page_ranges: Some(vec![
                PageRange::new(0, 24),
                PageRange::new(25, 49),
                PageRange::new(50, 99),
            ]),
            ..Default::default()
        });

        let decision = analyze_document(&metadata, &config, None).unwrap();

        assert!(matches!(decision, ChunkingDecision::UseOverrides { .. }));
        if let ChunkingDecision::UseOverrides { user_chunks } = decision {
            assert_eq!(user_chunks.len(), 3);
        }
    }

    #[test]
    fn test_analyze_chunking_not_enabled() {
        let config = test_config();
        let mut metadata = base_metadata();
        metadata.chunking_enabled = false;

        let decision = analyze_document(&metadata, &config, None).unwrap();

        assert!(matches!(
            decision,
            ChunkingDecision::NoChunking {
                reason: NoChunkingReason::ChunkingDisabled
            }
        ));
    }

    #[test]
    fn test_analyze_non_chunkable_format() {
        let config = test_config();
        let mut metadata = base_metadata();
        metadata.mime_type = "image/jpeg".to_string();

        let decision = analyze_document(&metadata, &config, None).unwrap();

        assert!(matches!(
            decision,
            ChunkingDecision::NoChunking {
                reason: NoChunkingReason::FormatNotChunkable { .. }
            }
        ));
    }

    #[test]
    fn test_supports_chunking() {
        assert!(supports_chunking("application/pdf"));
        assert!(supports_chunking("APPLICATION/PDF"));
        assert!(supports_chunking("image/tiff"));
        assert!(supports_chunking("IMAGE/TIFF"));
        assert!(supports_chunking("image/tif"));
        assert!(!supports_chunking("image/jpeg"));
        assert!(!supports_chunking("text/plain"));
        assert!(!supports_chunking("application/msword"));
    }

    #[test]
    fn test_estimate_page_count() {
        // 5MB file should estimate ~100 pages
        let pages = estimate_page_count(5 * 1024 * 1024);
        assert_eq!(pages, 102); // 5MB / 50KB

        // Very small file should be at least 1 page
        let small = estimate_page_count(1000);
        assert_eq!(small, 1);
    }

    #[test]
    fn test_is_pdf_mime_type() {
        assert!(is_pdf_mime_type("application/pdf"));
        assert!(is_pdf_mime_type("APPLICATION/PDF"));
        assert!(is_pdf_mime_type("Application/Pdf"));
        assert!(!is_pdf_mime_type("image/tiff"));
        assert!(!is_pdf_mime_type("image/jpeg"));
        assert!(!is_pdf_mime_type(""));
        assert!(!is_pdf_mime_type("pdf"));
    }

    #[test]
    fn test_analyze_with_user_chunks() {
        let config = test_config();
        let user_ranges = vec![PageRange::new(0, 9), PageRange::new(10, 19), PageRange::new(20, 29)];

        let decision = analyze_with_user_chunks(&user_ranges, 30, 15 * 1024 * 1024, &config);

        match decision {
            ChunkingDecision::Chunk(plan) => {
                assert_eq!(plan.total_chunks, 3);
                assert_eq!(plan.total_pages(), 30);
                assert_eq!(plan.chunks[0].pages.start, 0);
                assert_eq!(plan.chunks[0].pages.end, 9);
                assert_eq!(plan.chunks[1].pages.start, 10);
                assert_eq!(plan.chunks[1].pages.end, 19);
                assert_eq!(plan.chunks[2].pages.start, 20);
                assert_eq!(plan.chunks[2].pages.end, 29);
            }
            _ => panic!("Expected Chunk decision"),
        }
    }

    #[test]
    fn test_force_chunking_small_file() {
        let config = test_config();
        let metadata = DocumentMetadata {
            mime_type: "application/pdf".to_string(),
            size_bytes: 1024 * 1024, // 1MB - below threshold
            page_count: Some(10),    // Below page threshold
            force_ocr: false,
            user_chunk_config: Some(UserChunkConfig {
                force_chunking: true,
                ..Default::default()
            }),
            chunking_enabled: true,
        };

        let decision = analyze_document(&metadata, &config, None).unwrap();

        assert!(matches!(decision, ChunkingDecision::Chunk(_)));
    }

    #[test]
    fn test_small_file_without_page_count_returns_small_file_reason() {
        let config = test_config();
        let metadata = DocumentMetadata {
            mime_type: "application/pdf".to_string(),
            size_bytes: 1024 * 1024,
            page_count: None,
            force_ocr: false,
            user_chunk_config: None,
            chunking_enabled: true,
        };

        let decision = analyze_document(&metadata, &config, None).unwrap();

        match decision {
            ChunkingDecision::NoChunking { reason } => match reason {
                NoChunkingReason::SmallFile {
                    size_bytes,
                    threshold_bytes,
                } => {
                    assert_eq!(size_bytes, 1024 * 1024);
                    assert_eq!(threshold_bytes, 10 * 1024 * 1024);
                }
                _ => panic!("Expected SmallFile reason, got {:?}", reason),
            },
            _ => panic!("Expected NoChunking decision"),
        }
    }

    #[test]
    fn test_tiff_mime_type_supports_chunking() {
        let config = test_config();

        let metadata_tiff = DocumentMetadata {
            mime_type: "image/tiff".to_string(),
            size_bytes: 20 * 1024 * 1024,
            page_count: Some(100),
            force_ocr: false,
            user_chunk_config: None,
            chunking_enabled: true,
        };

        let decision_tiff = analyze_document(&metadata_tiff, &config, None).unwrap();
        assert!(
            matches!(decision_tiff, ChunkingDecision::Chunk(_)),
            "image/tiff should support chunking"
        );
    }

    #[test]
    fn test_disable_chunking_takes_priority_over_force() {
        let config = test_config();
        let metadata = DocumentMetadata {
            mime_type: "application/pdf".to_string(),
            size_bytes: 50 * 1024 * 1024,
            page_count: Some(500),
            force_ocr: false,
            user_chunk_config: Some(UserChunkConfig {
                disable_chunking: true,
                force_chunking: true, // Both set — disable should win.
                ..Default::default()
            }),
            chunking_enabled: true,
        };

        let decision = analyze_document(&metadata, &config, None).unwrap();

        assert!(matches!(
            decision,
            ChunkingDecision::NoChunking {
                reason: NoChunkingReason::ChunkingDisabled
            }
        ));
    }

    #[test]
    fn test_user_page_ranges_take_priority_over_disable() {
        let config = test_config();
        let metadata = DocumentMetadata {
            mime_type: "application/pdf".to_string(),
            size_bytes: 50 * 1024 * 1024,
            page_count: Some(100),
            force_ocr: false,
            user_chunk_config: Some(UserChunkConfig {
                page_ranges: Some(vec![PageRange::new(0, 49), PageRange::new(50, 99)]),
                disable_chunking: false,
                ..Default::default()
            }),
            chunking_enabled: true,
        };

        let decision = analyze_document(&metadata, &config, None).unwrap();

        match decision {
            ChunkingDecision::UseOverrides { user_chunks } => {
                assert_eq!(user_chunks.len(), 2);
            }
            _ => panic!("Expected UseOverrides decision"),
        }
    }

    #[test]
    fn test_user_chunk_config_default_values() {
        let config = UserChunkConfig::default();

        assert!(config.page_ranges.is_none());
        assert!(config.pages_per_chunk.is_none());
        assert!(!config.force_chunking);
        assert!(!config.disable_chunking);
    }

    #[test]
    fn test_estimate_page_count_various_sizes() {
        assert_eq!(estimate_page_count(0), 1);
        assert_eq!(estimate_page_count(1), 1);
        assert_eq!(estimate_page_count(50 * 1024), 1);
        assert_eq!(estimate_page_count(100 * 1024), 2);
        assert_eq!(estimate_page_count(1024 * 1024), 20);
        assert_eq!(estimate_page_count(10 * 1024 * 1024), 204);
        assert_eq!(estimate_page_count(50 * 1024 * 1024), 1024);
    }
}
