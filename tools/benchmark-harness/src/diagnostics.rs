//! Per-document diagnostic output for poor-scoring documents.
//!
//! When a document scores below the diagnostic threshold, this module generates
//! detailed diagnostics showing unmatched blocks, missing/extra tokens, cross-type
//! matches, and noise issues. Results are written to `/tmp/xberg_diagnose/`.

use crate::noise_detection::DiagnosticReport;
use crate::quality::structural_sidecar::{self, StructuralNode, StructuralSidecar};
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

fn node_preview(index: usize, node: &StructuralNode) -> BlockPreview {
    BlockPreview {
        block_type: node.kind_name().to_string(),
        content_preview: truncate(&node.repr_text(), 120),
        index,
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
    let (unmatched_gt_blocks, unmatched_extracted_blocks, cross_type_matches, sf1) = if let Some(md_gt) = gt_markdown {
        let extracted = StructuralSidecar::from_markdown(extracted_content);
        let ground_truth = StructuralSidecar::from_markdown(md_gt);
        let matches = structural_sidecar::diagnostic_matches(&extracted, &ground_truth);
        let mut matched_extracted = vec![false; extracted.nodes.len()];
        let mut matched_gt = vec![false; ground_truth.nodes.len()];
        let mut cross_types = Vec::new();

        for (extracted_index, gt_index, similarity) in matches {
            matched_extracted[extracted_index] = true;
            matched_gt[gt_index] = true;
            let extracted_node = &extracted.nodes[extracted_index];
            let gt_node = &ground_truth.nodes[gt_index];
            if extracted_node.kind_name() != gt_node.kind_name() {
                cross_types.push(CrossTypeMatch {
                    gt_type: gt_node.kind_name().to_string(),
                    extracted_type: extracted_node.kind_name().to_string(),
                    content_similarity: similarity,
                    type_compatibility: 0.0,
                });
            }
        }

        let unmatched_gt = ground_truth
            .nodes
            .iter()
            .enumerate()
            .filter(|(index, _)| !matched_gt[*index])
            .map(|(index, node)| node_preview(index, node))
            .collect();
        let unmatched_extracted = extracted
            .nodes
            .iter()
            .enumerate()
            .filter(|(index, _)| !matched_extracted[*index])
            .map(|(index, node)| node_preview(index, node))
            .collect();
        let score = structural_sidecar::score_structural(&extracted, &ground_truth);

        (unmatched_gt, unmatched_extracted, cross_types, score.sf1)
    } else {
        (Vec::new(), Vec::new(), Vec::new(), 0.0)
    };

    let ext_tokens = crate::quality::tokenize(extracted_content);
    let gt_tokens = crate::quality::tokenize(gt_text);
    let tf1 = crate::quality::compute_f1(&ext_tokens, &gt_tokens);
    let (mut missing_tokens, mut extra_tokens) = crate::quality::compute_token_diff(&ext_tokens, &gt_tokens);
    missing_tokens.truncate(30);
    extra_tokens.truncate(30);

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

const DIAGNOSTIC_OUTPUT_ROOT: &str = "/tmp/xberg_diagnose";
const MAX_PATH_COMPONENT_LENGTH: usize = 120;
const PATH_COMPONENT_HASH_LENGTH: usize = 16;

fn sanitize_path_component(value: &str) -> String {
    const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

    if value.is_empty() {
        return "~empty".to_string();
    }

    let mut sanitized = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') {
            sanitized.push(char::from(byte));
        } else {
            sanitized.push('~');
            sanitized.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
            sanitized.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
        }
    }
    if sanitized.len() <= MAX_PATH_COMPONENT_LENGTH {
        return sanitized;
    }

    let hash = blake3::hash(value.as_bytes()).to_hex();
    let prefix_length = MAX_PATH_COMPONENT_LENGTH - PATH_COMPONENT_HASH_LENGTH - 1;
    sanitized.truncate(prefix_length);
    sanitized.push('~');
    sanitized.push_str(&hash[..PATH_COMPONENT_HASH_LENGTH]);
    sanitized
}

fn diagnostic_output_dir(root: &std::path::Path, diag: &DocumentDiagnostic) -> std::path::PathBuf {
    root.join(sanitize_path_component(&diag.doc_name))
        .join(sanitize_path_component(&diag.file_type))
        .join(sanitize_path_component(&diag.pipeline))
}

fn write_diagnostic_files_to_root(
    root: &std::path::Path,
    diag: &DocumentDiagnostic,
    gt_markdown: Option<&str>,
    extracted_content: &str,
) -> std::io::Result<()> {
    let dir = diagnostic_output_dir(root, diag);
    std::fs::create_dir_all(&dir)?;

    if let Some(md) = gt_markdown {
        std::fs::write(dir.join("gt.md"), md)?;
    }

    std::fs::write(dir.join("extracted.md"), extracted_content)?;

    let json = serde_json::to_string_pretty(diag).map_err(std::io::Error::other)?;
    std::fs::write(dir.join("diagnostic.json"), json)?;

    Ok(())
}

/// Write diagnostic files to
/// `/tmp/xberg_diagnose/{doc_name}/{file_type}/{pipeline}/`.
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
    write_diagnostic_files_to_root(
        std::path::Path::new(DIAGNOSTIC_OUTPUT_ROOT),
        diag,
        gt_markdown,
        extracted_content,
    )
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
        assert!(!diag.unmatched_gt_blocks.is_empty() || !diag.top_missing_tokens.is_empty());
    }

    #[test]
    fn test_write_diagnostic_files() {
        let temp = tempfile::tempdir().expect("create diagnostic output directory");
        let diag = diagnose_document("write_test", "pdf", "baseline", "extracted text", "ground truth", None);
        let result = write_diagnostic_files_to_root(temp.path(), &diag, Some("# GT"), "extracted text");
        assert!(result.is_ok());

        let dir = temp.path().join("write_test/pdf/baseline");
        assert!(dir.join("gt.md").exists());
        assert!(dir.join("extracted.md").exists());
        assert!(dir.join("diagnostic.json").exists());
    }

    #[test]
    fn test_diagnostic_outputs_are_pipeline_specific() {
        let temp = tempfile::tempdir().expect("create diagnostic output directory");
        let baseline = diagnose_document("collision_test", "pdf", "baseline", "baseline", "ground truth", None);
        let layout = diagnose_document("collision_test", "pdf", "layout", "layout", "ground truth", None);

        write_diagnostic_files_to_root(temp.path(), &baseline, None, "baseline output")
            .expect("write baseline diagnostics");
        write_diagnostic_files_to_root(temp.path(), &layout, None, "layout output").expect("write layout diagnostics");

        let baseline_dir = diagnostic_output_dir(temp.path(), &baseline);
        let layout_dir = diagnostic_output_dir(temp.path(), &layout);
        assert_ne!(baseline_dir, layout_dir);
        assert_eq!(
            std::fs::read_to_string(baseline_dir.join("extracted.md")).unwrap(),
            "baseline output"
        );
        assert_eq!(
            std::fs::read_to_string(layout_dir.join("extracted.md")).unwrap(),
            "layout output"
        );
    }

    #[test]
    fn test_diagnostic_output_path_sanitizes_components() {
        let diag = diagnose_document("../../report name", "../pdf", "../layout+table", "", "", None);
        let root = std::path::Path::new(DIAGNOSTIC_OUTPUT_ROOT);
        let dir = diagnostic_output_dir(root, &diag);

        assert_eq!(
            dir,
            std::path::PathBuf::from(DIAGNOSTIC_OUTPUT_ROOT)
                .join("~2E~2E~2F~2E~2E~2Freport~20name")
                .join("~2E~2E~2Fpdf")
                .join("~2E~2E~2Flayout~2Btable")
        );
        assert!(dir.starts_with(root));
    }

    #[test]
    fn test_sanitized_components_are_bounded_and_collision_resistant() {
        let common_prefix = "a".repeat(MAX_PATH_COMPONENT_LENGTH * 2);
        let first = sanitize_path_component(&format!("{common_prefix}first"));
        let second = sanitize_path_component(&format!("{common_prefix}second"));

        assert_eq!(first.len(), MAX_PATH_COMPONENT_LENGTH);
        assert_eq!(second.len(), MAX_PATH_COMPONENT_LENGTH);
        assert_ne!(first, second);
    }
}
