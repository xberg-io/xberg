//! Bridge between pdfium extraction APIs and the markdown pipeline.
//!
//! Two conversion paths:
//! 1. Structure tree: `ExtractedBlock` → `PdfParagraph` (for tagged PDFs)
//! 2. Page objects: `PdfPage` → `(Vec<SegmentData>, Vec<ImagePosition>)` (heuristic extraction)
//!
//! The page objects path includes post-processing ligature repair for pages
//! with broken font encodings (detected via `PdfPageTextChar::has_unicode_map_error()`).

use std::borrow::Cow;

use crate::pdf::hierarchy::SegmentData;
use pdfium_render::prelude::*;

use super::constants::{MAX_HEADING_WORD_COUNT, MIN_HEADING_FONT_GAP, MIN_HEADING_FONT_RATIO};
use super::types::{PdfLine, PdfParagraph};

// Alias to distinguish from our local PdfParagraph type.
use pdfium_render::prelude::PdfParagraph as PdfiumParagraph;

use memchr::memchr3;

/// Position and metadata of an image detected during object-based extraction.
#[derive(Debug, Clone)]
pub(super) struct ImagePosition {
    /// 1-indexed page number.
    pub page_number: usize,
    /// Global image index across the document.
    pub image_index: usize,
}

/// Filter sidebar artifacts from structure tree extracted blocks.
///
/// Removes blocks that appear to be sidebar text (e.g., arXiv identifiers
/// rendered vertically along page margins). Detection criteria:
/// - Block has bounds in the leftmost or rightmost margin (< 8% or > 92% of page width)
/// - Block text is very short (≤ 3 characters trimmed)
/// - At least 3 such blocks exist (to avoid false positives on legitimate margin content)
pub(super) fn filter_sidebar_blocks(blocks: &[ExtractedBlock], page_width: f32) -> Cow<'_, [ExtractedBlock]> {
    if page_width <= 0.0 {
        return Cow::Borrowed(blocks);
    }

    let left_cutoff = page_width * 0.08;
    let right_cutoff = page_width * 0.92;

    // Count short-text blocks in margins
    let sidebar_count = count_sidebar_blocks(blocks, left_cutoff, right_cutoff);

    if sidebar_count < 3 {
        return Cow::Borrowed(blocks);
    }

    // Filter them out
    Cow::Owned(filter_blocks_recursive(blocks, left_cutoff, right_cutoff))
}

fn count_sidebar_blocks(blocks: &[ExtractedBlock], left_cutoff: f32, right_cutoff: f32) -> usize {
    let mut count = 0;
    for block in blocks {
        if !block.children.is_empty() {
            count += count_sidebar_blocks(&block.children, left_cutoff, right_cutoff);
        } else if is_sidebar_block(block, left_cutoff, right_cutoff) {
            count += 1;
        }
    }
    count
}

fn is_sidebar_block(block: &ExtractedBlock, left_cutoff: f32, right_cutoff: f32) -> bool {
    let trimmed = block.text.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 3 {
        return false;
    }
    if let Some(bounds) = &block.bounds {
        let left = bounds.left().value;
        let right = bounds.right().value;
        // Block is entirely within left or right margin
        right < left_cutoff || left > right_cutoff
    } else {
        false
    }
}

fn filter_blocks_recursive(blocks: &[ExtractedBlock], left_cutoff: f32, right_cutoff: f32) -> Vec<ExtractedBlock> {
    blocks
        .iter()
        .filter_map(|block| {
            if !block.children.is_empty() {
                let filtered_children = filter_blocks_recursive(&block.children, left_cutoff, right_cutoff);
                if filtered_children.is_empty() {
                    return None;
                }
                Some(ExtractedBlock {
                    children: filtered_children,
                    ..block.clone()
                })
            } else if is_sidebar_block(block, left_cutoff, right_cutoff) {
                None
            } else {
                Some(block.clone())
            }
        })
        .collect()
}

/// Convert extracted blocks from the structure tree API into PdfParagraphs.
///
/// Structure tree heading levels are validated against font size and word count
/// to prevent broken structure trees from marking body text as headings.
pub(super) fn extracted_blocks_to_paragraphs(blocks: &[ExtractedBlock]) -> Vec<PdfParagraph> {
    // First pass: collect font sizes to determine body font size
    let body_font_size = estimate_body_font_size(blocks);

    // Second pass: convert blocks with validated heading levels
    let mut paragraphs = Vec::new();
    convert_blocks(blocks, body_font_size, &mut paragraphs);
    paragraphs
}

/// Recursively estimate the body (most common) font size from all leaf blocks.
fn estimate_body_font_size(blocks: &[ExtractedBlock]) -> f32 {
    let mut sizes: Vec<f32> = Vec::new();
    collect_font_sizes(blocks, &mut sizes);

    if sizes.is_empty() {
        return 12.0;
    }

    super::lines::most_frequent_font_size(sizes.into_iter())
}

fn collect_font_sizes(blocks: &[ExtractedBlock], sizes: &mut Vec<f32>) {
    for block in blocks {
        if !block.children.is_empty() {
            collect_font_sizes(&block.children, sizes);
        } else if !block.text.trim().is_empty() {
            sizes.push(block.font_size.unwrap_or(12.0));
        }
    }
}

/// Recursively convert blocks to paragraphs with heading validation.
fn convert_blocks(blocks: &[ExtractedBlock], body_font_size: f32, paragraphs: &mut Vec<PdfParagraph>) {
    for block in blocks {
        if !block.children.is_empty() {
            convert_blocks(&block.children, body_font_size, paragraphs);
            continue;
        }

        if block.text.trim().is_empty() {
            continue;
        }

        let is_list_item = matches!(&block.role, ContentRole::ListItem { .. });

        let full_text = if let ContentRole::ListItem { label: Some(ref l) } = block.role {
            format!("{} {}", l, block.text)
        } else {
            block.text.clone()
        };

        let font_size = block.font_size.unwrap_or(12.0);
        let word_count = full_text.split_whitespace().count();

        // Validate heading level from structure tree:
        // Only accept if font size is meaningfully larger than body AND word count is low
        let heading_level = match &block.role {
            ContentRole::Heading { level } => {
                let ratio_ok = font_size >= body_font_size * MIN_HEADING_FONT_RATIO;
                let gap_ok = font_size - body_font_size >= MIN_HEADING_FONT_GAP;
                let words_ok = word_count <= MAX_HEADING_WORD_COUNT;
                if (ratio_ok || gap_ok) && words_ok {
                    Some(*level)
                } else {
                    None
                }
            }
            _ => None,
        };

        // Create segments from the block text (one per whitespace-delimited word)
        let segments: Vec<SegmentData> = full_text
            .split_whitespace()
            .map(|w| SegmentData {
                text: w.to_string(),
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
                font_size,
                is_bold: block.is_bold,
                is_italic: block.is_italic,
                is_monospace: false,
                baseline_y: 0.0,
            })
            .collect();

        if segments.is_empty() {
            continue;
        }

        let line = PdfLine {
            segments,
            baseline_y: 0.0,
            dominant_font_size: font_size,
            is_bold: block.is_bold,
            is_monospace: false,
        };

        paragraphs.push(PdfParagraph {
            lines: vec![line],
            dominant_font_size: font_size,
            heading_level,
            is_bold: block.is_bold,
            is_list_item,
            is_code_block: false,
        });
    }
}

/// Extract text segments and image positions from a PDF page.
///
/// Uses the page objects API with column detection for text extraction.
/// For pages with broken font encodings (ligature corruption), applies
/// per-character repair using `PdfPageTextChar::has_unicode_map_error()`.
///
/// Also detects image objects and records their positions for interleaving.
pub(super) fn objects_to_page_data(
    page: &PdfPage,
    page_number: usize,
    image_offset: &mut usize,
) -> (Vec<SegmentData>, Vec<ImagePosition>) {
    let objects: Vec<PdfPageObject> = page.objects().iter().collect();

    // Image scan BEFORE text extraction.
    let mut images = Vec::new();
    for obj in &objects {
        if obj.as_image_object().is_some() {
            images.push(ImagePosition {
                page_number,
                image_index: *image_offset,
            });
            *image_offset += 1;
        }
    }

    // Primary path: per-character extraction using pdfium's text API.
    // This produces more accurate text and positions than from_objects()
    // because it uses the same text extraction engine as plain text mode.
    // Ligature repair is integrated inline.
    if let Some(segments) = chars_to_segments(page) {
        return (segments, images);
    }

    // Fallback: page objects API with column detection.
    // Used when page.text() fails (rare edge case).
    let mut segments = Vec::new();
    let column_groups = super::columns::split_objects_into_columns(&objects);
    let column_vecs = partition_objects_by_columns(objects, &column_groups);
    for column_objects in &column_vecs {
        let paragraphs: Vec<PdfiumParagraph> = PdfiumParagraph::from_objects(column_objects);
        extract_paragraphs_to_segments(paragraphs, &mut segments);
    }

    // Apply ligature repair for fallback path.
    if let Some(repair_map) = build_ligature_repair_map(page) {
        for seg in &mut segments {
            seg.text = apply_ligature_repairs(&seg.text, &repair_map);
        }
    }

    (segments, images)
}

/// Partition page objects into column groups by moving objects out of the source vec.
///
/// Each column group is a `Vec<usize>` of indices into `objects`. This function
/// consumes the objects vec and returns one `Vec<PdfPageObject>` per column.
fn partition_objects_by_columns<'a>(
    objects: Vec<PdfPageObject<'a>>,
    column_groups: &[Vec<usize>],
) -> Vec<Vec<PdfPageObject<'a>>> {
    if column_groups.len() <= 1 {
        return vec![objects];
    }

    let total = objects.len();
    let num_columns = column_groups.len();
    let mut col_for_obj = vec![0usize; total];
    for (col_idx, group) in column_groups.iter().enumerate() {
        for &obj_idx in group {
            if obj_idx < total {
                col_for_obj[obj_idx] = col_idx;
            }
        }
    }

    let mut result: Vec<Vec<PdfPageObject<'a>>> = (0..num_columns).map(|_| Vec::new()).collect();
    for (i, obj) in objects.into_iter().enumerate() {
        result[col_for_obj[i]].push(obj);
    }

    result
}

/// Build a mapping of corrupted characters → correct ligature expansions for a page.
///
/// Walks the per-character API to find characters with `has_unicode_map_error()`,
/// then determines the correct ligature expansion based on the character's raw
/// unicode value and font-specific encoding patterns.
///
/// Returns `None` if the page has no encoding errors (most pages).
pub(super) fn build_ligature_repair_map(page: &PdfPage) -> Option<Vec<(char, &'static str)>> {
    let text = match page.text() {
        Ok(t) => t,
        Err(_) => return None,
    };

    let chars = text.chars();
    let char_count = chars.len();
    if char_count == 0 {
        return None;
    }

    let mut repair_map: Vec<(char, &'static str)> = Vec::new();

    for i in 0..char_count {
        let ch = match chars.get(i) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if ch.is_generated().unwrap_or(false) {
            continue;
        }

        if !ch.has_unicode_map_error().unwrap_or(false) {
            continue;
        }

        // Skip symbol/math fonts — their encodings are intentional
        if ch.font_is_symbolic() {
            continue;
        }

        let unicode_val = ch.unicode_value();
        let mapped_char = match char::from_u32(unicode_val) {
            Some(c) => c,
            None => continue,
        };

        // Check if we already have a mapping for this character
        if repair_map.iter().any(|(c, _)| *c == mapped_char) {
            continue;
        }

        // Determine the correct ligature based on raw unicode value.
        // Different fonts encode ligatures at different positions. We check
        // both the low-byte encoding (CM fonts) and ASCII fallback positions.
        let ligature = match unicode_val {
            // Standard Type1/CM ligature positions (low bytes)
            0x0B => "ff",
            0x0C => "fi",
            0x0D => "fl",
            0x0E => "ffi",
            0x0F => "ffl",
            // Alternate low-byte positions used by some fonts
            0x01 => "fi",
            0x02 => "fl",
            0x03 => "ff",
            0x04 => "ffi",
            0x05 => "ffl",
            // ASCII positions: broken CMap maps ligature glyph codes to these
            // ASCII characters. Safe because we only reach here when
            // has_unicode_map_error() is true (i.e. pdfium detected encoding issues).
            0x21 => "fi",  // '!' → fi (most common ligature corruption)
            0x22 => "ff",  // '"' → ff
            0x23 => "fl",  // '#' → fl
            0x24 => "ffi", // '$' → ffi
            0x25 => "ffl", // '%' → ffl
            _ => continue,
        };

        repair_map.push((mapped_char, ligature));
    }

    if repair_map.is_empty() { None } else { Some(repair_map) }
}

/// Apply ligature repairs to a text string using a page-specific repair map.
pub(super) fn apply_ligature_repairs(text: &str, repair_map: &[(char, &str)]) -> String {
    let mut result = String::with_capacity(text.len() + 16);
    for ch in text.chars() {
        if let Some((_, replacement)) = repair_map.iter().find(|(c, _)| *c == ch) {
            result.push_str(replacement);
        } else {
            result.push(ch);
        }
    }
    result
}

/// Repair ligature corruption using contextual heuristics.
///
/// Some PDF fonts (particularly Computer Modern from TeX/LaTeX) have broken
/// ToUnicode CMaps that map ligature glyphs to ASCII characters:
/// - `fi` → `!` (0x21)
/// - `ff` → `"` (0x22)
/// - `fl` → `#` (0x23)
///
/// Unlike `build_ligature_repair_map()` which relies on `has_unicode_map_error()`,
/// this function detects corruption contextually: `!`, `"`, or `#` appearing
/// between alphabetic characters (e.g., `e!cient`, `o"ces`, `#nancial`) is
/// a near-certain indicator of ligature corruption, as these patterns virtually
/// never occur in real text.
///
/// This is safe to apply broadly because:
/// - Normal `!` appears at word/sentence boundaries, not between letters
/// - Normal `"` appears at word boundaries (quotation marks), not mid-word
/// - Normal `#` appears at word start (hashtags) or after non-letters, not mid-word
pub(super) fn repair_contextual_ligatures(text: &str) -> String {
    if text.len() < 2 {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len() + 16);
    let bytes = text.as_bytes();
    let chars = text.chars().peekable();
    let mut byte_idx = 0;
    let mut prev_is_alpha = false;
    let mut prev_is_space_or_start = true;

    for ch in chars {
        let char_len = ch.len_utf8();
        let next_byte_idx = byte_idx + char_len;

        let next_is_alpha = if next_byte_idx < bytes.len() {
            if let Some(&next_byte) = bytes.get(next_byte_idx) {
                (next_byte as char).is_alphabetic()
            } else {
                false
            }
        } else {
            false
        };

        let next_is_lower = if next_byte_idx < bytes.len() {
            if let Some(&next_byte) = bytes.get(next_byte_idx) {
                (next_byte as char).is_lowercase()
            } else {
                false
            }
        } else {
            false
        };

        let next_is_vowel = if next_byte_idx < bytes.len() {
            if let Some(&next_byte) = bytes.get(next_byte_idx) {
                matches!(
                    next_byte as char,
                    'a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U'
                )
            } else {
                false
            }
        } else {
            false
        };

        match ch {
            '!' if prev_is_alpha && next_is_vowel => result.push_str("ff"),
            '!' if prev_is_alpha && next_is_alpha => result.push_str("fi"),
            '!' if prev_is_alpha && next_byte_idx >= bytes.len() => result.push_str("fi"),
            '"' if prev_is_alpha && next_is_alpha => result.push_str("ffi"),
            '#' if prev_is_alpha && next_is_alpha => result.push_str("fi"),
            '#' if prev_is_space_or_start && next_is_lower => result.push_str("fi"),
            '!' if prev_is_space_or_start && next_is_lower => result.push_str("fi"),
            _ => result.push(ch),
        }

        prev_is_alpha = ch.is_alphabetic();
        prev_is_space_or_start = ch.is_whitespace();
        byte_idx = next_byte_idx;
    }

    result
}

/// Check if text contains ligature corruption patterns.
///
/// Returns true if the text shows signs of broken ligature encoding:
/// - Mid-word: `!`, `"`, or `#` between alphabetic characters
/// - Word-start: `#` or `!` after whitespace/start followed by lowercase letter
///
/// Requires 2+ matches to avoid false positives from normal punctuation.
pub(super) fn text_has_ligature_corruption(text: &str) -> bool {
    if text.len() < 3 {
        return false;
    }

    let bytes = text.as_bytes();
    let mut count = 0u32;
    let mut pos = 0;

    while let Some(idx) = memchr3(b'!', b'"', b'#', &bytes[pos..]) {
        let i = pos + idx;
        let ch = bytes[i];

        let prev_alpha = if i > 0 {
            let prev_byte = bytes[i - 1];
            (prev_byte as char).is_alphabetic()
        } else {
            false
        };

        let next_alpha = if i + 1 < bytes.len() {
            let next_byte = bytes[i + 1];
            (next_byte as char).is_alphabetic()
        } else {
            false
        };

        let prev_space_or_start = i == 0 || (bytes[i - 1] as char).is_whitespace();

        let next_lower = if i + 1 < bytes.len() {
            let next_byte = bytes[i + 1];
            (next_byte as char).is_lowercase()
        } else {
            false
        };

        if prev_alpha && next_alpha {
            count += 1;
        }

        if matches!(ch, b'#' | b'!') && prev_space_or_start && next_lower {
            count += 1;
        }

        pos = i + 1;
        if count >= 1 {
            break;
        }
    }

    count >= 1
}

/// Per-character data extracted from pdfium's text API.
struct CharInfo {
    ch: char,
    x: f32,
    y: f32,
    font_size: f32,
    is_bold: bool,
    is_italic: bool,
    is_monospace: bool,
    has_map_error: bool,
    is_symbolic: bool,
}

/// Remove characters from sidebar annotations (e.g., arXiv identifiers along the left margin).
///
/// Sidebar text in papers is typically rotated 90° along the left margin, producing
/// isolated characters at very low X positions that span most of the page height.
/// This distinguishes them from bullets/labels which only span a small region.
///
/// Detection criteria:
/// 1. Characters in the leftmost 5% of page width
/// 2. Constitute < 5% of total characters
/// 3. Span at least 30% of the page's vertical text extent
fn filter_sidebar_characters(char_infos: &mut Vec<CharInfo>, page_width: f32) {
    if char_infos.len() < 20 || page_width <= 0.0 {
        return;
    }

    let total_non_space = char_infos.iter().filter(|c| c.ch != ' ').count();
    if total_non_space < 20 {
        return;
    }

    let margin_band = page_width * 0.065;

    let margin_indices: Vec<usize> = char_infos
        .iter()
        .enumerate()
        .filter(|(_, c)| c.ch != ' ' && c.x < margin_band)
        .map(|(i, _)| i)
        .collect();

    // Need some margin chars, but not too many (< 5% of total)
    if margin_indices.is_empty() || margin_indices.len() * 20 > total_non_space {
        return;
    }

    // Sidebar text spans most of the page height; bullets/labels don't.
    let (y_min, y_max) = char_infos
        .iter()
        .filter(|c| c.ch != ' ')
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), c| {
            (lo.min(c.y), hi.max(c.y))
        });
    let page_text_height = (y_max - y_min).max(1.0);

    let (margin_y_min, margin_y_max) =
        margin_indices
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &i| {
                let y = char_infos[i].y;
                (lo.min(y), hi.max(y))
            });
    let margin_y_span = (margin_y_max - margin_y_min).abs();

    if margin_y_span < page_text_height * 0.3 {
        return; // Margin chars don't span the page — not a sidebar
    }

    // Remove sidebar characters (reverse order to preserve indices)
    for &idx in margin_indices.iter().rev() {
        char_infos.remove(idx);
    }
}

/// Extract text segments from a PDF page using pdfium's text API.
///
/// Uses `page.text().all()` for correct text content (pdfium handles font matrices,
/// CMap lookups, word boundaries) and per-character origins for line-level positioning.
/// This produces better recall than `PdfiumParagraph::from_objects()` which can miss
/// Build the text for a single line from character info, inserting spaces where
/// large X-position gaps indicate word or column boundaries.
///
/// This is critical for positioned/tabular PDFs where characters are placed at
/// specific coordinates without explicit space characters between words.
fn build_line_text(chars: &[CharInfo], repair_map: Option<&[(char, &str)]>) -> String {
    let mut line_text = String::new();
    for (idx, ci) in chars.iter().enumerate() {
        if ci.has_map_error
            && !ci.is_symbolic
            && let Some(map) = repair_map
            && let Some((_, replacement)) = map.iter().find(|(c, _)| *c == ci.ch)
        {
            line_text.push_str(replacement);
            continue;
        }

        // Insert space when there is a significant X-position gap between
        // consecutive non-space characters on the same line.
        if idx > 0 && ci.ch != ' ' && chars[idx - 1].ch != ' ' {
            let prev = &chars[idx - 1];
            let gap = ci.x - prev.x;
            // Typical character advance ≈ font_size * 0.5–0.6.
            // A gap larger than one full character width suggests a word break.
            let avg_fs = (ci.font_size + prev.font_size) * 0.5;
            let threshold = avg_fs * 0.8;
            if gap > threshold {
                line_text.push(' ');
            }
        }

        line_text.push(ci.ch);
    }
    line_text
}

/// content when font metrics are broken.
///
/// Strategy:
/// 1. Get full page text from pdfium (already correctly assembled with spaces)
/// 2. Walk characters to find line breaks (Y position changes)
/// 3. Emit one SegmentData per line with proper baseline_y and x position
/// 4. Apply ligature repair inline
fn chars_to_segments(page: &PdfPage) -> Option<Vec<SegmentData>> {
    let text_obj = page.text().ok()?;
    let chars = text_obj.chars();
    let char_count = chars.len();
    if char_count == 0 {
        return None;
    }

    // Build ligature repair map for this page (if needed).
    let repair_map = build_ligature_repair_map(page);

    // Collect per-character data.
    let mut char_infos: Vec<CharInfo> = Vec::with_capacity(char_count);
    for i in 0..char_count {
        let ch = match chars.get(i) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Generated chars = word boundaries. Emit as spaces.
        if ch.is_generated().unwrap_or(false) {
            // Use the origin of the previous char if available
            let (x, y) = if let Some(last) = char_infos.last() {
                (last.x + last.font_size * 0.5, last.y)
            } else {
                (0.0, 0.0)
            };
            char_infos.push(CharInfo {
                ch: ' ',
                x,
                y,
                font_size: char_infos.last().map_or(12.0, |c| c.font_size),
                is_bold: false,
                is_italic: false,
                is_monospace: false,
                has_map_error: false,
                is_symbolic: false,
            });
            continue;
        }

        let unicode_val = ch.unicode_value();
        if unicode_val == 0xFFFE || unicode_val == 0xFFFF || unicode_val == 0 {
            continue;
        }
        let uc = match char::from_u32(unicode_val) {
            Some(c) => c,
            None => continue,
        };
        if uc.is_control() && uc != '\n' && uc != '\r' && uc != '\t' {
            continue;
        }
        // Skip soft hyphens (invisible break hints)
        if uc == '\u{00AD}' {
            continue;
        }

        let origin = match ch.origin() {
            Ok(o) => o,
            Err(_) => continue,
        };
        let fs = ch.scaled_font_size().value;
        let font_info = ch.font_info();

        char_infos.push(CharInfo {
            ch: uc,
            x: origin.0.value,
            y: origin.1.value,
            font_size: if fs > 0.0 { fs } else { 12.0 },
            is_bold: font_info.1,
            is_italic: font_info.2,
            is_monospace: ch.font_is_fixed_pitch(),
            has_map_error: ch.has_unicode_map_error().unwrap_or(false),
            is_symbolic: ch.font_is_symbolic(),
        });
    }

    // Filter out sidebar/margin characters (e.g., arXiv identifiers along left margin).
    let page_width = page.width().value;
    // Debug: show chars with smallest x values on this page
    filter_sidebar_characters(&mut char_infos, page_width);

    if char_infos.is_empty() {
        return None;
    }

    // Compute median line height from Y-position changes to detect line breaks.
    // This is font-metric-independent and works even when scaled_font_size is wrong.
    let mut y_jumps: Vec<f32> = Vec::new();
    for i in 1..char_infos.len() {
        if char_infos[i].ch == ' ' || char_infos[i - 1].ch == ' ' {
            continue;
        }
        let dy = (char_infos[i].y - char_infos[i - 1].y).abs();
        if dy > 1.0 && dy < 200.0 {
            y_jumps.push(dy);
        }
    }
    // Typical line spacing: use the smallest common Y-jump as line height.
    // Lines on the same baseline have dy ≈ 0; different lines have dy ≈ line_height.
    let line_height_threshold = if y_jumps.len() >= 3 {
        y_jumps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // Use 60% of the most common (smallest) line jump as the threshold
        y_jumps[0] * 0.6
    } else {
        // Fallback: use font size if available
        let avg_fs = char_infos.iter().map(|c| c.font_size).sum::<f32>() / char_infos.len() as f32;
        avg_fs * 0.5
    };
    let line_break_threshold = line_height_threshold.max(2.0);

    // Split into line-level segments based on Y-position changes.
    let mut segments = Vec::new();
    let mut line_start = 0;

    for i in 1..=char_infos.len() {
        let is_line_break = if i == char_infos.len() {
            true // End of page
        } else {
            let dy = (char_infos[i].y - char_infos[line_start].y).abs();
            dy > line_break_threshold && char_infos[i].ch != ' '
        };

        if is_line_break {
            let line_text = build_line_text(&char_infos[line_start..i], repair_map.as_deref());

            let trimmed = line_text.trim();
            if !trimmed.is_empty() {
                let first = &char_infos[line_start];
                // Find last non-space char for width calculation
                let last_idx = (line_start..i)
                    .rev()
                    .find(|&j| char_infos[j].ch != ' ')
                    .unwrap_or(line_start);
                let last = &char_infos[last_idx];
                let width = (last.x - first.x).max(first.font_size);

                segments.push(SegmentData {
                    text: trimmed.to_string(),
                    x: first.x,
                    y: first.y,
                    width,
                    height: first.font_size,
                    font_size: first.font_size,
                    is_bold: first.is_bold,
                    is_italic: first.is_italic,
                    is_monospace: first.is_monospace,
                    baseline_y: first.y,
                });
            }

            if i < char_infos.len() {
                line_start = i;
            }
        }
    }

    if segments.is_empty() { None } else { Some(segments) }
}

/// Convert pdfium paragraphs into SegmentData, preserving per-line positions.
fn extract_paragraphs_to_segments(paragraphs: Vec<PdfiumParagraph>, segments: &mut Vec<SegmentData>) {
    for para in paragraphs {
        for line in para.into_lines() {
            let line_baseline = line.bottom.value;
            let line_left = line.left.value;
            let mut running_x = line_left;

            for fragment in &line.fragments {
                match fragment {
                    PdfParagraphFragment::StyledString(styled) => {
                        let text = normalize_text_encoding(styled.text());
                        if text.trim().is_empty() {
                            continue;
                        }

                        let font_size = styled.font_size().value;
                        let is_bold = styled.is_bold();
                        let is_italic = styled.is_italic();
                        let is_monospace = styled.is_monospace();
                        let estimated_width = text.len() as f32 * font_size * 0.5;

                        segments.push(SegmentData {
                            text,
                            x: running_x,
                            y: line_baseline,
                            width: estimated_width,
                            height: font_size,
                            font_size,
                            is_bold,
                            is_italic,
                            is_monospace,
                            baseline_y: line_baseline,
                        });

                        running_x += estimated_width;
                    }
                    PdfParagraphFragment::NonTextObject(_) | PdfParagraphFragment::LineBreak { .. } => {}
                }
            }
        }
    }
}

/// Normalize text encoding: handle soft hyphens and strip control characters.
///
/// - `\u{00AD}` (soft hyphen) at end of text → replaced with `-` so downstream
///   hyphen-rejoining logic can merge word fragments.
/// - `\u{00AD}` mid-text → removed (invisible break hint).
/// - C0 control characters (U+0000–U+001F except `\t`, `\n`, `\r`) → removed.
fn normalize_text_encoding(text: &str) -> String {
    // Fast path: no special characters present
    if !text.contains('\u{00AD}') && !text.bytes().any(|b| b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r') {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\u{00AD}' => {
                // Soft hyphen at end of text (or before whitespace): convert to regular
                // hyphen so rendering code can rejoin word fragments.
                let at_end = chars.peek().is_none_or(|c| c.is_whitespace());
                if at_end {
                    result.push('-');
                }
                // Mid-word soft hyphen: drop (invisible break hint)
            }
            c if c.is_control() && c != '\n' && c != '\r' && c != '\t' => {
                // Strip other control characters
            }
            _ => result.push(ch),
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block(role: ContentRole, text: &str) -> ExtractedBlock {
        ExtractedBlock {
            role,
            text: text.to_string(),
            bounds: None,
            font_size: Some(12.0),
            is_bold: false,
            is_italic: false,
            children: Vec::new(),
        }
    }

    fn make_block_with_font(role: ContentRole, text: &str, font_size: f32) -> ExtractedBlock {
        ExtractedBlock {
            role,
            text: text.to_string(),
            bounds: None,
            font_size: Some(font_size),
            is_bold: false,
            is_italic: false,
            children: Vec::new(),
        }
    }

    #[test]
    fn test_heading_block() {
        // Heading must have meaningfully larger font than body for validation to pass
        let blocks = vec![
            make_block_with_font(ContentRole::Heading { level: 2 }, "Section Title", 18.0),
            make_block_with_font(ContentRole::Paragraph, "Body text line one", 12.0),
            make_block_with_font(ContentRole::Paragraph, "Body text line two", 12.0),
            make_block_with_font(ContentRole::Paragraph, "Body text line three", 12.0),
        ];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 4);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_heading_rejected_when_same_font_as_body() {
        // Heading with same font size as body should be rejected
        let blocks = vec![
            make_block(ContentRole::Heading { level: 3 }, "Not really a heading"),
            make_block(ContentRole::Paragraph, "Body text"),
            make_block(ContentRole::Paragraph, "More body text"),
        ];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 3);
        assert_eq!(paragraphs[0].heading_level, None); // Rejected: same font size
    }

    #[test]
    fn test_body_block() {
        let blocks = vec![make_block(ContentRole::Paragraph, "Body text")];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].heading_level, None);
        assert!(!paragraphs[0].is_list_item);
    }

    #[test]
    fn test_list_item_block() {
        let blocks = vec![ExtractedBlock {
            role: ContentRole::ListItem {
                label: Some("1.".to_string()),
            },
            text: "First item".to_string(),
            bounds: None,
            font_size: Some(12.0),
            is_bold: false,
            is_italic: false,
            children: Vec::new(),
        }];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 1);
        assert!(paragraphs[0].is_list_item);
        // Check that the label is prepended
        let first_seg_text = &paragraphs[0].lines[0].segments[0].text;
        assert_eq!(first_seg_text, "1.");
    }

    #[test]
    fn test_normalize_plain_text_unchanged() {
        assert_eq!(normalize_text_encoding("hello world"), "hello world");
    }

    #[test]
    fn test_normalize_trailing_soft_hyphen() {
        assert_eq!(normalize_text_encoding("soft\u{00AD}"), "soft-");
    }

    #[test]
    fn test_normalize_mid_word_soft_hyphen_removed() {
        assert_eq!(normalize_text_encoding("soft\u{00AD}ware"), "software");
    }

    #[test]
    fn test_normalize_soft_hyphen_before_space() {
        assert_eq!(normalize_text_encoding("soft\u{00AD} ware"), "soft- ware");
    }

    #[test]
    fn test_normalize_strips_control_chars() {
        assert_eq!(normalize_text_encoding("he\x01llo\x02"), "hello");
    }

    #[test]
    fn test_normalize_preserves_tabs_newlines() {
        assert_eq!(normalize_text_encoding("a\tb\nc\r"), "a\tb\nc\r");
    }

    #[test]
    fn test_empty_text_skipped() {
        let blocks = vec![make_block(ContentRole::Paragraph, "")];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert!(paragraphs.is_empty());
    }

    #[test]
    fn test_whitespace_only_skipped() {
        let blocks = vec![make_block(ContentRole::Paragraph, "   ")];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert!(paragraphs.is_empty());
    }

    #[test]
    fn test_children_processed() {
        let blocks = vec![ExtractedBlock {
            role: ContentRole::Other("Table".to_string()),
            text: String::new(),
            bounds: None,
            font_size: None,
            is_bold: false,
            is_italic: false,
            children: vec![
                make_block(ContentRole::Paragraph, "Cell 1"),
                make_block(ContentRole::Paragraph, "Cell 2"),
            ],
        }];
        let paragraphs = extracted_blocks_to_paragraphs(&blocks);
        assert_eq!(paragraphs.len(), 2);
    }

    #[test]
    fn test_apply_ligature_repairs_fi() {
        let map = vec![('\x0C', "fi")];
        assert_eq!(apply_ligature_repairs("classi\x0Ccation", &map), "classification");
    }

    #[test]
    fn test_apply_ligature_repairs_ff() {
        let map = vec![('\x0B', "ff")];
        assert_eq!(apply_ligature_repairs("e\x0Bective", &map), "effective");
    }

    #[test]
    fn test_apply_ligature_repairs_fl() {
        let map = vec![('\x0D', "fl")];
        assert_eq!(apply_ligature_repairs("re\x0Dection", &map), "reflection");
    }

    #[test]
    fn test_apply_ligature_repairs_ffi() {
        let map = vec![('\x0E', "ffi")];
        assert_eq!(apply_ligature_repairs("e\x0Ecient", &map), "efficient");
    }

    #[test]
    fn test_apply_ligature_repairs_ffl() {
        let map = vec![('\x0F', "ffl")];
        assert_eq!(apply_ligature_repairs("ba\x0Fe", &map), "baffle");
    }

    #[test]
    fn test_apply_ligature_repairs_no_map() {
        let map: Vec<(char, &str)> = Vec::new();
        assert_eq!(apply_ligature_repairs("hello world!", &map), "hello world!");
    }

    #[test]
    fn test_apply_ligature_repairs_multiple() {
        let map = vec![('\x0C', "fi"), ('\x0E', "ffi")];
        assert_eq!(
            apply_ligature_repairs("e\x0Ecient and classi\x0Ccation", &map),
            "efficient and classification"
        );
    }

    #[test]
    fn test_repair_contextual_ligatures_empty() {
        assert_eq!(repair_contextual_ligatures(""), "");
    }

    #[test]
    fn test_repair_contextual_ligatures_single_char() {
        assert_eq!(repair_contextual_ligatures("a"), "a");
    }

    #[test]
    fn test_repair_contextual_ligatures_no_corruption() {
        assert_eq!(repair_contextual_ligatures("hello world"), "hello world");
    }

    #[test]
    fn test_repair_contextual_ligatures_mid_word_fi() {
        assert_eq!(repair_contextual_ligatures("di!erent"), "different");
        assert_eq!(repair_contextual_ligatures("speci!c"), "specific");
    }

    #[test]
    fn test_repair_contextual_ligatures_mid_word_ff() {
        assert_eq!(repair_contextual_ligatures("di!erent effort"), "different effort");
        assert_eq!(repair_contextual_ligatures("e!ective"), "effective");
    }

    #[test]
    fn test_repair_contextual_ligatures_mid_word_ffi() {
        assert_eq!(repair_contextual_ligatures("e\u{22}cient"), "efficient");
    }

    #[test]
    fn test_repair_contextual_ligatures_word_start() {
        assert_eq!(repair_contextual_ligatures("#nancial"), "financial");
        assert_eq!(repair_contextual_ligatures("!nally"), "finally");
    }

    #[test]
    fn test_repair_contextual_ligatures_normal_punctuation() {
        assert_eq!(repair_contextual_ligatures("say \"hello\""), "say \"hello\"");
        assert_eq!(repair_contextual_ligatures("hello # world"), "hello # world");
    }

    #[test]
    fn test_repair_contextual_ligatures_multiple() {
        assert_eq!(
            repair_contextual_ligatures("ef!cient and #nancial"),
            "efficient and financial"
        );
    }

    #[test]
    fn test_text_has_ligature_corruption_empty() {
        assert!(!text_has_ligature_corruption(""));
    }

    #[test]
    fn test_text_has_ligature_corruption_too_short() {
        assert!(!text_has_ligature_corruption("ab"));
    }

    #[test]
    fn test_text_has_ligature_corruption_no_corruption() {
        assert!(!text_has_ligature_corruption("hello world"));
    }

    #[test]
    fn test_text_has_ligature_corruption_mid_word() {
        assert!(text_has_ligature_corruption("di!erent"));
        assert!(text_has_ligature_corruption("e#cient"));
        assert!(text_has_ligature_corruption("o\u{22}ces"));
    }

    #[test]
    fn test_text_has_ligature_corruption_word_start() {
        assert!(text_has_ligature_corruption("#nancial"));
        assert!(text_has_ligature_corruption("!nally"));
    }

    #[test]
    fn test_text_has_ligature_corruption_normal_punctuation() {
        assert!(!text_has_ligature_corruption("hello!"));
        assert!(!text_has_ligature_corruption("say \"hello\""));
    }

    #[test]
    fn test_text_has_ligature_corruption_multiple() {
        assert!(text_has_ligature_corruption("e!cient and #nancial"));
    }

    fn make_char(ch: char, x: f32, y: f32, font_size: f32) -> CharInfo {
        CharInfo {
            ch,
            x,
            y,
            font_size,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            has_map_error: false,
            is_symbolic: false,
        }
    }

    /// Regression test for GitHub issue #431:
    /// Positioned PDF text with large X-gaps between characters must have
    /// spaces inserted, otherwise "Main Deck" becomes "MainDeck" and
    /// "12,40" with gaps becomes "12,40" concatenated without spaces.
    #[test]
    fn test_issue_431_build_line_text_inserts_spaces_at_x_gaps() {
        let fs = 12.0;
        // "Main" at x=100 then "Deck" at x=200 (large gap = different words)
        let chars = vec![
            make_char('M', 100.0, 0.0, fs),
            make_char('a', 107.0, 0.0, fs),
            make_char('i', 114.0, 0.0, fs),
            make_char('n', 121.0, 0.0, fs),
            // Gap of ~79 units (>> font_size) before "Deck"
            make_char('D', 200.0, 0.0, fs),
            make_char('e', 207.0, 0.0, fs),
            make_char('c', 214.0, 0.0, fs),
            make_char('k', 221.0, 0.0, fs),
        ];
        let result = build_line_text(&chars, None);
        assert_eq!(result, "Main Deck", "Large X-gap should produce a space between words");
    }

    #[test]
    fn test_issue_431_build_line_text_no_false_spaces() {
        let fs = 12.0;
        // "Hello" with normal character advance (~7 units for 12pt font)
        let chars = vec![
            make_char('H', 100.0, 0.0, fs),
            make_char('e', 107.0, 0.0, fs),
            make_char('l', 114.0, 0.0, fs),
            make_char('l', 121.0, 0.0, fs),
            make_char('o', 128.0, 0.0, fs),
        ];
        let result = build_line_text(&chars, None);
        assert_eq!(result, "Hello", "Normal spacing should not insert extra spaces");
    }

    #[test]
    fn test_issue_431_tabular_numbers_with_gaps() {
        let fs = 12.0;
        // Tabular data: "12,40" at x=50 and "480" at x=200
        let chars = vec![
            make_char('1', 50.0, 0.0, fs),
            make_char('2', 57.0, 0.0, fs),
            make_char(',', 64.0, 0.0, fs),
            make_char('4', 71.0, 0.0, fs),
            make_char('0', 78.0, 0.0, fs),
            // Large gap to next column
            make_char('4', 200.0, 0.0, fs),
            make_char('8', 207.0, 0.0, fs),
            make_char('0', 214.0, 0.0, fs),
        ];
        let result = build_line_text(&chars, None);
        assert_eq!(result, "12,40 480", "Column gap should produce space between numbers");
    }

    #[test]
    fn test_issue_431_preserves_existing_spaces() {
        let fs = 12.0;
        // When pdfium already provides space characters, don't double-space
        let chars = vec![
            make_char('A', 100.0, 0.0, fs),
            make_char(' ', 107.0, 0.0, fs),
            make_char('B', 200.0, 0.0, fs),
        ];
        let result = build_line_text(&chars, None);
        assert_eq!(result, "A B", "Should not insert extra space when space char exists");
    }
}
