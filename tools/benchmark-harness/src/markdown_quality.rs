//! Markdown block parsing and shared reading-order helpers.
//!
//! Canonical SF1 scoring lives in [`crate::quality::structural_sidecar`]. This
//! module only provides the CommonMark parser used by ground-truth validation
//! and the order helpers shared by the structural sidecar.

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
    pub(crate) fn is_heading(self) -> bool {
        matches!(
            self,
            Self::Heading1 | Self::Heading2 | Self::Heading3 | Self::Heading4 | Self::Heading5 | Self::Heading6
        )
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

/// Shared pulldown-cmark parse options for all markdown structural analysis.
///
/// Enables GFM tables, `$…$`/`$$…$$` math, and strikethrough. The structural
/// sidecar ([`crate::quality::structural_sidecar`]) derives its typed node list
/// with these *exact* options so its parse tree stays identical to
/// [`parse_markdown_blocks`].
pub(crate) fn md_parser_options() -> pulldown_cmark::Options {
    use pulldown_cmark::Options;
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_MATH);
    opts
}

/// Parse a markdown string into a sequence of typed blocks using pulldown-cmark.
///
/// This uses a proper CommonMark parser, so it correctly handles all markdown
/// variants: fenced and indented code blocks, ATX and setext headings, different
/// list markers (-, *, +, 1.), tables with any separator style, etc.
pub fn parse_markdown_blocks(md: &str) -> Vec<MdBlock> {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};

    let parser = Parser::new_ext(md, md_parser_options());
    let mut blocks: Vec<MdBlock> = Vec::new();
    let mut index = 0;
    let mut current_text = String::new();
    let mut in_heading: Option<u8> = None;
    let mut in_code_block = false;
    let mut in_table = false;
    let mut in_list_item = false;
    let mut table_content = String::new();

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
            }
            Event::End(TagEnd::TableRow) => {}
            Event::Start(Tag::TableCell) => {}
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
            Event::Start(Tag::Image { dest_url, .. }) => {
                flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
                current_text.push_str("![");
                let _ = dest_url;
            }
            Event::End(TagEnd::Image) if current_text.starts_with("![") => {
                current_text.push(']');
                blocks.push(MdBlock {
                    block_type: MdBlockType::Image,
                    content: std::mem::take(&mut current_text),
                    index,
                });
                index += 1;
            }
            Event::Start(Tag::Paragraph) if !in_list_item && !in_table => {
                flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
            }
            Event::End(TagEnd::Paragraph) if !in_list_item && !in_table => {
                flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);
            }
            Event::Start(Tag::Strong) if !in_table && !in_code_block => {
                current_text.push_str("**");
            }
            Event::End(TagEnd::Strong) if !in_table && !in_code_block => {
                current_text.push_str("**");
            }
            Event::Text(text) | Event::Code(text) => {
                if in_table {
                    current_text.push_str(&text);
                } else {
                    if !current_text.is_empty()
                        && !current_text.ends_with(' ')
                        && !current_text.ends_with('\n')
                        && !current_text.ends_with("**")
                    {
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
                let text = strip_html_tags(&html);
                if !text.is_empty() {
                    current_text.push_str(&text);
                }
            }
            _ => {}
        }
    }

    flush_text(&mut current_text, &mut blocks, &mut index, MdBlockType::Paragraph);

    blocks
}

/// Flush accumulated text into a block if non-empty.
fn flush_text(text: &mut String, blocks: &mut Vec<MdBlock>, index: &mut usize, block_type: MdBlockType) {
    let content = std::mem::take(text);
    let trimmed = content.trim();
    if !trimmed.is_empty() {
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
///
/// Deliberately conservative: a long prose paragraph that merely *contains* an inline
/// superscript (`x^{2}`) or begins with a formatting command (`\textbf{...}`, `\section*{...}`)
/// is NOT a formula. Misclassifying such prose as `Formula` used to score it 0 against its
/// plain-text ground-truth twin (Formula↔Paragraph had no compatibility), asymmetrically
/// penalizing whichever extractor's markup diverged from the GT.
fn looks_like_formula(content: &str) -> bool {
    // Strong LaTeX math commands — reliable formula signals.
    if content.contains("\\frac")
        || content.contains("\\sum")
        || content.contains("\\int")
        || content.contains("\\begin{")
        || content.contains("\\end{")
        || content.contains("\\left")
        || content.contains("\\right")
        || content.contains("\\sqrt")
        || content.contains("\\mathbb")
        || content.contains("\\mathcal")
    {
        return true;
    }
    // Weak signal: a `^{…}` superscript only counts when the block is short and math-dominant,
    // not an inline superscript embedded in a full sentence of prose. ~keep
    if content.contains("^{") && content.contains('}') {
        return content.split_whitespace().count() <= 6;
    }
    false
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

/// Compute reading order score using longest increasing subsequence.
/// Minimum SF1 multiplier when reading order is fully scrambled. The structural metric is defined
/// over block structure AND reading order (LIS); a perfectly-ordered document keeps its full score,
/// a fully-scrambled one is penalized by at most `1 - ORDER_SCORE_FLOOR`. Kept modest so ordering
/// refines, rather than dominates, the content-structure score.
pub(crate) const ORDER_SCORE_FLOOR: f64 = 0.8;

/// Fold the LIS reading-order score into the block-structure SF1. Skipped when fewer than three
/// blocks matched, since order is not meaningful for one or two blocks.
pub(crate) fn fold_order_into_sf1(base_sf1: f64, order_score: f64, matched_blocks: usize) -> f64 {
    if matched_blocks < 3 {
        return base_sf1;
    }
    base_sf1 * (ORDER_SCORE_FLOOR + (1.0 - ORDER_SCORE_FLOOR) * order_score)
}

pub(crate) fn compute_order_score(matches: &[(usize, usize)]) -> f64 {
    if matches.is_empty() {
        return 0.0;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_heading_levels() {
        let blocks = parse_markdown_blocks("# Title\n\n## Section\n\n### Subsection\n\nBody text.\n");
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0].block_type, MdBlockType::Heading1);
        assert_eq!(blocks[0].content, "Title");
        assert_eq!(blocks[1].block_type, MdBlockType::Heading2);
        assert_eq!(blocks[2].block_type, MdBlockType::Heading3);
        assert_eq!(blocks[3].block_type, MdBlockType::Paragraph);
    }

    #[test]
    fn parses_code_formula_table_lists_and_images() {
        let cases = [
            ("```rust\nfn main() {}\n```", MdBlockType::CodeBlock),
            ("$$\nE = mc^2\n$$", MdBlockType::Formula),
            ("| Name | Age |\n|---|---|\n| Alice | 30 |", MdBlockType::Table),
            ("- Item one", MdBlockType::ListItem),
            ("![Alt text](image.png)", MdBlockType::Image),
        ];
        for (markdown, expected) in cases {
            let blocks = parse_markdown_blocks(markdown);
            assert!(
                blocks.iter().any(|block| block.block_type == expected),
                "expected {expected} in {blocks:?}"
            );
        }
    }

    #[test]
    fn groups_paragraph_lines() {
        let blocks =
            parse_markdown_blocks("Line one of a paragraph.\nLine two of the same paragraph.\n\nNew paragraph.\n");
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].content.contains("Line one"));
        assert!(blocks[0].content.contains("Line two"));
    }

    #[test]
    fn preserves_bold_markers_in_paragraph_content() {
        let blocks = parse_markdown_blocks("**Pricing**\n\nDetails here.\n");
        let bold_block = blocks.iter().find(|block| block.content.contains("Pricing")).unwrap();
        assert_eq!(bold_block.content, "**Pricing**");
    }

    #[test]
    fn computes_longest_increasing_subsequence() {
        assert_eq!(longest_increasing_subsequence_length(&[1, 3, 2, 4, 5]), 4);
        assert_eq!(longest_increasing_subsequence_length(&[5, 4, 3, 2, 1]), 1);
        assert_eq!(longest_increasing_subsequence_length(&[1, 2, 3, 4, 5]), 5);
        assert_eq!(longest_increasing_subsequence_length(&[]), 0);
    }

    #[test]
    fn scores_reading_order() {
        assert!((compute_order_score(&[(0, 0), (1, 1), (2, 2)]) - 1.0).abs() < 0.01);
        assert!((compute_order_score(&[(0, 2), (1, 1), (2, 0)]) - 1.0 / 3.0).abs() < 0.01);
        assert_eq!(compute_order_score(&[]), 0.0);
    }

    #[test]
    fn skips_order_penalty_for_fewer_than_three_matches() {
        assert_eq!(fold_order_into_sf1(0.9, 0.0, 2), 0.9);
        assert_eq!(fold_order_into_sf1(0.9, 0.0, 1), 0.9);
        assert!((fold_order_into_sf1(1.0, 0.0, 5) - ORDER_SCORE_FLOOR).abs() < 1e-9);
        assert!((fold_order_into_sf1(1.0, 1.0, 5) - 1.0).abs() < 1e-9);
    }
}
