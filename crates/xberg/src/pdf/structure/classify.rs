//! Heading classification for paragraphs using font-size clustering.

// TODO(xberg-io/xberg#1567): 4 cyclomatic-complexity and 12 size/complexity findings
// in this file, currently excluded via the quality-debt baseline in alef.toml. Splitting
// these needs compiler-in-the-loop verification, not a mechanical pass. Delete this
// note and the file's baseline entry together once it goes green. Help wanted.

use super::constants::{
    MAX_BOLD_HEADING_WORD_COUNT, MAX_HEADING_DISTANCE_MULTIPLIER, MAX_HEADING_WORD_COUNT, MIN_BLOCKS_FOR_FONT_HEADING,
    MIN_HEADING_FONT_GAP, MIN_HEADING_FONT_RATIO,
};
use super::regions::{looks_like_bare_url, looks_like_figure_label};
use super::types::{LayoutHintClass, PdfParagraph};

const SAL_DIRECTION_PREFIXES: [&str; 6] = [
    "__deref__inout",
    "__deref__out",
    "__deref__in",
    "__inout",
    "__out",
    "__in",
];
const SAL_DIRECTION_MODIFIERS: [&str; 7] = ["opt", "ecount", "bcount", "full", "part", "z", "nz"];
const CHANGELOG_VERSION_MAX_LEFT_OFFSET_RATIO: f32 = 0.75;
const CHANGELOG_VERSION_MAX_VERTICAL_GAP_RATIO: f32 = 2.0;
const CHANGELOG_VERSION_MAX_VERTICAL_OVERLAP_RATIO: f32 = 0.25;
const CHANGELOG_COMBINED_MIN_HEIGHT_RATIO: f32 = 1.5;
const CHANGELOG_COMBINED_MIN_BASELINE_GAP_RATIO: f32 = 0.5;
const SPARSE_PEER_HEADING_MIN_PAGES: usize = 2;
const SPARSE_PEER_HEADING_FONT_TOLERANCE: f32 = 0.5;
type ParagraphBbox = (f32, f32, f32, f32);

/// Demote structure-tree heading tags attached to standalone SAL annotations.
///
/// Tagged API references sometimes expose direction annotations such as `__in`
/// as headings. These are parameter metadata, not document structure. Keep the
/// guard deliberately narrow, and retain headings backed by a layout heading
/// class or code-block classification.
pub(super) fn demote_structure_annotation_headings(paragraphs: &mut [PdfParagraph]) {
    for para in paragraphs {
        if para.heading_level.is_none()
            || para.is_code_block
            || matches!(
                para.layout_class,
                Some(LayoutHintClass::Title | LayoutHintClass::SectionHeader)
            )
        {
            continue;
        }

        let text = paragraph_plain_text(para);
        if is_sal_direction_annotation(text.trim()) {
            para.heading_level = None;
        }
    }
}

fn is_sal_direction_annotation(text: &str) -> bool {
    let base = if let Some((base, arguments)) = text.split_once('(') {
        let Some(arguments) = arguments.strip_suffix(')') else {
            return false;
        };
        if arguments.is_empty()
            || !arguments
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ','))
        {
            return false;
        }
        base
    } else {
        text
    };

    SAL_DIRECTION_PREFIXES.iter().any(|prefix| {
        let Some(suffix) = base.strip_prefix(prefix) else {
            return false;
        };
        suffix.is_empty()
            || suffix.strip_prefix('_').is_some_and(|modifiers| {
                !modifiers.is_empty()
                    && modifiers
                        .split('_')
                        .all(|modifier| SAL_DIRECTION_MODIFIERS.contains(&modifier))
            })
    })
}

/// Classify paragraphs as headings or body using the global heading map and bold heuristic.
pub(super) fn classify_paragraphs(paragraphs: &mut [PdfParagraph], heading_map: &[(f32, Option<u8>)]) {
    tracing::debug!(
        paragraph_count = paragraphs.len(),
        heading_clusters = heading_map.len(),
        body_font = heading_map
            .iter()
            .find(|(_, l)| l.is_none())
            .map(|(c, _)| *c)
            .unwrap_or(0.0),
        "classify_paragraphs: start"
    );
    let gap_info = precompute_gap_info(heading_map);
    let body_font_size = heading_map
        .iter()
        .find(|(_, level)| level.is_none())
        .map(|(centroid, _)| *centroid)
        .unwrap_or(0.0);
    let assigned_heading_levels = paragraphs
        .iter()
        .map(|paragraph| paragraph.heading_level)
        .collect::<Vec<_>>();
    for (para, assigned_heading_level) in paragraphs.iter_mut().zip(&assigned_heading_levels) {
        if assigned_heading_level.is_some() {
            continue;
        }
        let word_count = para.word_count;

        let layout_says_text = para.layout_class == Some(super::types::LayoutHintClass::Text);
        let heading_level = find_heading_level(para.dominant_font_size, heading_map, &gap_info);
        let heading_level = if layout_says_text {
            if para.is_bold && heading_level.is_some() {
                heading_level
            } else {
                None
            }
        } else {
            heading_level
        };

        let para_text: String = if !para.text.is_empty() {
            para.text.clone()
        } else {
            para.lines
                .iter()
                .flat_map(|l| l.segments.iter())
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        };

        if let Some(level) = heading_level
            && word_count <= MAX_HEADING_WORD_COUNT
            && !super::layout_classify::is_separator_text(&para_text)
            && !looks_like_bare_url(&para_text)
        {
            para.heading_level = Some(level);
            continue;
        }

        let is_italic = if !para.text.is_empty() {
            para.lines
                .first()
                .and_then(|l| l.segments.first())
                .is_some_and(|s| s.is_italic)
        } else {
            !para.lines.is_empty() && para.lines.iter().all(|l| l.segments.iter().all(|s| s.is_italic))
        };
        let layout_text_overridable = if layout_says_text {
            body_font_size > 0.0 && para.dominant_font_size > body_font_size + 1.0
        } else {
            true
        };
        if (para.is_bold || is_italic)
            && !para.is_list_item
            && layout_text_overridable
            && word_count <= MAX_BOLD_HEADING_WORD_COUNT
        {
            let t = para_text.trim();
            let italic_ok = if is_italic && !para.is_bold {
                !t.contains('@') && !t.contains(',') && t.chars().next().is_some_and(|c| c.is_uppercase())
            } else {
                true
            };
            let too_short_at_body =
                word_count <= 2 && body_font_size > 0.0 && para.dominant_font_size <= body_font_size + 0.5;
            let period_ok = !ends_with_sentence_period(t) || is_section_pattern(t);
            let colon_ok = !t.ends_with(':') || is_all_caps_text(t);
            if italic_ok
                && !too_short_at_body
                && period_ok
                && colon_ok
                && !looks_like_figure_label(t)
                && !looks_like_bare_url(t)
                && !super::layout_classify::is_separator_text(t)
            {
                let level = infer_bold_heading_level(para.dominant_font_size, body_font_size, t);
                para.heading_level = Some(level);
            }
        }

        if para.heading_level.is_none()
            && !para.is_list_item
            && !para.is_code_block
            && (2..=MAX_HEADING_WORD_COUNT).contains(&word_count)
        {
            let t = para_text.trim();
            if starts_with_section_number(t)
                && !t.ends_with(':')
                && !looks_like_figure_label(t)
                && !super::layout_classify::is_separator_text(t)
            {
                let at_or_above_body = body_font_size <= 0.0 || para.dominant_font_size >= body_font_size - 0.5;
                let layout_ok = !layout_says_text
                    || (body_font_size > 0.0 && para.dominant_font_size > body_font_size + 1.0)
                    || para.is_bold;
                if at_or_above_body && layout_ok {
                    let level = infer_section_level(t);
                    para.heading_level = Some(level);
                }
            }
        }

        if para.is_code_block {
            para.heading_level = None;
        }

        if para.heading_level.is_none()
            && !para.is_list_item
            && !para.is_code_block
            && !para.is_formula
            && word_count <= 30
        {
            let math_char_count = para_text.chars().filter(|c| is_math_character(*c)).count();
            let total_chars = para_text.chars().count();
            if total_chars > 0 && (math_char_count >= 3 || (math_char_count as f64 / total_chars as f64) >= 0.15) {
                para.is_formula = true;
                para.heading_level = None;
            }
        }

        if para.heading_level.is_none()
            && !para.is_list_item
            && !para.is_code_block
            && !para.is_page_furniture
            && body_font_size > 0.0
            && para.dominant_font_size >= body_font_size + 1.0
        {
            let rescue_text = para_text.trim();
            let rescue_wc = rescue_text.split_whitespace().count();
            let rescue_colon_ok = !rescue_text.ends_with(':') || is_all_caps_text(rescue_text);
            if (1..=8).contains(&rescue_wc)
                && !ends_with_sentence_period(rescue_text)
                && rescue_colon_ok
                && !looks_like_figure_label(rescue_text)
                && !super::layout_classify::is_separator_text(rescue_text)
                && !starts_with_lowercase_or_continuation(rescue_text)
            {
                let ratio = para.dominant_font_size / body_font_size;
                let rescue_level = if ratio > 1.6 {
                    1
                } else if ratio > 1.3 {
                    2
                } else {
                    3
                };
                para.heading_level = Some(rescue_level);
            }
        }
    }

    demote_continuation_headings(paragraphs);
    for (paragraph, assigned_heading_level) in paragraphs.iter_mut().zip(assigned_heading_levels) {
        if assigned_heading_level.is_some() {
            paragraph.heading_level = assigned_heading_level;
        }
    }

    for para in paragraphs.iter_mut() {
        if para.heading_level.is_some()
            || para.is_list_item
            || para.is_code_block
            || para.is_formula
            || para.is_page_furniture
        {
            continue;
        }

        let text: String = if !para.text.is_empty() {
            para.text.clone()
        } else {
            para.lines
                .iter()
                .flat_map(|l| l.segments.iter())
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        };
        let t = text.trim();
        let wc = t.split_whitespace().count();
        if wc == 0 || wc > 15 {
            continue;
        }

        if wc == 1 && !t.ends_with(':') {
            continue;
        }

        if !is_all_caps_text(t) {
            continue;
        }

        if looks_like_figure_label(t) || super::layout_classify::is_separator_text(t) {
            continue;
        }

        let level = if body_font_size > 0.0 {
            let ratio = para.dominant_font_size / body_font_size;
            if ratio > 1.4 {
                1
            } else if ratio > 1.2 || wc <= 5 {
                2
            } else {
                3
            }
        } else if wc <= 5 {
            2
        } else {
            3
        };
        para.heading_level = Some(level);
    }

    detect_indentation_based_lists(paragraphs);

    detect_monospace_code_blocks(paragraphs);
}

/// W2.B: Detect indentation-based lists (unordered by indent level).
///
/// Paragraphs that are horizontally shifted right relative to surrounding paragraphs
/// are often list items. This pass identifies them by:
/// 1. Computing the modal left X across all paragraphs on each page
/// 2. Finding paragraphs indented >= 12 points right of the modal
/// 3. Requiring they follow or precede another short paragraph at the same indent
fn detect_indentation_based_lists(paragraphs: &mut [PdfParagraph]) {
    if paragraphs.len() < 2 {
        return;
    }

    const INDENT_THRESHOLD: f32 = 12.0;

    let mut left_positions: Vec<f32> = paragraphs
        .iter()
        .filter_map(|p| p.block_bbox.map(|(left, _, _, _)| left))
        .collect();

    if left_positions.is_empty() {
        return;
    }

    left_positions.sort_by(|a, b| a.total_cmp(b));
    let modal_left = {
        let mut freq_map: std::collections::HashMap<i32, usize> = std::collections::HashMap::new();
        for &pos in &left_positions {
            let bucket = (pos * 10.0) as i32;
            *freq_map.entry(bucket).or_default() += 1;
        }
        let (bucket, _) = freq_map.into_iter().max_by_key(|(_, count)| *count).unwrap_or((0, 0));
        bucket as f32 / 10.0
    };

    for i in 0..paragraphs.len() {
        let para = &paragraphs[i];

        if para.is_list_item || para.is_code_block || para.is_formula || para.heading_level.is_some() {
            continue;
        }

        let left_x = para.block_bbox.map(|(left, _, _, _)| left).unwrap_or(0.0);
        let is_indented = left_x >= modal_left + INDENT_THRESHOLD;

        if !is_indented {
            continue;
        }

        let para_text = if !para.text.is_empty() {
            para.text.clone()
        } else {
            para.lines
                .iter()
                .flat_map(|l| l.segments.iter())
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        };
        let word_count = para_text.split_whitespace().count();

        if super::pipeline::is_probable_author_byline(&para_text) || is_numeric_prose_continuation(&para_text) {
            continue;
        }

        let has_context = if i > 0 {
            let prev_text = if !paragraphs[i - 1].text.is_empty() {
                paragraphs[i - 1].text.clone()
            } else {
                paragraphs[i - 1]
                    .lines
                    .iter()
                    .flat_map(|l| l.segments.iter())
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let prev_wc = prev_text.split_whitespace().count();
            prev_wc <= 20
        } else {
            false
        };

        let followed_by_context = if i + 1 < paragraphs.len() {
            let next_text = if !paragraphs[i + 1].text.is_empty() {
                paragraphs[i + 1].text.clone()
            } else {
                paragraphs[i + 1]
                    .lines
                    .iter()
                    .flat_map(|l| l.segments.iter())
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let next_wc = next_text.split_whitespace().count();
            next_wc <= 20
        } else {
            false
        };

        if (word_count <= 30) && (has_context || followed_by_context) {
            paragraphs[i].is_list_item = true;
            paragraphs[i].layout_class = Some(LayoutHintClass::ListItem);
        }
    }
}

fn is_numeric_prose_continuation(text: &str) -> bool {
    let mut words = text.split_whitespace();
    let Some(first) = words.next() else {
        return false;
    };
    if first.is_empty() || !first.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    words
        .next()
        .is_none_or(|word| word.chars().next().is_some_and(char::is_lowercase))
}

/// W2.C: Detect monospace code-block sequences.
///
/// Multiple consecutive paragraphs where all lines are monospace should be
/// marked as code blocks. This handles code snippets that don't have explicit
/// code block markers.
fn detect_monospace_code_blocks(paragraphs: &mut [PdfParagraph]) {
    if paragraphs.is_empty() {
        return;
    }

    let mut i = 0;
    while i < paragraphs.len() {
        let para = &paragraphs[i];

        if para.is_code_block || para.is_list_item || para.is_formula || para.heading_level.is_some() {
            i += 1;
            continue;
        }

        let is_all_monospace = !para.lines.is_empty() && para.lines.iter().all(|l| l.is_monospace);

        if !is_all_monospace {
            i += 1;
            continue;
        }

        // A lone paragraph that already carries two or more monospace lines is a
        // complete multi-line code listing by itself — it does not need a consecutive
        // monospace neighbor to qualify, unlike the one-monospace-line-per-paragraph
        // case merged below (common when code line-leading splits each line into its
        // own paragraph). This purely font-based signal cannot distinguish a genuine
        // code listing from a document set entirely in a monospace face, or from a
        // 2-line caption/table cell that happens to share that font — both are
        // accepted, pre-existing limitations of this heuristic (unchanged by this
        // addition, which only mirrors pipeline.rs's identical paragraph-level gate),
        // not something overlooked here. ~keep
        if para.lines.len() >= 2 {
            paragraphs[i].is_code_block = true;
            paragraphs[i].layout_class = Some(LayoutHintClass::Code);
            i += 1;
            continue;
        }

        if i + 1 < paragraphs.len() {
            let next_para = &paragraphs[i + 1];
            let next_is_monospace = !next_para.lines.is_empty()
                && next_para.lines.iter().all(|l| l.is_monospace)
                && !next_para.is_list_item
                && !next_para.is_formula
                && next_para.heading_level.is_none();

            if next_is_monospace {
                paragraphs[i].is_code_block = true;
                paragraphs[i].layout_class = Some(LayoutHintClass::Code);
                paragraphs[i + 1].is_code_block = true;
                paragraphs[i + 1].layout_class = Some(LayoutHintClass::Code);

                let mut j = i + 2;
                while j < paragraphs.len() {
                    let para_j = &paragraphs[j];
                    let is_monospace_j = !para_j.lines.is_empty()
                        && para_j.lines.iter().all(|l| l.is_monospace)
                        && !para_j.is_list_item
                        && !para_j.is_formula
                        && para_j.heading_level.is_none();

                    if is_monospace_j {
                        paragraphs[j].is_code_block = true;
                        paragraphs[j].layout_class = Some(LayoutHintClass::Code);
                        j += 1;
                    } else {
                        break;
                    }
                }

                i = j;
                continue;
            }
        }

        i += 1;
    }
}

/// Check if text starts with a lowercase letter or a common sentence-continuation word.
///
/// Mid-sentence fragments from column breaks often start with lowercase words or
/// common continuation words. Real headings typically start with uppercase letters,
/// numbers, or section markers.
pub(super) fn starts_with_lowercase_or_continuation(text: &str) -> bool {
    let first_char = text.chars().next();
    if first_char.is_some_and(|c| c.is_lowercase()) {
        return true;
    }

    let first_word = text.split_whitespace().next().unwrap_or("");
    let lower = first_word.to_lowercase();
    matches!(
        lower.as_str(),
        "is" | "are"
            | "was"
            | "were"
            | "to"
            | "of"
            | "in"
            | "on"
            | "for"
            | "and"
            | "or"
            | "but"
            | "the"
            | "a"
            | "an"
            | "that"
            | "which"
            | "with"
    )
}

/// Check if text is ALL-CAPS (>80% of alphabetic characters are uppercase).
///
/// Used to identify label-headings like "AGENCY:", "DEPARTMENT OF TRANSPORTATION",
/// "SUMMARY:" that are common in government and legal documents.
fn is_all_caps_text(text: &str) -> bool {
    let alpha_chars: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    if alpha_chars.len() < 2 {
        return false;
    }
    let upper_count = alpha_chars.iter().filter(|c| c.is_uppercase()).count();
    (upper_count as f64 / alpha_chars.len() as f64) > 0.8
}

/// Demote headings that follow paragraphs ending without sentence-terminating punctuation.
///
/// When a paragraph doesn't end with `.`, `?`, `!`, `:`, or `;`, the next paragraph
/// is likely a continuation (e.g., from a multi-column layout split). Headings
/// promoted by the rescue pass in this position are demoted back to body text.
///
/// Only demotes headings that were NOT confirmed by font-size clustering (i.e.,
/// headings at levels that could have come from the rescue pass).
fn demote_continuation_headings(paragraphs: &mut [PdfParagraph]) {
    if paragraphs.len() < 2 {
        return;
    }

    for i in 1..paragraphs.len() {
        if paragraphs[i].heading_level.is_none() {
            continue;
        }

        let prev = &paragraphs[i - 1];
        if prev.heading_level.is_some()
            || prev.is_list_item
            || prev.is_code_block
            || prev.is_formula
            || prev.is_page_furniture
        {
            continue;
        }

        let prev_text = if !prev.text.is_empty() {
            prev.text.clone()
        } else {
            prev.lines
                .iter()
                .flat_map(|l| l.segments.iter())
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        };
        let prev_trimmed = prev_text.trim_end();
        let ends_with_terminator = matches!(prev_trimmed.chars().last(), Some('.' | '?' | '!' | ':' | ';'));

        if !ends_with_terminator && !prev_trimmed.is_empty() {
            paragraphs[i].heading_level = None;
        }
    }
}

/// Find the heading level for a given font size by matching against the cluster centroids.
pub(super) fn find_heading_level(font_size: f32, heading_map: &[(f32, Option<u8>)], gap_info: &GapInfo) -> Option<u8> {
    if heading_map.is_empty() {
        return None;
    }
    if heading_map.len() == 1 {
        return heading_map[0].1;
    }

    let mut best_distance = f32::INFINITY;
    let mut best_level: Option<u8> = None;
    for &(centroid, level) in heading_map {
        let dist = (font_size - centroid).abs();
        if dist < best_distance {
            best_distance = dist;
            best_level = level;
        }
    }

    if best_distance > MAX_HEADING_DISTANCE_MULTIPLIER * gap_info.avg_gap {
        return None;
    }

    best_level
}

pub(super) struct GapInfo {
    avg_gap: f32,
}

pub(super) fn precompute_gap_info(heading_map: &[(f32, Option<u8>)]) -> GapInfo {
    if heading_map.len() <= 1 {
        return GapInfo { avg_gap: f32::INFINITY };
    }

    let mut centroids: Vec<f32> = heading_map.iter().map(|(c, _)| *c).collect();
    centroids.sort_by(|a, b| a.total_cmp(b));
    let gaps: Vec<f32> = centroids.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
    let avg_gap = if gaps.is_empty() {
        f32::INFINITY
    } else {
        gaps.iter().sum::<f32>() / gaps.len() as f32
    };

    GapInfo { avg_gap }
}

/// Refine heading levels across the entire document.
///
/// 1. Promotes the first heading to H1 when no H1 exists (title inference),
///    unless a sparse document repeats the same H2 tier at the start of pages.
/// 2. Merges consecutive H1 headings at the same font size into one title (any page).
/// 3. Demotes numbered section headings from H1 to H2 when a non-numbered title H1 exists.
/// 4. Preserves changelog section/version hierarchy where geometry confirms adjacency.
pub(super) fn refine_heading_hierarchy(all_pages: &mut [Vec<PdfParagraph>]) {
    split_combined_changelog_version_headings(all_pages);
    refine_heading_hierarchy_inner(all_pages);
    promote_changelog_version_headings(all_pages);
}

fn refine_heading_hierarchy_inner(all_pages: &mut [Vec<PdfParagraph>]) {
    let h1_count: usize = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .filter(|p| p.heading_level == Some(1))
        .count();

    if h1_count == 0 {
        let has_any_heading = all_pages
            .iter()
            .flat_map(|page| page.iter())
            .any(|p| p.heading_level.is_some());
        if has_any_heading && !has_repeated_sparse_peer_heading_tier(all_pages) {
            promote_title_heading(all_pages);
        }

        let still_no_h1 = !all_pages
            .iter()
            .flat_map(|page| page.iter())
            .any(|p| p.heading_level == Some(1));
        if still_no_h1 && !all_pages.is_empty() && !all_pages[0].is_empty() {
            let total_paragraphs: usize = all_pages.iter().map(|page| page.len()).sum();
            let page0 = &all_pages[0];
            let max_font_on_page = page0.iter().map(|p| p.dominant_font_size).fold(0.0f32, f32::max);
            let first = &page0[0];
            let first_text = paragraph_plain_text(first);
            let first_wc = first_text.split_whitespace().count();
            let rest_font_size = other_paragraphs_font_size(all_pages, 0, 0);
            let clears_font_gate = rest_font_size.is_none_or(|body_font| {
                body_font <= 0.0
                    || (first.dominant_font_size >= body_font * MIN_HEADING_FONT_RATIO
                        && first.dominant_font_size >= body_font + MIN_HEADING_FONT_GAP)
            });
            if total_paragraphs >= MIN_BLOCKS_FOR_FONT_HEADING
                && clears_font_gate
                && first.dominant_font_size >= max_font_on_page
                && first_wc <= 10
                && first_wc > 0
                && !first.is_page_furniture
                && !looks_like_bare_url(&first_text)
                && !is_changelog_hierarchy_member(page0, 0)
            {
                all_pages[0][0].heading_level = Some(1);
            }
        }
    }

    let h1_count: usize = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .filter(|p| p.heading_level == Some(1))
        .count();

    if h1_count <= 1 {
        return;
    }

    for page in all_pages.iter_mut() {
        merge_consecutive_h1s(page);
    }

    let h1_count: usize = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .filter(|p| p.heading_level == Some(1))
        .count();

    if h1_count <= 1 {
        return;
    }

    let first_h1_is_title = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .find(|p| p.heading_level == Some(1))
        .is_some_and(|p| !starts_with_section_number(&paragraph_plain_text(p)));

    if !first_h1_is_title {
        return;
    }

    let mut found_first = false;
    for page in all_pages.iter_mut() {
        for para in page.iter_mut() {
            if para.heading_level == Some(1) {
                if !found_first {
                    found_first = true;
                    continue;
                }
                if starts_with_section_number(&paragraph_plain_text(para)) {
                    para.heading_level = Some(2);
                }
            }
        }
    }
}

fn split_combined_changelog_version_headings(all_pages: &mut [Vec<PdfParagraph>]) {
    for page in all_pages {
        let mut index = 0;
        while index < page.len() {
            let Some(version) = combined_changelog_version(&page[index]) else {
                index += 1;
                continue;
            };
            if !page[index].is_bold
                || page[index].is_list_item
                || page[index].is_code_block
                || page[index].is_formula
                || page[index].is_page_furniture
            {
                index += 1;
                continue;
            }
            let Some((parent_bbox, version_bbox)) = combined_changelog_split_bboxes(&page[index]) else {
                index += 1;
                continue;
            };

            let mut version_paragraph = page[index].clone();
            page[index].text = "Recent Change Log".to_string();
            page[index].lines.clear();
            page[index].heading_level = Some(2);
            page[index].block_bbox = Some(parent_bbox);
            page[index].word_count = 3;

            version_paragraph.text = version;
            version_paragraph.lines.clear();
            version_paragraph.heading_level = Some(3);
            version_paragraph.is_list_item = false;
            version_paragraph.is_code_block = false;
            version_paragraph.is_formula = false;
            version_paragraph.is_page_furniture = false;
            version_paragraph.block_bbox = Some(version_bbox);
            version_paragraph.word_count = 1;
            page.insert(index + 1, version_paragraph);
            index += 2;
        }
    }
}

fn combined_changelog_version(paragraph: &PdfParagraph) -> Option<String> {
    let text = effective_text(paragraph);
    let text = text.trim();
    let version = if let Some(version) = text.strip_prefix("Recent Change Log\n") {
        version
    } else {
        if !combined_changelog_has_multiline_geometry(paragraph) {
            return None;
        }
        text.strip_prefix("Recent Change Log ")?
    }
    .trim();
    (!version.contains(char::is_whitespace) && is_semantic_version(version)).then(|| version.to_string())
}

fn combined_changelog_has_multiline_geometry(paragraph: &PdfParagraph) -> bool {
    let baseline_gap = paragraph
        .lines
        .iter()
        .map(|line| line.baseline_y)
        .fold(None, |bounds, baseline| match bounds {
            None => Some((baseline, baseline)),
            Some((minimum, maximum)) => Some((minimum.min(baseline), maximum.max(baseline))),
        })
        .is_some_and(|(minimum, maximum)| {
            maximum - minimum >= paragraph.dominant_font_size * CHANGELOG_COMBINED_MIN_BASELINE_GAP_RATIO
        });
    let tall_bbox = paragraph.block_bbox.is_some_and(|(_, bottom, _, top)| {
        top - bottom >= paragraph.dominant_font_size * CHANGELOG_COMBINED_MIN_HEIGHT_RATIO
    });

    baseline_gap || tall_bbox
}

fn combined_changelog_split_bboxes(paragraph: &PdfParagraph) -> Option<(ParagraphBbox, ParagraphBbox)> {
    let (left, bottom, right, top) = paragraph.block_bbox.or_else(|| {
        let mut segments = paragraph.lines.iter().flat_map(|line| line.segments.iter());
        let first = segments.next()?;
        Some(segments.fold(
            (first.x, first.y, first.x + first.width, first.y + first.height),
            |(left, bottom, right, top), segment| {
                (
                    left.min(segment.x),
                    bottom.min(segment.y),
                    right.max(segment.x + segment.width),
                    top.max(segment.y + segment.height),
                )
            },
        ))
    })?;
    let midpoint = bottom + (top - bottom) / 2.0;
    Some(((left, midpoint, right, top), (left, bottom, right, midpoint)))
}

fn promote_changelog_version_headings(all_pages: &mut [Vec<PdfParagraph>]) {
    for page in all_pages {
        for index in 0..page.len().saturating_sub(1) {
            let (before, after) = page.split_at_mut(index + 1);
            let parent = &mut before[index];
            let version = &mut after[0];

            if !is_changelog_version_pair(parent, version) {
                continue;
            }

            parent.heading_level = Some(2);
            version.heading_level = Some(3);
        }
    }
}

fn is_changelog_version_pair(parent: &PdfParagraph, version: &PdfParagraph) -> bool {
    parent.heading_level.is_some()
        && effective_text(parent).trim() == "Recent Change Log"
        && version.is_bold
        && is_semantic_version(effective_text(version).trim())
        && !version.is_list_item
        && !version.is_code_block
        && !version.is_formula
        && !version.is_page_furniture
        && changelog_version_is_near_and_aligned(parent, version)
}

fn is_semantic_version(text: &str) -> bool {
    let version = text.strip_prefix('v').unwrap_or(text);
    let mut parts = version.split('.');
    let first = parts.next();
    let second = parts.next();
    let third = parts.next();
    let has_extra = parts.next().is_some();

    !has_extra
        && first.is_some_and(is_ascii_digits)
        && second.is_some_and(is_ascii_digits)
        && third.is_none_or(is_ascii_digits)
}

fn is_ascii_digits(part: &str) -> bool {
    !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
}

fn changelog_version_is_near_and_aligned(parent: &PdfParagraph, version: &PdfParagraph) -> bool {
    let (Some((parent_left, parent_bottom, _, _)), Some((version_left, _, _, version_top))) =
        (parent.block_bbox, version.block_bbox)
    else {
        return false;
    };

    let reference_size = parent.dominant_font_size.max(version.dominant_font_size);
    let left_offset = (parent_left - version_left).abs();
    let vertical_gap = parent_bottom - version_top;

    left_offset <= reference_size * CHANGELOG_VERSION_MAX_LEFT_OFFSET_RATIO
        && vertical_gap >= -reference_size * CHANGELOG_VERSION_MAX_VERTICAL_OVERLAP_RATIO
        && vertical_gap <= reference_size * CHANGELOG_VERSION_MAX_VERTICAL_GAP_RATIO
}

/// Determine heading level for a bold/italic paragraph based on font-size ratio to body.
///
/// - Font size > 1.2× body → H2 (clearly larger sub-heading)
/// - Font size at body size but bold → H3 (same-size bold sub-heading)
/// - Section numbering overrides: uses dot count for depth (e.g., "3.2" → H3)
fn infer_bold_heading_level(font_size: f32, body_font_size: f32, text: &str) -> u8 {
    if starts_with_section_number(text) {
        return infer_section_level(text);
    }

    if body_font_size > 0.0 {
        let ratio = font_size / body_font_size;
        if ratio > 1.2 {
            return 2;
        }
        return 3;
    }

    2
}

/// Infer heading level from section numbering in text.
///
/// Determines depth from the numbering pattern:
/// - "1 Introduction" or "I. INTRO" or "A. Proofs" → H2 (top-level section)
/// - "1.1 Details" or "A.1 Sub" → H3 (sub-section)
/// - "1.1.1 Deep" → H4 (sub-sub-section)
fn infer_section_level(text: &str) -> u8 {
    let trimmed = text.trim();

    let first_char = trimmed.chars().next().unwrap_or(' ');
    let is_alpha_prefix = first_char.is_ascii_alphabetic()
        && trimmed.len() >= 2
        && matches!(trimmed.as_bytes().get(1), Some(b'.' | b')' | b' '));

    let numbering_end = if is_alpha_prefix {
        let after_letter = &trimmed[1..];
        let rest_end = after_letter
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(0);
        1 + rest_end
    } else {
        let roman_chars: &[u8] = b"IVXLCDM";
        let bytes = trimmed.as_bytes();
        let roman_end = bytes.iter().position(|b| !roman_chars.contains(b)).unwrap_or(0);
        if roman_end > 0 && roman_end <= 5 && roman_end < bytes.len() {
            let next = bytes[roman_end];
            if (next == b'.' || next == b' ' || next == b')') && is_valid_roman(&trimmed[..roman_end]) {
                return 2;
            }
        }
        trimmed.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(0)
    };

    if numbering_end == 0 {
        return 2;
    }

    let numbering = &trimmed[..numbering_end];
    let dot_count = numbering.chars().filter(|&c| c == '.').count();
    let effective_dots = if numbering.ends_with('.') {
        dot_count.saturating_sub(1)
    } else {
        dot_count
    };

    match effective_dots {
        0 => 2,
        1 => 3,
        _ => 4,
    }
}

/// Check whether text ends with a genuine sentence-terminating period.
///
/// A trailing ellipsis (`...` or the `…` glyph) is a truncation marker — common
/// in headings and truncated titles ("Impaired Glucose Tolerance ...") — not a
/// sentence terminator, so it does not disqualify a line from being a heading.
pub(super) fn ends_with_sentence_period(text: &str) -> bool {
    let t = text.trim_end();
    t.ends_with('.') && !t.ends_with("..")
}

/// Check if text looks like a section/legal heading that legitimately ends with a period.
/// Uses language-agnostic structural signals only:
/// - Starts with § (universal section symbol)
/// - All-caps short text (e.g., "ARTICLE IV.", "CHAPITRE 3.")
/// - Starts with a section number (e.g., "3.2. Methods")
pub(super) fn is_section_pattern(text: &str) -> bool {
    let t = text.trim();
    if t.starts_with('§') {
        return true;
    }
    let words = t.split_whitespace().count();
    if words <= 6 && t.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase()) {
        return true;
    }
    starts_with_section_number(t)
}

/// Check if text starts like a numbered SECTION HEADING as opposed to a list item.
///
/// `is_section_pattern` treats ANY leading number as a section marker, which
/// makes numbered list items ("1. First point") unclassifiable as lists. This
/// tighter predicate only flags patterns that are reliably headings:
/// - multi-level numbering: "3.2 Methods", "3.2.1 Details"
/// - roman-numeral markers: "IV. Results", "II) Scope"
/// - single number with an ALL-CAPS remainder: "1. INTRODUCTION"
///
/// A single-level number followed by mixed-case text ("1. Énumération") is a
/// list item and returns `false`.
///
/// A heading may also put a keyword in front of its number -- "ARTIKEL 1.
/// TOEPASSELIJKHEID", "Appendix 1 Product list", "Exhibit A PRODUCT LIST". That
/// form is recognised by shape rather than by a keyword list; see
/// [`section_keyword_prefix`]. Behind a keyword the remainder need only be
/// capitalised, which is what keeps "Artikel 12 van de wet is van toepassing."
/// classified as the prose it is. See #1608.
pub(super) fn is_numbered_section_heading(text: &str) -> bool {
    let t = text.trim();
    if is_bare_numbered_section_heading(t) {
        return true;
    }
    section_keyword_prefix(t).is_some_and(is_keyword_numbered_section_heading)
}

/// The enumerator-first half of [`is_numbered_section_heading`]: the number,
/// or the roman numeral, is the line's own first token.
fn is_bare_numbered_section_heading(t: &str) -> bool {
    let bytes = t.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    let roman_chars: &[u8] = b"IVXLCDM";
    let roman_end = bytes.iter().position(|b| !roman_chars.contains(b)).unwrap_or(0);
    if roman_end > 0
        && roman_end < bytes.len()
        && matches!(bytes[roman_end], b'.' | b' ' | b')')
        && is_valid_roman(&t[..roman_end])
    {
        return true;
    }

    let mut levels = 0usize;
    let mut idx = 0usize;
    loop {
        let digit_len = bytes[idx..]
            .iter()
            .position(|b| !b.is_ascii_digit())
            .unwrap_or(bytes.len() - idx);
        if digit_len == 0 {
            break;
        }
        levels += 1;
        idx += digit_len;
        if bytes.get(idx) == Some(&b'.') && bytes.get(idx + 1).is_some_and(|b| b.is_ascii_digit()) {
            idx += 1;
        } else {
            break;
        }
    }
    if levels == 0 {
        return false;
    }
    if levels >= 2 {
        return true;
    }

    if matches!(bytes.get(idx), Some(b'.') | Some(b')')) {
        idx += 1;
    }
    let remainder = t[idx..].trim_start();
    remainder.chars().any(|c| c.is_alphabetic())
        && remainder
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(|c| c.is_uppercase())
}

/// A leading section keyword is a word, not a preposition. Without a floor,
/// `Op 3 MAART` and `In 5 STAPPEN` read as section numbering, because an
/// all-caps remainder satisfies every other term. See #1608. ~keep
const MIN_SECTION_KEYWORD_CHARS: usize = 3;

/// Split a leading section keyword (`ARTIKEL`, `Appendix`, `Annex`, `Chapter`,
/// `Artículo`, ...) off `t` and return what follows it.
///
/// Deliberately NOT a keyword list: enumerating them is endless and
/// language-bound, and measured on GH#1608's reproducer a list would still have
/// missed two of the five shapes. What identifies the form is its shape -- one
/// capitalised alphabetic word standing in front of an enumerator -- while the
/// enumerator and the case of the remainder do the discriminating. See #1608. ~keep
fn section_keyword_prefix(t: &str) -> Option<&str> {
    if !t.chars().next()?.is_uppercase() {
        return None;
    }
    let word_end = t.find(char::is_whitespace)?;
    let word = &t[..word_end];
    if !word.chars().all(char::is_alphabetic) || word.chars().count() < MIN_SECTION_KEYWORD_CHARS {
        return None;
    }
    Some(t[word_end..].trim_start())
}

/// Whether `rest` -- a line with its leading section keyword removed -- reads as
/// a numbered section heading.
fn is_keyword_numbered_section_heading(rest: &str) -> bool {
    let Some(after_enumerator) = section_enumerator_end(rest) else {
        return false;
    };
    let remainder = rest[after_enumerator..].trim_start_matches(['.', ')']).trim_start();
    // `Artikel 12 van de wet is van toepassing.` is prose and must stay prose;
    // `Appendix 1 Product list` is a heading. Keyword and enumerator are the same
    // shape in both, so the case of the first letter after the enumerator is the
    // only thing separating them. A bare enumerator keeps the stricter all-caps
    // rule -- there the keyword is not there to vouch for it. See #1608. ~keep
    match remainder.chars().find(|c| c.is_alphabetic()) {
        None => true,
        Some(first) => first.is_uppercase(),
    }
}

/// Byte offset just past a leading enumerator in `rest`: arabic digits with
/// optional `.`-separated levels, a roman numeral, or a single uppercase letter.
///
/// The enumerator must be terminated by end of line, `.`, `)` or a space, so
/// `Article 7a` and `Annex IVX` do not read as enumerated. The single-letter arm
/// exists for `Exhibit A` / `Annex B`, and is reachable only behind a keyword --
/// a bare `A.` is far more often a list marker than a section number. ~keep
fn section_enumerator_end(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let terminated = |end: usize| matches!(bytes.get(end), None | Some(b'.' | b')' | b' '));

    let mut end = 0usize;
    loop {
        let digit_len = bytes[end..]
            .iter()
            .position(|b| !b.is_ascii_digit())
            .unwrap_or(bytes.len() - end);
        if digit_len == 0 {
            break;
        }
        end += digit_len;
        if bytes.get(end) == Some(&b'.') && bytes.get(end + 1).is_some_and(u8::is_ascii_digit) {
            end += 1;
        } else {
            break;
        }
    }
    if end > 0 {
        return terminated(end).then_some(end);
    }

    let roman_chars: &[u8] = b"IVXLCDM";
    let roman_end = bytes
        .iter()
        .position(|b| !roman_chars.contains(b))
        .unwrap_or(bytes.len());
    if roman_end > 0 && terminated(roman_end) && is_valid_roman(&rest[..roman_end]) {
        return Some(roman_end);
    }

    (bytes[0].is_ascii_uppercase() && terminated(1)).then_some(1)
}

/// Check if text starts with a section number pattern (e.g., "1 ", "2.1 ", "A.", "III.").
pub(super) fn starts_with_section_number(text: &str) -> bool {
    let trimmed = text.trim();
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let digit_end = bytes.iter().position(|&b| !b.is_ascii_digit()).unwrap_or(0);
    if digit_end > 0 && digit_end < bytes.len() {
        let next = bytes[digit_end];
        if next == b' ' || next == b'.' || next == b')' {
            return true;
        }
    }
    let roman_chars: &[u8] = b"IVXLCDM";
    let roman_end = bytes.iter().position(|b| !roman_chars.contains(b)).unwrap_or(0);
    if roman_end > 0 && roman_end <= 5 && roman_end < bytes.len() {
        let next = bytes[roman_end];
        if next == b'.' || next == b' ' || next == b')' {
            let prefix = &trimmed[..roman_end];
            if is_valid_roman(prefix) {
                return true;
            }
        }
    }
    false
}

/// Check if a string is a valid roman numeral (I-XX range, covers most section numbering).
fn is_valid_roman(s: &str) -> bool {
    matches!(
        s,
        "I" | "II"
            | "III"
            | "IV"
            | "V"
            | "VI"
            | "VII"
            | "VIII"
            | "IX"
            | "X"
            | "XI"
            | "XII"
            | "XIII"
            | "XIV"
            | "XV"
            | "XVI"
            | "XVII"
            | "XVIII"
            | "XIX"
            | "XX"
    )
}

/// Demote unnumbered H2 headings to H3 when they appear between numbered H2 sections.
///
/// In documents with numbered sections (e.g., "1 INTRODUCTION", "5 EXPERIMENTS"),
/// unnumbered headings between consecutive numbered H2s are typically sub-sections.
/// For example, "Baselines for Object Detection" between "5 EXPERIMENTS" and
/// "6 CONCLUSION" should be H3, not H2.
///
/// Only applies when the document has at least 3 numbered H2 headings, indicating
/// a consistent numbering scheme.
pub(super) fn demote_unnumbered_subsections(all_pages: &mut [Vec<PdfParagraph>]) {
    let mut h2_info: Vec<(usize, usize, bool)> = Vec::new();
    for (page_idx, page) in all_pages.iter().enumerate() {
        for (para_idx, para) in page.iter().enumerate() {
            if para.heading_level == Some(2) {
                let text = paragraph_plain_text(para);
                h2_info.push((page_idx, para_idx, starts_with_section_number(&text)));
            }
        }
    }

    let numbered_count = h2_info.iter().filter(|(_, _, numbered)| *numbered).count();
    if numbered_count < 3 {
        return;
    }

    let numbered_positions: Vec<usize> = h2_info
        .iter()
        .enumerate()
        .filter(|(_, (_, _, numbered))| *numbered)
        .map(|(idx, _)| idx)
        .collect();

    for window in numbered_positions.windows(2) {
        let start = window[0];
        let end = window[1];
        for &(page_idx, para_idx, is_numbered) in &h2_info[start + 1..end] {
            if !is_numbered {
                let layout_confirmed = matches!(
                    all_pages[page_idx][para_idx].layout_class,
                    Some(super::types::LayoutHintClass::SectionHeader | super::types::LayoutHintClass::Title)
                );
                if !layout_confirmed {
                    all_pages[page_idx][para_idx].heading_level = Some(3);
                }
            }
        }
    }
}

/// Demote long runs of consecutive same-level headings to body text.
///
/// When the layout model (or font-size classification) produces 4+ consecutive
/// headings at the same level with no intervening body text, they're likely
/// misclassified (e.g., song lyrics, list items, short centered paragraphs).
/// Real documents rarely have more than 3 consecutive headings.
pub(super) fn demote_heading_runs(all_pages: &mut [Vec<PdfParagraph>]) {
    const MAX_CONSECUTIVE: usize = 3;

    for page in all_pages.iter_mut() {
        let mut run_start = 0;
        while run_start < page.len() {
            let Some(level) = page[run_start].heading_level else {
                run_start += 1;
                continue;
            };

            let mut run_end = run_start + 1;
            while run_end < page.len()
                && page[run_end].heading_level == Some(level)
                && page[run_end].layout_region_path == page[run_start].layout_region_path
            {
                run_end += 1;
            }

            let run_len = run_end - run_start;
            if run_len > MAX_CONSECUTIVE {
                for para in &mut page[run_start + 1..run_end] {
                    let layout_confirmed = matches!(
                        para.layout_class,
                        Some(super::types::LayoutHintClass::SectionHeader | super::types::LayoutHintClass::Title)
                    );
                    if !layout_confirmed {
                        para.heading_level = None;
                    }
                }
            }

            run_start = run_end;
        }
    }
}

/// Char-weighted dominant font size of every paragraph in the document except
/// the one at `(exclude_page, exclude_index)`.
///
/// Used to gate the "no heading anywhere" title-promotion fallback: the
/// excluded paragraph is the force-promotion candidate, so this answers "how
/// much larger is the candidate than the rest of the document's text", the
/// same ratio/gap test `assign_heading_levels_smart` applies to font-size
/// clusters. Returns `None` when there is no other text to compare against.
fn other_paragraphs_font_size(
    all_pages: &[Vec<PdfParagraph>],
    exclude_page: usize,
    exclude_index: usize,
) -> Option<f32> {
    let mut weighted_sum = 0.0f64;
    let mut total_chars = 0usize;
    for (page_idx, page) in all_pages.iter().enumerate() {
        for (para_idx, para) in page.iter().enumerate() {
            if page_idx == exclude_page && para_idx == exclude_index {
                continue;
            }
            let char_count = paragraph_plain_text(para).chars().count();
            if char_count == 0 {
                continue;
            }
            weighted_sum += f64::from(para.dominant_font_size) * char_count as f64;
            total_chars += char_count;
        }
    }
    if total_chars == 0 {
        None
    } else {
        Some((weighted_sum / total_chars as f64) as f32)
    }
}

/// Extract plain text from a paragraph.
fn paragraph_plain_text(para: &PdfParagraph) -> String {
    para.lines
        .iter()
        .flat_map(|l| l.segments.iter())
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn is_body_size_bold_signal(para: &PdfParagraph, body_font_size: f32) -> bool {
    if para.heading_level.is_some()
        || !para.is_bold
        || para.is_list_item
        || para.is_code_block
        || para.is_formula
        || para.is_page_furniture
        || para.lines.len() != 1
        || !body_font_size.is_finite()
        || body_font_size <= 0.0
        || (para.dominant_font_size - body_font_size).abs() > 0.5
        || para.word_count > MAX_BOLD_HEADING_WORD_COUNT
    {
        return false;
    }

    let text = paragraph_plain_text(para);
    let trimmed = text.trim();
    !trimmed.is_empty()
        && (!ends_with_sentence_period(trimmed) || is_section_pattern(trimmed))
        && (!trimmed.ends_with(':') || is_all_caps_text(trimmed))
        && !looks_like_figure_label(trimmed)
        && !looks_like_bare_url(trimmed)
        && !super::layout_classify::is_separator_text(trimmed)
}

/// On typography alone a bold body-size line needs at least this many words to read
/// as a heading -- shorter bold fragments are far more often emphasis, a label, or a
/// run-in lead. See [`is_body_size_bold_heading_candidate`] for the exemption. ~keep
const MIN_BOLD_HEADING_WORD_COUNT: usize = 3;

pub(super) fn is_body_size_bold_heading_candidate(para: &PdfParagraph, body_font_size: f32) -> bool {
    if !is_body_size_bold_signal(para, body_font_size) {
        return false;
    }
    // A numbered section heading carries its own evidence and does not need to clear
    // the word-count floor. Requiring three words silently excluded every two-word
    // numbered title -- `3. PRIJZEN`, `1. INTRODUCTION` -- from heading promotion, so
    // it stayed a plain bold paragraph and a RUN of them coalesced into a single bold
    // line in the rendered output while the element stream still showed them apart.
    // Measured on GH#1611: `ARTIKEL 1. TOEPASSELIJKHEID` (3 words) was promoted and
    // `1. TOEPASSELIJKHEID` (2 words) was not, at identical font, weight and body
    // size -- the keyword contributed nothing but the third word. See #1611. ~keep
    para.word_count >= MIN_BOLD_HEADING_WORD_COUNT || is_numbered_section_heading(paragraph_plain_text(para).trim())
}

/// Preserve peer H2 sections when a sparse document repeats their font tier at
/// the start of multiple pages. This prevents generic title inference from
/// arbitrarily promoting only the first peer section. ~keep
fn has_repeated_sparse_peer_heading_tier(all_pages: &[Vec<PdfParagraph>]) -> bool {
    let paragraph_count: usize = all_pages.iter().map(Vec::len).sum();
    if paragraph_count >= MIN_BLOCKS_FOR_FONT_HEADING {
        return false;
    }

    let has_explicit_title = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .any(|paragraph| paragraph.heading_level.is_some() && paragraph.layout_class == Some(LayoutHintClass::Title));
    if has_explicit_title {
        return false;
    }

    let leading_h2_fonts: Vec<f32> = all_pages
        .iter()
        .filter_map(|page| {
            page.iter()
                .find(|paragraph| !paragraph.is_page_furniture && !paragraph_plain_text(paragraph).trim().is_empty())
                .filter(|paragraph| paragraph.heading_level == Some(2) && paragraph.dominant_font_size.is_finite())
                .map(|paragraph| paragraph.dominant_font_size)
        })
        .collect();

    leading_h2_fonts.iter().any(|candidate| {
        leading_h2_fonts
            .iter()
            .filter(|font_size| (**font_size - *candidate).abs() <= SPARSE_PEER_HEADING_FONT_TOLERANCE)
            .count()
            >= SPARSE_PEER_HEADING_MIN_PAGES
    })
}

/// Promote the most likely title heading to H1 when no H1 exists.
///
/// Strategy:
/// 1. If any heading has layout_class == Title, promote it to H1.
/// 2. Otherwise, on the first page only, promote the heading with the largest
///    font size IF it's clearly larger than other headings (at least 1.5pt gap).
fn promote_title_heading(all_pages: &mut [Vec<PdfParagraph>]) {
    for page in all_pages.iter_mut() {
        for index in 0..page.len() {
            if is_changelog_hierarchy_member(page, index) {
                continue;
            }
            let para = &mut page[index];
            if para.heading_level.is_some() && para.layout_class == Some(super::types::LayoutHintClass::Title) {
                para.heading_level = Some(1);
                return;
            }
        }
    }

    if all_pages.is_empty() {
        return;
    }
    let page = &all_pages[0];
    let headings: Vec<(usize, f32)> = page
        .iter()
        .enumerate()
        .filter(|(index, p)| p.heading_level.is_some() && !is_changelog_hierarchy_member(page, *index))
        .map(|(i, p)| (i, p.dominant_font_size))
        .collect();

    if headings.is_empty() {
        return;
    }

    if headings.len() == 1 {
        all_pages[0][headings[0].0].heading_level = Some(1);
        return;
    }

    let max_size = headings.iter().map(|(_, s)| *s).fold(0.0f32, f32::max);
    let second_max = headings
        .iter()
        .map(|(_, s)| *s)
        .filter(|s| *s < max_size)
        .fold(0.0f32, f32::max);

    if max_size - second_max >= 1.5
        && let Some(&(idx, _)) = headings.iter().find(|(_, s)| *s == max_size)
    {
        all_pages[0][idx].heading_level = Some(1);
    }
}

fn is_changelog_hierarchy_member(page: &[PdfParagraph], index: usize) -> bool {
    (index + 1 < page.len() && is_changelog_version_pair(&page[index], &page[index + 1]))
        || (index > 0 && is_changelog_version_pair(&page[index - 1], &page[index]))
}

/// Merge consecutive H1 paragraphs at the same font size into a single heading.
///
/// Split titles (e.g., "KAISUN HOLDINGS" on one line, "LIMITED" on the next)
/// often produce separate H1 paragraphs. When they share the same font size
/// and look like grammatical continuations they should be a single heading.
///
/// Product model codes ("HR 22", "HR 28") at the same font size are NOT merged —
/// each is a distinct item, not a split title.
fn merge_consecutive_h1s(page: &mut Vec<PdfParagraph>) {
    let mut i = 0;
    while i < page.len() {
        if page[i].heading_level != Some(1) {
            i += 1;
            continue;
        }
        let base_fs = page[i].dominant_font_size;
        let mut run_end = i + 1;
        while run_end < page.len()
            && page[run_end].heading_level == Some(1)
            && page[run_end].layout_region_path == page[i].layout_region_path
            && (page[run_end].dominant_font_size - base_fs).abs() < 0.5
            && looks_like_title_continuation(&page[run_end - 1], &page[run_end])
        {
            run_end += 1;
        }
        if run_end - i > 1 {
            let mut merged_lines = std::mem::take(&mut page[i].lines);
            let mut merged_text_parts: Vec<String> = if page[i].text.is_empty() {
                Vec::new()
            } else {
                vec![std::mem::take(&mut page[i].text)]
            };
            for para in &page[i + 1..run_end] {
                merged_lines.extend(para.lines.clone());
                if !para.text.is_empty() {
                    merged_text_parts.push(para.text.clone());
                }
            }
            page[i].lines = merged_lines;
            if !merged_text_parts.is_empty() {
                page[i].text = merged_text_parts.join(" ");
            }
            page.drain(i + 1..run_end);
        }
        i += 1;
    }
}

/// True when `next` looks like a grammatical continuation of `prev` (split title).
///
/// Returns false for product-code-style lines ("HR 22", "HR 28/24") and numbered
/// items — those are standalone headings, not continuations of the previous line.
///
/// The 4-word limit is conservative by design: real split titles ("KAISUN HOLDINGS" /
/// "LIMITED", "BANCO CENTRAL" / "DO BRASIL") are invariably short. Longer first lines
/// are more likely lists or section labels than wrapped titles. Trades occasional
/// under-merging of unusually long company names for zero false merges on cover-page
/// model-code lists — the class of bug this function was introduced to fix.
fn looks_like_title_continuation(prev: &PdfParagraph, next: &PdfParagraph) -> bool {
    let next_text = effective_text(next);
    if looks_like_standalone_heading_text(&next_text) {
        return false;
    }
    let prev_text = effective_text(prev);
    if prev_text.trim_end().ends_with(['.', '!', '?', ':']) {
        return false;
    }
    let prev_wc = prev_text.split_whitespace().count();
    let next_wc = next_text.split_whitespace().count();
    prev_wc <= 4 && next_wc <= 4
}

/// Return the effective plain text for a paragraph, preferring `para.text` (the
/// full-text heuristic path) over segment-joined `para.lines` (structure-tree path).
///
/// Mirrors the `get_text` closure in `assembly::push_paragraph_element` so that
/// classification helpers and rendering always agree on what a paragraph says.
fn effective_text(para: &PdfParagraph) -> String {
    if !para.text.is_empty() {
        para.text.clone()
    } else {
        paragraph_plain_text(para)
    }
}

/// True when text looks like a standalone heading rather than a title continuation.
///
/// Matches product model codes ("HR 22", "HR 28/24"), mixed alphanumeric tokens
/// ("A4", "HR28"), and lines that start with a digit.
fn looks_like_standalone_heading_text(text: &str) -> bool {
    let trimmed = text.trim();
    let mut words = trimmed.splitn(3, ' ');
    let first = words.next().unwrap_or("");
    let second = words.next().unwrap_or("");

    if first.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return true;
    }
    let first_has_alpha = first.chars().any(|c| c.is_ascii_alphabetic());
    let first_has_digit = first.chars().any(|c| c.is_ascii_digit());
    if first_has_alpha && first_has_digit {
        return true;
    }
    if !second.is_empty()
        && first.chars().all(|c| c.is_ascii_alphabetic())
        && second.chars().next().is_some_and(|c| c.is_ascii_digit())
    {
        return true;
    }
    false
}

/// Detect paragraphs in page margins that repeat across pages.
///
/// Only considers paragraphs whose bounding box falls in the page margins
/// (top 10%, bottom 10%, or narrow left/right strips). If the same text
/// appears in the margins on >50% of pages, it's furniture (running headers,
/// footers, metadata stamps, watermarks).
///
/// `page_heights` provides the height of each page for margin calculation.
pub(super) fn mark_cross_page_repeating_text(all_pages: &mut [Vec<PdfParagraph>], page_heights: &[f32]) {
    if all_pages.len() < 4 {
        return;
    }

    let margin_frac = 0.10;

    let mut text_page_count: ahash::AHashMap<String, usize> = ahash::AHashMap::new();
    let mut alphanum_to_exact: ahash::AHashMap<String, ahash::AHashSet<String>> = ahash::AHashMap::new();
    let mut first_seen_page: ahash::AHashMap<String, usize> = ahash::AHashMap::new();

    for (page_idx, page) in all_pages.iter().enumerate() {
        let page_h = page_heights.get(page_idx).copied().unwrap_or(792.0);
        let top_margin_y = page_h * (1.0 - margin_frac);
        let bottom_margin_y = page_h * margin_frac;

        let mut seen: ahash::AHashSet<String> = ahash::AHashSet::new();
        for para in page {
            if para.is_page_furniture {
                continue;
            }

            let in_margin = para
                .block_bbox
                .is_some_and(|(_, bottom, _, top)| top > top_margin_y || bottom < bottom_margin_y);
            if !in_margin {
                continue;
            }

            let text = paragraph_plain_text(para);
            let normalized = text.trim().to_lowercase();
            if normalized.is_empty() {
                continue;
            }

            let alphanum_key: String = normalized.chars().filter(|c| c.is_alphanumeric()).collect();
            if alphanum_key.is_empty() {
                continue;
            }

            alphanum_to_exact
                .entry(alphanum_key.clone())
                .or_default()
                .insert(normalized.clone());

            if seen.insert(alphanum_key.clone()) {
                let count = text_page_count.entry(alphanum_key.clone()).or_insert(0);
                if *count == 0 {
                    first_seen_page.insert(alphanum_key, page_idx);
                }
                *count += 1;
            }
        }
    }

    let threshold = all_pages.len() / 2;

    let mut repeating: ahash::AHashSet<String> = ahash::AHashSet::new();
    for (alphanum_key, count) in &text_page_count {
        if *count > threshold
            && let Some(variants) = alphanum_to_exact.get(alphanum_key)
        {
            for v in variants {
                repeating.insert(v.clone());
            }
        }
    }

    if repeating.is_empty() {
        return;
    }

    tracing::debug!(
        repeating_count = repeating.len(),
        threshold,
        total_pages = all_pages.len(),
        "cross-page margin repeating text detected"
    );

    for (page_idx, page) in all_pages.iter_mut().enumerate() {
        let page_h = page_heights.get(page_idx).copied().unwrap_or(792.0);
        let top_margin_y = page_h * (1.0 - margin_frac);
        let bottom_margin_y = page_h * margin_frac;

        for para in page.iter_mut() {
            if para.is_page_furniture {
                continue;
            }
            let in_margin = para
                .block_bbox
                .is_some_and(|(_, bottom, _, top)| top > top_margin_y || bottom < bottom_margin_y);
            if !in_margin {
                continue;
            }
            let text = paragraph_plain_text(para);
            let normalized = text.trim().to_lowercase();
            if repeating.contains(&normalized) {
                let alphanum_key: String = normalized.chars().filter(|c| c.is_alphanumeric()).collect();
                if first_seen_page.get(&alphanum_key).copied() == Some(page_idx) {
                    continue;
                }
                tracing::trace!(
                    text = %normalized.chars().take(60).collect::<String>(),
                    was_heading = ?para.heading_level,
                    "marking margin text as furniture"
                );
                para.is_page_furniture = true;
                para.heading_level = None;
            }
        }
    }
}

/// Check if a character is a math/formula character.
///
/// Includes common math operators, Greek letters, set theory symbols,
/// and other characters commonly found in mathematical formulas.
fn is_math_character(c: char) -> bool {
    matches!(
        c,
        '\u{2200}'
            | '\u{2203}'
            | '\u{2208}'
            | '\u{2209}'
            | '\u{2282}'
            | '\u{2283}'
            | '\u{222A}'
            | '\u{2229}'
            | '\u{2211}'
            | '\u{222B}'
            | '\u{220F}'
            | '\u{2202}'
            | '\u{2207}'
            | '\u{2264}'
            | '\u{2265}'
            | '\u{2260}'
            | '\u{2248}'
            | '\u{00B1}'
            | '\u{221E}'
            | '\u{221A}'
            | '\u{2192}'
            | '\u{2190}'
            | '\u{2194}'
            | '\u{21D2}'
            | '\u{21D0}'
            | '\u{27E8}'
            | '\u{27E9}'
            | '\u{00D7}'
            | '\u{00F7}'
            | '+'
            | '='
            | '^'
            | '\u{00B9}'
            | '\u{00B2}'
            | '\u{00B3}'
            | '\u{2070}'..='\u{209F}'
    ) || is_greek_letter(c)
}

/// Check if a character is a Greek letter (lowercase α-ω or uppercase Α-Ω).
fn is_greek_letter(c: char) -> bool {
    matches!(c, '\u{0391}'..='\u{03A9}' | '\u{03B1}'..='\u{03C9}')
}

/// Remove arXiv watermark/sidebar noise from paragraphs on the first pages.
///
/// Handles two cases:
/// 1. Short standalone paragraphs that are just the arXiv identifier → mark as furniture.
/// 2. arXiv identifier appended to the end of a longer paragraph (LaTeX sidebar
///    text that the PDF extractor concatenates with body text) → strip the trailing noise.
pub(super) fn mark_arxiv_noise(all_pages: &mut [Vec<PdfParagraph>]) {
    let arxiv_re = regex::Regex::new(r"arXiv:\d{4}\.\d{4,5}").expect("valid regex");
    let trailing_re = regex::Regex::new(
        r"(?:\s+(?:\S+\s+){0,8})?arXiv:\d{4}\.\d{4,5}(?:v\d+)?(?:\s*\[[\w.-]+\])?\s*(?:\d{1,2}\s+\w+\s+\d{4})?\s*$",
    )
    .expect("valid regex");

    for page in all_pages.iter_mut().take(2) {
        for para in page.iter_mut() {
            if para.is_page_furniture {
                continue;
            }
            let text = paragraph_plain_text(para);
            let trimmed = text.trim();
            let word_count = trimmed.split_whitespace().count();

            if !arxiv_re.is_match(trimmed) {
                continue;
            }

            if word_count <= 25 {
                tracing::trace!(
                    text = %trimmed.chars().take(80).collect::<String>(),
                    "marking arXiv watermark as furniture"
                );
                para.is_page_furniture = true;
                para.heading_level = None;
            } else if let Some(m) = trailing_re.find(trimmed) {
                let noise = &trimmed[m.start()..];
                tracing::trace!(
                    stripped = %noise.chars().take(80).collect::<String>(),
                    "stripping trailing arXiv watermark from paragraph"
                );
                strip_trailing_text_from_paragraph(para, noise.trim());
            }
        }
    }
}

/// Strip trailing noise text from the last segment(s) of a paragraph.
fn strip_trailing_text_from_paragraph(para: &mut PdfParagraph, noise: &str) {
    for line in para.lines.iter_mut().rev() {
        for seg in line.segments.iter_mut().rev() {
            if let Some(pos) = seg.text.find(noise) {
                seg.text = seg.text[..pos].trim_end().to_string();
                return;
            }
            let seg_trimmed = seg.text.trim();
            if !seg_trimmed.is_empty() && noise.contains(seg_trimmed) {
                seg.text.clear();
            } else {
                return;
            }
        }
    }
}

/// Second-tier cross-page repeating text detection.
///
/// Supplements `mark_cross_page_repeating_text` by scanning ALL paragraphs
/// (not just margin-positioned ones) for short text that repeats on a
/// supermajority of pages. Catches inline conference headers, journal running
/// titles, and similar repeated boilerplate that appears outside the margin zone.
pub(super) fn mark_cross_page_repeating_short_text(all_pages: &mut [Vec<PdfParagraph>]) {
    if all_pages.len() < 5 {
        return;
    }

    let max_words = 20;
    let threshold = (all_pages.len() as f64 * 0.7).ceil() as usize;

    let mut text_page_count: ahash::AHashMap<String, usize> = ahash::AHashMap::new();
    let mut first_seen_page: ahash::AHashMap<String, usize> = ahash::AHashMap::new();
    for (page_idx, page) in all_pages.iter().enumerate() {
        let mut seen: ahash::AHashSet<String> = ahash::AHashSet::new();
        for para in page {
            // Positioned paragraphs are handled by the margin-aware first tier.
            // Restrict this broad fallback to unpositioned structure-tree text so
            // repeated semantic form fields in the page body are not discarded.
            if para.is_page_furniture || para.block_bbox.is_some() {
                continue;
            }
            let text = paragraph_plain_text(para);
            let normalized = text.trim().to_lowercase();
            if normalized.is_empty() {
                continue;
            }
            let word_count = normalized.split_whitespace().count();
            if word_count > max_words {
                continue;
            }
            let alphanum_key: String = normalized.chars().filter(|c| c.is_alphanumeric()).collect();
            if alphanum_key.is_empty() {
                continue;
            }
            if seen.insert(alphanum_key.clone()) {
                let count = text_page_count.entry(alphanum_key.clone()).or_insert(0);
                if *count == 0 {
                    first_seen_page.insert(alphanum_key, page_idx);
                }
                *count += 1;
            }
        }
    }

    let repeating: ahash::AHashSet<String> = text_page_count
        .into_iter()
        .filter(|(_, count)| *count >= threshold)
        .map(|(key, _)| key)
        .collect();

    if repeating.is_empty() {
        return;
    }

    tracing::debug!(
        repeating_count = repeating.len(),
        threshold,
        total_pages = all_pages.len(),
        "cross-page short-text repeating detection (tier 2)"
    );

    for (page_idx, page) in all_pages.iter_mut().enumerate() {
        for para in page.iter_mut() {
            if para.is_page_furniture || para.block_bbox.is_some() {
                continue;
            }
            let text = paragraph_plain_text(para);
            let normalized = text.trim().to_lowercase();
            let word_count = normalized.split_whitespace().count();
            if word_count > max_words {
                continue;
            }
            let alphanum_key: String = normalized.chars().filter(|c| c.is_alphanumeric()).collect();
            if repeating.contains(&alphanum_key) {
                if first_seen_page.get(&alphanum_key).copied() == Some(page_idx) {
                    continue;
                }
                tracing::trace!(
                    text = %normalized.chars().take(60).collect::<String>(),
                    "marking repeating short text as furniture (tier 2)"
                );
                para.is_page_furniture = true;
                para.heading_level = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::hierarchy::SegmentData;

    fn make_paragraph(font_size: f32, segment_count: usize) -> PdfParagraph {
        let segments: Vec<SegmentData> = (0..segment_count)
            .map(|i| SegmentData {
                text: format!("word{}", i),
                x: i as f32 * 50.0,
                y: 700.0,
                width: 40.0,
                height: font_size,
                font_size,
                is_bold: false,
                is_italic: false,
                is_monospace: false,
                baseline_y: 700.0,
                rotation_degrees: 0.0,
                assigned_role: None,
            })
            .collect();

        let lines = vec![super::super::types::PdfLine {
            segments,
            baseline_y: 700.0,
            dominant_font_size: font_size,
            is_bold: false,
            is_monospace: false,
        }];
        let word_count = PdfParagraph::compute_word_count("", &lines);

        PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: font_size,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        }
    }

    #[test]
    fn heading_runs_stop_at_layout_region_boundaries() {
        use crate::pdf::structure::types::{LayoutHintClass, LayoutRegionPath, LayoutRegionTag};

        let mut page = (0..4)
            .map(|index| {
                let mut paragraph = make_paragraph(18.0, 1);
                paragraph.heading_level = Some(2);
                paragraph.layout_region_path = Some(LayoutRegionPath {
                    root: LayoutRegionTag {
                        id: usize::from(index >= 2),
                        class_name: Some(LayoutHintClass::Text),
                    },
                    child: None,
                });
                paragraph
            })
            .collect::<Vec<_>>();

        demote_heading_runs(std::slice::from_mut(&mut page));
        assert!(page.iter().all(|paragraph| paragraph.heading_level == Some(2)));
    }

    #[test]
    fn test_classify_heading() {
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        let mut paragraphs = vec![make_paragraph(18.0, 3)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(1));
    }

    #[test]
    fn test_classify_body() {
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        let mut paragraphs = vec![make_paragraph(12.0, 5)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_classify_too_many_segments_for_heading() {
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        let mut paragraphs = vec![make_paragraph(18.0, 21)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_find_heading_level_empty_map() {
        let gap_info = precompute_gap_info(&[]);
        assert_eq!(find_heading_level(12.0, &[], &gap_info), None);
    }

    #[test]
    fn test_find_heading_level_single_entry() {
        let heading_map = vec![(12.0, Some(1))];
        let gap_info = precompute_gap_info(&heading_map);
        assert_eq!(find_heading_level(12.0, &heading_map, &gap_info), Some(1));
    }

    #[test]
    fn test_find_heading_level_outlier_rejected() {
        let heading_map = vec![(12.0, None), (16.0, Some(2)), (20.0, Some(1))];
        let gap_info = precompute_gap_info(&heading_map);
        assert_eq!(find_heading_level(50.0, &heading_map, &gap_info), None);
    }

    #[test]
    fn test_find_heading_level_close_match() {
        let heading_map = vec![(12.0, None), (16.0, Some(2)), (20.0, Some(1))];
        let gap_info = precompute_gap_info(&heading_map);
        assert_eq!(find_heading_level(15.5, &heading_map, &gap_info), Some(2));
    }

    #[test]
    fn test_classify_bold_short_paragraph_promoted_to_heading() {
        let heading_map = vec![(12.0, None)];
        let mut para = make_paragraph(12.0, 3);
        para.is_bold = true;
        para.lines[0].is_bold = true;
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(3));
    }

    #[test]
    fn test_classify_bold_long_paragraph_not_promoted() {
        let heading_map = vec![(12.0, None)];
        let mut para = make_paragraph(12.0, 20);
        para.is_bold = true;
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_classify_bold_list_item_not_promoted() {
        let heading_map = vec![(12.0, None)];
        let mut para = make_paragraph(12.0, 3);
        para.is_bold = true;
        para.is_list_item = true;
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_classify_few_segments_many_words_not_heading() {
        let segments: Vec<SegmentData> = (0..3)
            .map(|i| SegmentData {
                text: "one two three four five six seven".to_string(),
                x: i as f32 * 200.0,
                y: 700.0,
                width: 180.0,
                height: 18.0,
                font_size: 18.0,
                is_bold: false,
                is_italic: false,
                is_monospace: false,
                baseline_y: 700.0,
                rotation_degrees: 0.0,
                assigned_role: None,
            })
            .collect();

        let lines = vec![super::super::types::PdfLine {
            segments,
            baseline_y: 700.0,
            dominant_font_size: 18.0,
            is_bold: false,
            is_monospace: false,
        }];
        let word_count = PdfParagraph::compute_word_count("", &lines);

        let mut paragraphs = vec![PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: 18.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        }];
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    fn make_h1(font_size: f32, text: &str) -> PdfParagraph {
        let mut p = make_paragraph(font_size, 1);
        p.lines[0].segments[0].text = text.to_string();
        p.heading_level = Some(1);
        p
    }

    fn make_h1_with_text(font_size: f32, text: &str) -> PdfParagraph {
        let mut p = make_h1(font_size, text);
        p.text = text.to_string();
        p
    }

    #[test]
    fn test_merge_consecutive_h1s_same_font() {
        let mut page = vec![
            make_h1(24.0, "KAISUN HOLDINGS"),
            make_h1(24.0, "LIMITED"),
            make_paragraph(12.0, 3),
        ];
        merge_consecutive_h1s(&mut page);
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].heading_level, Some(1));
        assert_eq!(page[0].lines.len(), 2);
    }

    #[test]
    fn test_merge_h1s_text_field_synced() {
        let mut page = vec![
            make_h1_with_text(24.0, "KAISUN HOLDINGS"),
            make_h1_with_text(24.0, "LIMITED"),
        ];
        merge_consecutive_h1s(&mut page);
        assert_eq!(page.len(), 1);
        assert!(
            page[0].text.contains("KAISUN HOLDINGS"),
            "merged text must include first heading: {:?}",
            page[0].text
        );
        assert!(
            page[0].text.contains("LIMITED"),
            "merged text must include second heading: {:?}",
            page[0].text
        );
    }

    #[test]
    fn test_merge_h1s_model_codes_not_merged() {
        let mut page = vec![
            make_h1_with_text(24.0, "HR 22"),
            make_h1_with_text(24.0, "HR 28"),
            make_h1_with_text(24.0, "HR 28/24"),
            make_h1_with_text(24.0, "HR 36/30"),
        ];
        merge_consecutive_h1s(&mut page);
        assert_eq!(
            page.len(),
            4,
            "product model codes at same font size must stay as separate headings"
        );
    }

    #[test]
    fn test_merge_h1s_different_font_no_merge() {
        let mut page = vec![make_h1(24.0, "Title"), make_h1(18.0, "Subtitle")];
        merge_consecutive_h1s(&mut page);
        assert_eq!(page.len(), 2);
    }

    #[test]
    fn test_merge_h1s_long_first_line_not_merged() {
        let mut page = vec![
            make_h1_with_text(24.0, "The Quick Brown Fox Jumped"),
            make_h1_with_text(24.0, "Over"),
        ];
        merge_consecutive_h1s(&mut page);
        assert_eq!(
            page.len(),
            2,
            "first heading with >4 words must not merge with the next"
        );
        assert_eq!(page[0].text, "The Quick Brown Fox Jumped");
        assert_eq!(page[1].text, "Over");
    }

    #[test]
    fn test_merge_h1s_terminal_punctuation_not_merged() {
        for suffix in [".", "!", "?", ":"] {
            let first = format!("SECTION ONE{suffix}");
            let mut page = vec![make_h1_with_text(24.0, &first), make_h1_with_text(24.0, "INTRODUCTION")];
            merge_consecutive_h1s(&mut page);
            assert_eq!(
                page.len(),
                2,
                "heading ending with '{suffix}' must not merge: got {:?}",
                page[0].text
            );
        }
    }

    /// Create a paragraph with bbox in the top margin of a 792pt page.
    fn make_margin_body(text: &str) -> PdfParagraph {
        let mut p = make_paragraph(12.0, 1);
        p.lines[0].segments[0].text = text.to_string();
        p.block_bbox = Some((50.0, 740.0, 300.0, 760.0));
        p
    }

    /// Create a paragraph with bbox in the body (not margin) area.
    fn make_body_center(text: &str) -> PdfParagraph {
        let mut p = make_paragraph(12.0, 1);
        p.lines[0].segments[0].text = text.to_string();
        p.block_bbox = Some((50.0, 400.0, 300.0, 420.0));
        p
    }

    #[test]
    fn test_cross_page_repeating_text() {
        let page_heights = vec![792.0; 4];
        let mut pages = vec![
            vec![make_margin_body("Page 1 of 10"), make_body_center("Unique content A")],
            vec![make_margin_body("Page 1 of 10"), make_body_center("Unique content B")],
            vec![make_margin_body("Page 1 of 10"), make_body_center("Unique content C")],
            vec![make_margin_body("Page 1 of 10"), make_body_center("Unique content D")],
        ];
        mark_cross_page_repeating_text(&mut pages, &page_heights);
        assert!(!pages[0][0].is_page_furniture, "first occurrence must not be furniture");
        assert!(pages[1][0].is_page_furniture);
        assert!(pages[2][0].is_page_furniture);
        assert!(pages[3][0].is_page_furniture);
        assert!(!pages[0][1].is_page_furniture);
    }

    #[test]
    fn test_cross_page_repeating_marks_repeated_headings_as_furniture() {
        let page_heights = vec![792.0; 6];
        let mut pages = vec![];
        for _ in 0..6 {
            let mut h = make_h1(24.0, "Chapter");
            h.heading_level = Some(1);
            h.block_bbox = Some((50.0, 740.0, 300.0, 770.0));
            let mut body = make_paragraph(12.0, 3);
            body.block_bbox = Some((50.0, 400.0, 300.0, 420.0));
            pages.push(vec![h, body]);
        }
        mark_cross_page_repeating_text(&mut pages, &page_heights);
        assert!(!pages[0][0].is_page_furniture, "first occurrence must not be furniture");
        assert!(pages[1][0].is_page_furniture);
        assert!(pages[1][0].heading_level.is_none());
        assert!(pages[5][0].is_page_furniture);
    }

    #[test]
    fn test_cross_page_repeating_fuzzy_matches_iso_variants() {
        let page_heights = vec![792.0; 6];
        let mut pages = vec![
            vec![
                make_margin_body("O ISO 2021 All rights reserved"),
                make_body_center("Section content A"),
            ],
            vec![
                make_margin_body("O ISO 2021 All rights reserved"),
                make_body_center("Section content B"),
            ],
            vec![
                make_margin_body("O ISO 2021 All rights reserved"),
                make_body_center("Section content C"),
            ],
            vec![
                make_margin_body("OISO 2021Allrightsreserved"),
                make_body_center("Section content D"),
            ],
            vec![
                make_margin_body("OISO 2021Allrightsreserved"),
                make_body_center("Section content E"),
            ],
            vec![
                make_margin_body("OISO 2021Allrightsreserved"),
                make_body_center("Section content F"),
            ],
        ];
        mark_cross_page_repeating_text(&mut pages, &page_heights);
        assert!(
            !pages[0][0].is_page_furniture,
            "first occurrence of copyright notice must be preserved"
        );
        assert!(
            pages[1][0].is_page_furniture,
            "subsequent even-page copyright variant should be furniture"
        );
        assert!(
            pages[3][0].is_page_furniture,
            "odd-page copyright variant should be furniture"
        );
        assert!(!pages[0][1].is_page_furniture);
        assert!(!pages[3][1].is_page_furniture);
    }

    #[test]
    fn test_layout_text_bold_heading_font_promoted() {
        let heading_map = vec![(16.0, Some(2)), (12.0, None)];
        let mut para = make_paragraph(16.0, 3);
        para.is_bold = true;
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_layout_text_non_bold_heading_font_not_promoted() {
        let heading_map = vec![(16.0, Some(2)), (12.0, None)];
        let mut para = make_paragraph(16.0, 3);
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_layout_text_bold_body_font_not_promoted_pass1() {
        let heading_map = vec![(16.0, Some(2)), (12.0, None)];
        let mut para = make_paragraph(12.0, 3);
        para.is_bold = true;
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_layout_text_bold_larger_font_promoted_pass2() {
        let heading_map = vec![(12.0, None)];
        let mut para = make_paragraph(14.0, 3);
        para.is_bold = true;
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(3));
    }

    #[test]
    fn test_layout_text_bold_much_larger_font_promoted_h2() {
        let heading_map = vec![(12.0, None)];
        let mut para = make_paragraph(15.0, 3);
        para.is_bold = true;
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_infer_bold_heading_level_large_ratio_h2() {
        assert_eq!(infer_bold_heading_level(15.0, 12.0, "Methods"), 2);
    }

    #[test]
    fn test_infer_bold_heading_level_body_size_h3() {
        assert_eq!(infer_bold_heading_level(12.0, 12.0, "Methods"), 3);
    }

    #[test]
    fn test_infer_bold_heading_level_numbered_section() {
        assert_eq!(infer_bold_heading_level(12.0, 12.0, "3 Processing pipeline"), 2);
        assert_eq!(infer_bold_heading_level(12.0, 12.0, "3.2 AI models"), 3);
        assert_eq!(infer_bold_heading_level(12.0, 12.0, "3.2.1 Details"), 4);
    }

    #[test]
    fn test_infer_section_level_numeric() {
        assert_eq!(infer_section_level("1 Introduction"), 2);
        assert_eq!(infer_section_level("3.2 AI models"), 3);
        assert_eq!(infer_section_level("3.2.1 Details"), 4);
    }

    #[test]
    fn test_infer_section_level_roman() {
        assert_eq!(infer_section_level("I. INTRODUCTION"), 2);
        assert_eq!(infer_section_level("IV RESULTS"), 2);
    }

    #[test]
    fn test_infer_section_level_alpha() {
        assert_eq!(infer_section_level("A. Proofs"), 2);
        assert_eq!(infer_section_level("A.1 Sub-section"), 3);
    }

    #[test]
    fn test_infer_section_level_no_number() {
        assert_eq!(infer_section_level("Layout Analysis Model"), 2);
    }

    #[test]
    fn structure_sal_annotations_are_not_headings() {
        for annotation in [
            "__in",
            "__in_opt",
            "__out",
            "__out_opt",
            "__inout",
            "__inout_bcount_full(n)",
            "__deref__out_bcount(n)",
        ] {
            let mut para = make_text_paragraph(12.0, annotation, false);
            para.heading_level = Some(3);
            demote_structure_annotation_headings(std::slice::from_mut(&mut para));
            assert_eq!(para.heading_level, None, "annotation {annotation} remained a heading");
        }
    }

    #[test]
    fn structure_identifier_headings_are_preserved() {
        for identifier in ["__init__", "__input", "__in_section", "API_V2"] {
            let mut para = make_text_paragraph(18.0, identifier, true);
            para.heading_level = Some(2);
            demote_structure_annotation_headings(std::slice::from_mut(&mut para));
            assert_eq!(para.heading_level, Some(2), "identifier {identifier} was demoted");
        }
    }

    #[test]
    fn structure_sal_heading_with_strong_layout_evidence_is_preserved() {
        let mut para = make_text_paragraph(18.0, "__in", true);
        para.heading_level = Some(2);
        para.layout_class = Some(LayoutHintClass::SectionHeader);
        demote_structure_annotation_headings(std::slice::from_mut(&mut para));
        assert_eq!(para.heading_level, Some(2));
    }

    #[test]
    fn structure_sal_code_block_is_preserved() {
        let mut para = make_text_paragraph(12.0, "__out", false);
        para.heading_level = Some(3);
        para.is_code_block = true;
        demote_structure_annotation_headings(std::slice::from_mut(&mut para));
        assert_eq!(para.heading_level, Some(3));
    }

    /// Helper to create a paragraph with specific text.
    fn make_text_paragraph(font_size: f32, text: &str, is_bold: bool) -> PdfParagraph {
        let segments = vec![SegmentData {
            text: text.to_string(),
            x: 0.0,
            y: 700.0,
            width: 200.0,
            height: font_size,
            font_size,
            is_bold,
            is_italic: false,
            is_monospace: false,
            baseline_y: 700.0,
            rotation_degrees: 0.0,
            assigned_role: None,
        }];
        let lines = vec![super::super::types::PdfLine {
            segments,
            baseline_y: 700.0,
            dominant_font_size: font_size,
            is_bold,
            is_monospace: false,
        }];
        let word_count = PdfParagraph::compute_word_count("", &lines);

        PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: font_size,
            heading_level: None,
            is_bold,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        }
    }

    fn paragraph_at_indent(text: &str, left: f32) -> PdfParagraph {
        let mut paragraph = make_text_paragraph(12.0, text, false);
        paragraph.block_bbox = Some((left, 0.0, left + 200.0, 12.0));
        paragraph
    }

    #[test]
    fn indentation_does_not_promote_author_byline() {
        let mut paragraphs = vec![
            paragraph_at_indent("Header", 0.0),
            paragraph_at_indent("O. Sanni, A.P.I. Popoola / Data in Brief", 20.0),
            paragraph_at_indent("Footer", 0.0),
        ];

        detect_indentation_based_lists(&mut paragraphs);

        assert!(!paragraphs[1].is_list_item);
    }

    #[test]
    fn indentation_does_not_promote_numeric_continuations() {
        let mut paragraphs = vec![
            paragraph_at_indent("Header", 0.0),
            paragraph_at_indent("457", 20.0),
            paragraph_at_indent("8 show the remaining figures", 20.0),
            paragraph_at_indent("Footer", 0.0),
        ];

        detect_indentation_based_lists(&mut paragraphs);

        assert!(!paragraphs[1].is_list_item);
        assert!(!paragraphs[2].is_list_item);
    }

    #[test]
    fn numeric_prose_continuation_gate_is_narrow() {
        assert!(is_numeric_prose_continuation("457"));
        assert!(is_numeric_prose_continuation("8 show the remaining figures"));
        assert!(!is_numeric_prose_continuation("8 Results"));
        assert!(!is_numeric_prose_continuation("first item"));
    }

    #[test]
    fn test_section_numbering_promotes_non_bold_heading() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "3 Processing pipeline", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_section_numbering_blocked_by_layout_text() {
        let heading_map = vec![(12.0, None)];
        let mut para = make_text_paragraph(12.0, "3 Processing pipeline", false);
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_section_numbering_promotes_bold_at_body_size() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "3 Processing pipeline", true)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_section_numbering_subsection_bold() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "3.2 AI models", true)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(3));
    }

    #[test]
    fn test_bold_heading_with_heading_cluster_keeps_cluster_level() {
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(18.0, "Title", true)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(1));
    }

    #[test]
    fn test_formula_detection_math_symbols() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "∑ x∈S f(x) ≤ ∞", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert!(paragraphs[0].is_formula, "should detect formula with math symbols");
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_formula_detection_greek_letters() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "α + β = γ", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert!(paragraphs[0].is_formula, "should detect formula with Greek letters");
    }

    #[test]
    fn test_formula_detection_not_regular_text() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(
            12.0,
            "This is a normal paragraph with regular text.",
            false,
        )];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert!(!paragraphs[0].is_formula, "normal text should not be a formula");
    }

    #[test]
    fn test_formula_detection_skips_headings() {
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(18.0, "∑ Results", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(1));
    }

    #[test]
    fn test_formula_detection_ascii_operators() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "a2 + 8 = 12", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert!(paragraphs[0].is_formula, "ASCII equation should be a formula");
    }

    #[test]
    fn test_formula_detection_superscript_exponent() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "a\u{00B2} + 8 = 12", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert!(
            paragraphs[0].is_formula,
            "superscript exponent equation should be a formula"
        );
    }

    #[test]
    fn test_formula_detection_prose_with_equals_not_flagged() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(
            12.0,
            "The default setting is size = large for all new documents created here.",
            false,
        )];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert!(!paragraphs[0].is_formula, "prose with a single = must stay prose");
    }

    #[test]
    fn test_formula_detection_high_density() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "x→y", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert!(
            paragraphs[0].is_formula,
            "high density of math chars should trigger formula"
        );
    }

    #[test]
    fn test_is_math_character() {
        assert!(is_math_character('∑'));
        assert!(is_math_character('∫'));
        assert!(is_math_character('α'));
        assert!(is_math_character('Ω'));
        assert!(is_math_character('≤'));
        assert!(is_math_character('∞'));
        assert!(!is_math_character('a'));
        assert!(!is_math_character('1'));
        // '+', '=' and '^' are math characters: `a2 + 8 = 12` must classify as a
        // formula (test_formula_detection_ascii_operators). ~keep
        assert!(is_math_character('+'));
        assert!(is_math_character('='));
        assert!(is_math_character('^'));
    }

    #[test]
    fn test_is_greek_letter() {
        assert!(is_greek_letter('α'));
        assert!(is_greek_letter('ω'));
        assert!(is_greek_letter('Α'));
        assert!(is_greek_letter('Ω'));
        assert!(!is_greek_letter('a'));
        assert!(!is_greek_letter('Z'));
    }

    #[test]
    fn test_rescue_pass_promotes_large_font_short_paragraph() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(15.0, "Results and Discussion", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(3));
    }

    #[test]
    fn test_rescue_pass_h1_for_very_large_font() {
        let heading_map = vec![(10.0, None)];
        let mut paragraphs = vec![make_text_paragraph(17.0, "Document Title", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(1));
    }

    #[test]
    fn test_rescue_pass_h3_for_slightly_larger_font() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(13.0, "Methods", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(3));
    }

    #[test]
    fn test_rescue_pass_skips_long_paragraphs() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(
            15.0,
            "This is a very long paragraph that has way too many words",
            false,
        )];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_rescue_pass_skips_period_ending() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(15.0, "Some text here.", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_rescue_pass_skips_list_items() {
        let heading_map = vec![(12.0, None)];
        let mut para = make_text_paragraph(15.0, "Item One", false);
        para.is_list_item = true;
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_rescue_pass_skips_body_font_size() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "Methods", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_rescue_pass_skips_lowercase_start() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(15.0, "is necessary to address", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[0].heading_level, None,
            "lowercase-starting fragment should not be promoted"
        );
    }

    #[test]
    fn test_rescue_pass_skips_continuation_word_the() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(15.0, "The unsafe condition", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[0].heading_level, None,
            "continuation word 'The' should not be promoted"
        );
    }

    #[test]
    fn test_rescue_pass_skips_continuation_word_and() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(15.0, "And furthermore", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[0].heading_level, None,
            "continuation word 'And' should not be promoted"
        );
    }

    #[test]
    fn test_rescue_pass_allows_proper_heading() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(15.0, "Results and Discussion", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[0].heading_level,
            Some(3),
            "proper heading should still be promoted"
        );
    }

    #[test]
    fn test_demote_continuation_heading_after_unterminated_paragraph() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![
            make_text_paragraph(12.0, "the regulation requires that all operators", false),
            make_text_paragraph(15.0, "Safety Procedures", false),
        ];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[1].heading_level, None,
            "heading after unterminated paragraph should be demoted"
        );
    }

    #[test]
    fn test_keep_heading_after_terminated_paragraph() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![
            make_text_paragraph(12.0, "The previous section covered the basics.", false),
            make_text_paragraph(15.0, "Safety Procedures", false),
        ];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[1].heading_level,
            Some(3),
            "heading after terminated paragraph should stay"
        );
    }

    #[test]
    fn test_keep_heading_after_another_heading() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![
            make_text_paragraph(15.0, "Chapter One", false),
            make_text_paragraph(14.0, "Introduction", false),
        ];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert!(
            paragraphs[0].heading_level.is_some(),
            "first heading should be promoted"
        );
        assert!(
            paragraphs[1].heading_level.is_some(),
            "heading after heading should not be demoted by continuation check"
        );
    }

    #[test]
    fn test_starts_with_lowercase_or_continuation() {
        assert!(starts_with_lowercase_or_continuation("is necessary"));
        assert!(starts_with_lowercase_or_continuation("the quick fox"));
        assert!(starts_with_lowercase_or_continuation("The quick fox"));
        assert!(starts_with_lowercase_or_continuation("And furthermore"));
        assert!(starts_with_lowercase_or_continuation("But however"));
        assert!(starts_with_lowercase_or_continuation("Which means"));
        assert!(!starts_with_lowercase_or_continuation("Results"));
        assert!(!starts_with_lowercase_or_continuation("Safety Procedures"));
        assert!(!starts_with_lowercase_or_continuation("3 Methods"));
    }

    /// A 2-paragraph document is too sparse for the "no heading anywhere"
    /// fallback to trust a font-size difference: a larger opening line in a
    /// tiny document is as likely to be display prose (see `hello_structure.pdf`,
    /// `issue-987-test.pdf`) as an actual title. This replaces the previous
    /// `test_refine_promotes_first_largest_font_paragraph_to_h1`, which asserted
    /// promotion for this same 2-paragraph shape; that assertion encoded the
    /// over-promotion bug this sparsity gate fixes.
    #[test]
    fn test_refine_does_not_promote_sparse_two_paragraph_doc() {
        let mut pages = vec![vec![
            make_text_paragraph(18.0, "Annual Report", false),
            make_text_paragraph(12.0, "Some body text here for the document", false),
        ]];
        refine_heading_hierarchy(&mut pages);
        assert_eq!(
            pages[0][0].heading_level, None,
            "a 2-paragraph document must not force-promote its larger first line to h1"
        );
    }

    /// At and above the sparsity floor, a first paragraph that is both the
    /// largest font on the page and meaningfully larger than the rest of the
    /// document's text is still promoted to h1.
    #[test]
    fn test_refine_promotes_first_largest_font_paragraph_to_h1_at_floor() {
        let mut pages = vec![vec![
            make_text_paragraph(18.0, "Annual Report", false),
            make_text_paragraph(12.0, "Some body text here for the document", false),
            make_text_paragraph(12.0, "More body text in a second paragraph", false),
            make_text_paragraph(12.0, "Yet another paragraph of body text", false),
            make_text_paragraph(12.0, "A final paragraph rounding out the document", false),
        ]];
        refine_heading_hierarchy(&mut pages);
        assert_eq!(
            pages[0][0].heading_level,
            Some(1),
            "at the sparsity floor a clearly larger title line must still be promoted"
        );
    }

    #[test]
    fn refine_heading_hierarchy_should_promote_single_leading_h2_to_h1() {
        let mut heading = make_text_paragraph(24.0, "Hello World", false);
        heading.heading_level = Some(2);
        let body = make_text_paragraph(12.0, "I'll be back shortly!", false);
        let mut pages = vec![vec![heading, body]];

        refine_heading_hierarchy(&mut pages);

        assert_eq!(pages[0][0].heading_level, Some(1));
    }

    #[test]
    fn refine_heading_hierarchy_should_preserve_repeated_cross_page_h2_tier() {
        let mut first_heading = make_text_paragraph(24.0, "Hello World", false);
        first_heading.heading_level = Some(2);
        let mut second_heading = make_text_paragraph(24.0, "Goodbye Cruel World...", false);
        second_heading.heading_level = Some(2);
        let mut pages = vec![
            vec![first_heading, make_text_paragraph(12.0, "First page body text.", false)],
            vec![
                second_heading,
                make_text_paragraph(12.0, "Second page body text.", false),
            ],
        ];

        refine_heading_hierarchy(&mut pages);

        assert_eq!(
            [pages[0][0].heading_level, pages[1][0].heading_level],
            [Some(2), Some(2)]
        );
    }

    #[test]
    fn test_refine_preserves_changelog_h2_and_promotes_adjacent_version_to_h3() {
        let mut title = make_text_paragraph(24.0, "Project Guide", false);
        title.heading_level = Some(1);
        let mut changelog = make_text_paragraph(16.0, "Recent Change Log", false);
        changelog.heading_level = Some(2);
        changelog.block_bbox = Some((50.0, 700.0, 210.0, 718.0));
        let mut version = make_text_paragraph(12.0, "v0.9.1", true);
        version.block_bbox = Some((50.5, 680.0, 105.0, 692.0));
        let body = make_text_paragraph(12.0, "Maintenance details follow.", true);
        let mut pages = vec![vec![title, changelog, version, body]];

        refine_heading_hierarchy(&mut pages);

        assert_eq!(pages[0][1].heading_level, Some(2));
        assert_eq!(pages[0][2].heading_level, Some(3));
        assert_eq!(pages[0][3].heading_level, None);
    }

    #[test]
    fn test_refine_splits_combined_changelog_and_version_heading() {
        let mut title = make_text_paragraph(24.0, "Project Guide", false);
        title.heading_level = Some(1);
        let mut combined = make_text_paragraph(16.0, "Recent Change Log v0.9.1", true);
        combined.text = "Recent Change Log v0.9.1".to_string();
        combined.block_bbox = Some((50.0, 664.0, 210.0, 696.0));
        let mut pages = vec![vec![title, combined]];

        refine_heading_hierarchy(&mut pages);

        assert_eq!(pages[0].len(), 3);
        assert_eq!(pages[0][1].text, "Recent Change Log");
        assert_eq!(pages[0][1].heading_level, Some(2));
        assert_eq!(pages[0][2].text, "v0.9.1");
        assert_eq!(pages[0][2].heading_level, Some(3));
        assert!(!pages[0][2].is_list_item);
        assert!(!pages[0][2].is_code_block);
        assert!(!pages[0][2].is_formula);
        assert!(!pages[0][2].is_page_furniture);
        assert_eq!(pages[0][1].block_bbox, Some((50.0, 680.0, 210.0, 696.0)));
        assert_eq!(pages[0][2].block_bbox, Some((50.0, 664.0, 210.0, 680.0)));
    }

    #[test]
    fn test_refine_splits_combined_changelog_without_preexisting_h1() {
        let mut combined = make_text_paragraph(16.0, "Recent Change Log v0.9.1", true);
        combined.text = "Recent Change Log v0.9.1".to_string();
        combined.block_bbox = Some((50.0, 664.0, 210.0, 696.0));
        let mut pages = vec![vec![
            combined,
            make_text_paragraph(12.0, "First body paragraph.", false),
            make_text_paragraph(12.0, "Second body paragraph.", false),
            make_text_paragraph(12.0, "Third body paragraph.", false),
            make_text_paragraph(12.0, "Fourth body paragraph.", false),
        ]];

        refine_heading_hierarchy(&mut pages);

        assert_eq!(pages[0][0].heading_level, Some(2));
        assert_eq!(pages[0][1].heading_level, Some(3));
        assert!(
            pages[0].iter().all(|paragraph| paragraph.heading_level != Some(1)),
            "a synthetic changelog hierarchy must not create an unrelated h1"
        );
    }

    #[test]
    fn test_combined_changelog_split_rejects_ordinary_one_line_heading() {
        let mut combined = make_text_paragraph(16.0, "Recent Change Log v0.9.1", true);
        combined.text = "Recent Change Log v0.9.1".to_string();
        combined.block_bbox = Some((50.0, 680.0, 310.0, 696.0));
        let mut pages = vec![vec![combined]];

        split_combined_changelog_version_headings(&mut pages);

        assert_eq!(pages[0].len(), 1);
        assert_eq!(pages[0][0].text, "Recent Change Log v0.9.1");
        assert_eq!(pages[0][0].heading_level, None);
    }

    #[test]
    fn test_combined_changelog_split_accepts_distinct_line_baselines() {
        let mut combined = make_text_paragraph(16.0, "Recent Change Log v0.9.1", true);
        combined.text = "Recent Change Log v0.9.1".to_string();
        let mut version_line = combined.lines[0].clone();
        version_line.baseline_y = 680.0;
        version_line.segments[0].y = 680.0;
        version_line.segments[0].baseline_y = 680.0;
        combined.lines.push(version_line);
        let mut pages = vec![vec![combined]];

        split_combined_changelog_version_headings(&mut pages);

        assert_eq!(pages[0].len(), 2);
        assert_eq!(pages[0][0].heading_level, Some(2));
        assert_eq!(pages[0][1].heading_level, Some(3));
        assert!(is_changelog_version_pair(&pages[0][0], &pages[0][1]));
    }

    #[test]
    fn test_combined_changelog_split_requires_geometry_and_plain_heading_state() {
        let make_combined = || {
            let mut paragraph = make_text_paragraph(16.0, "Recent Change Log v0.9.1", true);
            paragraph.text = "Recent Change Log v0.9.1".to_string();
            paragraph
        };

        let mut without_geometry = vec![vec![make_combined()]];
        split_combined_changelog_version_headings(&mut without_geometry);
        assert_eq!(without_geometry[0].len(), 1);

        for invalid_state in 0..4 {
            let mut paragraph = make_combined();
            paragraph.block_bbox = Some((50.0, 680.0, 210.0, 696.0));
            match invalid_state {
                0 => paragraph.is_list_item = true,
                1 => paragraph.is_code_block = true,
                2 => paragraph.is_formula = true,
                3 => paragraph.is_page_furniture = true,
                _ => unreachable!(),
            }
            let mut pages = vec![vec![paragraph]];
            split_combined_changelog_version_headings(&mut pages);
            assert_eq!(pages[0].len(), 1);
        }
    }

    #[test]
    fn test_standalone_recent_change_log_title_can_be_promoted_to_h1() {
        let mut pages = vec![vec![
            make_text_paragraph(18.0, "Recent Change Log", false),
            make_text_paragraph(12.0, "First body paragraph.", false),
            make_text_paragraph(12.0, "Second body paragraph.", false),
            make_text_paragraph(12.0, "Third body paragraph.", false),
            make_text_paragraph(12.0, "Fourth body paragraph.", false),
        ]];

        refine_heading_hierarchy(&mut pages);

        assert_eq!(pages[0][0].heading_level, Some(1));
    }

    #[test]
    fn test_refine_does_not_promote_distant_or_unaligned_changelog_versions() {
        let mut title = make_text_paragraph(24.0, "Project Guide", false);
        title.heading_level = Some(1);
        let mut changelog = make_text_paragraph(16.0, "Recent Change Log", false);
        changelog.heading_level = Some(2);
        changelog.block_bbox = Some((50.0, 700.0, 210.0, 718.0));
        let mut distant_version = make_text_paragraph(12.0, "1.2.3", true);
        distant_version.block_bbox = Some((50.0, 600.0, 100.0, 612.0));
        let mut second_changelog = changelog.clone();
        second_changelog.block_bbox = Some((50.0, 700.0, 210.0, 718.0));
        let mut unaligned_version = make_text_paragraph(12.0, "v2.0", true);
        unaligned_version.block_bbox = Some((100.0, 680.0, 150.0, 692.0));
        let mut pages = vec![
            vec![title, changelog, distant_version],
            vec![second_changelog, unaligned_version],
        ];

        refine_heading_hierarchy(&mut pages);

        assert_eq!(pages[0][1].heading_level, Some(2));
        assert_eq!(pages[0][2].heading_level, None);
        assert_eq!(pages[1][0].heading_level, Some(2));
        assert_eq!(pages[1][1].heading_level, None);
    }

    #[test]
    fn test_semantic_version_match_is_bounded_to_two_or_three_numeric_parts() {
        assert!(is_semantic_version("0.9"));
        assert!(is_semantic_version("v1.2.3"));
        assert!(!is_semantic_version("1"));
        assert!(!is_semantic_version("1.2.3.4"));
        assert!(!is_semantic_version("version 1.2"));
    }

    #[test]
    fn test_is_all_caps_text_positive() {
        assert!(is_all_caps_text("DEPARTMENT OF TRANSPORTATION"));
        assert!(is_all_caps_text("AGENCY:"));
        assert!(is_all_caps_text("SUMMARY:"));
        assert!(is_all_caps_text("ACTION:"));
    }

    #[test]
    fn test_is_all_caps_text_negative() {
        assert!(!is_all_caps_text("Department of Transportation"));
        assert!(!is_all_caps_text("Some regular text here"));
        assert!(!is_all_caps_text("a"));
    }

    #[test]
    fn test_all_caps_short_promoted_to_h2() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "EXECUTIVE SUMMARY", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[0].heading_level,
            Some(2),
            "short ALL-CAPS should be promoted to H2"
        );
    }

    #[test]
    fn test_all_caps_longer_promoted_to_h3() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(
            12.0,
            "DEPARTMENT OF TRANSPORTATION FEDERAL AVIATION ADMINISTRATION",
            false,
        )];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[0].heading_level,
            Some(3),
            "longer ALL-CAPS should be promoted to H3"
        );
    }

    #[test]
    fn test_all_caps_with_colon_promoted() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "AGENCY:", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[0].heading_level,
            Some(2),
            "ALL-CAPS colon-ending label should be promoted"
        );
    }

    #[test]
    fn test_all_caps_single_word_no_colon_not_promoted() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "USA", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[0].heading_level, None,
            "single-word abbreviation should not be promoted"
        );
    }

    #[test]
    fn test_all_caps_too_long_not_promoted() {
        let heading_map = vec![(12.0, None)];
        let text = "THIS IS A VERY LONG ALL CAPS TEXT THAT HAS WAY TOO MANY WORDS TO BE A HEADING IN ANY DOCUMENT";
        let mut paragraphs = vec![make_text_paragraph(12.0, text, false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[0].heading_level, None,
            "ALL-CAPS text >15 words should not be promoted"
        );
    }

    #[test]
    fn test_all_caps_list_item_not_promoted() {
        let heading_map = vec![(12.0, None)];
        let mut para = make_text_paragraph(12.0, "ITEM ONE", false);
        para.is_list_item = true;
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[0].heading_level, None,
            "ALL-CAPS list item should not be promoted"
        );
    }

    #[test]
    fn test_all_caps_larger_font_gets_higher_level() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(18.0, "TITLE", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert!(
            paragraphs[0].heading_level.is_some(),
            "large ALL-CAPS should be promoted"
        );
    }

    #[test]
    fn test_bold_all_caps_colon_promoted() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "ACTION:", true)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert!(
            paragraphs[0].heading_level.is_some(),
            "bold ALL-CAPS with colon should be promoted"
        );
    }

    #[test]
    fn test_bold_mixed_case_colon_not_promoted() {
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "Agency:", true)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[0].heading_level, None,
            "bold mixed-case with colon should not be promoted"
        );
    }

    /// A title block that also appears as a running header on subsequent pages must
    /// NOT be marked as furniture on the first page where it occurs.
    /// Regression test for #917: MCID-tagged title dropped in markdown/html output.
    #[test]
    fn test_cross_page_repeating_text_preserves_first_occurrence() {
        let page_heights = vec![792.0_f32; 6];
        let title = "Analysis of Thermodynamic Properties";
        let mut pages: Vec<Vec<PdfParagraph>> = (0..6)
            .map(|_| vec![make_margin_body(title), make_body_center("body text here")])
            .collect();
        mark_cross_page_repeating_text(&mut pages, &page_heights);

        assert!(
            !pages[0][0].is_page_furniture,
            "first occurrence of title on page 0 must be preserved (issue #917)"
        );
        for (i, page) in pages.iter().enumerate().take(6).skip(1) {
            assert!(page[0].is_page_furniture, "page {i} running header should be furniture");
        }
        for (i, page) in pages.iter().enumerate().take(6) {
            assert!(
                !page[1].is_page_furniture,
                "body text on page {i} must not be furniture"
            );
        }
    }

    /// Short-text tier-2 detection must also exempt the first occurrence of repeating text.
    /// Regression test for #917: short title paragraphs dropped in markdown/html output.
    #[test]
    fn test_cross_page_repeating_short_text_preserves_first_occurrence() {
        let title = "Xberg Conference Proceedings 2024";
        let mut pages: Vec<Vec<PdfParagraph>> = (0..5)
            .map(|_| {
                let mut header = make_margin_body(title);
                header.block_bbox = None;
                vec![header, make_body_center("section body")]
            })
            .collect();
        mark_cross_page_repeating_short_text(&mut pages);

        assert!(
            !pages[0][0].is_page_furniture,
            "first occurrence of short repeating title must be preserved (issue #917)"
        );
        for (i, page) in pages.iter().enumerate().take(5).skip(1) {
            assert!(
                page[0].is_page_furniture,
                "page {i} short repeating text should be furniture"
            );
        }
    }

    #[test]
    fn test_cross_page_repeating_short_text_preserves_positioned_body_fields() {
        let mut pages: Vec<Vec<PdfParagraph>> = (0..5).map(|_| vec![make_body_center("Report Status")]).collect();

        mark_cross_page_repeating_short_text(&mut pages);

        assert!(pages.iter().all(|page| !page[0].is_page_furniture));
    }

    fn make_line_paragraph(line_count: usize, is_monospace: bool) -> PdfParagraph {
        let lines: Vec<super::super::types::PdfLine> = (0..line_count)
            .map(|i| super::super::types::PdfLine {
                segments: vec![SegmentData {
                    text: format!("line{i}"),
                    x: 0.0,
                    y: 700.0 - i as f32 * 12.0,
                    width: 40.0,
                    height: 10.0,
                    font_size: 10.0,
                    is_bold: false,
                    is_italic: false,
                    is_monospace,
                    baseline_y: 700.0 - i as f32 * 12.0,
                    rotation_degrees: 0.0,
                    assigned_role: None,
                }],
                baseline_y: 700.0 - i as f32 * 12.0,
                dominant_font_size: 10.0,
                is_bold: false,
                is_monospace,
            })
            .collect();
        let word_count = PdfParagraph::compute_word_count("", &lines);

        PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: 10.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        }
    }

    #[test]
    fn detect_monospace_code_blocks_fences_a_lone_multi_line_paragraph() {
        // Regression test for GH#1557: a standalone 2-line monospace paragraph (no
        // consecutive monospace neighbor) must still be recognized as a code block. ~keep
        let mut paragraphs = vec![make_line_paragraph(2, true), make_line_paragraph(3, false)];

        detect_monospace_code_blocks(&mut paragraphs);

        assert!(
            paragraphs[0].is_code_block,
            "a standalone 2-line all-monospace paragraph must be fenced as code"
        );
        assert_eq!(paragraphs[0].layout_class, Some(LayoutHintClass::Code));
        assert!(
            !paragraphs[1].is_code_block,
            "an ordinary multi-line prose paragraph must not be fenced as code"
        );
    }

    #[test]
    fn detect_monospace_code_blocks_still_merges_one_line_per_paragraph_listings() {
        // Pre-existing behavior must be unaffected: a code listing split into
        // one-line-per-paragraph still merges across consecutive monospace paragraphs. ~keep
        let mut paragraphs = vec![
            make_line_paragraph(1, true),
            make_line_paragraph(1, true),
            make_line_paragraph(1, true),
            make_line_paragraph(1, false),
        ];

        detect_monospace_code_blocks(&mut paragraphs);

        assert!(paragraphs[0].is_code_block);
        assert!(paragraphs[1].is_code_block);
        assert!(paragraphs[2].is_code_block);
        assert!(!paragraphs[3].is_code_block);
    }

    #[test]
    fn detect_monospace_code_blocks_renders_singleton_with_original_lines() {
        let mut paragraph = make_line_paragraph(3, true);
        paragraph.lines[0].segments[0].text = "import java.net.URI;".to_string();
        paragraph.lines[1].segments[0].text = "public class Example {".to_string();
        paragraph.lines[2].segments[0].text = "}".to_string();
        let mut paragraphs = vec![paragraph];

        detect_monospace_code_blocks(&mut paragraphs);

        assert!(
            paragraphs[0].is_code_block,
            "a singleton multi-line listing must be fenced"
        );
        let document =
            super::super::assembly::assemble_internal_document(vec![paragraphs], &[], None, &[], &Default::default());
        let markdown = crate::rendering::render_markdown(&document);
        assert_eq!(
            markdown.trim(),
            "```\nimport java.net.URI;\npublic class Example {\n}\n```",
            "assembly and Markdown rendering must preserve the code listing's physical lines"
        );
    }
}

#[cfg(test)]
mod numbered_section_heading_tests {
    use super::{ends_with_sentence_period, is_numbered_section_heading};

    #[test]
    fn multilevel_numbers_are_headings() {
        assert!(is_numbered_section_heading("3.2 Methods"));
        assert!(is_numbered_section_heading("3.2.1 Details"));
        assert!(is_numbered_section_heading("3.2. Methods"));
    }

    #[test]
    fn roman_markers_are_headings() {
        assert!(is_numbered_section_heading("IV. Results"));
        assert!(is_numbered_section_heading("II) Scope"));
        assert!(is_numbered_section_heading("I INTRODUCTION"));
    }

    #[test]
    fn single_number_with_all_caps_remainder_is_heading() {
        assert!(is_numbered_section_heading("1. INTRODUCTION"));
        assert!(is_numbered_section_heading("2) RELATED WORK"));
    }

    #[test]
    fn numbered_list_items_are_not_headings() {
        assert!(!is_numbered_section_heading("1. First point"));
        assert!(!is_numbered_section_heading("1. Énumération 1"));
        assert!(!is_numbered_section_heading("12) apples and oranges"));
        assert!(!is_numbered_section_heading("1.\nÉnumération 1"));
    }

    /// GH#1608: a heading that puts a keyword in front of its number was invisible
    /// to this predicate, which is the only boundary signal the paragraph grouper
    /// has for a heading sharing font, weight and spacing with its neighbour.
    #[test]
    fn keyword_numbered_section_headings_are_headings() {
        assert!(is_numbered_section_heading(
            "ARTIKEL 1. TOEPASSELIJKHEID VAN DE INKOOPVOORWAARDEN"
        ));
        assert!(is_numbered_section_heading("Appendix 1 PRODUCT LIST"));
        assert!(is_numbered_section_heading("Annex III SCOPE OF THE WORKS"));
        // The enumeration is a letter, which the bare roman/arabic scan cannot read.
        assert!(is_numbered_section_heading("Exhibit A PRODUCT LIST"));
        // A single-level number with a MIXED-CASE tail, which a bare enumerator rejects.
        assert!(is_numbered_section_heading("Appendix 1 Product list"));
        assert!(is_numbered_section_heading("Chapter 1"));
        assert!(is_numbered_section_heading("Artículo 5 CONDICIONES"));
    }

    /// The guard the widening must not breach: the same keyword and the same
    /// enumerator shape occur in ordinary prose, and only the case of the word
    /// after the enumerator separates them. GH#1608 page 8.
    #[test]
    fn prose_opening_with_a_keyword_and_a_number_is_not_a_heading() {
        assert!(!is_numbered_section_heading("Artikel 12 van de wet is van toepassing."));
        assert!(!is_numbered_section_heading("Article 7 of the contract applies here"));
        assert!(!is_numbered_section_heading("Bijlage bij de overeenkomst"));
        // `7a` is not a terminated enumerator.
        assert!(!is_numbered_section_heading("Article 7a IS NOT ENUMERATED"));
    }

    /// `MIN_SECTION_KEYWORD_CHARS` is load-bearing, not decoration: a two-letter
    /// preposition in front of a number and an all-caps tail satisfies every
    /// other term of the keyword arm.
    #[test]
    fn a_short_word_before_a_number_is_not_a_section_keyword() {
        assert!(!is_numbered_section_heading("Op 3 MAART"));
        assert!(!is_numbered_section_heading("In 5 STAPPEN"));
    }

    #[test]
    fn non_numbered_text_is_not_a_heading_marker() {
        assert!(!is_numbered_section_heading("Introduction"));
        assert!(!is_numbered_section_heading(""));
        assert!(!is_numbered_section_heading("1."));
    }

    #[test]
    fn sentence_period_detected_but_ellipsis_is_not() {
        // A genuine sentence-terminating period disqualifies a heading. ~keep
        assert!(ends_with_sentence_period("This is a sentence."));
        assert!(ends_with_sentence_period("Ends here.  "));
        // A trailing ellipsis is a truncation marker, not a terminator. ~keep
        assert!(!ends_with_sentence_period("Impaired Glucose Tolerance ..."));
        assert!(!ends_with_sentence_period("Thermal Comfort As Related To …"));
        assert!(!ends_with_sentence_period("No period at all"));
    }
}
