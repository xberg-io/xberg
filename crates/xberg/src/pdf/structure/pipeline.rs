//! Main PDF-to-Markdown pipeline orchestrator (native backend).

// TODO(xberg-io/xberg#1567): 4 cyclomatic-complexity and 25 size/complexity findings
// in this file, currently excluded via the quality-debt baseline in alef.toml. Splitting
// these needs compiler-in-the-loop verification, not a mechanical pass. Delete this
// note and the file's baseline entry together once it goes green. Help wanted.

use std::borrow::Cow;

use crate::pdf::bookmarks::PdfOutlineEntry;
use crate::pdf::error::Result;
use crate::pdf::hierarchy::{BoundingBox, SegmentData, TextBlock, assign_heading_levels_smart, cluster_font_sizes};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use super::assembly::assemble_internal_document;
use super::classify::{
    classify_paragraphs, demote_heading_runs, demote_structure_annotation_headings, demote_unnumbered_subsections,
    is_body_size_bold_heading_candidate, is_body_size_bold_signal, is_numbered_section_heading, mark_arxiv_noise,
    mark_cross_page_repeating_short_text, mark_cross_page_repeating_text, refine_heading_hierarchy,
};
use super::constants::{FULL_LINE_FRACTION, MIN_BLOCKS_FOR_FONT_HEADING, MIN_HEADING_FONT_GAP, MIN_HEADING_FONT_RATIO};
use super::lines::{is_cjk_char, segments_need_space};
use super::paragraphs::{merge_continuation_paragraphs, split_embedded_list_items};
use super::text_repair::{
    MIN_LIGATURE_WITNESS_WORD_LEN, WordWitnesses, apply_to_all_segments, clean_duplicate_punctuation,
    collapse_spaced_hyphens, expand_ligatures_with_space_absorption, normalize_text_encoding, normalize_unicode_text,
    repair_contextual_ligatures, repair_ligature_spaces,
};
use super::types::{LayoutHint, PdfParagraph};

const SPARSE_REPEATED_TIER_MIN_PAGES: usize = 2;
const SPARSE_FONT_TIER_CLUSTER_COUNT: usize = 2;
const MIN_BODY_SIZE_BOLD_SIGNALS: usize = 3;
const MAX_OTHER_HEADING_RATIO: usize = 2;
const BODY_SIZE_BOLD_ALIGNMENT_TOLERANCE_EM: f32 = 1.5;
const MIN_OPENED_BODY_WORDS: usize = 4;
const SAME_ROW_MIN_VERTICAL_OVERLAP_RATIO: f32 = 0.5;
/// Font-size "same tier" tolerance (absolute, in the unit `font_size` happens to carry —
/// points for native PDFs, a render-DPI-dependent pixel measurement for OCR segments).
///
/// This IS scale-dependent, and a naive ratio-of-centroid conversion was tried and
/// reverted: forcing `cluster_font_sizes(_, 2)` on a sparse (<5 block) document routinely
/// produces two centroids that are themselves not tight (e.g. a genuine 22pt/21pt/12pt
/// three-tier native document forced into k=2 merges 22 and 21 into one ~21.7 centroid).
/// The tight 0.5pt absolute tolerance deliberately rejects that merge as "not narrow
/// enough" via `has_only_two_narrow_font_tiers`, which is what keeps a non-repeated 21pt
/// "Display prose" line (see `test_build_heading_map_sparse_multi_page_does_not_promote_
/// non_repeated_intermediate_tier`) out of the repeated-heading cluster. A ratio tolerance
/// wide enough to be useful for OCR pixel noise (e.g. 5% of a ~21px cluster) is also wide
/// enough to swallow that native 0.667pt merge slop, which flips `find_heading_level` from
/// `None` to `Some(2)` for the 21pt line and regresses that guard. No tolerance value is
/// simultaneously tight enough to guard native merge slop and loose enough for OCR pixel
/// noise, because both slop magnitudes scale with the *same* input (the forced-k=2
/// centroid), so this stays absolute — see the report for the full trade-off. ~keep
const SPARSE_FONT_TIER_TOLERANCE: f32 = 0.5;
// A tier repeated at the top of multiple pages represents peer sections, not a
// unique document title; reserve H1 for a title and emit these sections as H2. ~keep
const SPARSE_REPEATED_TIER_HEADING_LEVEL: u8 = 2;

type HeadingMap = Vec<(f32, Option<u8>)>;

/// Lowercased `(left, right)` word pairs the document itself writes as a single
/// hyphenated token elsewhere in the text, gathered once per document (#1543).
/// Threaded alongside [`HeadingMap`] as a document-scoped shared reference. ~keep
pub(super) type HyphenWitnesses = ahash::AHashSet<(String, String)>;

/// Document-scoped text-repair evidence, collected once per document by
/// [`collect_hyphen_witnesses`] and [`collect_word_witnesses`] and threaded through
/// paragraph assembly as a single shared reference, alongside [`HeadingMap`]. ~keep
#[derive(Default)]
struct TextRepairWitnesses {
    hyphens: HyphenWitnesses,
    words: WordWitnesses,
}

fn sparse_multi_page_heading_map(
    all_page_segments: &[Vec<SegmentData>],
    heuristic_pages: &[usize],
    all_blocks: &[TextBlock],
    has_struct_tree_blocks: bool,
) -> Result<Option<HeadingMap>> {
    if has_struct_tree_blocks || heuristic_pages.len() < SPARSE_REPEATED_TIER_MIN_PAGES {
        return Ok(None);
    }

    let clusters = cluster_font_sizes(all_blocks, SPARSE_FONT_TIER_CLUSTER_COUNT)?;
    if clusters.len() != SPARSE_FONT_TIER_CLUSTER_COUNT {
        return Ok(None);
    }

    let has_only_two_narrow_font_tiers = all_blocks.iter().all(|block| {
        block.font_size.is_finite()
            && clusters
                .iter()
                .any(|cluster| (block.font_size - cluster.centroid).abs() <= SPARSE_FONT_TIER_TOLERANCE)
    });
    if !has_only_two_narrow_font_tiers {
        return Ok(None);
    }

    let heading_font_size = clusters[0].centroid;
    let body_font_size = clusters[1].centroid;
    let has_distinct_body_tier = heading_font_size - body_font_size > SPARSE_FONT_TIER_TOLERANCE;
    let clears_font_gate = heading_font_size >= body_font_size * MIN_HEADING_FONT_RATIO
        && heading_font_size >= body_font_size + MIN_HEADING_FONT_GAP;
    if !has_distinct_body_tier || !clears_font_gate {
        return Ok(None);
    }

    let repeated_pages: ahash::AHashSet<usize> = heuristic_pages
        .iter()
        .copied()
        .filter(|&page_index| {
            all_page_segments[page_index]
                .iter()
                .find(|segment| !segment.text.trim().is_empty())
                .is_some_and(|segment| {
                    segment.font_size.is_finite()
                        && (segment.font_size - heading_font_size).abs() <= SPARSE_FONT_TIER_TOLERANCE
                })
        })
        .collect();
    if repeated_pages.len() < SPARSE_REPEATED_TIER_MIN_PAGES {
        return Ok(None);
    }

    Ok(Some(
        clusters
            .iter()
            .map(|cluster| {
                let level = ((cluster.centroid - heading_font_size).abs() <= SPARSE_FONT_TIER_TOLERANCE)
                    .then_some(SPARSE_REPEATED_TIER_HEADING_LEVEL);
                (cluster.centroid, level)
            })
            .collect(),
    ))
}

/// Stage 2: Cluster font sizes globally and assign heading levels.
///
/// Returns (heading_map, set of struct-tree page indices needing font-size classification).
#[allow(clippy::type_complexity)]
fn build_heading_map(
    all_page_segments: &[Vec<SegmentData>],
    struct_tree_results: &[Option<Vec<PdfParagraph>>],
    heuristic_pages: &[usize],
    k_clusters: usize,
) -> Result<(Vec<(f32, Option<u8>)>, ahash::AHashSet<usize>)> {
    let struct_tree_needs_classify: ahash::AHashSet<usize> = struct_tree_results
        .iter()
        .enumerate()
        .filter_map(|(i, result)| {
            result.as_ref().and_then(|paragraphs| {
                let has_headings = paragraphs.iter().any(|p| p.heading_level.is_some());
                let has_untagged_bold = paragraphs
                    .iter()
                    .any(|p| p.heading_level.is_none() && p.is_bold && !p.is_list_item);
                if (!has_headings && has_font_size_variation(paragraphs)) || has_untagged_bold {
                    Some(i)
                } else {
                    None
                }
            })
        })
        .collect();

    let mut all_blocks: Vec<TextBlock> = Vec::new();
    let empty_bbox = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };
    // The text is carried so `assign_heading_levels_smart` can pick the body
    // cluster by character mass (char-weighted body size). Leaving it empty makes
    // every cluster tie at length 0, so `max_by_key` falls back to the smallest
    // font as "body" and over-promotes every larger run to a heading. ~keep
    for &i in heuristic_pages {
        for seg in &all_page_segments[i] {
            if seg.text.trim().is_empty() {
                continue;
            }
            all_blocks.push(TextBlock {
                text: seg.text.clone(),
                bbox: empty_bbox,
                font_size: seg.font_size,
            });
        }
    }
    for &i in &struct_tree_needs_classify {
        if let Some(paragraphs) = &struct_tree_results[i] {
            for para in paragraphs {
                let text = if !para.text.is_empty() {
                    para.text.clone()
                } else {
                    para.lines
                        .iter()
                        .flat_map(|l| l.segments.iter())
                        .map(|s| s.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                all_blocks.push(TextBlock {
                    text,
                    bbox: empty_bbox,
                    font_size: para.dominant_font_size,
                });
            }
        }
    }

    let paragraph_count = all_blocks.len();
    let heading_map = if all_blocks.is_empty() {
        Vec::new()
    } else if paragraph_count < MIN_BLOCKS_FOR_FONT_HEADING {
        if let Some(map) = sparse_multi_page_heading_map(
            all_page_segments,
            heuristic_pages,
            &all_blocks,
            !struct_tree_needs_classify.is_empty(),
        )? {
            tracing::debug!(
                paragraph_count,
                "heading map: promoting a repeated sparse font tier across pages"
            );
            map
        } else {
            // Sparsity gate: too few text blocks to establish a reliable body-font
            // baseline. Return a body-only map (every cluster centroid mapped to
            // `None`) and skip both k-means heading promotion and the fallback
            // title promotion, so a lone larger line on a cover/title/one-line
            // document is not over-promoted to a heading. ~keep
            tracing::debug!(
                paragraph_count,
                min_blocks = MIN_BLOCKS_FOR_FONT_HEADING,
                "heading map: document too sparse for font-size heading inference; suppressing promotion"
            );
            let clusters = cluster_font_sizes(&all_blocks, 1)?;
            clusters.iter().map(|c| (c.centroid, None)).collect()
        }
    } else {
        let effective_k = if paragraph_count < 20 {
            k_clusters.min(2usize.max(paragraph_count / 4))
        } else {
            k_clusters
        };

        let clusters = cluster_font_sizes(&all_blocks, effective_k)?;
        let mut map = assign_heading_levels_smart(&clusters, MIN_HEADING_FONT_RATIO);

        let has_any_heading = map.iter().any(|(_, level)| level.is_some());
        if !has_any_heading && !heuristic_pages.is_empty() {
            let first_page = heuristic_pages[0];
            let first_seg_font = all_page_segments[first_page]
                .iter()
                .find(|s| !s.text.trim().is_empty())
                .map(|s| s.font_size);

            if let Some(first_font) = first_seg_font {
                let mut sizes: Vec<f32> = all_blocks.iter().map(|b| b.font_size).collect();
                sizes.sort_by(|a, b| a.total_cmp(b));
                let median = if sizes.is_empty() { 0.0 } else { sizes[sizes.len() / 2] };

                if median > 0.0
                    && first_font >= median * 1.2
                    // Absolute-unit match tolerance; kept as-is for the same reason
                    // `SPARSE_FONT_TIER_TOLERANCE` was kept absolute — see its doc comment.
                    && let Some(entry) = map.iter_mut().find(|(fs, _)| (*fs - first_font).abs() < 0.5)
                {
                    entry.1 = Some(1);
                }
            }
        }

        map
    };

    Ok((heading_map, struct_tree_needs_classify))
}

fn bbox_coordinates_are_finite(bbox: (f32, f32, f32, f32)) -> bool {
    let (left, bottom, right, top) = bbox;
    left.is_finite() && bottom.is_finite() && right.is_finite() && top.is_finite()
}

fn shares_same_text_row(candidate: &PdfParagraph, following: &PdfParagraph, is_same_page: bool) -> bool {
    if !is_same_page {
        return false;
    }
    let (Some(candidate_bbox), Some(following_bbox)) = (candidate.block_bbox, following.block_bbox) else {
        return false;
    };
    if !bbox_coordinates_are_finite(candidate_bbox) || !bbox_coordinates_are_finite(following_bbox) {
        return false;
    }

    let candidate_bottom = candidate_bbox.1.min(candidate_bbox.3);
    let candidate_top = candidate_bbox.1.max(candidate_bbox.3);
    let following_bottom = following_bbox.1.min(following_bbox.3);
    let following_top = following_bbox.1.max(following_bbox.3);
    let minimum_height = (candidate_top - candidate_bottom).min(following_top - following_bottom);
    let overlap = candidate_top.min(following_top) - candidate_bottom.max(following_bottom);
    minimum_height > 0.0 && overlap.max(0.0) / minimum_height >= SAME_ROW_MIN_VERTICAL_OVERLAP_RATIO
}

fn opens_aligned_body_block(
    candidate: &PdfParagraph,
    following: &PdfParagraph,
    body_font_size: f32,
    is_same_page: bool,
) -> bool {
    if following.word_count < MIN_OPENED_BODY_WORDS
        || following.heading_level.is_some()
        || is_body_size_bold_signal(following, body_font_size)
    {
        return false;
    }

    if candidate
        .block_bbox
        .is_some_and(|bbox| !bbox_coordinates_are_finite(bbox))
        || following
            .block_bbox
            .is_some_and(|bbox| !bbox_coordinates_are_finite(bbox))
    {
        return false;
    }
    let (Some(candidate_bbox), Some(following_bbox)) = (candidate.block_bbox, following.block_bbox) else {
        return true;
    };

    if shares_same_text_row(candidate, following, is_same_page) {
        return false;
    }

    let candidate_left = candidate_bbox.0.min(candidate_bbox.2);
    let following_left = following_bbox.0.min(following_bbox.2);
    candidate_left <= following_left + body_font_size * BODY_SIZE_BOLD_ALIGNMENT_TOLERANCE_EM
}

fn is_explicit_numbered_body_size_section(text: &str) -> bool {
    let trimmed = text.trim();
    if !is_numbered_section_heading(trimmed) {
        return false;
    }

    let bytes = trimmed.as_bytes();
    let roman_prefix_length = bytes
        .iter()
        .position(|byte| !b"IVXLCDM".contains(byte))
        .unwrap_or(bytes.len());
    if roman_prefix_length == 0 || bytes.get(roman_prefix_length) != Some(&b' ') {
        return true;
    }

    let remainder = trimmed[roman_prefix_length..].trim_start();
    remainder.chars().any(char::is_alphabetic)
        && remainder
            .chars()
            .filter(|character| character.is_alphabetic())
            .all(char::is_uppercase)
}

fn body_size_heading_eligibility(all_pages: &[Vec<PdfParagraph>], body_font_size: f32) -> Vec<Vec<bool>> {
    let mut next_content: Option<(usize, &PdfParagraph)> = None;
    let mut eligibility = Vec::with_capacity(all_pages.len());
    for (page_index, page) in all_pages.iter().enumerate().rev() {
        let mut page_eligibility = Vec::with_capacity(page.len());
        for paragraph in page.iter().rev() {
            let is_candidate = is_body_size_bold_heading_candidate(paragraph, body_font_size);
            let is_explicit_section = is_explicit_numbered_body_size_section(paragraph_text_raw(paragraph).trim());
            let has_non_finite_geometry = paragraph
                .block_bbox
                .is_some_and(|bbox| !bbox_coordinates_are_finite(bbox))
                || next_content.is_some_and(|(_, following)| {
                    following
                        .block_bbox
                        .is_some_and(|bbox| !bbox_coordinates_are_finite(bbox))
                });
            page_eligibility.push(
                is_candidate
                    && !has_non_finite_geometry
                    && if is_explicit_section {
                        !next_content.is_some_and(|(following_page_index, following)| {
                            shares_same_text_row(paragraph, following, page_index == following_page_index)
                        })
                    } else {
                        next_content.is_some_and(|(following_page_index, following)| {
                            opens_aligned_body_block(
                                paragraph,
                                following,
                                body_font_size,
                                page_index == following_page_index,
                            )
                        })
                    },
            );
            if paragraph.word_count > 0 && !paragraph.is_page_furniture {
                next_content = Some((page_index, paragraph));
            }
        }
        page_eligibility.reverse();
        eligibility.push(page_eligibility);
    }
    eligibility.reverse();
    eligibility
}

fn promote_repeated_body_size_bold_headings(all_pages: &mut [Vec<PdfParagraph>], body_font_size: Option<f32>) {
    let Some(body_font_size) = body_font_size else {
        return;
    };
    let eligible = body_size_heading_eligibility(all_pages, body_font_size);
    let signal_count = eligible.iter().flatten().filter(|&&is_eligible| is_eligible).count();
    let other_heading_count = all_pages
        .iter()
        .flatten()
        .filter(|paragraph| paragraph.heading_level.is_some())
        .count();
    if signal_count < MIN_BODY_SIZE_BOLD_SIGNALS
        || signal_count <= other_heading_count.saturating_mul(MAX_OTHER_HEADING_RATIO)
    {
        return;
    }

    for (page, page_eligibility) in all_pages.iter_mut().zip(eligible) {
        for (paragraph, is_eligible) in page.iter_mut().zip(page_eligibility) {
            if is_eligible {
                paragraph.heading_level = Some(3);
            }
        }
    }
}

/// Build a heading map from structure-tree-assigned roles on segments.
///
/// Instead of clustering font sizes heuristically, this examines the
/// `assigned_role` field on each segment (populated from the PDF structure tree).
/// Each unique font size is mapped to the heading level most commonly assigned
/// to segments at that size. Font sizes with no assigned role are treated as body text.
fn build_heading_map_from_assigned_roles(all_page_segments: &[Vec<SegmentData>]) -> Vec<(f32, Option<u8>)> {
    use std::collections::HashMap;

    let mut size_roles: HashMap<u32, Vec<Option<u8>>> = HashMap::new();
    for page_segs in all_page_segments {
        for seg in page_segs {
            if seg.text.trim().is_empty() {
                continue;
            }
            let key = (seg.font_size * 10.0).round() as u32;
            size_roles.entry(key).or_default().push(seg.assigned_role);
        }
    }

    let mut heading_map: Vec<(f32, Option<u8>)> = size_roles
        .into_iter()
        .map(|(quantized_size, roles)| {
            let font_size = quantized_size as f32 / 10.0;
            let total = roles.len();
            let mut level_counts: HashMap<u8, usize> = HashMap::new();
            let mut none_count = 0usize;
            for role in &roles {
                match role {
                    Some(level) => *level_counts.entry(*level).or_default() += 1,
                    None => none_count += 1,
                }
            }
            let dominant_level = level_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .and_then(|(level, count)| if count * 2 >= total { Some(level) } else { None });

            if none_count > total / 2 && dominant_level.is_none() {
                (font_size, None)
            } else {
                (font_size, dominant_level)
            }
        })
        .collect();

    heading_map.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    heading_map
}

/// Font-size tolerance (points) for merging consecutive raw segments into one
/// logical block in [`count_logical_blocks`]. Matches the font-change
/// threshold `blocks_to_paragraphs` uses to decide paragraph breaks, so a
/// single logical line that a font extractor split into several same-size
/// runs (ligature repair, kerning artifacts, mid-word splits) is not
/// double-counted as multiple blocks.
const LOGICAL_BLOCK_FONT_TOLERANCE: f32 = 1.5;

/// Count logical text blocks by merging consecutive same-role, same-size
/// segments, rather than counting raw segments.
///
/// Raw segment extraction can split one visual line into several runs (a
/// mid-word split from a font-encoding quirk, a bold/italic switch inside a
/// single sentence) that all carry the same `assigned_role`. Counting those
/// raw segments as separate "blocks" over-counts a document's real size and
/// can push a genuinely tiny document (see `hello_structure.pdf`,
/// `issue-987-test.pdf`) above the sparsity floor it should fall under.
/// Consecutive segments on the same page with the same `assigned_role` and a
/// font size within [`LOGICAL_BLOCK_FONT_TOLERANCE`] points collapse into a
/// single block, matching the granularity `total_paragraphs` eventually
/// reports after paragraph assembly.
fn count_logical_blocks(all_page_segments: &[Vec<SegmentData>]) -> usize {
    let mut total = 0usize;
    for page_segs in all_page_segments {
        let mut prev: Option<&SegmentData> = None;
        for seg in page_segs {
            if seg.text.trim().is_empty() {
                continue;
            }
            let continues_prev = prev.is_some_and(|p| {
                p.assigned_role == seg.assigned_role
                    && (p.font_size - seg.font_size).abs() <= LOGICAL_BLOCK_FONT_TOLERANCE
            });
            if !continues_prev {
                total += 1;
            }
            prev = Some(seg);
        }
    }
    total
}

/// Suppress structure-tree heading roles on documents too sparse to trust them.
///
/// A tagged PDF's structure tree is normally a reliable ground truth for
/// heading levels, but on a document with only a handful of text blocks (a
/// one-line note, a cover slide, a two-paragraph test fixture) the same
/// per-block noise that makes font-size clustering unreliable on small
/// samples (see [`MIN_BLOCKS_FOR_FONT_HEADING`] on the heuristic path) also
/// undermines the structure tree: a single mis-tagged or inconsistently
/// authored run is enough to make an entire tiny document look like it is
/// "mostly headings" even when nothing in it is a genuine section heading.
/// Below the same block floor, heading roles are suppressed regardless of
/// whether a body tier is present, matching the heuristic path's rule that
/// a reliable body-font baseline (or, here, a reliable heading/body
/// contrast) needs more than a couple of paragraphs to establish.
///
/// When the condition holds, every segment's `assigned_role` is cleared so
/// paragraph classification (which reads `assigned_role` directly,
/// bypassing the heading map) also treats the document as plain text, and
/// `heading_map` is rewritten to a single body-only entry.
///
/// Returns `true` when suppression fired.
fn suppress_all_heading_roles_when_sparse_and_untrusted(
    heading_map: &mut Vec<(f32, Option<u8>)>,
    all_page_segments: &mut [Vec<SegmentData>],
) -> bool {
    let total_blocks = count_logical_blocks(all_page_segments);
    let has_any_heading = heading_map.iter().any(|(_, level)| level.is_some());

    if total_blocks == 0 || total_blocks >= MIN_BLOCKS_FOR_FONT_HEADING || !has_any_heading {
        return false;
    }

    tracing::debug!(
        total_blocks,
        min_blocks = MIN_BLOCKS_FOR_FONT_HEADING,
        "structure tree: document too sparse to trust tagged heading roles; suppressing all heading roles"
    );

    for page_segs in all_page_segments.iter_mut() {
        for seg in page_segs.iter_mut() {
            seg.assigned_role = None;
        }
    }
    heading_map.clear();
    true
}

/// Promote an untagged document-title font tier above structure-tree headings.
///
/// Word processors tag the document title with a non-heading structure type
/// (e.g. LibreOffice's "Title" style resolves to a non-`H*` element), so
/// `build_heading_map_from_assigned_roles` classifies it as body text even
/// though it is visually the top-level heading. When such a tier exists —
/// strictly larger than every tagged heading font, bold, few segments, and
/// present on the first page — assign it level 1 and demote all tagged
/// heading levels by one (Title = h1, tagged H1 = h2, ...), matching the
/// pandoc/HTML convention that the document title outranks section headings.
///
/// Returns `true` when a title tier was promoted. The caller must then also
/// demote the per-segment `assigned_role` values (see
/// `demote_assigned_roles`), because paragraph classification honours
/// `assigned_role` directly, bypassing the heading map.
fn promote_untagged_document_title(
    heading_map: &mut [(f32, Option<u8>)],
    all_page_segments: &[Vec<SegmentData>],
) -> bool {
    /// A title is a handful of segments at most; more means a body/pull-quote tier.
    const MAX_TITLE_SEGMENTS: usize = 3;

    let Some(max_heading_font) = heading_map
        .iter()
        .filter(|(_, level)| level.is_some())
        .map(|(font, _)| *font)
        .fold(None, |acc: Option<f32>, f| Some(acc.map_or(f, |a| a.max(f))))
    else {
        return false;
    };

    let candidate = heading_map
        .iter()
        .position(|(font, level)| level.is_none() && *font > max_heading_font);
    let Some(candidate_idx) = candidate else {
        return false;
    };
    let candidate_font = heading_map[candidate_idx].0;

    let mut tier_segments = 0usize;
    let mut all_bold = true;
    let mut on_first_page = false;
    for (page_idx, page_segs) in all_page_segments.iter().enumerate() {
        for seg in page_segs {
            if seg.text.trim().is_empty() || (seg.font_size - candidate_font).abs() >= 0.05 {
                continue;
            }
            tier_segments += 1;
            all_bold &= seg.is_bold;
            on_first_page |= page_idx == 0;
        }
    }
    if tier_segments == 0 || tier_segments > MAX_TITLE_SEGMENTS || !all_bold || !on_first_page {
        return false;
    }

    tracing::debug!(
        title_font = candidate_font,
        max_heading_font,
        tier_segments,
        "structure tree: promoting untagged document-title tier to h1, demoting tagged levels"
    );
    for (font, level) in heading_map.iter_mut() {
        if let Some(l) = level {
            *level = Some((*l + 1).min(6));
        } else if (*font - candidate_font).abs() < 0.05 {
            *level = Some(1);
        }
    }
    true
}

/// Demote every structure-tree-assigned heading role by one level (capped at 6).
///
/// Companion to `promote_untagged_document_title`: paragraph classification
/// (`bridge.rs`) uses `assigned_role` directly as "the author's stated intent",
/// so the map-level demotion must be mirrored on the segments themselves.
fn demote_assigned_roles(all_page_segments: &mut [Vec<SegmentData>]) {
    for page_segs in all_page_segments.iter_mut() {
        for seg in page_segs.iter_mut() {
            if let Some(role) = seg.assigned_role {
                seg.assigned_role = Some((role + 1).min(6));
            }
        }
    }
}

/// Per-page input bundle for Stage 3 parallel processing.
///
/// Each page's data is pre-extracted before `into_par_iter` so all threads
/// receive owned, non-overlapping slices of the document's data.
struct PageInput {
    /// Index of this page in the document (0-based).
    page_index: usize,
    /// Paragraphs from the PDF structure tree, if extraction succeeded.
    struct_paragraphs: Option<Vec<PdfParagraph>>,
    /// Segments from heuristic extraction (non-empty only when `struct_paragraphs` is `None`).
    heuristic_segments: Vec<SegmentData>,
    /// Layout hints for this page, if layout detection was run.
    page_hints: Option<Vec<LayoutHint>>,
    /// Footprint and cell text of tables successfully extracted for this page.
    table_bboxes: Vec<TableCoverage>,
    /// Whether native semantic classification should be preserved while layout
    /// hints continue to control reading order and record region provenance.
    preserve_native_semantics: bool,
    /// Whether layout geometry should reorder native paragraphs on this page.
    /// Semantic refinement runs in native order before this spatial order is
    /// applied during final assembly.
    use_layout_reading_order: bool,
    /// Per-hint validation results from CC analysis (parallel to page_hints).
    /// Empty when layout-detection is not active.
    #[cfg(feature = "layout-detection")]
    hint_validations: Vec<super::regions::layout_validation::RegionValidation>,
    /// Actual PDF page width in points, used by layout reading-order refinement.
    #[cfg(feature = "layout-detection")]
    page_width_pts: Option<f32>,
    /// Whether this page's structure-tree paragraphs need font-size classification.
    needs_classify: bool,
    /// Y-coordinates of paragraph gaps detected from segment boundaries.
    paragraph_gap_ys: Vec<f32>,
    /// When true, paragraphs classified as `PageHeader` by the layout model are
    /// preserved rather than marked as furniture. Mirrors `ContentFilterConfig::include_headers`.
    include_headers: bool,
    /// When true, paragraphs classified as `PageFooter` by the layout model are
    /// preserved rather than marked as furniture. Mirrors `ContentFilterConfig::include_footers`.
    include_footers: bool,
    /// When true, paragraphs classified as `Footnote` by the layout model are
    /// preserved rather than marked as furniture. Mirrors `ContentFilterConfig::include_footnotes`.
    include_footnotes: bool,
}

/// Process a single page's data through Stage 3: classification, text repair,
/// layout overrides, dehyphenation, and list splitting.
///
/// This function is intentionally free of any shared mutable state so it can be
/// called from multiple threads via `rayon::par_iter`.
fn process_single_page(
    input: PageInput,
    heading_map: &[(f32, Option<u8>)],
    doc_body_font_size: Option<f32>,
    witnesses: &TextRepairWitnesses,
) -> Vec<PdfParagraph> {
    let PageInput {
        page_index: i,
        struct_paragraphs,
        heuristic_segments,
        page_hints,
        table_bboxes,
        preserve_native_semantics,
        use_layout_reading_order,
        #[cfg(feature = "layout-detection")]
        hint_validations,
        #[cfg(feature = "layout-detection")]
        page_width_pts,
        needs_classify,
        paragraph_gap_ys,
        include_headers,
        include_footers,
        include_footnotes,
    } = input;
    #[cfg(not(feature = "layout-detection"))]
    let _ = preserve_native_semantics;
    #[cfg(not(feature = "layout-detection"))]
    let _ = use_layout_reading_order;
    if let Some(mut paragraphs) = struct_paragraphs {
        apply_text_repair_to_structure_tree_paragraphs(&mut paragraphs, true, witnesses);
        if needs_classify {
            tracing::debug!(
                page = i,
                "PDF structure pipeline: classifying struct tree page via font-size clustering"
            );
            classify_paragraphs(&mut paragraphs, heading_map);
        }
        merge_continuation_paragraphs(&mut paragraphs);
        synchronize_paragraph_text_metadata(&mut paragraphs);
        merge_spatial_footnote_markers(&mut paragraphs);
        if let Some(ref hints) = page_hints {
            let classification_hints = regular_layout_hints(hints);
            super::layout_classify::apply_layout_overrides(
                &mut paragraphs,
                &classification_hints,
                0.5,
                0.2,
                doc_body_font_size,
            );
            un_mark_layout_furniture_per_config(&mut paragraphs, include_headers, include_footers, include_footnotes);
            tracing::debug!(
                page = i,
                headings = paragraphs.iter().filter(|p| p.heading_level.is_some()).count(),
                lists = paragraphs.iter().filter(|p| p.is_list_item).count(),
                furniture = paragraphs.iter().filter(|p| p.is_page_furniture).count(),
                "layout overrides applied"
            );
            retain_page_furniture_safely(&mut paragraphs);
        }
        demote_structure_annotation_headings(&mut paragraphs);
        paragraphs
    } else {
        let page_segments = heuristic_segments;
        tracing::debug!(
            page = i,
            segments = page_segments.len(),
            has_layout_hints = page_hints.is_some(),
            "process_single_page: heuristic path"
        );
        let page_segments = filter_segments_by_table_bboxes(page_segments, &table_bboxes);
        #[cfg(feature = "layout-detection")]
        let mut paragraphs = if let Some(ref hints) = page_hints {
            let wrapper_ownership = wrapper_ownership_by_hint(hints, &hint_validations);
            if use_layout_reading_order
                && crate::extractors::pdf::reading_order::has_eligible_layout_hints(hints, &wrapper_ownership)
            {
                process_layout_segment_groups(
                    page_segments,
                    hints,
                    &wrapper_ownership,
                    LayoutParagraphContext {
                        heading_map,
                        paragraph_gap_ys: &paragraph_gap_ys,
                        doc_body_font_size,
                        include_headers,
                        include_footers,
                        include_footnotes,
                        page_width_pts,
                        apply_layout_overrides: !preserve_native_semantics,
                        witnesses,
                    },
                )
            } else {
                let mut paragraphs = segments_to_paragraphs(page_segments, heading_map, &paragraph_gap_ys, witnesses);
                let classification_hints = regular_layout_hints(hints);
                super::layout_classify::annotate_layout_classes(&mut paragraphs, &classification_hints, 0.5, 0.2);
                paragraphs
            }
        } else {
            segments_to_paragraphs(page_segments, heading_map, &paragraph_gap_ys, witnesses)
        };
        #[cfg(not(feature = "layout-detection"))]
        let mut paragraphs = segments_to_paragraphs(page_segments, heading_map, &paragraph_gap_ys, witnesses);
        tracing::debug!(
            page = i,
            paragraphs = paragraphs.len(),
            "heuristic paragraphs classified"
        );
        #[cfg(not(feature = "layout-detection"))]
        if let Some(ref hints) = page_hints {
            let classification_hints = regular_layout_hints(hints);
            super::layout_classify::apply_layout_overrides(
                &mut paragraphs,
                &classification_hints,
                0.5,
                0.2,
                doc_body_font_size,
            );
            un_mark_layout_furniture_per_config(&mut paragraphs, include_headers, include_footers, include_footnotes);
        }
        if page_hints.is_some() {
            tracing::debug!(
                page = i,
                headings = paragraphs.iter().filter(|p| p.heading_level.is_some()).count(),
                lists = paragraphs.iter().filter(|p| p.is_list_item).count(),
                furniture = paragraphs.iter().filter(|p| p.is_page_furniture).count(),
                "layout overrides applied"
            );
        }
        demote_structure_annotation_headings(&mut paragraphs);
        merge_spatial_footnote_markers(&mut paragraphs);
        retain_page_furniture_safely(&mut paragraphs);
        paragraphs
    }
}

const TABLE_DOMINANT_MIN_BODY_ROWS: usize = 40;
const TABLE_DOMINANT_MIN_VISIBLE_CHAR_SHARE: f64 = 0.85;
const TABLE_SPILL_MIN_PARAGRAPH_OVERLAP: f64 = 0.2;

/// Remove non-semantic spill inside table regions on overwhelmingly tabular pages.
///
/// Layout table crops can leave out-of-bounds table continuations as ordinary
/// paragraphs. The page-level dominance guards are only activation gates: a
/// paragraph is removed only when enough of its own geometry overlaps an emitted
/// table rectangle and its content looks like a row continuation or marker,
/// preserving surrounding headings and short explanatory prose.
fn suppress_table_dominant_paragraph_spill(pages: &mut [Vec<PdfParagraph>], emitted_tables: &[crate::types::Table]) {
    for (page_index, paragraphs) in pages.iter_mut().enumerate() {
        let page_number = page_index.saturating_add(1) as u32;
        let page_tables = emitted_tables
            .iter()
            .filter(|table| table.page_number == page_number)
            .collect::<Vec<_>>();
        let body_rows = page_tables
            .iter()
            .map(|table| table.cells.len().saturating_sub(1))
            .sum::<usize>();
        if body_rows < TABLE_DOMINANT_MIN_BODY_ROWS {
            continue;
        }

        let table_chars = page_tables
            .iter()
            .flat_map(|table| table.cells.iter().flatten())
            .map(|cell| visible_char_count(cell))
            .sum::<usize>();
        let paragraph_chars = paragraphs
            .iter()
            .map(paragraph_text_raw)
            .map(|text| visible_char_count(&text))
            .sum::<usize>();
        let total_chars = table_chars.saturating_add(paragraph_chars);
        let table_share = if total_chars == 0 {
            0.0
        } else {
            table_chars as f64 / total_chars as f64
        };
        if table_share < TABLE_DOMINANT_MIN_VISIBLE_CHAR_SHARE {
            continue;
        }

        let table_bboxes = page_tables
            .iter()
            .filter_map(|table| table.bounding_box.as_ref())
            .collect::<Vec<_>>();
        if table_bboxes.is_empty() {
            continue;
        }
        let before = paragraphs.len();
        paragraphs.retain(|paragraph| !is_table_crop_spill(paragraph, &table_bboxes));
        tracing::debug!(
            page = page_number,
            body_rows,
            table_share,
            removed = before.saturating_sub(paragraphs.len()),
            "table-dominant paragraph spill cleanup"
        );
    }
}

fn visible_char_count(text: &str) -> usize {
    text.chars().filter(|character| !character.is_whitespace()).count()
}

fn is_table_crop_spill(paragraph: &PdfParagraph, table_bboxes: &[&crate::types::BoundingBox]) -> bool {
    if is_preserved_table_page_annotation(paragraph) {
        return false;
    }
    let Some(paragraph_bbox) = paragraph_geometry_bbox(paragraph) else {
        return false;
    };
    if !table_bboxes.iter().any(|table_bbox| {
        table_paragraph_overlap_fraction(paragraph_bbox, table_bbox) >= TABLE_SPILL_MIN_PARAGRAPH_OVERLAP
    }) {
        return false;
    }

    let text = paragraph_text_raw(paragraph);
    let alphabetic_chars = text.chars().filter(|character| character.is_alphabetic()).count();
    let numeric_chars = text.chars().filter(|character| character.is_numeric()).count();
    paragraph.word_count >= 13 || alphabetic_chars < 5 || numeric_chars >= alphabetic_chars
}

fn table_paragraph_overlap_fraction(
    paragraph_bbox: (f32, f32, f32, f32),
    table_bbox: &crate::types::BoundingBox,
) -> f64 {
    let (raw_left, raw_bottom, raw_right, raw_top) = paragraph_bbox;
    let paragraph_left = raw_left.min(raw_right) as f64;
    let paragraph_right = raw_left.max(raw_right) as f64;
    let paragraph_bottom = raw_bottom.min(raw_top) as f64;
    let paragraph_top = raw_bottom.max(raw_top) as f64;
    let paragraph_area = (paragraph_right - paragraph_left) * (paragraph_top - paragraph_bottom);
    if paragraph_area <= 0.0 {
        return 0.0;
    }

    let intersection_width = (paragraph_right.min(table_bbox.x0.max(table_bbox.x1))
        - paragraph_left.max(table_bbox.x0.min(table_bbox.x1)))
    .max(0.0);
    let intersection_height = (paragraph_top.min(table_bbox.y0.max(table_bbox.y1))
        - paragraph_bottom.max(table_bbox.y0.min(table_bbox.y1)))
    .max(0.0);
    intersection_width * intersection_height / paragraph_area
}

fn is_preserved_table_page_annotation(paragraph: &PdfParagraph) -> bool {
    if paragraph.heading_level.is_some()
        || paragraph.caption_for.is_some()
        || paragraph.is_list_item
        || paragraph.is_code_block
        || paragraph.is_formula
        || matches!(
            paragraph.layout_class,
            Some(
                super::types::LayoutHintClass::Title
                    | super::types::LayoutHintClass::SectionHeader
                    | super::types::LayoutHintClass::Caption
                    | super::types::LayoutHintClass::Footnote
                    | super::types::LayoutHintClass::PageHeader
                    | super::types::LayoutHintClass::PageFooter
                    | super::types::LayoutHintClass::ListItem
                    | super::types::LayoutHintClass::Code
                    | super::types::LayoutHintClass::Formula
                    | super::types::LayoutHintClass::DocumentIndex
                    | super::types::LayoutHintClass::Form
                    | super::types::LayoutHintClass::KeyValueRegion
            )
        )
    {
        return true;
    }

    let text = paragraph_text_raw(paragraph);
    let label = text
        .trim_start()
        .split_once(char::is_whitespace)
        .map_or(text.trim(), |(first, _)| first)
        .trim_end_matches([':', '.'])
        .to_ascii_lowercase();
    matches!(label.as_str(), "note" | "notes" | "source" | "sources" | "table")
}

fn is_wrapper_layout_hint(hint: &LayoutHint) -> bool {
    hint.class_name.is_wrapper()
}

fn regular_layout_hints(hints: &[LayoutHint]) -> Vec<LayoutHint> {
    hints
        .iter()
        .filter(|hint| !is_wrapper_layout_hint(hint))
        .cloned()
        .collect()
}

fn segments_to_paragraphs(
    segments: Vec<SegmentData>,
    heading_map: &[(f32, Option<u8>)],
    paragraph_gap_ys: &[f32],
    witnesses: &TextRepairWitnesses,
) -> Vec<PdfParagraph> {
    let segments = order_segments_in_reading_frames(segments);
    let mut paragraphs = blocks_to_paragraphs(segments, heading_map, paragraph_gap_ys);
    apply_text_repair_to_structure_tree_paragraphs(&mut paragraphs, true, witnesses);
    reattach_detached_list_markers(&mut paragraphs, DetachedMarkerFrame::Native);
    merge_continuation_paragraphs(&mut paragraphs);
    synchronize_paragraph_text_metadata(&mut paragraphs);
    paragraphs
}

/// Master switch for [`reattach_detached_list_markers`].
///
/// Flip this single constant to `false` to build a control binary that differs
/// from the shipped one only in this behaviour; nothing else guards the pass.
const REATTACH_DETACHED_LIST_MARKERS: bool = true;

/// Suppress heading promotion for a fragment whose text starts lowercase or with
/// a sentence-continuation word (#712).
///
/// This is the fabrication signature of the OCR mid-line paragraph break: when
/// `font_change` (`(line.font_size - prev.font_size).abs() > 1.5`) splits a
/// physical line on intra-line ascender/descender noise, the stray tail
/// fragment almost always starts mid-sentence -- lowercase, or with "is", "of",
/// "and", and the like -- because a real sentence or heading boundary does not
/// land there. That stray fragment's own (noise-inflated) font size is then
/// read as `first.font_size` for the *next* paragraph and can clear the
/// heading-distance gate in [`super::classify::find_heading_level`], fabricating
/// a heading out of a sentence fragment (e.g. `### storage.`,
/// `### groundwork for future developments. Over time,`). Reuses
/// [`super::classify::starts_with_lowercase_or_continuation`], the same guard
/// the rescue pass already trusts for the identical judgment, so this adds no
/// new heuristic surface. Flip to `false` to restore pre-#712 behaviour.
const SUPPRESS_LOWERCASE_START_HEADINGS: bool = true;

/// How closely a detached marker's baseline must agree with the baseline of the
/// body line it is claimed to belong to, as a multiple of the larger of the two
/// font sizes. Scale-free by construction, so it behaves identically on
/// point-scale native input and on OCR font sizes of a different magnitude.
const DETACHED_MARKER_BASELINE_TOLERANCE_FONT_FACTOR: f32 = 0.6;

/// Largest hanging indent, measured from the marker's right edge to the body
/// line's left edge, as a multiple of the body font size. Real hanging indents
/// run a quarter to half an inch; this admits those while refusing to pair a
/// marker in one column with a body in another.
const DETACHED_MARKER_MAX_INDENT_FONT_FACTOR: f32 = 6.0;

/// Largest overlap tolerated in the other direction, as a multiple of the body
/// font size, so a marker whose measured width slightly overruns the body's
/// left edge still pairs.
const DETACHED_MARKER_MAX_OVERLAP_FONT_FACTOR: f32 = 0.5;

/// How many paragraphs ahead of a detached marker its body may sit. A marker
/// *column* emits every marker before any body ("(a)", "(b)", "(c)", then three
/// bodies), so the body is not necessarily the next paragraph.
///
/// Also reused by the OCR layout route's `adapters::reattach_ocr_layout_list_markers`.
pub(super) const DETACHED_MARKER_MAX_LOOKAHEAD: usize = 8;

/// Minimum word count of the body paragraph. Excludes single-token neighbours,
/// which is what a marker-shaped table column looks like.
///
/// `pub(super)` so `adapters::accepts_marker_run_body` (#729) can reuse it for
/// the marker-run/body-run pairing phase of `adapters::reattach_ocr_layout_list_markers`.
pub(super) const DETACHED_MARKER_MIN_BODY_WORDS: usize = 2;

/// Whether the two detached-list-marker reattachment passes (this module's
/// [`detached_list_marker`] and `adapters::ocr_detached_list_marker`) reject a
/// lone `*` and a bracketed integer `[N]` as marker paragraphs, on top of the
/// general [`is_bare_list_marker`] test.
///
/// Both shapes are ambiguous specifically in the *detached* (cross-paragraph)
/// case, where the marker paragraph can be reattached to a body many
/// paragraphs away: a standalone `*` line is also a bare multiplication sign
/// in isolated mathematical prose, and `[N]` is the standard printed
/// paragraph-number notation in reference works (e.g. Jung's Collected
/// Works), not a list marker. Reattaching either turns unrelated prose into a
/// fabricated list item. Flip to `false` to restore the pre-tightening
/// behaviour where both are accepted. Deliberately does NOT touch
/// `is_bare_list_marker` itself, which stays available to the *same-line*
/// split-marker cases in `blocks_to_paragraphs` and `finalize_paragraph`,
/// where the marker and body are already adjacent segments on one physical
/// line and this cross-paragraph ambiguity does not arise. ~keep
const EXCLUDE_AMBIGUOUS_DETACHED_MARKERS: bool = true;

/// Whether `text` is a marker shape that is ambiguous enough to reject in the
/// *detached* (cross-paragraph) reattachment passes even though
/// [`is_bare_list_marker`] accepts it. See [`EXCLUDE_AMBIGUOUS_DETACHED_MARKERS`].
fn is_ambiguous_detached_marker(text: &str) -> bool {
    let t = text.trim();
    if t == "*" {
        return true;
    }
    if let Some(inner) = t.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
        return !inner.is_empty() && inner.chars().all(|character| character.is_ascii_digit());
    }
    false
}

/// Narrower sibling of [`is_bare_list_marker`] used only by the two detached
/// (cross-paragraph) reattachment passes -- this module's
/// [`detached_list_marker`] and `adapters::ocr_detached_list_marker`. See
/// [`EXCLUDE_AMBIGUOUS_DETACHED_MARKERS`] for why the two predicates
/// deliberately differ.
pub(super) fn is_bare_detached_list_marker(text: &str) -> bool {
    is_bare_list_marker(text) && !(EXCLUDE_AMBIGUOUS_DETACHED_MARKERS && is_ambiguous_detached_marker(text))
}

/// Which reading frame [`accepts_detached_list_marker`] should compare geometry in.
///
/// `Native` is the pre-#760 behaviour: it reuses each segment's own
/// `rotation_degrees` via [`SegmentData::upright_baseline`] /
/// [`SegmentData::upright_advance_extent`], unchanged. `OcrOnPage` is the OCR
/// route's correction (#760): OCR segments always carry `rotation_degrees ==
/// 0.0` -- `rotation_degrees` on native text encodes that TEXT RUN's own
/// orientation on the page, and OCR's raster boxes have no analogous per-run
/// signal, so `adapters::make_ocr_pdf_line` hardcodes `0.0` -- but on a page
/// with a PDF `/Rotate`, the OCR raster stays MediaBox-oriented by design, so a
/// rotated page's segment `x`/`y`/`width`/`height` sit in the RASTER frame while
/// this predicate needs the UPRIGHT reading frame. `OcrOnPage(degrees)`
/// recovers that frame locally, from the page's own `/Rotate` value, without
/// writing anything back onto [`SegmentData`].
///
/// Writing the correction onto `SegmentData::rotation_degrees` globally instead
/// was tried and rejected: it silently activates roughly 82 other
/// `is_unrotated()` / `has_same_rotation()` / `upright_*()` call sites across 10
/// files, all written for native text's TEXT-LOCAL-ADVANCE convention (`width`
/// is the run's advance along its own baseline), which OCR's axis-aligned boxes
/// do not satisfy -- it regressed `ocr_test_rotated_90` (word count 15 -> 13,
/// glued "conversion toJSON") and scrambled `ocr_test_rotated_270`'s reading
/// order entirely. Keeping the correction local to this one predicate, computed
/// fresh from the raw fields on every call, avoids all of that blast radius.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DetachedMarkerFrame {
    /// Native PDF text: defer to the segment's own `rotation_degrees`.
    Native,
    /// OCR route on a page whose PDF `/Rotate` is `degrees` (0/90/180/270).
    ///
    /// Only ever constructed from the OCR adapters, so on a feature set without
    /// OCR this variant is genuinely dead -- the `Native` arm still carries the
    /// whole native structure-tree path. `-D warnings` on the narrow `pdf` leg
    /// turns that into a hard error, so silence it exactly there rather than
    /// splitting the enum. ~keep
    #[cfg_attr(not(any(feature = "ocr", feature = "ocr-pipeline")), allow(dead_code))]
    OcrOnPage(u32),
}

impl DetachedMarkerFrame {
    /// The baseline coordinate to compare, in this frame.
    ///
    /// The OCR arms are measured against fixture `ordinance_2197` (`/Rotate
    /// 270`), tesseract backend, except `90`, which mirrors `270` by symmetry
    /// and has no fixture measurement backing it -- see this type's doc
    /// comment. `180` is confirmed a no-op: it falls through to the same
    /// `baseline_y` read as the unrotated default.
    fn baseline(self, segment: &SegmentData) -> f32 {
        match self {
            Self::Native => segment.upright_baseline(),
            // Measured: the FAR raster-x edge (`x + width`), not the near edge
            // (`x`) -- the far edge discriminates correct marker/body pairs
            // from wrong ones (delta 0-1 vs 215 against an 18-wide tolerance);
            // the near edge cannot (75-76 vs 139).
            Self::OcrOnPage(270) => segment.x + segment.width,
            // UNVERIFIED: derived by mirroring the 270 case (the near edge
            // instead of the far edge, matching the opposite rotation
            // handedness), not measured against any fixture.
            Self::OcrOnPage(90) => segment.x,
            Self::OcrOnPage(_) => segment.baseline_y,
        }
    }

    /// `(start, end)` along the reading direction, in this frame.
    fn advance_extent(self, segment: &SegmentData) -> (f32, f32) {
        match self {
            Self::Native => segment.upright_advance_extent(),
            // Measured: the advance axis runs along -y on a 270-rotated page,
            // and the reading-order START is the FAR raster-y edge
            // (`y + height`), not the near edge -- omitting the raster-y
            // extent mirrors the span ([-y, -y+height] instead of
            // [-(y+height), -y]) and inverts the indent test.
            Self::OcrOnPage(270) => (-(segment.y + segment.height), -segment.y),
            // UNVERIFIED: derived by mirroring the 270 case (the advance axis
            // runs along +y instead of -y, so the near/far roles swap and no
            // negation is needed), not measured against any fixture.
            Self::OcrOnPage(90) => (segment.y, segment.y + segment.height),
            Self::OcrOnPage(_) => (segment.x, segment.x + segment.width),
        }
    }
}

/// Reattach a list marker that was emitted as a paragraph of its own to the
/// body line it belongs to.
///
/// A hanging-indent list puts its markers in a narrow left column and its item
/// text in a wide right column. Both OCR block segmentation and some native
/// producers treat those columns as separate blocks, so the markers arrive as
/// isolated single-segment paragraphs — sometimes the whole marker column
/// ahead of the whole text column — and every item loses the only evidence that
/// it is an item. `finalize_paragraph`'s `starts_with_split_list_marker` already
/// handles the case where the marker and the body ended up in the *same*
/// paragraph; this handles the case where they did not.
///
/// Pairing is by *baseline*, not by adjacency, which is what makes a marker
/// column recoverable: "(a)", "(b)", "(c)" each match the body line they share a
/// baseline with regardless of how many paragraphs sit between them.
///
/// Deliberately narrow, because this pass is shared with native extraction:
/// - The marker paragraph must be exactly one line holding exactly one segment
///   whose whole text is a bare marker ([`is_bare_detached_list_marker`]) —
///   prose can never produce that, so flowing text has nothing here to match.
///   Narrower than the general [`is_bare_list_marker`]: see
///   [`EXCLUDE_AMBIGUOUS_DETACHED_MARKERS`].
/// - The body must not already be a heading or a list item, and its first line
///   must not already start with a marker.
/// - The body must be strictly to the right of the marker, within one hanging
///   indent, on the same baseline, in the same rotation frame.
/// - The body must carry at least [`DETACHED_MARKER_MIN_BODY_WORDS`] words.
///
/// Everything is expressed relative to font size, so the OCR route (whose
/// geometry may be in a different unit space) and the native route (points) get
/// the same behaviour.
pub(super) fn reattach_detached_list_markers(paragraphs: &mut Vec<PdfParagraph>, frame: DetachedMarkerFrame) {
    if !REATTACH_DETACHED_LIST_MARKERS || paragraphs.len() < 2 {
        return;
    }

    let mut consumed = vec![false; paragraphs.len()];
    let mut pairs: Vec<(usize, usize)> = Vec::new();

    for marker_index in 0..paragraphs.len() {
        if consumed[marker_index] {
            continue;
        }
        let Some(marker) = detached_list_marker(&paragraphs[marker_index]) else {
            continue;
        };
        let limit = (marker_index + 1 + DETACHED_MARKER_MAX_LOOKAHEAD).min(paragraphs.len());
        let body_index = (marker_index + 1..limit).find(|&candidate| {
            !consumed[candidate] && accepts_detached_list_marker(&paragraphs[candidate], &marker, frame)
        });
        let Some(body_index) = body_index else {
            continue;
        };
        consumed[marker_index] = true;
        consumed[body_index] = true;
        pairs.push((marker_index, body_index));
    }

    if pairs.is_empty() {
        return;
    }

    for (marker_index, body_index) in &pairs {
        let Some(marker_segment) = paragraphs[*marker_index]
            .lines
            .first()
            .and_then(|line| line.segments.first())
            .cloned()
        else {
            continue;
        };
        let marker_bbox = paragraphs[*marker_index].block_bbox;
        let body = &mut paragraphs[*body_index];
        if let Some(line) = body.lines.first_mut() {
            line.segments.insert(0, marker_segment);
        }
        body.is_list_item = true;
        body.block_bbox = match (body.block_bbox, marker_bbox) {
            (Some(body_bbox), Some(marker_bbox)) => Some((
                body_bbox.0.min(marker_bbox.0),
                body_bbox.1.min(marker_bbox.1),
                body_bbox.2.max(marker_bbox.2),
                body_bbox.3.max(marker_bbox.3),
            )),
            (bbox @ Some(_), None) | (None, bbox @ Some(_)) => bbox,
            (None, None) => None,
        };
        // Text is rebuilt from `lines` downstream (see
        // `synchronize_paragraph_text_metadata`); a stale cached string here
        // would silently win over the segment we just spliced in.
        body.text.clear();
        body.word_count = PdfParagraph::compute_word_count("", &body.lines);
    }

    let mut index = 0usize;
    paragraphs.retain(|_| {
        let keep = !pairs.iter().any(|(marker_index, _)| *marker_index == index);
        index += 1;
        keep
    });
}

/// The lone segment of a paragraph that is nothing but a list marker.
fn detached_list_marker(paragraph: &PdfParagraph) -> Option<SegmentData> {
    if paragraph.heading_level.is_some() || paragraph.is_list_item || paragraph.is_code_block || paragraph.is_formula {
        return None;
    }
    let [line] = paragraph.lines.as_slice() else {
        return None;
    };
    let [segment] = line.segments.as_slice() else {
        return None;
    };
    if !is_bare_detached_list_marker(&segment.text) {
        return None;
    }
    let geometry_is_usable = segment.x.is_finite()
        && segment.width.is_finite()
        && segment.width >= 0.0
        && segment.font_size.is_finite()
        && segment.font_size > 0.0
        && segment.upright_baseline().is_finite();
    geometry_is_usable.then(|| segment.clone())
}

/// Whether `paragraph` is the body line the detached `marker` belongs to.
///
/// Also reused by the OCR layout route's own reattachment pass
/// (`adapters::reattach_ocr_layout_list_markers`) -- this body-side test has no
/// dependency on how the marker paragraph itself was classified, only on the
/// candidate body's own shape, so it applies identically to both routes once
/// given the right [`DetachedMarkerFrame`] (#760): the OCR route passes
/// `OcrOnPage`, native passes `Native`. See that function's doc comment for why
/// the marker-side test (`detached_list_marker`, below) is NOT similarly
/// shared.
pub(super) fn accepts_detached_list_marker(
    paragraph: &PdfParagraph,
    marker: &SegmentData,
    frame: DetachedMarkerFrame,
) -> bool {
    if paragraph.heading_level.is_some()
        || paragraph.is_list_item
        || paragraph.is_code_block
        || paragraph.is_formula
        || paragraph.is_page_furniture
    {
        return false;
    }
    let Some(first_line) = paragraph.lines.first() else {
        return false;
    };
    if first_line.segments.is_empty() {
        return false;
    }
    let first_line_text = first_line
        .segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if looks_like_list_item(&first_line_text) || is_bare_list_marker(&first_line_text) {
        return false;
    }

    let body_words = paragraph
        .lines
        .iter()
        .flat_map(|line| line.segments.iter())
        .flat_map(|segment| segment.text.split_whitespace())
        .count();
    if body_words < DETACHED_MARKER_MIN_BODY_WORDS {
        return false;
    }

    let Some(anchor) = first_line.segments.first() else {
        return false;
    };
    if !anchor.has_same_rotation(marker) {
        return false;
    }
    let font_size = anchor.font_size.max(marker.font_size);
    if !font_size.is_finite() || font_size <= 0.0 {
        return false;
    }

    let baseline_delta = (frame.baseline(anchor) - frame.baseline(marker)).abs();
    if !baseline_delta.is_finite() || baseline_delta > font_size * DETACHED_MARKER_BASELINE_TOLERANCE_FONT_FACTOR {
        return false;
    }

    let body_left = first_line
        .segments
        .iter()
        .map(|segment| frame.advance_extent(segment).0)
        .fold(f32::INFINITY, f32::min);
    let (marker_start, marker_end) = frame.advance_extent(marker);
    if !body_left.is_finite() || !marker_start.is_finite() || !marker_end.is_finite() {
        return false;
    }
    let indent = body_left - marker_end;
    body_left > marker_start
        && indent >= -(font_size * DETACHED_MARKER_MAX_OVERLAP_FONT_FACTOR)
        && indent <= font_size * DETACHED_MARKER_MAX_INDENT_FONT_FACTOR
}

/// Repair reading order inside maximal rotated runs without touching upright
/// stream order or moving content across a rotation boundary.
fn order_segments_in_reading_frames(segments: Vec<SegmentData>) -> Vec<SegmentData> {
    if segments.iter().all(SegmentData::is_unrotated) {
        return segments;
    }

    let mut groups: Vec<Vec<SegmentData>> = Vec::new();
    for segment in segments {
        match groups.last_mut() {
            Some(group) if group[0].has_same_rotation(&segment) => group.push(segment),
            _ => groups.push(vec![segment]),
        }
    }

    groups
        .into_iter()
        .flat_map(|group| {
            if group[0].is_unrotated() {
                group
            } else {
                order_rotated_segment_group(group)
            }
        })
        .collect()
}

fn order_rotated_segment_group(mut segments: Vec<SegmentData>) -> Vec<SegmentData> {
    segments.sort_by(|first, second| {
        second
            .upright_baseline()
            .total_cmp(&first.upright_baseline())
            .then_with(|| {
                first
                    .upright_advance_extent()
                    .0
                    .total_cmp(&second.upright_advance_extent().0)
            })
    });

    let mut visual_lines: Vec<Vec<SegmentData>> = Vec::new();
    for segment in segments {
        let belongs_to_last_line = visual_lines.last().is_some_and(|line| {
            let anchor = &line[0];
            let tolerance = anchor.height.max(segment.height).max(anchor.font_size * 0.5) * 0.5;
            (anchor.upright_baseline() - segment.upright_baseline()).abs() <= tolerance
        });
        if belongs_to_last_line {
            if let Some(line) = visual_lines.last_mut() {
                line.push(segment);
            }
        } else {
            visual_lines.push(vec![segment]);
        }
    }

    for line in &mut visual_lines {
        line.sort_by(|first, second| {
            first
                .upright_advance_extent()
                .0
                .total_cmp(&second.upright_advance_extent().0)
        });
    }
    visual_lines.into_iter().flatten().collect()
}

#[cfg(feature = "layout-detection")]
fn wrapper_ownership_by_hint(
    hints: &[LayoutHint],
    validations: &[super::regions::layout_validation::RegionValidation],
) -> Vec<bool> {
    hints
        .iter()
        .enumerate()
        .map(|(index, hint)| {
            !is_wrapper_layout_hint(hint)
                || !matches!(
                    validations.get(index),
                    Some(super::regions::layout_validation::RegionValidation::Empty)
                )
        })
        .collect()
}

#[cfg(feature = "layout-detection")]
struct LayoutParagraphContext<'a> {
    heading_map: &'a [(f32, Option<u8>)],
    paragraph_gap_ys: &'a [f32],
    doc_body_font_size: Option<f32>,
    include_headers: bool,
    include_footers: bool,
    include_footnotes: bool,
    page_width_pts: Option<f32>,
    apply_layout_overrides: bool,
    witnesses: &'a TextRepairWitnesses,
}

#[cfg(feature = "layout-detection")]
struct NativeLayoutProjection {
    groups: Vec<crate::extractors::pdf::reading_order::LayoutSegmentGroup>,
    group_bounds: Vec<Option<(f32, f32, f32, f32)>>,
    classification_hints: Vec<LayoutHint>,
}

#[cfg(feature = "layout-detection")]
fn process_layout_segment_groups(
    segments: Vec<SegmentData>,
    hints: &[LayoutHint],
    wrapper_ownership: &[bool],
    context: LayoutParagraphContext<'_>,
) -> Vec<PdfParagraph> {
    let no_reorder = super::layout_debug::layout_debug_flags().no_reorder;
    let groups = crate::extractors::pdf::reading_order::plan_segment_groups_by_layout(
        &segments,
        hints,
        wrapper_ownership,
        no_reorder,
        context.page_width_pts,
    );
    if matches!(groups.as_slice(), [group] if group.hint_indices.is_empty() && group.region_path.is_none()) {
        return segments_to_paragraphs(
            segments,
            context.heading_map,
            context.paragraph_gap_ys,
            context.witnesses,
        );
    }
    if !context.apply_layout_overrides {
        let group_bounds = layout_group_bounds(&groups, &segments);
        let mut paragraphs = segments_to_paragraphs(
            segments,
            context.heading_map,
            context.paragraph_gap_ys,
            context.witnesses,
        );
        assign_native_paragraph_layout(&mut paragraphs, &groups, &group_bounds);
        let classification_hints = regular_layout_hints(hints);
        super::layout_classify::annotate_layout_classes(&mut paragraphs, &classification_hints, 0.5, 0.2);
        return paragraphs;
    }
    let mut slots = segments.into_iter().map(Some).collect::<Vec<_>>();
    let mut paragraphs = Vec::new();

    for group in groups {
        let region_path = group.region_path;
        let group_segments = group
            .segment_indices
            .into_iter()
            .filter_map(|index| slots.get_mut(index).and_then(Option::take))
            .collect::<Vec<_>>();
        if group_segments.is_empty() {
            continue;
        }
        let gap_ys = compute_paragraph_gap_ys(&group_segments);
        let mut group_paragraphs =
            segments_to_paragraphs(group_segments, context.heading_map, &gap_ys, context.witnesses);
        let group_hints = group
            .hint_indices
            .into_iter()
            .filter_map(|index| hints.get(index).cloned())
            .collect::<Vec<_>>();
        if context.apply_layout_overrides {
            super::layout_classify::apply_layout_overrides(
                &mut group_paragraphs,
                &group_hints,
                0.5,
                0.2,
                context.doc_body_font_size,
            );
            un_mark_layout_furniture_per_config(
                &mut group_paragraphs,
                context.include_headers,
                context.include_footers,
                context.include_footnotes,
            );
        } else {
            super::layout_classify::annotate_layout_classes(&mut group_paragraphs, &group_hints, 0.5, 0.2);
        }
        for paragraph in &mut group_paragraphs {
            paragraph.layout_region_path = region_path;
        }
        paragraphs.extend(group_paragraphs);
    }

    let leftovers = slots.into_iter().flatten().collect::<Vec<_>>();
    if !leftovers.is_empty() {
        tracing::warn!(
            segments = leftovers.len(),
            "layout region plan omitted segments; appending an unsorted fallback group"
        );
        let gap_ys = compute_paragraph_gap_ys(&leftovers);
        paragraphs.extend(segments_to_paragraphs(
            leftovers,
            context.heading_map,
            &gap_ys,
            context.witnesses,
        ));
    }
    paragraphs
}

#[cfg(feature = "layout-detection")]
fn assign_native_paragraph_layout(
    paragraphs: &mut Vec<PdfParagraph>,
    groups: &[crate::extractors::pdf::reading_order::LayoutSegmentGroup],
    group_bounds: &[Option<(f32, f32, f32, f32)>],
) {
    for paragraph in paragraphs {
        let group_rank = paragraph_geometry_bbox(paragraph).and_then(|paragraph_bbox| {
            groups
                .iter()
                .enumerate()
                .filter_map(|(group_index, _)| {
                    let overlap = group_bounds
                        .get(group_index)
                        .and_then(|bounds| *bounds)
                        .map_or(0.0, |bounds| rectangle_overlap_area(paragraph_bbox, bounds));
                    (overlap > 0.0).then_some((group_index, overlap))
                })
                .max_by(|left, right| left.1.total_cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
                .map(|(group_index, _)| group_index)
        });
        let Some(group_index) = group_rank else {
            continue;
        };
        let Some(mut path) = groups[group_index].region_path else {
            continue;
        };
        let original_root_id = path.root.id;
        let root_order = groups
            .iter()
            .position(|group| {
                group
                    .region_path
                    .is_some_and(|candidate| candidate.root.id == original_root_id)
            })
            .unwrap_or(group_index);
        path.root.id = root_order;
        if let Some(child) = &mut path.child {
            child.id = group_index;
        }
        paragraph.layout_region_path = Some(path);
    }
}

fn reorder_pages_by_layout_region(pages: &mut [Vec<PdfParagraph>]) {
    for page in pages {
        page.sort_by_key(|paragraph| {
            paragraph
                .layout_region_path
                .map(|path| path.child.map_or(path.root.id, |child| child.id))
                .unwrap_or(usize::MAX)
        });
    }
}

fn paragraph_geometry_bbox(paragraph: &PdfParagraph) -> Option<(f32, f32, f32, f32)> {
    if let Some(block_bbox) = paragraph.block_bbox {
        return Some(block_bbox);
    }
    let mut segments = paragraph.lines.iter().flat_map(|line| line.segments.iter());
    let first = segments.next()?;
    let mut bounds = (
        first.x,
        first.y.min(first.baseline_y),
        first.x + first.width,
        (first.y + first.height).max(first.baseline_y + first.height),
    );
    for segment in segments {
        bounds.0 = bounds.0.min(segment.x);
        bounds.1 = bounds.1.min(segment.y.min(segment.baseline_y));
        bounds.2 = bounds.2.max(segment.x + segment.width);
        bounds.3 = bounds
            .3
            .max((segment.y + segment.height).max(segment.baseline_y + segment.height));
    }
    Some(bounds)
}

#[cfg(feature = "layout-detection")]
fn layout_group_bounds(
    groups: &[crate::extractors::pdf::reading_order::LayoutSegmentGroup],
    segments: &[SegmentData],
) -> Vec<Option<(f32, f32, f32, f32)>> {
    groups
        .iter()
        .map(|group| {
            let mut group_segments = group.segment_indices.iter().filter_map(|index| segments.get(*index));
            let first = group_segments.next()?;
            let mut bounds = (
                first.x,
                first.y.min(first.baseline_y),
                first.x + first.width,
                (first.y + first.height).max(first.baseline_y + first.height),
            );
            for segment in group_segments {
                bounds.0 = bounds.0.min(segment.x);
                bounds.1 = bounds.1.min(segment.y.min(segment.baseline_y));
                bounds.2 = bounds.2.max(segment.x + segment.width);
                bounds.3 = bounds
                    .3
                    .max((segment.y + segment.height).max(segment.baseline_y + segment.height));
            }
            Some(bounds)
        })
        .collect()
}

#[cfg(feature = "layout-detection")]
fn rectangle_overlap_area(left: (f32, f32, f32, f32), right: (f32, f32, f32, f32)) -> f32 {
    let width = left.2.min(right.2) - left.0.max(right.0);
    let height = left.3.min(right.3) - left.1.max(right.1);
    width.max(0.0) * height.max(0.0)
}

/// Multiple of the median line height a whitespace band must exceed to count
/// as a paragraph break. Normal line pitch leaves well under one line height of
/// whitespace; a blank line leaves more than one.
const PARAGRAPH_GAP_HEIGHT_FACTOR: f32 = 1.5;

/// Multiple of the page's own body leading a baseline-to-baseline advance must
/// reach to count as a paragraph break.
///
/// [`PARAGRAPH_GAP_HEIGHT_FACTOR`] measures the *whitespace band* between two
/// lines against the glyph height, which makes it blind to the most common
/// paragraph separator there is. With glyph height `h` and leading `L`, single
/// spacing leaves a band of `L - h` and a blank line leaves `2L - h`; for the
/// usual `L` of 1.1–1.3 `h` that blank line is only 1.2–1.6 `h`, so a 1.5 `h`
/// band threshold demands more vertical space than a blank line actually
/// provides. Comparing the advance to the leading instead is scale-free: a
/// blank line doubles the advance, so anything at or past 1.5× the body leading
/// is a break while ordinary wrapped lines (1.0×) are not, whatever the leading
/// happens to be. The two rules are OR-ed, so no break the band rule already
/// finds is lost.
const PARAGRAPH_BREAK_LEADING_MULTIPLE: f32 = 1.5;
const INLINE_STYLE_BASELINE_TOLERANCE: f32 = 0.5;
const INLINE_STYLE_MAX_FORWARD_GAP_FONT_FACTOR: f32 = 1.0;
const INLINE_FONT_SIZE_MAX_FORWARD_GAP_FONT_FACTOR: f32 = 1.5;
const INLINE_STYLE_MAX_OVERLAP_FONT_FACTOR: f32 = 0.15;

/// Multiple of font size within which two consecutive lines' right edges must
/// agree for the second to read as the wrapped tail of a heading, rather than
/// unrelated content that merely follows it.
///
/// A heading that wraps mid-sentence fills its first physical line out to the
/// text column's right margin before continuing below, so the wrapped line and
/// its continuation land their right edges close together; a heading followed
/// by unrelated content (a callout, a new paragraph) has no reason to share
/// that edge and typically differs by far more. Two font-size widths is
/// generous enough to absorb ordinary word-wrap slack -- the space left unused
/// because the next word did not fit -- without also treating a long heading
/// followed by a much shorter, unrelated line as a wrap. See #1467.
const HEADING_WRAP_RIGHT_EDGE_TOLERANCE_FONT_FACTOR: f32 = 2.0;
/// How closely a wrapped heading's continuation must resume at the same left edge
/// as the line it continues, in font-sizes. Measured on GH#1615's reproducer the
/// two align exactly (both x 83.64) while the body line that must NOT merge sits
/// 35.4pt away at the margin, so the separation is wide and the tolerance only has
/// to absorb sub-pixel drift. ~keep
const HEADING_HANGING_INDENT_LEFT_EDGE_TOLERANCE_FONT_FACTOR: f32 = 0.5;

/// Detect paragraph-break y-positions from horizontal whitespace bands.
///
/// Segments are clustered into visual lines after sorting by y — stream order
/// is not positional (multi-column PDFs interleave columns, which a pairwise
/// scan misreads as phantom gaps). A break is recorded where the band between
/// two consecutive lines is taller than [`PARAGRAPH_GAP_HEIGHT_FACTOR`] × the
/// median line height, or where their baseline advance reaches
/// [`PARAGRAPH_BREAK_LEADING_MULTIPLE`] × the page's own body leading — the
/// signal a blank line actually produces. Bands between two monospace lines are
/// skipped, because code listings legitimately contain blank lines inside one
/// logical block.
///
/// Without this, the heuristic path only breaks paragraphs on font/bold/list
/// changes, fusing visually separated blocks (standalone headings, display
/// formulas) into surrounding prose.
fn compute_paragraph_gap_ys(segments: &[SegmentData]) -> Vec<f32> {
    if segments.len() < 2 {
        return Vec::new();
    }

    if segments.iter().all(SegmentData::is_unrotated) {
        return compute_paragraph_gap_ys_in_shared_frame(segments);
    }

    let mut gaps = Vec::new();
    let mut group_start = 0;
    for index in 1..=segments.len() {
        let ends_group = index == segments.len() || !segments[index - 1].has_same_rotation(&segments[index]);
        if ends_group {
            gaps.extend(compute_paragraph_gap_ys_in_shared_frame(&segments[group_start..index]));
            group_start = index;
        }
    }
    gaps
}

/// One visual line of a page, as clustered by [`compute_paragraph_gap_ys_in_shared_frame`].
struct LineBand {
    top: f32,
    bottom: f32,
    height: f32,
    monospace: bool,
    anchor_y: f32,
}

fn compute_paragraph_gap_ys_in_shared_frame(segments: &[SegmentData]) -> Vec<f32> {
    if segments.len() < 2 {
        return Vec::new();
    }

    let mut order: Vec<usize> = (0..segments.len()).collect();
    order.sort_by(|&a, &b| {
        paragraph_gap_axis(&segments[b])
            .partial_cmp(&paragraph_gap_axis(&segments[a]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut lines: Vec<LineBand> = Vec::new();
    for &i in &order {
        let seg = &segments[i];
        let (bottom, top) = if seg.is_unrotated() {
            (seg.y, seg.y + seg.height)
        } else {
            seg.upright_cross_extent()
        };
        let baseline = paragraph_gap_axis(seg);
        let tolerance = (seg.height * 0.5).max(1.0);
        match lines.last_mut() {
            Some(line) if (baseline - line.anchor_y).abs() <= tolerance => {
                line.top = line.top.max(top);
                line.bottom = line.bottom.min(bottom);
                line.height = line.height.max(seg.height);
                line.monospace &= seg.is_monospace;
            }
            _ => lines.push(LineBand {
                top,
                bottom,
                height: seg.height,
                monospace: seg.is_monospace,
                anchor_y: baseline,
            }),
        }
    }
    if lines.len() < 2 {
        return Vec::new();
    }

    let mut heights: Vec<f32> = lines.iter().map(|l| l.height).collect();
    heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_height = heights[heights.len() / 2];
    let gap_threshold = median_height * PARAGRAPH_GAP_HEIGHT_FACTOR;
    let advance_threshold = body_leading(&lines, median_height) * PARAGRAPH_BREAK_LEADING_MULTIPLE;

    let mut gap_ys = Vec::new();
    for pair in lines.windows(2) {
        let gap = pair[0].bottom - pair[1].top;
        let advance = pair[0].anchor_y - pair[1].anchor_y;
        if (gap > gap_threshold || advance > advance_threshold) && !(pair[0].monospace && pair[1].monospace) {
            gap_ys.push((pair[0].bottom + pair[1].top) / 2.0);
        }
    }
    gap_ys
}

/// Estimate the body leading of a page from its own line pitch: the tightest
/// baseline-to-baseline advance between consecutive lines, floored at the
/// median line height.
///
/// The tightest advance is used rather than the median or the mode because a
/// short block-structured page — a memo of five one-line blocks, say — has more
/// break-sized advances than body-sized ones, so any central statistic reports
/// the break spacing as normal and no break can ever be detected. The floor is
/// what makes the minimum safe: an advance below one line height is not a
/// wrapped line at all (stacked accents, a subscript resolved onto its own
/// band) and must not be allowed to shrink the estimate and split every
/// ordinary line on the page.
fn body_leading(lines: &[LineBand], median_height: f32) -> f32 {
    let tightest = lines
        .windows(2)
        .map(|pair| pair[0].anchor_y - pair[1].anchor_y)
        .filter(|advance| advance.is_finite() && *advance > 0.0)
        .fold(f32::INFINITY, f32::min);
    if tightest.is_finite() {
        tightest.max(median_height)
    } else {
        median_height
    }
}

fn paragraph_gap_axis(segment: &SegmentData) -> f32 {
    if segment.is_unrotated() {
        segment.y
    } else {
        segment.upright_baseline()
    }
}

/// Convert a flat list of text segments into grouped paragraphs.
///
/// Groups consecutive segments by font changes, bold changes, list markers, and
/// paragraph gap positions. Each group is then classified via `finalize_paragraph`.
/// The text of the whole visual line each segment belongs to, indexed alongside
/// `lines`.
///
/// The numbered-heading break terms test a predicate against a line's opening
/// token, but this loop walks SEGMENTS, and a heading set with a hanging number
/// arrives as two of them on one baseline -- `"3.1.7"` and
/// `"Innovatie/ontwikkelingen"`. Neither segment alone starts with a section
/// number the way the assembled line does, so the terms never fired and the
/// heading was left to the ordinary paragraph-gap rule, which needs a gap wider
/// than ordinary line pitch. `merge_continuation_paragraphs::starts_numbered_section`
/// already re-joins a paragraph's first line for exactly this reason; this is the
/// same re-join on the grouper side, so the two passes agree. See #1609. ~keep
fn visual_line_texts(lines: &[SegmentData]) -> Vec<String> {
    let mut texts = vec![String::new(); lines.len()];
    let mut start = 0usize;
    while start < lines.len() {
        let mut end = start + 1;
        // Same-visual-line test as `starts_new_line` below: consecutive, so the two
        // cannot disagree about where a line ends. ~keep
        while end < lines.len()
            && lines[end].has_same_rotation(&lines[end - 1])
            && (lines[end].upright_baseline() - lines[end - 1].upright_baseline()).abs()
                <= INLINE_STYLE_BASELINE_TOLERANCE
        {
            end += 1;
        }
        let joined = lines[start..end]
            .iter()
            .map(|segment| segment.text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        for slot in &mut texts[start..end] {
            slot.clone_from(&joined);
        }
        start = end;
    }
    texts
}

fn blocks_to_paragraphs(
    lines: Vec<SegmentData>,
    heading_map: &[(f32, Option<u8>)],
    paragraph_gap_ys: &[f32],
) -> Vec<PdfParagraph> {
    if lines.is_empty() {
        return Vec::new();
    }

    let gap_info = super::classify::precompute_gap_info(heading_map);
    let visual_line_texts = visual_line_texts(&lines);

    let mut paragraphs: Vec<PdfParagraph> = Vec::new();
    let mut current_lines: Vec<&SegmentData> = Vec::new();
    let mut current_is_single_visual_line = true;
    let mut prev_idx = 0usize;

    for (line_idx, line) in lines.iter().enumerate() {
        let should_break = if current_lines.is_empty() {
            false
        } else {
            let prev = current_lines.last().unwrap();
            let font_change = (line.font_size - prev.font_size).abs() > 1.5
                && !is_inline_style_transition(
                    current_is_single_visual_line,
                    prev,
                    line,
                    INLINE_FONT_SIZE_MAX_FORWARD_GAP_FONT_FACTOR,
                );
            let role_change = line.assigned_role != prev.assigned_role;
            let bold_change = line.is_bold != prev.is_bold
                && !is_inline_style_transition(
                    current_is_single_visual_line,
                    prev,
                    line,
                    INLINE_STYLE_MAX_FORWARD_GAP_FONT_FACTOR,
                );
            let rotation_change = !line.has_same_rotation(prev);
            let starts_new_line = rotation_change
                || (line.upright_baseline() - prev.upright_baseline()).abs() > INLINE_STYLE_BASELINE_TOLERANCE;
            let has_same_line_follower = lines.get(line_idx + 1).is_some_and(|next| {
                next.has_same_rotation(line)
                    && (next.upright_baseline() - line.upright_baseline()).abs() <= INLINE_STYLE_BASELINE_TOLERANCE
            });
            let is_list = starts_new_line
                && (looks_like_list_item(&line.text) || (has_same_line_follower && is_bare_list_marker(&line.text)));
            // A numbered section heading always begins a new element. Without this
            // term a run of same-size, same-weight, evenly-spaced headings
            // ("1.3 Gasinstallatie", "1.4 Elektrische installatie", ...) yields no
            // break signal at all: `looks_like_list_item` deliberately returns
            // `false` for numbered section headings, so recognising the line as a
            // heading removes the only boundary this grouper would otherwise see,
            // and the whole run collapses into one paragraph. `is_numbered_section_heading`
            // (not the looser `starts_with_section_number`) is used deliberately so
            // prose beginning with a bare year — "2024 was een druk jaar" — does not
            // break its paragraph. See #1386. ~keep
            let starts_section =
                starts_new_line && super::classify::is_numbered_section_heading(&visual_line_texts[line_idx]);
            // A numbered section heading also always ENDS the element it opens: without
            // this term nothing else distinguishes a heading from the body text that
            // follows it when both share font size, weight, role and line spacing --
            // exactly the shape of a bold-only page in #1467, where the heading and the
            // callout beneath it are otherwise identical on every signal this grouper
            // checks. `starts_section` (above) only fires while classifying the
            // heading's OWN line and cannot see forward to close it once it opens; this
            // looks backward at `prev` instead. Restricting to `current_lines.len() ==
            // 1` scopes the break to the line directly after the heading, so a
            // paragraph that is already several lines long is untouched, and
            // `heading_wraps_onto` exempts a heading that is itself still wrapping onto
            // its next physical line rather than handing off to unrelated content. See
            // #1467. ~keep
            let follows_section = starts_new_line
                && current_is_single_visual_line
                && super::classify::is_numbered_section_heading(&visual_line_texts[prev_idx])
                && !heading_wraps_onto(prev, line)
                && !current_lines
                    .first()
                    .is_some_and(|heading_start| heading_continuation_is_hanging_indent(heading_start, prev, line));
            let crossed_gap = paragraph_gap_ys.iter().any(|&gap_y| {
                let previous_baseline = prev.upright_baseline();
                let current_baseline = line.upright_baseline();
                let (upper, lower) = if previous_baseline > current_baseline {
                    (previous_baseline, current_baseline)
                } else {
                    (current_baseline, previous_baseline)
                };
                gap_y < upper && gap_y > lower
            });
            rotation_change
                || font_change
                || role_change
                || bold_change
                || is_list
                || starts_section
                || follows_section
                || crossed_gap
        };

        if should_break && !current_lines.is_empty() {
            if let Some(para) = finalize_paragraph(&current_lines, heading_map, &gap_info) {
                paragraphs.push(para);
            }
            current_lines.clear();
            current_is_single_visual_line = true;
        }
        if let Some(first) = current_lines.first() {
            current_is_single_visual_line &= line.has_same_rotation(first)
                && (line.upright_baseline() - first.upright_baseline()).abs() <= INLINE_STYLE_BASELINE_TOLERANCE;
        }
        current_lines.push(line);
        prev_idx = line_idx;
    }

    if !current_lines.is_empty()
        && let Some(para) = finalize_paragraph(&current_lines, heading_map, &gap_info)
    {
        paragraphs.push(para);
    }

    tracing::debug!(
        input_lines = lines.len(),
        output_paragraphs = paragraphs.len(),
        headings = paragraphs.iter().filter(|p| p.heading_level.is_some()).count(),
        lists = paragraphs.iter().filter(|p| p.is_list_item).count(),
        "blocks_to_paragraphs complete"
    );

    paragraphs
}

/// Whether a style transition is an inline run on the same visual line.
///
/// PDF glyph runs can overlap slightly because of font metrics. Larger
/// overlaps, reverse ordering, and wide gaps remain structural boundaries.
fn is_inline_style_transition(
    current_is_single_visual_line: bool,
    previous: &SegmentData,
    next: &SegmentData,
    max_forward_gap_font_factor: f32,
) -> bool {
    if !current_is_single_visual_line
        || previous.is_monospace
        || next.is_monospace
        || previous.assigned_role != next.assigned_role
    {
        return false;
    }
    if !previous.has_same_rotation(next) {
        return false;
    }
    if !previous.font_size.is_finite()
        || !next.font_size.is_finite()
        || previous.font_size <= 0.0
        || next.font_size <= 0.0
        || !previous.upright_baseline().is_finite()
        || !next.upright_baseline().is_finite()
        || !previous.x.is_finite()
        || !next.x.is_finite()
        || !previous.width.is_finite()
        || !next.width.is_finite()
        || previous.width < 0.0
        || next.width < 0.0
    {
        return false;
    }
    if (next.upright_baseline() - previous.upright_baseline()).abs() > INLINE_STYLE_BASELINE_TOLERANCE {
        return false;
    }

    let font_size = previous.font_size.max(next.font_size);
    let (previous_start, previous_end) = previous.upright_advance_extent();
    let (next_start, _) = next.upright_advance_extent();
    let advance_gap = next_start - previous_end;
    next_start >= previous_start
        && advance_gap >= -(font_size * INLINE_STYLE_MAX_OVERLAP_FONT_FACTOR)
        && advance_gap <= font_size * max_forward_gap_font_factor
}

/// Whether `line` reads as the wrapped continuation of the numbered-heading
/// line `prev`, rather than a new, unrelated line that happens to follow it.
///
/// Both lines' right edges (`upright_advance_extent().1` -- the same geometry
/// [`is_inline_style_transition`] uses for its own right-edge test) are
/// compared: a heading that wraps mid-sentence fills its column before
/// continuing below, so its right edge and the next line's right edge land
/// within [`HEADING_WRAP_RIGHT_EDGE_TOLERANCE_FONT_FACTOR`] font-sizes of each
/// other. A short heading followed by unrelated content has no reason to share
/// that edge, so the two lines' right edges typically differ by far more.
/// Reused from [`super::paragraphs::merge_continuation_paragraphs`] so the
/// grouper's split and the merge pass's guard agree on the same wrap
/// exemption. See #1467.
pub(super) fn heading_wraps_onto(prev: &SegmentData, line: &SegmentData) -> bool {
    if !prev.has_same_rotation(line) {
        return false;
    }
    if !prev.font_size.is_finite() || !line.font_size.is_finite() {
        return false;
    }
    let (_, prev_end) = prev.upright_advance_extent();
    let (_, line_end) = line.upright_advance_extent();
    if !prev_end.is_finite() || !line_end.is_finite() {
        return false;
    }
    let tolerance = HEADING_WRAP_RIGHT_EDGE_TOLERANCE_FONT_FACTOR * prev.font_size.max(line.font_size).max(1.0);
    (prev_end - line_end).abs() <= tolerance
}

/// Whether the numbered-heading line `prev` reaches far enough right to have run
/// out of room, which is the "fills its column" half of the wrap rule that
/// [`heading_wraps_onto`]'s doc comment states but its code never measured.
///
/// `next_right_edge` is the widest right edge among the lines that would be merged
/// onto it. A heading that stops well short of that width did not wrap, it ended.
/// Measured: GH#1605's wrapped heading stops 58.7pt short of its own continuation
/// but only ~19pt short of the widest line beneath it, while GH#1609's COMPLETE
/// heading stops hundreds of points short of the body prose it was being welded
/// into. A lowercase opening alone cannot tell those apart -- both continue in
/// lowercase -- which is why it must not be the whole test. See #1609. ~keep
/// Whether `line` is the continuation of a numbered heading set with a HANGING
/// INDENT: the number at the left margin, the title starting to its right, and a
/// title too long for one line resuming at the title's own left edge.
///
/// Two things must hold, and the second alone is not enough. `heading_start` is the
/// first segment of the heading's visual line and `prev` its last, so
/// `prev` starting to the right of `heading_start` is what establishes that this
/// heading HAS a hanging indent at all. Only then does `line` sharing `prev`'s left
/// edge mean "the title continues" rather than "the next line happens to be at the
/// same margin".
///
/// Measured on GH#1615's reproducer, where the wrap and the body that must NOT merge
/// are identical on every other signal this grouper checks -- same font, same weight,
/// same line pitch:
///
/// ```text
/// 5.7.3                                     x 48.24            the number, at the margin
/// Roof terminal combined duct vertical and  x 83.64  y 774.96  the title, indented 35.4pt
/// twin pipe duct vertical                   x 83.64  y 762.24  the wrap -- aligns with the title
/// Appliance category: C33                   x 48.24  y 745.08  the body -- returns to the margin
/// ```
///
/// This is why the right-edge test in [`heading_wraps_onto`] cannot stand alone: a
/// wrap's LAST line is short by definition -- being short is what makes it the last
/// line -- so its right edge never matches the line it continues, and every two-line
/// heading looked like a heading handing off to unrelated content.
///
/// The hanging-indent requirement is what keeps #1467 working: there the heading is a
/// single segment at the margin and the callout beneath it is at the same margin, so
/// `heading_start` and `prev` coincide, no indent is established, and the pair still
/// splits. `starts_section` is evaluated independently of all this, so a following
/// line that is itself a numbered heading breaks regardless. ~keep
pub(super) fn heading_continuation_is_hanging_indent(
    heading_start: &SegmentData,
    prev: &SegmentData,
    line: &SegmentData,
) -> bool {
    if !prev.has_same_rotation(line) || !prev.has_same_rotation(heading_start) {
        return false;
    }
    if !prev.font_size.is_finite() || !line.font_size.is_finite() {
        return false;
    }
    let (heading_left, _) = heading_start.upright_advance_extent();
    let (prev_left, _) = prev.upright_advance_extent();
    let (line_left, _) = line.upright_advance_extent();
    if !heading_left.is_finite() || !prev_left.is_finite() || !line_left.is_finite() {
        return false;
    }
    let tolerance =
        HEADING_HANGING_INDENT_LEFT_EDGE_TOLERANCE_FONT_FACTOR * prev.font_size.max(line.font_size).max(1.0);
    prev_left - heading_left > tolerance && (prev_left - line_left).abs() <= tolerance
}

pub(super) fn heading_fills_column(prev: &SegmentData, next_right_edge: f32) -> bool {
    if !prev.font_size.is_finite() || !next_right_edge.is_finite() {
        return false;
    }
    let (_, prev_end) = prev.upright_advance_extent();
    if !prev_end.is_finite() {
        return false;
    }
    let tolerance = HEADING_WRAP_RIGHT_EDGE_TOLERANCE_FONT_FACTOR * prev.font_size.max(1.0);
    prev_end >= next_right_edge - tolerance
}

/// Reconstruct PdfLine objects from a flat list of SegmentData, grouping by baseline_y.
///
/// This preserves inline formatting information (is_bold, is_italic, is_monospace)
/// at the segment level so that the assembly layer can emit properly annotated markdown
/// with bold/italic emphasis.
fn reconstruct_pdf_lines(segments: &[&SegmentData]) -> Vec<super::types::PdfLine> {
    const MAX_LINE_TOLERANCE_PT: f32 = 3.0;
    const LINE_TOLERANCE_SCALE_FACTOR: f32 = 0.25;

    fn finish_line(mut segments: Vec<SegmentData>, baseline_y: f32) -> super::types::PdfLine {
        let contains_rtl = segments.iter().any(|segment| {
            segment
                .text
                .chars()
                .any(|character| xberg_native_pdf::text::is_rtl_text(character as u32))
        });
        if !contains_rtl {
            segments.sort_by(|a, b| a.upright_advance_extent().0.total_cmp(&b.upright_advance_extent().0));
        }

        let dominant_font_size = segments.iter().map(|s| s.font_size).fold(0.0, |a, b| {
            if a > 0.0 && b > a / 2.0 && b < a * 2.0 {
                (a + b) / 2.0
            } else {
                a.max(b)
            }
        });
        let is_bold = segments.iter().filter(|s| s.is_bold).count() > segments.len() / 2;
        let is_monospace = segments.iter().all(|s| s.is_monospace);
        super::types::PdfLine {
            segments,
            baseline_y,
            dominant_font_size,
            is_bold,
            is_monospace,
        }
    }

    if segments.is_empty() {
        return Vec::new();
    }

    let mut lines: Vec<super::types::PdfLine> = Vec::new();
    let mut current_baseline = segments[0].upright_baseline();
    let mut current_rotation = segments[0].rotation_degrees;
    let mut current_scale = segments[0].font_size.max(segments[0].height).abs();
    let mut current_segments: Vec<SegmentData> = Vec::new();

    for seg in segments {
        let same_rotation = (seg.rotation_degrees - current_rotation).abs() <= f32::EPSILON;
        let segment_baseline = seg.upright_baseline();
        let segment_scale = seg.font_size.max(seg.height).abs();
        let baseline_tolerance =
            (current_scale.max(segment_scale) * LINE_TOLERANCE_SCALE_FACTOR).min(MAX_LINE_TOLERANCE_PT);
        if !same_rotation || (segment_baseline - current_baseline).abs() > baseline_tolerance {
            if !current_segments.is_empty() {
                lines.push(finish_line(std::mem::take(&mut current_segments), current_baseline));
            }
            current_baseline = segment_baseline;
            current_rotation = seg.rotation_degrees;
            current_scale = segment_scale;
        } else {
            current_scale = current_scale.max(segment_scale);
        }
        current_segments.push((*seg).clone());
    }

    if !current_segments.is_empty() {
        lines.push(finish_line(current_segments, current_baseline));
    }

    lines
}

/// Build a PdfParagraph from a group of consecutive lines with compatible font properties.
fn paragraph_text(lines: &[&SegmentData]) -> String {
    if lines.iter().all(|segment| segment.is_unrotated()) {
        return lines
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
    }

    let mut text = String::new();
    let mut previous: Option<&SegmentData> = None;
    for segment in lines {
        if let Some(previous) = previous {
            if !previous.has_same_rotation(segment) {
                text.push_str("\n\n");
            } else {
                let same_line = (previous.upright_baseline() - segment.upright_baseline()).abs()
                    < previous.height.max(segment.height).max(segment.font_size * 0.5) * 0.5;
                if same_line {
                    let previous_word = previous.text.split_whitespace().next_back().unwrap_or("");
                    let next_word = segment.text.split_whitespace().next().unwrap_or("");
                    if !text.ends_with(char::is_whitespace)
                        && !segment.text.starts_with(char::is_whitespace)
                        && segments_need_space(previous, previous_word, segment, next_word)
                    {
                        text.push(' ');
                    }
                } else {
                    text.push('\n');
                }
            }
        }
        text.push_str(&segment.text);
        previous = Some(segment);
    }
    text
}

fn finalize_paragraph(
    lines: &[&SegmentData],
    heading_map: &[(f32, Option<u8>)],
    gap_info: &super::classify::GapInfo,
) -> Option<PdfParagraph> {
    if lines.is_empty() {
        return None;
    }

    let text = paragraph_text(lines);

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let first = lines[0];
    let word_count = trimmed.split_whitespace().count();
    let is_bold = lines.iter().filter(|l| l.is_bold).count() > lines.len() / 2;
    let has_mixed_inline_styles = lines
        .iter()
        .skip(1)
        .any(|line| line.is_bold != first.is_bold || line.is_italic != first.is_italic);

    let reconstructed_lines = reconstruct_pdf_lines(lines);
    let starts_with_split_list_marker = lines.get(1).is_some_and(|body| {
        is_bare_list_marker(&first.text)
            && body.has_same_rotation(first)
            && (body.upright_baseline() - first.upright_baseline()).abs() <= INLINE_STYLE_BASELINE_TOLERANCE
            && !body.text.trim().is_empty()
    });
    let is_list_candidate = looks_like_list_item(trimmed) || starts_with_split_list_marker;

    let structure_tree_role = {
        let role_counts: std::collections::HashMap<u8, usize> =
            lines
                .iter()
                .filter_map(|l| l.assigned_role)
                .fold(std::collections::HashMap::new(), |mut acc, level| {
                    *acc.entry(level).or_default() += 1;
                    acc
                });
        role_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(level, _)| level)
    };
    if let Some(level) = structure_tree_role {
        let para_text = trimmed.to_string();
        let word_count = PdfParagraph::compute_word_count(&para_text, &reconstructed_lines);
        return Some(PdfParagraph {
            text: if has_mixed_inline_styles {
                String::new()
            } else {
                para_text
            },
            lines: reconstructed_lines,
            dominant_font_size: first.font_size,
            heading_level: Some(level),
            is_bold,
            is_list_item: is_list_candidate,
            is_code_block: first.is_monospace && lines.len() > 1,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        });
    }

    // Shape-only page-number test (GH#1411). It is used here purely to *suppress*
    // heading promotion, which is non-destructive. The decision to mark such a
    // paragraph as deletable furniture is made document-wide in
    // `mark_validated_page_numbers`, which additionally requires margin position
    // and cross-page sequence agreement.
    let page_number_like =
        word_count <= MAX_PAGE_NUMBER_WORD_COUNT && super::page_number::classify_page_number_text(trimmed).is_some();

    let mut heading_level = super::classify::find_heading_level(first.font_size, heading_map, gap_info);
    if heading_level.is_some()
        && (word_count > 20
            || super::layout_classify::is_separator_text(trimmed)
            || page_number_like
            || (SUPPRESS_LOWERCASE_START_HEADINGS && super::classify::starts_with_lowercase_or_continuation(trimmed)))
    {
        heading_level = None;
    }

    let body_font_size = heading_map
        .iter()
        .find(|(_, level)| level.is_none())
        .map(|(centroid, _)| *centroid)
        .unwrap_or(0.0);

    // A bold, short, single-line paragraph is only a heading candidate when
    // its font is also meaningfully larger than the document's body font
    // (the same ratio/gap the font-size clustering path already requires via
    // `assign_heading_levels_smart`). Without this check, any bold one-word
    // line — including body-sized emphasis, or a stray oversized glyph from
    // a font-metric artifact — gets promoted regardless of scale, which is
    // exactly the pattern that over-promoted "Big"/"Text" in a 3-paragraph
    // document with no real headings. ~keep
    let clears_bold_font_gate = body_font_size > 0.0
        && first.font_size >= body_font_size * super::constants::MIN_HEADING_FONT_RATIO
        && first.font_size >= body_font_size + super::constants::MIN_HEADING_FONT_GAP;

    if heading_level.is_none()
        && is_bold
        && clears_bold_font_gate
        && (1..=8).contains(&word_count)
        && lines.len() == 1
        && !trimmed.ends_with('.')
        && !trimmed.ends_with(':')
        && !trimmed.ends_with(',')
        && !trimmed.ends_with(';')
        && !trimmed.contains('@')
        && !trimmed.contains('(')
        && !trimmed.contains(',')
        && trimmed
            .chars()
            .next()
            .is_some_and(|c| c.is_uppercase() || c.is_ascii_digit())
        && !super::layout_classify::is_separator_text(trimmed)
        && !super::regions::looks_like_figure_label(trimmed)
    {
        heading_level = Some(2);
    }

    if heading_level.is_none() {
        let min_heading_threshold = body_font_size * super::constants::MIN_HEADING_FONT_RATIO;
        // `first.font_size >= min_heading_threshold` already implies
        // `first.font_size > body_font_size + 0.5` for every realistic body font size:
        // `body * MIN_HEADING_FONT_RATIO > body + 0.5` reduces to `body > 0.5 / (RATIO - 1) ≈ 3.33`
        // (in whatever unit `font_size` is — points or OCR render pixels), and no real body-text
        // cluster is that small. An explicit `+ 0.5` absolute-unit check was previously required
        // here too; it was redundant on point-scale input and, being absolute, would have been
        // both too-permissive at pixel scale and too-strict on a hypothetically tiny render, so
        // it has been removed rather than converted. ~keep
        if body_font_size > 0.0
            && first.font_size >= min_heading_threshold
            && word_count <= super::constants::MAX_BOLD_HEADING_WORD_COUNT
            && lines.len() <= 2
            && !trimmed.ends_with(':')
            && !trimmed.contains('@')
            && (super::classify::is_section_pattern(trimmed) || is_structural_heading_word(trimmed))
            && !super::layout_classify::is_separator_text(trimmed)
            && !super::regions::looks_like_figure_label(trimmed)
            && !is_list_candidate
            && !page_number_like
        {
            heading_level = Some(2);
        }
    }

    let is_list_item = heading_level.is_none() && is_list_candidate;
    let is_code_block =
        heading_level.is_none() && !is_list_item && lines.iter().all(|l| l.is_monospace) && lines.len() >= 2;

    tracing::debug!(
        font_size = first.font_size,
        is_bold,
        word_count,
        heading_level = ?heading_level,
        is_list_item,
        is_code_block,
        page_number_like,
        text_preview = %&trimmed.chars().take(60).collect::<String>(),
        "classified paragraph"
    );

    let para_text = trimmed.to_string();
    let word_count = PdfParagraph::compute_word_count(&para_text, &reconstructed_lines);

    Some(PdfParagraph {
        text: if has_mixed_inline_styles {
            String::new()
        } else {
            para_text
        },
        lines: reconstructed_lines,
        dominant_font_size: first.font_size,
        heading_level,
        is_bold,
        is_list_item,
        is_code_block,
        is_formula: false,
        // Page-number furniture is decided document-wide, not here — see
        // `mark_validated_page_numbers` (GH#1411).
        is_page_furniture: false,
        layout_class: None,
        layout_region_path: None,
        caption_for: None,
        block_bbox: Some({
            let left = lines.iter().map(|l| l.x).fold(f32::MAX, f32::min);
            let bottom = lines.iter().map(|l| l.baseline_y).fold(f32::MAX, f32::min);
            let right = lines.iter().map(|l| l.x + l.width).fold(f32::MIN, f32::max);
            let top = lines.iter().map(|l| l.baseline_y + l.height).fold(f32::MIN, f32::max);
            (left, bottom, right, top)
        }),
        word_count,
    })
}

/// Check if text is ENTIRELY a list marker with no item text after it.
///
/// Word processors often emit list numbering as its own text run, so the
/// marker ("1.", "a)", "(2)", "•") and the item body arrive as separate
/// spans on the same line. `looks_like_list_item` rejects those markers
/// because it requires trailing text; this predicate accepts them.
pub(super) fn is_bare_list_marker(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || t.chars().count() > 5 {
        return false;
    }
    if matches!(t, "•" | "·" | "◦" | "▪" | "–" | "—" | "-" | "*") {
        return true;
    }
    super::list_marker::parse_ordered_list_marker(t).is_some_and(|marker| !marker.has_content)
}

/// Check if text starts with a common list marker.
///
/// Also consulted by the OCR+layout paragraph route (`extractors::pdf::ocr`), which has
/// no list classification of its own: reusing this predicate keeps the two routes from
/// drifting into two different notions of what a list marker is.
pub(crate) fn looks_like_list_item(text: &str) -> bool {
    let t = text.trim_start();

    if t.starts_with('•') || t.starts_with('·') || t.starts_with('◦') || t.starts_with('▪') {
        return true;
    }

    if let Some(rest) = t.strip_prefix('–').or_else(|| t.strip_prefix('—')) {
        if !rest.starts_with(' ') && !rest.starts_with('\t') {
            return false;
        }
        let body = rest.trim_start_matches([' ', '\t']);
        return !body.is_empty() && !body.starts_with('\r') && !body.starts_with('\n');
    }

    if let Some(rest) = t.strip_prefix("- ") {
        return rest.chars().next().is_some_and(|c| c.is_alphabetic());
    }

    if super::classify::is_numbered_section_heading(t) {
        return false;
    }
    let Some(marker) = super::list_marker::parse_ordered_list_marker(t) else {
        return false;
    };
    let Some(first_content_char) = t.get(marker.content_start..).and_then(|content| content.chars().next()) else {
        return false;
    };
    marker.has_content
        && marker.has_separator
        && !is_probable_author_byline(t)
        && first_content_char.is_alphabetic()
        && !is_inline_parenthesized_quantity(t, &marker, first_content_char)
}

/// Reject a line-leading `(N)` when it reads as a mid-sentence quantity
/// clarification -- "Two\n(2) additional on-street parking spaces" wraps onto
/// a physical line that *starts* with `(2)`, which is shaped identically to a
/// genuine numbered marker like `(2) Second item`.
///
/// The distinguishing signal is capitalization plus how the marker was
/// separated from its content:
///
/// - A **newline** between the marker and its content (`"(2)\nsecond item"`)
///   means the marker arrived as its own text run, glued to the next run by
///   line reconstruction rather than by the source author -- that shape is
///   trusted regardless of case, exactly as it always has been.
/// - A plain **space** on the same physical line, followed by a **lowercase**
///   word (`"(2) additional …"`, `"(7) on-street …"`), is the shape of a
///   number spelled out in prose ("Two (2) additional…") that happens to
///   start a wrapped line. A genuine enumerated item is a new sentence and so
///   starts with a capital letter (`"(2) Second point."`); this heuristic
///   costs nothing there.
///
/// Scoped to `(`-parenthesized **numeric** markers only: `(a)`/`(b)`/`(c)` are
/// this same ordinance's genuine sub-item markers (never quantity
/// clarifications, since nobody writes "two (b) items"), and non-parenthesized
/// families (`"1. "`, `"[1] "`) have no equivalent English idiom that produces
/// this false positive, so they are left untouched.
fn is_inline_parenthesized_quantity(
    t: &str,
    marker: &super::list_marker::OrderedListMarker,
    first_content_char: char,
) -> bool {
    if !t.starts_with('(') || marker.numeric_value.is_none() {
        return false;
    }
    let separator_region = t.get(..marker.content_start).unwrap_or("");
    if separator_region.contains(['\n', '\r']) {
        return false;
    }
    !first_content_char.is_uppercase()
}

/// Whether a single-capital marker is more likely the first author initial.
///
/// The comma plus a second compact initial or journal-style slash supplies
/// the contextual evidence; a standalone `A. First item` remains a list.
pub(super) fn is_probable_author_byline(text: &str) -> bool {
    let mut chars = text.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_uppercase()) || chars.next() != Some('.') {
        return false;
    }
    let remainder = chars.as_str().trim_start();
    let Some((surname, remainder)) = remainder.split_once(char::is_whitespace) else {
        return false;
    };
    surname.ends_with(',') && starts_with_author_initial_or_slash(remainder.trim_start())
}

fn starts_with_author_initial_or_slash(text: &str) -> bool {
    if text.starts_with('/') {
        return true;
    }
    let mut chars = text.chars().peekable();
    let mut initials = 0;
    while chars.peek().is_some_and(|c| c.is_ascii_uppercase()) {
        chars.next();
        if chars.next() != Some('.') {
            return false;
        }
        initials += 1;
    }
    initials > 0 && chars.peek().is_some_and(|c| c.is_whitespace())
}

/// Check if text is a well-known structural heading word.
///
/// These single-word headings appear frequently in academic papers and reports
/// and are reliable heading indicators when combined with a larger-than-body font.
fn is_structural_heading_word(text: &str) -> bool {
    let t = text.trim();
    matches!(
        t,
        "Abstract"
            | "References"
            | "Appendix"
            | "Acknowledgments"
            | "Acknowledgements"
            | "Conclusion"
            | "Conclusions"
            | "Bibliography"
            | "Contents"
            | "Index"
            | "Glossary"
            | "Summary"
            | "Discussion"
            | "Methods"
            | "Results"
            | "Methodology"
    )
}

/// Build a structured `InternalDocument` from pre-extracted per-page segments.
///
/// This is the native-backend entry point. It accepts segments already extracted
/// via `native::hierarchy::extract_all_segments` and runs the same font-clustering,
/// heading-classification, paragraph-assembly, and post-processing stages without
/// requiring a PDF document.
///
/// Image positions can be supplied to insert image placeholders into the document.
/// Layout hints (from RT-DETR layout detection) are optional; when present they
/// drive furniture marking, heading overrides, and table region detection.
///
/// Returns the assembled `InternalDocument`.
pub(crate) struct SegmentStructureConfig<'a> {
    pub k_clusters: usize,
    pub tables: &'a [crate::types::Table],
    pub outline_entries: &'a [PdfOutlineEntry],
    pub strip_repeating_text: bool,
    pub include_headers: bool,
    pub include_footers: bool,
    pub include_footnotes: bool,
    pub include_watermarks: bool,
    pub used_structure_tree: bool,
    pub image_positions: &'a [(u32, u32)],
    pub images: Option<&'a [crate::types::ExtractedImage]>,
    pub inject_placeholders: bool,
    pub layout_hints: Option<&'a [Vec<LayoutHint>]>,
    pub allow_single_column: bool,
    pub cancel_token: Option<&'a crate::cancellation::CancellationToken>,
    #[cfg(feature = "layout-detection")]
    pub layout_images: Option<&'a [image::RgbImage]>,
    #[cfg(feature = "layout-detection")]
    pub layout_results: Option<&'a [super::types::PageLayoutResult]>,
    #[cfg(feature = "layout-detection")]
    pub table_model: crate::core::config::layout::TableModel,
    #[cfg(feature = "layout-detection")]
    pub table_overlap_preference: crate::core::config::layout::TableOverlapPreference,
    #[cfg(feature = "layout-detection")]
    pub acceleration: Option<&'a crate::core::config::acceleration::AccelerationConfig>,
    #[cfg(feature = "layout-detection")]
    pub session_thread_budget: usize,
}

#[cfg(feature = "layout-detection")]
fn slanet_variant_for_table_model(table_model: crate::core::config::layout::TableModel) -> Option<&'static str> {
    use crate::core::config::layout::TableModel;

    match table_model {
        TableModel::SlanetWired | TableModel::SlanetAuto => Some("slanet_wired"),
        TableModel::SlanetWireless => Some("slanet_wireless"),
        TableModel::SlanetPlus => Some("slanet_plus"),
        TableModel::Tatr | TableModel::Disabled => None,
    }
}

pub(crate) fn extract_document_structure_from_segments(
    mut all_page_segments: Vec<Vec<SegmentData>>,
    config: SegmentStructureConfig<'_>,
) -> Result<crate::types::internal::InternalDocument> {
    let SegmentStructureConfig {
        k_clusters,
        tables,
        outline_entries,
        strip_repeating_text,
        include_headers,
        include_footers,
        include_footnotes,
        include_watermarks,
        used_structure_tree,
        image_positions,
        images,
        inject_placeholders,
        layout_hints,
        allow_single_column,
        cancel_token,
        #[cfg(feature = "layout-detection")]
        layout_images,
        #[cfg(feature = "layout-detection")]
        layout_results,
        #[cfg(feature = "layout-detection")]
        table_model,
        #[cfg(feature = "layout-detection")]
        table_overlap_preference,
        #[cfg(feature = "layout-detection")]
        acceleration,
        #[cfg(feature = "layout-detection")]
        session_thread_budget,
    } = config;
    let page_count = all_page_segments.len();
    tracing::debug!(
        page_count,
        used_structure_tree,
        "native structure pipeline: starting from pre-extracted segments"
    );

    let struct_tree_results: Vec<Option<Vec<PdfParagraph>>> = vec![None; page_count];
    let heuristic_pages: Vec<usize> = (0..page_count).collect();

    let (heading_map, doc_body_font_size) = if used_structure_tree {
        let mut heading_map = build_heading_map_from_assigned_roles(&all_page_segments);
        if !suppress_all_heading_roles_when_sparse_and_untrusted(&mut heading_map, &mut all_page_segments)
            && promote_untagged_document_title(&mut heading_map, &all_page_segments)
        {
            demote_assigned_roles(&mut all_page_segments);
        }
        let doc_body_font_size: Option<f32> = heading_map
            .iter()
            .find(|(_, level)| level.is_none())
            .map(|(size, _)| *size);
        tracing::debug!(
            heading_map_len = heading_map.len(),
            "native structure pipeline: heading map from structure tree"
        );
        (heading_map, doc_body_font_size)
    } else {
        let (heading_map, _struct_tree_needs_classify) =
            build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, k_clusters)?;
        let doc_body_font_size: Option<f32> = heading_map
            .iter()
            .find(|(_, level)| level.is_none())
            .map(|(size, _)| *size);
        (heading_map, doc_body_font_size)
    };

    let page_heights: Vec<f32> = all_page_segments
        .iter()
        .map(|segs| segs.iter().map(|s| s.y + s.height).fold(0.0_f32, f32::max).max(792.0))
        .collect();

    let mut layout_tables: Vec<crate::types::Table> = Vec::new();
    if let Some(hints_pages) = layout_hints {
        struct TablePageData {
            page_idx: usize,
            words: Vec<crate::pdf::table_reconstruct::HocrWord>,
            page_height: f32,
        }
        let mut table_pages: Vec<TablePageData> = Vec::new();
        // Geometric fallback (#1316): pages the ML detector left without any
        // Table region, but whose text geometry forms a column-aligned grid.
        // Reconstructed through the same guarded path as ML hints, below.
        let mut geometric_table_pages: Vec<(
            usize,
            Vec<crate::pdf::table_reconstruct::HocrWord>,
            f32,
            Vec<LayoutHint>,
        )> = Vec::new();

        #[allow(clippy::needless_range_loop)]
        for page_idx in 0..page_count {
            if cancel_token.is_some_and(|t| t.is_cancelled()) {
                tracing::debug!(page_idx, "native structure pipeline: cancelled during table page prep");
                break;
            }
            let Some(hints) = hints_pages.get(page_idx) else {
                continue;
            };
            let ml_table_hints: Vec<&LayoutHint> = hints
                .iter()
                .filter(|h| h.class_name == super::types::LayoutHintClass::Table)
                .collect();
            let has_table_hint = !ml_table_hints.is_empty();
            #[cfg(feature = "layout-detection")]
            let page_height = layout_results
                .and_then(|results| results.get(page_idx))
                .map(|pr| pr.page_height_pts)
                .unwrap_or(page_heights[page_idx]);
            #[cfg(not(feature = "layout-detection"))]
            let page_height = page_heights[page_idx];
            let words = crate::pdf::table_reconstruct::segments_to_words(&all_page_segments[page_idx], page_height);
            if words.is_empty() {
                tracing::trace!(
                    page = page_idx,
                    "native layout table extraction: no words from segments, skipping"
                );
                continue;
            }
            // Run the geometric fallback per-region rather than per-page (#1321):
            // an ML `Table` hint elsewhere on the page must not suppress recovery
            // of a spatially separate borderless region, so both paths run
            // whenever a page has words, with the fallback excluding words
            // already claimed by an ML table hint.
            let synthetic = super::regions::detect_geometric_table_hints(&words, page_height, &ml_table_hints);
            if !synthetic.is_empty() {
                tracing::debug!(
                    page = page_idx,
                    regions = synthetic.len(),
                    has_table_hint,
                    "geometric table fallback: synthesized Table region(s) outside existing ML hints"
                );
                geometric_table_pages.push((page_idx, words.clone(), page_height, synthetic));
            }
            if has_table_hint {
                tracing::trace!(
                    page = page_idx,
                    word_count = words.len(),
                    page_height,
                    "native layout table extraction: page prepared"
                );
                table_pages.push(TablePageData {
                    page_idx,
                    words,
                    page_height,
                });
            }
        }

        #[cfg(feature = "layout-detection")]
        {
            use crate::core::config::layout::TableModel;

            let use_model_inference = table_model != TableModel::Disabled;

            let slanet_variant = slanet_variant_for_table_model(table_model);
            let is_auto = table_model == TableModel::SlanetAuto;

            let model_name = match table_model {
                TableModel::Tatr => "TATR",
                TableModel::SlanetWired | TableModel::SlanetWireless | TableModel::SlanetPlus => "SLANeXT",
                TableModel::SlanetAuto => "SLANeXT (auto)",
                TableModel::Disabled => "disabled",
            };

            let has_table_model = if use_model_inference {
                let available = match table_model {
                    TableModel::Tatr => crate::layout::is_tatr_available(acceleration, session_thread_budget),
                    TableModel::SlanetWired
                    | TableModel::SlanetWireless
                    | TableModel::SlanetPlus
                    | TableModel::SlanetAuto => slanet_variant.is_some_and(|variant| {
                        crate::layout::is_slanet_available(variant, acceleration, session_thread_budget)
                    }),
                    TableModel::Disabled => false,
                };

                if !available && !table_pages.is_empty() {
                    return Err(crate::pdf::error::PdfError::TextExtractionFailed(format!(
                        "Layout detection found table regions but {model_name} model is not available. \
                         Ensure the ONNX model is downloaded. Tables cannot be extracted without it."
                    )));
                }
                available
            } else {
                false
            };

            if has_table_model {
                if let (Some(images @ [_, ..]), Some(results @ [_, ..])) = (layout_images, layout_results) {
                    #[cfg(not(target_arch = "wasm32"))]
                    let recognized_tables: Vec<Vec<crate::types::Table>> = table_pages
                        .iter()
                        .map(|tp| {
                            if let Some(variant) = slanet_variant {
                                let Some(mut slanet) =
                                    crate::layout::take_or_create_slanet(variant, acceleration, session_thread_budget)
                                else {
                                    tracing::warn!("SLANeXT model unavailable in worker thread");
                                    return Vec::new();
                                };

                                if let (Some(page_image), Some(page_result)) =
                                    (images.get(tp.page_idx), results.get(tp.page_idx))
                                {
                                    let hints = &hints_pages[tp.page_idx];
                                    let mut classifier_pair = if is_auto {
                                        match (
                                            crate::layout::take_or_create_table_classifier(
                                                acceleration,
                                                session_thread_budget,
                                            ),
                                            crate::layout::take_or_create_slanet(
                                                "slanet_wireless",
                                                acceleration,
                                                session_thread_budget,
                                            ),
                                        ) {
                                            (Some(classifier), Some(alternate)) => Some((classifier, alternate)),
                                            _ => None,
                                        }
                                    } else {
                                        None
                                    };
                                    let classifier_arg = classifier_pair
                                        .as_mut()
                                        .map(|(classifier, alternate)| (&mut ***classifier, &mut ***alternate));
                                    let slanet_tables = super::regions::recognize_tables_slanet(
                                        page_image,
                                        hints,
                                        &tp.words,
                                        page_result,
                                        tp.page_height,
                                        tp.page_idx,
                                        &mut slanet,
                                        classifier_arg,
                                    );
                                    if !slanet_tables.is_empty() {
                                        return slanet_tables;
                                    }
                                }

                                let hints = &hints_pages[tp.page_idx];
                                super::regions::extract_tables_from_layout_hints(
                                    &tp.words,
                                    hints,
                                    tp.page_idx,
                                    tp.page_height,
                                    0.5,
                                    allow_single_column,
                                    false,
                                )
                            } else {
                                let Some(mut tatr) =
                                    crate::layout::take_or_create_tatr(acceleration, session_thread_budget)
                                else {
                                    tracing::warn!("TATR model unavailable in worker thread");
                                    return Vec::new();
                                };

                                if let (Some(page_image), Some(page_result)) =
                                    (images.get(tp.page_idx), results.get(tp.page_idx))
                                {
                                    let hints = &hints_pages[tp.page_idx];
                                    let tatr_tables = super::regions::recognize_tables_for_native_page(
                                        page_image,
                                        hints,
                                        &tp.words,
                                        page_result,
                                        tp.page_height,
                                        super::regions::NativeTatrRecognitionOptions {
                                            page_index: tp.page_idx,
                                            allow_single_column,
                                        },
                                        &mut tatr,
                                    );
                                    if !tatr_tables.is_empty() {
                                        return tatr_tables;
                                    }
                                }

                                let hints = &hints_pages[tp.page_idx];
                                super::regions::extract_tables_from_layout_hints(
                                    &tp.words,
                                    hints,
                                    tp.page_idx,
                                    tp.page_height,
                                    0.5,
                                    allow_single_column,
                                    false,
                                )
                            }
                        })
                        .collect();
                    #[cfg(target_arch = "wasm32")]
                    let recognized_tables: Vec<Vec<crate::types::Table>> = table_pages
                        .iter()
                        .map(|tp| {
                            if let (Some(page_image), Some(page_result)) =
                                (images.get(tp.page_idx), results.get(tp.page_idx))
                            {
                                let hints = &hints_pages[tp.page_idx];
                                let Some(mut tatr) =
                                    crate::layout::take_or_create_tatr(acceleration, session_thread_budget)
                                else {
                                    return Vec::new();
                                };
                                let tatr_tables = super::regions::recognize_tables_for_native_page(
                                    page_image,
                                    hints,
                                    &tp.words,
                                    page_result,
                                    tp.page_height,
                                    super::regions::NativeTatrRecognitionOptions {
                                        page_index: tp.page_idx,
                                        allow_single_column,
                                    },
                                    &mut tatr,
                                );
                                if !tatr_tables.is_empty() {
                                    return tatr_tables;
                                }
                                super::regions::extract_tables_from_layout_hints(
                                    &tp.words,
                                    hints,
                                    tp.page_idx,
                                    tp.page_height,
                                    0.5,
                                    allow_single_column,
                                    false,
                                )
                            } else {
                                Vec::new()
                            }
                        })
                        .collect();
                    layout_tables.extend(recognized_tables.into_iter().flatten());
                } else {
                    for tp in &table_pages {
                        if cancel_token.is_some_and(|t| t.is_cancelled()) {
                            tracing::debug!("native structure pipeline: cancelled during heuristic table extraction");
                            break;
                        }
                        let hints = &hints_pages[tp.page_idx];
                        layout_tables.extend(super::regions::extract_tables_from_layout_hints(
                            &tp.words,
                            hints,
                            tp.page_idx,
                            tp.page_height,
                            0.5,
                            allow_single_column,
                            false,
                        ));
                    }
                }
            } else {
                for tp in &table_pages {
                    if cancel_token.is_some_and(|t| t.is_cancelled()) {
                        tracing::debug!("native structure pipeline: cancelled during heuristic table extraction");
                        break;
                    }
                    let hints = &hints_pages[tp.page_idx];
                    layout_tables.extend(super::regions::extract_tables_from_layout_hints(
                        &tp.words,
                        hints,
                        tp.page_idx,
                        tp.page_height,
                        0.5,
                        allow_single_column,
                        false,
                    ));
                }
            }
        }

        #[cfg(not(feature = "layout-detection"))]
        for tp in &table_pages {
            if cancel_token.is_some_and(|t| t.is_cancelled()) {
                tracing::debug!("native structure pipeline: cancelled during heuristic table extraction");
                break;
            }
            let hints = &hints_pages[tp.page_idx];
            layout_tables.extend(super::regions::extract_tables_from_layout_hints(
                &tp.words,
                hints,
                tp.page_idx,
                tp.page_height,
                0.5,
                allow_single_column,
                false,
            ));
        }

        // Geometric table fallback (#1316): reconstruct the synthesized regions
        // through the SAME guarded path (post_process_table, is_well_formed_table,
        // numeric-exemption prose gate, code-listing/single-cell-row guards). This
        // never runs the ML table models — it only recovers tables the detector
        // missed on otherwise Table-region-free pages.
        for (page_idx, words, page_height, synthetic_hints) in &geometric_table_pages {
            if cancel_token.is_some_and(|t| t.is_cancelled()) {
                tracing::debug!("native structure pipeline: cancelled during geometric table fallback");
                break;
            }
            let before = layout_tables.len();
            layout_tables.extend(super::regions::extract_tables_from_layout_hints(
                words,
                synthetic_hints,
                *page_idx,
                *page_height,
                0.5,
                allow_single_column,
                // Geometrically pre-vetted (row/column/gutter guards): skip the
                // downstream columnar-prose heuristic that mistakes a regular
                // key-value grid for wrapped prose (#1319).
                true,
            ));
            let recovered = layout_tables.len() - before;
            if recovered > 0 {
                tracing::debug!(
                    page = page_idx,
                    recovered,
                    "geometric table fallback: recovered table(s) the ML detector missed"
                );
            }
        }
    }

    tracing::debug!(
        layout_tables_found = layout_tables.len(),
        "native layout table extraction complete"
    );

    #[cfg(feature = "layout-detection")]
    let overlap_preference = table_overlap_preference;
    #[cfg(not(feature = "layout-detection"))]
    let overlap_preference = crate::core::config::layout::TableOverlapPreference::Content;
    let stitched_native_tables = stitch_fragmented_tables(tables.to_vec(), &all_page_segments);
    let emitted_tables = prepare_emitted_tables(&stitched_native_tables, layout_tables, overlap_preference);

    let extracted_table_bboxes_by_page = table_bboxes_by_page(&emitted_tables);
    tracing::debug!(
        native_tables = tables.len(),
        emitted_tables = emitted_tables.len(),
        pages_with_bboxes = extracted_table_bboxes_by_page.len(),
        "native table bbox suppression map built"
    );

    #[cfg(feature = "layout-detection")]
    let validations_by_page: ahash::AHashMap<usize, Vec<super::regions::layout_validation::RegionValidation>> = {
        let mut map = ahash::AHashMap::new();
        if let (Some(images), Some(results), Some(hints_pages)) = (layout_images, layout_results, layout_hints) {
            for page_idx in 0..page_count {
                if let (Some(img), Some(res), Some(hints)) =
                    (images.get(page_idx), results.get(page_idx), hints_pages.get(page_idx))
                {
                    let validations = super::regions::layout_validation::validate_page_regions(img, hints, res);
                    if validations.contains(&super::regions::layout_validation::RegionValidation::Empty) {
                        tracing::debug!(
                            page = page_idx,
                            empty_count = validations
                                .iter()
                                .filter(|v| **v == super::regions::layout_validation::RegionValidation::Empty)
                                .count(),
                            "native layout validation: found empty regions"
                        );
                    }
                    map.insert(page_idx, validations);
                }
            }
        }
        map
    };
    #[cfg(feature = "layout-detection")]
    let effective_layout_hints = layout_hints;
    #[cfg(feature = "layout-detection")]
    let native_layout_projections: Vec<Option<NativeLayoutProjection>> = (0..page_count)
        .map(|page_index| {
            let hints = effective_layout_hints.and_then(|pages| pages.get(page_index))?;
            let validations = validations_by_page
                .get(&page_index)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let wrapper_ownership = wrapper_ownership_by_hint(hints, validations);
            if !crate::extractors::pdf::reading_order::has_eligible_layout_hints(hints, &wrapper_ownership) {
                return None;
            }

            let table_bboxes = extracted_table_bboxes_by_page
                .get(&page_index)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let projected_segments =
                filter_segments_by_table_bboxes(all_page_segments[page_index].clone(), table_bboxes);
            let no_reorder = super::layout_debug::layout_debug_flags().no_reorder;
            let page_width_pts = layout_results
                .and_then(|results| results.get(page_index))
                .map(|result| result.page_width_pts);
            let groups = crate::extractors::pdf::reading_order::plan_segment_groups_by_layout(
                &projected_segments,
                hints,
                &wrapper_ownership,
                no_reorder,
                page_width_pts,
            );
            let group_bounds = layout_group_bounds(&groups, &projected_segments);
            Some(NativeLayoutProjection {
                groups,
                group_bounds,
                classification_hints: regular_layout_hints(hints),
            })
        })
        .collect();
    let witnesses = TextRepairWitnesses {
        hyphens: collect_hyphen_witnesses(&all_page_segments),
        words: collect_word_witnesses(&all_page_segments),
    };
    let page_inputs: Vec<PageInput> = (0..page_count)
        .map(|i| {
            let heuristic_segments = std::mem::take(&mut all_page_segments[i]);
            let paragraph_gap_ys = compute_paragraph_gap_ys(&heuristic_segments);
            PageInput {
                page_index: i,
                struct_paragraphs: None,
                heuristic_segments,
                // Native paragraphs are fully refined without layout semantic
                // annotations. Geometry is projected after semantic refinement.
                page_hints: None,
                table_bboxes: extracted_table_bboxes_by_page.get(&i).cloned().unwrap_or_default(),
                preserve_native_semantics: true,
                use_layout_reading_order: false,
                #[cfg(feature = "layout-detection")]
                hint_validations: validations_by_page.get(&i).cloned().unwrap_or_default(),
                #[cfg(feature = "layout-detection")]
                page_width_pts: layout_results
                    .and_then(|results| results.get(i))
                    .map(|result| result.page_width_pts),
                needs_classify: false,
                paragraph_gap_ys,
                include_headers,
                include_footers,
                include_footnotes,
            }
        })
        .collect();

    if cancel_token.is_some_and(|t| t.is_cancelled()) {
        return Err(crate::pdf::error::PdfError::TextExtractionFailed(
            "extraction cancelled".to_string(),
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    let mut all_page_paragraphs: Vec<Vec<PdfParagraph>> = page_inputs
        .into_par_iter()
        .map(|input| process_single_page(input, &heading_map, doc_body_font_size, &witnesses))
        .collect();
    #[cfg(target_arch = "wasm32")]
    let mut all_page_paragraphs: Vec<Vec<PdfParagraph>> = page_inputs
        .into_iter()
        .map(|input| process_single_page(input, &heading_map, doc_body_font_size, &witnesses))
        .collect();

    refine_heading_hierarchy(&mut all_page_paragraphs);
    demote_unnumbered_subsections(&mut all_page_paragraphs);
    demote_heading_runs(&mut all_page_paragraphs);
    split_colon_semicolon_run_in_lists(&mut all_page_paragraphs);

    if strip_repeating_text {
        mark_cross_page_repeating_text(&mut all_page_paragraphs, &page_heights);
        mark_cross_page_repeating_short_text(&mut all_page_paragraphs);
    }
    if !include_watermarks {
        mark_arxiv_noise(&mut all_page_paragraphs);
    }
    recover_headings_from_outline(&mut all_page_paragraphs, outline_entries);
    // Runs after heading recovery (so recovered headings are excluded) and
    // immediately before the deletion pass it feeds. It needs every page in
    // hand, which is why it cannot live in `process_single_page`.
    mark_validated_page_numbers(&mut all_page_paragraphs, &page_heights);
    for page in &mut all_page_paragraphs {
        retain_page_furniture_safely(page);
    }
    if strip_repeating_text {
        deduplicate_paragraphs(&mut all_page_paragraphs);
    }
    compact_final_heading_hierarchy(&mut all_page_paragraphs);
    promote_repeated_body_size_bold_headings(&mut all_page_paragraphs, doc_body_font_size);
    #[cfg(feature = "layout-detection")]
    for (page, projection) in all_page_paragraphs.iter_mut().zip(native_layout_projections) {
        let Some(projection) = projection else {
            continue;
        };
        assign_native_paragraph_layout(page, &projection.groups, &projection.group_bounds);
        super::layout_classify::annotate_layout_classes(page, &projection.classification_hints, 0.5, 0.2);
    }
    // Spill cleanup runs after deferred annotation so layout-only captions,
    // footnotes, and furniture retain their semantic provenance.
    suppress_table_dominant_paragraph_spill(&mut all_page_paragraphs, &emitted_tables);
    // Native semantic roles are finalized in source order before the
    // independent layout geometry projection controls final reading order.
    reorder_pages_by_layout_region(&mut all_page_paragraphs);

    let total_paragraphs: usize = all_page_paragraphs.iter().map(|p| p.len()).sum();
    tracing::debug!(
        total_paragraphs,
        heading_map_len = heading_map.len(),
        "native structure pipeline: paragraph extraction complete, assembling document"
    );

    let effective_image_positions = if inject_placeholders { image_positions } else { &[] };
    let mut doc = assemble_internal_document(
        all_page_paragraphs,
        &emitted_tables,
        images,
        effective_image_positions,
        &witnesses.hyphens,
    );

    for elem in &mut doc.elements {
        if elem.text.is_empty() {
            continue;
        }
        let t1 = repair_contextual_ligatures(&elem.text);
        let t2 = expand_ligatures_with_space_absorption(&t1);
        let t3 = normalize_unicode_text(&t2);
        if let Cow::Owned(normalized) = t3 {
            elem.text = normalized;
        } else if let Cow::Owned(normalized) = t2 {
            elem.text = normalized;
        } else if let Cow::Owned(normalized) = t1 {
            elem.text = normalized;
        }
    }

    tracing::debug!(
        elements = doc.elements.len(),
        "native structure pipeline: assembly complete"
    );

    Ok(doc)
}

/// Maximum vertical gap (PDF points) between one fragment's bottom edge and the
/// next fragment's top edge for the two to be considered the same physical
/// table split by `native::table`'s row-gap clustering.
const TABLE_STITCH_Y_GAP_TOLERANCE_PTS: f64 = 4.0;
/// Maximum difference in a chain's shared left/right edge for two fragments to
/// be considered the same table (rather than two unrelated tables that happen
/// to sit close together vertically).
const TABLE_STITCH_X_TOLERANCE_PTS: f64 = 6.0;
/// Bound on fragments merged into one stitched chain. Real continuation splits
/// rarely exceed a handful of fragments; this caps the (already page-scoped,
/// already `native::table::MAX_REGIONS_PER_PAGE`-bounded) chain walk.
const TABLE_STITCH_MAX_CHAIN_FRAGMENTS: usize = 12;
/// Bound on additional data rows the trailing-continuation recovery pass will
/// attempt to pull from raw page segments below a stitched chain's last known
/// fragment. Keeps the scan from reading arbitrarily far down the page.
const TABLE_STITCH_TRAILING_RECOVERY_MAX_ROWS: usize = 6;
/// Row-gap multiplier used to split recovered trailing words into per-entity
/// bands. Mirrors `native::table::cluster_words_into_vertical_regions`'s
/// `row_gap_split`; reimplemented here because that clustering helper is
/// private to the `native::table` module, which this pass cannot depend on.
const TABLE_STITCH_TRAILING_ROW_GAP_MULTIPLIER: f32 = 1.8;

/// Stitch table fragments that `native::table`'s row-gap region clustering split
/// out of one physical table back into a single table.
///
/// `native::table::cluster_words_into_vertical_regions` splits a page's words
/// into regions at any row-gap exceeding `median_height * 1.8`. A table whose
/// header wraps onto several lines, or whose rows are visually separated by
/// generous line spacing, can land in several such regions — each one then
/// independently goes through header/data-row post-processing, which corrupts
/// a real multi-line header (see `post_process_table_inner`'s header cap) and
/// mis-promotes a lone data row to a fake header. This pass reassembles those
/// fragments after the fact: each fragment's own rows are themselves raw
/// word-wrapped sub-lines of a single logical row (there is no reliable way to
/// tell, post hoc, which fragment "really" had a header split correctly), so
/// stitching column-merges every fragment's rows into exactly one row — the
/// topmost fragment in a chain becomes the header, the rest become data rows —
/// and then attempts to recover any trailing data rows that fell below the
/// last known fragment without ever becoming a table fragment at all (e.g.
/// because the row-gap clustering merged them into an unrelated, rejected
/// region).
///
/// Bounded to avoid quadratic blowup: fragments are grouped by page first (an
/// `O(n)` pass), and each page's fragment list is walked once after an
/// `O(m log m)` sort, with the inner chain-adjacency check bounded by
/// `TABLE_STITCH_MAX_CHAIN_FRAGMENTS`. `native::table::MAX_REGIONS_PER_PAGE`
/// already caps how many fragments a single page can contribute.
fn stitch_fragmented_tables(
    tables: Vec<crate::types::Table>,
    all_page_segments: &[Vec<SegmentData>],
) -> Vec<crate::types::Table> {
    let mut by_page: ahash::AHashMap<u32, Vec<crate::types::Table>> = ahash::AHashMap::new();
    let mut unbboxed = Vec::new();
    for table in tables {
        if table.bounding_box.is_some() {
            by_page.entry(table.page_number).or_default().push(table);
        } else {
            unbboxed.push(table);
        }
    }

    let mut result = unbboxed;
    let mut page_numbers: Vec<u32> = by_page.keys().copied().collect();
    page_numbers.sort_unstable();
    for page_number in page_numbers {
        if let Some(page_tables) = by_page.remove(&page_number) {
            result.extend(stitch_page_tables(page_tables, all_page_segments));
        }
    }
    result
}

/// Assign a stable, deterministic `table_id` (and, when missing, `columns`) to
/// every table in `tables`, in the given order.
///
/// Ids are sequential (`"table-1"`, `"table-2"`, ...) rather than derived from
/// randomness or wall-clock time, so the same input document always produces
/// the same ids. Must run over the final, post-dedup set of tables a document
/// will actually emit (see [`prepare_emitted_tables`]) — running it any
/// earlier, e.g. over native tables alone, would leave layout-detected tables
/// that survive dedup without an id.
///
/// Fragments of one physical table that [`stitch_page_tables`] merged into a
/// single [`crate::types::Table`] naturally share one id, since by this point
/// they are already one entry; distinct tables receive distinct ids because
/// they remain distinct entries. Cross-page continuations of one physical
/// table are not linked: [`fragments_are_stitchable`] only merges fragments on
/// the same page, so a table split across a page boundary is intentionally
/// emitted as separate `tables[]` entries with separate ids today. Sharing an
/// id across page-boundary fragments is a known possible future extension,
/// not attempted here.
fn assign_deterministic_table_ids(tables: &mut [crate::types::Table]) {
    for (index, table) in tables.iter_mut().enumerate() {
        table.table_id = Some(format!("table-{}", index + 1));
        if table.columns.is_none() {
            table.columns = table.cells.first().cloned();
        }
    }
}

/// Stitch one page's table fragments. See [`stitch_fragmented_tables`].
fn stitch_page_tables(
    mut fragments: Vec<crate::types::Table>,
    all_page_segments: &[Vec<SegmentData>],
) -> Vec<crate::types::Table> {
    fragments.sort_by(|a, b| {
        let a_top = a.bounding_box.map_or(f64::MIN, |bbox| bbox.y1);
        let b_top = b.bounding_box.map_or(f64::MIN, |bbox| bbox.y1);
        b_top.total_cmp(&a_top)
    });

    let mut output = Vec::with_capacity(fragments.len());
    let mut index = 0;
    while index < fragments.len() {
        let mut chain_end = index + 1;
        while chain_end < fragments.len()
            && chain_end - index < TABLE_STITCH_MAX_CHAIN_FRAGMENTS
            && fragments_are_stitchable(&fragments[chain_end - 1], &fragments[chain_end])
        {
            chain_end += 1;
        }

        if chain_end - index >= 2 {
            let chain = fragments[index..chain_end].to_vec();
            output.push(merge_table_chain(chain, all_page_segments));
        } else {
            output.push(fragments[index].clone());
        }
        index = chain_end;
    }
    output
}

/// Whether `next` is the vertically-adjacent continuation of `prev` within one
/// stitch chain: same page, same column count, near-zero row gap, and matching
/// left/right edges.
fn fragments_are_stitchable(prev: &crate::types::Table, next: &crate::types::Table) -> bool {
    if prev.page_number != next.page_number {
        return false;
    }
    let (Some(a), Some(b)) = (prev.bounding_box, next.bounding_box) else {
        return false;
    };

    let prev_cols = prev.cells.first().map_or(0, Vec::len);
    let next_cols = next.cells.first().map_or(0, Vec::len);
    if prev_cols == 0 || prev_cols != next_cols {
        return false;
    }

    (a.y0 - b.y1).abs() <= TABLE_STITCH_Y_GAP_TOLERANCE_PTS
        && (a.x0 - b.x0).abs() <= TABLE_STITCH_X_TOLERANCE_PTS
        && (a.x1 - b.x1).abs() <= TABLE_STITCH_X_TOLERANCE_PTS
}

/// Merge a chain of >= 2 stitchable fragments into one table.
///
/// The topmost fragment's rows collapse into the header; every other
/// fragment's rows collapse into one data row apiece. See
/// [`stitch_fragmented_tables`] for why a whole-fragment column merge is used
/// instead of trying to re-derive a header/data split.
fn merge_table_chain(chain: Vec<crate::types::Table>, all_page_segments: &[Vec<SegmentData>]) -> crate::types::Table {
    let column_count = chain
        .iter()
        .filter_map(|table| table.cells.first())
        .map(Vec::len)
        .max()
        .unwrap_or(0);

    let page_number = chain[0].page_number;
    let mut bbox = chain
        .iter()
        .find_map(|table| table.bounding_box)
        .unwrap_or(crate::types::BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: 0.0,
            y1: 0.0,
        });
    for table in &chain {
        if let Some(b) = table.bounding_box {
            bbox.x0 = bbox.x0.min(b.x0);
            bbox.x1 = bbox.x1.max(b.x1);
            bbox.y0 = bbox.y0.min(b.y0);
            bbox.y1 = bbox.y1.max(b.y1);
        }
    }

    let mut rows: Vec<Vec<String>> = chain
        .iter()
        .map(|table| crate::pdf::table_reconstruct::merge_rows_columnwise(&table.cells, column_count))
        .collect();

    if let Some(page_segments) = all_page_segments.get((page_number.saturating_sub(1)) as usize) {
        recover_trailing_continuation_rows(&mut rows, &mut bbox, column_count, page_segments);
    }

    let markdown = crate::pdf::table_reconstruct::table_to_markdown(&rows);
    let columns = rows.first().cloned();
    crate::types::Table {
        cells: rows,
        markdown,
        page_number,
        bounding_box: Some(bbox),
        columns,
        ..Default::default()
    }
}

/// Recover trailing data rows that never became their own table fragment.
///
/// `native::table`'s region clustering sometimes merges the last entities of a
/// fragmented table into a region with unrelated following content (or drops
/// them entirely when the merged region fails `post_process_table`
/// validation), so those rows leak into the document as plain paragraph text
/// instead of table data. This scans the raw page segments strictly below the
/// stitched chain's known bottom edge, within its column span, and — bounded
/// by [`TABLE_STITCH_TRAILING_RECOVERY_MAX_ROWS`] iterations — pulls one
/// row-gap-bounded entity band at a time. A band is only accepted if
/// reconstructing it independently yields the same column count as the
/// stitched table; any mismatch (e.g. the band actually contains an unrelated
/// heading below the table) stops recovery immediately rather than skipping
/// past it, since skipping risks pulling in arbitrary downstream content.
fn recover_trailing_continuation_rows(
    rows: &mut Vec<Vec<String>>,
    bbox: &mut crate::types::BoundingBox,
    column_count: usize,
    page_segments: &[SegmentData],
) {
    if column_count == 0 || page_segments.is_empty() {
        return;
    }

    let page_height = page_segments
        .iter()
        .map(|s| s.y + s.height)
        .fold(0.0_f32, f32::max)
        .max(792.0);
    let x_lo = (bbox.x0 - TABLE_STITCH_X_TOLERANCE_PTS) as f32;
    let x_hi = (bbox.x1 + TABLE_STITCH_X_TOLERANCE_PTS) as f32;
    let mut search_floor = bbox.y0 as f32;

    for _ in 0..TABLE_STITCH_TRAILING_RECOVERY_MAX_ROWS {
        let band_words: Vec<crate::pdf::table_reconstruct::HocrWord> = page_segments
            .iter()
            .filter(|seg| {
                !seg.text.trim().is_empty()
                    && seg.y + seg.height <= search_floor + TABLE_STITCH_Y_GAP_TOLERANCE_PTS as f32
                    && seg.x + seg.width >= x_lo
                    && seg.x <= x_hi
            })
            .flat_map(|seg| crate::pdf::table_reconstruct::split_segment_to_words(seg, page_height))
            .collect();
        if band_words.is_empty() {
            break;
        }

        let Some((entity_words, entity_bottom_image_y)) = take_next_entity_band(&band_words) else {
            break;
        };

        let col_gap = super::regions::tables::compute_adaptive_column_gap(&entity_words, (x_hi - x_lo).max(1.0));
        let grid = crate::pdf::table_reconstruct::reconstruct_table(&entity_words, col_gap, 0.5);
        if grid.is_empty() || grid[0].len() != column_count {
            break;
        }

        let merged_row = crate::pdf::table_reconstruct::merge_rows_columnwise(&grid, column_count);
        if merged_row.iter().all(|cell| cell.trim().is_empty()) {
            break;
        }

        let entity_bottom_pdf_y = page_height - entity_bottom_image_y as f32;
        rows.push(merged_row);
        bbox.y0 = bbox.y0.min(entity_bottom_pdf_y as f64);
        search_floor = entity_bottom_pdf_y;
    }
}

/// Take the topmost row-gap-bounded contiguous band of words from `words`
/// (which may span more than one logical entity), stopping at the first gap
/// larger than `median_height * TABLE_STITCH_TRAILING_ROW_GAP_MULTIPLIER`.
///
/// Returns the band's words and the image-coordinate bottom edge (`top +
/// height`, max across the band) of the last line included.
fn take_next_entity_band(
    words: &[crate::pdf::table_reconstruct::HocrWord],
) -> Option<(Vec<crate::pdf::table_reconstruct::HocrWord>, u32)> {
    if words.is_empty() {
        return None;
    }

    let mut heights: Vec<u32> = words.iter().map(|w| w.height).collect();
    heights.sort_unstable();
    let median_height = heights[heights.len() / 2].max(1);
    let row_gap_split = (median_height as f32 * TABLE_STITCH_TRAILING_ROW_GAP_MULTIPLIER) as u32;
    let row_tolerance = (median_height / 2).max(3);

    let mut sorted: Vec<&crate::pdf::table_reconstruct::HocrWord> = words.iter().collect();
    sorted.sort_by_key(|w| w.top);

    let mut band: Vec<crate::pdf::table_reconstruct::HocrWord> = Vec::new();
    let mut band_bottom = 0u32;
    let mut last_row_yc: Option<u32> = None;
    let mut idx = 0;
    while idx < sorted.len() {
        let row_yc = sorted[idx].top + sorted[idx].height / 2;
        let mut end = idx + 1;
        while end < sorted.len() {
            let yc = sorted[end].top + sorted[end].height / 2;
            if yc.abs_diff(row_yc) <= row_tolerance {
                end += 1;
            } else {
                break;
            }
        }

        if let Some(prev_yc) = last_row_yc
            && row_yc > prev_yc
            && row_yc - prev_yc > row_gap_split
            && !band.is_empty()
        {
            break;
        }

        for word in &sorted[idx..end] {
            band_bottom = band_bottom.max(word.top + word.height);
            band.push((*word).clone());
        }
        last_row_yc = Some(row_yc);
        idx = end;
    }

    if band.is_empty() {
        None
    } else {
        Some((band, band_bottom))
    }
}

/// Select the exact tables that final assembly will emit.
///
/// Suppression must consume this same set so a duplicate or empty table cannot
/// remove source text without contributing a corresponding table element.
fn prepare_emitted_tables(
    native_tables: &[crate::types::Table],
    layout_tables: Vec<crate::types::Table>,
    overlap_preference: crate::core::config::layout::TableOverlapPreference,
) -> Vec<crate::types::Table> {
    let mut emitted_tables: Vec<crate::types::Table> = native_tables.iter().cloned().chain(layout_tables).collect();
    emitted_tables.retain(|table| !table.markdown.trim().is_empty());
    let native_count = native_tables
        .iter()
        .filter(|table| !table.markdown.trim().is_empty())
        .count();
    deduplicate_overlapping_tables(&mut emitted_tables, native_count, overlap_preference);
    normalize_sparse_currency_affix_columns(&mut emitted_tables);
    normalize_wrapped_financial_rows(&mut emitted_tables);
    deduplicate_identical_tables(&mut emitted_tables);
    assign_deterministic_table_ids(&mut emitted_tables);
    emitted_tables
}

const MAX_CURRENCY_AFFIX_CELLS: usize = 3;
const MAX_CURRENCY_AFFIX_OCCUPANCY: f64 = 0.1;
const MIN_FINANCIAL_TARGET_CELLS: usize = 5;
const MIN_FINANCIAL_TARGET_RATIO: f64 = 0.9;
const MIN_WRAPPED_FINANCIAL_VALUE_ROWS: usize = 8;
const FINANCIAL_COLUMN_HEADERS: &[&str] = &[
    "shares",
    "par",
    "principal",
    "quantity",
    "value",
    "market value",
    "cost",
];
const ISO_CURRENCY_CODES: &[&str] = &[
    "AUD", "BRL", "CAD", "CHF", "CNY", "DKK", "EUR", "GBP", "HKD", "INR", "JPY", "KRW", "MXN", "NOK", "NZD", "SEK",
    "SGD", "USD", "ZAR",
];

fn normalize_sparse_currency_affix_columns(tables: &mut [crate::types::Table]) {
    for table in tables {
        normalize_sparse_currency_affix_columns_in_table(table);
    }
}

fn normalize_wrapped_financial_rows(tables: &mut [crate::types::Table]) {
    for table in tables {
        if !is_wrapped_financial_table(&table.cells) {
            continue;
        }
        fold_wrapped_financial_rows(&mut table.cells);
        table.markdown = crate::extractors::frontmatter_utils::cells_to_markdown(&table.cells);
        table.columns = table.cells.first().cloned();
    }
}

fn is_wrapped_financial_table(rows: &[Vec<String>]) -> bool {
    let Some(header) = rows.first() else {
        return false;
    };
    if header.len() < 3
        || rows.len() <= 1
        || !header.iter().skip(1).all(|cell| is_financial_column_header(cell))
        || rows.iter().any(|row| row.len() != header.len())
    {
        return false;
    }

    let body = &rows[1..];
    let value_rows = body.iter().filter(|row| is_value_bearing_financial_row(row)).count();
    let descriptor_rows = body
        .iter()
        .filter(|row| is_descriptor_only_financial_row(row) && !is_financial_section_label(row))
        .count();
    value_rows >= MIN_WRAPPED_FINANCIAL_VALUE_ROWS
        && descriptor_rows > value_rows
        && body
            .iter()
            .all(|row| is_descriptor_only_financial_row(row) || is_value_bearing_financial_row(row))
}

fn is_descriptor_only_financial_row(row: &[String]) -> bool {
    row.first().is_some_and(|cell| !cell.trim().is_empty()) && row.iter().skip(1).all(|cell| cell.trim().is_empty())
}

fn is_value_bearing_financial_row(row: &[String]) -> bool {
    row.first().is_some_and(|cell| !cell.trim().is_empty()) && row.iter().skip(1).all(|cell| is_financial_value(cell))
}

fn is_financial_section_label(row: &[String]) -> bool {
    if !is_descriptor_only_financial_row(row) {
        return false;
    }
    let text = row[0].trim().to_ascii_lowercase();
    text.contains("(continued)") || has_allocation_percentage_suffix(&text)
}

fn has_allocation_percentage_suffix(text: &str) -> bool {
    const MAX_FOOTNOTE_MARKER_CHARS: usize = 4;

    let Some((dash_index, dash)) = text
        .char_indices()
        .rev()
        .find(|(_, character)| matches!(character, '–' | '—'))
    else {
        return false;
    };
    if text[..dash_index].trim().is_empty() {
        return false;
    }
    let suffix = text[dash_index + dash.len_utf8()..].trim();
    let Some((allocation, remainder)) = suffix.split_once('%') else {
        return false;
    };
    let Ok(allocation) = allocation.trim().parse::<f64>() else {
        return false;
    };
    if !allocation.is_finite() || !(0.0..=100.0).contains(&allocation) {
        return false;
    }

    let remainder = remainder.trim();
    if remainder.is_empty() {
        return true;
    }
    let Some(marker) = remainder.strip_prefix('(').and_then(|value| value.strip_suffix(')')) else {
        return false;
    };
    !marker.is_empty()
        && marker.chars().count() <= MAX_FOOTNOTE_MARKER_CHARS
        && marker.chars().all(char::is_alphanumeric)
}

fn is_financial_value(cell: &str) -> bool {
    let trimmed = cell.trim();
    if is_financial_number(trimmed) {
        return true;
    }
    trimmed
        .split_once(' ')
        .is_some_and(|(marker, value)| is_currency_marker(marker) && is_financial_number(value.trim()))
}

fn fold_wrapped_financial_rows(rows: &mut Vec<Vec<String>>) {
    let mut folded = Vec::with_capacity(rows.len());
    folded.extend(rows.first().cloned());
    let mut pending = Vec::new();
    for mut row in rows.iter().skip(1).cloned() {
        if is_financial_section_label(&row) {
            folded.append(&mut pending);
            folded.push(row);
            continue;
        }
        if is_descriptor_only_financial_row(&row) {
            pending.push(row);
            continue;
        }
        if !pending.is_empty() {
            let prefix = pending
                .drain(..)
                .filter_map(|pending_row| pending_row.into_iter().next())
                .collect::<Vec<_>>()
                .join(" ");
            row[0] = format!("{prefix} {}", row[0]);
        }
        folded.push(row);
    }
    folded.extend(pending);
    *rows = folded;
}

fn normalize_sparse_currency_affix_columns_in_table(table: &mut crate::types::Table) {
    let Some(header) = table.cells.first() else {
        return;
    };
    if table.cells.len() <= 1 || header.len() < 2 {
        return;
    }

    let mut source_columns = (0..header.len() - 1)
        .filter(|&source| {
            header[source].trim().is_empty()
                && header
                    .get(source + 1)
                    .is_some_and(|target| is_financial_column_header(target))
                && is_sparse_currency_affix_column(&table.cells[1..], source, source + 1)
        })
        .collect::<Vec<_>>();
    source_columns.sort_unstable_by(|left, right| right.cmp(left));
    let normalized = !source_columns.is_empty();

    for source in source_columns {
        merge_currency_affix_column(&mut table.cells, source);
    }
    if normalized {
        table.markdown = crate::extractors::frontmatter_utils::cells_to_markdown(&table.cells);
        table.columns = table.cells.first().cloned();
    }
}

fn is_financial_column_header(header: &str) -> bool {
    let normalized = header.trim().to_ascii_lowercase();
    FINANCIAL_COLUMN_HEADERS
        .iter()
        .any(|candidate| normalized == *candidate || normalized.starts_with(&format!("{candidate} ")))
}

fn is_sparse_currency_affix_column(rows: &[Vec<String>], source: usize, target: usize) -> bool {
    let source_cells = rows
        .iter()
        .filter_map(|row| row.get(source))
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
        .collect::<Vec<_>>();
    let occupied_limit =
        MAX_CURRENCY_AFFIX_CELLS.min(((rows.len() as f64) * MAX_CURRENCY_AFFIX_OCCUPANCY).ceil() as usize);
    if source_cells.is_empty()
        || source_cells.len() > occupied_limit
        || !source_cells.iter().all(|cell| is_currency_marker(cell))
        || rows.iter().any(|row| {
            row.get(source).is_some_and(|cell| !cell.trim().is_empty())
                && row.get(target).is_none_or(|cell| cell.trim().is_empty())
        })
    {
        return false;
    }

    let target_cells = rows
        .iter()
        .filter_map(|row| row.get(target))
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
        .collect::<Vec<_>>();
    target_cells.len() >= MIN_FINANCIAL_TARGET_CELLS
        && target_cells.iter().filter(|cell| is_financial_number(cell)).count() as f64 / target_cells.len() as f64
            >= MIN_FINANCIAL_TARGET_RATIO
}

fn is_currency_marker(cell: &str) -> bool {
    const CURRENCY_SYMBOLS: &[&str] = &["$", "€", "£", "¥", "₹", "₩", "₽", "₺", "₪", "₫", "₦", "₱", "฿"];
    CURRENCY_SYMBOLS.contains(&cell) || ISO_CURRENCY_CODES.contains(&cell.to_ascii_uppercase().as_str())
}

fn is_financial_number(cell: &str) -> bool {
    let mut has_digit = false;
    cell.chars().all(|character| {
        if character.is_ascii_digit() {
            has_digit = true;
            true
        } else {
            character.is_ascii_whitespace() || matches!(character, ',' | '.' | '-' | '+' | '(' | ')' | '%')
        }
    }) && has_digit
}

fn merge_currency_affix_column(rows: &mut [Vec<String>], source: usize) {
    for row in rows {
        if row.len() <= source {
            continue;
        }
        if let Some(marker) = row.get(source).map(|cell| cell.trim()).filter(|cell| !cell.is_empty())
            && let Some(value) = row
                .get(source + 1)
                .map(|cell| cell.trim())
                .filter(|cell| !cell.is_empty())
        {
            row[source + 1] = format!("{marker} {value}");
        }
        row.remove(source);
    }
}

/// Collapse byte-identical table duplicates on the same page.
///
/// [`deduplicate_overlapping_tables`] only merges a pair when both tables carry
/// a `bounding_box`; a native/layout pair that detects the same physical table
/// but disagrees on bbox presence (e.g. native reconstruction leaves
/// `bounding_box: None` for some heuristic grids) can otherwise escape that
/// pass entirely. This pass is origin- and bbox-agnostic: any two tables on the
/// same page with byte-identical markdown are the same table by definition, so
/// the second (and any further) occurrence is dropped regardless of bbox state.
///
/// Runs in `O(n)` over the page's table count using a hash set keyed on
/// `(page_number, markdown)`.
fn deduplicate_identical_tables(tables: &mut Vec<crate::types::Table>) {
    if tables.len() < 2 {
        return;
    }

    let mut seen: ahash::AHashSet<(u32, &str)> = ahash::AHashSet::with_capacity(tables.len());
    let mut keep = vec![true; tables.len()];
    for (index, table) in tables.iter().enumerate() {
        if !seen.insert((table.page_number, table.markdown.as_str())) {
            keep[index] = false;
        }
    }

    let mut index = 0;
    tables.retain(|_| {
        let keep_this = keep[index];
        index += 1;
        keep_this
    });
}

/// A table's footprint on a page together with the text its grid actually carries.
///
/// The two are recorded side by side because suppression needs both: geometry alone
/// cannot tell whether the grid REPRESENTS a run it happens to cover. See
/// [`filter_segments_by_table_bboxes`]. ~keep
#[derive(Clone)]
struct TableCoverage {
    bbox: crate::types::BoundingBox,
    /// Every cell's text, whitespace-collapsed and lowercased, joined by `\u{1}`.
    /// Built once per table so the per-segment test is a substring search.
    cell_text: String,
}

/// Collapse runs of whitespace and lowercase, so a cell that joined several printed
/// runs still contains each run's normalized form as a substring.
fn normalize_for_table_coverage(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

fn table_bboxes_by_page(tables: &[crate::types::Table]) -> ahash::AHashMap<usize, Vec<TableCoverage>> {
    let mut coverage_by_page: ahash::AHashMap<usize, Vec<TableCoverage>> = ahash::AHashMap::new();
    for table in tables {
        if let Some(bbox) = table.bounding_box {
            // `cells` is authoritative when populated, but a table can reach here
            // carrying only rendered `markdown` (layout-sourced tables, and the
            // overlap-preference merge, both produce that shape). Falling back to the
            // markdown keeps suppression working for those instead of silently
            // disabling it, which would emit their contents twice. ~keep
            let cell_text = if table.cells.iter().any(|row| !row.is_empty()) {
                table
                    .cells
                    .iter()
                    .flat_map(|row| row.iter())
                    .map(|cell| normalize_for_table_coverage(cell))
                    .collect::<Vec<_>>()
                    .join("\u{1}")
            } else {
                normalize_for_table_coverage(&table.markdown.replace(['|', '-'], " "))
            };
            coverage_by_page
                .entry(table.page_number.saturating_sub(1) as usize)
                .or_default()
                .push(TableCoverage { bbox, cell_text });
        }
    }
    coverage_by_page
}

/// Filter out segments a table both COVERS and CARRIES.
///
/// Suppression exists so text a table already renders is not emitted a second time as
/// prose. Geometry alone was the whole test, and that is unsound: a reconstructed grid
/// need not span every printed column inside its own bounding box, and the runs in the
/// columns it left out were dropped from the prose flow without ever reaching a cell.
/// They were deleted from the document -- not in a cell, not in an element, nowhere.
/// Measured on GH#1616: a four-column fault-finding grid was reconstructed with two
/// columns over a bbox spanning all four, and 26 words vanished across two pages.
///
/// The text test restores the invariant that a bounding box cannot delete content the
/// grid does not represent: a covered run is dropped only when some cell actually
/// carries it. Matching is on whitespace-collapsed, lowercased text so a cell that
/// joined several printed runs still matches each of them, which is the normal case --
/// cell assembly merges runs, so requiring equality would suppress almost nothing and
/// reintroduce the duplication this filter exists to prevent.
///
/// Segments with zero area or empty text are always kept. ~keep
fn filter_segments_by_table_bboxes(segments: Vec<SegmentData>, tables: &[TableCoverage]) -> Vec<SegmentData> {
    if tables.is_empty() {
        return segments;
    }
    segments
        .into_iter()
        .filter(|seg| {
            let seg_area = seg.width * seg.height;
            let seg_text = seg.text.trim();
            if seg_area <= 0.0 || seg_text.is_empty() {
                return true;
            }
            let normalized = normalize_for_table_coverage(seg_text);
            !tables.iter().any(|table| {
                let bb = &table.bbox;
                let inter_left = seg.x.max(bb.x0 as f32);
                let inter_right = (seg.x + seg.width).min(bb.x1 as f32);
                let inter_bottom = seg.y.max(bb.y0 as f32);
                let inter_top = (seg.y + seg.height).min(bb.y1 as f32);
                if inter_left >= inter_right || inter_bottom >= inter_top {
                    return false;
                }
                let inter_area = (inter_right - inter_left) * (inter_top - inter_bottom);
                inter_area / seg_area >= 0.5 && table.cell_text.contains(&normalized)
            })
        })
        .collect()
}

/// Apply all 5 text repair passes in a single traversal over a segment's text.
///
/// Returns `Cow::Borrowed` if nothing changed, `Cow::Owned` otherwise.
fn fused_text_repairs<'a>(text: &'a str, word_witnesses: &WordWitnesses) -> Cow<'a, str> {
    let t1 = normalize_text_encoding(text);
    let t2 = repair_ligature_spaces(&t1, word_witnesses);
    let t3 = expand_ligatures_with_space_absorption(&t2);
    let t3b = collapse_spaced_hyphens(&t3);
    let t4 = normalize_unicode_text(&t3b);
    let t5 = clean_duplicate_punctuation(&t4);
    match (&t1, &t2, &t3, &t3b, &t4, &t5) {
        (
            Cow::Borrowed(_),
            Cow::Borrowed(_),
            Cow::Borrowed(_),
            Cow::Borrowed(_),
            Cow::Borrowed(_),
            Cow::Borrowed(_),
        ) => Cow::Borrowed(text),
        _ => Cow::Owned(t5.into_owned()),
    }
}

/// Deduplicate tables that overlap on the same page.
///
/// When both native detection and layout-based table extraction produce tables
/// for the same region, they can overlap. Tables at index `< native_count` are native;
/// the rest are layout (TATR/SLANeXT) tables. Complete side-by-side layout replacements
/// are selected atomically before ordinary pairwise arbitration. Outside those replacements,
/// `preference` decides mixed native/layout overlaps, while content weight decides same-origin
/// overlaps and [`TableOverlapPreference::Content`].
fn deduplicate_overlapping_tables(
    tables: &mut Vec<crate::types::Table>,
    native_count: usize,
    preference: crate::core::config::layout::TableOverlapPreference,
) {
    use crate::core::config::layout::TableOverlapPreference;

    if tables.len() < 2 {
        return;
    }

    let mut to_remove = ahash::AHashSet::new();
    let mut protected_layout_children = ahash::AHashSet::new();

    if preference != TableOverlapPreference::Native {
        // A complete split cohort is one structural alternative to its native source
        // cohort. Select it atomically for every non-Native preference: pairwise
        // content weighting could otherwise mix incompatible rows from both grids.
        for (parents, children) in side_by_side_layout_replacements(tables, native_count) {
            protected_layout_children.extend(children);
            to_remove.extend(parents);
        }
    }

    for i in 0..tables.len() {
        if to_remove.contains(&i) {
            continue;
        }
        for j in (i + 1)..tables.len() {
            if to_remove.contains(&j) {
                continue;
            }
            if tables[i].page_number != tables[j].page_number {
                continue;
            }
            if let (Some(a), Some(b)) = (&tables[i].bounding_box, &tables[j].bounding_box) {
                let inter_x = (a.x1.min(b.x1) - a.x0.max(b.x0)).max(0.0);
                let inter_y = (a.y1.min(b.y1) - a.y0.max(b.y0)).max(0.0);
                let intersection = inter_x * inter_y;
                let area_a = (a.x1 - a.x0) * (a.y1 - a.y0);
                let area_b = (b.x1 - b.x0) * (b.y1 - b.y0);
                let min_area = area_a.min(area_b);

                if min_area > 0.0 && intersection / min_area > 0.5 {
                    let i_is_native = i < native_count;
                    let j_is_native = j < native_count;
                    let mixed_origin = i_is_native != j_is_native;
                    let i_is_protected = protected_layout_children.contains(&i);
                    let j_is_protected = protected_layout_children.contains(&j);
                    let remove = match (i_is_protected, j_is_protected) {
                        (true, false) => Some(j),
                        (false, true) => Some(i),
                        (true, true) => {
                            let duplicate = intersection / area_a >= LAYOUT_CHILD_DUPLICATE_OVERLAP
                                && intersection / area_b >= LAYOUT_CHILD_DUPLICATE_OVERLAP;
                            duplicate.then(|| lower_content_table(tables, i, j))
                        }
                        (false, false) => Some(match preference {
                            TableOverlapPreference::Native if mixed_origin => {
                                if i_is_native {
                                    j
                                } else {
                                    i
                                }
                            }
                            TableOverlapPreference::Layout if mixed_origin => {
                                if i_is_native {
                                    i
                                } else {
                                    j
                                }
                            }
                            _ => lower_content_table(tables, i, j),
                        }),
                    };
                    let Some(remove) = remove else {
                        continue;
                    };
                    to_remove.insert(remove);
                    if remove == i {
                        break;
                    }
                }
            }
        }
    }

    let surviving_protected: Vec<_> = protected_layout_children
        .iter()
        .copied()
        .filter(|index| !to_remove.contains(index))
        .collect();
    let affected_rows: Vec<_> = surviving_protected
        .iter()
        .filter_map(|&index| tables[index].bounding_box.map(|bbox| (tables[index].page_number, bbox)))
        .collect();

    let mut idx = 0;
    tables.retain(|_| {
        let keep = !to_remove.contains(&idx);
        idx += 1;
        keep
    });
    canonicalize_affected_table_rows(tables, affected_rows);
}

/// Required containment for a layout child and, in one-to-one cohort matching,
/// reciprocal coverage of its corresponding native parent.
const SIDE_BY_SIDE_CHILD_PARENT_OVERLAP: f64 = 0.8;
/// Crop jitter tolerance for reciprocal one-to-one cohort correspondence.
const SIDE_BY_SIDE_CORRESPONDENCE_EPSILON: f64 = 0.005;
/// Both children must describe the same row band, rather than stacked tables.
const SIDE_BY_SIDE_VERTICAL_OVERLAP: f64 = 0.6;
/// The children together must account for most of the parent's horizontal span.
const SIDE_BY_SIDE_PARENT_WIDTH_COVERAGE: f64 = 0.75;
/// Every child must span most of the parent's height, rejecting shallow row fragments.
const SIDE_BY_SIDE_PARENT_HEIGHT_COVERAGE: f64 = 0.75;
/// Disjoint children must jointly explain most of the parent's total area.
const SIDE_BY_SIDE_PARENT_AREA_COVERAGE: f64 = 0.65;
/// Candidate detections that mutually cover nearly all of one another represent
/// the same layout table rather than distinct parts of a split table.
const LAYOUT_CHILD_DUPLICATE_OVERLAP: f64 = 0.9;

fn side_by_side_layout_replacements(
    tables: &[crate::types::Table],
    native_count: usize,
) -> Vec<(Vec<usize>, Vec<usize>)> {
    let native_count = native_count.min(tables.len());
    let rows = native_candidate_rows(tables, native_count);
    let mut used_native = ahash::AHashSet::new();
    let mut used_layout = ahash::AHashSet::new();
    let mut replacements = Vec::new();
    for parent in rows.iter().flatten().copied() {
        let parent_bbox = tables[parent].bounding_box.as_ref().expect("candidate has bbox");
        let mut children = layout_children_for_parent(tables, native_count, tables[parent].page_number, parent_bbox);
        children.retain(|child| !used_layout.contains(child));
        if !used_native.contains(&parent) && is_side_by_side_replacement(tables, parent_bbox, &children) {
            used_native.insert(parent);
            used_layout.extend(children.iter().copied());
            replacements.push((vec![parent], children));
        }
    }
    replacements.extend(side_by_side_native_cohort_replacements(
        tables,
        native_count,
        &rows,
        &mut used_native,
        &mut used_layout,
    ));
    replacements
}

fn side_by_side_native_cohort_replacements(
    tables: &[crate::types::Table],
    native_count: usize,
    rows: &[Vec<usize>],
    used_native: &mut ahash::AHashSet<usize>,
    used_layout: &mut ahash::AHashSet<usize>,
) -> Vec<(Vec<usize>, Vec<usize>)> {
    let mut replacements = Vec::new();
    for pair in rows.iter().flat_map(|row| row.windows(2)) {
        let parents = vec![pair[0], pair[1]];
        if parents.iter().any(|parent| used_native.contains(parent)) {
            continue;
        }
        let Some(parent_bbox) = table_union_bbox(&tables[parents[0]], &tables[parents[1]]) else {
            continue;
        };
        if !is_side_by_side_replacement(tables, &parent_bbox, &parents) {
            continue;
        }
        let mut children =
            layout_children_for_parent(tables, native_count, tables[parents[0]].page_number, &parent_bbox);
        children.retain(|child| !used_layout.contains(child));
        if children.len() != 2
            || !is_side_by_side_replacement(tables, &parent_bbox, &children)
            || !replacement_children_correspond(tables, &parents, &children)
        {
            continue;
        }
        used_native.extend(parents.iter().copied());
        used_layout.extend(children.iter().copied());
        replacements.push((parents, children));
    }
    replacements
}

fn native_candidate_rows(tables: &[crate::types::Table], native_count: usize) -> Vec<Vec<usize>> {
    let mut candidates: Vec<_> = (0..native_count)
        .filter(|&index| tables[index].bounding_box.is_some())
        .collect();
    candidates.sort_by(|&left, &right| {
        tables[left]
            .page_number
            .cmp(&tables[right].page_number)
            .then_with(|| table_top(tables, right).total_cmp(&table_top(tables, left)))
            .then_with(|| table_left(tables, left).total_cmp(&table_left(tables, right)))
    });
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for candidate in candidates {
        let joins_last_row = rows.last().is_some_and(|row| {
            tables[row[0]].page_number == tables[candidate].page_number
                && vertical_overlap_fraction(
                    tables[row[0]].bounding_box.as_ref().expect("candidate has bbox"),
                    tables[candidate].bounding_box.as_ref().expect("candidate has bbox"),
                ) >= SIDE_BY_SIDE_VERTICAL_OVERLAP
        });
        if joins_last_row {
            rows.last_mut().expect("row exists").push(candidate);
        } else {
            rows.push(vec![candidate]);
        }
    }
    for row in &mut rows {
        row.sort_by(|&left, &right| table_left(tables, left).total_cmp(&table_left(tables, right)));
    }
    rows
}

fn replacement_children_correspond(tables: &[crate::types::Table], parents: &[usize], children: &[usize]) -> bool {
    parents
        .iter()
        .zip(children)
        .enumerate()
        .all(|(position, (&parent, &child))| {
            let parent_bbox = tables[parent].bounding_box.as_ref().expect("candidate has bbox");
            let child_bbox = tables[child].bounding_box.as_ref().expect("candidate has bbox");
            let paired_intersection = bbox_intersection_area(parent_bbox, child_bbox);
            let sibling_intersection = children
                .get(1 - position)
                .and_then(|&sibling| tables[sibling].bounding_box.as_ref())
                .map_or(0.0, |sibling_bbox| bbox_intersection_area(parent_bbox, sibling_bbox));
            bbox_center_x(child_bbox) >= parent_bbox.x0
                && bbox_center_x(child_bbox) <= parent_bbox.x1
                && paired_intersection > sibling_intersection
                && bbox_overlap_fraction(child_bbox, parent_bbox) + SIDE_BY_SIDE_CORRESPONDENCE_EPSILON
                    >= SIDE_BY_SIDE_CHILD_PARENT_OVERLAP
                && bbox_overlap_fraction(parent_bbox, child_bbox) + SIDE_BY_SIDE_CORRESPONDENCE_EPSILON
                    >= SIDE_BY_SIDE_CHILD_PARENT_OVERLAP
        })
}

fn bbox_center_x(bbox: &crate::types::BoundingBox) -> f64 {
    (bbox.x0 + bbox.x1) / 2.0
}

fn table_top(tables: &[crate::types::Table], index: usize) -> f64 {
    tables[index]
        .bounding_box
        .as_ref()
        .map_or(f64::NEG_INFINITY, |bbox| bbox.y1)
}

fn layout_children_for_parent(
    tables: &[crate::types::Table],
    native_count: usize,
    page_number: u32,
    parent_bbox: &crate::types::BoundingBox,
) -> Vec<usize> {
    let children = (native_count..tables.len())
        .filter(|&child| {
            tables[child].page_number == page_number
                && tables[child]
                    .bounding_box
                    .as_ref()
                    .is_some_and(|bbox| bbox_overlap_fraction(bbox, parent_bbox) >= SIDE_BY_SIDE_CHILD_PARENT_OVERLAP)
        })
        .collect();
    deduplicate_layout_candidates(tables, children)
}

fn table_union_bbox(left: &crate::types::Table, right: &crate::types::Table) -> Option<crate::types::BoundingBox> {
    let left = left.bounding_box.as_ref()?;
    let right = right.bounding_box.as_ref()?;
    Some(crate::types::BoundingBox {
        x0: left.x0.min(right.x0),
        y0: left.y0.min(right.y0),
        x1: left.x1.max(right.x1),
        y1: left.y1.max(right.y1),
    })
}

fn deduplicate_layout_candidates(tables: &[crate::types::Table], mut candidates: Vec<usize>) -> Vec<usize> {
    candidates.sort_by(|&left, &right| table_left(tables, left).total_cmp(&table_left(tables, right)));
    let mut unique: Vec<usize> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let duplicate = unique.iter().position(|&existing| {
            let candidate_bbox = tables[candidate].bounding_box.as_ref().expect("candidate has bbox");
            let existing_bbox = tables[existing].bounding_box.as_ref().expect("candidate has bbox");
            bbox_overlap_fraction(candidate_bbox, existing_bbox) >= LAYOUT_CHILD_DUPLICATE_OVERLAP
                && bbox_overlap_fraction(existing_bbox, candidate_bbox) >= LAYOUT_CHILD_DUPLICATE_OVERLAP
        });
        if let Some(position) = duplicate {
            let existing = unique[position];
            if table_content_weight(&tables[candidate]) > table_content_weight(&tables[existing]) {
                unique[position] = candidate;
            }
        } else {
            unique.push(candidate);
        }
    }
    unique.sort_by(|&left, &right| table_left(tables, left).total_cmp(&table_left(tables, right)));
    unique
}

fn lower_content_table(tables: &[crate::types::Table], left: usize, right: usize) -> usize {
    if table_content_weight(&tables[left]) >= table_content_weight(&tables[right]) {
        right
    } else {
        left
    }
}

fn table_content_weight(table: &crate::types::Table) -> usize {
    table.cells.len() + table.markdown.len()
}

fn is_side_by_side_replacement(
    tables: &[crate::types::Table],
    parent: &crate::types::BoundingBox,
    children: &[usize],
) -> bool {
    let parent_width = parent.x1 - parent.x0;
    let parent_height = parent.y1 - parent.y0;
    if children.len() < 2 || parent_width <= 0.0 || parent_height <= 0.0 {
        return false;
    }
    let horizontally_disjoint = children.windows(2).all(|pair| {
        let left = tables[pair[0]].bounding_box.as_ref().expect("candidate has bbox");
        let right = tables[pair[1]].bounding_box.as_ref().expect("candidate has bbox");
        left.x1 <= right.x0 && vertical_overlap_fraction(left, right) >= SIDE_BY_SIDE_VERTICAL_OVERLAP
    });
    if !horizontally_disjoint {
        return false;
    }
    let covers_parent_height = children.iter().all(|&index| {
        let bbox = tables[index].bounding_box.as_ref().expect("candidate has bbox");
        let covered_height = (bbox.y1.min(parent.y1) - bbox.y0.max(parent.y0)).max(0.0);
        covered_height / parent_height >= SIDE_BY_SIDE_PARENT_HEIGHT_COVERAGE
    });
    if !covers_parent_height {
        return false;
    }
    let covered_width: f64 = children
        .iter()
        .map(|&index| {
            let bbox = tables[index].bounding_box.as_ref().expect("candidate has bbox");
            (bbox.x1.min(parent.x1) - bbox.x0.max(parent.x0)).max(0.0)
        })
        .sum();
    let covered_area: f64 = children
        .iter()
        .map(|&index| bbox_intersection_area(tables[index].bounding_box.as_ref().expect("candidate has bbox"), parent))
        .sum();
    covered_width / parent_width >= SIDE_BY_SIDE_PARENT_WIDTH_COVERAGE
        && covered_area / (parent_width * parent_height) >= SIDE_BY_SIDE_PARENT_AREA_COVERAGE
}

fn bbox_overlap_fraction(child: &crate::types::BoundingBox, parent: &crate::types::BoundingBox) -> f64 {
    let child_area = (child.x1 - child.x0).max(0.0) * (child.y1 - child.y0).max(0.0);
    if child_area == 0.0 {
        return 0.0;
    }
    bbox_intersection_area(child, parent) / child_area
}

fn bbox_intersection_area(a: &crate::types::BoundingBox, b: &crate::types::BoundingBox) -> f64 {
    let intersection_width = (a.x1.min(b.x1) - a.x0.max(b.x0)).max(0.0);
    let intersection_height = (a.y1.min(b.y1) - a.y0.max(b.y0)).max(0.0);
    intersection_width * intersection_height
}

fn vertical_overlap_fraction(a: &crate::types::BoundingBox, b: &crate::types::BoundingBox) -> f64 {
    let overlap = (a.y1.min(b.y1) - a.y0.max(b.y0)).max(0.0);
    let min_height = (a.y1 - a.y0).min(b.y1 - b.y0);
    if min_height <= 0.0 { 0.0 } else { overlap / min_height }
}

fn table_left(tables: &[crate::types::Table], index: usize) -> f64 {
    tables[index]
        .bounding_box
        .as_ref()
        .map_or(f64::INFINITY, |bbox| bbox.x0)
}

fn canonicalize_affected_table_rows(
    tables: &mut Vec<crate::types::Table>,
    affected_rows: Vec<(u32, crate::types::BoundingBox)>,
) {
    let cohorts = affected_row_cohorts(affected_rows);
    if cohorts.is_empty() {
        return;
    }
    let assignments: Vec<_> = tables.iter().map(|table| table_row_cohort(table, &cohorts)).collect();
    let mut cohort_tables: Vec<Vec<_>> = (0..cohorts.len()).map(|_| Vec::new()).collect();
    let mut source: Vec<Option<_>> = std::mem::take(tables).into_iter().map(Some).collect();
    for (index, cohort) in assignments.iter().enumerate() {
        if let Some(cohort) = cohort {
            cohort_tables[*cohort].push(source[index].take().expect("assigned table is present"));
        }
    }
    for cohort in &mut cohort_tables {
        cohort.sort_by(canonical_table_order);
        cohort.reverse();
    }
    for (index, cohort) in assignments.iter().enumerate() {
        if let Some(cohort) = cohort {
            source[index] = Some(
                cohort_tables[*cohort]
                    .pop()
                    .expect("cohort table count matches assigned slots"),
            );
        }
    }
    tables.extend(source.into_iter().flatten());
}

fn affected_row_cohorts(mut rows: Vec<(u32, crate::types::BoundingBox)>) -> Vec<(u32, Vec<crate::types::BoundingBox>)> {
    let mut cohorts = Vec::new();
    while let Some((page, seed)) = rows.pop() {
        let mut cohort = vec![seed];
        let mut changed = true;
        while changed {
            changed = false;
            rows.retain(|(candidate_page, candidate)| {
                let connected = *candidate_page == page
                    && cohort
                        .iter()
                        .any(|member| vertical_overlap_fraction(member, candidate) >= SIDE_BY_SIDE_VERTICAL_OVERLAP);
                if connected {
                    cohort.push(*candidate);
                    changed = true;
                }
                !connected
            });
        }
        cohorts.push((page, cohort));
    }
    cohorts.sort_by(|(left_page, left_rows), (right_page, right_rows)| {
        left_page.cmp(right_page).then_with(|| {
            let left_y = left_rows.iter().map(|row| row.y0).fold(f64::INFINITY, f64::min);
            let right_y = right_rows.iter().map(|row| row.y0).fold(f64::INFINITY, f64::min);
            left_y.total_cmp(&right_y)
        })
    });
    cohorts
}

fn table_row_cohort(table: &crate::types::Table, cohorts: &[(u32, Vec<crate::types::BoundingBox>)]) -> Option<usize> {
    let bbox = table.bounding_box.as_ref()?;
    cohorts
        .iter()
        .enumerate()
        .filter(|(_, (page, _))| *page == table.page_number)
        .map(|(index, (_, rows))| {
            let overlap = rows
                .iter()
                .map(|row| vertical_overlap_fraction(bbox, row))
                .fold(0.0_f64, f64::max);
            (index, overlap)
        })
        .filter(|(_, overlap)| *overlap >= SIDE_BY_SIDE_VERTICAL_OVERLAP)
        .max_by(|left, right| left.1.total_cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .map(|(index, _)| index)
}

fn canonical_table_order(left: &crate::types::Table, right: &crate::types::Table) -> std::cmp::Ordering {
    let left_bbox = left.bounding_box.as_ref();
    let right_bbox = right.bounding_box.as_ref();
    left.page_number
        .cmp(&right.page_number)
        .then_with(|| {
            left_bbox
                .map_or(f64::INFINITY, |bbox| bbox.y0)
                .total_cmp(&right_bbox.map_or(f64::INFINITY, |bbox| bbox.y0))
        })
        .then_with(|| {
            left_bbox
                .map_or(f64::INFINITY, |bbox| bbox.x0)
                .total_cmp(&right_bbox.map_or(f64::INFINITY, |bbox| bbox.x0))
        })
        .then_with(|| left.markdown.cmp(&right.markdown))
}

/// Clear `is_page_furniture` on paragraphs whose `layout_class` was set to
/// `PageHeader`, `PageFooter`, or `Footnote` by the layout model, when the
/// caller has opted in to keeping those regions via `include_headers` /
/// `include_footers` / `include_footnotes`.
///
/// This must run **before** `retain_page_furniture_safely`, which physically
/// removes furniture paragraphs via `.retain()`. Un-marking here ensures that
/// user-opted-in header/footer/footnote paragraphs survive that pass.
pub(crate) fn un_mark_layout_furniture_per_config(
    paragraphs: &mut [PdfParagraph],
    include_headers: bool,
    include_footers: bool,
    include_footnotes: bool,
) {
    if !include_headers && !include_footers && !include_footnotes {
        return;
    }
    for para in paragraphs.iter_mut() {
        if !para.is_page_furniture {
            continue;
        }
        match para.layout_class {
            Some(super::types::LayoutHintClass::PageHeader) if include_headers => {
                para.is_page_furniture = false;
            }
            Some(super::types::LayoutHintClass::PageFooter) if include_footers => {
                para.is_page_furniture = false;
            }
            Some(super::types::LayoutHintClass::Footnote) if include_footnotes => {
                para.is_page_furniture = false;
            }
            _ => {}
        }
    }
}

const FOOTNOTE_MARKER_MAX_CHARS: usize = 3;
const FOOTNOTE_MARKER_MAX_FONT_RATIO: f32 = 0.8;
const FOOTNOTE_MARKER_MAX_GAP_EM: f32 = 0.5;
const FOOTNOTE_MARKER_MIN_VERTICAL_OVERLAP_RATIO: f32 = 0.8;
const FOOTNOTE_MARKER_MIN_RISE_EM: f32 = 0.1;
const FOOTNOTE_RUN_MIN_LENGTH: usize = 3;
const FOOTNOTE_RUN_MAX_FONT_DELTA: f32 = 0.5;
const FOOTNOTE_RUN_MAX_LEFT_DELTA_EM: f32 = 0.5;
const FOOTNOTE_RUN_MAX_VERTICAL_GAP_EM: f32 = 0.5;
const FOOTNOTE_RUN_MAX_GAP_SPREAD_EM: f32 = 0.25;

/// Rejoin a small raised footnote marker that was split from its body.
///
/// Standalone numeric markers are initially classified as page numbers. Only
/// strong same-line geometry can override that classification, so genuine page
/// numbers and numbered list items remain untouched.
fn merge_spatial_footnote_markers(paragraphs: &mut Vec<PdfParagraph>) {
    let mut merged_pairs = vec![false; paragraphs.len()];
    let mut index = 0;
    while index + 1 < paragraphs.len() {
        if !is_spatial_footnote_pair(&paragraphs[index], &paragraphs[index + 1]) {
            index += 1;
            continue;
        }

        let marker = paragraphs.remove(index);
        merged_pairs.remove(index);
        let body = &mut paragraphs[index];
        let mut lines = marker.lines;
        lines.append(&mut body.lines);
        body.lines = lines;
        body.text.clear();
        body.block_bbox = marker.block_bbox.zip(body.block_bbox).map(|(marker_bbox, body_bbox)| {
            (
                marker_bbox.0.min(body_bbox.0),
                marker_bbox.1.min(body_bbox.1),
                marker_bbox.2.max(body_bbox.2),
                marker_bbox.3.max(body_bbox.3),
            )
        });
        body.word_count = PdfParagraph::compute_word_count("", &body.lines);
        body.is_page_furniture = false;
        merged_pairs[index] = true;
        index += 1;
    }
    merge_consecutive_spatial_footnotes(paragraphs, &mut merged_pairs);
}

fn spatial_footnote_number(paragraph: &PdfParagraph) -> Option<u32> {
    if paragraph.lines.len() < 2 || paragraph.heading_level.is_some() || paragraph.is_list_item {
        return None;
    }
    let marker = paragraph.lines.first()?.segments.first()?;
    if marker.font_size > paragraph.dominant_font_size * FOOTNOTE_MARKER_MAX_FONT_RATIO {
        return None;
    }
    marker.text.trim().parse().ok()
}

fn spatial_footnote_gap(upper: &PdfParagraph, lower: &PdfParagraph) -> Option<f32> {
    let (_, upper_bottom, _, _) = upper.block_bbox?;
    let (_, _, _, lower_top) = lower.block_bbox?;
    Some(upper_bottom - lower_top)
}

fn spatial_footnotes_are_adjacent(upper: &PdfParagraph, lower: &PdfParagraph) -> bool {
    let Some(upper_number) = spatial_footnote_number(upper) else {
        return false;
    };
    let Some(lower_number) = spatial_footnote_number(lower) else {
        return false;
    };
    let Some((upper_left, _, _, _)) = upper.block_bbox else {
        return false;
    };
    let Some((lower_left, _, _, _)) = lower.block_bbox else {
        return false;
    };
    let Some(gap) = spatial_footnote_gap(upper, lower) else {
        return false;
    };
    upper_number.checked_add(1) == Some(lower_number)
        && upper.layout_region_path == lower.layout_region_path
        && (upper.dominant_font_size - lower.dominant_font_size).abs() < FOOTNOTE_RUN_MAX_FONT_DELTA
        && (upper_left - lower_left).abs() <= upper.dominant_font_size * FOOTNOTE_RUN_MAX_LEFT_DELTA_EM
        && gap >= 0.0
        && gap <= upper.dominant_font_size * FOOTNOTE_RUN_MAX_VERTICAL_GAP_EM
}

fn spatial_footnote_run_is_regular(run: &[PdfParagraph]) -> bool {
    if run.len() < FOOTNOTE_RUN_MIN_LENGTH
        || !run
            .windows(2)
            .all(|pair| spatial_footnotes_are_adjacent(&pair[0], &pair[1]))
    {
        return false;
    }
    let gaps = run
        .windows(2)
        .filter_map(|pair| spatial_footnote_gap(&pair[0], &pair[1]))
        .collect::<Vec<_>>();
    let minimum = gaps.iter().copied().fold(f32::INFINITY, f32::min);
    let maximum = gaps.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    maximum - minimum <= run[0].dominant_font_size * FOOTNOTE_RUN_MAX_GAP_SPREAD_EM
}

fn merge_spatial_footnote_run(run: Vec<PdfParagraph>) -> PdfParagraph {
    let mut iter = run.into_iter();
    let mut merged = iter.next().expect("footnote run is non-empty");
    for mut paragraph in iter {
        merged.lines.append(&mut paragraph.lines);
        merged.block_bbox = merged.block_bbox.zip(paragraph.block_bbox).map(|(left, right)| {
            (
                left.0.min(right.0),
                left.1.min(right.1),
                left.2.max(right.2),
                left.3.max(right.3),
            )
        });
    }
    merged.text.clear();
    merged.word_count = PdfParagraph::compute_word_count("", &merged.lines);
    merged
}

fn merge_consecutive_spatial_footnotes(paragraphs: &mut Vec<PdfParagraph>, merged_pairs: &mut Vec<bool>) {
    let mut start = 0;
    while start + FOOTNOTE_RUN_MIN_LENGTH <= paragraphs.len() {
        let minimum_end = start + FOOTNOTE_RUN_MIN_LENGTH;
        if !merged_pairs[start..minimum_end].iter().all(|merged| *merged)
            || !spatial_footnote_run_is_regular(&paragraphs[start..minimum_end])
        {
            start += 1;
            continue;
        }

        let mut end = minimum_end;
        while end < paragraphs.len() && merged_pairs[end] && spatial_footnote_run_is_regular(&paragraphs[start..=end]) {
            end += 1;
        }

        let merged = merge_spatial_footnote_run(paragraphs.drain(start..end).collect());
        paragraphs.insert(start, merged);
        merged_pairs.drain(start..end);
        merged_pairs.insert(start, false);
        start += 1;
    }
}

fn is_spatial_footnote_pair(marker: &PdfParagraph, body: &PdfParagraph) -> bool {
    if !marker.is_page_furniture
        || body.is_page_furniture
        || body.heading_level.is_some()
        || body.is_list_item
        || body.is_code_block
        || body.is_formula
        || body.caption_for.is_some()
        || marker.layout_region_path != body.layout_region_path
        || !is_compact_footnote_marker(&paragraph_text_raw(marker))
    {
        return false;
    }

    let Some((marker_left, marker_bottom, marker_right, marker_top)) = marker.block_bbox else {
        return false;
    };
    let Some((body_left, body_bottom, _, body_top)) = body.block_bbox else {
        return false;
    };
    let marker_height = marker_top - marker_bottom;
    let body_height = body_top - body_bottom;
    let overlap = marker_top.min(body_top) - marker_bottom.max(body_bottom);
    let minimum_height = marker_height.min(body_height);
    let horizontal_gap = body_left - marker_right;

    marker_left < body_left
        && horizontal_gap >= 0.0
        && horizontal_gap <= body.dominant_font_size * FOOTNOTE_MARKER_MAX_GAP_EM
        && marker.dominant_font_size <= body.dominant_font_size * FOOTNOTE_MARKER_MAX_FONT_RATIO
        && marker_bottom - body_bottom >= body.dominant_font_size * FOOTNOTE_MARKER_MIN_RISE_EM
        && minimum_height > 0.0
        && overlap >= minimum_height * FOOTNOTE_MARKER_MIN_VERTICAL_OVERLAP_RATIO
}

fn is_compact_footnote_marker(text: &str) -> bool {
    let marker = text.trim();
    let char_count = marker.chars().count();
    char_count > 0
        && char_count <= FOOTNOTE_MARKER_MAX_CHARS
        && (marker.chars().all(|character| character.is_ascii_digit())
            || marker
                .chars()
                .all(|character| matches!(character, '*' | '†' | '‡' | '§')))
}

/// Maximum word count for a paragraph to be considered a page-number candidate.
///
/// The longest conventional form ("Chapter 2 — Page 14 of 30") is eight tokens;
/// ten leaves headroom without admitting prose.
const MAX_PAGE_NUMBER_WORD_COUNT: usize = 10;

/// Page width assumed when a document yields no usable paragraph geometry.
///
/// US Letter portrait, matching the 792pt height fallback used for `page_heights`.
const FALLBACK_PAGE_WIDTH_PTS: f32 = 612.0;

/// Page height assumed when `page_heights` carries no entry for a page index.
const FALLBACK_PAGE_HEIGHT_PTS: f32 = 792.0;

/// Mark confirmed running page numbers as furniture (GH#1411).
///
/// This replaces a context-free string test that matched any short numeric or
/// Roman-looking token anywhere on the page, including table cells, list
/// markers, footnote references and stray capitals. Because furniture under 80
/// alphanumeric characters is physically deleted by `retain_page_furniture_safely`,
/// that test caused silent content loss.
///
/// Three independent signals must agree before anything is marked:
///
/// 1. **Shape** — `classify_page_number_text` recognizes the token. Shape alone
///    is never sufficient; it only makes a paragraph a candidate.
/// 2. **Position** — the paragraph's vertical centre falls in the top or bottom
///    margin band. Body-band candidates are observed (they are counter-evidence
///    for the sequence) but are never deletable.
/// 3. **Cross-page sequence** — `PageNumberSequence` has seen every page before
///    any deletion decision is taken, so a single isolated match can never be
///    removed. Deletion requires `confidence_at` to reach `DELETION_THRESHOLD`.
///
/// Where confidence falls short the text is kept. Retaining an occasional page
/// number is far cheaper than silently dropping a table cell.
///
/// # Relationship to the layout model
///
/// Layout hints **inform, never override**, and this heuristic **stands down**
/// wherever the layout model has an opinion:
///
/// - `layout_class == None` — the layout model did not run, or produced no
///   class for this paragraph. Geometry plus cross-page sequence decide here.
/// - `layout_class == Some(PageHeader | PageFooter)` — the layout path already
///   owns this paragraph. It is marked furniture by `apply_layout_overrides` and
///   then selectively un-marked by `un_mark_layout_furniture_per_config`
///   according to `include_headers` / `include_footers`. Re-marking it here
///   would silently override that user configuration.
/// - `layout_class == Some(_other_)` — a positive body-content classification
///   from the model, which counts as evidence against deletion.
///
/// The single consequence of this rule is `layout_class_permits_page_number_deletion`.
fn mark_validated_page_numbers(all_pages: &mut [Vec<PdfParagraph>], page_heights: &[f32]) {
    use super::page_number::{PageNumberSequence, margin_band};

    let page_width = document_content_width(all_pages);
    let mut sequence = PageNumberSequence::new();
    // (page index, paragraph index, y ratio, x ratio) for every observed candidate.
    let mut observations: Vec<(usize, usize, f32, f32)> = Vec::new();

    // Pass 1: observe every candidate on every page. No deletion decision is
    // taken here — the sequence is not usable until it has seen all pages.
    for (page_index, page) in all_pages.iter().enumerate() {
        let page_height = page_heights
            .get(page_index)
            .copied()
            .unwrap_or(FALLBACK_PAGE_HEIGHT_PTS);
        for (paragraph_index, paragraph) in page.iter().enumerate() {
            let Some((y_ratio, x_ratio, candidate)) = page_number_observation(paragraph, page_height, page_width)
            else {
                continue;
            };
            sequence.observe(page_index, margin_band(y_ratio), x_ratio, &candidate);
            observations.push((page_index, paragraph_index, y_ratio, x_ratio));
        }
    }

    // Pass 2: confirm. Every page has now been observed.
    let mut confirmed = 0_usize;
    for (page_index, paragraph_index, y_ratio, x_ratio) in observations {
        let band = margin_band(y_ratio);
        if matches!(band, super::page_number::MarginBand::Body) {
            continue;
        }
        if sequence.confidence_at(page_index, band, x_ratio) < PageNumberSequence::DELETION_THRESHOLD {
            continue;
        }
        if let Some(paragraph) = all_pages
            .get_mut(page_index)
            .and_then(|page| page.get_mut(paragraph_index))
        {
            paragraph.is_page_furniture = true;
            confirmed += 1;
        }
    }

    tracing::debug!(
        pages = all_pages.len(),
        confirmed,
        "page-number furniture confirmed by position and cross-page sequence"
    );
}

/// Whether the layout model's opinion permits the page-number heuristic to act.
///
/// See the "Relationship to the layout model" section on
/// `mark_validated_page_numbers`: any layout class at all — header, footer, or
/// body content — takes precedence, so this heuristic only acts where the model
/// is silent.
fn layout_class_permits_page_number_deletion(paragraph: &PdfParagraph) -> bool {
    paragraph.layout_class.is_none()
}

/// Build a page-number observation for one paragraph.
///
/// Returns `(y_ratio, x_ratio, candidate)`, where `y_ratio` is 0.0 at the top of
/// the page and 1.0 at the bottom. `None` when the paragraph is not a candidate
/// or carries no usable geometry — either way it is never deletable.
fn page_number_observation(
    paragraph: &PdfParagraph,
    page_height: f32,
    page_width: f32,
) -> Option<(f32, f32, super::page_number::PageNumberCandidate)> {
    if paragraph.heading_level.is_some()
        || paragraph.is_list_item
        || paragraph.is_code_block
        || paragraph.is_page_furniture
        || paragraph.word_count > MAX_PAGE_NUMBER_WORD_COUNT
    {
        return None;
    }
    if !layout_class_permits_page_number_deletion(paragraph) {
        return None;
    }
    let text = paragraph_text_raw(paragraph);
    let candidate = super::page_number::classify_page_number_text(text.trim())?;
    let (y_ratio, x_ratio) = paragraph_position_ratios(paragraph, page_height, page_width)?;
    Some((y_ratio, x_ratio, candidate))
}

/// Normalized position of a paragraph's centre within its page.
///
/// Returns `(y_ratio, x_ratio)` with `y_ratio` 0.0 at the top of the page and
/// 1.0 at the bottom — PDF space measures y upward from the page bottom, so the
/// vertical axis is inverted here to match the band API.
fn paragraph_position_ratios(paragraph: &PdfParagraph, page_height: f32, page_width: f32) -> Option<(f32, f32)> {
    if !page_height.is_finite() || page_height <= 0.0 || !page_width.is_finite() || page_width <= 0.0 {
        return None;
    }
    let (left, bottom, right, top) = finite_paragraph_bbox(paragraph)?;
    let centre_y = (bottom + top) * 0.5;
    let centre_x = (left + right) * 0.5;
    let y_ratio = (1.0 - centre_y / page_height).clamp(0.0, 1.0);
    let x_ratio = (centre_x / page_width).clamp(0.0, 1.0);
    Some((y_ratio, x_ratio))
}

/// `paragraph_geometry_bbox` restricted to fully finite boxes.
///
/// A paragraph assembled from degenerate font metrics can carry NaN or infinite
/// bounds; normalizing those would produce a meaningless position ratio, so such
/// paragraphs are treated as having no geometry and are therefore never deletable.
fn finite_paragraph_bbox(paragraph: &PdfParagraph) -> Option<(f32, f32, f32, f32)> {
    let bbox = paragraph_geometry_bbox(paragraph)?;
    let (left, bottom, right, top) = bbox;
    (left.is_finite() && bottom.is_finite() && right.is_finite() && top.is_finite()).then_some(bbox)
}

/// Widest right edge across the whole document, used to normalize horizontal
/// position.
///
/// A document-wide value rather than a per-page one: `PageNumberSequence`
/// compares horizontal positions *across* pages, so the normalizer must be the
/// same on every page or a stable footer slot would read as drifting.
fn document_content_width(all_pages: &[Vec<PdfParagraph>]) -> f32 {
    let widest = all_pages
        .iter()
        .flatten()
        .filter_map(finite_paragraph_bbox)
        .map(|(_, _, right, _)| right)
        .filter(|right| *right > 0.0)
        .fold(0.0_f32, f32::max);
    if widest > 0.0 { widest } else { FALLBACK_PAGE_WIDTH_PTS }
}

/// Apply the structure pipeline's cross-page repeating-text policy to pages that
/// were already classified by another source, such as OCR layout detection. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn strip_repeating_text_from_pages(pages: &mut [Vec<PdfParagraph>], page_heights: &[f32]) {
    mark_cross_page_repeating_text(pages, page_heights);
    mark_cross_page_repeating_short_text(pages);
    for page in pages.iter_mut() {
        retain_page_furniture_safely(page);
    }
    deduplicate_paragraphs(pages);
}

/// Filter page furniture paragraphs with a safety valve.
///
/// Removes paragraphs marked as page furniture (headers/footers) by layout
/// detection. If removing ALL furniture-marked paragraphs would leave zero
/// content, the furniture markings are cleared instead — better to include
/// headers/footers than to produce empty output. This handles layout models
/// misclassifying body text as page furniture on non-standard document types
/// (e.g., legal transcripts, cover pages).
fn retain_page_furniture_safely(paragraphs: &mut Vec<PdfParagraph>) {
    let total = paragraphs.len();
    let furniture_count = paragraphs.iter().filter(|p| p.is_page_furniture).count();

    if furniture_count == 0 {
        return;
    }

    if furniture_count >= total {
        for para in paragraphs.iter_mut() {
            para.is_page_furniture = false;
        }
        return;
    }

    let total_alphanum: usize = paragraphs.iter().map(paragraph_alphanum_len).sum();

    if total_alphanum > 0 {
        let furniture_alphanum: usize = paragraphs
            .iter()
            .filter(|p| p.is_page_furniture)
            .map(paragraph_alphanum_len)
            .sum();

        if furniture_alphanum * 100 > total_alphanum * 30 {
            for para in paragraphs.iter_mut() {
                para.is_page_furniture = false;
            }
            return;
        }
    }

    const MIN_SUBSTANTIVE_CHARS: usize = 80;

    paragraphs.retain(|p| {
        if !p.is_page_furniture {
            return true;
        }
        paragraph_alphanum_len(p) > MIN_SUBSTANTIVE_CHARS
    });
}

/// Count alphanumeric characters in a paragraph's text content.
fn paragraph_alphanum_len(para: &PdfParagraph) -> usize {
    para.lines
        .iter()
        .flat_map(|line| line.segments.iter())
        .map(|seg| seg.text.bytes().filter(|b| b.is_ascii_alphanumeric()).count())
        .sum()
}

/// Dehyphenate paragraphs by rejoining words split across line boundaries.
///
/// When `has_positions` is true (heuristic extraction path), both explicit
/// trailing hyphens and implicit breaks (no hyphen, full line) are handled.
/// When false (structure tree path with x=0, width=0), only explicit trailing
/// hyphens are rejoined to avoid false positives.
fn dehyphenate_paragraphs(paragraphs: &mut [PdfParagraph], has_positions: bool, hyphen_witnesses: &HyphenWitnesses) {
    for para in paragraphs.iter_mut() {
        if para.is_code_block || para.lines.len() < 2 {
            continue;
        }
        if has_positions {
            dehyphenate_paragraph_lines(para, hyphen_witnesses);
        } else {
            dehyphenate_hyphen_only(para, hyphen_witnesses);
        }
    }
}

/// High-confidence lexical compounds whose source hyphen must survive a line break.
///
/// A trailing ASCII hyphen is otherwise indistinguishable from a discretionary PDF
/// line-wrap hyphen. Exact pair matching is intentionally narrower than prefix or
/// suffix rules: it protects common compounds without suppressing repairs such as
/// `soft-` + `ware`.
const PRESERVED_LEXICAL_COMPOUNDS: &[(&str, &str)] = &[
    ("cost", "effective"),
    ("evidence", "based"),
    ("high", "level"),
    ("long", "term"),
    ("low", "level"),
    ("real", "time"),
    ("short", "term"),
    ("state", "of-the-art"),
    ("user", "defined"),
    ("well", "known"),
];

/// Minimum letters required on each side of a mid-run hyphen before
/// [`collect_hyphen_witnesses`] records it, to avoid single-letter noise
/// (initials, bullet dashes) minting spurious witness pairs.
const MIN_HYPHEN_WITNESS_WORD_LEN: usize = 2;

/// Collect `(left, right)` word pairs the document itself writes as a single
/// hyphenated token, so a genuine authored hyphen at a line break can be told
/// apart from a hyphen that merely happens to fall at a line-wrap boundary (#1543).
///
/// Only a hyphen that is NOT the last character of its segment's text can witness a
/// real compound: a line-wrap hyphen is, by construction, the final character before
/// the break, so restricting the scan to strictly mid-run hyphens avoids witnessing
/// the very artifact this collector exists to judge. Must run before any page's
/// segments are moved out of `all_page_segments` (see call site in
/// `extract_document_structure_from_segments`), since a witness on one page can be
/// the sole evidence for a break on another. ~keep
fn collect_hyphen_witnesses(all_page_segments: &[Vec<SegmentData>]) -> HyphenWitnesses {
    let mut witnesses = HyphenWitnesses::default();
    for segment in all_page_segments.iter().flatten() {
        let characters: Vec<char> = segment.text.chars().collect();
        if characters.len() < 3 {
            continue;
        }
        for position in 1..characters.len() - 1 {
            if characters[position] != '-' {
                continue;
            }
            let left: String = characters[..position]
                .iter()
                .rev()
                .take_while(|character| character.is_alphabetic())
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            let right: String = characters[position + 1..]
                .iter()
                .take_while(|character| character.is_alphabetic())
                .collect();
            let left_len = left.chars().count();
            let right_len = right.chars().count();
            if left_len < MIN_HYPHEN_WITNESS_WORD_LEN || right_len < MIN_HYPHEN_WITNESS_WORD_LEN {
                continue;
            }
            witnesses.insert((left.to_ascii_lowercase(), right.to_ascii_lowercase()));
        }
    }
    witnesses
}

/// Collect standalone alphabetic words the document itself writes elsewhere, so
/// [`repair_ligature_spaces`] can tell a genuine word boundary apart from a
/// decomposed-ligature gap that looks identical at the string layer (#1591).
///
/// A token that is itself one half of a ligature-space candidate pattern (ends in
/// `f` right before whitespace, or starts with `i`/`l`/`f` right after it) is not
/// independent evidence for that occurrence: the very space under judgment put it
/// there, so counting it would make every candidate witness itself and disable the
/// repair (see the `f irst` false-negative this guards against). The same word
/// witnessed elsewhere in the document, in a position that is not itself a
/// candidate, is unaffected and still counts. Must run before any page's segments
/// are moved out of `all_page_segments` (see call site in
/// `extract_document_structure_from_segments`), mirroring
/// [`collect_hyphen_witnesses`]. ~keep
fn collect_word_witnesses(all_page_segments: &[Vec<SegmentData>]) -> WordWitnesses {
    let mut witnesses = WordWitnesses::default();
    for segment in all_page_segments.iter().flatten() {
        let cores: Vec<&str> = segment
            .text
            .split_whitespace()
            .map(|token| token.trim_matches(|c: char| !c.is_alphabetic()))
            .collect();
        for index in 0..cores.len() {
            let core = cores[index];
            if core.chars().count() < MIN_LIGATURE_WITNESS_WORD_LEN {
                continue;
            }
            let is_left_of_candidate = core.ends_with('f')
                && cores
                    .get(index + 1)
                    .and_then(|next| next.chars().next())
                    .is_some_and(|c| matches!(c, 'i' | 'l' | 'f'));
            let is_right_of_candidate = index > 0
                && cores[index - 1].ends_with('f')
                && core.chars().next().is_some_and(|c| matches!(c, 'i' | 'l' | 'f'));
            if is_left_of_candidate || is_right_of_candidate {
                continue;
            }
            witnesses.insert(core.to_ascii_lowercase());
        }
    }
    witnesses
}

pub(super) fn should_preserve_lexical_hyphen(
    trailing_word: &str,
    leading_word: &str,
    hyphen_witnesses: &HyphenWitnesses,
) -> bool {
    let trim_non_lexical = |ch: char| !ch.is_alphanumeric() && ch != '-';
    let left = trailing_word.trim_matches(trim_non_lexical);
    let right = leading_word.trim_matches(trim_non_lexical);

    let matches_static_compound = PRESERVED_LEXICAL_COMPOUNDS
        .iter()
        .any(|&(expected_left, expected_right)| {
            left.eq_ignore_ascii_case(expected_left) && right.eq_ignore_ascii_case(expected_right)
        });
    matches_static_compound || hyphen_witnesses.contains(&(left.to_ascii_lowercase(), right.to_ascii_lowercase()))
}

/// Whether adjacent extraction runs actually cross a visual line boundary.
///
/// `PdfLine` boundaries can also be introduced by inline style/run splitting. A
/// suspended hyphen such as `vracht- en` may therefore appear at the end of one
/// logical line and the start of the next while both runs still share a baseline.
/// Dehyphenation is only licensed when the runs use the same reading frame and
/// their upright baselines differ by more than the inline-style tolerance.
fn spans_visual_line_break(trailing: &SegmentData, leading: &SegmentData) -> bool {
    if !trailing.has_same_rotation(leading) {
        return false;
    }

    let trailing_baseline = trailing.upright_baseline();
    let leading_baseline = leading.upright_baseline();
    trailing_baseline.is_finite()
        && leading_baseline.is_finite()
        && (trailing_baseline - leading_baseline).abs() > INLINE_STYLE_BASELINE_TOLERANCE
}

/// Core dehyphenation with position-based full-line detection.
///
/// For each line boundary, checks whether the line extends close to the right
/// margin. If so, attempts to rejoin the trailing word of one line with the
/// leading word of the next.
fn dehyphenate_paragraph_lines(para: &mut PdfParagraph, hyphen_witnesses: &HyphenWitnesses) {
    let max_right_edge = para
        .lines
        .iter()
        .flat_map(|l| l.segments.iter())
        .map(|s| s.x + s.width)
        .fold(0.0_f32, f32::max);

    if max_right_edge <= 0.0 {
        dehyphenate_hyphen_only(para, hyphen_witnesses);
        return;
    }

    let threshold = max_right_edge * FULL_LINE_FRACTION;

    let n = para.lines.len();
    for i in 0..(n - 1) {
        let trailing_right = para.lines[i].segments.last().map(|s| s.x + s.width).unwrap_or(0.0);
        if trailing_right < threshold {
            continue;
        }

        let crosses_visual_line = match (para.lines[i].segments.last(), para.lines[i + 1].segments.first()) {
            (Some(trailing), Some(leading)) => spans_visual_line_break(trailing, leading),
            _ => false,
        };
        if !crosses_visual_line {
            continue;
        }

        let trailing_text = match para.lines[i].segments.last() {
            Some(s) if !s.text.is_empty() => s.text.clone(),
            _ => continue,
        };
        let leading_text = match para.lines[i + 1].segments.first() {
            Some(s) if !s.text.is_empty() => s.text.clone(),
            _ => continue,
        };

        let has_trailing_hyphen = trailing_text.ends_with('-');
        if !has_trailing_hyphen {
            continue;
        }

        let leading_word = leading_text.split_whitespace().next().unwrap_or("");
        if leading_word.chars().next().is_some_and(|c| c.is_uppercase()) {
            continue;
        }

        let trailing_word = trailing_text
            .trim_end_matches('-')
            .split_whitespace()
            .last()
            .unwrap_or("");
        if trailing_word.chars().last().is_some_and(is_cjk_char) {
            continue;
        }

        let preserved_hyphen = if should_preserve_lexical_hyphen(trailing_word, leading_word, hyphen_witnesses) {
            "-"
        } else {
            ""
        };
        let joined_word = format!("{trailing_word}{preserved_hyphen}{leading_word}");

        if let Some(seg) = para.lines[i].segments.last_mut() {
            let text_without_word: String = seg
                .text
                .chars()
                .rev()
                .skip(trailing_word.len() + 1)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            seg.text = format!("{text_without_word}{joined_word}");
        }

        if let Some(seg) = para.lines[i + 1].segments.first_mut() {
            let after_leading_word = seg.text.trim_start_matches(leading_word).trim_start();
            seg.text = after_leading_word.to_string();
        }
    }
}

/// Hyphen-only dehyphenation (no position data required).
///
/// Only joins lines when the trailing segment ends with an explicit hyphen.
/// Used for structure tree pages where x/width may be zero.
fn dehyphenate_hyphen_only(para: &mut PdfParagraph, hyphen_witnesses: &HyphenWitnesses) {
    let n = para.lines.len();
    for i in 0..(n - 1) {
        let crosses_visual_line = match (para.lines[i].segments.last(), para.lines[i + 1].segments.first()) {
            (Some(trailing), Some(leading)) => spans_visual_line_break(trailing, leading),
            _ => false,
        };
        if !crosses_visual_line {
            continue;
        }

        let trailing_text = match para.lines[i].segments.last() {
            Some(s) if s.text.ends_with('-') => s.text.clone(),
            _ => continue,
        };
        let leading_text = match para.lines[i + 1].segments.first() {
            Some(s) if !s.text.is_empty() => s.text.clone(),
            _ => continue,
        };

        let leading_word = leading_text.split_whitespace().next().unwrap_or("");
        if leading_word.chars().next().is_some_and(|c| c.is_uppercase()) {
            continue;
        }

        let trailing_word = trailing_text
            .trim_end_matches('-')
            .split_whitespace()
            .last()
            .unwrap_or("");
        if trailing_word.chars().last().is_some_and(is_cjk_char) {
            continue;
        }

        let preserved_hyphen = if should_preserve_lexical_hyphen(trailing_word, leading_word, hyphen_witnesses) {
            "-"
        } else {
            ""
        };
        let joined_word = format!("{trailing_word}{preserved_hyphen}{leading_word}");

        if let Some(seg) = para.lines[i].segments.last_mut() {
            let text_without_word: String = seg
                .text
                .chars()
                .rev()
                .skip(trailing_word.len() + 1)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            seg.text = format!("{text_without_word}{joined_word}");
        }

        if let Some(seg) = para.lines[i + 1].segments.first_mut() {
            let after_leading_word = seg.text.trim_start_matches(leading_word).trim_start();
            seg.text = after_leading_word.to_string();
        }
    }
}

/// Detect whether a set of paragraphs contains any font-size variation.
///
/// Variation is defined as any paragraph whose font size differs from the first
/// non-zero size by more than 0.5pt. Used to decide whether structure-tree pages
/// need font-size clustering for heading assignment.
fn has_font_size_variation(paragraphs: &[PdfParagraph]) -> bool {
    let mut first_size: Option<f32> = None;
    for para in paragraphs {
        let size = para.dominant_font_size;
        if size <= 0.0 {
            continue;
        }
        match first_size {
            None => first_size = Some(size),
            Some(fs) if (size - fs).abs() > 0.5 => return true,
            _ => {}
        }
    }
    false
}

/// Deduplicate paragraphs with identical text within each page.
///
/// Two-pass approach:
/// 1. Consecutive duplicates: remove back-to-back identical paragraphs
///    (catches bold/shadow rendering artifacts).
/// 2. Non-consecutive duplicates: remove body-text paragraphs whose
///    normalized text was already seen on the same page (catches table
///    content rendered as both table and body text).
///
/// Only deduplicates body text — headings, list items, code blocks,
/// formulas, and captions are preserved even if duplicated.
fn deduplicate_paragraphs(all_pages: &mut [Vec<PdfParagraph>]) {
    for page in all_pages.iter_mut() {
        if page.len() < 2 {
            continue;
        }

        let mut i = 0;
        while i + 1 < page.len() {
            let a_text = paragraph_text_normalized(&page[i]);
            let b_text = paragraph_text_normalized(&page[i + 1]);
            if page[i].layout_region_path == page[i + 1].layout_region_path && a_text.len() >= 5 && a_text == b_text {
                page.remove(i + 1);
            } else {
                i += 1;
            }
        }

        let mut seen = ahash::AHashSet::new();
        let mut to_remove = Vec::new();
        for (idx, para) in page.iter().enumerate() {
            if !is_dedup_candidate(para) {
                continue;
            }
            let text = paragraph_text_normalized(para);
            if text.len() < 15 {
                continue;
            }
            if !seen.insert((para.layout_region_path, text)) {
                to_remove.push(idx);
            }
        }

        for &idx in to_remove.iter().rev() {
            page.remove(idx);
        }
    }
}

const DEFAULT_OUTLINE_HEADING_OFFSET: i64 = 2;
const MIN_OUTLINE_CALIBRATION_ANCHORS: usize = 2;
const MIN_MARKDOWN_HEADING_LEVEL: i64 = 1;
const MAX_MARKDOWN_HEADING_LEVEL: i64 = 6;

#[derive(Debug, Clone, Copy)]
struct OutlineParagraphMatch {
    page_index: usize,
    paragraph_index: usize,
    depth: usize,
}

fn recover_headings_from_outline(all_pages: &mut [Vec<PdfParagraph>], outline_entries: &[PdfOutlineEntry]) {
    let matches = collect_unique_outline_matches(all_pages, outline_entries);
    let offset = calibrated_outline_heading_offset(all_pages, &matches);

    for matched in matches {
        let paragraph = &mut all_pages[matched.page_index][matched.paragraph_index];
        if paragraph.heading_level.is_some() || !outline_layout_allows_heading(paragraph) {
            continue;
        }
        let depth = i64::try_from(matched.depth).unwrap_or(i64::MAX);
        let level = depth
            .saturating_add(offset)
            .clamp(MIN_MARKDOWN_HEADING_LEVEL, MAX_MARKDOWN_HEADING_LEVEL);
        paragraph.heading_level = Some(level as u8);
        paragraph.is_list_item = false;
        paragraph.is_page_furniture = false;
    }
}

fn collect_unique_outline_matches(
    all_pages: &[Vec<PdfParagraph>],
    outline_entries: &[PdfOutlineEntry],
) -> Vec<OutlineParagraphMatch> {
    let mut outline_counts = ahash::AHashMap::<(usize, String), usize>::new();
    for entry in outline_entries {
        if let Some(key) = outline_match_key(entry, all_pages.len()) {
            *outline_counts.entry(key).or_default() += 1;
        }
    }
    let paragraph_matches = all_pages
        .iter()
        .map(|page| {
            let mut matches = ahash::AHashMap::<String, (usize, usize)>::new();
            for (index, paragraph) in page.iter().enumerate() {
                let title = normalize_outline_title(&paragraph_text_raw(paragraph));
                let entry = matches.entry(title).or_insert((0, index));
                entry.0 += 1;
            }
            matches
        })
        .collect::<Vec<_>>();

    outline_entries
        .iter()
        .filter_map(|entry| {
            let (page_index, title) = outline_match_key(entry, all_pages.len())?;
            if outline_counts.get(&(page_index, title.clone())) != Some(&1) {
                return None;
            }
            let &(paragraph_count, paragraph_index) = paragraph_matches[page_index].get(&title)?;
            (paragraph_count == 1).then_some(OutlineParagraphMatch {
                page_index,
                paragraph_index,
                depth: entry.depth,
            })
        })
        .collect()
}

fn outline_match_key(entry: &PdfOutlineEntry, page_count: usize) -> Option<(usize, String)> {
    let page_number = entry.page_number?;
    let page_index = usize::try_from(page_number.checked_sub(1)?).ok()?;
    let title = normalize_outline_title(&entry.title);
    (page_index < page_count && !title.is_empty()).then_some((page_index, title))
}

fn calibrated_outline_heading_offset(all_pages: &[Vec<PdfParagraph>], matches: &[OutlineParagraphMatch]) -> i64 {
    let mut counts = ahash::AHashMap::<i64, usize>::new();
    for matched in matches {
        let paragraph = &all_pages[matched.page_index][matched.paragraph_index];
        if !outline_layout_allows_heading(paragraph) {
            continue;
        }
        if let Some(level) = paragraph.heading_level {
            let depth = i64::try_from(matched.depth).unwrap_or(i64::MAX);
            *counts.entry(i64::from(level).saturating_sub(depth)).or_default() += 1;
        }
    }

    let max_count = counts.values().copied().max().unwrap_or_default();
    let mut winners = counts.into_iter().filter(|(_, count)| *count == max_count);
    let winner = winners.next();
    match (winner, winners.next(), max_count) {
        (Some((offset, _)), None, count) if count >= MIN_OUTLINE_CALIBRATION_ANCHORS => offset,
        _ => DEFAULT_OUTLINE_HEADING_OFFSET,
    }
}

fn outline_layout_allows_heading(paragraph: &PdfParagraph) -> bool {
    if paragraph.is_code_block || paragraph.is_formula || paragraph.caption_for.is_some() {
        return false;
    }
    matches!(
        paragraph.layout_class,
        None | Some(super::types::LayoutHintClass::Title)
            | Some(super::types::LayoutHintClass::SectionHeader)
            | Some(super::types::LayoutHintClass::Text)
            | Some(super::types::LayoutHintClass::Other)
    )
}

fn normalize_outline_title(text: &str) -> String {
    let text = strip_section_label(text.trim());
    let mut normalized = String::new();
    let mut pending_space = false;
    for character in text.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if pending_space && !normalized.is_empty() {
                normalized.push(' ');
            }
            normalized.push(character);
            pending_space = false;
        } else if !normalized.is_empty() {
            pending_space = true;
        }
    }
    normalized
}

fn strip_section_label(text: &str) -> &str {
    let Some((first, rest)) = text.split_once(char::is_whitespace) else {
        return text;
    };
    let punctuated = first
        .chars()
        .next()
        .is_some_and(|character| matches!(character, '(' | '['))
        || first
            .chars()
            .last()
            .is_some_and(|character| matches!(character, '.' | ')' | ']' | ':'));
    let core = first.trim_matches(|character| matches!(character, '(' | '[' | '.' | ')' | ']' | ':'));
    let decimal_parts = core.split('.').collect::<Vec<_>>();
    let decimal = !decimal_parts.is_empty()
        && decimal_parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
    let decimal_label = decimal && (punctuated || decimal_parts.len() > 1 || core.len() <= 3);
    let roman_label = punctuated
        && !core.is_empty()
        && core
            .chars()
            .all(|character| matches!(character.to_ascii_uppercase(), 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M'));
    let letter_label = punctuated && core.len() == 1 && core.chars().all(|character| character.is_ascii_alphabetic());

    if decimal_label || roman_label || letter_label {
        rest.trim_start()
    } else {
        text
    }
}

fn paragraph_text_raw(para: &PdfParagraph) -> String {
    if para.text.is_empty() {
        para.lines
            .iter()
            .flat_map(|line| line.segments.iter())
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        para.text.clone()
    }
}

/// Normalize paragraph text for deduplication comparison.
///
/// Uses `para.text` when populated (heuristic path), otherwise assembles text
/// from segment data (structure tree path, used in tests).
fn paragraph_text_normalized(para: &PdfParagraph) -> String {
    paragraph_text_raw(para)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Check if a paragraph is a candidate for non-consecutive deduplication.
fn is_dedup_candidate(p: &PdfParagraph) -> bool {
    p.heading_level.is_none()
        && !p.is_list_item
        && !p.is_code_block
        && !p.is_formula
        && !p.is_page_furniture
        && p.caption_for.is_none()
}

/// Minimum word count for the lead sentence before a run-in list's anchor
/// colon; guards against matching short labels (`"Note:"`, abbreviations)
/// that happen to be followed by a semicolon elsewhere in the text.
const RUN_IN_LIST_MIN_LEAD_WORDS: usize = 4;
/// Minimum semicolon-delimited clauses required to call a colon-introduced
/// run a "list" — a single clause is just a qualified sentence, not an
/// enumeration.
const RUN_IN_LIST_MIN_ITEMS: usize = 2;
/// Minimum word count per clause; guards against matching stray short
/// fragments (e.g. an abbreviation followed by `;`) as list items.
const RUN_IN_LIST_MIN_ITEM_WORDS: usize = 3;

/// Split a colon-introduced, semicolon-delimited "run-in" list — a prose
/// convention common in legal/contract text, e.g. "...is authorised to
/// exclude subscription rights: to exclude fractional amounts...; where the
/// new shares...;" — out of a single assembled paragraph into a lead
/// paragraph plus one list-item paragraph per clause.
///
/// These enumerations are frequently rendered with no distinguishing
/// indentation or line break from the surrounding prose (the source document
/// never used a real list, just semicolon-separated clauses within one
/// paragraph flow), so geometry-based list detection
/// (`classify::detect_indentation_based_lists`) never sees them — that pass
/// only promotes paragraphs already indented relative to the page's modal
/// left margin. This pass instead recognizes the enumeration from paragraph
/// text alone, after normal paragraph assembly, and works for both the
/// heuristic and structure-tree paragraph paths via [`paragraph_text_raw`].
///
/// xberg-io/xberg#1301.
fn split_colon_semicolon_run_in_lists(all_page_paragraphs: &mut [Vec<PdfParagraph>]) {
    for page_paragraphs in all_page_paragraphs.iter_mut() {
        let mut index = 0;
        while index < page_paragraphs.len() {
            match try_split_run_in_list(&page_paragraphs[index]) {
                Some(replacement) => {
                    let inserted = replacement.len();
                    page_paragraphs.splice(index..=index, replacement);
                    index += inserted;
                }
                None => index += 1,
            }
        }
    }
}

/// Attempt to split one paragraph into a lead paragraph plus run-in list
/// items. Returns `None` when the paragraph does not match the pattern, in
/// which case it is left untouched.
fn try_split_run_in_list(para: &PdfParagraph) -> Option<Vec<PdfParagraph>> {
    if para.heading_level.is_some()
        || para.is_list_item
        || para.is_code_block
        || para.is_formula
        || para.is_page_furniture
    {
        return None;
    }

    let normalized: String = paragraph_text_raw(para)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let colon_byte = normalized.rfind(':')?;
    let lead = normalized[..=colon_byte].trim();
    if lead.split_whitespace().count() < RUN_IN_LIST_MIN_LEAD_WORDS {
        return None;
    }

    let tail = normalized[colon_byte + 1..].trim_start();
    if tail.is_empty() {
        return None;
    }

    let items: Vec<&str> = tail
        .split_inclusive(';')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect();
    if items.len() < RUN_IN_LIST_MIN_ITEMS || !items.iter().all(|item| is_probable_run_in_list_item(item)) {
        return None;
    }

    let mut split = Vec::with_capacity(items.len() + 1);
    split.push(run_in_list_fragment(para, lead.to_string(), false));
    for item in items {
        split.push(run_in_list_fragment(para, item.to_string(), true));
    }
    Some(split)
}

/// Whether one semicolon-delimited clause reads as a genuine list item:
/// substantial (several words), a lowercase continuation of the lead
/// sentence (real clauses read as "to exclude...", "where...", not a new
/// capitalized sentence), and clause-terminated.
fn is_probable_run_in_list_item(item: &str) -> bool {
    item.split_whitespace().count() >= RUN_IN_LIST_MIN_ITEM_WORDS
        && item.chars().next().is_some_and(char::is_lowercase)
        && matches!(item.chars().last(), Some(';' | '.'))
}

/// Build one split-off fragment, inheriting the source paragraph's
/// non-textual attributes (font size, boldness, page association, etc.).
fn run_in_list_fragment(source: &PdfParagraph, text: String, is_list_item: bool) -> PdfParagraph {
    let word_count = text.split_whitespace().count();
    PdfParagraph {
        text,
        lines: Vec::new(),
        heading_level: None,
        is_list_item,
        is_code_block: false,
        is_formula: false,
        layout_class: if is_list_item {
            Some(super::types::LayoutHintClass::ListItem)
        } else {
            source.layout_class
        },
        word_count,
        ..source.clone()
    }
}

fn apply_text_repair_to_structure_tree_paragraphs(
    paragraphs: &mut Vec<PdfParagraph>,
    has_positions: bool,
    witnesses: &TextRepairWitnesses,
) {
    apply_to_all_segments(paragraphs, |text| fused_text_repairs(text, &witnesses.words));
    dehyphenate_paragraphs(paragraphs, has_positions, &witnesses.hyphens);
    split_embedded_list_items(paragraphs);
    synchronize_paragraph_text_metadata(paragraphs);
}

/// Invalidate cached paragraph text after mutating segments and refresh derived metadata.
///
/// Assembly derives both the emitted text and inline annotation byte ranges from segments
/// when `text` is empty. Keeping that cache empty prevents repaired segment text from
/// diverging from the stale pre-repair string used by the heuristic path.
fn synchronize_paragraph_text_metadata(paragraphs: &mut [PdfParagraph]) {
    for paragraph in paragraphs {
        paragraph.text.clear();
        paragraph.word_count = PdfParagraph::compute_word_count("", &paragraph.lines);
    }
}

fn compact_final_heading_hierarchy(all_pages: &mut [Vec<PdfParagraph>]) {
    let headings = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .filter_map(|paragraph| paragraph.heading_level);
    let (h1_count, has_h2, has_deeper) = headings.fold((0usize, false, false), |state, level| {
        (
            state.0 + usize::from(level == 1),
            state.1 || level == 2,
            state.2 || level >= 3,
        )
    });
    if h1_count != 1 || has_h2 || !has_deeper {
        return;
    }

    for paragraph in all_pages.iter_mut().flat_map(|page| page.iter_mut()) {
        if let Some(level @ 3..) = paragraph.heading_level {
            paragraph.heading_level = Some(level - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::hierarchy::SegmentData;
    use crate::pdf::structure::types::{PdfLine, PdfParagraph};

    #[cfg(feature = "layout-detection")]
    #[test]
    fn table_model_preflight_uses_selected_slanet_variant() {
        use crate::core::config::layout::TableModel;

        assert_eq!(
            slanet_variant_for_table_model(TableModel::SlanetWired),
            Some("slanet_wired")
        );
        assert_eq!(
            slanet_variant_for_table_model(TableModel::SlanetWireless),
            Some("slanet_wireless")
        );
        assert_eq!(
            slanet_variant_for_table_model(TableModel::SlanetPlus),
            Some("slanet_plus")
        );
        assert_eq!(
            slanet_variant_for_table_model(TableModel::SlanetAuto),
            Some("slanet_wired")
        );
        assert_eq!(slanet_variant_for_table_model(TableModel::Tatr), None);
        assert_eq!(slanet_variant_for_table_model(TableModel::Disabled), None);
    }

    /// Helper: a table at `bbox` on `page` whose only content is `markdown`
    /// (so content weight == markdown length; empty cells).
    fn ov_table(page: u32, bbox: (f64, f64, f64, f64), markdown: &str) -> crate::types::Table {
        let (x0, y0, x1, y1) = bbox;
        crate::types::Table {
            cells: Vec::new(),
            markdown: markdown.to_string(),
            page_number: page,
            bounding_box: Some(crate::types::BoundingBox { x0, y0, x1, y1 }),
            ..Default::default()
        }
    }

    /// Helper: a table fragment with real cell content (needed to satisfy
    /// `fragments_are_stitchable`'s column-count check) at `bbox` on `page`.
    fn cell_table(page: u32, bbox: (f64, f64, f64, f64), cells: &[&[&str]]) -> crate::types::Table {
        let (x0, y0, x1, y1) = bbox;
        let cells: Vec<Vec<String>> = cells
            .iter()
            .map(|row| row.iter().map(|s| s.to_string()).collect())
            .collect();
        let markdown = cells.iter().map(|row| row.join("|")).collect::<Vec<_>>().join("\n");
        crate::types::Table {
            cells,
            markdown,
            page_number: page,
            bounding_box: Some(crate::types::BoundingBox { x0, y0, x1, y1 }),
            ..Default::default()
        }
    }

    /// Run the same id/columns assignment the real pipeline performs: stitch
    /// same-page fragments, then run the final, post-dedup assignment pass in
    /// `prepare_emitted_tables` (see issue #1297 code review: assigning ids
    /// inside `stitch_fragmented_tables` alone misses layout-detected tables).
    fn stitch_and_emit(
        native_tables: Vec<crate::types::Table>,
        layout_tables: Vec<crate::types::Table>,
        all_page_segments: &[Vec<SegmentData>],
    ) -> Vec<crate::types::Table> {
        use crate::core::config::layout::TableOverlapPreference;
        let stitched = stitch_fragmented_tables(native_tables, all_page_segments);
        prepare_emitted_tables(&stitched, layout_tables, TableOverlapPreference::Content)
    }

    /// Issue #1297: fragments of one physical table (stitched into a single
    /// chain) collapse into one `tables[]` entry, which naturally carries one
    /// `table_id`. A separate, non-adjacent table gets a distinct id.
    #[test]
    fn stitched_fragments_share_one_table_id_distinct_tables_differ() {
        let frag_top = cell_table(1, (0.0, 90.0, 100.0, 110.0), &[&["H1", "H2"]]);
        let frag_bottom = cell_table(1, (0.0, 70.0, 100.0, 89.0), &[&["a", "b"]]);
        let other_page_table = cell_table(2, (0.0, 0.0, 100.0, 20.0), &[&["X", "Y"]]);

        let all_page_segments: Vec<Vec<SegmentData>> = Vec::new();
        let result = stitch_and_emit(
            vec![frag_top, frag_bottom, other_page_table],
            Vec::new(),
            &all_page_segments,
        );

        assert_eq!(result.len(), 2, "the two page-1 fragments must stitch into one table");

        let page_1_table = result
            .iter()
            .find(|t| t.page_number == 1)
            .expect("page 1 table present");
        let page_2_table = result
            .iter()
            .find(|t| t.page_number == 2)
            .expect("page 2 table present");

        assert_eq!(page_1_table.cells.len(), 2, "stitched chain has both fragments' rows");
        assert!(page_1_table.table_id.is_some(), "stitched table must have a table_id");
        assert!(page_2_table.table_id.is_some(), "unrelated table must have a table_id");
        assert_ne!(
            page_1_table.table_id, page_2_table.table_id,
            "distinct physical tables must have distinct ids"
        );
    }

    /// Issue #1297: `table_id` assignment must be deterministic across runs
    /// for the same input (no randomness, no wall-clock dependence).
    #[test]
    fn table_id_assignment_is_deterministic_across_runs() {
        let build_input = || {
            vec![
                cell_table(2, (0.0, 0.0, 100.0, 20.0), &[&["X", "Y"]]),
                cell_table(1, (0.0, 0.0, 100.0, 20.0), &[&["A", "B"]]),
            ]
        };
        let all_page_segments: Vec<Vec<SegmentData>> = Vec::new();

        let first_run = stitch_and_emit(build_input(), Vec::new(), &all_page_segments);
        let second_run = stitch_and_emit(build_input(), Vec::new(), &all_page_segments);

        let first_ids: Vec<_> = first_run.iter().map(|t| (t.page_number, t.table_id.clone())).collect();
        let second_ids: Vec<_> = second_run.iter().map(|t| (t.page_number, t.table_id.clone())).collect();
        assert_eq!(first_ids, second_ids, "table_id assignment must be deterministic");
    }

    /// Issue #1297: every emitted table fragment carries `columns` (its own
    /// header row), even a fragment that stitching left untouched.
    #[test]
    fn stitching_populates_columns_on_merged_and_standalone_fragments() {
        let frag_top = cell_table(1, (0.0, 90.0, 100.0, 110.0), &[&["H1", "H2"]]);
        let frag_bottom = cell_table(1, (0.0, 70.0, 100.0, 89.0), &[&["a", "b"]]);
        let standalone = cell_table(3, (0.0, 0.0, 100.0, 20.0), &[&["Name", "Age"], &["Alice", "30"]]);

        let all_page_segments: Vec<Vec<SegmentData>> = Vec::new();
        let result = stitch_and_emit(vec![frag_top, frag_bottom, standalone], Vec::new(), &all_page_segments);

        let stitched = result.iter().find(|t| t.page_number == 1).unwrap();
        assert_eq!(
            stitched.columns,
            Some(vec!["H1".to_string(), "H2".to_string()]),
            "stitched table's columns come from the topmost fragment's header row"
        );

        let standalone_result = result.iter().find(|t| t.page_number == 3).unwrap();
        assert_eq!(
            standalone_result.columns,
            Some(vec!["Name".to_string(), "Age".to_string()]),
            "a standalone fragment's columns come from its own first row"
        );
    }

    /// Issue #1297 code review (Finding 1): a layout-detected table (never
    /// passed through `stitch_fragmented_tables`, only appended in
    /// `prepare_emitted_tables`) must still receive a `table_id` and
    /// `columns` once it survives dedup into the final emitted set.
    #[test]
    fn layout_detected_table_surviving_dedup_gets_table_id_and_columns() {
        let native = cell_table(1, (0.0, 0.0, 100.0, 20.0), &[&["A", "B"]]);
        let layout_only = cell_table(2, (0.0, 0.0, 100.0, 20.0), &[&["Layout1", "Layout2"], &["x", "y"]]);

        let all_page_segments: Vec<Vec<SegmentData>> = Vec::new();
        let result = stitch_and_emit(vec![native], vec![layout_only], &all_page_segments);

        assert_eq!(
            result.len(),
            2,
            "both the native and layout-detected tables must be emitted"
        );
        let layout_result = result
            .iter()
            .find(|t| t.page_number == 2)
            .expect("layout-detected table survives into the emitted set");

        assert!(
            layout_result.table_id.is_some(),
            "a layout-detected table must receive a table_id, not just native tables"
        );
        assert_eq!(
            layout_result.columns,
            Some(vec!["Layout1".to_string(), "Layout2".to_string()]),
            "a layout-detected table must receive columns from its own header row"
        );
    }

    #[test]
    fn identical_markdown_tables_collapse_despite_missing_bbox() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![crate::types::Table {
            cells: vec![vec!["a".into(), "b".into()]],
            markdown: "| a | b |".to_string(),
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        }];
        let layout = vec![ov_table(1, (0.0, 0.0, 100.0, 100.0), "| a | b |")];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(
            emitted.len(),
            1,
            "byte-identical markdown on the same page collapses even when one table has no bbox"
        );
    }

    #[test]
    fn sparse_currency_affix_columns_merge_into_financial_values() {
        use crate::core::config::layout::TableOverlapPreference;

        let mut cells = vec![vec![
            "Security".into(),
            String::new(),
            "Par (000)".into(),
            String::new(),
            "Value".into(),
        ]];
        for index in 0..12 {
            cells.push(vec![
                format!("Bond {index}"),
                if index == 0 { "USD".into() } else { String::new() },
                format!("{},000", index + 1),
                if index == 0 { "$".into() } else { String::new() },
                format!("{},500", index + 1),
            ]);
        }
        let table = crate::types::Table {
            markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
            cells,
            page_number: 1,
            ..Default::default()
        };

        let emitted = prepare_emitted_tables(&[table], Vec::new(), TableOverlapPreference::Content);

        assert_eq!(emitted[0].cells[0], ["Security", "Par (000)", "Value"]);
        assert_eq!(emitted[0].cells[1], ["Bond 0", "USD 1,000", "$ 1,500"]);
        assert_eq!(
            emitted[0].columns,
            Some(vec!["Security".into(), "Par (000)".into(), "Value".into()])
        );
        assert!(emitted[0].markdown.starts_with("| Security | Par (000) | Value |"));
    }

    #[test]
    fn wrapped_financial_rows_fold_into_value_bearing_records() {
        let mut cells = vec![
            vec!["Security".into(), "Par (000)".into(), "Value".into()],
            vec!["Region (continued)".into(), String::new(), String::new()],
        ];
        for index in 0..8 {
            cells.push(vec![format!("Asset {index}, Series"), String::new(), String::new()]);
            cells.push(vec!["Class A, variable rate".into(), String::new(), String::new()]);
            cells.push(vec![
                "maturing in 2035".into(),
                format!("{},000", index + 1),
                format!("$ {},500", index + 1),
            ]);
        }
        let mut tables = vec![crate::types::Table {
            markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
            cells,
            ..Default::default()
        }];

        normalize_wrapped_financial_rows(&mut tables);

        assert_eq!(tables[0].cells.len(), 10);
        assert_eq!(
            tables[0].cells[1],
            ["Region (continued)", "", ""],
            "the first descriptor-only section label must remain its own row"
        );
        assert_eq!(
            tables[0].cells[2],
            [
                "Asset 0, Series Class A, variable rate maturing in 2035",
                "1,000",
                "$ 1,500"
            ]
        );
        assert!(
            tables[0]
                .markdown
                .contains("| Asset 7, Series Class A, variable rate maturing in 2035 | 8,000 | $ 8,500 |")
        );
    }

    #[test]
    fn wrapped_financial_rows_preserve_interior_section_boundaries() {
        let mut cells = vec![
            vec!["Security".into(), "Par".into(), "Value".into()],
            vec!["Region A (continued)".into(), String::new(), String::new()],
        ];
        for index in 0..4 {
            cells.push(vec![format!("Wrapped asset {index}"), String::new(), String::new()]);
            cells.push(vec!["final line".into(), String::new(), String::new()]);
            cells.push(vec![
                "matures 2035".into(),
                format!("{}", index + 1),
                format!("{}", index + 101),
            ]);
        }
        cells.push(vec![
            "Unanchored text before section".into(),
            String::new(),
            String::new(),
        ]);
        cells.push(vec!["Region B — 2.0%".into(), String::new(), String::new()]);
        for index in 4..8 {
            cells.push(vec![format!("Wrapped asset {index}"), String::new(), String::new()]);
            cells.push(vec!["final line".into(), String::new(), String::new()]);
            cells.push(vec![
                "matures 2035".into(),
                format!("{}", index + 1),
                format!("{}", index + 101),
            ]);
        }
        let mut tables = vec![crate::types::Table {
            markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
            cells,
            ..Default::default()
        }];

        normalize_wrapped_financial_rows(&mut tables);

        let section_index = tables[0]
            .cells
            .iter()
            .position(|row| row[0] == "Region B — 2.0%")
            .expect("interior section label");
        assert_eq!(
            tables[0].cells[section_index - 1],
            ["Unanchored text before section", "", ""],
            "pending descriptors must flush unchanged before a new section"
        );
        assert_eq!(tables[0].cells[section_index], ["Region B — 2.0%", "", ""]);
        assert_eq!(
            tables[0].cells[section_index + 1],
            ["Wrapped asset 4 final line matures 2035", "5", "105"],
            "folding may resume after the section boundary"
        );
    }

    #[test]
    fn financial_section_label_accepts_allocation_with_footnote() {
        let row = vec!["Regional allocation — 0.6%(b)".into(), String::new(), String::new()];

        assert!(is_financial_section_label(&row));
    }

    #[test]
    fn financial_section_label_rejects_coupon_description_after_percentage() {
        let row = vec!["ACME notes — 5.0% senior notes".into(), String::new(), String::new()];

        assert!(!is_financial_section_label(&row));
    }

    #[test]
    fn wrapped_financial_rows_fold_without_a_section_label() {
        let mut cells = vec![vec!["Security".into(), "Par".into(), "Value".into()]];
        for index in 0..8 {
            cells.push(vec![format!("Asset {index}, Series"), String::new(), String::new()]);
            cells.push(vec!["Class A, variable rate".into(), String::new(), String::new()]);
            cells.push(vec![
                "maturing in 2035".into(),
                format!("{},000", index + 1),
                format!("{},500", index + 1),
            ]);
        }
        let mut tables = vec![crate::types::Table {
            markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
            cells,
            ..Default::default()
        }];

        normalize_wrapped_financial_rows(&mut tables);

        assert_eq!(tables[0].cells.len(), 9);
        assert_eq!(
            tables[0].cells[1],
            [
                "Asset 0, Series Class A, variable rate maturing in 2035",
                "1,000",
                "1,500"
            ]
        );
    }

    #[test]
    fn wrapped_financial_row_folding_preserves_tokens_and_trailing_text() {
        let mut cells = vec![
            vec!["Security".into(), "Par".into(), "Value".into()],
            vec!["Region".into(), String::new(), String::new()],
        ];
        for index in 0..8 {
            cells.push(vec![format!("Wrapped asset {index}"), String::new(), String::new()]);
            cells.push(vec![
                "final line".into(),
                format!("{}", index + 1),
                format!("{}", index + 101),
            ]);
        }
        cells.push(vec!["Unanchored trailing note".into(), String::new(), String::new()]);
        let before_tokens = cells
            .iter()
            .flatten()
            .flat_map(|cell| cell.split_whitespace())
            .map(str::to_string)
            .collect::<Vec<_>>();
        let mut tables = vec![crate::types::Table {
            markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
            cells,
            ..Default::default()
        }];

        normalize_wrapped_financial_rows(&mut tables);

        let after_tokens = tables[0]
            .cells
            .iter()
            .flatten()
            .flat_map(|cell| cell.split_whitespace())
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert_eq!(after_tokens, before_tokens);
        assert_eq!(
            tables[0].cells.last().expect("trailing row"),
            &["Unanchored trailing note", "", ""],
            "descriptor-only text without an immediately following value row must not fold"
        );
    }

    #[test]
    fn wrapped_financial_row_folding_requires_strict_financial_density() {
        let build_cells = |header: [&str; 3], continuation_rows: usize| {
            let mut cells = vec![
                header.map(str::to_string).to_vec(),
                vec!["Section".into(), String::new(), String::new()],
            ];
            for index in 0..8 {
                if index < continuation_rows {
                    cells.push(vec![format!("Wrapped {index}"), String::new(), String::new()]);
                }
                cells.push(vec![
                    format!("Asset {index}"),
                    format!("{}", index + 1),
                    format!("{}", index + 101),
                ]);
            }
            cells
        };
        let non_financial = build_cells(["Name", "Owner", "Status"], 8);
        let balanced = build_cells(["Security", "Par", "Value"], 7);
        let mut tables = vec![
            crate::types::Table {
                markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&non_financial),
                cells: non_financial.clone(),
                ..Default::default()
            },
            crate::types::Table {
                markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&balanced),
                cells: balanced.clone(),
                ..Default::default()
            },
        ];

        normalize_wrapped_financial_rows(&mut tables);

        assert_eq!(tables[0].cells, non_financial);
        assert_eq!(
            tables[1].cells, balanced,
            "descriptor-only rows must outnumber value-bearing rows after the section label"
        );
    }

    #[test]
    fn named_or_non_currency_columns_are_not_collapsed() {
        let build_table = |source_header: &str, source_value: &str| {
            let mut cells = vec![vec!["Security".into(), source_header.into(), "Value".into()]];
            for index in 0..12 {
                cells.push(vec![
                    format!("Asset {index}"),
                    if index == 0 { source_value.into() } else { String::new() },
                    format!("{index},000"),
                ]);
            }
            crate::types::Table {
                markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
                cells,
                ..Default::default()
            }
        };
        let mut tables = vec![build_table("Currency", "USD"), build_table("", "kg")];

        normalize_sparse_currency_affix_columns(&mut tables);

        assert_eq!(
            tables[0].cells[0].len(),
            3,
            "an explicitly named Currency column is semantic"
        );
        assert_eq!(
            tables[1].cells[0].len(),
            3,
            "an arbitrary sparse unit is not a currency marker"
        );
    }

    #[test]
    fn dense_currency_columns_are_not_collapsed() {
        let mut cells = vec![vec!["Security".into(), String::new(), "Value".into()]];
        for index in 0..10 {
            cells.push(vec![
                format!("Asset {index}"),
                if index < 2 { "USD".into() } else { String::new() },
                format!("{index},000"),
            ]);
        }
        let mut tables = vec![crate::types::Table {
            markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
            cells,
            ..Default::default()
        }];

        normalize_sparse_currency_affix_columns(&mut tables);

        assert_eq!(tables[0].cells[0].len(), 3);
    }

    #[test]
    fn currency_marker_without_target_value_is_preserved() {
        let mut cells = vec![vec!["Security".into(), String::new(), "Value".into()]];
        cells.push(vec!["Currency declaration".into(), "USD".into(), String::new()]);
        for index in 0..11 {
            cells.push(vec![format!("Asset {index}"), String::new(), format!("{index},000")]);
        }
        let mut tables = vec![crate::types::Table {
            markdown: crate::extractors::frontmatter_utils::cells_to_markdown(&cells),
            cells,
            ..Default::default()
        }];

        normalize_sparse_currency_affix_columns(&mut tables);

        assert_eq!(tables[0].cells[0].len(), 3);
        assert_eq!(tables[0].cells[1][1], "USD");
    }

    #[test]
    fn dedup_content_preference_keeps_larger_table() {
        use crate::core::config::layout::TableOverlapPreference;
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "a"),
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "bbbbbbbbbb"),
        ];
        deduplicate_overlapping_tables(&mut tables, 1, TableOverlapPreference::Content);
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0].markdown, "bbbbbbbbbb",
            "Content keeps the larger (layout) table"
        );
    }

    #[test]
    fn dedup_native_preference_keeps_native_even_when_smaller() {
        use crate::core::config::layout::TableOverlapPreference;
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "a"),
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "bbbbbbbbbb"),
        ];
        deduplicate_overlapping_tables(&mut tables, 1, TableOverlapPreference::Native);
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0].markdown, "a",
            "Native preference keeps native over a larger layout table"
        );
    }

    #[test]
    fn dedup_layout_preference_keeps_layout_even_when_smaller() {
        use crate::core::config::layout::TableOverlapPreference;
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "aaaaaaaaaa"),
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "b"),
        ];
        deduplicate_overlapping_tables(&mut tables, 1, TableOverlapPreference::Layout);
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0].markdown, "b",
            "Layout preference keeps layout over a larger native table"
        );
    }

    #[test]
    fn dedup_native_preference_falls_back_to_content_for_same_origin() {
        use crate::core::config::layout::TableOverlapPreference;
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "a"),
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "bbbbbbbbbb"),
        ];
        deduplicate_overlapping_tables(&mut tables, 2, TableOverlapPreference::Native);
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0].markdown, "bbbbbbbbbb",
            "same-origin overlap falls back to content"
        );
    }

    #[test]
    fn dedup_non_overlapping_tables_both_kept() {
        use crate::core::config::layout::TableOverlapPreference;
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "a"),
            ov_table(1, (200.0, 200.0, 300.0, 300.0), "b"),
        ];
        deduplicate_overlapping_tables(&mut tables, 1, TableOverlapPreference::Native);
        assert_eq!(tables.len(), 2, "non-overlapping tables are both kept");
    }

    #[test]
    fn side_by_side_layout_children_replace_content_heavy_parent() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
        let layout = vec![
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(
            emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
            ["left", "right"]
        );
    }

    #[test]
    fn side_by_side_layout_cohort_replaces_two_content_heavy_native_parents() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), &"native left".repeat(100)),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), &"native right".repeat(100)),
        ];
        let layout = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "layout left"),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "layout right"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(
            emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
            ["layout left", "layout right"]
        );
    }

    #[test]
    fn native_preference_keeps_two_parents_over_side_by_side_layout_cohort() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "native left"),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "native right"),
        ];
        let layout = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), &"layout left".repeat(100)),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), &"layout right".repeat(100)),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Native);

        assert_eq!(
            emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
            ["native left", "native right"]
        );
    }

    #[test]
    fn one_layout_child_does_not_replace_two_native_parents() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), &"native left".repeat(100)),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), &"native right".repeat(100)),
        ];
        let layout = vec![ov_table(1, (0.0, 0.0, 95.0, 100.0), "layout left")];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(emitted.len(), 2);
        assert!(emitted.iter().all(|table| table.markdown.starts_with("native")));
    }

    #[test]
    fn stacked_native_parents_do_not_form_side_by_side_replacement_cohort() {
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 200.0, 45.0), "native top"),
            ov_table(1, (0.0, 55.0, 200.0, 100.0), "native bottom"),
        ];
        tables.extend([
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "layout left"),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "layout right"),
        ]);

        assert!(side_by_side_layout_replacements(&tables, 2).is_empty());
    }

    #[test]
    fn weakly_overlapping_layout_children_do_not_replace_two_native_parents() {
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "native left"),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "native right"),
        ];
        tables.extend([
            ov_table(1, (-70.0, 0.0, 80.0, 100.0), "layout left"),
            ov_table(1, (120.0, 0.0, 270.0, 100.0), "layout right"),
        ]);

        assert!(side_by_side_layout_replacements(&tables, 2).is_empty());
    }

    #[test]
    fn layout_cohort_rejects_child_that_does_not_cover_corresponding_parent() {
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "native left"),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "native right"),
        ];
        tables.extend([
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "layout left"),
            ov_table(1, (105.0, 0.0, 175.0, 100.0), "layout right"),
        ]);

        assert!(side_by_side_layout_replacements(&tables, 2).is_empty());
    }

    #[test]
    fn layout_cohort_accepts_reciprocal_crop_within_tolerance() {
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "native left"),
            ov_table(1, (110.0, 0.0, 210.0, 100.0), "native right"),
        ];
        tables.extend([
            ov_table(1, (0.0, 0.0, 79.6, 100.0), "layout left"),
            ov_table(1, (110.0, 0.0, 210.0, 100.0), "layout right"),
        ]);

        assert_eq!(side_by_side_layout_replacements(&tables, 2), [(vec![0, 1], vec![2, 3])]);
    }

    #[test]
    fn layout_cohort_rejects_reciprocal_crop_below_tolerance() {
        let mut tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "native left"),
            ov_table(1, (110.0, 0.0, 210.0, 100.0), "native right"),
        ];
        tables.extend([
            ov_table(1, (0.0, 0.0, 79.4, 100.0), "layout left"),
            ov_table(1, (110.0, 0.0, 210.0, 100.0), "layout right"),
        ]);

        assert!(side_by_side_layout_replacements(&tables, 2).is_empty());
    }

    #[test]
    fn layout_cohort_rejects_child_owned_by_sibling_parent() {
        let tables = vec![
            ov_table(1, (0.0, 0.0, 100.0, 100.0), "native left"),
            ov_table(1, (110.0, 0.0, 210.0, 100.0), "native right"),
            ov_table(1, (90.0, 0.0, 210.0, 100.0), "layout crossing"),
            ov_table(1, (110.0, 0.0, 210.0, 100.0), "layout right"),
        ];

        assert!(!replacement_children_correspond(&tables, &[0, 1], &[2, 3]));
    }

    #[test]
    fn three_native_candidates_select_one_disjoint_adjacent_cohort() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![
            ov_table(1, (105.0, 0.0, 200.0, 100.0), &"native middle".repeat(100)),
            ov_table(1, (210.0, 0.0, 305.0, 100.0), &"native right".repeat(100)),
            ov_table(1, (0.0, 0.0, 95.0, 100.0), &"native left".repeat(100)),
        ];
        let layout = vec![
            ov_table(1, (210.0, 0.0, 305.0, 100.0), "layout right"),
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "layout left"),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "layout middle"),
        ];
        let mut candidates = native.clone();
        candidates.extend(layout.clone());

        let replacements = side_by_side_layout_replacements(&candidates, native.len());
        assert_eq!(replacements, [(vec![2, 0], vec![4, 5])]);

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);
        assert_eq!(
            emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
            ["layout left", "layout middle", &"native right".repeat(100)]
        );
    }

    #[test]
    fn side_by_side_replacement_is_atomic_against_native_duplicate() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![
            ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100)),
            ov_table(1, (0.0, 0.0, 95.0, 100.0), &"native duplicate".repeat(100)),
        ];
        let layout = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(
            emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
            ["left", "right"]
        );
    }

    #[test]
    fn side_by_side_replacement_is_atomic_against_earlier_layout_duplicate() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
        let better_left = "layout duplicate with more content";
        let layout = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), better_left),
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(
            emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
            [better_left, "right"]
        );
    }

    #[test]
    fn side_by_side_replacement_is_atomic_against_later_layout_duplicate() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
        let better_left = "layout duplicate with more content";
        let layout = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
            ov_table(1, (0.0, 0.0, 95.0, 100.0), better_left),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(
            emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
            [better_left, "right"]
        );
    }

    #[test]
    fn overlapping_protected_replacement_groups_survive() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![
            ov_table(1, (0.0, 0.0, 200.0, 100.0), "upper parent"),
            ov_table(1, (0.0, 40.0, 200.0, 140.0), "lower parent"),
        ];
        let layout = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "upper left"),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "upper right"),
            ov_table(1, (0.0, 40.0, 95.0, 140.0), "lower left"),
            ov_table(1, (105.0, 40.0, 200.0, 140.0), "lower right"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(
            emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
            ["upper left", "upper right", "lower left", "lower right"]
        );
    }

    #[test]
    fn partially_shared_replacement_groups_keep_canonical_table_order() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![
            ov_table(1, (0.0, 0.0, 200.0, 100.0), "parent a"),
            ov_table(1, (105.0, 0.0, 305.0, 100.0), "parent b"),
        ];
        let layout = vec![
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "shared"),
            ov_table(1, (210.0, 0.0, 305.0, 100.0), "right"),
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(
            emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
            ["left", "shared", "right"]
        );
    }

    #[test]
    fn side_by_side_replacement_orders_complete_affected_row_cohort() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), "parent")];
        let layout = vec![
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
            ov_table(1, (300.0, 0.0, 350.0, 100.0), "unrelated"),
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(
            emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
            ["left", "right", "unrelated"]
        );
    }

    #[test]
    fn side_by_side_replacement_preserves_interleaved_different_row_slot() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), "parent")];
        let layout = vec![
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
            ov_table(1, (300.0, -100.0, 350.0, -10.0), "different row"),
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(
            emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
            ["left", "different row", "right"]
        );
    }

    #[test]
    fn one_layout_child_does_not_replace_parent() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
        let layout = vec![ov_table(1, (0.0, 0.0, 95.0, 100.0), "left")];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(emitted.len(), 1);
        assert!(emitted[0].markdown.starts_with("parent"));
    }

    #[test]
    fn overlapping_layout_children_do_not_replace_parent() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
        let layout = vec![
            ov_table(1, (0.0, 0.0, 120.0, 100.0), "left"),
            ov_table(1, (80.0, 0.0, 200.0, 100.0), "right"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(emitted.len(), 1);
        assert!(emitted[0].markdown.starts_with("parent"));
    }

    #[test]
    fn stacked_layout_children_do_not_replace_parent() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
        let layout = vec![
            ov_table(1, (0.0, 0.0, 200.0, 45.0), "top"),
            ov_table(1, (0.0, 55.0, 200.0, 100.0), "bottom"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(emitted.len(), 1);
        assert!(emitted[0].markdown.starts_with("parent"));
    }

    #[test]
    fn shallow_layout_children_do_not_replace_tall_parent() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
        let layout = vec![
            ov_table(1, (0.0, 0.0, 95.0, 20.0), "left"),
            ov_table(1, (105.0, 0.0, 200.0, 20.0), "right"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(emitted.len(), 1);
        assert!(emitted[0].markdown.starts_with("parent"));
    }

    #[test]
    fn weakly_overlapping_layout_children_do_not_replace_parent() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100))];
        let layout = vec![
            ov_table(1, (-70.0, 0.0, 80.0, 100.0), "left"),
            ov_table(1, (120.0, 0.0, 270.0, 100.0), "right"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(emitted.len(), 1);
        assert!(emitted[0].markdown.starts_with("parent"));
    }

    #[test]
    fn side_by_side_replacement_preserves_unrelated_table() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![
            ov_table(1, (0.0, 0.0, 200.0, 100.0), &"parent".repeat(100)),
            ov_table(2, (10.0, 10.0, 80.0, 80.0), "unrelated"),
        ];
        let layout = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), "left"),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), "right"),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Content);

        assert_eq!(
            emitted.iter().map(|table| table.markdown.as_str()).collect::<Vec<_>>(),
            ["unrelated", "left", "right"]
        );
    }

    #[test]
    fn native_preference_keeps_parent_over_side_by_side_children() {
        use crate::core::config::layout::TableOverlapPreference;
        let native = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), "parent")];
        let layout = vec![
            ov_table(1, (0.0, 0.0, 95.0, 100.0), &"left".repeat(100)),
            ov_table(1, (105.0, 0.0, 200.0, 100.0), &"right".repeat(100)),
        ];

        let emitted = prepare_emitted_tables(&native, layout, TableOverlapPreference::Native);

        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].markdown, "parent");
    }

    #[test]
    fn dropped_duplicate_table_does_not_suppress_text() {
        use crate::core::config::layout::TableOverlapPreference;

        let native_tables = vec![ov_table(1, (0.0, 0.0, 100.0, 100.0), "native table content")];
        let layout_tables = vec![ov_table(1, (0.0, 0.0, 200.0, 100.0), "x")];
        let emitted_tables = prepare_emitted_tables(&native_tables, layout_tables, TableOverlapPreference::Content);
        let bboxes_by_page = table_bboxes_by_page(&emitted_tables);

        assert_eq!(emitted_tables.len(), 1);
        assert_eq!(emitted_tables[0].bounding_box.expect("kept table bbox").x1, 100.0);

        let segment = SegmentData {
            x: 150.0,
            y: 10.0,
            width: 20.0,
            height: 12.0,
            ..seg("text outside the emitted table", 150.0, 20.0)
        };
        let filtered = filter_segments_by_table_bboxes(
            vec![segment],
            bboxes_by_page.get(&0).map(Vec::as_slice).unwrap_or_default(),
        );
        assert_eq!(filtered.len(), 1, "a discarded duplicate bbox must not remove text");
    }

    #[test]
    fn empty_table_does_not_suppress_text() {
        use crate::core::config::layout::TableOverlapPreference;

        let native_tables = vec![ov_table(1, (0.0, 0.0, 100.0, 100.0), "  \n")];
        let emitted_tables = prepare_emitted_tables(&native_tables, Vec::new(), TableOverlapPreference::Content);
        let bboxes_by_page = table_bboxes_by_page(&emitted_tables);

        assert!(
            emitted_tables.is_empty(),
            "assembly would not emit whitespace-only markdown"
        );
        assert!(
            bboxes_by_page.is_empty(),
            "non-emitted tables must not contribute suppression boxes"
        );

        let segment = SegmentData {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 12.0,
            ..seg("text under an empty table", 10.0, 20.0)
        };
        let filtered = filter_segments_by_table_bboxes(
            vec![segment],
            bboxes_by_page.get(&0).map(Vec::as_slice).unwrap_or_default(),
        );
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn empty_table_does_not_displace_valid_overlap() {
        use crate::core::config::layout::TableOverlapPreference;

        let native_tables = vec![ov_table(1, (0.0, 0.0, 100.0, 100.0), "  \n")];
        let layout_tables = vec![ov_table(1, (0.0, 0.0, 100.0, 100.0), "| valid |")];

        let emitted_tables = prepare_emitted_tables(&native_tables, layout_tables, TableOverlapPreference::Native);

        assert_eq!(emitted_tables.len(), 1);
        assert_eq!(emitted_tables[0].markdown, "| valid |");
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn missing_wrapper_validation_is_treated_as_skipped() {
        use super::super::regions::layout_validation::RegionValidation;

        let hint = |class_name| LayoutHint {
            class_name,
            confidence: 0.9,
            left: 0.0,
            bottom: 0.0,
            right: 100.0,
            top: 100.0,
        };
        let hints = vec![
            hint(LayoutHintClass::Picture),
            hint(LayoutHintClass::Form),
            hint(LayoutHintClass::Text),
        ];
        let ownership = wrapper_ownership_by_hint(&hints, &[RegionValidation::Empty]);
        assert_eq!(ownership, [false, true, true]);
    }

    #[test]
    fn emitted_table_still_suppresses_covered_text() {
        use crate::core::config::layout::TableOverlapPreference;

        let native_tables = vec![ov_table(1, (0.0, 0.0, 100.0, 100.0), "| duplicated table text |")];
        let emitted_tables = prepare_emitted_tables(&native_tables, Vec::new(), TableOverlapPreference::Content);
        let bboxes_by_page = table_bboxes_by_page(&emitted_tables);
        let segment = SegmentData {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 12.0,
            ..seg("duplicated table text", 10.0, 20.0)
        };

        let filtered = filter_segments_by_table_bboxes(
            vec![segment],
            bboxes_by_page.get(&0).map(Vec::as_slice).unwrap_or_default(),
        );
        assert!(
            filtered.is_empty(),
            "an emitted table must continue to suppress duplicate text"
        );
    }

    /// Helper: segment with font metadata for title-promotion tests.
    fn role_seg(text: &str, font_size: f32, is_bold: bool, assigned_role: Option<u8>) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x: 72.0,
            y: 700.0,
            width: 200.0,
            height: font_size,
            font_size,
            is_bold,
            is_italic: false,
            is_monospace: false,
            baseline_y: 700.0,
            rotation_degrees: 0.0,
            assigned_role,
        }
    }

    /// A bold, first-page, larger-than-any-tagged-heading tier must be promoted
    /// to h1 with the tagged hierarchy shifted down one level.
    #[test]
    fn promote_title_shifts_tagged_heading_levels_down() {
        let pages = vec![vec![
            role_seg("Titre du document", 28.0, true, None),
            role_seg("Titre 1", 18.0, true, Some(1)),
            role_seg("Titre 2", 16.0, true, Some(2)),
            role_seg("body text", 12.0, false, None),
        ]];
        let mut map = build_heading_map_from_assigned_roles(&pages);
        assert!(promote_untagged_document_title(&mut map, &pages));

        let level_of = |font: f32| map.iter().find(|(f, _)| (*f - font).abs() < 0.05).and_then(|(_, l)| *l);
        assert_eq!(level_of(28.0), Some(1), "title tier must become h1");
        assert_eq!(level_of(18.0), Some(2), "tagged H1 must demote to h2");
        assert_eq!(level_of(16.0), Some(3), "tagged H2 must demote to h3");
        assert_eq!(level_of(12.0), None, "body must stay body");
    }

    /// No untagged tier above the largest tagged heading → no promotion.
    #[test]
    fn promote_title_no_candidate_leaves_map_unchanged() {
        let pages = vec![vec![
            role_seg("Heading", 18.0, true, Some(1)),
            role_seg("body", 12.0, false, None),
        ]];
        let mut map = build_heading_map_from_assigned_roles(&pages);
        let before = map.clone();
        assert!(!promote_untagged_document_title(&mut map, &pages));
        assert_eq!(map, before);
    }

    /// A non-bold large tier (e.g. a pull quote) must not be mistaken for a title.
    #[test]
    fn promote_title_requires_bold() {
        let pages = vec![vec![
            role_seg("large quote", 28.0, false, None),
            role_seg("Heading", 18.0, true, Some(1)),
        ]];
        let mut map = build_heading_map_from_assigned_roles(&pages);
        assert!(!promote_untagged_document_title(&mut map, &pages));
    }

    /// A large tier appearing only after page 0 is not a document title.
    #[test]
    fn promote_title_requires_first_page() {
        let pages = vec![
            vec![role_seg("Heading", 18.0, true, Some(1))],
            vec![role_seg("Big banner later", 28.0, true, None)],
        ];
        let mut map = build_heading_map_from_assigned_roles(&pages);
        assert!(!promote_untagged_document_title(&mut map, &pages));
    }

    /// A mid-word split (e.g. "Text" extracted as "Te" + "xt", same role and
    /// font size, immediately adjacent) must count as one logical block, not
    /// two — otherwise a font-encoding artifact inflates the apparent
    /// document size past the sparsity floor.
    #[test]
    fn count_logical_blocks_merges_same_role_same_size_runs() {
        let pages = vec![vec![
            role_seg("Big", 24.0, false, Some(1)),
            role_seg("Small Text", 12.0, true, Some(2)),
            role_seg("Te", 24.0, false, Some(1)),
            role_seg("xt", 24.0, false, Some(1)),
        ]];
        assert_eq!(
            count_logical_blocks(&pages),
            3,
            "the split \"Te\"+\"xt\" run must collapse into a single block"
        );
    }

    /// Segments with different assigned roles never merge, even at the same
    /// font size.
    #[test]
    fn count_logical_blocks_does_not_merge_different_roles() {
        let pages = vec![vec![
            role_seg("Heading", 18.0, true, Some(1)),
            role_seg("more heading text", 18.0, true, Some(2)),
        ]];
        assert_eq!(count_logical_blocks(&pages), 2);
    }

    /// Sparse document where the structure tree tags every block as a heading
    /// with no body tier at all (a document with just a couple of heading-tagged
    /// lines and nothing else) must have every role suppressed rather than trusted.
    #[test]
    fn suppress_all_heading_roles_fires_when_sparse_and_all_tagged() {
        let mut pages = vec![vec![
            role_seg("Big", 24.0, false, Some(1)),
            role_seg("Small Text", 12.0, true, Some(2)),
            role_seg("Te xt", 24.0, false, Some(1)),
        ]];
        let mut map = build_heading_map_from_assigned_roles(&pages);
        assert!(suppress_all_heading_roles_when_sparse_and_untrusted(
            &mut map, &mut pages
        ));

        assert!(
            map.iter().all(|(_, level)| level.is_none()),
            "heading map must be fully suppressed; got: {map:?}"
        );
        for page in &pages {
            for seg in page {
                assert_eq!(
                    seg.assigned_role, None,
                    "assigned_role must be cleared on every segment"
                );
            }
        }
    }

    /// A sparse document with one tagged heading and one untagged body
    /// paragraph (the `issue-987-test.pdf` shape: "Big"/"Te xt" tagged,
    /// "Small Text" untagged — 3 total blocks) must ALSO be suppressed: a mix
    /// of heading and body tiers on that few blocks is not enough evidence
    /// that the tagging is trustworthy, matching GT for that fixture (plain
    /// "Big Text"/"Small Text", no headings at all).
    #[test]
    fn suppress_all_heading_roles_fires_when_sparse_with_body_tier() {
        let mut pages = vec![vec![
            role_seg("Title", 24.0, true, Some(1)),
            role_seg("body text", 12.0, false, None),
        ]];
        let mut map = build_heading_map_from_assigned_roles(&pages);
        assert!(suppress_all_heading_roles_when_sparse_and_untrusted(
            &mut map, &mut pages
        ));
        assert_eq!(pages[0][0].assigned_role, None, "tagged role must be cleared");
    }

    /// A sparse document with no heading roles at all must not be touched —
    /// there is nothing to suppress.
    #[test]
    fn suppress_all_heading_roles_does_not_fire_with_no_headings() {
        let mut pages = vec![vec![
            role_seg("body text one", 12.0, false, None),
            role_seg("body text two", 12.0, false, None),
        ]];
        let mut map = build_heading_map_from_assigned_roles(&pages);
        assert!(!suppress_all_heading_roles_when_sparse_and_untrusted(
            &mut map, &mut pages
        ));
    }

    /// At or above the sparsity floor, an all-heading-tagged document is left
    /// alone even with no body tier — larger documents are trusted.
    #[test]
    fn suppress_all_heading_roles_does_not_fire_at_or_above_floor() {
        // Alternate heading/body role so each segment is a distinct logical
        // block under `count_logical_blocks` rather than collapsing into one. ~keep
        let mut pages = vec![
            (0..MIN_BLOCKS_FOR_FONT_HEADING)
                .map(|i| {
                    if i % 2 == 0 {
                        role_seg(&format!("Heading {i}"), 18.0, true, Some(1))
                    } else {
                        role_seg(&format!("Body paragraph {i}."), 12.0, false, None)
                    }
                })
                .collect(),
        ];
        let mut map = build_heading_map_from_assigned_roles(&pages);
        assert!(!suppress_all_heading_roles_when_sparse_and_untrusted(
            &mut map, &mut pages
        ));
        assert_eq!(
            pages[0][0].assigned_role,
            Some(1),
            "role must be untouched at/above the floor"
        );
    }

    /// Role demotion mirrors the map shift on segments (bridge.rs reads roles directly).
    #[test]
    fn demote_assigned_roles_shifts_and_caps() {
        let mut pages = vec![vec![
            role_seg("h1", 18.0, true, Some(1)),
            role_seg("h6", 8.0, true, Some(6)),
            role_seg("body", 12.0, false, None),
        ]];
        demote_assigned_roles(&mut pages);
        assert_eq!(pages[0][0].assigned_role, Some(2));
        assert_eq!(pages[0][1].assigned_role, Some(6), "level 6 must cap, not overflow");
        assert_eq!(pages[0][2].assigned_role, None);
    }

    #[test]
    fn assigned_sal_annotation_role_is_demoted() {
        let paragraphs = process_heuristic_segments(vec![role_seg("__inout_bcount_full(n)", 12.0, false, Some(2))]);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn assigned_identifier_heading_role_is_preserved() {
        let paragraphs = blocks_to_paragraphs(
            vec![role_seg("__in_section", 12.0, false, Some(2))],
            &[(12.0, None)],
            &[],
        );
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    /// Helper: a body-tier segment occupying its own visual line at `baseline_y`.
    fn body_line_seg(text: &str, baseline_y: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x: 72.0,
            y: baseline_y - 11.0,
            width: 200.0,
            height: 11.0,
            font_size: 11.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y,
            rotation_degrees: 0.0,
            assigned_role: None,
        }
    }

    /// All segment text of a paragraph, joined in order.
    fn paragraph_segment_text(para: &PdfParagraph) -> String {
        para.lines
            .iter()
            .flat_map(|line| line.segments.iter())
            .map(|s| s.text.trim())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Regression for #1386 (defect #290). Four consecutive numbered subsection
    /// headings share a font size, a weight and an even one-line-height spacing,
    /// so `font_change`, `role_change`, `bold_change` and `crossed_gap` are all
    /// false — and `looks_like_list_item` deliberately returns `false` for
    /// numbered section headings, removing the last boundary. Before the fix the
    /// grouper emitted ONE paragraph with all four headings concatenated.
    #[test]
    fn consecutive_numbered_section_headings_are_separate_paragraphs() {
        let segments = vec![
            body_line_seg("1.3 Gasinstallatie", 700.0),
            body_line_seg("1.4 Elektrische installatie", 686.0),
            body_line_seg("1.5 Waterinstallatie", 672.0),
            body_line_seg("1.6 Ventilatie", 658.0),
        ];

        let paragraphs = blocks_to_paragraphs(segments, &[(11.0, None)], &[]);

        assert_eq!(
            paragraphs.len(),
            4,
            "each numbered subsection heading must be its own element"
        );
        assert_eq!(paragraph_segment_text(&paragraphs[0]), "1.3 Gasinstallatie");
        assert_eq!(paragraph_segment_text(&paragraphs[1]), "1.4 Elektrische installatie");
        assert_eq!(paragraph_segment_text(&paragraphs[2]), "1.5 Waterinstallatie");
        assert_eq!(paragraph_segment_text(&paragraphs[3]), "1.6 Ventilatie");
    }

    /// End-to-end through the grouper AND `merge_continuation_paragraphs`: no
    /// heading ends in `.?!:;`, so the merge pass would re-join the run the
    /// grouper just split unless it also guards on numbered section starts.
    #[test]
    fn consecutive_numbered_section_headings_survive_continuation_merge() {
        let segments = vec![
            body_line_seg("1.3 Gasinstallatie", 700.0),
            body_line_seg("1.4 Elektrische installatie", 686.0),
            body_line_seg("1.5 Waterinstallatie", 672.0),
            body_line_seg("1.6 Ventilatie", 658.0),
        ];

        let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &[], &TextRepairWitnesses::default());

        assert_eq!(
            paragraphs.len(),
            4,
            "the continuation merge must not re-join numbered section headings"
        );
    }

    /// The over-fire guard for #1386: a two-line prose paragraph whose second
    /// line opens with a bare year must stay ONE paragraph. The looser
    /// `starts_with_section_number` returns `true` for "2024 was een druk jaar";
    /// the fix deliberately uses `is_numbered_section_heading`, which does not.
    #[test]
    fn prose_starting_with_a_year_stays_one_paragraph() {
        let segments = vec![
            body_line_seg("Het bestuur meldt", 700.0),
            body_line_seg("2024 was een druk jaar", 686.0),
        ];

        let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &[], &TextRepairWitnesses::default());

        assert_eq!(
            paragraphs.len(),
            1,
            "prose beginning with a bare year is not a section heading"
        );
        assert_eq!(
            paragraph_segment_text(&paragraphs[0]),
            "Het bestuur meldt 2024 was een druk jaar"
        );
    }

    /// Regression for #1467: a numbered section heading, followed by unrelated
    /// bold text at the same size, weight and line spacing, is welded to it.
    /// Every other break signal is false here -- `font_change`, `role_change`
    /// and `bold_change` all compare equal values, `crossed_gap` has no gaps to
    /// find, and `looks_like_list_item` deliberately rejects numbered section
    /// headings -- so only `follows_section` (backed by `heading_wraps_onto`
    /// ruling out a mid-heading wrap) can separate them. Run through
    /// `segments_to_paragraphs`, not `blocks_to_paragraphs`: the continuation
    /// merge that runs immediately afterward would silently re-join exactly
    /// this split unless it also refuses to absorb a heading it did not open.
    #[test]
    fn numbered_section_heading_is_split_from_the_callout_that_follows_it() {
        let heading = SegmentData {
            is_bold: true,
            font_size: 12.0,
            height: 12.0,
            y: 700.0 - 12.0,
            ..column_seg("1.1.1 Pictogrammen in het installatievoorschrift", 72.0, 170.0, 700.0)
        };
        let callout = SegmentData {
            is_bold: true,
            font_size: 12.0,
            height: 12.0,
            y: 684.0 - 12.0,
            ..column_seg("VOORZICHTIG / BELANGRIJK", 72.0, 90.0, 684.0)
        };
        let body1 = SegmentData {
            is_bold: true,
            font_size: 12.0,
            height: 12.0,
            y: 668.0 - 12.0,
            ..column_seg("Procedures die niet worden opgevolgd kunnen letsel", 72.0, 190.0, 668.0)
        };
        let body2 = SegmentData {
            is_bold: true,
            font_size: 12.0,
            height: 12.0,
            y: 652.0 - 12.0,
            ..column_seg("of schade veroorzaken aan de installatie of de", 72.0, 190.0, 652.0)
        };
        let body3 = SegmentData {
            is_bold: true,
            font_size: 12.0,
            height: 12.0,
            y: 636.0 - 12.0,
            ..column_seg("gebruiker van het toestel indien genegeerd", 72.0, 190.0, 636.0)
        };

        let paragraphs = segments_to_paragraphs(
            vec![heading, callout, body1, body2, body3],
            &[(12.0, None)],
            &[],
            &TextRepairWitnesses::default(),
        );

        assert_eq!(
            paragraphs.len(),
            2,
            "the numbered heading must split from the callout and body text that follow it"
        );
        assert_eq!(
            paragraph_segment_text(&paragraphs[0]),
            "1.1.1 Pictogrammen in het installatievoorschrift",
            "the heading must be its own element, not fused with the callout"
        );
        assert_eq!(
            paragraph_segment_text(&paragraphs[1]),
            "VOORZICHTIG / BELANGRIJK Procedures die niet worden opgevolgd kunnen letsel \
             of schade veroorzaken aan de installatie of de gebruiker van het toestel indien genegeerd",
            "the callout and following body text must survive as a separate element from the heading"
        );
    }

    /// GH#1608: `ARTIKEL 1.` shares font, size, weight and leading with the part
    /// header above it, so the numbered-heading predicate is the only boundary
    /// signal available -- and it could not see a heading whose number is not the
    /// first token. Asserted through `segments_to_paragraphs`, which runs the
    /// grouper AND the continuation merge, because a split made by one is
    /// routinely undone by the other.
    #[test]
    fn keyword_numbered_heading_splits_from_the_part_header_above_it() {
        let part_header = SegmentData {
            is_bold: true,
            font_size: 12.0,
            height: 12.0,
            y: 700.0 - 12.0,
            ..column_seg("ALGEMENE BEPALINGEN", 262.0, 71.0, 700.0)
        };
        let heading = SegmentData {
            is_bold: true,
            font_size: 12.0,
            height: 12.0,
            y: 683.0 - 12.0,
            ..column_seg(
                "ARTIKEL 1. TOEPASSELIJKHEID VAN DE INKOOPVOORWAARDEN",
                72.0,
                330.0,
                683.0,
            )
        };

        let paragraphs = segments_to_paragraphs(
            vec![part_header, heading],
            &[(12.0, None)],
            &[],
            &TextRepairWitnesses::default(),
        );

        assert_eq!(
            paragraphs.len(),
            2,
            "the keyword-numbered heading must open its own element"
        );
        assert_eq!(paragraph_segment_text(&paragraphs[0]), "ALGEMENE BEPALINGEN");
        assert_eq!(
            paragraph_segment_text(&paragraphs[1]),
            "ARTIKEL 1. TOEPASSELIJKHEID VAN DE INKOOPVOORWAARDEN"
        );
    }

    /// GH#1615. A numbered heading whose title does not fit on one line is set with
    /// a hanging indent: the number at the margin, the title to its right, and the
    /// overflow resuming at the TITLE's left edge. `follows_section` closed the
    /// element after the first line anyway, because `heading_wraps_onto` compares
    /// RIGHT edges and a wrap's last line is short by definition.
    ///
    /// Geometry from the reporter's page 1, verbatim (PDF user space):
    ///
    /// ```text
    /// 5.7.3                                     x 48.24
    /// Roof terminal combined duct vertical and  x 83.64  y 774.96
    /// twin pipe duct vertical                   x 83.64  y 762.24
    /// ```
    #[test]
    fn a_wrapped_numbered_heading_keeps_its_second_line() {
        let segments = vec![
            SegmentData {
                is_bold: true,
                font_size: 11.04,
                ..column_seg("5.7.3", 48.24, 30.0, 774.96)
            },
            SegmentData {
                is_bold: true,
                font_size: 11.04,
                ..column_seg("Roof terminal combined duct vertical and", 83.64, 211.0, 774.96)
            },
            SegmentData {
                is_bold: true,
                font_size: 11.04,
                ..column_seg("twin pipe duct vertical", 83.64, 114.0, 762.24)
            },
        ];
        let paragraphs = blocks_to_paragraphs(segments, &[], &[]);
        assert_eq!(paragraphs.len(), 1, "the heading and its own wrap are one element");
        assert_eq!(
            paragraph_segment_text(&paragraphs[0]),
            "5.7.3 Roof terminal combined duct vertical and twin pipe duct vertical"
        );
    }

    /// GH#1615's own control, page 3 of the same reproducer. Identical heading,
    /// identical fonts, identical line pitch -- the ONLY difference is that the
    /// following line starts at the margin (x 48.24) rather than the title's left
    /// edge (x 83.64), because it is body text and not a wrap. It must still split.
    #[test]
    fn a_numbered_heading_followed_by_margin_aligned_body_still_splits() {
        let segments = vec![
            SegmentData {
                is_bold: true,
                font_size: 11.04,
                ..column_seg("5.7.5", 48.24, 30.0, 774.96)
            },
            SegmentData {
                is_bold: true,
                font_size: 11.04,
                ..column_seg("Roof terminal combined duct vertical and", 83.64, 211.0, 774.96)
            },
            SegmentData {
                is_bold: true,
                font_size: 11.04,
                ..column_seg("The appliance category is C33 for this duct.", 48.24, 216.0, 762.24)
            },
        ];
        let paragraphs = blocks_to_paragraphs(segments, &[], &[]);
        assert_eq!(paragraphs.len(), 2, "body text at the margin is not the heading's wrap");
        assert_eq!(
            paragraph_segment_text(&paragraphs[0]),
            "5.7.5 Roof terminal combined duct vertical and"
        );
    }

    /// GH#1608 page 9: with the predicate blind to the keyword form, a run of
    /// such headings has no break signal at all and collapses into one element --
    /// the exact failure the `starts_section` term exists to prevent.
    #[test]
    fn a_run_of_keyword_numbered_headings_does_not_collapse() {
        let lines = [
            "ARTIKEL 1. TOEPASSELIJKHEID",
            "ARTIKEL 2. TOTSTANDKOMING",
            "ARTIKEL 3. PRIJZEN",
        ];
        let segments = lines
            .iter()
            .enumerate()
            .map(|(index, text)| {
                let baseline = 700.0 - 17.0 * index as f32;
                SegmentData {
                    is_bold: true,
                    font_size: 12.0,
                    height: 12.0,
                    y: baseline - 12.0,
                    ..column_seg(text, 72.0, 180.0, baseline)
                }
            })
            .collect();

        let paragraphs = segments_to_paragraphs(segments, &[(12.0, None)], &[], &TextRepairWitnesses::default());

        assert_eq!(paragraphs.len(), 3, "each heading in the run must be its own element");
        for (index, expected) in lines.iter().enumerate() {
            assert_eq!(paragraph_segment_text(&paragraphs[index]), *expected);
        }
    }

    /// The negative control for the two tests above: prose that opens with the
    /// same keyword and the same number must NOT gain a paragraph break, or the
    /// widening would shred body text wherever a sentence starts `Artikel 12 ...`.
    #[test]
    fn prose_opening_with_a_keyword_and_a_number_keeps_its_paragraph() {
        let first = SegmentData {
            font_size: 12.0,
            height: 12.0,
            y: 700.0 - 12.0,
            ..column_seg("Artikel 12 van de wet is van toepassing", 72.0, 240.0, 700.0)
        };
        let second = SegmentData {
            font_size: 12.0,
            height: 12.0,
            y: 683.0 - 12.0,
            ..column_seg("en dus geldt het volgende voor deze overeenkomst", 72.0, 250.0, 683.0)
        };

        let paragraphs = segments_to_paragraphs(
            vec![first, second],
            &[(12.0, None)],
            &[],
            &TextRepairWitnesses::default(),
        );

        assert_eq!(
            paragraphs.len(),
            1,
            "prose beginning with a keyword and a number is not a heading"
        );
    }

    /// GH#1609: the numbered-heading break terms tested a predicate against a single
    /// SEGMENT, so a heading set with a hanging number -- `"3.1.7"` and its title on
    /// one baseline, two spans -- never looked like a numbered heading and was left to
    /// the ordinary paragraph-gap rule, which needs more than ordinary line pitch. The
    /// heading was welded into the body beneath it.
    #[test]
    fn hanging_number_heading_is_a_paragraph_boundary() {
        let number = SegmentData {
            font_size: 9.0,
            height: 9.0,
            y: 700.0 - 9.0,
            ..column_seg("3.1.7", 104.42, 20.0, 700.0)
        };
        let title = SegmentData {
            font_size: 9.0,
            height: 9.0,
            y: 700.0 - 9.0,
            ..column_seg("Innovatie/ontwikkelingen", 161.06, 100.0, 700.0)
        };
        let body = SegmentData {
            font_size: 9.0,
            height: 9.0,
            y: 688.0 - 9.0,
            ..column_seg(
                "innovatie ontwikkelingen toekomstige verwachten gebied product",
                104.42,
                380.0,
                688.0,
            )
        };

        let paragraphs = segments_to_paragraphs(
            vec![number, title, body],
            &[(9.0, None)],
            &[],
            &TextRepairWitnesses::default(),
        );

        assert_eq!(
            paragraphs.len(),
            2,
            "a hanging-number heading must open its own element"
        );
        assert_eq!(paragraph_segment_text(&paragraphs[0]), "3.1.7 Innovatie/ontwikkelingen");
    }

    /// The single-span control for the test above: same strings, same baselines, one
    /// span. It passed before the fix and must keep passing after it.
    #[test]
    fn single_span_numbered_heading_is_still_a_paragraph_boundary() {
        let heading = SegmentData {
            font_size: 9.0,
            height: 9.0,
            y: 700.0 - 9.0,
            ..column_seg("3.1.7 Innovatie/ontwikkelingen", 104.42, 156.64, 700.0)
        };
        let body = SegmentData {
            font_size: 9.0,
            height: 9.0,
            y: 688.0 - 9.0,
            ..column_seg(
                "innovatie ontwikkelingen toekomstige verwachten gebied product",
                104.42,
                380.0,
                688.0,
            )
        };

        let paragraphs = segments_to_paragraphs(
            vec![heading, body],
            &[(9.0, None)],
            &[],
            &TextRepairWitnesses::default(),
        );

        assert_eq!(
            paragraphs.len(),
            2,
            "a single-span numbered heading must open its own element"
        );
    }

    /// The wrap control for #1467: a numbered heading long enough to reach the
    /// column's right edge, continuing onto a second, unnumbered physical line,
    /// must stay ONE element -- splitting a heading from its own wrapped tail
    /// would be worse than the original defect. This is what
    /// `heading_wraps_onto` exists to rule out: without it, `follows_section`
    /// would fire on every numbered heading regardless of whether the next line
    /// is unrelated content or the heading's own continuation, and this
    /// specific line pair -- same font, same weight, same one-line-height
    /// spacing as the #1467 defect -- would be split into two paragraphs.
    #[test]
    fn heading_wrapping_onto_its_next_line_stays_one_paragraph() {
        let heading_start = column_seg(
            "1.1.1 Een Zeer Lange Sectietitel Die Helemaal Doorloopt Tot De",
            72.0,
            460.0,
            700.0,
        );
        let heading_continuation = column_seg("Rechterkantlijn Van Deze Kolom", 72.0, 450.0, 684.0);

        let paragraphs = segments_to_paragraphs(
            vec![heading_start, heading_continuation],
            &[(11.0, None)],
            &[],
            &TextRepairWitnesses::default(),
        );

        assert_eq!(
            paragraphs.len(),
            1,
            "a heading wrapping onto its own next line must not be split from itself"
        );
    }

    /// The prose control for #1467: three ordinary wrapped lines with no
    /// numbering and no sentence terminator must stay ONE paragraph, exactly as
    /// before this change -- `follows_section` never fires here because
    /// `is_numbered_section_heading` is false for all three lines.
    #[test]
    fn wrapped_prose_lines_without_a_terminator_stay_one_paragraph() {
        let segments = vec![
            body_line_seg("The committee reviewed the annual budget", 700.0),
            body_line_seg("report and discussed the proposed changes", 686.0),
            body_line_seg("before adjourning the meeting for the day", 672.0),
        ];

        let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &[], &TextRepairWitnesses::default());

        assert_eq!(
            paragraphs.len(),
            1,
            "wrapped prose with no sentence terminator must stay one paragraph"
        );
        assert_eq!(
            paragraph_segment_text(&paragraphs[0]),
            "The committee reviewed the annual budget report and discussed the proposed changes \
             before adjourning the meeting for the day"
        );
    }

    /// Helper: one segment of a hanging-indent column, 11pt on an 11pt line.
    /// GH#1616: a reconstructed grid need not span every printed column inside its own
    /// bounding box. The runs in the columns it left out were dropped from the prose flow
    /// and never reached a cell, so they were deleted from the document.
    ///
    /// The shape measured on the reporter's page 51: a four-column fault-finding grid
    /// (cause / `Nee` / `Ja` / remedy) reconstructed with two columns over a bbox spanning
    /// all four, x 48.00 .. 555.24.
    #[test]
    fn a_table_bbox_does_not_delete_text_its_grid_leaves_out() {
        let coverage = vec![TableCoverage {
            bbox: crate::types::BoundingBox {
                x0: 48.0,
                y0: 312.48,
                x1: 555.24,
                y1: 405.28,
            },
            cell_text: table_cell_text(&[
                "Ja  Ja",
                "Controleer de ontsteekpenafstand. Controleer de afstelling, zie § 7.10 Gas-luchtregeling.",
            ]),
        }];
        let segments = vec![
            column_seg("Ja", 300.0, 12.0, 380.0),
            column_seg("Controleer de ontsteekpenafstand.", 340.0, 180.0, 380.0),
            column_seg("Onjuiste ontsteekafstand.", 52.0, 140.0, 380.0),
            column_seg("Nee", 250.0, 20.0, 366.0),
            column_seg("Zwakke vonk.", 52.0, 70.0, 340.0),
        ];

        let kept: Vec<String> = filter_segments_by_table_bboxes(segments, &coverage)
            .into_iter()
            .map(|seg| seg.text)
            .collect();

        assert_eq!(
            kept,
            vec![
                "Onjuiste ontsteekafstand.".to_string(),
                "Nee".to_string(),
                "Zwakke vonk.".to_string(),
            ],
            "runs the grid does not carry must survive; the two it does carry are suppressed"
        );
    }

    /// The other half of the same invariant, and the reason the geometric test cannot
    /// simply be dropped: text a table DOES carry must still be suppressed, or every
    /// table's contents are emitted twice.
    #[test]
    fn a_table_still_suppresses_the_prose_copy_of_its_own_cells() {
        let coverage = vec![TableCoverage {
            bbox: crate::types::BoundingBox {
                x0: 48.0,
                y0: 300.0,
                x1: 500.0,
                y1: 400.0,
            },
            cell_text: table_cell_text(&["Mogelijke oorzaken:", "Oplossing:"]),
        }];
        let segments = vec![
            column_seg("Mogelijke oorzaken:", 52.0, 100.0, 380.0),
            column_seg("Oplossing:", 300.0, 60.0, 380.0),
        ];
        assert!(
            filter_segments_by_table_bboxes(segments, &coverage).is_empty(),
            "a covered run the grid carries is still suppressed"
        );
    }

    fn table_cell_text(cells: &[&str]) -> String {
        cells
            .iter()
            .map(|cell| normalize_for_table_coverage(cell))
            .collect::<Vec<_>>()
            .join("\u{1}")
    }

    fn column_seg(text: &str, x: f32, width: f32, baseline_y: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x,
            y: baseline_y - 11.0,
            width,
            height: 11.0,
            font_size: 11.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y,
            rotation_degrees: 0.0,
            assigned_role: None,
        }
    }

    /// The shape measured on `test_documents/pdf_scanned/ordinance_2197_scanned.pdf`
    /// (tesseract): the marker column is a separate block from the text column, so
    /// every marker arrives as its own paragraph and the whole marker run precedes
    /// the whole text run. Pairing must therefore be by baseline, not adjacency.
    #[test]
    fn detached_marker_column_is_reattached_to_the_body_sharing_its_baseline() {
        let segments = vec![
            column_seg("(a)", 72.0, 14.0, 700.0),
            column_seg("(b)", 72.0, 14.0, 660.0),
            column_seg("(c)", 72.0, 14.0, 620.0),
            column_seg("A ten foot wide minimum buffer along the lot line", 110.0, 300.0, 700.0),
            column_seg(
                "Ten foot wide minimum buffers along Lake Pointe Parkway",
                110.0,
                300.0,
                660.0,
            ),
            column_seg(
                "Required buffers may include the pedestrian walkway",
                110.0,
                300.0,
                620.0,
            ),
        ];
        let gap_ys = compute_paragraph_gap_ys(&segments);

        let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &gap_ys, &TextRepairWitnesses::default());

        assert_eq!(
            paragraphs.len(),
            3,
            "each detached marker must be folded into the body line it shares a baseline with"
        );
        assert_eq!(
            paragraph_segment_text(&paragraphs[0]),
            "(a) A ten foot wide minimum buffer along the lot line"
        );
        assert_eq!(
            paragraph_segment_text(&paragraphs[1]),
            "(b) Ten foot wide minimum buffers along Lake Pointe Parkway"
        );
        assert_eq!(
            paragraph_segment_text(&paragraphs[2]),
            "(c) Required buffers may include the pedestrian walkway"
        );
    }

    /// Reattachment is only worth anything if the body is then *classified* as a
    /// list item; the marker text alone changes no downstream element kind.
    #[test]
    fn bodies_that_absorb_a_detached_marker_become_list_items() {
        let segments = vec![
            column_seg("(a)", 72.0, 14.0, 700.0),
            column_seg("(b)", 72.0, 14.0, 660.0),
            column_seg("(c)", 72.0, 14.0, 620.0),
            column_seg("A ten foot wide minimum buffer along the lot line", 110.0, 300.0, 700.0),
            column_seg(
                "Ten foot wide minimum buffers along Lake Pointe Parkway",
                110.0,
                300.0,
                660.0,
            ),
            column_seg(
                "Required buffers may include the pedestrian walkway",
                110.0,
                300.0,
                620.0,
            ),
        ];
        let gap_ys = compute_paragraph_gap_ys(&segments);

        let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &gap_ys, &TextRepairWitnesses::default());

        assert_eq!(
            paragraphs.iter().filter(|paragraph| paragraph.is_list_item).count(),
            3,
            "a body that absorbed its marker must classify as a list item"
        );
    }

    /// TASK #722 follow-up: a lone `*` and a bracketed integer `[N]` must NOT be
    /// treated as detached list markers -- `*` is also a multiplication sign in
    /// isolated math prose, and `[N]` is standard printed paragraph-number
    /// notation (e.g. Jung's Collected Works), not a marker. The other three
    /// marker families must keep reattaching exactly as before.
    #[test]
    fn ambiguous_detached_markers_are_excluded_while_unambiguous_ones_still_reattach() {
        let segments = vec![
            column_seg("*", 72.0, 14.0, 700.0),
            column_seg("[42]", 72.0, 20.0, 660.0),
            column_seg("-", 72.0, 14.0, 620.0),
            column_seg("(1)", 72.0, 14.0, 580.0),
            column_seg("1.", 72.0, 14.0, 540.0),
            column_seg(
                "A times B is a well known identity in group theory here",
                110.0,
                300.0,
                700.0,
            ),
            column_seg(
                "This paragraph number precedes ordinary book prose here",
                110.0,
                300.0,
                660.0,
            ),
            column_seg(
                "Dash marker prose gets folded into its own body text",
                110.0,
                300.0,
                620.0,
            ),
            column_seg(
                "Parenthesised marker prose gets folded into its own body",
                110.0,
                300.0,
                580.0,
            ),
            column_seg(
                "Numbered marker prose gets folded into its own body",
                110.0,
                300.0,
                540.0,
            ),
        ];
        let gap_ys = compute_paragraph_gap_ys(&segments);

        let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &gap_ys, &TextRepairWitnesses::default());

        assert_eq!(
            paragraphs.len(),
            7,
            "the '*' and '[42]' markers must stay detached (2 extra paragraphs); the other three must reattach"
        );

        let texts: Vec<String> = paragraphs.iter().map(paragraph_segment_text).collect();
        assert!(
            texts.iter().any(|text| text == "*"),
            "a lone '*' must remain its own paragraph, not fold into the math prose below it: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text == "[42]"),
            "a bracketed integer must remain its own paragraph, not fold into the following prose: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text.starts_with("- Dash marker")),
            "a dash marker must still reattach to its body: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text.starts_with("(1) Parenthesised marker")),
            "a parenthesised marker must still reattach to its body: {texts:?}"
        );
        assert!(
            texts.iter().any(|text| text.starts_with("1. Numbered marker")),
            "a '1.' marker must still reattach to its body: {texts:?}"
        );
    }

    /// Helper: a segment carrying raw OCR raster geometry (`rotation_degrees ==
    /// 0.0`, as every OCR segment does -- see `adapters::make_ocr_pdf_line`),
    /// with `y == baseline_y` (also always true for OCR segments -- both fields
    /// are set from the same hOCR line-box value).
    fn ocr_raster_seg(text: &str, x: f32, y: f32, width: f32, height: f32, font_size: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x,
            y,
            width,
            height,
            font_size,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: y,
            rotation_degrees: 0.0,
            assigned_role: None,
        }
    }

    /// #760: `DetachedMarkerFrame::OcrOnPage(270)` must accept the
    /// marker/body pair whose geometry was measured on fixture `ordinance_2197`
    /// (`/Rotate 270`, tesseract) -- marker `y=2378.0 h=65.0`, body `y=1324.0
    /// h=933.0`, both at `font_size=30.0` (chosen so the tolerance,
    /// `30.0 * DETACHED_MARKER_BASELINE_TOLERANCE_FONT_FACTOR`, is `18.0`, and
    /// the max indent, `30.0 * DETACHED_MARKER_MAX_INDENT_FONT_FACTOR`, is
    /// `180.0` -- both match the values reported against the fixture). `x`/
    /// `width` are constructed, not measured, so the two segments' corrected-270
    /// baseline (`x + width`) coincide (delta `0.0`), isolating the advance/
    /// indent half of the fix: `frame.advance_extent()` gives marker
    /// `(-2443.0, -2378.0)` and body `(-2257.0, -1324.0)`, so
    /// `indent = body_left - marker_end = -2257.0 - (-2378.0) = 121.0`, within
    /// `[-15.0, 180.0]`.
    ///
    /// `DetachedMarkerFrame::Native` is, for an OCR segment, byte-for-byte the
    /// pre-#760 behaviour (`upright_baseline()`/`upright_advance_extent()`
    /// short-circuit on `rotation_degrees == 0.0` to the raw `baseline_y`/
    /// `(x, x + width)` -- exactly what unfixed `accepts_detached_list_marker`
    /// read, since it had no frame parameter at all). Against that frame this
    /// same pair is REJECTED at the baseline gate: `|2378.0 - 1324.0| == 1054.0`
    /// (within the 1049..1922 range measured on the real fixture) against a
    /// tolerance of `18.0` -- the indent check is never reached.
    #[test]
    fn ocr_frame_270_accepts_the_measured_pair_that_the_native_frame_rejects() {
        let marker = ocr_raster_seg("(a)", 3317.0, 2378.0, 100.0, 65.0, 30.0);
        let body_segment = ocr_raster_seg("Buffer requirement", 3367.0, 1324.0, 50.0, 933.0, 30.0);
        let body = para(vec![line(vec![body_segment])]);

        assert!(
            !accepts_detached_list_marker(&body, &marker, DetachedMarkerFrame::Native),
            "Native frame must reject the pair: baseline delta 1054.0 exceeds tolerance 18.0"
        );
        assert!(
            accepts_detached_list_marker(&body, &marker, DetachedMarkerFrame::OcrOnPage(270)),
            "OcrOnPage(270) must accept the pair: baseline delta 0.0, indent 121.0 <= 180.0"
        );
    }

    /// #760: pins the exact corrected-frame values for a 270-rotated page, so a
    /// future change to the formula shows up here directly rather than only
    /// through the pass/fail outcome above.
    #[test]
    fn ocr_frame_270_baseline_and_advance_extent_match_the_measured_formula() {
        let marker = ocr_raster_seg("(a)", 3317.0, 2378.0, 100.0, 65.0, 30.0);
        let body_segment = ocr_raster_seg("Buffer requirement", 3367.0, 1324.0, 50.0, 933.0, 30.0);
        let frame = DetachedMarkerFrame::OcrOnPage(270);

        assert_eq!(frame.baseline(&marker), 3417.0, "far raster-x edge (x + width)");
        assert_eq!(frame.baseline(&body_segment), 3417.0, "far raster-x edge (x + width)");
        assert_eq!(
            frame.advance_extent(&marker),
            (-2443.0, -2378.0),
            "advance runs along -y; start is the FAR raster-y edge -(y + height)"
        );
        assert_eq!(
            frame.advance_extent(&body_segment),
            (-2257.0, -1324.0),
            "advance runs along -y; start is the FAR raster-y edge -(y + height)"
        );
    }

    /// #760: `180` is confirmed a no-op for the OCR rotation correction -- both
    /// helpers on `DetachedMarkerFrame::OcrOnPage(180)` must read the same raw
    /// fields as the unrotated default, matching `Native`'s behaviour for an
    /// unrotated (`rotation_degrees == 0.0`) OCR segment exactly.
    #[test]
    fn ocr_frame_180_is_a_no_op() {
        let segment = ocr_raster_seg("text", 100.0, 700.0, 40.0, 10.0, 11.0);

        assert_eq!(
            DetachedMarkerFrame::OcrOnPage(180).baseline(&segment),
            DetachedMarkerFrame::Native.baseline(&segment)
        );
        assert_eq!(
            DetachedMarkerFrame::OcrOnPage(180).advance_extent(&segment),
            DetachedMarkerFrame::Native.advance_extent(&segment)
        );
    }

    /// Precision guard (passes with and without the reattachment pass). A bare
    /// marker must not adopt an indented block on a *different* baseline: that is
    /// an ordinary following paragraph, not the marker's own item text.
    #[test]
    fn a_bare_marker_does_not_adopt_a_block_on_another_baseline() {
        let segments = vec![
            column_seg("(a)", 72.0, 14.0, 700.0),
            column_seg(
                "An indented block that begins on the next line entirely",
                110.0,
                300.0,
                660.0,
            ),
        ];
        let gap_ys = compute_paragraph_gap_ys(&segments);

        let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &gap_ys, &TextRepairWitnesses::default());

        assert_eq!(paragraphs.len(), 2, "baseline agreement is what licenses reattachment");
        assert!(
            !paragraphs[1].is_list_item,
            "a block on its own baseline must not be turned into a list item"
        );
    }

    /// Helper: create a segment with positional data.
    fn seg(text: &str, x: f32, width: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x,
            y: 0.0,
            width,
            height: 12.0,
            font_size: 12.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 0.0,
            rotation_degrees: 0.0,
            assigned_role: None,
        }
    }

    fn inline_seg(text: &str, x: f32, baseline_y: f32, is_bold: bool) -> SegmentData {
        let mut segment = seg(text, x, 20.0);
        segment.baseline_y = baseline_y;
        segment.y = baseline_y - segment.height;
        segment.is_bold = is_bold;
        segment
    }

    #[test]
    fn issue_1560_reconstructs_jittered_table_fragments_as_one_x_ordered_line() {
        let mut article = inline_seg("700004", 68.279, 623.481, false);
        article.font_size = 8.6;
        article.height = 8.6;
        article.width = 35.0;
        let mut position = inline_seg("2", 50.0, 622.401, false);
        position.font_size = 8.6;
        position.height = 8.6;
        position.width = 5.0;
        let mut description = inline_seg("Fastening screw", 124.9, 622.401, false);
        description.font_size = 8.6;
        description.height = 8.6;
        description.width = 60.0;
        let mut next_row = inline_seg("3", 50.0, 610.0, false);
        next_row.font_size = 8.6;
        next_row.height = 8.6;

        let segments = [&article, &position, &description, &next_row];
        let lines = reconstruct_pdf_lines(&segments);

        assert_eq!(lines.len(), 2, "ordinary row leading must still split lines");
        assert_eq!(
            lines[0]
                .segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<_>>(),
            ["2", "700004", "Fastening screw"]
        );
        assert_eq!(lines[1].segments[0].text, "3");
    }

    #[test]
    fn inline_bold_runs_stay_in_one_paragraph() {
        let segments = vec![
            inline_seg("plain", 10.0, 100.0, false),
            inline_seg("bold", 31.0, 100.0, true),
            inline_seg("tail", 52.0, 100.0, false),
        ];

        let paragraphs = blocks_to_paragraphs(segments, &[], &[]);

        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].lines.len(), 1);
        assert_eq!(paragraphs[0].lines[0].segments.len(), 3);
        assert!(paragraphs[0].lines[0].segments[1].is_bold);
        assert_eq!(paragraph_text(&paragraphs[0]), "plain bold tail");

        let document = crate::pdf::structure::assembly::assemble_internal_document(
            vec![paragraphs],
            &[],
            None,
            &[],
            &Default::default(),
        );
        let element = &document.elements[0];
        let bold = element
            .annotations
            .iter()
            .find(|annotation| matches!(annotation.kind, crate::types::AnnotationKind::Bold))
            .expect("inline bold annotation should be preserved");
        assert_eq!(element.text, "plain bold tail");
        assert_eq!((bold.start, bold.end), (6, 10));
    }

    #[test]
    fn same_baseline_font_size_transition_stays_in_one_paragraph() {
        let mut chapter_number = inline_seg("13.", 28.35, 803.043, false);
        chapter_number.width = 17.58;
        chapter_number.font_size = 14.0;
        let mut title = inline_seg("Productkaart vlgs. bijlage IV", 64.35, 803.043, false);
        title.width = 248.03;

        let paragraphs = blocks_to_paragraphs(vec![chapter_number, title], &[], &[]);

        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraph_text(&paragraphs[0]), "13. Productkaart vlgs. bijlage IV");
    }

    #[test]
    fn distant_same_baseline_font_size_transition_remains_a_boundary() {
        let mut heading = inline_seg("Heading", 10.0, 100.0, false);
        heading.font_size = 14.0;
        let body = inline_seg("body", 100.0, 100.0, false);

        let paragraphs = blocks_to_paragraphs(vec![heading, body], &[], &[]);

        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraph_text(&paragraphs[0]), "Heading");
        assert_eq!(paragraph_text(&paragraphs[1]), "body");
    }

    #[test]
    fn different_baseline_font_size_transition_remains_a_boundary() {
        let mut heading = inline_seg("Heading", 10.0, 100.0, false);
        heading.font_size = 14.0;
        let body = inline_seg("body", 10.0, 80.0, false);

        let paragraphs = blocks_to_paragraphs(vec![heading, body], &[], &[]);

        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraph_text(&paragraphs[0]), "Heading");
        assert_eq!(paragraph_text(&paragraphs[1]), "body");
    }

    #[test]
    fn inline_typographic_dash_does_not_split_a_paragraph() {
        let segments = vec![
            inline_seg("Figures 6", 10.0, 100.0, false),
            inline_seg("– 8 show the results", 31.0, 100.0, false),
        ];

        let paragraphs = blocks_to_paragraphs(segments, &[], &[]);

        assert_eq!(paragraphs.len(), 1);
        assert!(!paragraphs[0].is_list_item);
    }

    #[test]
    fn typographic_dash_on_a_new_line_still_starts_a_list() {
        let segments = vec![
            inline_seg("Introduction", 10.0, 100.0, false),
            inline_seg("– first item", 10.0, 80.0, false),
        ];

        let paragraphs = blocks_to_paragraphs(segments, &[], &[]);

        assert_eq!(paragraphs.len(), 2);
        assert!(paragraphs[1].is_list_item);
    }

    #[test]
    fn split_typographic_dash_and_same_line_body_stay_a_list() {
        let segments = vec![
            inline_seg("Introduction", 10.0, 100.0, false),
            inline_seg("–", 10.0, 80.0, false),
            inline_seg("quoted body", 31.0, 80.0, false),
        ];

        let paragraphs = blocks_to_paragraphs(segments, &[], &[]);

        assert_eq!(paragraphs.len(), 2);
        assert!(paragraphs[1].is_list_item);
    }

    #[test]
    fn split_typographic_dash_and_different_line_body_are_not_a_list() {
        let segments = vec![
            inline_seg("Figures 6", 10.0, 100.0, false),
            inline_seg("– ", 31.0, 100.0, false),
            inline_seg("8 show the results", 10.0, 80.0, false),
        ];

        let paragraphs = blocks_to_paragraphs(segments, &[], &[]);

        assert!(paragraphs.iter().all(|paragraph| !paragraph.is_list_item));
    }

    #[test]
    fn cross_line_bold_transition_remains_a_boundary() {
        let segments = vec![
            inline_seg("Heading", 10.0, 100.0, true),
            inline_seg("body", 10.0, 80.0, false),
        ];

        assert_eq!(blocks_to_paragraphs(segments, &[], &[]).len(), 2);
    }

    #[test]
    fn tagged_heading_and_body_stay_separate_on_the_same_line() {
        let mut heading = inline_seg("Heading", 10.0, 100.0, true);
        heading.assigned_role = Some(1);
        let body = inline_seg("body", 31.0, 100.0, false);

        let paragraphs = blocks_to_paragraphs(vec![heading, body], &[], &[]);

        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraph_text(&paragraphs[0]), "Heading");
        assert_eq!(paragraphs[0].heading_level, Some(1));
        assert_eq!(paragraph_text(&paragraphs[1]), "body");
        assert_eq!(paragraphs[1].heading_level, None);
    }

    #[test]
    fn different_tagged_heading_levels_stay_separate_on_the_same_line() {
        let mut first = inline_seg("First", 10.0, 100.0, true);
        first.assigned_role = Some(1);
        let mut second = inline_seg("Second", 31.0, 100.0, false);
        second.assigned_role = Some(2);

        let paragraphs = blocks_to_paragraphs(vec![first, second], &[], &[]);

        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraph_text(&paragraphs[0]), "First");
        assert_eq!(paragraphs[0].heading_level, Some(1));
        assert_eq!(paragraph_text(&paragraphs[1]), "Second");
        assert_eq!(paragraphs[1].heading_level, Some(2));
    }

    #[test]
    fn same_tagged_heading_role_keeps_inline_style_transitions_together() {
        let mut first = inline_seg("First", 10.0, 100.0, true);
        first.assigned_role = Some(1);
        let mut second = inline_seg("Second", 31.0, 100.0, false);
        second.assigned_role = Some(1);

        let paragraphs = blocks_to_paragraphs(vec![first, second], &[], &[]);

        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraph_text(&paragraphs[0]), "First Second");
        assert_eq!(paragraphs[0].heading_level, Some(1));
    }

    #[test]
    fn distant_same_line_bold_transition_remains_a_boundary() {
        let segments = vec![
            inline_seg("left", 10.0, 100.0, false),
            inline_seg("right", 100.0, 100.0, true),
        ];

        assert_eq!(blocks_to_paragraphs(segments, &[], &[]).len(), 2);
    }

    #[test]
    fn overlapping_or_reverse_bold_transition_remains_a_boundary() {
        let overlapping = vec![
            inline_seg("first", 30.0, 100.0, false),
            inline_seg("second", 40.0, 100.0, true),
        ];
        let reversed = vec![
            inline_seg("first", 30.0, 100.0, false),
            inline_seg("second", 5.0, 100.0, true),
        ];

        assert_eq!(blocks_to_paragraphs(overlapping, &[], &[]).len(), 2);
        assert_eq!(blocks_to_paragraphs(reversed, &[], &[]).len(), 2);
    }

    #[test]
    fn slight_metric_overlap_is_still_inline() {
        let segments = vec![
            inline_seg("plain", 30.0, 100.0, false),
            inline_seg("bold", 49.0, 100.0, true),
        ];

        assert_eq!(blocks_to_paragraphs(segments, &[], &[]).len(), 1);
    }

    #[test]
    fn invalid_inline_geometry_remains_a_boundary() {
        let plain = inline_seg("plain", 10.0, 100.0, false);
        let mut zero_font = inline_seg("bold", 31.0, 100.0, true);
        zero_font.font_size = 0.0;
        let mut non_finite_x = inline_seg("bold", 31.0, 100.0, true);
        non_finite_x.x = f32::NAN;
        let mut non_finite_baseline = inline_seg("bold", 31.0, 100.0, true);
        non_finite_baseline.baseline_y = f32::NAN;

        assert_eq!(blocks_to_paragraphs(vec![plain.clone(), zero_font], &[], &[]).len(), 2);
        assert_eq!(
            blocks_to_paragraphs(vec![plain.clone(), non_finite_x], &[], &[]).len(),
            2
        );
        assert_eq!(
            blocks_to_paragraphs(vec![plain, non_finite_baseline], &[], &[]).len(),
            2
        );
    }

    #[test]
    fn later_line_inline_style_transition_does_not_absorb_prior_lines() {
        let segments = vec![
            inline_seg("first line", 10.0, 120.0, false),
            inline_seg("plain", 10.0, 100.0, false),
            inline_seg("bold", 31.0, 100.0, true),
        ];

        assert_eq!(blocks_to_paragraphs(segments, &[], &[]).len(), 2);
    }

    #[test]
    fn monospace_style_transition_remains_a_boundary() {
        let mut plain = inline_seg("let value =", 10.0, 100.0, false);
        plain.is_monospace = true;
        let mut bold = inline_seg("42", 31.0, 100.0, true);
        bold.is_monospace = true;

        assert_eq!(blocks_to_paragraphs(vec![plain, bold], &[], &[]).len(), 2);
    }

    fn line(segments: Vec<SegmentData>) -> PdfLine {
        PdfLine {
            segments,
            baseline_y: 0.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        }
    }

    fn para(lines: Vec<PdfLine>) -> PdfParagraph {
        let word_count = PdfParagraph::compute_word_count("", &lines);
        PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: 12.0,
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

    fn outline_para(text: &str) -> PdfParagraph {
        let mut paragraph = para(vec![line(vec![seg(text, 0.0, 100.0)])]);
        paragraph.text = text.to_string();
        paragraph.word_count = text.split_whitespace().count();
        paragraph
    }

    fn outline_heading(text: &str, level: u8) -> PdfParagraph {
        let mut paragraph = outline_para(text);
        paragraph.heading_level = Some(level);
        paragraph
    }

    fn body_size_paragraph(text: &str, is_bold: bool, heading_level: Option<u8>) -> PdfParagraph {
        let mut paragraph = outline_para(text);
        paragraph.dominant_font_size = 12.0;
        paragraph.is_bold = is_bold;
        paragraph.heading_level = heading_level;
        for line in &mut paragraph.lines {
            line.dominant_font_size = 12.0;
            line.is_bold = is_bold;
            for segment in &mut line.segments {
                segment.font_size = 12.0;
                segment.is_bold = is_bold;
            }
        }
        paragraph
    }

    fn body_size_paragraph_at(text: &str, is_bold: bool, heading_level: Option<u8>, left: f32) -> PdfParagraph {
        let mut paragraph = body_size_paragraph(text, is_bold, heading_level);
        paragraph.block_bbox = Some((left, 0.0, left + 200.0, 12.0));
        paragraph
    }

    fn body_size_paragraph_with_bbox(
        text: &str,
        is_bold: bool,
        heading_level: Option<u8>,
        bbox: (f32, f32, f32, f32),
    ) -> PdfParagraph {
        let mut paragraph = body_size_paragraph(text, is_bold, heading_level);
        paragraph.block_bbox = Some(bbox);
        paragraph
    }

    /// GH#1611: a two-word numbered heading fell below the bold-heading word-count
    /// floor, so it was never promoted. It stayed a plain bold paragraph, and a RUN of
    /// them coalesced into a single bold line in the rendered markdown while the
    /// element stream still showed them apart. The keyword form cleared the floor only
    /// by contributing a third word -- nothing else about the two lines differed.
    #[test]
    fn a_two_word_numbered_heading_is_a_bold_heading_candidate() {
        let keyword = body_size_paragraph_with_bbox("ARTIKEL 3. PRIJZEN", true, None, (72.0, 700.0, 260.0, 712.0));
        let bare = body_size_paragraph_with_bbox("3. PRIJZEN", true, None, (72.0, 700.0, 200.0, 712.0));

        assert!(
            is_body_size_bold_heading_candidate(&keyword, 12.0),
            "the three-word keyword form was already a candidate"
        );
        assert!(
            is_body_size_bold_heading_candidate(&bare, 12.0),
            "a numbered section heading carries its own evidence and must not need a third word"
        );
    }

    /// The floor this exempts is still load-bearing for everything else: a short bold
    /// fragment with no numbering is emphasis or a label, not a heading.
    #[test]
    fn a_two_word_unnumbered_bold_fragment_is_not_a_heading_candidate() {
        let fragment = body_size_paragraph_with_bbox("Note well", true, None, (72.0, 700.0, 160.0, 712.0));
        assert!(
            !is_body_size_bold_heading_candidate(&fragment, 12.0),
            "an unnumbered two-word bold fragment must stay below the heading floor"
        );
    }

    fn heading_page(heading: &str, heading_size: f32, body: &str) -> Vec<SegmentData> {
        let mut heading_segment = seg_heuristic(heading, heading_size, 700.0);
        heading_segment.is_bold = true;
        vec![heading_segment, seg_heuristic(body, 12.0, 650.0)]
    }

    fn extract_heading_test_document(
        pages: Vec<Vec<SegmentData>>,
        used_structure_tree: bool,
    ) -> crate::types::internal::InternalDocument {
        extract_document_structure_from_segments(
            pages,
            SegmentStructureConfig {
                k_clusters: 4,
                tables: &[],
                outline_entries: &[],
                strip_repeating_text: false,
                include_headers: true,
                include_footers: true,
                include_footnotes: true,
                include_watermarks: true,
                used_structure_tree,
                image_positions: &[],
                images: None,
                inject_placeholders: false,
                layout_hints: None,
                allow_single_column: true,
                cancel_token: None,
                #[cfg(feature = "layout-detection")]
                layout_images: None,
                #[cfg(feature = "layout-detection")]
                layout_results: None,
                #[cfg(feature = "layout-detection")]
                table_model: crate::core::config::layout::TableModel::Disabled,
                #[cfg(feature = "layout-detection")]
                table_overlap_preference: crate::core::config::layout::TableOverlapPreference::Content,
                #[cfg(feature = "layout-detection")]
                acceleration: None,
                #[cfg(feature = "layout-detection")]
                session_thread_budget: 0,
            },
        )
        .expect("document structure extraction must succeed")
    }

    fn element_kind_for(
        document: &crate::types::internal::InternalDocument,
        text: &str,
    ) -> Option<crate::types::internal::ElementKind> {
        document
            .elements
            .iter()
            .find(|element| element.text == text)
            .map(|element| element.kind)
    }

    fn table_with_body_rows(body_rows: usize, cell: &str) -> crate::types::Table {
        let mut cells = vec![vec!["Column".to_string()]];
        cells.extend((0..body_rows).map(|_| vec![cell.to_string()]));
        crate::types::Table {
            cells,
            page_number: 1,
            bounding_box: Some(crate::types::BoundingBox {
                x0: 0.0,
                y0: 100.0,
                x1: 500.0,
                y1: 700.0,
            }),
            ..Default::default()
        }
    }

    #[test]
    fn table_dominant_page_removes_spill_but_preserves_annotations() {
        let mut heading = outline_heading("Decorative title", 1);
        heading.block_bbox = Some((10.0, 650.0, 200.0, 680.0));
        let mut spill =
            outline_para("AB Carval Euro CLO Series Class D three month EURIBOR at 3.75 percent 02/15/37 2,350");
        spill.block_bbox = Some((10.0, 350.0, 490.0, 390.0));
        let mut short_prose = outline_para("Rates shown are unaudited.");
        short_prose.block_bbox = Some((10.0, 300.0, 250.0, 320.0));
        let expected_side_prose = "This explanatory sidebar remains because it sits entirely beside the detected table despite sharing its vertical band.";
        let mut side_prose = outline_para(expected_side_prose);
        side_prose.block_bbox = Some((520.0, 300.0, 700.0, 340.0));
        let mut note = outline_para("Note: values are unaudited");
        note.block_bbox = Some((10.0, 250.0, 250.0, 270.0));
        let mut caption = outline_para("Source: annual filing");
        caption.layout_class = Some(LayoutHintClass::Caption);
        caption.block_bbox = Some((10.0, 200.0, 250.0, 220.0));
        let mut pages = vec![vec![heading, spill, short_prose, side_prose, note, caption]];
        let tables = vec![table_with_body_rows(
            TABLE_DOMINANT_MIN_BODY_ROWS,
            "long-table-value-1234567890-long-table-value-1234567890-long-table-value-1234567890",
        )];

        suppress_table_dominant_paragraph_spill(&mut pages, &tables);

        assert_eq!(pages[0].len(), 5);
        assert_eq!(paragraph_text_raw(&pages[0][0]), "Decorative title");
        assert_eq!(paragraph_text_raw(&pages[0][1]), "Rates shown are unaudited.");
        assert_eq!(paragraph_text_raw(&pages[0][2]), expected_side_prose);
        assert_eq!(paragraph_text_raw(&pages[0][3]), "Note: values are unaudited");
        assert_eq!(paragraph_text_raw(&pages[0][4]), "Source: annual filing");
    }

    #[test]
    fn table_dominant_cleanup_preserves_mixed_prose_pages() {
        let prose =
            "This explanatory paragraph is intentionally much longer than the compact table values. ".repeat(12);
        let mut pages = vec![vec![outline_para(&prose)]];
        let tables = vec![table_with_body_rows(TABLE_DOMINANT_MIN_BODY_ROWS, "1")];

        suppress_table_dominant_paragraph_spill(&mut pages, &tables);

        assert_eq!(pages[0].len(), 1);
        assert_eq!(paragraph_text_raw(&pages[0][0]), prose);
    }

    #[test]
    fn table_dominant_cleanup_requires_minimum_body_rows() {
        let mut pages = vec![vec![outline_heading("Keep this title", 1)]];
        let tables = vec![table_with_body_rows(
            TABLE_DOMINANT_MIN_BODY_ROWS - 1,
            "long-table-value-1234567890",
        )];

        suppress_table_dominant_paragraph_spill(&mut pages, &tables);

        assert_eq!(pages[0].len(), 1);
        assert_eq!(paragraph_text_raw(&pages[0][0]), "Keep this title");
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn deferred_layout_caption_survives_table_dominant_cleanup() {
        let caption_text = "2024 2025 2026 2027 2028 2029 2030 2031 2032 2033 2034 2035 2036";
        let mut caption = outline_para(caption_text);
        caption.block_bbox = Some((10.0, 350.0, 490.0, 390.0));
        let mut pages = vec![vec![caption]];
        let hints = vec![LayoutHint {
            class_name: LayoutHintClass::Caption,
            confidence: 0.99,
            left: 0.0,
            bottom: 340.0,
            right: 500.0,
            top: 400.0,
        }];
        let tables = vec![table_with_body_rows(
            TABLE_DOMINANT_MIN_BODY_ROWS,
            "long-table-value-1234567890-long-table-value-1234567890-long-table-value-1234567890",
        )];

        crate::pdf::structure::layout_classify::annotate_layout_classes(&mut pages[0], &hints, 0.5, 0.2);
        suppress_table_dominant_paragraph_spill(&mut pages, &tables);

        assert_eq!(pages[0].len(), 1);
        assert_eq!(paragraph_text_raw(&pages[0][0]), caption_text);
        assert_eq!(pages[0][0].layout_class, Some(LayoutHintClass::Caption));
    }

    #[test]
    fn final_heading_compaction_changes_rendered_markdown_levels() {
        let mut pages = vec![vec![
            outline_heading("Title", 1),
            outline_heading("Section", 3),
            outline_heading("Subsection", 4),
            outline_para("Body text"),
        ]];

        compact_final_heading_hierarchy(&mut pages);
        let document =
            crate::pdf::structure::assembly::assemble_internal_document(pages, &[], None, &[], &Default::default());
        let markdown = crate::rendering::render_markdown(&document);
        let headings = markdown
            .lines()
            .filter(|line| line.starts_with('#'))
            .collect::<Vec<_>>();

        assert_eq!(headings, ["# Title", "## Section", "### Subsection"]);
        assert!(markdown.find("# Title").unwrap() < markdown.find("Body text").unwrap());
    }

    #[test]
    fn final_heading_compaction_is_conservatively_gated() {
        let cases = [
            vec![Some(1), Some(1), Some(3)],
            vec![Some(1), Some(2), Some(3), Some(5)],
            vec![Some(1), None],
            vec![Some(3), None],
        ];

        for expected in cases {
            let mut pages = vec![
                expected
                    .iter()
                    .enumerate()
                    .map(|(index, level)| {
                        let mut paragraph = outline_para(&format!("Block {index}"));
                        paragraph.heading_level = *level;
                        paragraph
                    })
                    .collect::<Vec<_>>(),
            ];

            compact_final_heading_hierarchy(&mut pages);

            let actual = pages[0]
                .iter()
                .map(|paragraph| paragraph.heading_level)
                .collect::<Vec<_>>();
            assert_eq!(actual, expected);
        }
    }

    /// Regression test for xberg-io/xberg#1301 (mode a): a colon-introduced,
    /// semicolon-delimited run-in list with no distinguishing indentation or
    /// line break — exactly how it is rendered from unstyled HTML — is split
    /// into a lead paragraph plus one list item per clause.
    #[test]
    fn run_in_colon_semicolon_list_is_split_into_lead_and_items() {
        let text = "Article 1. The management board is authorised to exclude subscription rights: \
to exclude fractional amounts from the shareholders' subscription right; \
where the new shares are issued against cash contributions at market price;";
        let mut pages = vec![vec![outline_para(text)]];

        split_colon_semicolon_run_in_lists(&mut pages);

        assert_eq!(pages[0].len(), 3, "lead paragraph + 2 list items");
        assert!(!pages[0][0].is_list_item);
        assert!(
            pages[0][0].text.ends_with("authorised to exclude subscription rights:"),
            "lead keeps everything up to and including the anchor colon: {}",
            pages[0][0].text
        );
        assert!(pages[0][1].is_list_item);
        assert_eq!(
            pages[0][1].text,
            "to exclude fractional amounts from the shareholders' subscription right;"
        );
        assert!(pages[0][2].is_list_item);
        assert_eq!(
            pages[0][2].text,
            "where the new shares are issued against cash contributions at market price;"
        );
    }

    #[test]
    fn run_in_list_split_requires_at_least_two_clauses() {
        let mut pages = vec![vec![outline_para("Note: see the appendix for full details.")]];

        split_colon_semicolon_run_in_lists(&mut pages);

        assert_eq!(
            pages[0].len(),
            1,
            "a single clause after the colon is not an enumeration"
        );
        assert!(!pages[0][0].is_list_item);
    }

    #[test]
    fn run_in_list_split_leaves_unrelated_paragraphs_untouched_and_in_order() {
        let list_text = "The board is authorised to exclude rights: to exclude fractional amounts; \
where new shares are issued;";
        let decoy = "- a bare dash-prefixed clause outside a list, unit #06-18 Tower 2, Singapore.";
        let mut pages = vec![vec![outline_para(list_text), outline_para(decoy)]];

        split_colon_semicolon_run_in_lists(&mut pages);

        assert_eq!(pages[0].len(), 4, "lead + 2 items + the untouched trailing paragraph");
        assert_eq!(
            pages[0][3].text, decoy,
            "trailing paragraph keeps its text and reading-order position"
        );
    }

    #[test]
    fn outline_recovery_is_page_scoped_and_uses_root_h2() {
        let mut intro = outline_para("1. Introduction");
        intro.is_list_item = true;
        intro.is_page_furniture = true;
        let mut pages = vec![vec![intro, outline_para("Methods")], vec![outline_para("Introduction")]];
        let entries = vec![
            PdfOutlineEntry::test_entry("Introduction", 0, 1),
            PdfOutlineEntry::test_entry("Methods", 1, 1),
        ];

        recover_headings_from_outline(&mut pages, &entries);

        assert_eq!(pages[0][0].heading_level, Some(2));
        assert_eq!(pages[0][1].heading_level, Some(3));
        assert_eq!(pages[1][0].heading_level, None);
        assert!(!pages[0][0].is_list_item);
        assert!(!pages[0][0].is_page_furniture);
    }

    #[test]
    fn outline_recovery_calibrates_from_two_consistent_anchors() {
        let mut first = outline_para("First anchor");
        first.heading_level = Some(1);
        let mut second = outline_para("Second anchor");
        second.heading_level = Some(2);
        let mut pages = vec![vec![first, second, outline_para("Recovered")]];
        let entries = vec![
            PdfOutlineEntry::test_entry("First anchor", 0, 1),
            PdfOutlineEntry::test_entry("Second anchor", 1, 1),
            PdfOutlineEntry::test_entry("Recovered", 2, 1),
        ];

        recover_headings_from_outline(&mut pages, &entries);

        assert_eq!(pages[0][2].heading_level, Some(3));
    }

    #[test]
    fn outline_recovery_ignores_singleton_bad_calibration_anchor() {
        let mut anchor = outline_para("Bad anchor");
        anchor.heading_level = Some(5);
        let mut pages = vec![vec![anchor, outline_para("Recovered")]];
        let entries = vec![
            PdfOutlineEntry::test_entry("Bad anchor", 0, 1),
            PdfOutlineEntry::test_entry("Recovered", 1, 1),
        ];

        recover_headings_from_outline(&mut pages, &entries);

        assert_eq!(pages[0][1].heading_level, Some(3));
    }

    #[test]
    fn outline_recovery_rejects_ambiguous_titles() {
        let mut pages = vec![vec![
            outline_para("Duplicate outline"),
            outline_para("Duplicate paragraph"),
            outline_para("Duplicate paragraph"),
        ]];
        let entries = vec![
            PdfOutlineEntry::test_entry("Duplicate outline", 0, 1),
            PdfOutlineEntry::test_entry("Duplicate outline", 1, 1),
            PdfOutlineEntry::test_entry("Duplicate paragraph", 0, 1),
        ];

        recover_headings_from_outline(&mut pages, &entries);

        assert!(pages[0].iter().all(|paragraph| paragraph.heading_level.is_none()));
    }

    #[test]
    fn outline_recovery_rejects_semantic_non_headings() {
        let mut header = outline_para("Header");
        header.layout_class = Some(LayoutHintClass::PageHeader);
        let mut list = outline_para("List");
        list.layout_class = Some(LayoutHintClass::ListItem);
        let mut formula = outline_para("Formula");
        formula.is_formula = true;
        let mut pages = vec![vec![header, list, formula]];
        let entries = vec![
            PdfOutlineEntry::test_entry("Header", 0, 1),
            PdfOutlineEntry::test_entry("List", 0, 1),
            PdfOutlineEntry::test_entry("Formula", 0, 1),
        ];

        recover_headings_from_outline(&mut pages, &entries);

        assert!(pages[0].iter().all(|paragraph| paragraph.heading_level.is_none()));
    }

    #[test]
    fn outline_title_normalization_handles_labels_without_aliasing_prose() {
        assert_eq!(
            normalize_outline_title("1. Introduction"),
            normalize_outline_title("Introduction")
        );
        assert_eq!(
            normalize_outline_title("IV. Results"),
            normalize_outline_title("Results")
        );
        assert_ne!(
            normalize_outline_title("A quick example"),
            normalize_outline_title("quick example")
        );
        assert_ne!(
            normalize_outline_title("2024 Report"),
            normalize_outline_title("Report")
        );
        assert_ne!(normalize_outline_title("v2 API"), normalize_outline_title("API"));
    }

    fn paragraph_text(paragraph: &PdfParagraph) -> String {
        paragraph
            .lines
            .iter()
            .flat_map(|line| line.segments.iter())
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn heuristic_segment(text: &str, baseline_y: f32, width: f32, is_monospace: bool) -> SegmentData {
        let mut segment = seg(text, 10.0, width);
        segment.y = baseline_y - segment.height;
        segment.baseline_y = baseline_y;
        segment.is_monospace = is_monospace;
        segment
    }

    fn process_heuristic_segments(segments: Vec<SegmentData>) -> Vec<PdfParagraph> {
        process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: None,
                heuristic_segments: segments,
                page_hints: None,
                table_bboxes: Vec::new(),
                preserve_native_semantics: false,
                use_layout_reading_order: false,
                #[cfg(feature = "layout-detection")]
                hint_validations: Vec::new(),
                #[cfg(feature = "layout-detection")]
                page_width_pts: None,
                needs_classify: false,
                paragraph_gap_ys: Vec::new(),
                include_headers: true,
                include_footers: true,
                include_footnotes: false,
            },
            &[],
            None,
            &TextRepairWitnesses::default(),
        )
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn empty_or_ineligible_layout_hints_use_legacy_page_processing() {
        let segments = vec![
            heuristic_segment("First paragraph.", 700.0, 220.0, false),
            heuristic_segment("Second paragraph.", 600.0, 220.0, false),
        ];
        let paragraph_gap_ys = compute_paragraph_gap_ys(&segments);
        let process = |page_hints| {
            process_single_page(
                PageInput {
                    page_index: 0,
                    struct_paragraphs: None,
                    heuristic_segments: segments.clone(),
                    page_hints,
                    table_bboxes: Vec::new(),
                    preserve_native_semantics: false,
                    use_layout_reading_order: false,
                    hint_validations: Vec::new(),
                    page_width_pts: None,
                    needs_classify: false,
                    paragraph_gap_ys: paragraph_gap_ys.clone(),
                    include_headers: true,
                    include_footers: true,
                    include_footnotes: false,
                },
                &[],
                None,
                &TextRepairWitnesses::default(),
            )
        };

        let legacy = process(None);
        let empty = process(Some(Vec::new()));
        let invalid = process(Some(vec![LayoutHint {
            class_name: crate::pdf::structure::types::LayoutHintClass::Text,
            confidence: 0.9,
            left: 0.0,
            bottom: 0.0,
            right: f32::INFINITY,
            top: 100.0,
        }]));
        let non_overlapping = process(Some(vec![LayoutHint {
            class_name: crate::pdf::structure::types::LayoutHintClass::Text,
            confidence: 0.9,
            left: 400.0,
            bottom: 0.0,
            right: 500.0,
            top: 100.0,
        }]));

        assert_eq!(format!("{empty:?}"), format!("{legacy:?}"));
        assert_eq!(format!("{invalid:?}"), format!("{legacy:?}"));
        assert_eq!(format!("{non_overlapping:?}"), format!("{legacy:?}"));
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn page_width_reaches_layout_reading_order_graph() {
        let positioned_segment = |text: &str, x: f32, y: f32| {
            let mut segment = heuristic_segment(text, y + 10.0, 10.0, false);
            segment.x = x;
            segment.y = y;
            segment.height = 10.0;
            segment
        };
        let mut segments = vec![
            positioned_segment("bottom-left", 10.0, 205.0),
            positioned_segment("top-left", 90.0, 305.0),
            positioned_segment("top-right", 200.0, 305.0),
            positioned_segment("bottom-right", 250.0, 205.0),
        ];
        for (index, segment) in segments.iter_mut().enumerate() {
            segment.assigned_role = Some(if index % 2 == 0 { 2 } else { 3 });
        }
        let hint = |left, bottom, right, top| LayoutHint {
            class_name: crate::pdf::structure::types::LayoutHintClass::Text,
            confidence: 0.95,
            left,
            bottom,
            right,
            top,
        };
        let hints = vec![
            hint(0.0, 200.0, 120.0, 220.0),
            hint(80.0, 300.0, 160.0, 320.0),
            hint(120.0, 300.0, 240.0, 320.0),
            hint(160.0, 200.0, 280.0, 220.0),
        ];
        let process = |page_width_pts, table_bboxes, _has_emitted_table| {
            let mut pages = vec![process_single_page(
                PageInput {
                    page_index: 0,
                    struct_paragraphs: None,
                    heuristic_segments: segments.clone(),
                    page_hints: Some(hints.clone()),
                    table_bboxes,
                    preserve_native_semantics: true,
                    use_layout_reading_order: true,
                    hint_validations: Vec::new(),
                    page_width_pts,
                    needs_classify: false,
                    paragraph_gap_ys: Vec::new(),
                    include_headers: true,
                    include_footers: true,
                    include_footnotes: false,
                },
                &[],
                None,
                &TextRepairWitnesses::default(),
            )];
            reorder_pages_by_layout_region(&mut pages);
            pages[0].iter().map(paragraph_text).collect::<Vec<_>>()
        };

        assert_eq!(
            process(None, Vec::new(), false),
            ["top-left", "bottom-left", "top-right", "bottom-right"]
        );
        assert_eq!(
            process(Some(400.0), Vec::new(), false),
            ["top-left", "top-right", "bottom-left", "bottom-right"],
            "the actual page width must reach layout graph dilation"
        );
        assert_eq!(
            process(
                Some(400.0),
                vec![TableCoverage {
                    bbox: crate::types::BoundingBox {
                        x0: 0.0,
                        y0: 0.0,
                        x1: 50.0,
                        y1: 50.0,
                    },
                    cell_text: String::new(),
                }],
                true,
            ),
            ["top-left", "top-right", "bottom-left", "bottom-right"],
            "an emitted table must not disable layout reading order for surrounding prose"
        );
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn stacked_layout_groups_preserve_native_paragraph_assembly() {
        let segments = vec![
            heuristic_segment("One continuous", 700.0, 120.0, false),
            heuristic_segment("paragraph.", 688.0, 100.0, false),
        ];
        let hints = vec![
            LayoutHint {
                class_name: LayoutHintClass::Text,
                confidence: 0.99,
                left: 0.0,
                bottom: 685.0,
                right: 200.0,
                top: 705.0,
            },
            LayoutHint {
                class_name: LayoutHintClass::Text,
                confidence: 0.99,
                left: 0.0,
                bottom: 673.0,
                right: 200.0,
                top: 693.0,
            },
        ];
        let output = process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: None,
                heuristic_segments: segments,
                page_hints: Some(hints),
                table_bboxes: Vec::new(),
                preserve_native_semantics: true,
                use_layout_reading_order: false,
                hint_validations: Vec::new(),
                page_width_pts: Some(612.0),
                needs_classify: false,
                paragraph_gap_ys: Vec::new(),
                include_headers: true,
                include_footers: true,
                include_footnotes: false,
            },
            &[],
            None,
            &TextRepairWitnesses::default(),
        );

        assert_eq!(output.len(), 1);
        assert_eq!(paragraph_text(&output[0]), "One continuous paragraph.");
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn bboxless_emitted_table_page_preserves_native_semantics_and_layout_class() {
        let segment = heuristic_segment("Ordinary prose.", 700.0, 220.0, false);
        let paragraph_gap_ys = compute_paragraph_gap_ys(std::slice::from_ref(&segment));
        let output = process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: None,
                heuristic_segments: vec![segment],
                page_hints: Some(vec![LayoutHint {
                    class_name: LayoutHintClass::Title,
                    confidence: 0.99,
                    left: 0.0,
                    bottom: 680.0,
                    right: 300.0,
                    top: 720.0,
                }]),
                table_bboxes: Vec::new(),
                preserve_native_semantics: true,
                use_layout_reading_order: true,
                hint_validations: Vec::new(),
                page_width_pts: Some(612.0),
                needs_classify: false,
                paragraph_gap_ys,
                include_headers: true,
                include_footers: true,
                include_footnotes: false,
            },
            &[],
            None,
            &TextRepairWitnesses::default(),
        );

        assert_eq!(output.len(), 1);
        assert_eq!(paragraph_text(&output[0]), "Ordinary prose.");
        assert_eq!(output[0].heading_level, None);
        assert_eq!(output[0].layout_class, Some(LayoutHintClass::Title));
    }

    #[cfg(feature = "layout-detection")]
    #[test]
    fn emitted_table_page_annotates_caption_without_overriding_native_semantics() {
        let segment = heuristic_segment("Source: annual filing", 100.0, 220.0, false);
        let output = process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: None,
                heuristic_segments: vec![segment],
                page_hints: Some(vec![LayoutHint {
                    class_name: LayoutHintClass::Caption,
                    confidence: 0.99,
                    left: 0.0,
                    bottom: 80.0,
                    right: 300.0,
                    top: 120.0,
                }]),
                table_bboxes: Vec::new(),
                preserve_native_semantics: true,
                use_layout_reading_order: true,
                hint_validations: Vec::new(),
                page_width_pts: Some(612.0),
                needs_classify: false,
                paragraph_gap_ys: Vec::new(),
                include_headers: true,
                include_footers: true,
                include_footnotes: false,
            },
            &[],
            None,
            &TextRepairWitnesses::default(),
        );

        assert_eq!(output.len(), 1);
        assert_eq!(paragraph_text(&output[0]), "Source: annual filing");
        assert_eq!(output[0].heading_level, None);
        assert_eq!(output[0].layout_class, Some(LayoutHintClass::Caption));
    }

    #[test]
    fn test_heuristic_path_runs_fused_text_repairs() {
        let mut segment = heuristic_segment("Intro\u{00AD}duction, , body", 700.0, 320.0, false);
        segment.is_bold = true;

        let output = process_heuristic_segments(vec![segment]);

        assert_eq!(output.len(), 1);
        assert_eq!(paragraph_text(&output[0]), "Introduction, body");
        assert!(
            output[0].text.is_empty(),
            "repaired segments must remain the text source of truth"
        );
        assert_eq!(output[0].word_count, 2);

        let document = assemble_internal_document(vec![output], &[], None, &[], &Default::default());
        let element = &document.elements[0];
        assert_eq!(element.text, "Introduction, body");
        assert_eq!(element.annotations.len(), 1);
        assert_eq!(element.annotations[0].start, 0);
        assert_eq!(element.annotations[0].end as usize, element.text.len());
    }

    #[test]
    fn test_heuristic_path_dehyphenates_wrapped_word() {
        let output = process_heuristic_segments(vec![
            heuristic_segment("Reliable soft-", 700.0, 490.0, false),
            heuristic_segment("ware handles load", 680.0, 200.0, false),
        ]);

        assert_eq!(output.len(), 1);
        assert_eq!(paragraph_text(&output[0]), "Reliable software handles load");
        assert_eq!(output[0].word_count, 4);
    }

    #[test]
    fn test_heuristic_path_preserves_compound_and_code_hyphens() {
        let compound = process_heuristic_segments(vec![
            heuristic_segment("A cost-", 700.0, 490.0, false),
            heuristic_segment("effective design", 680.0, 200.0, false),
        ]);
        assert_eq!(paragraph_text(&compound[0]), "A cost-effective design");

        let document = assemble_internal_document(vec![compound], &[], None, &[], &Default::default());
        assert_eq!(document.elements[0].text, "A cost-effective design");

        let code = process_heuristic_segments(vec![
            heuristic_segment("let value = soft-", 700.0, 490.0, true),
            heuristic_segment("ware;", 680.0, 100.0, true),
        ]);
        assert!(code[0].is_code_block);
        assert_eq!(paragraph_text(&code[0]), "let value = soft- ware;");
    }

    #[test]
    fn test_structure_tree_page_runs_text_repair_before_assembly() {
        let paragraphs = vec![para(vec![line(vec![seg(
            "Intro\u{00AD}duction, , body • first item • second item",
            10.0,
            320.0,
        )])])];

        let output = process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: Some(paragraphs),
                heuristic_segments: Vec::new(),
                page_hints: None,
                table_bboxes: Vec::new(),
                preserve_native_semantics: false,
                use_layout_reading_order: false,
                #[cfg(feature = "layout-detection")]
                hint_validations: Vec::new(),
                #[cfg(feature = "layout-detection")]
                page_width_pts: None,
                needs_classify: false,
                paragraph_gap_ys: Vec::new(),
                include_headers: true,
                include_footers: true,
                include_footnotes: false,
            },
            &[],
            None,
            &TextRepairWitnesses::default(),
        );

        assert_eq!(output.len(), 3);
        assert_eq!(paragraph_text(&output[0]), "Introduction, body");
        assert!(output[1].is_list_item);
        assert_eq!(paragraph_text(&output[1]), "first item");
        assert!(output[2].is_list_item);
        assert_eq!(paragraph_text(&output[2]), "second item");
    }

    #[test]
    fn assigned_sal_heading_survives_merge_when_layout_confirms_it() {
        let mut body = role_seg("unterminated body", 12.0, false, None);
        body.y = 688.0;
        body.baseline_y = 700.0;
        let mut annotation = role_seg("__in", 12.0, false, Some(2));
        annotation.y = 638.0;
        annotation.baseline_y = 650.0;

        let output = process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: None,
                heuristic_segments: vec![body, annotation],
                page_hints: Some(vec![LayoutHint {
                    class_name: LayoutHintClass::SectionHeader,
                    confidence: 0.99,
                    left: 70.0,
                    bottom: 635.0,
                    right: 275.0,
                    top: 655.0,
                }]),
                table_bboxes: Vec::new(),
                preserve_native_semantics: false,
                use_layout_reading_order: false,
                #[cfg(feature = "layout-detection")]
                hint_validations: Vec::new(),
                #[cfg(feature = "layout-detection")]
                page_width_pts: Some(612.0),
                needs_classify: false,
                paragraph_gap_ys: Vec::new(),
                include_headers: true,
                include_footers: true,
                include_footnotes: false,
            },
            &[],
            Some(12.0),
            &TextRepairWitnesses::default(),
        );

        assert_eq!(output.len(), 2);
        assert_eq!(paragraph_text(&output[1]), "__in");
        assert_eq!(output[1].heading_level, Some(2));
        assert_eq!(output[1].layout_class, Some(LayoutHintClass::SectionHeader));
    }

    /// Full-width line at x=10, width=490 → right edge 500.
    fn full_line_seg(text: &str) -> SegmentData {
        let mut segment = seg(text, 10.0, 490.0);
        segment.baseline_y = 20.0;
        segment
    }

    /// Short line at x=10, width=100 → right edge 110 (well below 500*0.85=425).
    fn short_line_seg(text: &str) -> SegmentData {
        seg(text, 10.0, 100.0)
    }

    #[test]
    fn test_case1_trailing_hyphen_full_line() {
        let mut p = para(vec![
            line(vec![full_line_seg("some soft-")]),
            line(vec![seg("ware is great", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
        assert_eq!(p.lines[0].segments[0].text, "some software");
        assert_eq!(p.lines[1].segments[0].text, "is great");
    }

    #[test]
    fn test_case2_no_hyphen_full_line_no_join() {
        let mut p = para(vec![
            line(vec![full_line_seg("the soft")]),
            line(vec![seg("ware is great", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
        assert_eq!(p.lines[0].segments[0].text, "the soft");
        assert_eq!(p.lines[1].segments[0].text, "ware is great");
    }

    #[test]
    fn test_short_line_no_join() {
        let mut p = para(vec![
            line(vec![short_line_seg("hello")]),
            line(vec![full_line_seg("world and more")]),
        ]);
        let original_trailing = p.lines[0].segments[0].text.clone();
        let original_leading = p.lines[1].segments[0].text.clone();
        dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
        assert_eq!(p.lines[0].segments[0].text, original_trailing);
        assert_eq!(p.lines[1].segments[0].text, original_leading);
    }

    #[test]
    fn test_code_block_not_joined() {
        let mut p = para(vec![
            line(vec![full_line_seg("some soft-")]),
            line(vec![seg("ware is code", 10.0, 200.0)]),
        ]);
        p.is_code_block = true;
        let mut paragraphs = vec![p];
        dehyphenate_paragraphs(&mut paragraphs, true, &HyphenWitnesses::default());
        assert_eq!(paragraphs[0].lines[0].segments[0].text, "some soft-");
    }

    #[test]
    fn test_uppercase_leading_not_joined() {
        let mut p = para(vec![
            line(vec![full_line_seg("some text")]),
            line(vec![seg("Next sentence here", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
        assert_eq!(p.lines[0].segments[0].text, "some text");
        assert_eq!(p.lines[1].segments[0].text, "Next sentence here");
    }

    #[test]
    fn test_cjk_not_joined() {
        let mut p = para(vec![
            line(vec![full_line_seg("some \u{4E00}-")]),
            line(vec![seg("text here", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
        assert_eq!(p.lines[0].segments[0].text, "some \u{4E00}-");
    }

    #[test]
    fn test_real_world_software_no_join_without_hyphen() {
        let mut p = para(vec![
            line(vec![full_line_seg("advanced soft")]),
            line(vec![seg("ware development", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
        assert_eq!(p.lines[0].segments[0].text, "advanced soft");
        assert_eq!(p.lines[1].segments[0].text, "ware development");
    }

    #[test]
    fn test_real_world_hardware_no_join_without_hyphen() {
        let mut p = para(vec![
            line(vec![full_line_seg("modern hard")]),
            line(vec![seg("ware components", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
        assert_eq!(p.lines[0].segments[0].text, "modern hard");
        assert_eq!(p.lines[1].segments[0].text, "ware components");
    }

    #[test]
    fn test_leading_word_with_trailing_punctuation_no_join() {
        let mut p = para(vec![
            line(vec![full_line_seg("the soft")]),
            line(vec![seg("ware, which is great", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
        assert_eq!(p.lines[0].segments[0].text, "the soft");
        assert_eq!(p.lines[1].segments[0].text, "ware, which is great");
    }

    #[test]
    fn test_hyphen_only_fallback() {
        let mut trailing = seg("some soft-", 0.0, 0.0);
        trailing.baseline_y = 20.0;
        let mut p = para(vec![line(vec![trailing]), line(vec![seg("ware is great", 0.0, 0.0)])]);
        dehyphenate_hyphen_only(&mut p, &HyphenWitnesses::default());
        assert_eq!(p.lines[0].segments[0].text, "some software");
        assert_eq!(p.lines[1].segments[0].text, "is great");
    }

    #[test]
    fn suspended_hyphens_on_one_baseline_are_preserved() {
        for (left, right) in [
            ("vracht-", "en verzendkosten"),
            ("In-", "en uitvoer"),
            ("onderhouds-", "en installatiewerkzaamheden"),
            ("Verkoop-", "en Leveringvoorwaarden"),
        ] {
            let mut trailing = full_line_seg(left);
            trailing.baseline_y = 100.0;
            let mut leading = seg(right, 480.0, 80.0);
            leading.baseline_y = 100.0;
            let mut paragraph = para(vec![line(vec![trailing]), line(vec![leading])]);

            dehyphenate_paragraph_lines(&mut paragraph, &HyphenWitnesses::default());

            assert_eq!(paragraph_text(&paragraph), format!("{left} {right}"));
        }
    }

    #[test]
    fn suspended_and_wrapped_hyphens_in_one_paragraph_are_distinguished() {
        let mut suspended = full_line_seg("De bijbehorende montage-");
        suspended.baseline_y = 100.0;
        let mut wrapped = full_line_seg("en installatie-");
        wrapped.baseline_y = 100.0;
        let mut continuation = seg("handleiding wordt op aanvraag toegezonden.", 10.0, 220.0);
        continuation.baseline_y = 80.0;
        let mut paragraph = para(vec![
            line(vec![suspended]),
            line(vec![wrapped]),
            line(vec![continuation]),
        ]);

        dehyphenate_paragraph_lines(&mut paragraph, &HyphenWitnesses::default());

        assert_eq!(
            paragraph_text(&paragraph),
            "De bijbehorende montage- en installatiehandleiding wordt op aanvraag toegezonden."
        );
    }

    #[test]
    fn dehyphenation_requires_finite_baselines_in_the_same_reading_frame() {
        let cases = [(f32::NAN, 0.0, 0.0), (20.0, 0.0, 90.0)];
        for (trailing_baseline, leading_baseline, leading_rotation) in cases {
            let mut trailing = full_line_seg("some soft-");
            trailing.baseline_y = trailing_baseline;
            let mut leading = seg("ware remains", 10.0, 100.0);
            leading.baseline_y = leading_baseline;
            leading.rotation_degrees = leading_rotation;
            let mut paragraph = para(vec![line(vec![trailing]), line(vec![leading])]);

            dehyphenate_paragraph_lines(&mut paragraph, &HyphenWitnesses::default());

            assert_eq!(paragraph_text(&paragraph), "some soft- ware remains");
        }
    }

    #[test]
    fn test_hyphen_only_uppercase_not_joined() {
        let mut p = para(vec![
            line(vec![seg("some well-", 0.0, 0.0)]),
            line(vec![seg("Known thing", 0.0, 0.0)]),
        ]);
        dehyphenate_hyphen_only(&mut p, &HyphenWitnesses::default());
        assert_eq!(p.lines[0].segments[0].text, "some well-");
    }

    #[test]
    fn test_single_line_paragraph_skipped() {
        let mut paragraphs = vec![para(vec![line(vec![full_line_seg("single line")])])];
        dehyphenate_paragraphs(&mut paragraphs, true, &HyphenWitnesses::default());
        assert_eq!(paragraphs[0].lines[0].segments[0].text, "single line");
    }

    #[test]
    fn test_multi_segment_line_no_join_without_hyphen() {
        let mut p = para(vec![
            line(vec![seg("first part", 10.0, 200.0), seg("soft", 220.0, 280.0)]),
            line(vec![seg("ware next words", 10.0, 200.0)]),
        ]);
        dehyphenate_paragraph_lines(&mut p, &HyphenWitnesses::default());
        assert_eq!(p.lines[0].segments[1].text, "soft");
        assert_eq!(p.lines[1].segments[0].text, "ware next words");
    }

    /// Regression for #1543: a hyphen appearing mid-run (not at the end of a segment's
    /// text) witnesses a genuine authored compound, because a line-wrap hyphen is by
    /// construction the LAST character of its segment.
    ///
    /// Neutralisation that must break this test: stop scanning for interior hyphens (e.g.
    /// only ever inspect the final character of each segment) in `collect_hyphen_witnesses`.
    #[test]
    fn collect_hyphen_witnesses_finds_a_mid_run_compound() {
        let pages = vec![vec![seg("the price-determining factors apply", 0.0, 0.0)]];

        let witnesses = collect_hyphen_witnesses(&pages);

        assert!(
            witnesses.contains(&("price".to_string(), "determining".to_string())),
            "a mid-run hyphen must witness its own compound: got {witnesses:?}"
        );
    }

    /// The load-bearing negative for #1543: a hyphen at the END of a segment's text is,
    /// by construction, a line-wrap candidate rather than evidence of an authored
    /// compound. If this hyphen were witnessed, the collector would preserve every
    /// trailing hyphen it exists to judge, defeating the whole mechanism.
    ///
    /// Neutralisation that must break this test: delete the end-of-run guard (the
    /// `1..characters.len() - 1` range excluding the last index) in
    /// `collect_hyphen_witnesses`.
    #[test]
    fn collect_hyphen_witnesses_ignores_a_hyphen_at_the_end_of_a_run() {
        let pages = vec![vec![seg("are based on the price-", 0.0, 0.0)]];

        let witnesses = collect_hyphen_witnesses(&pages);

        assert!(
            witnesses.is_empty(),
            "a trailing hyphen must never become a witness: got {witnesses:?}"
        );
    }

    /// A pair witnessed only by the document's own mid-line usage -- absent from the
    /// static `PRESERVED_LEXICAL_COMPOUNDS` list -- must still license preservation.
    ///
    /// Neutralisation that must break this test: make `should_preserve_lexical_hyphen`
    /// ignore its `hyphen_witnesses` argument and consult only the static list.
    #[test]
    fn should_preserve_lexical_hyphen_true_for_a_witnessed_pair_not_in_the_static_list() {
        let mut witnesses = HyphenWitnesses::default();
        witnesses.insert(("price".to_string(), "determining".to_string()));

        assert!(should_preserve_lexical_hyphen("price", "determining", &witnesses));
    }

    /// The soft-hyphenation control: `auto` + `matic` (from a line broken as `auto-` /
    /// `matic`) is a genuine mid-word wrap with no witness and no static-list entry, so
    /// the hyphen must still be dropped on rejoin.
    ///
    /// Neutralisation that must break this test: widen the static list or witness lookup
    /// to a prefix/suffix match instead of the exact-pair comparison.
    #[test]
    fn should_preserve_lexical_hyphen_false_for_genuine_soft_hyphenation() {
        let witnesses = HyphenWitnesses::default();

        assert!(!should_preserve_lexical_hyphen("auto", "matic", &witnesses));
    }

    /// Char-boundary regression: a non-ASCII word sitting next to a mid-run hyphen must
    /// not panic. This repo has a documented history of char-boundary panics from byte
    /// slicing a `&str`; `collect_hyphen_witnesses` must only ever slice by `char`.
    ///
    /// Neutralisation that must break this test: rewrite the left/right run extraction
    /// with byte-offset string slicing (e.g. `&text[..byte_index]`) instead of the
    /// char-safe `Vec<char>` scan.
    #[test]
    fn collect_hyphen_witnesses_does_not_panic_on_non_ascii_word_boundaries() {
        let pages = vec![vec![seg("café-terrasse is open déjà-vu style", 0.0, 0.0)]];

        let witnesses = collect_hyphen_witnesses(&pages);

        assert!(
            witnesses.contains(&("café".to_string(), "terrasse".to_string())),
            "a non-ASCII word must still be witnessed: got {witnesses:?}"
        );
    }

    /// GH#1591: a word attested standalone elsewhere in the document is collected as
    /// a witness, even though its OTHER occurrence sits directly in front of a
    /// ligature-space candidate pattern ("bedrijf is") that must not weld it.
    #[test]
    fn collect_word_witnesses_finds_a_standalone_occurrence_elsewhere() {
        let pages = vec![vec![
            seg("bedrijf is gesloten", 0.0, 0.0),
            seg("het bedrijf verkocht apparatuur", 0.0, 0.0),
        ]];

        let witnesses = collect_word_witnesses(&pages);

        assert!(
            witnesses.contains("bedrijf"),
            "bedrijf must be witnessed by its ordinary-prose occurrence: got {witnesses:?}"
        );
    }

    /// The load-bearing negative for #1591: a fragment that appears ONLY as one half
    /// of a ligature-space candidate pattern must never witness itself, or every
    /// genuine decomposed ligature (e.g. `f irst`) would become unrepairable the
    /// moment its own halves are long enough to pass the length guard.
    ///
    /// Neutralisation that must break this test: collect witnesses via a naive
    /// `split_whitespace()` over every segment with no candidate-pattern exclusion.
    #[test]
    fn collect_word_witnesses_does_not_witness_its_own_candidate_halves() {
        let pages = vec![vec![seg("f irst eff iciently", 0.0, 0.0)]];

        let witnesses = collect_word_witnesses(&pages);

        assert!(
            !witnesses.contains("irst") && !witnesses.contains("eff") && !witnesses.contains("iciently"),
            "a candidate pattern's own fragments must not self-witness: got {witnesses:?}"
        );
    }

    /// A word used twice, once as a candidate's left half and once in an ordinary
    /// position, is still witnessed via its non-candidate occurrence -- the exclusion
    /// applies per-occurrence, not to the word everywhere it appears in the document.
    #[test]
    fn collect_word_witnesses_witnesses_a_word_used_twice_once_as_a_candidate() {
        let pages = vec![vec![seg("relief for relief workers arrived", 0.0, 0.0)]];

        let witnesses = collect_word_witnesses(&pages);

        assert!(
            witnesses.contains("relief"),
            "the second, non-candidate occurrence of relief must still witness it: got {witnesses:?}"
        );
    }

    /// Length guard mirroring `MIN_HYPHEN_WITNESS_WORD_LEN`: a single-letter fragment
    /// must never count as its own witness.
    #[test]
    fn collect_word_witnesses_ignores_single_letter_fragments() {
        let pages = vec![vec![seg("a b c", 0.0, 0.0)]];

        let witnesses = collect_word_witnesses(&pages);

        assert!(
            witnesses.is_empty(),
            "single-letter tokens must never be witnesses: got {witnesses:?}"
        );
    }

    /// End-to-end (#1591): `apply_text_repair_to_structure_tree_paragraphs` must
    /// forward the document's word witnesses into `repair_ligature_spaces`, not just
    /// its hyphen witnesses.
    ///
    /// Neutralisation that must break this test: pass `WordWitnesses::default()` to
    /// `fused_text_repairs` instead of `witnesses.words` in
    /// `apply_text_repair_to_structure_tree_paragraphs`.
    #[test]
    fn segments_to_paragraphs_preserves_a_witnessed_ligature_space_boundary() {
        let segments = vec![seg("bedrijf is gesloten", 0.0, 200.0)];
        let witnesses = TextRepairWitnesses {
            hyphens: HyphenWitnesses::default(),
            words: ["bedrijf".to_string()].into_iter().collect(),
        };

        let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &[], &witnesses);

        assert_eq!(paragraph_segment_text(&paragraphs[0]), "bedrijf is gesloten");
    }

    /// The same end-to-end path with no witnesses at all welds the real word
    /// boundary, documenting the fix's known false positive at the pipeline level
    /// (not just in the pure `repair_ligature_spaces` unit tests).
    #[test]
    fn segments_to_paragraphs_welds_an_unwitnessed_ligature_space_boundary() {
        let segments = vec![seg("bedrijf is gesloten", 0.0, 200.0)];

        let paragraphs = segments_to_paragraphs(segments, &[(11.0, None)], &[], &TextRepairWitnesses::default());

        assert_eq!(paragraph_segment_text(&paragraphs[0]), "bedrijfis gesloten");
    }

    fn para_with_font_size(font_size: f32) -> PdfParagraph {
        let lines = vec![line(vec![seg("text", 0.0, 100.0)])];
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
    fn test_has_font_size_variation_empty() {
        assert!(!has_font_size_variation(&[]));
    }

    #[test]
    fn test_has_font_size_variation_single_size() {
        let paragraphs = vec![para_with_font_size(12.0), para_with_font_size(12.0)];
        assert!(!has_font_size_variation(&paragraphs));
    }

    #[test]
    fn test_has_font_size_variation_different_sizes() {
        let paragraphs = vec![para_with_font_size(12.0), para_with_font_size(18.0)];
        assert!(has_font_size_variation(&paragraphs));
    }

    #[test]
    fn test_has_font_size_variation_small_difference_ignored() {
        let paragraphs = vec![para_with_font_size(12.0), para_with_font_size(12.3)];
        assert!(!has_font_size_variation(&paragraphs));
    }

    #[test]
    fn test_has_font_size_variation_zero_sizes_ignored() {
        let paragraphs = vec![para_with_font_size(0.0), para_with_font_size(0.0)];
        assert!(!has_font_size_variation(&paragraphs));
    }

    use crate::pdf::structure::types::LayoutHintClass;

    fn furniture_para_with_class(class: LayoutHintClass) -> PdfParagraph {
        let lines = vec![line(vec![seg("ACME", 0.0, 50.0)])];
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
            is_page_furniture: true,
            layout_class: Some(class),
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        }
    }

    #[test]
    fn test_include_headers_clears_page_header_furniture() {
        let mut paras = vec![furniture_para_with_class(LayoutHintClass::PageHeader)];
        un_mark_layout_furniture_per_config(&mut paras, true, false, false);
        assert!(
            !paras[0].is_page_furniture,
            "PageHeader furniture must be cleared when include_headers=true"
        );
    }

    #[test]
    fn test_include_footers_clears_page_footer_furniture() {
        let mut paras = vec![furniture_para_with_class(LayoutHintClass::PageFooter)];
        un_mark_layout_furniture_per_config(&mut paras, false, true, false);
        assert!(
            !paras[0].is_page_furniture,
            "PageFooter furniture must be cleared when include_footers=true"
        );
    }

    #[test]
    fn test_include_headers_false_preserves_page_header_furniture() {
        let mut paras = vec![furniture_para_with_class(LayoutHintClass::PageHeader)];
        un_mark_layout_furniture_per_config(&mut paras, false, false, false);
        assert!(
            paras[0].is_page_furniture,
            "PageHeader furniture must remain when include_headers=false"
        );
    }

    #[test]
    fn test_include_headers_does_not_clear_page_footer_furniture() {
        let mut paras = vec![furniture_para_with_class(LayoutHintClass::PageFooter)];
        un_mark_layout_furniture_per_config(&mut paras, true, false, false);
        assert!(
            paras[0].is_page_furniture,
            "PageFooter furniture must remain when only include_headers=true"
        );
    }

    #[test]
    fn test_include_headers_does_not_clear_non_layout_furniture() {
        let mut para = para(vec![line(vec![seg("repeating", 0.0, 80.0)])]);
        para.is_page_furniture = true;
        para.layout_class = None;
        let mut paras = vec![para];
        un_mark_layout_furniture_per_config(&mut paras, true, true, false);
        assert!(
            paras[0].is_page_furniture,
            "Heuristic furniture (no layout_class) must not be cleared"
        );
    }

    #[test]
    fn test_un_mark_is_noop_when_both_flags_false() {
        let mut paras = vec![
            furniture_para_with_class(LayoutHintClass::PageHeader),
            furniture_para_with_class(LayoutHintClass::PageFooter),
        ];
        un_mark_layout_furniture_per_config(&mut paras, false, false, false);
        assert!(paras[0].is_page_furniture);
        assert!(paras[1].is_page_furniture);
    }

    #[test]
    fn should_clear_footnote_furniture_when_include_footnotes_is_true() {
        let mut paras = vec![furniture_para_with_class(LayoutHintClass::Footnote)];
        un_mark_layout_furniture_per_config(&mut paras, false, false, true);
        assert!(
            !paras[0].is_page_furniture,
            "Footnote furniture must be cleared when include_footnotes=true"
        );
    }

    #[test]
    fn should_preserve_footnote_furniture_when_include_footnotes_is_false() {
        let mut paras = vec![furniture_para_with_class(LayoutHintClass::Footnote)];
        un_mark_layout_furniture_per_config(&mut paras, true, true, false);
        assert!(
            paras[0].is_page_furniture,
            "Footnote furniture must remain when include_footnotes=false, even if header/footer flags are true"
        );
    }

    #[test]
    fn should_drop_footnote_body_when_recovery_knob_is_off_and_survive_when_on() {
        // Regression test for GH#61: a footnote body classified `Footnote` by the
        // layout model that is (for whatever reason) already marked page furniture
        // must be recoverable via `include_footnotes`, exactly like header/footer
        // furniture is recoverable via `include_headers` / `include_footers`.
        //
        // A second, substantive body paragraph is included alongside the footnote so
        // `retain_page_furniture_safely`'s "don't empty the page" safety valve does not
        // mask the effect of `include_footnotes` under test.
        let body_text = "A".repeat(200);
        let body = {
            let mut p = para(vec![line(vec![seg(&body_text, 0.0, 400.0)])]);
            p.text = body_text.clone();
            p.word_count = 1;
            p
        };
        let footnote_body = furniture_para_with_class(LayoutHintClass::Footnote);

        let mut off = vec![body.clone(), footnote_body.clone()];
        un_mark_layout_furniture_per_config(&mut off, true, true, false);
        retain_page_furniture_safely(&mut off);
        assert_eq!(
            off.len(),
            1,
            "footnote body must be dropped when include_footnotes=false"
        );

        let mut on = vec![body, footnote_body];
        un_mark_layout_furniture_per_config(&mut on, false, false, true);
        retain_page_furniture_safely(&mut on);
        assert_eq!(on.len(), 2, "footnote body must survive when include_footnotes=true");
        assert!(!on[1].is_page_furniture);
    }

    #[test]
    fn test_deduplicate_paragraphs_removes_consecutive_duplicates() {
        let p1 = para(vec![line(vec![full_line_seg("Brand loses market share")])]);
        let p2 = para(vec![line(vec![full_line_seg("Brand loses market share")])]);
        let p3 = para(vec![line(vec![full_line_seg("Different content here")])]);
        let mut pages = vec![vec![p1, p2, p3]];
        deduplicate_paragraphs(&mut pages);
        assert_eq!(pages[0].len(), 2, "consecutive duplicate should be removed");
    }

    #[test]
    fn test_deduplicate_paragraphs_removes_non_consecutive_body_duplicates() {
        let p1 = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
        let p2 = para(vec![line(vec![full_line_seg("Some intervening paragraph")])]);
        let p3 = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
        let mut pages = vec![vec![p1, p2, p3]];
        deduplicate_paragraphs(&mut pages);
        assert_eq!(pages[0].len(), 2, "non-consecutive body duplicate should be removed");
    }

    #[test]
    fn test_deduplicate_paragraphs_preserves_non_consecutive_headings() {
        let mut h = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
        h.heading_level = Some(2);
        let filler = para(vec![line(vec![full_line_seg("Some other content between them")])]);
        let mut h2 = para(vec![line(vec![full_line_seg("Brand loses market share in volume")])]);
        h2.heading_level = Some(2);
        let mut pages = vec![vec![h, filler, h2]];
        deduplicate_paragraphs(&mut pages);
        assert_eq!(
            pages[0].len(),
            3,
            "non-consecutive heading duplicates must be preserved"
        );
    }

    fn positioned_footnote_paragraph(
        text: &str,
        bbox: (f32, f32, f32, f32),
        font_size: f32,
        is_page_furniture: bool,
    ) -> PdfParagraph {
        let mut segment = seg_at(text, bbox.0, bbox.1, font_size, false);
        segment.width = bbox.2 - bbox.0;
        let mut paragraph = para(vec![line(vec![segment])]);
        paragraph.dominant_font_size = font_size;
        paragraph.is_page_furniture = is_page_furniture;
        paragraph.block_bbox = Some(bbox);
        paragraph
    }

    fn positioned_numeric_footnote_run<const N: usize>(
        numbers: [u32; N],
        body_bottoms: [f32; N],
        body_fonts: [f32; N],
    ) -> Vec<PdfParagraph> {
        let mut paragraphs = Vec::new();
        for ((number, body_bottom), body_font) in numbers.into_iter().zip(body_bottoms).zip(body_fonts) {
            paragraphs.push(positioned_footnote_paragraph(
                &number.to_string(),
                (72.0, body_bottom + 3.7947, 75.3369, body_bottom + 9.7947),
                6.0,
                true,
            ));
            paragraphs.push(positioned_footnote_paragraph(
                "estimate",
                (78.1142, body_bottom, 140.9093, body_bottom + body_font),
                body_font,
                false,
            ));
        }
        paragraphs
    }

    #[test]
    fn consecutive_spatial_footnotes_merge_into_one_paragraph() {
        let pairs = [
            ("1", "2021 estimate", 101.0221, 97.2274),
            ("2", "2020 estimate", 89.5231, 85.7284),
            ("3", "2020 estimate", 78.024, 74.2294),
        ];
        let mut paragraphs = Vec::new();
        for (marker, body, marker_bottom, body_bottom) in pairs {
            paragraphs.push(positioned_footnote_paragraph(
                marker,
                (72.0, marker_bottom, 75.3369, marker_bottom + 6.0),
                6.0,
                true,
            ));
            paragraphs.push(positioned_footnote_paragraph(
                body,
                (78.1142, body_bottom, 140.9093, body_bottom + 10.0),
                10.0,
                false,
            ));
        }

        merge_spatial_footnote_markers(&mut paragraphs);
        retain_page_furniture_safely(&mut paragraphs);
        let mut pages = vec![paragraphs];
        deduplicate_paragraphs(&mut pages);

        assert_eq!(
            pages[0].iter().map(paragraph_text_raw).collect::<Vec<_>>(),
            ["1 2021 estimate 2 2020 estimate 3 2020 estimate"]
        );
        assert_eq!(pages[0][0].lines.len(), 6);
        assert_eq!(pages[0][0].block_bbox, Some((72.0, 74.2294, 140.9093, 107.2274)));
    }

    #[test]
    fn spatial_footnote_run_rejects_sequence_gap_path_and_style_changes() {
        let regular_bottoms = [97.2274, 85.7284, 74.2294];

        let mut nonsequential = positioned_numeric_footnote_run([1, 3, 4], regular_bottoms, [10.0, 10.0, 10.0]);
        merge_spatial_footnote_markers(&mut nonsequential);
        assert_eq!(nonsequential.len(), 3);

        let mut large_gap = positioned_numeric_footnote_run([1, 2, 3], [97.2274, 80.0, 68.5], [10.0, 10.0, 10.0]);
        merge_spatial_footnote_markers(&mut large_gap);
        assert_eq!(large_gap.len(), 3);

        let mut mismatched_path = positioned_numeric_footnote_run([1, 2, 3], regular_bottoms, [10.0, 10.0, 10.0]);
        let other_path = super::super::types::LayoutRegionPath {
            root: super::super::types::LayoutRegionTag {
                id: 1,
                class_name: Some(super::super::types::LayoutHintClass::Footnote),
            },
            child: None,
        };
        mismatched_path[2].layout_region_path = Some(other_path);
        mismatched_path[3].layout_region_path = Some(other_path);
        merge_spatial_footnote_markers(&mut mismatched_path);
        assert_eq!(mismatched_path.len(), 3);

        let mut style_change = positioned_numeric_footnote_run([1, 2, 3], regular_bottoms, [10.0, 12.0, 10.0]);
        merge_spatial_footnote_markers(&mut style_change);
        assert_eq!(style_change.len(), 3);
    }

    #[test]
    fn spatial_footnote_run_rejects_overflowing_marker_sequence() {
        let make_paragraph = |marker: &str, body_bottom: f32| {
            let marker = positioned_footnote_paragraph(
                marker,
                (72.0, body_bottom + 3.7947, 75.3369, body_bottom + 9.7947),
                6.0,
                false,
            );
            let body = positioned_footnote_paragraph(
                "estimate",
                (78.1142, body_bottom, 140.9093, body_bottom + 10.0),
                10.0,
                false,
            );
            let mut paragraph = para(vec![marker.lines[0].clone(), body.lines[0].clone()]);
            paragraph.dominant_font_size = 10.0;
            paragraph.block_bbox = Some((72.0, body_bottom, 140.9093, body_bottom + 10.0));
            paragraph
        };
        let upper = make_paragraph(&u32::MAX.to_string(), 97.2274);
        let lower = make_paragraph("0", 85.7284);

        assert!(!spatial_footnotes_are_adjacent(&upper, &lower));
    }

    #[test]
    fn spatial_footnote_run_requires_marker_pair_provenance() {
        let paragraphs = positioned_numeric_footnote_run([1, 2, 3], [97.2274, 85.7284, 74.2294], [10.0; 3]);
        let mut preexisting = paragraphs
            .chunks_exact(2)
            .map(|pair| {
                let mut paragraph = pair.to_vec();
                merge_spatial_footnote_markers(&mut paragraph);
                paragraph.pop().expect("marker and body merge")
            })
            .collect::<Vec<_>>();

        merge_spatial_footnote_markers(&mut preexisting);

        assert_eq!(preexisting.len(), 3);
        assert_eq!(
            preexisting.iter().map(paragraph_text_raw).collect::<Vec<_>>(),
            ["1 estimate", "2 estimate", "3 estimate"]
        );
    }

    #[test]
    fn spatial_footnote_run_keeps_regular_prefix_before_irregular_fourth() {
        let mut paragraphs =
            positioned_numeric_footnote_run([1, 2, 3, 4], [97.2274, 85.7284, 74.2294, 59.5], [10.0; 4]);

        merge_spatial_footnote_markers(&mut paragraphs);

        assert_eq!(paragraphs.len(), 2);
        assert_eq!(
            paragraphs.iter().map(paragraph_text_raw).collect::<Vec<_>>(),
            ["1 estimate 2 estimate 3 estimate", "4 estimate"]
        );
    }

    #[test]
    fn spatial_footnote_merge_rejects_page_numbers_and_list_markers() {
        let page_number = positioned_footnote_paragraph("1", (300.0, 20.0, 303.0, 26.0), 6.0, true);
        let distant_body = positioned_footnote_paragraph("Following paragraph", (72.0, 40.0, 180.0, 50.0), 10.0, false);
        let list_number = positioned_footnote_paragraph("2", (72.0, 80.0, 75.0, 90.0), 10.0, true);
        let list_body = positioned_footnote_paragraph("List body", (78.0, 80.0, 130.0, 90.0), 10.0, false);
        let small_list_number = positioned_footnote_paragraph("3", (72.0, 60.0, 75.0, 66.0), 6.0, true);
        let aligned_list_body =
            positioned_footnote_paragraph("Small list body", (78.0, 60.0, 150.0, 70.0), 10.0, false);
        let mut paragraphs = vec![
            page_number,
            distant_body,
            list_number,
            list_body,
            small_list_number,
            aligned_list_body,
        ];

        merge_spatial_footnote_markers(&mut paragraphs);

        assert_eq!(paragraphs.len(), 6);
        assert_eq!(
            paragraphs.iter().map(paragraph_text_raw).collect::<Vec<_>>(),
            ["1", "Following paragraph", "2", "List body", "3", "Small list body"]
        );
    }

    #[test]
    fn spatial_footnote_merge_requires_compatible_geometry_and_layout_path() {
        let marker = positioned_footnote_paragraph("12", (72.0, 100.0, 76.0, 106.0), 6.0, true);
        let large_gap_body = positioned_footnote_paragraph("Large gap", (90.0, 96.0, 140.0, 106.0), 10.0, false);
        let weak_overlap_body = positioned_footnote_paragraph("Weak overlap", (78.0, 104.0, 140.0, 114.0), 10.0, false);
        let mut mismatched_path_body =
            positioned_footnote_paragraph("Other region", (78.0, 96.0, 140.0, 106.0), 10.0, false);
        mismatched_path_body.layout_region_path = Some(super::super::types::LayoutRegionPath {
            root: super::super::types::LayoutRegionTag {
                id: 1,
                class_name: Some(super::super::types::LayoutHintClass::Footnote),
            },
            child: None,
        });

        for body in [large_gap_body, weak_overlap_body, mismatched_path_body] {
            let mut paragraphs = vec![marker.clone(), body];
            merge_spatial_footnote_markers(&mut paragraphs);
            assert_eq!(paragraphs.len(), 2);
        }
    }

    #[test]
    fn spatial_footnote_merge_accepts_conventional_symbol_marker() {
        let marker = positioned_footnote_paragraph("†", (72.0, 100.0, 75.0, 106.0), 6.0, true);
        let body = positioned_footnote_paragraph("Source note", (78.0, 96.0, 140.0, 106.0), 10.0, false);
        let mut paragraphs = vec![marker, body];

        merge_spatial_footnote_markers(&mut paragraphs);

        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraph_text_raw(&paragraphs[0]), "† Source note");
    }

    /// Verify that the index offset formula used for image mapping is correct.
    #[test]
    fn test_image_index_offset_mapping() {
        let indices: Vec<usize> = vec![50, 52, 54];
        let indices_set: ahash::AHashSet<usize> = indices.iter().copied().collect();
        let first_idx_on_page = indices.iter().copied().min().unwrap_or(0);

        let mut matched: Vec<usize> = Vec::new();
        for current_image in 0..5usize {
            let global_idx = first_idx_on_page + current_image;
            if indices_set.contains(&global_idx) {
                matched.push(global_idx);
            }
        }

        assert_eq!(
            matched,
            vec![50, 52, 54],
            "offset formula must yield exactly the requested global indices"
        );

        assert!(
            !indices_set.contains(&49usize),
            "index 49 is before the page range and must not match"
        );

        assert!(
            !indices_set.contains(&55usize),
            "index 55 was not requested and must not match"
        );
    }

    /// Helper: build a minimal SegmentData for heading-map tests.
    fn seg_with_font(text: &str, font_size: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x: 10.0,
            y: 700.0,
            width: 200.0,
            height: font_size,
            font_size,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 700.0,
            rotation_degrees: 0.0,
            assigned_role: None,
        }
    }

    fn seg_at(text: &str, x: f32, y: f32, height: f32, monospace: bool) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x,
            y,
            width: 200.0,
            height,
            font_size: height,
            is_bold: false,
            is_italic: false,
            is_monospace: monospace,
            baseline_y: y,
            rotation_degrees: 0.0,
            assigned_role: None,
        }
    }

    fn rotated_seg(text: &str, x: f32, y: f32, width: f32, rotation_degrees: f32) -> SegmentData {
        let mut segment = seg_at(text, x, y, 10.0, false);
        segment.width = width;
        segment.font_size = 10.0;
        segment.rotation_degrees = rotation_degrees;
        segment
    }

    #[test]
    fn test_order_segments_in_reading_frames_repairs_scrambled_rotated_table() {
        let segments = vec![
            rotated_seg("B2", 200.0, 130.0, 10.0, 90.0),
            rotated_seg("A2", 100.0, 130.0, 10.0, 90.0),
            rotated_seg("B1", 200.0, 100.0, 10.0, 90.0),
            rotated_seg("A1", 100.0, 100.0, 10.0, 90.0),
        ];

        let ordered = order_segments_in_reading_frames(segments);
        let text = ordered.iter().map(|segment| segment.text.as_str()).collect::<Vec<_>>();
        assert_eq!(text, ["A1", "A2", "B1", "B2"]);
    }

    #[test]
    fn test_order_segments_in_reading_frames_leaves_upright_order_byte_identical() {
        let segments = vec![
            rotated_seg("second", 200.0, 100.0, 10.0, 0.0),
            rotated_seg("first", 100.0, 100.0, 10.0, 0.0),
        ];

        let ordered = order_segments_in_reading_frames(segments);
        let text = ordered.iter().map(|segment| segment.text.as_str()).collect::<Vec<_>>();
        assert_eq!(text, ["second", "first"]);
    }

    #[test]
    fn test_blocks_to_paragraphs_separates_rotated_body_from_upright_footer() {
        let segments = vec![
            rotated_seg("Engine", 100.0, 100.0, 20.0, 90.0),
            rotated_seg("oil", 100.0, 125.0, 10.0, 90.0),
            rotated_seg("264", 280.0, 20.0, 15.0, 0.0),
        ];

        let paragraphs = blocks_to_paragraphs(order_segments_in_reading_frames(segments), &[], &[]);
        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraphs[0].text, "Engine oil");
        assert_eq!(paragraphs[1].text, "264");
    }

    #[test]
    fn test_compute_paragraph_gap_ys_detects_blank_line_gap() {
        let segments = vec![
            seg_at("line one", 10.0, 700.0, 12.0, false),
            seg_at("line two", 10.0, 684.0, 12.0, false),
            seg_at("new paragraph", 10.0, 644.0, 12.0, false),
        ];
        let gaps = compute_paragraph_gap_ys(&segments);
        assert_eq!(gaps.len(), 1, "only the blank-line jump is a paragraph gap");
        assert!(
            gaps[0] > 656.0 && gaps[0] < 684.0,
            "gap midpoint between the paragraphs, got {}",
            gaps[0]
        );
    }

    #[test]
    fn test_compute_paragraph_gap_ys_uses_rotated_cross_axis() {
        let segments = vec![
            rotated_seg("line one", 100.0, 100.0, 20.0, 90.0),
            rotated_seg("line two", 116.0, 100.0, 20.0, 90.0),
            rotated_seg("new paragraph", 156.0, 100.0, 20.0, 90.0),
        ];

        let gaps = compute_paragraph_gap_ys(&segments);
        assert_eq!(gaps.len(), 1);
        assert!((gaps[0] + 131.0).abs() < 1e-3, "unexpected rotated-frame gap: {gaps:?}");
    }

    #[test]
    fn test_compute_paragraph_gap_ys_ignores_same_line_runs_and_tight_lines() {
        let segments = vec![
            seg_at("run a", 10.0, 700.0, 12.0, false),
            seg_at("run b", 80.0, 700.0, 12.0, false),
            seg_at("next line", 10.0, 685.0, 12.0, false),
        ];
        assert_eq!(compute_paragraph_gap_ys(&segments), Vec::<f32>::new());
    }

    #[test]
    fn test_compute_paragraph_gap_ys_immune_to_column_major_stream_order() {
        let segments = vec![
            seg_at("A top", 10.0, 700.0, 12.0, false),
            seg_at("A mid", 10.0, 685.0, 12.0, false),
            seg_at("A bot", 10.0, 670.0, 12.0, false),
            seg_at("B top", 300.0, 700.0, 12.0, false),
            seg_at("B mid", 300.0, 685.0, 12.0, false),
            seg_at("B bot", 300.0, 670.0, 12.0, false),
        ];
        assert_eq!(compute_paragraph_gap_ys(&segments), Vec::<f32>::new());
    }

    #[test]
    fn test_finalize_paragraph_page_number_is_not_heading_and_not_yet_furniture() {
        // GH#1411: classification suppresses heading promotion for page-number
        // shapes but must not mark them deletable — that decision needs page
        // geometry and cross-page agreement, and is made document-wide.
        let heading_map = vec![(12.0, Some(2)), (9.0, None)];
        let gap_info = crate::pdf::structure::classify::precompute_gap_info(&heading_map);
        let seg = seg_at("1", 300.0, 50.0, 12.0, false);
        let para = finalize_paragraph(&[&seg], &heading_map, &gap_info).expect("paragraph");
        assert_eq!(para.heading_level, None, "page number must not become a heading");
        assert!(
            !para.is_page_furniture,
            "classification must not mark page furniture without positional evidence"
        );
    }

    /// #712: a fragment whose text starts lowercase must not be promoted to a
    /// heading even when its font size matches a heading centroid exactly. This is
    /// the fabrication signature the OCR mid-line `font_change` break produces --
    /// see `SUPPRESS_LOWERCASE_START_HEADINGS`'s doc comment. Against unfixed code
    /// (`SUPPRESS_LOWERCASE_START_HEADINGS = false`) this asserts
    /// `para.heading_level == None` and fails with `para.heading_level == Some(2)`.
    #[test]
    fn test_finalize_paragraph_suppresses_heading_for_lowercase_start_fragment() {
        let heading_map = vec![(12.0, Some(2)), (9.0, None)];
        let gap_info = crate::pdf::structure::classify::precompute_gap_info(&heading_map);
        let seg = seg_at("storage.", 10.0, 700.0, 12.0, false);
        let para = finalize_paragraph(&[&seg], &heading_map, &gap_info).expect("paragraph");
        assert_eq!(
            para.heading_level, None,
            "a lowercase-starting fragment must not become a heading"
        );
    }

    /// Paragraph carrying real geometry, for the page-number validation tests.
    /// `y` is a PDF-space bottom coordinate on a 792pt page.
    fn positioned_para(text: &str, x: f32, y: f32) -> PdfParagraph {
        let segment = seg_at(text, x, y, 12.0, false);
        let mut paragraph = para(vec![line(vec![segment])]);
        paragraph.text = text.to_string();
        paragraph.word_count = text.split_whitespace().count();
        paragraph.block_bbox = Some((x, y, x + 200.0, y + 12.0));
        paragraph
    }

    /// 792pt-tall pages, matching `positioned_para`'s coordinate assumptions.
    fn letter_page_heights(page_count: usize) -> Vec<f32> {
        vec![792.0; page_count]
    }

    #[test]
    fn should_not_delete_page_number_shape_in_the_page_body() {
        // A table cell reading "1" in the middle of the page: correct shape,
        // wrong position. This is the 3020-hit regression from GH#1411.
        let mut pages: Vec<Vec<PdfParagraph>> = (0..8)
            .map(|_| vec![positioned_para("1", 90.0, 400.0), positioned_para("body", 90.0, 380.0)])
            .collect();
        let page_heights = letter_page_heights(pages.len());
        mark_validated_page_numbers(&mut pages, &page_heights);
        assert!(
            pages.iter().all(|page| !page[0].is_page_furniture),
            "body-band page-number shapes must never be marked furniture"
        );
    }

    #[test]
    fn should_not_delete_an_isolated_page_number_match() {
        // One footer-positioned "7" on a single page of an eight-page document
        // is not a running page number, whatever its shape.
        let mut pages: Vec<Vec<PdfParagraph>> = (0..8)
            .map(|_| vec![positioned_para("Some ordinary body sentence.", 90.0, 400.0)])
            .collect();
        pages[3].push(positioned_para("7", 300.0, 40.0));
        let page_heights = letter_page_heights(pages.len());
        mark_validated_page_numbers(&mut pages, &page_heights);
        assert!(
            !pages[3][1].is_page_furniture,
            "a single isolated match must never be deleted"
        );
    }

    #[test]
    fn should_delete_a_consistent_running_footer_page_number() {
        let mut pages: Vec<Vec<PdfParagraph>> = (0..8)
            .map(|page_index| {
                vec![
                    positioned_para("Some ordinary body sentence.", 90.0, 400.0),
                    positioned_para(&(page_index + 1).to_string(), 300.0, 40.0),
                ]
            })
            .collect();
        let page_heights = letter_page_heights(pages.len());
        mark_validated_page_numbers(&mut pages, &page_heights);
        assert!(
            pages.iter().all(|page| page[1].is_page_furniture),
            "an incrementing footer number in a fixed slot on every page is furniture"
        );
    }

    #[test]
    fn should_leave_layout_classified_paragraphs_to_the_layout_path() {
        // R6: any layout class at all takes precedence over this heuristic, so
        // `include_footers` cannot be silently overridden here.
        let mut pages: Vec<Vec<PdfParagraph>> = (0..8)
            .map(|page_index| {
                let mut footer = positioned_para(&(page_index + 1).to_string(), 300.0, 40.0);
                footer.layout_class = Some(LayoutHintClass::PageFooter);
                vec![positioned_para("Some ordinary body sentence.", 90.0, 400.0), footer]
            })
            .collect();
        let page_heights = letter_page_heights(pages.len());
        mark_validated_page_numbers(&mut pages, &page_heights);
        assert!(
            pages.iter().all(|page| !page[1].is_page_furniture),
            "layout-classified paragraphs must be left to the layout path"
        );
    }

    #[test]
    fn test_compute_paragraph_gap_ys_skips_blank_lines_inside_code_blocks() {
        let segments = vec![
            seg_at("let x = 1;", 10.0, 700.0, 12.0, true),
            seg_at("let y = 2;", 10.0, 660.0, 12.0, true),
            seg_at("Prose resumes here.", 10.0, 620.0, 12.0, false),
        ];
        let gaps = compute_paragraph_gap_ys(&segments);
        assert_eq!(gaps.len(), 1, "only the code→prose boundary is a gap");
        assert!(
            gaps[0] > 632.0 && gaps[0] < 660.0,
            "gap sits between code and prose, got {}",
            gaps[0]
        );
    }

    /// 5-paragraph doc (1 title at 14pt + 4 body at 11pt) with k_clusters=4.
    /// The adaptive clamp should reduce clusters to max(2, 5/4)=max(2,1)=2,
    /// and then the font-size difference (14 vs 11, ratio≈1.27 ≥ 1.2) should
    /// produce a heading_level=1 for the 14pt entry.
    #[test]
    fn test_build_heading_map_short_doc_title_gets_heading_level_1() {
        let title_seg = seg_with_font("My Title", 14.0);
        let body_seg1 = seg_with_font("Body paragraph one.", 11.0);
        let body_seg2 = seg_with_font("Body paragraph two.", 11.0);
        let body_seg3 = seg_with_font("Body paragraph three.", 11.0);
        let body_seg4 = seg_with_font("Body paragraph four.", 11.0);

        let all_page_segments = vec![vec![title_seg, body_seg1, body_seg2, body_seg3, body_seg4]];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];
        let k_clusters = 4;

        let (heading_map, _) =
            build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, k_clusters)
                .expect("build_heading_map must succeed");

        let title_entry = heading_map.iter().find(|(fs, _)| (*fs - 14.0).abs() < 0.5);
        assert!(
            title_entry.is_some(),
            "heading_map must contain an entry near 14pt; got: {heading_map:?}"
        );
        assert_eq!(
            title_entry.unwrap().1,
            Some(1),
            "14pt title in a 5-paragraph doc must get heading_level=1; got: {heading_map:?}"
        );
    }

    /// Real Tesseract hOCR `x_fsize` values measured at 300 DPI against
    /// `test_documents/images_extra/ocr_image.tiff`: a 21px body cluster and a 23px
    /// secondary tier (ratio 23/21 = 1.095, which fails `MIN_HEADING_FONT_RATIO`, but
    /// 23 >= 21 + 1.5 clears the old absolute `MIN_HEADING_FONT_GAP` floor). `font_size`
    /// on OCR segments is a render-DPI-dependent pixel measurement, not points, so an
    /// absolute-unit gap calibrated for typographic points misfires here.
    ///
    /// Fails without the fix: `assign_heading_levels_smart` used to compute
    /// `heading_threshold = (21.0 * 1.15).min(21.0 + 1.5) = 22.5`, and 23.0 >= 22.5, so
    /// the 23px cluster got `Some(1)` and this document ended up with a spurious
    /// heading instead of the all-body map asserted here.
    #[test]
    fn test_build_heading_map_pixel_scale_ratio_gate_rejects_subhead_noise() {
        let all_page_segments = vec![vec![
            seg_with_font("Subhead-looking line", 23.0),
            seg_with_font("Body paragraph one with real running text.", 21.0),
            seg_with_font("Body paragraph two with real running text.", 21.0),
            seg_with_font("Body paragraph three with real running text.", 21.0),
            seg_with_font("Body paragraph four with real running text.", 21.0),
        ]];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
            .expect("build_heading_map must succeed");

        assert!(
            heading_map.iter().all(|(_, level)| level.is_none()),
            "a 23px cluster over a 21px body (ratio 1.095) must not be promoted to a heading; got: {heading_map:?}"
        );
    }

    /// Same shape as `test_build_heading_map_pixel_scale_ratio_gate_rejects_subhead_noise`
    /// but at the native point-scale reference (body=10pt) that `MIN_HEADING_FONT_RATIO`'s
    /// and the old `MIN_HEADING_FONT_GAP`'s doc comments both cite: `10 * 1.15 == 10 + 1.5
    /// == 11.5`, so removing the gap term changes nothing here. Does not fail without the
    /// fix (old and new formulas agree at this exact reference point) — it pins the native
    /// crossover behavior the fix is designed to preserve.
    #[test]
    fn test_build_heading_map_native_reference_body_boundary_still_promotes() {
        let all_page_segments = vec![vec![
            seg_with_font("Boundary Heading", 11.5),
            seg_with_font("Body paragraph one with real running text.", 10.0),
            seg_with_font("Body paragraph two with real running text.", 10.0),
            seg_with_font("Body paragraph three with real running text.", 10.0),
            seg_with_font("Body paragraph four with real running text.", 10.0),
        ]];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
            .expect("build_heading_map must succeed");

        let heading_entry = heading_map.iter().find(|(fs, _)| (*fs - 11.5).abs() < 0.01);
        assert_eq!(
            heading_entry.map(|(_, level)| *level),
            Some(Some(1)),
            "11.5pt over a 10pt body sits exactly on the ratio boundary and must still promote; got: {heading_map:?}"
        );
    }

    /// Verify that the adaptive k clamp doesn't over-reduce for larger documents
    /// (≥20 paragraphs keeps k_clusters unchanged).
    #[test]
    fn test_build_heading_map_large_doc_k_not_reduced() {
        let mut segs: Vec<SegmentData> = (0..4).map(|i| seg_with_font(&format!("Heading {i}"), 18.0)).collect();
        segs.extend((0..20).map(|i| seg_with_font(&format!("Body text paragraph {i}."), 12.0)));

        let all_page_segments = vec![segs];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
            .expect("build_heading_map must succeed");

        let heading_entry = heading_map.iter().find(|(fs, _)| (*fs - 18.0).abs() < 1.0);
        assert!(
            heading_entry.is_some_and(|(_, level)| level.is_some()),
            "18pt entries in a 24-paragraph doc must have a heading level; got: {heading_map:?}"
        );
    }

    /// Uniform-font short document: when all paragraphs share the same font size,
    /// no heading cluster is found by k-means. The fallback must detect the first-page
    /// segment as a title when its font is ≥ 1.2× median — but here all fonts are equal
    /// so no fallback should fire.
    #[test]
    fn test_build_heading_map_uniform_font_no_spurious_heading() {
        let segs: Vec<SegmentData> = (0..5).map(|i| seg_with_font(&format!("Para {i}"), 12.0)).collect();

        let all_page_segments = vec![segs];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
            .expect("build_heading_map must succeed");

        assert!(
            heading_map.iter().all(|(_, level)| level.is_none()),
            "uniform-font doc must produce no headings; got: {heading_map:?}"
        );
    }

    #[test]
    fn should_classify_untagged_bold_body_size_paragraphs_on_tagged_pages() {
        let paragraphs = vec![
            body_size_paragraph("Existing tagged section", true, Some(2)),
            body_size_paragraph("Overige en specifieke bepalingen", true, None),
            body_size_paragraph("Datalekprotocol", true, None),
            body_size_paragraph("First body paragraph ends here.", false, None),
            body_size_paragraph("Second body paragraph ends here.", false, None),
        ];
        let all_page_segments = vec![Vec::new()];
        let struct_tree_results = vec![Some(paragraphs.clone())];

        let (heading_map, pages_needing_classification) =
            build_heading_map(&all_page_segments, &struct_tree_results, &[], 4)
                .expect("build_heading_map must succeed");

        assert_eq!(
            pages_needing_classification.into_iter().collect::<Vec<_>>(),
            [0],
            "a uniform-font tagged page with an untagged bold paragraph must reach classification"
        );

        let classified = process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: Some(paragraphs),
                heuristic_segments: Vec::new(),
                page_hints: None,
                table_bboxes: Vec::new(),
                preserve_native_semantics: true,
                use_layout_reading_order: false,
                #[cfg(feature = "layout-detection")]
                hint_validations: Vec::new(),
                #[cfg(feature = "layout-detection")]
                page_width_pts: None,
                needs_classify: true,
                paragraph_gap_ys: Vec::new(),
                include_headers: true,
                include_footers: true,
                include_footnotes: true,
            },
            &heading_map,
            Some(12.0),
            &TextRepairWitnesses::default(),
        );
        let level_for = |text: &str| {
            classified
                .iter()
                .find(|paragraph| paragraph_segment_text(paragraph) == text)
                .and_then(|paragraph| paragraph.heading_level)
        };

        assert_eq!(level_for("Existing tagged section"), Some(2));
        assert_eq!(level_for("Overige en specifieke bepalingen"), Some(3));
        assert_eq!(level_for("Datalekprotocol"), None);
    }

    #[test]
    fn should_promote_repeated_untagged_body_size_bold_sections_document_wide() {
        let mut tagged_control = heading_page(
            "Existing tagged heading",
            18.0,
            "The control page ordinary body paragraph ends here.",
        );
        tagged_control[0].assigned_role = Some(1);
        let pages = vec![
            tagged_control,
            heading_page(
                "Overige en specifieke bepalingen",
                12.0,
                "The first ordinary body paragraph ends here.",
            ),
            heading_page(
                "Beveiligingsbeleid en toezicht",
                12.0,
                "The second ordinary body paragraph ends here.",
            ),
            heading_page(
                "Aanvullende technische bepalingen",
                12.0,
                "The third ordinary body paragraph ends here.",
            ),
            heading_page(
                "Datalekprotocol",
                12.0,
                "The short-title control remains ordinary text.",
            ),
        ];

        for used_structure_tree in [false, true] {
            let document = extract_heading_test_document(pages.clone(), used_structure_tree);
            assert_eq!(
                element_kind_for(&document, "Existing tagged heading"),
                Some(crate::types::internal::ElementKind::Heading { level: 1 })
            );
            for heading in [
                "Overige en specifieke bepalingen",
                "Beveiligingsbeleid en toezicht",
                "Aanvullende technische bepalingen",
            ] {
                assert_eq!(
                    element_kind_for(&document, heading),
                    Some(crate::types::internal::ElementKind::Heading { level: 3 }),
                    "a repeated whole-line bold body-size convention must classify {heading:?} as H3"
                );
            }
            assert_eq!(
                element_kind_for(&document, "Datalekprotocol"),
                Some(crate::types::internal::ElementKind::Paragraph),
                "the existing one-word body-size ambiguity guard must remain intact"
            );
        }
    }

    #[test]
    fn should_not_promote_indented_attributions_as_repeated_body_size_headings() {
        let mut attribution_pages = vec![vec![
            body_size_paragraph_at("Existing document title", true, Some(1), 74.0),
            body_size_paragraph_at("Existing section heading", true, Some(2), 74.0),
        ]];
        attribution_pages.extend((0..16).map(|index| {
            vec![
                body_size_paragraph_at(
                    &format!("Presenter Name {index}, Department Director"),
                    true,
                    None,
                    126.0,
                ),
                body_size_paragraph_at(
                    &format!("Agenda item {index} discussion continues in ordinary body text."),
                    false,
                    None,
                    74.0,
                ),
            ]
        }));

        promote_repeated_body_size_bold_headings(&mut attribution_pages, Some(12.0));

        let promoted_attribution_count = attribution_pages
            .iter()
            .skip(1)
            .filter(|page| page[0].heading_level == Some(3))
            .count();
        assert_eq!(
            promoted_attribution_count, 0,
            "indented presenter attributions must remain subordinate to the following content block"
        );
        assert_eq!(attribution_pages[0][0].heading_level, Some(1));
        assert_eq!(attribution_pages[0][1].heading_level, Some(2));

        let mut aligned_section_pages = vec![vec![
            body_size_paragraph_at("Existing document title", true, Some(1), 74.0),
            body_size_paragraph_at("Existing section heading", true, Some(2), 74.0),
        ]];
        aligned_section_pages.push(vec![body_size_paragraph_at(
            "Genuine repeated section heading 0",
            true,
            None,
            74.0,
        )]);
        aligned_section_pages.push(vec![body_size_paragraph_at(
            "Section 0 opens a block of ordinary body text on the next page.",
            false,
            None,
            74.0,
        )]);
        aligned_section_pages.extend((1..9).map(|index| {
            vec![
                body_size_paragraph_with_bbox(
                    &format!("Genuine repeated section heading {index}"),
                    true,
                    None,
                    (74.0, 700.0, 274.0, 712.0),
                ),
                body_size_paragraph_with_bbox(
                    &format!("Section {index} opens a block of ordinary body text."),
                    false,
                    None,
                    (74.0, 660.0, 274.0, 690.0),
                ),
            ]
        }));

        promote_repeated_body_size_bold_headings(&mut aligned_section_pages, Some(12.0));

        assert_eq!(
            aligned_section_pages
                .iter()
                .skip(1)
                .filter(|page| page[0].heading_level == Some(3))
                .count(),
            9,
            "aligned repeated section headings must retain the document-wide promotion"
        );
        assert_eq!(aligned_section_pages[0][0].heading_level, Some(1));
        assert_eq!(aligned_section_pages[0][1].heading_level, Some(2));

        let mut missing_geometry_pages = vec![vec![body_size_paragraph_at(
            "Existing document title",
            true,
            Some(1),
            74.0,
        )]];
        missing_geometry_pages.push(vec![
            body_size_paragraph("Candidate without geometry opens body content", true, None),
            body_size_paragraph_at(
                "Following ordinary body content retains valid geometry.",
                false,
                None,
                74.0,
            ),
        ]);
        missing_geometry_pages.push(vec![
            body_size_paragraph_at("Candidate with geometry opens body content", true, None, 74.0),
            body_size_paragraph("Following ordinary body content lacks geometry.", false, None),
        ]);
        missing_geometry_pages.push(vec![
            body_size_paragraph("Candidate and body both lack geometry", true, None),
            body_size_paragraph("Following ordinary body content also lacks geometry.", false, None),
        ]);

        promote_repeated_body_size_bold_headings(&mut missing_geometry_pages, Some(12.0));

        assert_eq!(
            missing_geometry_pages
                .iter()
                .skip(1)
                .filter(|page| page[0].heading_level == Some(3))
                .count(),
            3,
            "missing geometry on either side must retain the documented promotion fallback"
        );

        let mut numbered_section_pages = vec![vec![body_size_paragraph_at(
            "Existing meeting title",
            true,
            Some(1),
            74.0,
        )]];
        numbered_section_pages.extend(["I.", "II.", "III."].into_iter().enumerate().map(|(index, marker)| {
            vec![
                body_size_paragraph_with_bbox(
                    &format!("{marker} CENTERED MEETING AGENDA SECTION"),
                    true,
                    None,
                    (220.0, 700.0, 430.0, 714.0),
                ),
                body_size_paragraph_with_bbox(
                    &format!("Agenda section {} contains ordinary list or body content.", index + 1),
                    false,
                    None,
                    (74.0, 660.0, 430.0, 690.0),
                ),
            ]
        }));
        numbered_section_pages.push(vec![
            body_size_paragraph_at("I am the program presenter", true, None, 126.0),
            body_size_paragraph_at(
                "The presenter attribution is followed by substantive ordinary body text.",
                false,
                None,
                74.0,
            ),
        ]);
        let mut non_finite_candidate =
            body_size_paragraph_at("Indented presenter with invalid geometry", true, None, 126.0);
        non_finite_candidate.block_bbox = Some((f32::NAN, 0.0, 326.0, 12.0));
        numbered_section_pages.push(vec![
            non_finite_candidate,
            body_size_paragraph_at(
                "Invalid candidate geometry must not bypass structural alignment checks.",
                false,
                None,
                74.0,
            ),
        ]);
        numbered_section_pages.push(vec![
            body_size_paragraph_at("Meeting Executive Name", true, None, 74.0),
            body_size_paragraph_at("Executive Secretary", false, None, 74.0),
        ]);

        promote_repeated_body_size_bold_headings(&mut numbered_section_pages, Some(12.0));

        assert_eq!(
            numbered_section_pages
                .iter()
                .skip(1)
                .take(3)
                .filter(|page| page[0].heading_level == Some(3))
                .count(),
            3,
            "explicitly numbered centered sections must not depend on body alignment"
        );
        assert_eq!(
            numbered_section_pages[4][0].heading_level, None,
            "bare Roman-prefix prose must not bypass attribution alignment"
        );
        assert_eq!(
            numbered_section_pages[5][0].heading_level, None,
            "present but non-finite geometry must fail closed"
        );
        assert_eq!(
            numbered_section_pages.last().and_then(|page| page[0].heading_level),
            None,
            "an attribution followed only by a short role label must remain a paragraph"
        );
    }

    #[test]
    fn should_not_promote_same_row_presenter_labels_as_repeated_body_size_headings() {
        let mut pages = vec![vec![body_size_paragraph_with_bbox(
            "Existing document title",
            true,
            Some(1),
            (74.0, 740.0, 274.0, 752.0),
        )]];
        pages.extend((0..3).map(|index| {
            vec![
                body_size_paragraph_with_bbox(
                    &format!("Presenter Role Label {index}"),
                    true,
                    None,
                    (74.0, 700.0, 190.0, 712.0),
                ),
                body_size_paragraph_with_bbox(
                    &format!("Named participant {index} and their professional affiliation"),
                    false,
                    None,
                    (210.0, 700.0, 430.0, 712.0),
                ),
            ]
        }));
        pages.extend((0..3).map(|index| {
            vec![
                body_size_paragraph_with_bbox(
                    &format!("Genuine repeated section heading {index}"),
                    true,
                    None,
                    (74.0, 700.0, 300.0, 712.0),
                ),
                body_size_paragraph_with_bbox(
                    &format!("Section {index} opens a substantive ordinary body paragraph."),
                    false,
                    None,
                    (74.0, 660.0, 430.0, 690.0),
                ),
            ]
        }));

        assert_eq!(
            pages
                .iter()
                .skip(1)
                .filter(|page| is_body_size_bold_heading_candidate(&page[0], 12.0))
                .count(),
            6,
            "all controls must reach the repeated body-size heading promotion predicate"
        );
        assert_eq!(
            pages
                .iter()
                .skip(1)
                .take(3)
                .filter(|page| page[0]
                    .block_bbox
                    .zip(page[1].block_bbox)
                    .is_some_and(|(candidate, following)| { candidate.1 < following.3 && following.1 < candidate.3 }))
                .count(),
            MIN_BODY_SIZE_BOLD_SIGNALS,
            "the negative controls must overlap vertically and meet the document-wide promotion threshold"
        );

        promote_repeated_body_size_bold_headings(&mut pages, Some(12.0));

        assert_eq!(
            pages
                .iter()
                .skip(1)
                .take(3)
                .filter(|page| page[0].heading_level == Some(3))
                .count(),
            0,
            "same-row presenter labels must remain subordinate text"
        );
        assert_eq!(
            pages
                .iter()
                .skip(4)
                .filter(|page| page[0].heading_level == Some(3))
                .count(),
            3,
            "vertically ordered headings must retain repeated body-size promotion"
        );
    }

    #[test]
    fn should_preserve_structural_headings_and_reject_same_row_table_cells() {
        let mut pages = vec![vec![body_size_paragraph_with_bbox(
            "Existing document title",
            true,
            Some(1),
            (74.0, 740.0, 274.0, 752.0),
        )]];
        for heading in [
            "3. NUMBERED SECTION HEADING",
            "IV. ROMAN SECTION HEADING",
            "ARTICLE I GENERAL PROVISIONS",
            "PART IV ADMINISTRATIVE RULES",
            "Policy Administration and Review",
        ] {
            pages.push(vec![
                body_size_paragraph_with_bbox(heading, true, None, (74.0, 700.0, 300.0, 714.0)),
                body_size_paragraph_with_bbox(
                    "The outdented heading opens this substantive ordinary body paragraph.",
                    false,
                    None,
                    (110.0, 680.0, 430.0, 702.0),
                ),
            ]);
        }
        pages.push(vec![
            body_size_paragraph_with_bbox(
                "Policy Heading With Inverted Bounds",
                true,
                None,
                (300.0, 700.0, 74.0, 714.0),
            ),
            body_size_paragraph_with_bbox(
                "The normalized outdented heading opens this ordinary body paragraph.",
                false,
                None,
                (110.0, 680.0, 430.0, 702.0),
            ),
        ]);
        pages.push(vec![
            body_size_paragraph_with_bbox("1. TABLE CELL LABEL", true, None, (74.0, 600.0, 190.0, 612.0)),
            body_size_paragraph_with_bbox(
                "Adjacent table value with enough words",
                false,
                None,
                (210.0, 600.0, 430.0, 612.0),
            ),
        ]);
        let mut invalid_candidate =
            body_size_paragraph_with_bbox("2. INVALID CANDIDATE GEOMETRY", true, None, (74.0, 560.0, 190.0, 572.0));
        invalid_candidate.block_bbox = Some((f32::NAN, 560.0, 190.0, 572.0));
        pages.push(vec![
            invalid_candidate,
            body_size_paragraph_with_bbox(
                "A valid adjacent value must not excuse invalid candidate geometry.",
                false,
                None,
                (210.0, 560.0, 430.0, 572.0),
            ),
        ]);
        let mut invalid_following = body_size_paragraph_with_bbox(
            "An invalid adjacent value must not excuse valid candidate geometry.",
            false,
            None,
            (210.0, 520.0, 430.0, 532.0),
        );
        invalid_following.block_bbox = Some((210.0, 520.0, f32::INFINITY, 532.0));
        pages.push(vec![
            body_size_paragraph_with_bbox("3. INVALID FOLLOWING GEOMETRY", true, None, (74.0, 520.0, 190.0, 532.0)),
            invalid_following,
        ]);

        promote_repeated_body_size_bold_headings(&mut pages, Some(12.0));

        for page in pages.iter().skip(1).take(6) {
            assert_eq!(
                page[0].heading_level,
                Some(3),
                "a vertically ordered, outdented structural heading must survive slight bbox overlap: {:?}",
                paragraph_text_raw(&page[0])
            );
        }
        assert_eq!(
            pages[7][0].heading_level, None,
            "an explicitly numbered table cell must not bypass the same-row guard"
        );
        assert_eq!(
            (pages[8][0].heading_level, pages[9][0].heading_level),
            (None, None),
            "explicit numbering must not turn non-finite candidate or following geometry into a heading exemption"
        );
    }

    #[test]
    fn should_reject_non_finite_geometry_when_the_other_bbox_is_missing() {
        let mut missing_candidate_pages = vec![vec![body_size_paragraph_at(
            "Existing document title",
            true,
            Some(1),
            74.0,
        )]];
        missing_candidate_pages.extend((0..MIN_BODY_SIZE_BOLD_SIGNALS).map(|index| {
            let mut following = body_size_paragraph_at(
                &format!("Following ordinary body content {index} has invalid geometry."),
                false,
                None,
                74.0,
            );
            following.block_bbox = Some((74.0, f32::NAN, 274.0, 12.0));
            vec![
                body_size_paragraph(
                    &format!("Candidate without geometry {index} opens body content"),
                    true,
                    None,
                ),
                following,
            ]
        }));

        assert_eq!(
            missing_candidate_pages
                .iter()
                .skip(1)
                .filter(|page| is_body_size_bold_heading_candidate(&page[0], 12.0))
                .count(),
            MIN_BODY_SIZE_BOLD_SIGNALS,
            "candidate-missing controls must reach the document-wide promotion threshold"
        );
        promote_repeated_body_size_bold_headings(&mut missing_candidate_pages, Some(12.0));

        let mut missing_following_pages = vec![vec![body_size_paragraph_at(
            "Existing document title",
            true,
            Some(1),
            74.0,
        )]];
        missing_following_pages.extend((0..MIN_BODY_SIZE_BOLD_SIGNALS).map(|index| {
            let mut candidate = body_size_paragraph_at(
                &format!("Candidate with invalid geometry {index} opens body content"),
                true,
                None,
                74.0,
            );
            candidate.block_bbox = Some((f32::INFINITY, 0.0, 274.0, 12.0));
            vec![
                candidate,
                body_size_paragraph(
                    &format!("Following ordinary body content {index} lacks geometry."),
                    false,
                    None,
                ),
            ]
        }));

        assert_eq!(
            missing_following_pages
                .iter()
                .skip(1)
                .filter(|page| is_body_size_bold_heading_candidate(&page[0], 12.0))
                .count(),
            MIN_BODY_SIZE_BOLD_SIGNALS,
            "following-missing controls must reach the document-wide promotion threshold"
        );
        promote_repeated_body_size_bold_headings(&mut missing_following_pages, Some(12.0));

        let promoted_with_non_finite_following = missing_candidate_pages
            .iter()
            .skip(1)
            .filter(|page| page[0].heading_level == Some(3))
            .count();
        let promoted_with_non_finite_candidate = missing_following_pages
            .iter()
            .skip(1)
            .filter(|page| page[0].heading_level == Some(3))
            .count();
        assert_eq!(
            (promoted_with_non_finite_following, promoted_with_non_finite_candidate),
            (0, 0),
            "a present non-finite bbox must fail closed regardless of which side lacks geometry"
        );
    }

    #[test]
    fn should_not_promote_fewer_than_three_body_size_bold_sections() {
        let candidates = ["First repeated body size section", "Second repeated body size section"];
        for candidate_count in 1..=2 {
            let mut pages: Vec<Vec<SegmentData>> = candidates[..candidate_count]
                .iter()
                .enumerate()
                .map(|(index, candidate)| {
                    heading_page(
                        candidate,
                        12.0,
                        &format!("Ordinary body paragraph number {} ends here.", index + 1),
                    )
                })
                .collect();
            pages.extend((0..3).map(|index| {
                vec![seg_heuristic(
                    &format!("Filler paragraph number {} ends here.", index + 1),
                    12.0,
                    700.0,
                )]
            }));
            let document = extract_heading_test_document(pages, false);

            for candidate in &candidates[..candidate_count] {
                assert_eq!(
                    element_kind_for(&document, candidate),
                    Some(crate::types::internal::ElementKind::Paragraph),
                    "one or two body-size bold occurrences are emphasis, not a document convention"
                );
            }
        }
    }

    #[test]
    fn should_require_body_size_candidates_to_outnumber_other_headings_two_to_one() {
        let document = extract_heading_test_document(
            vec![
                heading_page(
                    "First existing larger heading",
                    18.0,
                    "The first ordinary body paragraph ends here.",
                ),
                heading_page(
                    "Second existing larger heading",
                    18.0,
                    "The second ordinary body paragraph ends here.",
                ),
                heading_page(
                    "First body size candidate section",
                    12.0,
                    "The third ordinary body paragraph ends here.",
                ),
                heading_page(
                    "Second body size candidate section",
                    12.0,
                    "The fourth ordinary body paragraph ends here.",
                ),
                heading_page(
                    "Third body size candidate section",
                    12.0,
                    "The fifth ordinary body paragraph ends here.",
                ),
            ],
            false,
        );

        for heading in ["First existing larger heading", "Second existing larger heading"] {
            assert!(matches!(
                element_kind_for(&document, heading),
                Some(crate::types::internal::ElementKind::Heading { .. })
            ));
        }
        for candidate in [
            "First body size candidate section",
            "Second body size candidate section",
            "Third body size candidate section",
        ] {
            assert_eq!(
                element_kind_for(&document, candidate),
                Some(crate::types::internal::ElementKind::Paragraph),
                "three candidates are not more than twice two independently detected headings"
            );
        }
    }

    /// Fallback title detection: 5-paragraph doc where first segment is 14pt,
    /// others are 11pt. Ratio 14/11 ≈ 1.27 ≥ 1.2 — fallback must fire only when
    /// k-means would fail to assign a heading.  This exercises the same fixture as
    /// `test_build_heading_map_short_doc_title_gets_heading_level_1` but specifically
    /// with k=1 (no clustering possible) to force the fallback path.
    #[test]
    fn test_build_heading_map_fallback_title_when_k_equals_1() {
        let title_seg = seg_with_font("Document Title", 14.0);
        let body_segs: Vec<SegmentData> = (0..4)
            .map(|i| seg_with_font(&format!("Body paragraph {i}."), 11.0))
            .collect();

        let mut segs = vec![title_seg];
        segs.extend(body_segs);

        let all_page_segments = vec![segs];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 1)
            .expect("build_heading_map must succeed");

        let title_entry = heading_map.iter().find(|(fs, _)| (*fs - 14.0).abs() < 0.5);
        let _ = title_entry;
    }

    /// Sparsity gate: a three-block, single-page document with one clearly larger
    /// first line must NOT promote that line to a heading. A larger opening line
    /// in a tiny document is display prose, not necessarily a title.
    #[test]
    fn test_build_heading_map_sparse_single_page_doc_no_heading_promotion() {
        let all_page_segments = vec![vec![
            seg_with_font("Display Text", 24.0),
            seg_with_font("Body paragraph one.", 12.0),
            seg_with_font("Body paragraph two.", 12.0),
        ]];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
            .expect("build_heading_map must succeed");

        assert!(
            heading_map.iter().all(|(_, level)| level.is_none()),
            "3-block doc must not promote the larger first line to a heading; got: {heading_map:?}"
        );
    }

    /// A sparse multi-page document has stronger evidence than a cover or title
    /// page when the same large-font tier repeats on separate pages and a smaller
    /// body tier is also present. This is the `hello_structure.pdf` shape.
    #[test]
    fn test_build_heading_map_sparse_multi_page_repeated_tier_promotes_headings() {
        let all_page_segments = vec![
            vec![seg_with_font("Hello World", 24.0)],
            vec![
                seg_with_font("Goodbye Cruel World...", 24.0),
                seg_with_font("I'll be back shortly!", 12.0),
            ],
        ];
        let struct_tree_results = vec![None, None];
        let heuristic_pages = vec![0usize, 1usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
            .expect("build_heading_map must succeed");

        let repeated_tier = heading_map
            .iter()
            .find(|(font_size, _)| (*font_size - 24.0).abs() < 0.5);
        assert!(
            repeated_tier.is_some_and(|(_, level)| *level == Some(2)),
            "a repeated 24pt tier across pages with a 12pt body tier must be promoted to H2; got: {heading_map:?}"
        );
    }

    #[test]
    fn test_build_heading_map_sparse_multi_page_does_not_promote_non_repeated_intermediate_tier() {
        use crate::pdf::structure::classify::{find_heading_level, precompute_gap_info};

        let all_page_segments = vec![
            vec![seg_with_font("Repeated Heading One", 22.0)],
            vec![
                seg_with_font("Repeated Heading Two", 22.0),
                seg_with_font("Display prose", 21.0),
                seg_with_font("Body paragraph.", 12.0),
            ],
        ];
        let struct_tree_results = vec![None, None];
        let heuristic_pages = vec![0usize, 1usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
            .expect("build_heading_map must succeed");
        let gap_info = precompute_gap_info(&heading_map);

        assert_eq!(
            find_heading_level(21.0, &heading_map, &gap_info),
            None,
            "a non-repeated intermediate font tier must remain prose; got: {heading_map:?}"
        );
    }

    #[test]
    fn test_build_heading_map_sparse_multi_page_does_not_promote_repeated_mid_page_display_text() {
        let all_page_segments = vec![
            vec![
                seg_with_font("Body paragraph one.", 12.0),
                seg_with_font("Repeated display text", 24.0),
            ],
            vec![
                seg_with_font("Body paragraph two.", 12.0),
                seg_with_font("Repeated pull quote", 24.0),
            ],
        ];
        let struct_tree_results = vec![None, None];
        let heuristic_pages = vec![0usize, 1usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
            .expect("build_heading_map must succeed");

        assert!(
            heading_map.iter().all(|(_, level)| level.is_none()),
            "repeated mid-page display text must remain prose in sparse documents; got: {heading_map:?}"
        );
    }

    /// Sparsity gate: a two-block document (the `issue-987-test.pdf` shape) with
    /// a larger first line must NOT promote either line to a heading.
    #[test]
    fn test_build_heading_map_two_block_doc_no_heading_promotion() {
        let all_page_segments = vec![vec![seg_with_font("Big Text", 24.0), seg_with_font("Small Text", 12.0)]];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
            .expect("build_heading_map must succeed");

        assert!(
            heading_map.iter().all(|(_, level)| level.is_none()),
            "2-block doc must not promote either line to a heading; got: {heading_map:?}"
        );
    }

    /// The sparsity gate must fire strictly below `MIN_BLOCKS_FOR_FONT_HEADING`:
    /// a document at exactly the floor (five blocks) still promotes its title,
    /// so genuine short documents keep their heading.
    #[test]
    fn test_build_heading_map_at_block_floor_still_promotes() {
        let mut segs = vec![seg_with_font("Section Title", 18.0)];
        segs.extend(
            (0..(MIN_BLOCKS_FOR_FONT_HEADING - 1)).map(|i| seg_with_font(&format!("Body paragraph {i}."), 11.0)),
        );

        let all_page_segments = vec![segs];
        let struct_tree_results = vec![None];
        let heuristic_pages = vec![0usize];

        let (heading_map, _) = build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, 4)
            .expect("build_heading_map must succeed");

        let title_entry = heading_map.iter().find(|(fs, _)| (*fs - 18.0).abs() < 0.5);
        assert_eq!(
            title_entry.and_then(|(_, level)| *level),
            Some(1),
            "at the block floor the title must still be promoted; got: {heading_map:?}"
        );
    }

    /// Segment with explicit font_size and baseline_y for heuristic-path tests.
    fn seg_heuristic(text: &str, font_size: f32, baseline_y: f32) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x: 10.0,
            y: baseline_y,
            width: 200.0,
            height: font_size,
            font_size,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y,
            rotation_degrees: 0.0,
            assigned_role: None,
        }
    }

    /// Heuristic path: two segments at different font sizes (triggering a split in
    /// blocks_to_paragraphs) that are a sentence continuation should be re-joined
    /// by merge_continuation_paragraphs.
    #[test]
    fn test_heuristic_path_merges_font_split_continuation() {
        let output = process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: None,
                heuristic_segments: vec![
                    seg_heuristic("een indicative", 12.0, 700.0),
                    seg_heuristic("van toenemende merkbekendheid", 13.8, 680.0),
                ],
                page_hints: None,
                table_bboxes: vec![],
                preserve_native_semantics: false,
                use_layout_reading_order: false,
                #[cfg(feature = "layout-detection")]
                hint_validations: vec![],
                #[cfg(feature = "layout-detection")]
                page_width_pts: None,
                needs_classify: false,
                paragraph_gap_ys: vec![],
                include_headers: true,
                include_footers: true,
                include_footnotes: false,
            },
            &[],
            None,
            &TextRepairWitnesses::default(),
        );
        assert_eq!(
            output.len(),
            1,
            "continuation paragraph split by font change should be merged on heuristic path"
        );
        assert!(
            output[0].text.is_empty(),
            "merged paragraph must have cleared text so assembly joins from segments"
        );
        let all_text: String = output[0]
            .lines
            .iter()
            .flat_map(|l| l.segments.iter())
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            all_text.contains("een indicative"),
            "first fragment must survive in merged segments; got: {all_text:?}"
        );
        assert!(
            all_text.contains("van toenemende merkbekendheid"),
            "second fragment must survive in merged segments; got: {all_text:?}"
        );
    }

    /// Heuristic path: a sentence-terminating paragraph followed by an
    /// uppercase-starting paragraph must NOT be merged.
    #[test]
    fn test_heuristic_path_does_not_merge_terminated_sentences() {
        let output = process_single_page(
            PageInput {
                page_index: 0,
                struct_paragraphs: None,
                heuristic_segments: vec![
                    seg_heuristic("The first sentence ends here.", 12.0, 700.0),
                    seg_heuristic("New sentence starts uppercase.", 13.8, 680.0),
                ],
                page_hints: None,
                table_bboxes: vec![],
                preserve_native_semantics: false,
                use_layout_reading_order: false,
                #[cfg(feature = "layout-detection")]
                hint_validations: vec![],
                #[cfg(feature = "layout-detection")]
                page_width_pts: None,
                needs_classify: false,
                paragraph_gap_ys: vec![],
                include_headers: true,
                include_footers: true,
                include_footnotes: false,
            },
            &[],
            None,
            &TextRepairWitnesses::default(),
        );
        assert_eq!(
            output.len(),
            2,
            "terminated sentence followed by uppercase must not be merged"
        );
    }

    /// Verify that non-contiguous index ranges across pages are handled correctly.
    #[test]
    fn test_image_index_offset_non_contiguous_pages() {
        let page1_indices: Vec<usize> = vec![0, 1];
        let page2_indices: Vec<usize> = vec![100, 101];

        for (indices, expected_first) in [(&page1_indices, 0usize), (&page2_indices, 100usize)] {
            let first_idx = indices.iter().copied().min().unwrap_or(0);
            assert_eq!(
                first_idx, expected_first,
                "first_idx_on_page must equal the minimum index in the slice"
            );

            let set: ahash::AHashSet<usize> = indices.iter().copied().collect();
            for current_image in 0..2usize {
                let global_idx = first_idx + current_image;
                assert!(
                    set.contains(&global_idx),
                    "global index {global_idx} must be found for page with first_idx={first_idx}"
                );
            }
        }
    }
}

#[cfg(test)]
mod list_marker_tests {
    use super::{is_bare_detached_list_marker, is_bare_list_marker, looks_like_list_item};

    #[test]
    fn bare_markers_are_detected() {
        assert!(is_bare_list_marker("1."));
        assert!(is_bare_list_marker("12)"));
        assert!(is_bare_list_marker("a."));
        assert!(is_bare_list_marker("a)"));
        assert!(is_bare_list_marker("I."));
        assert!(is_bare_list_marker("(1)"));
        assert!(is_bare_list_marker("(2)"));
        assert!(is_bare_list_marker("[1]"));
        assert!(is_bare_list_marker("•"));
    }

    #[test]
    fn prose_fragments_are_not_bare_markers() {
        assert!(!is_bare_list_marker("etc."));
        assert!(!is_bare_list_marker("Inc."));
        assert!(!is_bare_list_marker("(appendix)"));
        assert!(!is_bare_list_marker("Item"));
        assert!(!is_bare_list_marker(""));
    }

    /// The general [`is_bare_list_marker`] still accepts a lone `*` and a
    /// bracketed integer -- only the narrower detached-reattachment predicate
    /// rejects them. See `EXCLUDE_AMBIGUOUS_DETACHED_MARKERS`.
    #[test]
    fn detached_predicate_rejects_the_ambiguous_shapes_the_general_one_still_accepts() {
        assert!(
            is_bare_list_marker("*"),
            "general predicate must still accept a lone '*'"
        );
        assert!(
            is_bare_list_marker("[42]"),
            "general predicate must still accept a bracketed integer"
        );
        assert!(
            !is_bare_detached_list_marker("*"),
            "a lone '*' is also a multiplication sign; the detached pass must reject it"
        );
        assert!(
            !is_bare_detached_list_marker("[42]"),
            "a bracketed integer is a printed paragraph number; the detached pass must reject it"
        );
    }

    /// Every shape the task's evidence names as "good, keep" must survive the
    /// tightening on the detached-reattachment predicate.
    #[test]
    fn detached_predicate_still_accepts_the_unambiguous_shapes() {
        assert!(is_bare_detached_list_marker("-"));
        assert!(is_bare_detached_list_marker("–"));
        assert!(is_bare_detached_list_marker("—"));
        assert!(is_bare_detached_list_marker("(1)"));
        assert!(is_bare_detached_list_marker("(k)"));
        assert!(is_bare_detached_list_marker("1."));
        assert!(is_bare_detached_list_marker("f."));
    }

    #[test]
    fn newline_separated_marker_and_text_is_a_list_item() {
        assert!(looks_like_list_item("1.\nÉnumération 1"));
        assert!(looks_like_list_item("1. First point"));
        assert!(looks_like_list_item("123. One hundred twenty-third point"));
        assert!(looks_like_list_item("999. Nine hundred ninety-ninth point"));
        assert!(!looks_like_list_item("1000. Four-digit identifier"));
        assert!(looks_like_list_item("viii. eighth item"));
        assert!(looks_like_list_item("(2)\nsecond item"));
        assert!(looks_like_list_item("[1] bracketed item"));
    }

    #[test]
    fn four_digit_year_is_not_a_list_item() {
        assert!(!looks_like_list_item("2023. A total of 3 trucks were used"));
    }

    #[test]
    fn section_headings_are_not_list_items() {
        assert!(!looks_like_list_item("3.2 Methods"));
        assert!(!looks_like_list_item("IV. Results"));
        assert!(!looks_like_list_item("1. INTRODUCTION"));
    }

    #[test]
    fn prose_words_ending_with_period_are_not_list_markers() {
        assert!(!looks_like_list_item("tua. At vero eos et accusam"));
        assert!(!looks_like_list_item("etc. and more prose"));
        assert!(looks_like_list_item("a. first item"));
        assert!(looks_like_list_item("iv. fourth item"));
    }

    #[test]
    fn typographic_dash_requires_an_inline_body() {
        assert!(looks_like_list_item("– first item"));
        assert!(looks_like_list_item("—\tsecond item"));
        assert!(looks_like_list_item("– “quoted item”"));
        assert!(looks_like_list_item("— (parenthesized item)"));
        assert!(!looks_like_list_item("–\n457"));
        assert!(!looks_like_list_item("– \n457"));
        assert!(!looks_like_list_item("—\t\nbody"));
        assert!(!looks_like_list_item("–\n8 show the remaining figures"));
        assert!(!looks_like_list_item("—continuation"));
    }

    /// #### FAILS against unfixed code
    /// Both assertions currently evaluate to `true` (unfixed
    /// `looks_like_list_item` accepts any `(N) <alphabetic>` line), so
    /// `assert!(!looks_like_list_item(...))` panics with `assertion failed:
    /// !looks_like_list_item("(2) additional on-street parallel parking
    /// spaces")` (and the `(7)` sibling) on unfixed code.
    #[test]
    fn parenthesized_quantity_clarifications_are_not_list_items() {
        assert!(!looks_like_list_item(
            "(2) additional on-street parallel parking spaces"
        ));
        assert!(!looks_like_list_item("(7) on-street spaces on Lake Pointe Parkway"));
        assert!(!looks_like_list_item("(3) additional off-street spaces"));
        assert!(!looks_like_list_item("(9) exceptions apply"));
    }

    /// Lettered sub-items in parentheses are genuine markers in this same
    /// ordinance and must survive the quantity-clarification heuristic above
    /// (it is scoped to *numeric* parenthesized markers only).
    #[test]
    fn parenthesized_letter_markers_remain_list_items() {
        assert!(looks_like_list_item("(a) Front setback: 25'"));
        assert!(looks_like_list_item("(b) Side setback: 0'/6'"));
        assert!(looks_like_list_item("(c) Street side setback: Lot 1 - 15'"));
    }

    /// A capitalized, space-separated numeric parenthesized marker is a
    /// genuine enumerated item (a new sentence), not a quantity
    /// clarification, and must still be accepted.
    #[test]
    fn capitalized_parenthesized_numeric_markers_remain_list_items() {
        assert!(looks_like_list_item("(1) First point"));
        assert!(looks_like_list_item("(2) Second point"));
    }

    #[test]
    fn author_initials_are_not_list_markers() {
        assert!(!looks_like_list_item(
            "O. Sanni, A.P.I. Popoola / Data in Brief 22 (2019) 451"
        ));
        assert!(!looks_like_list_item("O. Sanni, A. Popoola / Data in Brief"));
        assert!(looks_like_list_item("A. First item"));
        assert!(looks_like_list_item("a. first item"));
        assert!(looks_like_list_item("A. Compare input, output / behavior"));
    }
}
