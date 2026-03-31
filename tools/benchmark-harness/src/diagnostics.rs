//! Per-document diagnostic output for poor-scoring documents.
//!
//! When a document scores below the diagnostic threshold, this module generates
//! detailed diagnostics showing unmatched blocks, missing/extra tokens, cross-type
//! matches, and noise issues. Results are written to `/tmp/kreuzberg_diagnose/`.

use crate::noise_detection::DiagnosticReport;
use serde::Serialize;

/// Full diagnostic report for a single document with poor scores.
#[derive(Debug, Serialize)]
pub struct DocumentDiagnostic {
    /// Name of the document being diagnosed.
    pub doc_name: String,
    /// File type (e.g., "pdf", "docx").
    pub file_type: String,
    /// Pipeline that produced the extraction.
    pub pipeline: String,
    /// Structural F1 score.
    pub sf1: f64,
    /// Token F1 score.
    pub tf1: f64,
    /// GT blocks that had no match in the extracted output.
    pub unmatched_gt_blocks: Vec<BlockPreview>,
    /// Extracted blocks that had no match in the ground truth.
    pub unmatched_extracted_blocks: Vec<BlockPreview>,
    /// Blocks that matched across different types (e.g., heading matched as paragraph).
    pub cross_type_matches: Vec<CrossTypeMatch>,
    /// Top tokens present in GT but missing in extraction (recall misses).
    pub top_missing_tokens: Vec<(String, usize)>,
    /// Top tokens present in extraction but absent from GT (precision misses).
    pub top_extra_tokens: Vec<(String, usize)>,
    /// Noise detection results for the extracted content.
    pub noise: DiagnosticReport,
}

/// A preview of a single markdown block for diagnostic output.
#[derive(Debug, Serialize)]
pub struct BlockPreview {
    /// Block type name (e.g., "H1", "Paragraph", "Table").
    pub block_type: String,
    /// First 120 characters of the block content.
    pub content_preview: String,
    /// Block index in the parsed sequence.
    pub index: usize,
}

/// A match between blocks of different types.
#[derive(Debug, Serialize)]
pub struct CrossTypeMatch {
    /// Ground truth block type.
    pub gt_type: String,
    /// Extracted block type.
    pub extracted_type: String,
    /// Token-level content similarity (0.0-1.0).
    pub content_similarity: f64,
    /// Type compatibility score (0.0-1.0).
    pub type_compatibility: f64,
}

/// Truncate a string to `max_len` characters, appending "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{}...", truncated)
    }
}

/// Generate diagnostics for a document with poor scores.
///
/// Analyzes the structural matching, token diffs, and noise to produce a
/// comprehensive diagnostic report explaining why the document scored poorly.
pub fn diagnose_document(
    doc_name: &str,
    file_type: &str,
    pipeline_name: &str,
    extracted_content: &str,
    gt_text: &str,
    gt_markdown: Option<&str>,
) -> DocumentDiagnostic {
    // Structural diagnostics (unmatched blocks, cross-type matches)
    let (unmatched_gt_blocks, unmatched_extracted_blocks, cross_type_matches, sf1) = if let Some(md_gt) = gt_markdown {
        let (sq, diag) = crate::markdown_quality::score_structural_quality_diagnostic(extracted_content, md_gt);

        let unmatched_gt: Vec<BlockPreview> = diag
            .unmatched_gt
            .iter()
            .map(|(idx, block)| BlockPreview {
                block_type: block.block_type.to_string(),
                content_preview: truncate(&block.content, 120),
                index: *idx,
            })
            .collect();

        let unmatched_ext: Vec<BlockPreview> = diag
            .unmatched_extracted
            .iter()
            .map(|(idx, block)| BlockPreview {
                block_type: block.block_type.to_string(),
                content_preview: truncate(&block.content, 120),
                index: *idx,
            })
            .collect();

        let cross_types: Vec<CrossTypeMatch> = diag
            .cross_type_matches
            .iter()
            .map(|(gt_block, ext_block, sim, compat)| CrossTypeMatch {
                gt_type: gt_block.block_type.to_string(),
                extracted_type: ext_block.block_type.to_string(),
                content_similarity: *sim,
                type_compatibility: *compat,
            })
            .collect();

        (unmatched_gt, unmatched_ext, cross_types, sq.structural_f1)
    } else {
        (Vec::new(), Vec::new(), Vec::new(), 0.0)
    };

    // Token diff (missing/extra tokens)
    let ext_tokens = crate::quality::tokenize(extracted_content);
    let gt_tokens = crate::quality::tokenize(gt_text);
    let tf1 = crate::quality::compute_f1(&ext_tokens, &gt_tokens);
    let (mut missing_tokens, mut extra_tokens) = crate::quality::compute_token_diff(&ext_tokens, &gt_tokens);
    missing_tokens.truncate(30);
    extra_tokens.truncate(30);

    // Noise detection
    let noise = crate::noise_detection::detect_noise(extracted_content);

    DocumentDiagnostic {
        doc_name: doc_name.to_string(),
        file_type: file_type.to_string(),
        pipeline: pipeline_name.to_string(),
        sf1,
        tf1,
        unmatched_gt_blocks,
        unmatched_extracted_blocks,
        cross_type_matches,
        top_missing_tokens: missing_tokens,
        top_extra_tokens: extra_tokens,
        noise,
    }
}

/// Write diagnostic files to `/tmp/kreuzberg_diagnose/{doc_name}/`.
///
/// Creates the directory and writes:
/// - `gt.md` — ground truth markdown (if available)
/// - `extracted.md` — extracted output
/// - `diagnostic.json` — serialized `DocumentDiagnostic`
pub fn write_diagnostic_files(
    diag: &DocumentDiagnostic,
    gt_markdown: Option<&str>,
    extracted_content: &str,
) -> std::io::Result<()> {
    let dir = std::path::PathBuf::from("/tmp/kreuzberg_diagnose").join(&diag.doc_name);
    std::fs::create_dir_all(&dir)?;

    if let Some(md) = gt_markdown {
        std::fs::write(dir.join("gt.md"), md)?;
    }

    std::fs::write(dir.join("extracted.md"), extracted_content)?;

    let json = serde_json::to_string_pretty(diag).map_err(std::io::Error::other)?;
    std::fs::write(dir.join("diagnostic.json"), json)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short() {
        assert_eq!(truncate("hello", 120), "hello");
    }

    #[test]
    fn test_truncate_long() {
        let long = "a".repeat(200);
        let result = truncate(&long, 120);
        assert!(result.ends_with("..."));
        // 120 chars + "..."
        assert_eq!(result.len(), 123);
    }

    #[test]
    fn test_diagnose_document_no_markdown_gt() {
        let diag = diagnose_document("test_doc", "pdf", "baseline", "hello world", "hello world", None);
        assert_eq!(diag.doc_name, "test_doc");
        assert_eq!(diag.file_type, "pdf");
        assert!(diag.unmatched_gt_blocks.is_empty());
        assert!(diag.unmatched_extracted_blocks.is_empty());
        assert!(diag.cross_type_matches.is_empty());
    }

    #[test]
    fn test_diagnose_document_with_markdown_gt() {
        let extracted = "# Title\n\nSome content here.";
        let gt_text = "Title Some content here.";
        let gt_md = "# Title\n\nSome content here.\n\n## Missing Section\n\nMore text.";
        let diag = diagnose_document("test_doc", "pdf", "layout", extracted, gt_text, Some(gt_md));
        assert_eq!(diag.pipeline, "layout");
        // There should be some unmatched GT blocks (the missing section)
        assert!(!diag.unmatched_gt_blocks.is_empty() || !diag.top_missing_tokens.is_empty());
    }

    #[test]
    fn test_write_diagnostic_files() {
        let diag = diagnose_document("write_test", "pdf", "baseline", "extracted text", "ground truth", None);
        let result = write_diagnostic_files(&diag, Some("# GT"), "extracted text");
        assert!(result.is_ok());

        let dir = std::path::PathBuf::from("/tmp/kreuzberg_diagnose/write_test");
        assert!(dir.join("gt.md").exists());
        assert!(dir.join("extracted.md").exists());
        assert!(dir.join("diagnostic.json").exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }
}
