//! Markdown rendering for paragraphs and lines with inline bold/italic markup.

use std::borrow::Cow;

use crate::pdf::hierarchy::SegmentData;

use super::lines::needs_space_between;
use super::types::{LayoutHintClass, PdfLine, PdfParagraph};

/// Render a single paragraph to the output string.
pub(crate) fn render_paragraph_to_output(para: &PdfParagraph, output: &mut String) {
    if let Some(level) = para.heading_level {
        let prefix = "#".repeat(level as usize);
        let joined = join_line_texts(&para.lines);
        let text = escape_html_entities(&joined);
        output.push_str(&prefix);
        output.push(' ');
        output.push_str(&text);
    } else if para.is_code_block {
        output.push_str("```\n");
        for line in &para.lines {
            let line_text = line
                .segments
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            output.push_str(&line_text);
            output.push('\n');
        }
        output.push_str("```");
    } else if para.is_formula {
        let text = join_line_texts(&para.lines);
        output.push_str("$$\n");
        output.push_str(&text);
        output.push_str("\n$$");
    } else if para.is_list_item {
        let text = render_paragraph_with_inline_markup(para);
        let normalized = normalize_list_prefix(&text);
        output.push_str(&normalized);
    } else if matches!(para.layout_class, Some(LayoutHintClass::Caption)) {
        // Captions are rendered in italic to visually distinguish them from body text.
        // Asterisks in the caption text must be escaped so they don't break the italic
        // delimiter (`*...*`) and produce malformed markdown.
        let joined = join_line_texts(&para.lines);
        let text = escape_html_entities(&joined);
        let escaped = text.replace('*', "\\*");
        output.push('*');
        output.push_str(&escaped);
        output.push('*');
    } else {
        let text = render_paragraph_with_inline_markup(para);
        output.push_str(&text);
    }
}

/// Render a slice of paragraphs into a single markdown string.
///
/// Paragraphs are separated by double newlines. Returns an empty string when
/// `paragraphs` is empty.
pub(crate) fn render_paragraphs_to_string(paragraphs: &[PdfParagraph]) -> String {
    let mut output = String::new();
    for para in paragraphs {
        if !output.is_empty() {
            output.push_str("\n\n");
        }
        render_paragraph_to_output(para, &mut output);
    }
    output
}

/// Inject image placeholders into markdown based on page numbers.
pub fn inject_image_placeholders(markdown: &str, images: &[crate::types::ExtractedImage]) -> String {
    if images.is_empty() {
        return markdown.to_string();
    }

    let mut images_by_page: std::collections::BTreeMap<usize, Vec<(usize, &crate::types::ExtractedImage)>> =
        std::collections::BTreeMap::new();
    for (idx, img) in images.iter().enumerate() {
        let page = img.page_number.unwrap_or(0);
        images_by_page.entry(page).or_default().push((idx, img));
    }

    let mut result = markdown.to_string();

    for (&page, page_images) in &images_by_page {
        for (_idx, img) in page_images {
            let ii = img.image_index;
            let label = if page > 0 {
                format!("![Image {} (page {})](embedded:p{}_i{})", ii, page, page, ii)
            } else {
                format!("![Image {}](embedded:i{})", ii, ii)
            };
            result.push_str("\n\n");
            result.push_str(&label);
            if let Some(ref ocr) = img.ocr_result {
                let text = ocr.content.trim();
                if !text.is_empty() {
                    result.push_str(&format!("\n> *Image text: {}*", text));
                }
            }
        }
    }

    result
}

/// Normalize bullet/number list prefix to standard markdown syntax.
fn normalize_list_prefix(text: &str) -> String {
    let trimmed = text.trim_start();
    // Standard bullet chars (•, *) → "- "
    if trimmed.starts_with('\u{2022}') || trimmed.starts_with("* ") {
        let rest = if trimmed.starts_with('\u{2022}') {
            trimmed['\u{2022}'.len_utf8()..].trim_start()
        } else {
            trimmed[2..].trim_start()
        };
        return format!("- {rest}");
    }
    if trimmed.starts_with("- ") {
        return text.trim_start().to_string();
    }
    // Dash-like bullet chars: replace the leading character with "- " instead of
    // prepending, to avoid double prefixes like "- – text".
    // Covers: en dash (–), em dash (—), hyphen-minus variants (−, ‐, ‑, ‒, ―).
    const DASH_BULLETS: &[char] = &[
        '–', // U+2013 EN DASH
        '—', // U+2014 EM DASH
        '−', // U+2212 MINUS SIGN
        '‐', // U+2010 HYPHEN
        '‑', // U+2011 NON-BREAKING HYPHEN
        '‒', // U+2012 FIGURE DASH
        '―', // U+2015 HORIZONTAL BAR
        '➤', // U+27A4
        '►', // U+25BA
        '▶', // U+25B6
        '○', // U+25CB
        '●', // U+25CF
        '◦', // U+25E6
    ];
    for &ch in DASH_BULLETS {
        if trimmed.starts_with(ch) {
            let rest = trimmed[ch.len_utf8()..].trim_start();
            return format!("- {rest}");
        }
    }
    // Numbered prefix: keep as-is (e.g. "1. text")
    let bytes = trimmed.as_bytes();
    let digit_end = bytes.iter().position(|&b| !b.is_ascii_digit()).unwrap_or(0);
    if digit_end > 0 && digit_end < bytes.len() {
        let suffix = bytes[digit_end];
        if suffix == b'.' || suffix == b')' {
            return text.trim_start().to_string();
        }
    }
    // Fallback: prefix with "- "
    format!("- {trimmed}")
}

/// Join lines into a single string (no inline markup).
fn join_line_texts(lines: &[PdfLine]) -> String {
    let all_words: Vec<&str> = lines
        .iter()
        .flat_map(|l| l.segments.iter().flat_map(|s| s.text.split_whitespace()))
        .collect();
    join_texts_cjk_aware(&all_words)
}

/// Join text chunks with spaces, but omit the space when both adjacent chunks are CJK.
/// Also performs dehyphenation: if a word ends with `-` (preceded by an alphabetic char)
/// and the next word starts with a lowercase letter, joins them without space and removes
/// the trailing hyphen.
fn join_texts_cjk_aware(texts: &[&str]) -> String {
    if texts.is_empty() {
        return String::new();
    }
    let mut result = String::from(texts[0]);
    for pair in texts.windows(2) {
        let prev = pair[0];
        let next = pair[1];

        // Dehyphenation: "syl-" + "lable" → "syllable"
        if should_dehyphenate(prev, next) {
            // Remove trailing hyphen from result and join directly
            result.pop(); // remove the '-'
            result.push_str(next);
        } else {
            if needs_space_between(prev, next) {
                result.push(' ');
            }
            result.push_str(next);
        }
    }
    result
}

/// Check if a line-ending hyphen should be removed and words joined.
///
/// Returns true when `prev` ends with `-` preceded by an alphabetic character
/// and `next` starts with a lowercase letter.
fn should_dehyphenate(prev: &str, next: &str) -> bool {
    if prev.len() < 2 || !prev.ends_with('-') {
        return false;
    }
    // Character before hyphen must be alphabetic
    let before_hyphen = prev[..prev.len() - 1].chars().next_back();
    if !before_hyphen.is_some_and(|c| c.is_alphabetic()) {
        return false;
    }
    // Next word must start with a lowercase letter
    next.chars().next().is_some_and(|c| c.is_lowercase())
}

/// Escape HTML entities in text for safe markdown output.
///
/// Replacements applied in order (`&` first to avoid double-escaping):
/// - `&` → `&amp;`
/// - `<` → `&lt;`
/// - `>` → `&gt;`
///
/// Also escapes `_` as `\_` unless the text contains `://` (to preserve URLs).
///
/// Uses a single-pass scan: if no special characters are found, returns a
/// borrowed `Cow` with no allocation.
///
/// Visibility is `pub(in crate::pdf::markdown)` so child modules such as
/// `crate::pdf::markdown::regions::slanet` can import it.
pub(in crate::pdf::markdown) fn escape_html_entities(text: &str) -> Cow<'_, str> {
    // Determine which replacements are needed with a fast pre-scan.
    let is_url = text.contains("://");
    let needs_amp = text.contains('&');
    let needs_lt = text.contains('<');
    let needs_gt = text.contains('>');
    let needs_underscore = !is_url && text.contains('_');

    if !needs_amp && !needs_lt && !needs_gt && !needs_underscore {
        return Cow::Borrowed(text);
    }

    // Single allocation: build result in one pass.
    let mut result = String::with_capacity(text.len() + 16);
    for ch in text.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '_' if !is_url => result.push_str("\\_"),
            _ => result.push(ch),
        }
    }
    Cow::Owned(result)
}

/// Render an entire body paragraph with inline bold/italic markup.
fn render_paragraph_with_inline_markup(para: &PdfParagraph) -> String {
    let all_segments: Vec<&SegmentData> = para.lines.iter().flat_map(|l| l.segments.iter()).collect();
    let rendered = render_segment_refs_with_markup(&all_segments);
    escape_html_entities(&rendered).into_owned()
}

/// Core inline markup renderer working on segment references.
///
/// Groups consecutive segments sharing the same bold/italic state, wraps groups
/// in `**...**` or `*...*` as appropriate.
fn render_segment_refs_with_markup(segments: &[&SegmentData]) -> String {
    if segments.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut i = 0;

    while i < segments.len() {
        let bold = segments[i].is_bold;
        let italic = segments[i].is_italic;

        // Find the run of segments with the same formatting
        let run_start = i;
        while i < segments.len() && segments[i].is_bold == bold && segments[i].is_italic == italic {
            i += 1;
        }

        // Split each segment's text into words for proper CJK-aware joining
        let mut run_words: Vec<&str> = Vec::new();
        for seg in &segments[run_start..i] {
            for word in seg.text.split_whitespace() {
                run_words.push(word);
            }
        }
        let run_text = join_texts_cjk_aware(&run_words);

        if !result.is_empty() {
            let prev_last = segments[run_start - 1]
                .text
                .split_whitespace()
                .next_back()
                .unwrap_or("");
            let next_first = segments[run_start].text.split_whitespace().next().unwrap_or("");
            if needs_space_between(prev_last, next_first) {
                result.push(' ');
            }
        }

        match (bold, italic) {
            (true, true) => {
                result.push_str("***");
                result.push_str(&run_text);
                result.push_str("***");
            }
            (true, false) => {
                result.push_str("**");
                result.push_str(&run_text);
                result.push_str("**");
            }
            (false, true) => {
                result.push('*');
                result.push_str(&run_text);
                result.push('*');
            }
            (false, false) => {
                result.push_str(&run_text);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_segment(text: &str, is_bold: bool, is_italic: bool) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 12.0,
            font_size: 12.0,
            is_bold,
            is_italic,
            is_monospace: false,
            baseline_y: 700.0,
        }
    }

    fn make_line(segments: Vec<SegmentData>) -> PdfLine {
        PdfLine {
            segments,
            baseline_y: 700.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        }
    }

    #[test]
    fn test_render_plain_paragraph() {
        let para = PdfParagraph {
            lines: vec![make_line(vec![
                make_segment("Hello", false, false),
                make_segment("world", false, false),
            ])],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "Hello world");
    }

    #[test]
    fn test_render_heading() {
        let para = PdfParagraph {
            lines: vec![make_line(vec![make_segment("Title", false, false)])],
            dominant_font_size: 18.0,
            heading_level: Some(2),
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "## Title");
    }

    #[test]
    fn test_render_bold_markup() {
        let para = PdfParagraph {
            lines: vec![make_line(vec![
                make_segment("bold", true, false),
                make_segment("text", true, false),
            ])],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "**bold text**");
    }

    #[test]
    fn test_render_italic_markup() {
        let para = PdfParagraph {
            lines: vec![make_line(vec![make_segment("italic", false, true)])],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "*italic*");
    }

    #[test]
    fn test_render_bold_italic_markup() {
        let para = PdfParagraph {
            lines: vec![make_line(vec![make_segment("both", true, true)])],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "***both***");
    }

    #[test]
    fn test_render_mixed_formatting() {
        let para = PdfParagraph {
            lines: vec![make_line(vec![
                make_segment("normal", false, false),
                make_segment("bold", true, false),
                make_segment("normal2", false, false),
            ])],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "normal **bold** normal2");
    }

    #[test]
    fn test_inject_image_placeholders_empty() {
        let result = inject_image_placeholders("Hello", &[]);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_render_multiword_segments_no_double_space() {
        // Segments with trailing whitespace should not produce double spaces
        let para = PdfParagraph {
            lines: vec![make_line(vec![
                make_segment("hello ", false, false),
                make_segment("world", false, false),
            ])],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "hello world");
    }

    #[test]
    fn test_render_mixed_formatting_multiword() {
        // Multi-word segments with formatting transitions
        let para = PdfParagraph {
            lines: vec![make_line(vec![
                make_segment("normal text", false, false),
                make_segment("bold text", true, false),
                make_segment("more normal", false, false),
            ])],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "normal text **bold text** more normal");
    }

    #[test]
    fn test_dehyphenate_basic() {
        // "syl-" + "lable" → "syllable"
        let words = vec!["syl-", "lable"];
        assert_eq!(join_texts_cjk_aware(&words), "syllable");
    }

    #[test]
    fn test_dehyphenate_in_paragraph() {
        // Across line boundaries in a paragraph
        let para = PdfParagraph {
            lines: vec![
                make_line(vec![make_segment("The neglect-", false, false)]),
                make_line(vec![make_segment("ed buildings are old.", false, false)]),
            ],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "The neglected buildings are old.");
    }

    #[test]
    fn test_no_dehyphenate_uppercase_next() {
        // "word-" + "The" → keep hyphen (next word uppercase)
        let words = vec!["word-", "The"];
        assert_eq!(join_texts_cjk_aware(&words), "word- The");
    }

    #[test]
    fn test_no_dehyphenate_standalone_hyphen() {
        // "-" + "word" → not dehyphenated (standalone hyphen)
        let words = vec!["-", "word"];
        assert_eq!(join_texts_cjk_aware(&words), "- word");
    }

    #[test]
    fn test_no_dehyphenate_number_suffix() {
        // "item-" + "3" → keep as-is (next starts with digit, not lowercase)
        let words = vec!["item-", "3"];
        assert_eq!(join_texts_cjk_aware(&words), "item- 3");
    }

    #[test]
    fn test_heading_multiword_segments() {
        // Heading with multi-word segments should join words correctly
        let para = PdfParagraph {
            lines: vec![make_line(vec![
                make_segment("Chapter One", false, false),
                make_segment("Title", false, false),
            ])],
            dominant_font_size: 18.0,
            heading_level: Some(1),
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "# Chapter One Title");
    }

    #[test]
    fn test_escape_html_entities_ampersand() {
        assert_eq!(escape_html_entities("a & b"), "a &amp; b");
    }

    #[test]
    fn test_escape_html_entities_lt_gt() {
        assert_eq!(escape_html_entities("a < b > c"), "a &lt; b &gt; c");
    }

    #[test]
    fn test_escape_html_entities_no_double_escape() {
        // & must be replaced first so &lt; doesn't become &amp;lt;
        assert_eq!(escape_html_entities("a & b < c"), "a &amp; b &lt; c");
    }

    #[test]
    fn test_escape_html_entities_underscore() {
        assert_eq!(escape_html_entities("foo_bar"), "foo\\_bar");
    }

    #[test]
    fn test_escape_html_entities_url_preserves_underscore() {
        // URLs with :// should not have underscores escaped
        assert_eq!(
            escape_html_entities("https://example.com/foo_bar"),
            "https://example.com/foo_bar"
        );
    }

    #[test]
    fn test_render_paragraph_html_entities_escaped() {
        let para = PdfParagraph {
            lines: vec![make_line(vec![make_segment("a & b < c > d", false, false)])],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "a &amp; b &lt; c &gt; d");
    }

    #[test]
    fn test_render_code_block_no_html_escaping() {
        // Code blocks must NOT have HTML entities escaped
        let para = PdfParagraph {
            lines: vec![make_line(vec![make_segment("a & b < c", false, false)])],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: true,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "```\na & b < c\n```");
    }

    #[test]
    fn test_render_formula_no_html_escaping() {
        // Formula blocks must NOT have HTML entities escaped
        let para = PdfParagraph {
            lines: vec![make_line(vec![make_segment("x < y & z > w", false, false)])],
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: true,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "$$\nx < y & z > w\n$$");
    }

    #[test]
    fn test_escape_html_entities_basic() {
        // & → &amp;, < → &lt;, > → &gt;
        assert_eq!(escape_html_entities("a & b"), "a &amp; b");
        assert_eq!(escape_html_entities("x < y"), "x &lt; y");
        assert_eq!(escape_html_entities("p > q"), "p &gt; q");
        // All three together
        assert_eq!(escape_html_entities("a & b < c > d"), "a &amp; b &lt; c &gt; d");
    }

    #[test]
    fn test_escape_underscores() {
        // Underscores are escaped to \_ when the text does not contain "://"
        assert_eq!(escape_html_entities("foo_bar"), "foo\\_bar");
        assert_eq!(escape_html_entities("a_b_c"), "a\\_b\\_c");
        // Plain text without underscores is unchanged
        assert_eq!(escape_html_entities("no underscores here"), "no underscores here");
    }

    #[test]
    fn test_escape_preserves_urls() {
        // URLs containing "://" must NOT have underscores escaped
        let url = "https://example.com/path_to_resource";
        assert_eq!(escape_html_entities(url), url);
        // Protocol-relative URL also counts
        let proto = "ftp://host/file_name.txt";
        assert_eq!(escape_html_entities(proto), proto);
    }

    #[test]
    fn test_render_caption_layout_class() {
        let para = PdfParagraph {
            lines: vec![make_line(vec![make_segment("Figure 1. A caption.", false, false)])],
            dominant_font_size: 10.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: Some(LayoutHintClass::Caption),
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "*Figure 1. A caption.*");
    }

    #[test]
    fn test_render_non_caption_layout_class_not_italic() {
        // A paragraph with Footnote layout_class should not be wrapped in italics.
        let para = PdfParagraph {
            lines: vec![make_line(vec![make_segment("Footnote text.", false, false)])],
            dominant_font_size: 8.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: Some(LayoutHintClass::Footnote),
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert_eq!(output, "Footnote text.");
    }

    #[test]
    fn test_heading_text_is_escaped() {
        // A heading with "<" in its text should produce "&lt;" in the rendered output

        let para = PdfParagraph {
            lines: vec![make_line(vec![make_segment("Result <unknown>", false, false)])],
            dominant_font_size: 18.0,
            heading_level: Some(2),
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            caption_for: None,
            block_bbox: None,
        };
        let mut output = String::new();
        render_paragraph_to_output(&para, &mut output);
        assert!(output.contains("&lt;"), "heading should contain &lt; but got: {output}");
        assert!(output.contains("&gt;"), "heading should contain &gt; but got: {output}");
        assert!(!output.contains('<'), "raw < should not appear in heading output");
    }
}
