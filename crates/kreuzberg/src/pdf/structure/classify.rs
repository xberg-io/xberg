//! Heading classification for paragraphs using font-size clustering.

use super::constants::{MAX_BOLD_HEADING_WORD_COUNT, MAX_HEADING_DISTANCE_MULTIPLIER, MAX_HEADING_WORD_COUNT};
use super::regions::looks_like_figure_label;
use super::types::PdfParagraph;

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
    // Body font size = centroid of the cluster with no heading level
    let body_font_size = heading_map
        .iter()
        .find(|(_, level)| level.is_none())
        .map(|(centroid, _)| *centroid)
        .unwrap_or(0.0);
    for para in paragraphs.iter_mut() {
        let word_count: usize = if !para.text.is_empty() {
            para.text.split_whitespace().count()
        } else {
            para.lines
                .iter()
                .flat_map(|l| l.segments.iter())
                .map(|s| s.text.split_whitespace().count())
                .sum()
        };

        // Pass 1: font-size-based heading classification.
        // When the layout model says Text, only promote to heading if there is
        // strong typographic evidence: the font size maps to a heading cluster
        // AND the paragraph is bold (sub-headings the model missed).
        let layout_says_text = para.layout_class == Some(super::types::LayoutHintClass::Text);
        let heading_level = find_heading_level(para.dominant_font_size, heading_map, &gap_info);
        let heading_level = if layout_says_text {
            // Override layout Text only when bold + heading font size → sub-heading
            if para.is_bold && heading_level.is_some() {
                heading_level
            } else {
                None
            }
        } else {
            heading_level
        };

        // Helper: get paragraph text from either full-text path or segment path.
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
        {
            para.heading_level = Some(level);
            continue;
        }

        // Pass 2: bold or italic short paragraphs → section headings (H2).
        let is_italic = if !para.text.is_empty() {
            // Full-text path: check the is_italic flag on the paragraph (from char API majority)
            para.lines
                .first()
                .and_then(|l| l.segments.first())
                .is_some_and(|s| s.is_italic)
        } else {
            !para.lines.is_empty() && para.lines.iter().all(|l| l.segments.iter().all(|s| s.is_italic))
        };
        let layout_text_overridable = if layout_says_text {
            body_font_size > 0.0 && para.dominant_font_size > body_font_size + 0.5
        } else {
            true
        };
        if (para.is_bold || is_italic)
            && !para.is_list_item
            && layout_text_overridable
            && word_count <= MAX_BOLD_HEADING_WORD_COUNT
        {
            let t = para_text.trim();
            // Italic-only paragraphs need extra guards: academic papers use italic
            // for author names, affiliations, emails which shouldn't be headings.
            let italic_ok = if is_italic && !para.is_bold {
                !t.contains('@') && !t.contains(',') && t.chars().next().is_some_and(|c| c.is_uppercase())
            } else {
                true
            };
            // Guard: very short text (1-2 words) at body font size is typically a
            // figure label (e.g., "Untightened nut"), not a real heading.
            let too_short_at_body =
                word_count <= 2 && body_font_size > 0.0 && para.dominant_font_size <= body_font_size + 0.5;
            let period_ok = !t.ends_with('.') || is_section_pattern(t);
            // Allow colon-ending for ALL-CAPS label-headings like "AGENCY:" or "ACTION:"
            let colon_ok = !t.ends_with(':') || is_all_caps_text(t);
            if italic_ok
                && !too_short_at_body
                && period_ok
                && colon_ok
                && !looks_like_figure_label(t)
                && !super::layout_classify::is_separator_text(t)
            {
                // Use font-size ratio to body to differentiate H2 vs H3:
                // - font > 1.2× body → H2 (clearly larger sub-heading)
                // - font > body (bold at body size) → H3 (same-size bold sub-heading)
                // - section numbering also influences level via infer_section_level
                let level = infer_bold_heading_level(para.dominant_font_size, body_font_size, t);
                para.heading_level = Some(level);
            }
        }

        // Pass 2.5: section numbering pattern detection.
        // Short paragraphs starting with section numbers (1., 2.1, I., A.) are
        // likely headings even if not bold — common in academic papers.
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
                // Only promote if font size is at or above body size (avoid promoting
                // footnotes or captions that happen to start with numbers).
                let at_or_above_body = body_font_size <= 0.0 || para.dominant_font_size >= body_font_size - 0.5;
                // Guard against layout model saying this is body text
                let layout_ok = !layout_says_text
                    || (body_font_size > 0.0 && para.dominant_font_size > body_font_size + 0.5)
                    || para.is_bold;
                if at_or_above_body && layout_ok {
                    let level = infer_section_level(t);
                    para.heading_level = Some(level);
                }
            }
        }

        // Pass 3: code blocks should never be headings
        if para.is_code_block {
            para.heading_level = None;
        }

        // Pass 3.5: heuristic formula detection.
        // Short paragraphs with high density of math characters (Greek letters,
        // math operators, set theory symbols) are likely formulas.
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

        // Pass 4: rescue pass — promote short, large-font paragraphs to headings.
        // Catches headings that were missed by Passes 1-2.5 (e.g., non-bold headings
        // at a font size that didn't form its own cluster but is clearly larger than body).
        if para.heading_level.is_none()
            && !para.is_list_item
            && !para.is_code_block
            && !para.is_page_furniture
            && body_font_size > 0.0
            && para.dominant_font_size >= body_font_size + 0.5
        {
            let rescue_text = para_text.trim();
            let rescue_wc = rescue_text.split_whitespace().count();
            let rescue_colon_ok = !rescue_text.ends_with(':') || is_all_caps_text(rescue_text);
            if (1..=8).contains(&rescue_wc)
                && !rescue_text.ends_with('.')
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

    // Pass 5: demote rescued headings that follow non-terminated paragraphs.
    // If the previous paragraph ends without sentence-ending punctuation, the
    // current paragraph is likely a continuation fragment (e.g., from a column
    // break), not a real heading.
    demote_continuation_headings(paragraphs);

    // Pass 6: ALL-CAPS heading detection.
    // Short ALL-CAPS paragraphs (<=15 words, >80% uppercase letters) are often
    // headings in government/legal documents (e.g., "DEPARTMENT OF TRANSPORTATION",
    // "AGENCY:", "SUMMARY:"). Promote if not already a heading/list/code.
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

        // Skip single words that are just abbreviations (e.g., "USA")
        // unless they end with colon (label-heading like "AGENCY:")
        if wc == 1 && !t.ends_with(':') {
            continue;
        }

        if !is_all_caps_text(t) {
            continue;
        }

        // Skip if it looks like a figure label or separator
        if looks_like_figure_label(t) || super::layout_classify::is_separator_text(t) {
            continue;
        }

        // Determine heading level from font-size ratio to body
        let level = if body_font_size > 0.0 {
            let ratio = para.dominant_font_size / body_font_size;
            if ratio > 1.4 {
                1
            } else if ratio > 1.2 {
                2
            } else if wc <= 5 {
                2 // Short ALL-CAPS title → H2
            } else {
                3 // Longer ALL-CAPS label → H3
            }
        } else if wc <= 5 {
            2
        } else {
            3
        };
        para.heading_level = Some(level);
    }
}

/// Check if text starts with a lowercase letter or a common sentence-continuation word.
///
/// Mid-sentence fragments from column breaks often start with lowercase words or
/// common continuation words. Real headings typically start with uppercase letters,
/// numbers, or section markers.
fn starts_with_lowercase_or_continuation(text: &str) -> bool {
    let first_char = text.chars().next();
    // If the first character is a lowercase letter, it's likely a sentence continuation.
    if first_char.is_some_and(|c| c.is_lowercase()) {
        return true;
    }

    // Check for common continuation words that indicate a sentence fragment.
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
        // Only consider paragraphs that have a heading level
        if paragraphs[i].heading_level.is_none() {
            continue;
        }

        // Skip if the previous paragraph is a heading, list item, code, or formula
        // (those are structurally distinct and don't indicate continuation).
        let prev = &paragraphs[i - 1];
        if prev.heading_level.is_some()
            || prev.is_list_item
            || prev.is_code_block
            || prev.is_formula
            || prev.is_page_furniture
        {
            continue;
        }

        // Check if the previous paragraph ends without sentence-ending punctuation.
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
/// 1. Promotes the first heading to H1 when no H1 exists (title inference).
/// 2. Merges consecutive H1 headings at the same font size into one title (any page).
/// 3. Demotes numbered section headings from H1 to H2 when a non-numbered title H1 exists.
pub(super) fn refine_heading_hierarchy(all_pages: &mut [Vec<PdfParagraph>]) {
    // Step 0: If no H1 exists but H2+ headings do, promote the first heading
    // on page 0 (or the largest font heading) to H1. Many docs have a title
    // that gets classified as H2 by bold promotion or font clustering.
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
        if has_any_heading {
            promote_title_heading(all_pages);
        }

        // Additional title promotion: if still no H1 exists, check if the first
        // paragraph on page 0 has the largest font size on that page AND is short
        // (<=10 words). This catches unclassified title paragraphs at the top of
        // documents that were missed by font-size clustering and bold heuristics.
        let still_no_h1 = !all_pages
            .iter()
            .flat_map(|page| page.iter())
            .any(|p| p.heading_level == Some(1));
        if still_no_h1 && !all_pages.is_empty() && !all_pages[0].is_empty() {
            let page0 = &all_pages[0];
            let max_font_on_page = page0.iter().map(|p| p.dominant_font_size).fold(0.0f32, f32::max);
            let first = &page0[0];
            let first_text = paragraph_plain_text(first);
            let first_wc = first_text.split_whitespace().count();
            if first.dominant_font_size >= max_font_on_page
                && first_wc <= 10
                && first_wc > 0
                && !first.is_page_furniture
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

    // Step 1: Merge consecutive H1s at the same font size on each page.
    // Split titles like "KAISUN HOLDINGS" / "LIMITED" appear as consecutive
    // H1 paragraphs with the same font size.
    for page in all_pages.iter_mut() {
        merge_consecutive_h1s(page);
    }

    // Re-count after merging
    let h1_count: usize = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .filter(|p| p.heading_level == Some(1))
        .count();

    if h1_count <= 1 {
        return;
    }

    // Step 2: Demote numbered section headings.
    // If the first H1 is a title (not starting with a number), demote subsequent
    // numbered H1s to H2.
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

/// Determine heading level for a bold/italic paragraph based on font-size ratio to body.
///
/// - Font size > 1.2× body → H2 (clearly larger sub-heading)
/// - Font size at body size but bold → H3 (same-size bold sub-heading)
/// - Section numbering overrides: uses dot count for depth (e.g., "3.2" → H3)
fn infer_bold_heading_level(font_size: f32, body_font_size: f32, text: &str) -> u8 {
    // If text starts with a section number, use numbering depth
    if starts_with_section_number(text) {
        return infer_section_level(text);
    }

    // Use font-size ratio to differentiate H2 vs H3
    if body_font_size > 0.0 {
        let ratio = font_size / body_font_size;
        if ratio > 1.2 {
            return 2; // Clearly larger than body → H2
        }
        // Bold at roughly body size → H3 (sub-section heading)
        return 3;
    }

    // No body font size info, default to H2
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

    // Check for alphabetic prefix: "A.", "B.1"
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
        // Check for roman numerals
        let roman_chars: &[u8] = b"IVXLCDM";
        let bytes = trimmed.as_bytes();
        let roman_end = bytes.iter().position(|b| !roman_chars.contains(b)).unwrap_or(0);
        if roman_end > 0 && roman_end <= 5 && roman_end < bytes.len() {
            let next = bytes[roman_end];
            if (next == b'.' || next == b' ' || next == b')') && is_valid_roman(&trimmed[..roman_end]) {
                // Roman numeral → top-level (H2), no sub-numbering
                return 2;
            }
        }
        // Numeric prefix: "3.2.1"
        trimmed.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(0)
    };

    if numbering_end == 0 {
        return 2; // No numbering, default H2
    }

    let numbering = &trimmed[..numbering_end];
    let dot_count = numbering.chars().filter(|&c| c == '.').count();
    let effective_dots = if numbering.ends_with('.') {
        dot_count.saturating_sub(1)
    } else {
        dot_count
    };

    match effective_dots {
        0 => 2, // "1 Introduction" → H2
        1 => 3, // "3.2 AI models" → H3
        _ => 4, // "3.2.1 Details" → H4
    }
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
    // All-caps short text ending with period is likely a structural heading
    let words = t.split_whitespace().count();
    if words <= 6 && t.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase()) {
        return true;
    }
    // Starts with section number: "3.2. Methods" or "162-56. Place"
    starts_with_section_number(t)
}

/// Check if text starts with a section number pattern (e.g., "1 ", "2.1 ", "A.", "III.").
pub(super) fn starts_with_section_number(text: &str) -> bool {
    let trimmed = text.trim();
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    // Arabic digits: "1 ", "2.1 ", "3.2. "
    let digit_end = bytes.iter().position(|&b| !b.is_ascii_digit()).unwrap_or(0);
    if digit_end > 0 && digit_end < bytes.len() {
        let next = bytes[digit_end];
        if next == b' ' || next == b'.' || next == b')' {
            return true;
        }
    }
    // Roman numerals: "I.", "II.", "III.", "IV.", "V.", "VI.", "VII.", "VIII.", "IX.", "X."
    // Also "I " without dot (e.g., "I INTRODUCTION")
    let roman_chars: &[u8] = b"IVXLCDM";
    let roman_end = bytes.iter().position(|b| !roman_chars.contains(b)).unwrap_or(0);
    if roman_end > 0 && roman_end <= 5 && roman_end < bytes.len() {
        let next = bytes[roman_end];
        if next == b'.' || next == b' ' || next == b')' {
            // Verify it's a valid roman numeral (not just random letters like "INVALID")
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
    // Collect all H2 headings with their position and numbered status
    let mut h2_info: Vec<(usize, usize, bool)> = Vec::new(); // (page_idx, para_idx, is_numbered)
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
        return; // Not enough numbered sections to establish a pattern
    }

    // Find ranges: between consecutive numbered H2s, demote unnumbered H2s to H3
    let numbered_positions: Vec<usize> = h2_info
        .iter()
        .enumerate()
        .filter(|(_, (_, _, numbered))| *numbered)
        .map(|(idx, _)| idx)
        .collect();

    for window in numbered_positions.windows(2) {
        let start = window[0];
        let end = window[1];
        // Demote unnumbered H2s between these two numbered H2s
        for &(page_idx, para_idx, is_numbered) in &h2_info[start + 1..end] {
            if !is_numbered {
                // Don't demote headings confirmed by the layout model.
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

            // Find the end of this consecutive same-level heading run
            let mut run_end = run_start + 1;
            while run_end < page.len() && page[run_end].heading_level == Some(level) {
                run_end += 1;
            }

            let run_len = run_end - run_start;
            if run_len > MAX_CONSECUTIVE {
                // Demote all but the first heading in the run, unless
                // the layout model confirmed it as a heading region.
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

/// Extract plain text from a paragraph.
fn paragraph_plain_text(para: &PdfParagraph) -> String {
    para.lines
        .iter()
        .flat_map(|l| l.segments.iter())
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Promote the most likely title heading to H1 when no H1 exists.
///
/// Strategy:
/// 1. If any heading has layout_class == Title, promote it to H1.
/// 2. Otherwise, on the first page only, promote the heading with the largest
///    font size IF it's clearly larger than other headings (at least 1.5pt gap).
fn promote_title_heading(all_pages: &mut [Vec<PdfParagraph>]) {
    // First check for layout-confirmed Title
    for page in all_pages.iter_mut() {
        for para in page.iter_mut() {
            if para.heading_level.is_some() && para.layout_class == Some(super::types::LayoutHintClass::Title) {
                para.heading_level = Some(1);
                return;
            }
        }
    }

    // Only promote on the first page, and only if one heading is clearly
    // larger than others (to avoid false promotion when all headings are same size)
    if all_pages.is_empty() {
        return;
    }
    let page = &all_pages[0];
    let headings: Vec<(usize, f32)> = page
        .iter()
        .enumerate()
        .filter(|(_, p)| p.heading_level.is_some())
        .map(|(i, p)| (i, p.dominant_font_size))
        .collect();

    if headings.is_empty() {
        return;
    }

    // If there's exactly one heading on page 0, promote it
    if headings.len() == 1 {
        all_pages[0][headings[0].0].heading_level = Some(1);
        return;
    }

    // If multiple headings, only promote if the largest is clearly bigger (1.5pt gap)
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

/// Merge consecutive H1 paragraphs at the same font size into a single heading.
///
/// Split titles (e.g., "KAISUN HOLDINGS" on one line, "LIMITED" on the next)
/// often produce separate H1 paragraphs. When they share the same font size
/// they should be a single heading.
fn merge_consecutive_h1s(page: &mut Vec<PdfParagraph>) {
    let mut i = 0;
    while i < page.len() {
        if page[i].heading_level != Some(1) {
            i += 1;
            continue;
        }
        // Find the run of consecutive H1s at the same font size.
        let base_fs = page[i].dominant_font_size;
        let mut run_end = i + 1;
        while run_end < page.len()
            && page[run_end].heading_level == Some(1)
            && (page[run_end].dominant_font_size - base_fs).abs() < 0.5
        {
            run_end += 1;
        }
        if run_end - i > 1 {
            // Merge lines from paragraphs [i+1..run_end] into page[i].
            let mut merged_lines = std::mem::take(&mut page[i].lines);
            for para in &page[i + 1..run_end] {
                merged_lines.extend(para.lines.clone());
            }
            page[i].lines = merged_lines;
            page.drain(i + 1..run_end);
        }
        i += 1;
    }
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

    let margin_frac = 0.10; // Top/bottom 10% of page = margin zone

    // Count how many pages each margin text appears on (fuzzy: alphanum-only).
    let mut text_page_count: ahash::AHashMap<String, usize> = ahash::AHashMap::new();
    let mut alphanum_to_exact: ahash::AHashMap<String, ahash::AHashSet<String>> = ahash::AHashMap::new();

    for (page_idx, page) in all_pages.iter().enumerate() {
        let page_h = page_heights.get(page_idx).copied().unwrap_or(792.0);
        let top_margin_y = page_h * (1.0 - margin_frac); // PDF coords: top is high
        let bottom_margin_y = page_h * margin_frac;

        let mut seen: ahash::AHashSet<String> = ahash::AHashSet::new();
        for para in page {
            if para.is_page_furniture {
                continue;
            }

            // Only consider paragraphs in the margins.
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
                *text_page_count.entry(alphanum_key).or_insert(0) += 1;
            }
        }
    }

    let threshold = all_pages.len() / 2;

    // Collect text variants that repeat in margins on >50% of pages.
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

    // Mark matching paragraphs as furniture — only in margins.
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
        // Math operators and symbols
        '\u{2200}' // ∀
        | '\u{2203}' // ∃
        | '\u{2208}' // ∈
        | '\u{2209}' // ∉
        | '\u{2282}' // ⊂
        | '\u{2283}' // ⊃
        | '\u{222A}' // ∪
        | '\u{2229}' // ∩
        | '\u{2211}' // ∑
        | '\u{222B}' // ∫
        | '\u{220F}' // ∏
        | '\u{2202}' // ∂
        | '\u{2207}' // ∇
        | '\u{2264}' // ≤
        | '\u{2265}' // ≥
        | '\u{2260}' // ≠
        | '\u{2248}' // ≈
        | '\u{00B1}' // ±
        | '\u{221E}' // ∞
        | '\u{221A}' // √
        | '\u{2192}' // →
        | '\u{2190}' // ←
        | '\u{2194}' // ↔
        | '\u{21D2}' // ⇒
        | '\u{21D0}' // ⇐
        | '\u{27E8}' // ⟨
        | '\u{27E9}' // ⟩
        | '\u{00D7}' // ×
        | '\u{00F7}' // ÷
    ) || is_greek_letter(c)
}

/// Check if a character is a Greek letter (lowercase α-ω or uppercase Α-Ω).
fn is_greek_letter(c: char) -> bool {
    // Greek and Coptic block: U+0370–U+03FF
    // Lowercase: α (U+03B1) to ω (U+03C9)
    // Uppercase: Α (U+0391) to Ω (U+03A9)
    matches!(c, '\u{0391}'..='\u{03A9}' | '\u{03B1}'..='\u{03C9}')
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
            })
            .collect();

        PdfParagraph {
            text: String::new(),
            lines: vec![super::super::types::PdfLine {
                segments,
                baseline_y: 700.0,
                dominant_font_size: font_size,
                is_bold: false,
                is_monospace: false,
            }],
            dominant_font_size: font_size,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        }
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
        let mut paragraphs = vec![make_paragraph(18.0, 21)]; // > MAX_HEADING_WORD_COUNT
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
        // Font size 50.0 is way too far from any centroid
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
        let heading_map = vec![(12.0, None)]; // no heading clusters
        let mut para = make_paragraph(12.0, 3);
        para.is_bold = true;
        para.lines[0].is_bold = true;
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        // Bold at body font size → H3 (sub-section heading)
        assert_eq!(paragraphs[0].heading_level, Some(3));
    }

    #[test]
    fn test_classify_bold_long_paragraph_not_promoted() {
        let heading_map = vec![(12.0, None)];
        let mut para = make_paragraph(12.0, 20); // too many words
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
        // 3 segments but each contains many words — total word count exceeds threshold
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
            })
            .collect();

        let mut paragraphs = vec![PdfParagraph {
            text: String::new(),
            lines: vec![super::super::types::PdfLine {
                segments,
                baseline_y: 700.0,
                dominant_font_size: 18.0,
                is_bold: false,
                is_monospace: false,
            }],
            dominant_font_size: 18.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        }];
        // 3 segments × 7 words = 21 words > MAX_HEADING_WORD_COUNT
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

    #[test]
    fn test_merge_consecutive_h1s_same_font() {
        let mut page = vec![
            make_h1(24.0, "KAISUN HOLDINGS"),
            make_h1(24.0, "LIMITED"),
            make_paragraph(12.0, 3), // body
        ];
        merge_consecutive_h1s(&mut page);
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].heading_level, Some(1));
        assert_eq!(page[0].lines.len(), 2);
    }

    #[test]
    fn test_merge_h1s_different_font_no_merge() {
        let mut page = vec![make_h1(24.0, "Title"), make_h1(18.0, "Subtitle")];
        merge_consecutive_h1s(&mut page);
        assert_eq!(page.len(), 2); // Not merged — different font sizes.
    }

    /// Create a paragraph with bbox in the top margin of a 792pt page.
    fn make_margin_body(text: &str) -> PdfParagraph {
        let mut p = make_paragraph(12.0, 1);
        p.lines[0].segments[0].text = text.to_string();
        // Place in top margin: y > 792 * 0.90 = 712.8
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
        // "Page 1 of 10" appears in the margin on all 4 pages (>50%) → furniture
        assert!(pages[0][0].is_page_furniture);
        assert!(!pages[0][1].is_page_furniture);
    }

    #[test]
    fn test_cross_page_repeating_marks_repeated_headings_as_furniture() {
        let page_heights = vec![792.0; 6];
        let mut pages = vec![];
        for _ in 0..6 {
            let mut h = make_h1(24.0, "Chapter");
            h.heading_level = Some(1);
            // Place heading in top margin
            h.block_bbox = Some((50.0, 740.0, 300.0, 770.0));
            let mut body = make_paragraph(12.0, 3);
            body.block_bbox = Some((50.0, 400.0, 300.0, 420.0));
            pages.push(vec![h, body]);
        }
        mark_cross_page_repeating_text(&mut pages, &page_heights);
        // Headings repeating in margins on >50% of pages → furniture.
        assert!(pages[0][0].is_page_furniture);
        assert!(pages[0][0].heading_level.is_none());
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
            pages[0][0].is_page_furniture,
            "even-page copyright variant should be furniture"
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
        // Sub-heading: layout model says Text, but bold + heading font size → promote
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
        // Layout says Text, heading font size but NOT bold → stay as body
        let heading_map = vec![(16.0, Some(2)), (12.0, None)];
        let mut para = make_paragraph(16.0, 3);
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_layout_text_bold_body_font_not_promoted_pass1() {
        // Layout says Text, bold but body font size → Pass 1 skips (no heading cluster match)
        let heading_map = vec![(16.0, Some(2)), (12.0, None)];
        let mut para = make_paragraph(12.0, 3);
        para.is_bold = true;
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        // Pass 1 won't promote (body font), Pass 2 won't either (body font <= body + 0.5)
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_layout_text_bold_larger_font_promoted_pass2() {
        // Layout says Text, bold + font distinctly larger than body → Pass 2 promotes
        // 14pt / 12pt body = 1.167 ratio (< 1.2) → H3 (sub-section bold heading)
        let heading_map = vec![(12.0, None)]; // no heading clusters
        let mut para = make_paragraph(14.0, 3);
        para.is_bold = true;
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(3));
    }

    #[test]
    fn test_layout_text_bold_much_larger_font_promoted_h2() {
        // Layout says Text, bold + font much larger than body → H2
        // 15pt / 12pt body = 1.25 ratio (> 1.2) → H2
        let heading_map = vec![(12.0, None)]; // no heading clusters
        let mut para = make_paragraph(15.0, 3);
        para.is_bold = true;
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_infer_bold_heading_level_large_ratio_h2() {
        // 15pt / 12pt = 1.25 → H2
        assert_eq!(infer_bold_heading_level(15.0, 12.0, "Methods"), 2);
    }

    #[test]
    fn test_infer_bold_heading_level_body_size_h3() {
        // 12pt / 12pt = 1.0 → H3
        assert_eq!(infer_bold_heading_level(12.0, 12.0, "Methods"), 3);
    }

    #[test]
    fn test_infer_bold_heading_level_numbered_section() {
        // Bold numbered section uses numbering depth
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
        }];
        PdfParagraph {
            text: String::new(),
            lines: vec![super::super::types::PdfLine {
                segments,
                baseline_y: 700.0,
                dominant_font_size: font_size,
                is_bold,
                is_monospace: false,
            }],
            dominant_font_size: font_size,
            heading_level: None,
            is_bold,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        }
    }

    #[test]
    fn test_section_numbering_promotes_non_bold_heading() {
        // Non-bold paragraph starting with section number at body font size
        // gets promoted via Pass 2.5 (no layout class = no blocking)
        let heading_map = vec![(12.0, None)]; // only body cluster
        let mut paragraphs = vec![make_text_paragraph(12.0, "3 Processing pipeline", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_section_numbering_blocked_by_layout_text() {
        // Layout model says Text + non-bold + body font size → Pass 2.5 blocks promotion
        let heading_map = vec![(12.0, None)];
        let mut para = make_text_paragraph(12.0, "3 Processing pipeline", false);
        para.layout_class = Some(super::super::types::LayoutHintClass::Text);
        let mut paragraphs = vec![para];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_section_numbering_promotes_bold_at_body_size() {
        // Bold paragraph with section number at body font size
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "3 Processing pipeline", true)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        // Bold + numbered → promoted via Pass 2 with infer_bold_heading_level
        assert_eq!(paragraphs[0].heading_level, Some(2));
    }

    #[test]
    fn test_section_numbering_subsection_bold() {
        // Bold paragraph with sub-section number
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "3.2 AI models", true)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(3));
    }

    #[test]
    fn test_bold_heading_with_heading_cluster_keeps_cluster_level() {
        // When font-size clustering assigns H1, bold text should keep it
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(18.0, "Title", true)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, Some(1));
    }

    #[test]
    fn test_formula_detection_math_symbols() {
        // Paragraph with several math characters should be detected as formula
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "∑ x∈S f(x) ≤ ∞", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert!(paragraphs[0].is_formula, "should detect formula with math symbols");
        assert_eq!(paragraphs[0].heading_level, None);
    }

    #[test]
    fn test_formula_detection_greek_letters() {
        // Paragraph with Greek letters at high density
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "α + β = γ", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert!(paragraphs[0].is_formula, "should detect formula with Greek letters");
    }

    #[test]
    fn test_formula_detection_not_regular_text() {
        // Regular paragraph text should NOT be marked as formula
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
        // A heading with math characters should stay a heading, not become formula
        let heading_map = vec![(18.0, Some(1)), (12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(18.0, "∑ Results", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        // Should be heading, not formula (heading takes precedence)
        assert_eq!(paragraphs[0].heading_level, Some(1));
    }

    #[test]
    fn test_formula_detection_high_density() {
        // Even with fewer than 3 math chars, 15%+ density triggers formula
        let heading_map = vec![(12.0, None)];
        // "x→y" has 3 chars total, 1 math char → 33% > 15%
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
        assert!(!is_math_character('+'));
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

    // ── Pass 4: rescue pass tests ──

    #[test]
    fn test_rescue_pass_promotes_large_font_short_paragraph() {
        // Non-bold, non-heading-cluster paragraph at 15pt (body=12pt) with 3 words
        // should be rescued as a heading via Pass 4.
        let heading_map = vec![(12.0, None)]; // only body cluster
        let mut paragraphs = vec![make_text_paragraph(15.0, "Results and Discussion", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        // 15/12 = 1.25, > 1.0 but <= 1.3 -> H3
        assert_eq!(paragraphs[0].heading_level, Some(3));
    }

    #[test]
    fn test_rescue_pass_h1_for_very_large_font() {
        // Font size > 1.6x body -> H1
        let heading_map = vec![(10.0, None)];
        let mut paragraphs = vec![make_text_paragraph(17.0, "Document Title", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        // 17/10 = 1.7 > 1.6 -> H1
        assert_eq!(paragraphs[0].heading_level, Some(1));
    }

    #[test]
    fn test_rescue_pass_h3_for_slightly_larger_font() {
        // Font slightly larger than body -> H3
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(13.0, "Methods", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        // 13/12 = 1.083, > 1.0 but <= 1.2 -> H3
        assert_eq!(paragraphs[0].heading_level, Some(3));
    }

    #[test]
    fn test_rescue_pass_skips_long_paragraphs() {
        // Paragraph with >8 words should NOT be rescued
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
        // Paragraph ending with period should NOT be rescued
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
        // Font at body size should NOT be rescued (needs > body + 0.5)
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "Methods", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(paragraphs[0].heading_level, None);
    }

    // ── Pass 4 fragment guards ──

    #[test]
    fn test_rescue_pass_skips_lowercase_start() {
        // Fragment starting with lowercase letter should NOT be promoted
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
        // Fragment starting with "The" (continuation word, even uppercase) should NOT be promoted
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
        // Proper heading starting with uppercase non-continuation word
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(15.0, "Results and Discussion", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        // 15/12 = 1.25, > 1.0 but <= 1.3 -> H3
        assert_eq!(
            paragraphs[0].heading_level,
            Some(3),
            "proper heading should still be promoted"
        );
    }

    #[test]
    fn test_demote_continuation_heading_after_unterminated_paragraph() {
        // A heading following a paragraph without sentence-ending punctuation
        // should be demoted (it's likely a column-break fragment).
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
        // A heading following a properly terminated paragraph should stay.
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![
            make_text_paragraph(12.0, "The previous section covered the basics.", false),
            make_text_paragraph(15.0, "Safety Procedures", false),
        ];
        classify_paragraphs(&mut paragraphs, &heading_map);
        // 15/12 = 1.25, > 1.0 but <= 1.3 -> H3
        assert_eq!(
            paragraphs[1].heading_level,
            Some(3),
            "heading after terminated paragraph should stay"
        );
    }

    #[test]
    fn test_keep_heading_after_another_heading() {
        // Heading after another heading should not be demoted by continuation check.
        // Use a single body cluster so the second paragraph gets promoted via rescue pass.
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![
            make_text_paragraph(15.0, "Chapter One", false),
            make_text_paragraph(14.0, "Introduction", false),
        ];
        classify_paragraphs(&mut paragraphs, &heading_map);
        // Both should be promoted by rescue pass: 15/12=1.25→H2, 14/12=1.17→H3
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
        // Lowercase start
        assert!(starts_with_lowercase_or_continuation("is necessary"));
        assert!(starts_with_lowercase_or_continuation("the quick fox"));
        // Continuation words (capitalized)
        assert!(starts_with_lowercase_or_continuation("The quick fox"));
        assert!(starts_with_lowercase_or_continuation("And furthermore"));
        assert!(starts_with_lowercase_or_continuation("But however"));
        assert!(starts_with_lowercase_or_continuation("Which means"));
        // Not continuation
        assert!(!starts_with_lowercase_or_continuation("Results"));
        assert!(!starts_with_lowercase_or_continuation("Safety Procedures"));
        assert!(!starts_with_lowercase_or_continuation("3 Methods"));
    }

    // ── refine_heading_hierarchy: enhanced title promotion tests ──

    #[test]
    fn test_refine_promotes_first_largest_font_paragraph_to_h1() {
        // First paragraph on page 0 has the largest font, short text, no heading level
        let mut pages = vec![vec![
            make_text_paragraph(18.0, "Annual Report", false),
            make_text_paragraph(12.0, "Some body text here for the document", false),
        ]];
        // No headings at all initially -- refine should promote the first paragraph
        refine_heading_hierarchy(&mut pages);
        assert_eq!(pages[0][0].heading_level, Some(1));
    }

    // ── is_all_caps_text tests ──

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
        assert!(!is_all_caps_text("a")); // too short (< 2 alpha chars)
    }

    // ── Pass 6: ALL-CAPS heading detection tests ──

    #[test]
    fn test_all_caps_short_promoted_to_h2() {
        // Short ALL-CAPS text (2+ words) at body font size → H2
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
        // Longer ALL-CAPS text (6-15 words) at body font size → H3
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
        // ALL-CAPS label ending with colon → heading
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
        // Single ALL-CAPS word without colon → abbreviation, skip
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
        // ALL-CAPS text with >15 words → not promoted
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
        // ALL-CAPS list items should not be promoted
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
        // ALL-CAPS at larger font size → higher heading level
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(18.0, "TITLE", false)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        // 18/12 = 1.5 > 1.4 → H1 (from rescue pass, actually)
        assert!(
            paragraphs[0].heading_level.is_some(),
            "large ALL-CAPS should be promoted"
        );
    }

    // ── Bold colon-ending ALL-CAPS tests ──

    #[test]
    fn test_bold_all_caps_colon_promoted() {
        // Bold ALL-CAPS text ending with colon should be promoted (colon guard relaxed)
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
        // Bold mixed-case text ending with colon should NOT be promoted (colon blocks)
        let heading_map = vec![(12.0, None)];
        let mut paragraphs = vec![make_text_paragraph(12.0, "Agency:", true)];
        classify_paragraphs(&mut paragraphs, &heading_map);
        assert_eq!(
            paragraphs[0].heading_level, None,
            "bold mixed-case with colon should not be promoted"
        );
    }
}
