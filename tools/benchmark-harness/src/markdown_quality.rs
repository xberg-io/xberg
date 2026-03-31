//! Structural quality scoring for markdown extraction.
//!
//! Parses markdown into typed blocks (headings, paragraphs, code, formulas, etc.)
//! and computes structural F1 scores by matching extracted blocks against ground truth.
//!
//! # Block weights
//!
//! Each block type carries a weight that reflects how much it tells us about
//! layout detection quality:
//!
//! | Weight | Types | Rationale |
//! |--------|-------|-----------|
//! | 2.0 | Headings (H1-H6) | Layout detection is the primary differentiator for heading identification; getting headings right matters most. |
//! | 1.5 | Code, Formula, Table | These require format-specific handling (fence detection, LaTeX parsing, cell alignment) and are strong quality signals. |
//! | 1.0 | ListItem | Standard structural element; neither trivially extracted nor especially hard. |
//! | 0.5 | Paragraph, Image | Common and less discriminating -- most extractors get paragraphs right, so they contribute less to the score. |
//!
//! # Fuzzy cross-type matching
//!
//! Blocks can match across types with a continuous compatibility score in
//! `[0.0, 1.0]`. The rationale for the key cross-type scores:
//!
//! - **Heading level mismatch** (0.6-0.9): content is correct, only the level
//!   is wrong -- partial credit scaled by level distance.
//! - **Bold paragraph matching heading** (0.4): extractors that miss heading
//!   markup but preserve bold formatting deserve some credit since bold
//!   paragraphs are a plausible pseudo-heading representation.
//! - **Plain paragraph matching heading** (0.25): content preserved but heading
//!   detection failed entirely.
//! - **Code/Formula cross-match** (0.3): math content is frequently code-fenced
//!   in practice, so misclassification between these two is understandable.
//! - **ListItem/Paragraph** (0.5): structurally different but the content is
//!   essentially the same.
//!
//! # Greedy matching algorithm
//!
//! The matcher builds all candidate `(GT block, extracted block)` pairs,
//! scores each as `content_similarity * type_compatibility`, then greedily
//! assigns matches from highest score to lowest. This is an O(n*m) approach
//! where n and m are the block counts.
//!
//! An **O(n^2) complexity safeguard** at [`MAX_PAIRS_FOR_MATCHING`] (40,000
//! candidate pairs) prevents degenerate performance on very large documents.
//! When the safeguard triggers, scoring falls back to a count-ratio heuristic.

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

/// Diagnostic information about block matching for debugging poor scores.
#[derive(Debug)]
pub struct MatchDiagnostics {
    /// GT blocks that were not matched to any extracted block: (index, block).
    pub unmatched_gt: Vec<(usize, MdBlock)>,
    /// Extracted blocks that were not matched to any GT block: (index, block).
    pub unmatched_extracted: Vec<(usize, MdBlock)>,
    /// Cross-type matches: (gt_block, ext_block, content_similarity, type_compatibility).
    pub cross_type_matches: Vec<(MdBlock, MdBlock, f64, f64)>,
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

/// Parse a markdown string into a sequence of typed blocks using pulldown-cmark.
///
/// This uses a proper CommonMark parser, so it correctly handles all markdown
/// variants: fenced and indented code blocks, ATX and setext headings, different
/// list markers (-, *, +, 1.), tables with any separator style, etc.
pub fn parse_markdown_blocks(md: &str) -> Vec<MdBlock> {
    use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_MATH);

    let parser = Parser::new_ext(md, opts);
    let mut blocks: Vec<MdBlock> = Vec::new();
    let mut index = 0;
    let mut current_text = String::new();
    let mut in_heading: Option<u8> = None;
    let mut in_code_block = false;
    let mut in_table = false;
    let mut in_list_item = false;
    let mut table_content = String::new();
    let mut table_cell_idx = 0;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
                in_heading = Some(level as u8);
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(level) = in_heading.take() {
                    let block_type = match level {
                        1 => MdBlockType::Heading1,
                        2 => MdBlockType::Heading2,
                        3 => MdBlockType::Heading3,
                        4 => MdBlockType::Heading4,
                        5 => MdBlockType::Heading5,
                        _ => MdBlockType::Heading6,
                    };
                    let content = std::mem::take(&mut current_text);
                    if !content.trim().is_empty() {
                        blocks.push(MdBlock {
                            block_type,
                            content: content.trim().to_string(),
                            index,
                        });
                        index += 1;
                    }
                }
            }
            Event::Start(Tag::CodeBlock(_)) => {
                flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                let content = std::mem::take(&mut current_text);
                if !content.trim().is_empty() {
                    // Check if it's a math/formula block
                    let block_type = if content.trim().starts_with("\\")
                        || content.contains("\\frac")
                        || content.contains("\\sum")
                        || content.contains("\\int")
                    {
                        MdBlockType::Formula
                    } else {
                        MdBlockType::CodeBlock
                    };
                    blocks.push(MdBlock {
                        block_type,
                        content: content.trim_end().to_string(),
                        index,
                    });
                    index += 1;
                }
            }
            Event::Start(Tag::Table(_)) => {
                flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
                in_table = true;
                table_content.clear();
                table_cell_idx = 0;
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
                let content = std::mem::take(&mut table_content);
                if !content.trim().is_empty() {
                    blocks.push(MdBlock {
                        block_type: MdBlockType::Table,
                        content: content.trim().to_string(),
                        index,
                    });
                    index += 1;
                }
            }
            Event::Start(Tag::TableHead) => {}
            Event::End(TagEnd::TableHead) => {}
            Event::Start(Tag::TableRow) => {
                if !table_content.is_empty() {
                    table_content.push('\n');
                }
                table_content.push('|');
                table_cell_idx = 0;
            }
            Event::End(TagEnd::TableRow) => {
                // Row already ended with last cell's |
            }
            Event::Start(Tag::TableCell) => {
                if table_cell_idx > 0 {
                    // Cell separator already added by previous cell end
                }
                table_cell_idx += 1;
            }
            Event::End(TagEnd::TableCell) => {
                let cell_text = std::mem::take(&mut current_text);
                table_content.push(' ');
                table_content.push_str(cell_text.trim());
                table_content.push_str(" |");
            }
            Event::Start(Tag::List(_)) => {
                flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
            }
            Event::End(TagEnd::List(_)) => {}
            Event::Start(Tag::Item) => {
                flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
                in_list_item = true;
            }
            Event::End(TagEnd::Item) => {
                in_list_item = false;
                let content = std::mem::take(&mut current_text);
                if !content.trim().is_empty() {
                    blocks.push(MdBlock {
                        block_type: MdBlockType::ListItem,
                        content: content.trim().to_string(),
                        index,
                    });
                    index += 1;
                }
            }
            Event::Start(Tag::Image { dest_url, title: _, .. }) => {
                flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
                // Store URL temporarily
                current_text.push_str("![");
                let _ = dest_url; // alt text comes as Text events
            }
            Event::End(TagEnd::Image) => {
                // current_text has "![alt text" — close it
                if current_text.starts_with("![") {
                    current_text.push(']');
                    blocks.push(MdBlock {
                        block_type: MdBlockType::Image,
                        content: std::mem::take(&mut current_text),
                        index,
                    });
                    index += 1;
                }
            }
            Event::Start(Tag::Paragraph) => {
                if !in_list_item && !in_table {
                    flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if !in_list_item && !in_table {
                    flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if in_table {
                    current_text.push_str(&text);
                } else {
                    if !current_text.is_empty() && !current_text.ends_with(' ') && !current_text.ends_with('\n') {
                        current_text.push(' ');
                    }
                    current_text.push_str(&text);
                }
            }
            Event::SoftBreak => {
                if in_code_block {
                    current_text.push('\n');
                } else {
                    current_text.push(' ');
                }
            }
            Event::HardBreak => {
                current_text.push('\n');
            }
            Event::InlineMath(text) => {
                current_text.push_str(&text);
            }
            Event::DisplayMath(text) => {
                // Display math ($$...$$) is a formula block
                flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    blocks.push(MdBlock {
                        block_type: MdBlockType::Formula,
                        content: trimmed.to_string(),
                        index,
                    });
                    index += 1;
                }
            }
            Event::Html(html) => {
                // Extract text content from HTML blocks instead of skipping them.
                // This handles cases where GT or extraction contains <b>, <table>,
                // <p>, etc. instead of markdown equivalents.
                let text = strip_html_tags(&html);
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
                    blocks.push(MdBlock {
                        block_type: MdBlockType::Paragraph,
                        content: trimmed.to_string(),
                        index,
                    });
                    index += 1;
                }
            }
            Event::InlineHtml(html) => {
                // Strip HTML tags but keep text content for inline HTML.
                // e.g., <b>bold</b> → "bold", <br> → ""
                let text = strip_html_tags(&html);
                if !text.is_empty() {
                    current_text.push_str(&text);
                }
            }
            _ => {}
        }
    }

    // Flush any remaining text
    flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);

    blocks
}

/// Flush accumulated text into a block if non-empty.
fn flush_text(text: &mut String, blocks: &mut Vec<MdBlock>, index: &mut usize, block_type: MdBlockType) {
    let content = std::mem::take(text);
    let trimmed = content.trim();
    if !trimmed.is_empty() {
        // If the block is a paragraph but looks like a math formula, classify as Formula.
        let actual_type = if block_type == MdBlockType::Paragraph && looks_like_formula(trimmed) {
            MdBlockType::Formula
        } else {
            block_type
        };
        blocks.push(MdBlock {
            block_type: actual_type,
            content: trimmed.to_string(),
            index: *index,
        });
        *index += 1;
    }
}

/// Check if content looks like a math/LaTeX formula.
fn looks_like_formula(content: &str) -> bool {
    content.contains("\\frac")
        || content.contains("\\sum")
        || content.contains("\\int")
        || content.contains("\\begin{")
        || content.contains("\\end{")
        || content.contains("\\left")
        || content.contains("\\right")
        || content.contains("\\sqrt")
        || content.contains("\\mathbb")
        || content.contains("\\mathcal")
        || (content.starts_with("\\") && content.len() > 2)
        || (content.contains("^{") && content.contains("}"))
}

// Old manual parsing helpers removed — pulldown-cmark handles everything.

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

    // Heading ↔ Paragraph: symmetric partial credit — wrong type but content preserved
    if ext.is_heading() && gt == MdBlockType::Paragraph {
        return 0.25;
    }
    if ext == MdBlockType::Paragraph && gt.is_heading() {
        if is_bold_wrapped(&ext_block.content) {
            return 0.4; // bold paragraph is a plausible heading
        }
        return 0.25; // missed heading detection, but content is there
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

    // Image ↔ Paragraph: image references sometimes rendered as paragraph text
    if (ext == MdBlockType::Image && gt == MdBlockType::Paragraph)
        || (ext == MdBlockType::Paragraph && gt == MdBlockType::Image)
    {
        return 0.5;
    }

    // Table ↔ ListItem: table content sometimes extracted as list items
    if (ext == MdBlockType::Table && gt == MdBlockType::ListItem)
        || (ext == MdBlockType::ListItem && gt == MdBlockType::Table)
    {
        return 0.3;
    }

    // Everything else cross-category: incompatible
    0.0
}

/// Strip HTML tags from a string, preserving text content.
///
/// Handles common HTML formatting tags that appear in pandoc output or
/// ground truth. Converts `<br>` and `<br/>` to spaces.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut tag_name = String::new();

    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
            tag_name.clear();
        } else if ch == '>' && in_tag {
            in_tag = false;
            // Convert <br> / <br/> to space
            let lower = tag_name.to_lowercase();
            if lower == "br" || lower == "br/" || lower == "/br" {
                result.push(' ');
            }
        } else if in_tag {
            tag_name.push(ch);
        } else {
            result.push(ch);
        }
    }
    result
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

/// Compute structural quality with diagnostic information about unmatched and cross-type matched blocks.
///
/// Returns both the standard `StructuralQuality` scores and a `MatchDiagnostics` with details
/// about which blocks were unmatched or matched across types.
pub fn score_structural_quality_diagnostic(
    extracted_md: &str,
    ground_truth_md: &str,
) -> (StructuralQuality, MatchDiagnostics) {
    let ext_blocks = parse_markdown_blocks(extracted_md);
    let gt_blocks = parse_markdown_blocks(ground_truth_md);

    let (match_results, all_matches) = match_blocks_global(&gt_blocks, &ext_blocks);

    let structural_f1 = compute_weighted_sf1_from_matches(&gt_blocks, &ext_blocks, &match_results);
    let per_type = derive_per_type_scores(&gt_blocks, &ext_blocks, &match_results);
    let order_score = compute_order_score(&all_matches);

    let ext_tokens = tokenize(extracted_md);
    let gt_tokens = tokenize(ground_truth_md);
    let text_f1 = crate::quality::compute_f1(&ext_tokens, &gt_tokens);

    // Build sets of matched indices
    let matched_gt_indices: std::collections::HashSet<usize> = match_results.iter().map(|m| m.gt_idx).collect();
    let mut matched_ext_indices: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for m in &match_results {
        matched_ext_indices.insert(m.ext_idx);
        if m.is_concat && m.ext_idx + 1 < ext_blocks.len() {
            matched_ext_indices.insert(m.ext_idx + 1);
        }
    }

    // Collect unmatched blocks
    let unmatched_gt: Vec<(usize, MdBlock)> = gt_blocks
        .iter()
        .enumerate()
        .filter(|(i, _)| !matched_gt_indices.contains(i))
        .map(|(i, b)| (i, b.clone()))
        .collect();

    let unmatched_extracted: Vec<(usize, MdBlock)> = ext_blocks
        .iter()
        .enumerate()
        .filter(|(i, _)| !matched_ext_indices.contains(i))
        .map(|(i, b)| (i, b.clone()))
        .collect();

    // Collect cross-type matches (where gt type != ext type)
    let cross_type_matches: Vec<(MdBlock, MdBlock, f64, f64)> = match_results
        .iter()
        .filter(|m| gt_blocks[m.gt_idx].block_type != ext_blocks[m.ext_idx].block_type)
        .map(|m| {
            (
                gt_blocks[m.gt_idx].clone(),
                ext_blocks[m.ext_idx].clone(),
                m.content_sim,
                m.type_compat,
            )
        })
        .collect();

    let quality = StructuralQuality {
        structural_f1,
        per_type,
        order_score,
        text_f1,
    };

    let diagnostics = MatchDiagnostics {
        unmatched_gt,
        unmatched_extracted,
        cross_type_matches,
    };

    (quality, diagnostics)
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

    // Complexity safeguard: use windowed matching for very large documents.
    // Instead of comparing all O(n*m) pairs, we use a diagonal band approach:
    // each GT block is only compared against nearby extracted blocks within a
    // proportional window. This preserves reasonable matching quality while
    // keeping computation bounded.
    if count_gt * count_ext > MAX_PAIRS_FOR_MATCHING {
        tracing::debug!(
            gt = count_gt,
            ext = count_ext,
            "block count too large, using windowed matching fallback"
        );
        return match_blocks_windowed(gt_blocks, ext_blocks);
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

/// Windowed matching fallback for large documents that exceed [`MAX_PAIRS_FOR_MATCHING`].
///
/// Instead of comparing every GT block against every extracted block (O(n*m)),
/// each GT block is compared only against extracted blocks within a proportional
/// window around its expected position. The window size is chosen so the total
/// number of comparisons stays under `MAX_PAIRS_FOR_MATCHING`.
///
/// This preserves reasonable matching quality because document order is roughly
/// preserved between extraction and ground truth, so the best match for GT block
/// `i` is almost always near position `i * (ext_len / gt_len)`.
fn match_blocks_windowed(gt_blocks: &[MdBlock], ext_blocks: &[MdBlock]) -> (Vec<BlockMatch>, Vec<(usize, usize)>) {
    let count_gt = gt_blocks.len();
    let count_ext = ext_blocks.len();

    // Window half-size: ensure total pairs <= MAX_PAIRS_FOR_MATCHING
    // Total pairs = count_gt * (2 * half_window + 1), solve for half_window:
    let half_window = if count_gt > 0 {
        ((MAX_PAIRS_FOR_MATCHING / count_gt).max(1) - 1) / 2
    } else {
        0
    };
    // Ensure minimum window of 10 blocks on each side for quality
    let half_window = half_window.max(10);

    tracing::debug!(
        half_window = half_window,
        estimated_pairs = count_gt * (2 * half_window + 1).min(count_ext),
        "windowed matching parameters"
    );

    // Pre-tokenize all blocks
    let gt_tokens: Vec<Vec<String>> = gt_blocks.iter().map(|b| tokenize(&b.content)).collect();
    let ext_tokens: Vec<Vec<String>> = ext_blocks.iter().map(|b| tokenize(&b.content)).collect();

    let ratio = count_ext as f64 / count_gt as f64;

    let mut candidates: Vec<(usize, usize, f64, f64, f64, bool)> = Vec::new();

    for gi in 0..count_gt {
        // Expected position of this GT block in extracted output
        let center = (gi as f64 * ratio) as usize;
        let start = center.saturating_sub(half_window);
        let end = (center + half_window + 1).min(count_ext);

        for ei in start..end {
            let compat = type_compat(&ext_blocks[ei], &gt_blocks[gi]);
            if compat <= 0.0 {
                continue;
            }

            let content_sim = crate::quality::compute_f1(&ext_tokens[ei], &gt_tokens[gi]);
            let score = content_sim * compat;
            if score >= 0.10 {
                candidates.push((gi, ei, content_sim, compat, score, false));
            }

            // Adjacent concatenation
            if ei + 1 < count_ext && ei + 1 < end + 1 {
                let concat_compat = type_compat(&ext_blocks[ei], &gt_blocks[gi]);
                if concat_compat <= 0.0 {
                    continue;
                }
                let mut concat_tokens = ext_tokens[ei].clone();
                concat_tokens.extend(ext_tokens[ei + 1].iter().cloned());
                let concat_sim = crate::quality::compute_f1(&concat_tokens, &gt_tokens[gi]);
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

    tracing::debug!(
        matched = results.len(),
        total_gt = count_gt,
        total_ext = count_ext,
        "windowed matching complete"
    );

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
        assert!((type_compat(&a, &b) - 0.25).abs() < 0.01);
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

    #[test]
    fn test_flat_paragraphs_vs_headings_gives_partial_credit() {
        // Simulates baseline pipeline: extracted has flat paragraphs,
        // GT has headings + paragraphs. SF1 should be >0 because
        // paragraph content matching heading content gets 0.25 type compat.
        let extracted = "Introduction\n\nThis is the first section content.\n\nMethods\n\nThis describes the methods used.\n\nResults\n\nHere are the results.\n";
        let gt = "# Introduction\n\nThis is the first section content.\n\n## Methods\n\nThis describes the methods used.\n\n## Results\n\nHere are the results.\n";
        let result = score_structural_quality(extracted, gt);
        // Paragraphs matching headings (0.25 compat) + exact paragraph matches (1.0 compat)
        // should give meaningful partial credit, not 0%
        assert!(
            result.structural_f1 > 0.15,
            "expected >0.15 for flat paragraphs vs headings, got {}",
            result.structural_f1
        );
        // But should be less than perfect since heading structure is missing
        assert!(
            result.structural_f1 < 0.85,
            "expected <0.85 for flat paragraphs vs headings, got {}",
            result.structural_f1
        );
    }

    #[test]
    fn test_zero_headings_extracted_many_headings_gt() {
        // All content as paragraphs, GT has many headings but same text
        let extracted = "Title\n\nSection One\n\nContent of section one.\n\nSection Two\n\nContent of section two.\n";
        let gt = "# Title\n\n## Section One\n\nContent of section one.\n\n## Section Two\n\nContent of section two.\n";
        let result = score_structural_quality(extracted, gt);
        // Should NOT be 0% — paragraphs match headings with 0.25 compat
        assert!(
            result.structural_f1 > 0.0,
            "SF1 should not be 0% when all text content matches but headings are missing"
        );
    }

    #[test]
    fn test_large_document_windowed_fallback() {
        // Generate a document large enough to trigger the windowed fallback
        // MAX_PAIRS_FOR_MATCHING = 40,000, so 201 * 201 = 40,401 > 40,000
        let mut gt_md = String::new();
        let mut ext_md = String::new();
        for i in 0..201 {
            gt_md.push_str(&format!(
                "## Section {i}\n\nContent for section number {i} with some words.\n\n"
            ));
            ext_md.push_str(&format!(
                "## Section {i}\n\nContent for section number {i} with some words.\n\n"
            ));
        }
        let result = score_structural_quality(&ext_md, &gt_md);
        // Identical documents should score high even with windowed fallback
        assert!(
            result.structural_f1 > 0.8,
            "windowed fallback should score >0.8 for identical large documents, got {}",
            result.structural_f1
        );
    }

    #[test]
    fn test_large_document_windowed_fallback_not_zero() {
        // Large document with similar but not identical content
        // Should NOT return 0% like the old fallback did
        let mut gt_md = String::new();
        let mut ext_md = String::new();
        for i in 0..201 {
            gt_md.push_str(&format!("# Heading {i}\n\nParagraph content {i}.\n\n"));
            // Extracted has paragraphs instead of headings (flat extraction)
            ext_md.push_str(&format!("Heading {i}\n\nParagraph content {i}.\n\n"));
        }
        let result = score_structural_quality(&ext_md, &gt_md);
        // Must be greater than 0 — the old code returned 0% for large docs
        assert!(
            result.structural_f1 > 0.0,
            "windowed fallback must not return SF1=0% for large documents with matching content, got {}",
            result.structural_f1
        );
    }

    #[test]
    fn test_empty_extracted_vs_nonempty_gt() {
        let gt = "# Title\n\nSome content.\n";
        let result = score_structural_quality("", gt);
        assert!(
            result.structural_f1 < 0.01,
            "empty extraction should score ~0%, got {}",
            result.structural_f1
        );
    }

    #[test]
    fn test_both_empty() {
        let result = score_structural_quality("", "");
        assert!(
            (result.structural_f1 - 1.0).abs() < 0.01,
            "both empty should score 100%, got {}",
            result.structural_f1
        );
    }
}
