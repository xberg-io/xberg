//! Structural quality scoring for markdown extraction.
//!
//! Parses markdown into typed blocks (headings, paragraphs, code, formulas, etc.)
//! and computes structural F1 scores by matching extracted blocks against ground truth.
//!
//! Uses **fuzzy cross-type matching**: blocks can match across types with a
//! continuous compatibility score. Heading level mismatches get partial credit,
//! bold paragraphs matching headings get small credit, etc.

use std::collections::HashMap;

use crate::quality::tokenize;

/// Block types in a markdown document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MdBlockType {
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    Paragraph,
    CodeBlock,
    Formula,
    Table,
    ListItem,
    Image,
}

impl MdBlockType {
    /// Weight for structural F1 scoring.
    /// Higher weights for elements that layout detection directly influences.
    fn weight(&self) -> f64 {
        match self {
            MdBlockType::Heading1
            | MdBlockType::Heading2
            | MdBlockType::Heading3
            | MdBlockType::Heading4
            | MdBlockType::Heading5
            | MdBlockType::Heading6 => 2.0,
            MdBlockType::CodeBlock | MdBlockType::Formula | MdBlockType::Table => 1.5,
            MdBlockType::ListItem => 1.0,
            MdBlockType::Paragraph | MdBlockType::Image => 0.5,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            MdBlockType::Heading1 => "H1",
            MdBlockType::Heading2 => "H2",
            MdBlockType::Heading3 => "H3",
            MdBlockType::Heading4 => "H4",
            MdBlockType::Heading5 => "H5",
            MdBlockType::Heading6 => "H6",
            MdBlockType::Paragraph => "Paragraph",
            MdBlockType::CodeBlock => "Code",
            MdBlockType::Formula => "Formula",
            MdBlockType::Table => "Table",
            MdBlockType::ListItem => "ListItem",
            MdBlockType::Image => "Image",
        }
    }

    /// Return heading level (1-6) if this is a heading, None otherwise.
    pub fn heading_level(&self) -> Option<u8> {
        match self {
            MdBlockType::Heading1 => Some(1),
            MdBlockType::Heading2 => Some(2),
            MdBlockType::Heading3 => Some(3),
            MdBlockType::Heading4 => Some(4),
            MdBlockType::Heading5 => Some(5),
            MdBlockType::Heading6 => Some(6),
            _ => None,
        }
    }

    /// True if this is any heading level.
    pub fn is_heading(&self) -> bool {
        self.heading_level().is_some()
    }
}

impl std::fmt::Display for MdBlockType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// A parsed markdown block with its type and content.
#[derive(Debug, Clone)]
pub struct MdBlock {
    pub block_type: MdBlockType,
    pub content: String,
    pub index: usize,
}

/// Per-type precision, recall, and F1.
#[derive(Debug, Clone)]
pub struct TypeScore {
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub count_extracted: usize,
    pub count_gt: usize,
}

/// Overall structural quality metrics.
#[derive(Debug, Clone)]
pub struct StructuralQuality {
    /// Weighted structural F1 across all block types.
    pub structural_f1: f64,
    /// Per-block-type scores.
    pub per_type: HashMap<MdBlockType, TypeScore>,
    /// Reading order score (LIS-based, 0.0-1.0).
    pub order_score: f64,
    /// Bag-of-words text F1 (regression guard).
    pub text_f1: f64,
}

// ---------------------------------------------------------------------------
// Block parsing (unchanged)
// ---------------------------------------------------------------------------

/// Parse a markdown string into a sequence of typed blocks.
pub fn parse_markdown_blocks(md: &str) -> Vec<MdBlock> {
    let mut blocks: Vec<MdBlock> = Vec::new();
    let lines: Vec<&str> = md.lines().collect();
    let mut i = 0;
    let mut index = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Skip blank lines
        if trimmed.is_empty() {
            i += 1;
            continue;
        }

        // Code block (fenced)
        if trimmed.starts_with("```") {
            let mut content = String::new();
            i += 1; // skip opening fence
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                i += 1; // skip closing fence
            }
            blocks.push(MdBlock {
                block_type: MdBlockType::CodeBlock,
                content,
                index,
            });
            index += 1;
            continue;
        }

        // Formula block ($$...$$)
        if trimmed.starts_with("$$") && !trimmed[2..].contains("$$") {
            let mut content = String::new();
            i += 1;
            while i < lines.len() && !lines[i].trim().starts_with("$$") {
                if !content.is_empty() {
                    content.push('\n');
                }
                content.push_str(lines[i].trim());
                i += 1;
            }
            if i < lines.len() {
                i += 1; // skip closing $$
            }
            blocks.push(MdBlock {
                block_type: MdBlockType::Formula,
                content,
                index,
            });
            index += 1;
            continue;
        }

        // Headings
        if let Some(heading) = parse_heading(trimmed) {
            blocks.push(MdBlock {
                block_type: heading.0,
                content: heading.1.to_string(),
                index,
            });
            index += 1;
            i += 1;
            continue;
        }

        // Table (consecutive lines starting with |)
        if trimmed.starts_with('|') {
            let mut content = String::new();
            while i < lines.len() && lines[i].trim().starts_with('|') {
                let table_line = lines[i].trim();
                // Skip separator lines (|---|---| or |:--|--:|)
                // A separator cell contains only dashes with optional leading/trailing colons.
                if !is_table_separator(table_line) {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(table_line);
                }
                i += 1;
            }
            if !content.is_empty() {
                blocks.push(MdBlock {
                    block_type: MdBlockType::Table,
                    content,
                    index,
                });
                index += 1;
            }
            continue;
        }

        // Image
        if trimmed.starts_with("![") {
            blocks.push(MdBlock {
                block_type: MdBlockType::Image,
                content: trimmed.to_string(),
                index,
            });
            index += 1;
            i += 1;
            continue;
        }

        // List item
        if is_list_item(trimmed) {
            let content = strip_list_prefix(trimmed);
            blocks.push(MdBlock {
                block_type: MdBlockType::ListItem,
                content,
                index,
            });
            index += 1;
            i += 1;
            continue;
        }

        // Paragraph (group consecutive non-blank, non-special lines)
        let mut content = String::new();
        while i < lines.len() {
            let line = lines[i].trim();
            if line.is_empty()
                || line.starts_with('#')
                || line.starts_with("```")
                || line.starts_with("$$")
                || line.starts_with('|')
                || line.starts_with("![")
                || is_list_item(line)
            {
                break;
            }
            if !content.is_empty() {
                content.push(' ');
            }
            content.push_str(line);
            i += 1;
        }
        if !content.is_empty() {
            blocks.push(MdBlock {
                block_type: MdBlockType::Paragraph,
                content,
                index,
            });
            index += 1;
        } else {
            // Line was unrecognized (e.g. "#1: ..." which isn't a valid heading).
            // Skip it to avoid an infinite loop.
            i += 1;
        }
    }

    blocks
}

fn parse_heading(line: &str) -> Option<(MdBlockType, &str)> {
    if let Some(rest) = line.strip_prefix("######") {
        Some((MdBlockType::Heading6, rest.trim_start()))
    } else if let Some(rest) = line.strip_prefix("#####") {
        Some((MdBlockType::Heading5, rest.trim_start()))
    } else if let Some(rest) = line.strip_prefix("####") {
        Some((MdBlockType::Heading4, rest.trim_start()))
    } else if let Some(rest) = line.strip_prefix("###") {
        Some((MdBlockType::Heading3, rest.trim_start()))
    } else if let Some(rest) = line.strip_prefix("##") {
        // Must have space after ## to be a heading
        if rest.starts_with(' ') || rest.is_empty() {
            Some((MdBlockType::Heading2, rest.trim_start()))
        } else {
            None
        }
    } else if let Some(rest) = line.strip_prefix('#') {
        if rest.starts_with(' ') || rest.is_empty() {
            Some((MdBlockType::Heading1, rest.trim_start()))
        } else {
            None
        }
    } else {
        None
    }
}

/// Check if a table line is a separator (e.g., `|---|:--:|--:|`).
/// Each cell between pipes must contain at least one dash and only dashes, colons, and spaces.
fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim().trim_start_matches('|').trim_end_matches('|');
    if trimmed.is_empty() {
        return false;
    }
    trimmed.split('|').all(|cell| {
        let c = cell.trim();
        !c.is_empty() && c.contains('-') && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
    })
}

fn is_list_item(line: &str) -> bool {
    line.starts_with("- ")
        || line.starts_with("* ")
        || line.starts_with("+ ")
        || (line.len() >= 3 && line.chars().next().is_some_and(|c| c.is_ascii_digit()) && line.contains(". "))
}

fn strip_list_prefix(line: &str) -> String {
    if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
        line[2..].to_string()
    } else if let Some(dot_pos) = line.find(". ") {
        if line[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
            line[dot_pos + 2..].to_string()
        } else {
            line.to_string()
        }
    } else {
        line.to_string()
    }
}

// ---------------------------------------------------------------------------
// Type compatibility matrix
// ---------------------------------------------------------------------------

/// Compute type compatibility score between an extracted block and a GT block.
///
/// Returns a value in `[0.0, 1.0]` representing how compatible the types are:
/// - 1.0 = exact type match
/// - 0.6-0.9 = heading level mismatch (still a heading)
/// - 0.3 = Code ↔ Formula (math sometimes code-fenced)
/// - 0.15 = bold paragraph ↔ heading (pseudo-heading detection)
/// - 0.0 = incompatible types
fn type_compat(ext_block: &MdBlock, gt_block: &MdBlock) -> f64 {
    let ext = ext_block.block_type;
    let gt = gt_block.block_type;

    // Exact match
    if ext == gt {
        return 1.0;
    }

    // Heading ↔ Heading: partial credit based on level distance
    if let (Some(ext_level), Some(gt_level)) = (ext.heading_level(), gt.heading_level()) {
        let distance = (ext_level as i8 - gt_level as i8).unsigned_abs();
        return (1.0 - 0.1 * distance as f64).max(0.6);
    }

    // Heading ↔ Paragraph: partial credit — wrong type but content preserved
    if ext.is_heading() && gt == MdBlockType::Paragraph {
        return 0.3;
    }
    if ext == MdBlockType::Paragraph && gt.is_heading() {
        if is_bold_wrapped(&ext_block.content) {
            return 0.4; // bold paragraph is a plausible heading
        }
        return 0.2; // missed heading detection, but content is there
    }

    // ListItem ↔ Paragraph: structurally different but close
    if (ext == MdBlockType::ListItem && gt == MdBlockType::Paragraph)
        || (ext == MdBlockType::Paragraph && gt == MdBlockType::ListItem)
    {
        return 0.5;
    }

    // Code ↔ Paragraph: code block false positives should get partial credit
    if (ext == MdBlockType::CodeBlock && gt == MdBlockType::Paragraph)
        || (ext == MdBlockType::Paragraph && gt == MdBlockType::CodeBlock)
    {
        return 0.2;
    }

    // Code ↔ Formula: math content sometimes gets code-fenced
    if (ext == MdBlockType::CodeBlock && gt == MdBlockType::Formula)
        || (ext == MdBlockType::Formula && gt == MdBlockType::CodeBlock)
    {
        return 0.3;
    }

    // Table ↔ Paragraph: table extraction failures
    if (ext == MdBlockType::Table && gt == MdBlockType::Paragraph)
        || (ext == MdBlockType::Paragraph && gt == MdBlockType::Table)
    {
        return 0.25;
    }

    // Everything else cross-category: incompatible
    0.0
}

/// Check if content is bold-wrapped (e.g., `**Title**` or `__Title__`).
fn is_bold_wrapped(content: &str) -> bool {
    let trimmed = content.trim();
    (trimmed.starts_with("**") && trimmed.ends_with("**") && trimmed.len() > 4)
        || (trimmed.starts_with("__") && trimmed.ends_with("__") && trimmed.len() > 4)
}

/// Truncate a string to `max_len` chars, appending "..." if truncated.
fn truncate_preview(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.replace('\n', "\\n")
    } else {
        let truncated: String = s.chars().take(max_len).collect();
        format!("{}...", truncated.replace('\n', "\\n"))
    }
}

// ---------------------------------------------------------------------------
// Scoring entry points
// ---------------------------------------------------------------------------

/// Compute structural quality by comparing extracted markdown against ground truth.
///
/// Uses fuzzy cross-type matching: heading level mismatches get partial credit,
/// bold paragraphs can match headings, etc.
pub fn score_structural_quality(extracted_md: &str, ground_truth_md: &str) -> StructuralQuality {
    score_structural_quality_impl(extracted_md, ground_truth_md)
}

/// Like `score_structural_quality` — both now use the same fuzzy matching.
///
/// Kept for backward compatibility; heading normalization is subsumed by the
/// fuzzy type compatibility system.
pub fn score_structural_quality_normalized(extracted_md: &str, ground_truth_md: &str) -> StructuralQuality {
    score_structural_quality_impl(extracted_md, ground_truth_md)
}

/// Maximum total candidate pairs before falling back to count-based scoring.
const MAX_PAIRS_FOR_MATCHING: usize = 40_000;

fn score_structural_quality_impl(extracted_md: &str, ground_truth_md: &str) -> StructuralQuality {
    let ext_blocks = parse_markdown_blocks(extracted_md);
    let gt_blocks = parse_markdown_blocks(ground_truth_md);

    let count_ext = ext_blocks.len();
    let count_gt = gt_blocks.len();

    tracing::debug!(
        ext_blocks = count_ext,
        gt_blocks = count_gt,
        "scoring structural quality"
    );

    // Global cross-type matching
    let (match_results, all_matches) = match_blocks_global(&gt_blocks, &ext_blocks);

    // Compute weighted SF1 directly from global matches.
    // Each match contributes its match_score weighted by the GT block's type weight.
    // Unmatched GT blocks contribute 0 recall; unmatched ext blocks penalize precision.
    let structural_f1 = compute_weighted_sf1_from_matches(&gt_blocks, &ext_blocks, &match_results);

    // Derive per-type scores for diagnostic breakdown
    let per_type = derive_per_type_scores(&gt_blocks, &ext_blocks, &match_results);

    // Order score using longest increasing subsequence
    let order_score = compute_order_score(&all_matches);

    // Text F1 (bag-of-words regression guard)
    let ext_tokens = tokenize(extracted_md);
    let gt_tokens = tokenize(ground_truth_md);
    let text_f1 = crate::quality::compute_f1(&ext_tokens, &gt_tokens);

    let ext_used: usize = match_results.iter().map(|m| if m.is_concat { 2 } else { 1 }).sum();
    tracing::debug!(
        sf1 = format!("{:.3}", structural_f1),
        order = format!("{:.3}", order_score),
        text_f1 = format!("{:.3}", text_f1),
        matched = match_results.len(),
        unmatched_gt = count_gt.saturating_sub(match_results.len()),
        unmatched_ext = count_ext.saturating_sub(ext_used),
        "structural quality scored"
    );

    StructuralQuality {
        structural_f1,
        per_type,
        order_score,
        text_f1,
    }
}

// ---------------------------------------------------------------------------
// Global cross-type matching
// ---------------------------------------------------------------------------

/// A matched pair of blocks with scoring details.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct BlockMatch {
    gt_idx: usize,
    ext_idx: usize,
    content_sim: f64,
    type_compat: f64,
    match_score: f64,
    is_concat: bool,
}

/// Match GT blocks against extracted blocks using fuzzy cross-type matching.
///
/// Returns (matched pairs, index pairs for order scoring).
fn match_blocks_global(gt_blocks: &[MdBlock], ext_blocks: &[MdBlock]) -> (Vec<BlockMatch>, Vec<(usize, usize)>) {
    let count_gt = gt_blocks.len();
    let count_ext = ext_blocks.len();

    if count_gt == 0 || count_ext == 0 {
        // Log unmatched blocks
        for (i, b) in gt_blocks.iter().enumerate() {
            tracing::trace!(
                idx = i,
                block_type = %b.block_type,
                preview = %truncate_preview(&b.content, 60),
                "MISS_GT no extracted blocks"
            );
        }
        for (i, b) in ext_blocks.iter().enumerate() {
            tracing::trace!(
                idx = i,
                block_type = %b.block_type,
                preview = %truncate_preview(&b.content, 60),
                "MISS_EXT false positive"
            );
        }
        return (Vec::new(), Vec::new());
    }

    // Complexity safeguard: fall back to count-ratio for very large documents
    if count_gt * count_ext > MAX_PAIRS_FOR_MATCHING {
        tracing::debug!(
            gt = count_gt,
            ext = count_ext,
            "block count too large, using count-ratio fallback"
        );
        return (Vec::new(), Vec::new());
    }

    // Pre-tokenize all blocks
    let gt_tokens: Vec<Vec<String>> = gt_blocks.iter().map(|b| tokenize(&b.content)).collect();
    let ext_tokens: Vec<Vec<String>> = ext_blocks.iter().map(|b| tokenize(&b.content)).collect();

    // Build candidate pairs across ALL types
    let mut candidates: Vec<(usize, usize, f64, f64, f64, bool)> = Vec::new(); // (gi, ei, content_sim, compat, score, is_concat)

    for (gi, gt_tok) in gt_tokens.iter().enumerate() {
        for (ei, ext_tok) in ext_tokens.iter().enumerate() {
            let compat = type_compat(&ext_blocks[ei], &gt_blocks[gi]);
            if compat <= 0.0 {
                continue;
            }

            let content_sim = crate::quality::compute_f1(ext_tok, gt_tok);
            let score = content_sim * compat;
            if score >= 0.10 {
                candidates.push((gi, ei, content_sim, compat, score, false));
            }

            // Adjacent concatenation: try ext[ei] + ext[ei+1]
            if ei + 1 < ext_tokens.len() {
                let concat_compat = type_compat(&ext_blocks[ei], &gt_blocks[gi]);
                if concat_compat <= 0.0 {
                    continue;
                }
                let mut concat_tokens = ext_tok.clone();
                concat_tokens.extend(ext_tokens[ei + 1].iter().cloned());
                let concat_sim = crate::quality::compute_f1(&concat_tokens, gt_tok);
                let concat_score = concat_sim * concat_compat;
                if concat_score > score && concat_score >= 0.10 {
                    candidates.push((gi, ei, concat_sim, concat_compat, concat_score, true));
                }
            }
        }
    }

    // Greedy matching: sort by score descending
    candidates.sort_by(|a, b| b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal));

    let mut matched_gt: Vec<bool> = vec![false; count_gt];
    let mut matched_ext: Vec<bool> = vec![false; count_ext];
    let mut results: Vec<BlockMatch> = Vec::new();
    let mut order_pairs: Vec<(usize, usize)> = Vec::new();

    for &(gi, ei, content_sim, compat, score, is_concat) in &candidates {
        if matched_gt[gi] || matched_ext[ei] {
            // Log high-scoring rejected candidates
            if score > 0.5 {
                tracing::trace!(
                    gt_idx = gi,
                    ext_idx = ei,
                    score = format!("{:.3}", score),
                    "REJECTED already matched"
                );
            }
            continue;
        }
        if is_concat && ei + 1 < count_ext && matched_ext[ei + 1] {
            continue;
        }

        matched_gt[gi] = true;
        matched_ext[ei] = true;
        if is_concat && ei + 1 < count_ext {
            matched_ext[ei + 1] = true;
        }

        tracing::trace!(
            gt_idx = gi,
            gt_type = %gt_blocks[gi].block_type,
            ext_idx = ei,
            ext_type = %ext_blocks[ei].block_type,
            content_sim = format!("{:.3}", content_sim),
            type_compat = format!("{:.2}", compat),
            match_score = format!("{:.3}", score),
            is_concat = is_concat,
            gt_preview = %truncate_preview(&gt_blocks[gi].content, 60),
            ext_preview = %truncate_preview(&ext_blocks[ei].content, 60),
            "MATCH"
        );

        results.push(BlockMatch {
            gt_idx: gi,
            ext_idx: ei,
            content_sim,
            type_compat: compat,
            match_score: score,
            is_concat,
        });
        order_pairs.push((gt_blocks[gi].index, ext_blocks[ei].index));
    }

    // Log unmatched GT blocks
    for (i, is_matched) in matched_gt.iter().enumerate() {
        if !is_matched {
            tracing::trace!(
                idx = i,
                block_type = %gt_blocks[i].block_type,
                preview = %truncate_preview(&gt_blocks[i].content, 60),
                "MISS_GT no match in extracted"
            );
        }
    }

    // Log unmatched extracted blocks
    for (i, is_matched) in matched_ext.iter().enumerate() {
        if !is_matched {
            tracing::trace!(
                idx = i,
                block_type = %ext_blocks[i].block_type,
                preview = %truncate_preview(&ext_blocks[i].content, 60),
                "MISS_EXT false positive"
            );
        }
    }

    (results, order_pairs)
}

// ---------------------------------------------------------------------------
// Weighted SF1 from global matches
// ---------------------------------------------------------------------------

/// Compute weighted structural F1 directly from global match results.
///
/// Each GT block contributes to recall weighted by its type importance.
/// Each ext block contributes to precision weighted by its type importance.
/// Match scores incorporate both content similarity and type compatibility.
fn compute_weighted_sf1_from_matches(gt_blocks: &[MdBlock], ext_blocks: &[MdBlock], matches: &[BlockMatch]) -> f64 {
    if gt_blocks.is_empty() && ext_blocks.is_empty() {
        return 1.0;
    }
    if gt_blocks.is_empty() || ext_blocks.is_empty() {
        return 0.0;
    }

    // Weighted recall: sum of (weight * match_score) for matched GT blocks
    // divided by sum of weights for ALL GT blocks.
    let total_gt_weight: f64 = gt_blocks.iter().map(|b| b.block_type.weight()).sum();
    let matched_gt_weight: f64 = matches
        .iter()
        .map(|m| gt_blocks[m.gt_idx].block_type.weight() * m.match_score)
        .sum();
    let recall = if total_gt_weight > 0.0 {
        matched_gt_weight / total_gt_weight
    } else {
        0.0
    };

    // Weighted precision: sum of (weight * match_score) for matched ext blocks
    // divided by sum of weights for ALL ext blocks.
    let total_ext_weight: f64 = ext_blocks.iter().map(|b| b.block_type.weight()).sum();
    let matched_ext_weight: f64 = matches
        .iter()
        .map(|m| ext_blocks[m.ext_idx].block_type.weight() * m.match_score)
        .sum();
    let precision = if total_ext_weight > 0.0 {
        matched_ext_weight / total_ext_weight
    } else {
        0.0
    };

    if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    }
}

// ---------------------------------------------------------------------------
// Per-type score derivation (diagnostic)
// ---------------------------------------------------------------------------

/// Derive per-type scores by grouping global matches by GT block type.
///
/// This is for diagnostic breakdown only — the main SF1 uses
/// `compute_weighted_sf1_from_matches` which handles cross-type matching
/// correctly.
fn derive_per_type_scores(
    gt_blocks: &[MdBlock],
    _ext_blocks: &[MdBlock],
    matches: &[BlockMatch],
) -> HashMap<MdBlockType, TypeScore> {
    // Collect all GT types present
    let mut gt_types: Vec<MdBlockType> = Vec::new();
    for b in gt_blocks {
        if !gt_types.contains(&b.block_type) {
            gt_types.push(b.block_type);
        }
    }

    let mut per_type: HashMap<MdBlockType, TypeScore> = HashMap::new();

    for &block_type in &gt_types {
        let count_gt = gt_blocks.iter().filter(|b| b.block_type == block_type).count();
        // Count ext blocks that matched GT blocks of this type
        let type_matches: Vec<&BlockMatch> = matches
            .iter()
            .filter(|m| gt_blocks[m.gt_idx].block_type == block_type)
            .collect();

        let sum_scores: f64 = type_matches.iter().map(|m| m.match_score).sum();
        let matched_count = type_matches.len();

        let recall = if count_gt > 0 {
            sum_scores / count_gt as f64
        } else {
            1.0
        };

        // For diagnostic precision, use matched_count as denominator
        let precision = if matched_count > 0 {
            sum_scores / matched_count as f64
        } else {
            0.0
        };

        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };

        tracing::debug!(
            block_type = %block_type,
            count_gt = count_gt,
            matched = matched_count,
            avg_score = format!("{:.3}", if matched_count > 0 { sum_scores / matched_count as f64 } else { 0.0 }),
            f1 = format!("{:.3}", f1),
            "per-type score"
        );

        per_type.insert(
            block_type,
            TypeScore {
                precision,
                recall,
                f1,
                count_extracted: matched_count,
                count_gt,
            },
        );
    }

    per_type
}

// ---------------------------------------------------------------------------
// Weighted F1 and order scoring (unchanged logic)
// ---------------------------------------------------------------------------

/// Compute reading order score using longest increasing subsequence.
fn compute_order_score(matches: &[(usize, usize)]) -> f64 {
    if matches.is_empty() {
        return 1.0;
    }

    let mut sorted: Vec<(usize, usize)> = matches.to_vec();
    sorted.sort_by_key(|m| m.0);

    let ext_indices: Vec<usize> = sorted.iter().map(|m| m.1).collect();
    let lis_len = longest_increasing_subsequence_length(&ext_indices);
    lis_len as f64 / matches.len() as f64
}

/// Compute the length of the longest increasing subsequence.
fn longest_increasing_subsequence_length(seq: &[usize]) -> usize {
    if seq.is_empty() {
        return 0;
    }

    // Patience sorting approach: O(n log n)
    let mut tails: Vec<usize> = Vec::new();
    for &val in seq {
        match tails.binary_search(&val) {
            Ok(_) => {}
            Err(pos) => {
                if pos == tails.len() {
                    tails.push(val);
                } else {
                    tails[pos] = val;
                }
            }
        }
    }
    tails.len()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading_levels() {
        let md = "# Title\n\n## Section\n\n### Subsection\n\nBody text.\n";
        let blocks = parse_markdown_blocks(md);
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0].block_type, MdBlockType::Heading1);
        assert_eq!(blocks[0].content, "Title");
        assert_eq!(blocks[1].block_type, MdBlockType::Heading2);
        assert_eq!(blocks[1].content, "Section");
        assert_eq!(blocks[2].block_type, MdBlockType::Heading3);
        assert_eq!(blocks[2].content, "Subsection");
        assert_eq!(blocks[3].block_type, MdBlockType::Paragraph);
    }

    #[test]
    fn test_parse_code_block() {
        let md = "Some text.\n\n```python\ndef hello():\n    print('hi')\n```\n\nMore text.\n";
        let blocks = parse_markdown_blocks(md);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].block_type, MdBlockType::Paragraph);
        assert_eq!(blocks[1].block_type, MdBlockType::CodeBlock);
        assert!(blocks[1].content.contains("def hello()"));
        assert_eq!(blocks[2].block_type, MdBlockType::Paragraph);
    }

    #[test]
    fn test_parse_formula() {
        let md = "Before.\n\n$$\nE = mc^2\n$$\n\nAfter.\n";
        let blocks = parse_markdown_blocks(md);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[1].block_type, MdBlockType::Formula);
        assert_eq!(blocks[1].content, "E = mc^2");
    }

    #[test]
    fn test_parse_table() {
        let md = "| Name | Age |\n|------|-----|\n| Alice | 30 |\n| Bob | 25 |\n";
        let blocks = parse_markdown_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, MdBlockType::Table);
        assert!(blocks[0].content.contains("Alice"));
    }

    #[test]
    fn test_parse_list_items() {
        let md = "- Item one\n- Item two\n- Item three\n";
        let blocks = parse_markdown_blocks(md);
        assert_eq!(blocks.len(), 3);
        assert!(blocks.iter().all(|b| b.block_type == MdBlockType::ListItem));
        assert_eq!(blocks[0].content, "Item one");
    }

    #[test]
    fn test_parse_numbered_list() {
        let md = "1. First\n2. Second\n";
        let blocks = parse_markdown_blocks(md);
        assert_eq!(blocks.len(), 2);
        assert!(blocks.iter().all(|b| b.block_type == MdBlockType::ListItem));
    }

    #[test]
    fn test_parse_image() {
        let md = "![Alt text](image.png)\n";
        let blocks = parse_markdown_blocks(md);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, MdBlockType::Image);
    }

    #[test]
    fn test_parse_paragraph_grouping() {
        let md = "Line one of a paragraph.\nLine two of the same paragraph.\n\nNew paragraph.\n";
        let blocks = parse_markdown_blocks(md);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].content.contains("Line one"));
        assert!(blocks[0].content.contains("Line two"));
    }

    #[test]
    fn test_identical_markdown() {
        let md = "# Title\n\nBody text here.\n\n## Section\n\nMore text.\n";
        let result = score_structural_quality(md, md);
        assert!((result.structural_f1 - 1.0).abs() < 0.01, "f1={}", result.structural_f1);
        assert!((result.order_score - 1.0).abs() < 0.01, "order={}", result.order_score);
        assert!((result.text_f1 - 1.0).abs() < 0.01, "text_f1={}", result.text_f1);
    }

    #[test]
    fn test_completely_different() {
        let extracted = "# Title\n\nSome content here.\n";
        let gt = "## Other\n\nDifferent content entirely.\n";
        let result = score_structural_quality(extracted, gt);
        assert!(result.structural_f1 < 0.5);
    }

    #[test]
    fn test_heading_level_off_by_one_gets_partial_credit() {
        let extracted = "## Title\n\nBody text here.\n";
        let gt = "# Title\n\nBody text here.\n";
        let result = score_structural_quality(extracted, gt);
        // H2 matching H1 should get 0.9 type compat * 1.0 content sim
        assert!(
            result.structural_f1 > 0.7,
            "expected >0.7 for off-by-1 heading, got {}",
            result.structural_f1
        );
    }

    #[test]
    fn test_heading_level_off_by_three() {
        let extracted = "#### Deep Section\n\nBody text.\n";
        let gt = "# Deep Section\n\nBody text.\n";
        let result = score_structural_quality(extracted, gt);
        // 0.7 type compat for off-by-3
        assert!(
            result.structural_f1 > 0.5,
            "expected >0.5 for off-by-3, got {}",
            result.structural_f1
        );
        assert!(
            result.structural_f1 < 0.95,
            "expected <0.95 for off-by-3, got {}",
            result.structural_f1
        );
    }

    #[test]
    fn test_bold_paragraph_as_pseudo_heading() {
        let extracted = "**Pricing**\n\nDetails about pricing here.\n";
        let gt = "## Pricing\n\nDetails about pricing here.\n";
        let result = score_structural_quality(extracted, gt);
        // Bold paragraph → heading gets 0.15 type compat
        assert!(
            result.structural_f1 > 0.0,
            "expected >0 for bold pseudo-heading, got {}",
            result.structural_f1
        );
    }

    #[test]
    fn test_code_formula_cross_match() {
        let extracted = "```\nE = mc^2\n```\n";
        let gt = "$$\nE = mc^2\n$$\n";
        let result = score_structural_quality(extracted, gt);
        // Code ↔ Formula gets 0.3 type compat
        assert!(
            result.structural_f1 > 0.1,
            "expected >0.1 for code/formula cross-match, got {}",
            result.structural_f1
        );
    }

    #[test]
    fn test_normalized_same_as_regular() {
        let md = "# Title\n\n## Section\n\nBody.\n";
        let r1 = score_structural_quality(md, md);
        let r2 = score_structural_quality_normalized(md, md);
        assert!(
            (r1.structural_f1 - r2.structural_f1).abs() < 0.01,
            "regular={} normalized={}",
            r1.structural_f1,
            r2.structural_f1
        );
    }

    #[test]
    fn test_lis_length() {
        assert_eq!(longest_increasing_subsequence_length(&[1, 3, 2, 4, 5]), 4);
        assert_eq!(longest_increasing_subsequence_length(&[5, 4, 3, 2, 1]), 1);
        assert_eq!(longest_increasing_subsequence_length(&[1, 2, 3, 4, 5]), 5);
        assert_eq!(longest_increasing_subsequence_length(&[]), 0);
    }

    #[test]
    fn test_order_score_perfect() {
        let matches = vec![(0, 0), (1, 1), (2, 2)];
        assert!((compute_order_score(&matches) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_order_score_reversed() {
        let matches = vec![(0, 2), (1, 1), (2, 0)];
        assert!((compute_order_score(&matches) - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_type_compat_heading_same_level() {
        let a = MdBlock {
            block_type: MdBlockType::Heading2,
            content: "X".into(),
            index: 0,
        };
        let b = MdBlock {
            block_type: MdBlockType::Heading2,
            content: "X".into(),
            index: 0,
        };
        assert!((type_compat(&a, &b) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_type_compat_heading_off_by_one() {
        let a = MdBlock {
            block_type: MdBlockType::Heading1,
            content: "X".into(),
            index: 0,
        };
        let b = MdBlock {
            block_type: MdBlockType::Heading2,
            content: "X".into(),
            index: 0,
        };
        assert!((type_compat(&a, &b) - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_type_compat_heading_off_by_five() {
        let a = MdBlock {
            block_type: MdBlockType::Heading1,
            content: "X".into(),
            index: 0,
        };
        let b = MdBlock {
            block_type: MdBlockType::Heading6,
            content: "X".into(),
            index: 0,
        };
        // Should be floor of 0.6
        assert!((type_compat(&a, &b) - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_type_compat_paragraph_heading_not_bold() {
        let a = MdBlock {
            block_type: MdBlockType::Paragraph,
            content: "Normal text".into(),
            index: 0,
        };
        let b = MdBlock {
            block_type: MdBlockType::Heading1,
            content: "X".into(),
            index: 0,
        };
        assert!((type_compat(&a, &b) - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_type_compat_paragraph_heading_bold() {
        let a = MdBlock {
            block_type: MdBlockType::Paragraph,
            content: "**Bold Title**".into(),
            index: 0,
        };
        let b = MdBlock {
            block_type: MdBlockType::Heading1,
            content: "X".into(),
            index: 0,
        };
        assert!((type_compat(&a, &b) - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_type_compat_table_paragraph() {
        let a = MdBlock {
            block_type: MdBlockType::Table,
            content: "X".into(),
            index: 0,
        };
        let b = MdBlock {
            block_type: MdBlockType::Paragraph,
            content: "X".into(),
            index: 0,
        };
        assert!((type_compat(&a, &b) - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_type_compat_incompatible() {
        let a = MdBlock {
            block_type: MdBlockType::Table,
            content: "X".into(),
            index: 0,
        };
        let b = MdBlock {
            block_type: MdBlockType::CodeBlock,
            content: "X".into(),
            index: 0,
        };
        assert!((type_compat(&a, &b)).abs() < 0.01);
    }
}
