//! Core RTF parsing logic.

use crate::extractors::rtf::encoding::{decode_windows_1252, parse_hex_byte, parse_rtf_control_word};
use crate::extractors::rtf::formatting::{map_offset, normalize_whitespace_with_mapping};
use crate::extractors::rtf::images::{RtfImage, extract_pict_image};
use crate::extractors::rtf::tables::TableState;
use crate::types::Table;
use crate::types::TextAnnotation;
use crate::types::document_structure::AnnotationKind;

/// Metadata for a single paragraph extracted from RTF.
#[derive(Debug, Clone, Default)]
pub struct ParagraphMeta {
    /// Heading level (1-based): 1 = H1, 2 = H2, etc. 0 = not a heading.
    pub heading_level: u8,
    /// List nesting level (0-based). `None` means not a list item.
    pub list_level: Option<u8>,
    /// List override ID (\lsN). Used to detect list boundaries.
    pub list_id: Option<u16>,
    /// Whether this paragraph is a table placeholder (text is in tables vec).
    pub is_table: bool,
    /// Whether this list item is ordered (numbered/lettered). Detected from
    /// `\listtext` or `\pntext` content. `false` = unordered (bullet).
    pub ordered: bool,
}

/// A formatting span tracked during RTF parsing.
#[derive(Debug, Clone)]
pub struct RtfFormattingSpan {
    /// Byte offset in the output text where this format starts.
    pub start: usize,
    /// Byte offset in the output text where this format ends.
    pub end: usize,
    /// Whether bold was active.
    pub bold: bool,
    /// Whether italic was active.
    pub italic: bool,
    /// Whether underline was active.
    pub underline: bool,
    /// Whether strikethrough was active.
    pub strikethrough: bool,
    /// Color index into the color table (0 = default/auto).
    pub color_index: u16,
}

/// RTF formatting metadata extracted alongside text.
pub struct RtfFormattingData {
    /// Formatting spans corresponding to text regions.
    pub spans: Vec<RtfFormattingSpan>,
    /// Color table entries (index 0 is auto/default).
    pub color_table: Vec<String>,
    /// Header text content (from \header groups).
    pub header_text: Option<String>,
    /// Footer text content (from \footer groups).
    pub footer_text: Option<String>,
    /// Hyperlink spans: (start_byte, end_byte, url).
    pub hyperlinks: Vec<(usize, usize, String)>,
}

/// Tracks formatting state during the text extraction pass so that
/// formatting spans have byte offsets that exactly match the extracted text.
///
/// This is used inside `extract_text_from_rtf` to produce spans whose
/// byte ranges are guaranteed to align with the output text, eliminating
/// the offset-drift bug that occurred when formatting was tracked in a
/// separate pass.
#[derive(Clone, Default)]
struct FmtState {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    color_idx: u16,
}

struct FormattingTracker {
    /// Current formatting state.
    fmt: FmtState,
    /// Stack of formatting states pushed on `{` and popped on `}`.
    fmt_stack: Vec<FmtState>,
    /// Byte offset where the current span started.
    span_start: usize,
    /// Accumulated formatting spans (byte offsets into pre-normalized result).
    spans: Vec<RtfFormattingSpan>,
}

impl FormattingTracker {
    fn new() -> Self {
        Self {
            fmt: FmtState::default(),
            fmt_stack: Vec::new(),
            span_start: 0,
            spans: Vec::new(),
        }
    }

    /// Push current formatting state onto the stack (called on `{`).
    fn push(&mut self) {
        self.fmt_stack.push(self.fmt.clone());
    }

    /// Pop formatting state from the stack (called on `}`).
    /// If formatting changed inside the group, close the current span.
    fn pop(&mut self, text_offset: usize) {
        if let Some(parent) = self.fmt_stack.pop() {
            let changed = self.fmt.bold != parent.bold
                || self.fmt.italic != parent.italic
                || self.fmt.underline != parent.underline
                || self.fmt.strikethrough != parent.strikethrough
                || self.fmt.color_idx != parent.color_idx;
            if changed {
                if text_offset > self.span_start {
                    self.spans.push(RtfFormattingSpan {
                        start: self.span_start,
                        end: text_offset,
                        bold: self.fmt.bold,
                        italic: self.fmt.italic,
                        underline: self.fmt.underline,
                        strikethrough: self.fmt.strikethrough,
                        color_index: self.fmt.color_idx,
                    });
                }
                self.span_start = text_offset;
                self.fmt = parent;
            }
        }
    }

    /// Update a formatting field, closing the current span if the value changed.
    fn update_bold(&mut self, text_offset: usize, val: bool) {
        if val != self.fmt.bold {
            self.close_span(text_offset);
            self.fmt.bold = val;
        }
    }

    fn update_italic(&mut self, text_offset: usize, val: bool) {
        if val != self.fmt.italic {
            self.close_span(text_offset);
            self.fmt.italic = val;
        }
    }

    fn update_underline(&mut self, text_offset: usize, val: bool) {
        if val != self.fmt.underline {
            self.close_span(text_offset);
            self.fmt.underline = val;
        }
    }

    fn update_strikethrough(&mut self, text_offset: usize, val: bool) {
        if val != self.fmt.strikethrough {
            self.close_span(text_offset);
            self.fmt.strikethrough = val;
        }
    }

    fn update_color(&mut self, text_offset: usize, val: u16) {
        if val != self.fmt.color_idx {
            self.close_span(text_offset);
            self.fmt.color_idx = val;
        }
    }

    /// Reset all formatting to default, closing the current span if needed.
    fn reset_all(&mut self, text_offset: usize) {
        if self.fmt.bold || self.fmt.italic || self.fmt.underline || self.fmt.strikethrough || self.fmt.color_idx != 0 {
            self.close_span(text_offset);
            self.fmt = FmtState::default();
        }
    }

    fn close_span(&mut self, text_offset: usize) {
        if text_offset > self.span_start {
            self.spans.push(RtfFormattingSpan {
                start: self.span_start,
                end: text_offset,
                bold: self.fmt.bold,
                italic: self.fmt.italic,
                underline: self.fmt.underline,
                strikethrough: self.fmt.strikethrough,
                color_index: self.fmt.color_idx,
            });
        }
        self.span_start = text_offset;
    }

    /// Close the final span at the end of parsing.
    fn finalize(&mut self, text_offset: usize) {
        if text_offset > self.span_start
            && (self.fmt.bold
                || self.fmt.italic
                || self.fmt.underline
                || self.fmt.strikethrough
                || self.fmt.color_idx != 0)
        {
            self.spans.push(RtfFormattingSpan {
                start: self.span_start,
                end: text_offset,
                bold: self.fmt.bold,
                italic: self.fmt.italic,
                underline: self.fmt.underline,
                strikethrough: self.fmt.strikethrough,
                color_index: self.fmt.color_idx,
            });
        }
    }

    /// Remap all span byte offsets using a normalization mapping.
    fn remap_spans(&mut self, mapping: &[(usize, usize)]) {
        for span in &mut self.spans {
            span.start = map_offset(mapping, span.start);
            span.end = map_offset(mapping, span.end);
        }
        // Remove zero-length spans that may result from normalization
        self.spans.retain(|s| s.start < s.end);
    }
}

/// Extract the color table from RTF content.
///
/// Looks for `{\colortbl ...}` and parses semicolon-delimited color entries.
/// Each entry is formatted as `\red{R}\green{G}\blue{B};`.
fn parse_rtf_color_table(content: &str) -> Vec<String> {
    let mut colors = Vec::new();
    // Find {\colortbl
    let Some(start) = content.find("{\\colortbl") else {
        return colors;
    };
    let rest = &content[start..];
    // Find the closing brace
    let mut depth = 0;
    let mut table_content = String::new();
    for ch in rest.chars() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        if depth > 0 {
            table_content.push(ch);
        }
    }
    // Remove the leading `{\colortbl` prefix
    let table_body = table_content.strip_prefix("{\\colortbl").unwrap_or(&table_content);

    // Split on semicolons
    for entry in table_body.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            // Auto/default color entry
            colors.push(String::new());
            continue;
        }
        // Parse \red{N}\green{N}\blue{N}
        let mut r = 0u8;
        let mut g = 0u8;
        let mut b = 0u8;
        for part in entry.split('\\') {
            let part = part.trim();
            if let Some(val) = part.strip_prefix("red") {
                r = val.parse().unwrap_or(0);
            } else if let Some(val) = part.strip_prefix("green") {
                g = val.parse().unwrap_or(0);
            } else if let Some(val) = part.strip_prefix("blue") {
                b = val.parse().unwrap_or(0);
            }
        }
        colors.push(format!("#{r:02x}{g:02x}{b:02x}"));
    }
    colors
}

/// Extract formatting metadata from RTF content.
///
/// This performs a lightweight pass over the RTF to extract:
/// - Bold/italic/underline formatting state changes
/// - Color table and color references
/// - Header/footer text
/// - Hyperlink field instructions
pub fn extract_rtf_formatting(content: &str) -> RtfFormattingData {
    let color_table = parse_rtf_color_table(content);
    let mut spans = Vec::new();
    let mut hyperlinks = Vec::new();
    let mut text_offset: usize = 0;
    let mut span_start: usize = 0;

    // Track header/footer destinations
    let mut in_header = false;
    let mut in_footer = false;
    let mut header_depth: i32 = 0;
    let mut footer_depth: i32 = 0;
    let mut header_buf = String::new();
    let mut footer_buf = String::new();

    // Track HYPERLINK fields
    let mut in_fldinst = false;
    let mut fldinst_depth: i32 = 0;
    let mut fldinst_content = String::new();
    let mut in_fldrslt = false;
    let mut fldrslt_depth: i32 = 0;
    let mut fldrslt_start: usize = 0;
    let mut pending_hyperlink_url: Option<String> = None;

    // Formatting state stack: pushed on `{`, popped on `}` so that
    // formatting set inside a group is properly scoped and does not
    // bleed into subsequent groups.
    #[derive(Clone)]
    struct FmtState {
        bold: bool,
        italic: bool,
        underline: bool,
        strikethrough: bool,
        color_idx: u16,
    }
    let mut fmt = FmtState {
        bold: false,
        italic: false,
        underline: false,
        strikethrough: false,
        color_idx: 0,
    };
    let mut fmt_stack: Vec<FmtState> = Vec::new();

    let mut group_depth: i32 = 0;
    let mut skip_depth: i32 = 0;
    let mut chars = content.chars().peekable();
    let mut expect_destination = false;
    let mut ignorable_pending = false;

    // Subset of SKIP_DESTINATIONS -- we DON'T skip "field" or "fldinst" here
    // because we want to parse hyperlinks.
    let skip_dests = [
        "fonttbl",
        "stylesheet",
        "info",
        "listtable",
        "listoverridetable",
        "generator",
        "filetbl",
        "revtbl",
        "rsidtbl",
        "xmlnstbl",
        "mmathPr",
        "themedata",
        "colorschememapping",
        "datastore",
        "latentstyles",
        "datafield",
        "objdata",
        "objclass",
        "panose",
        "bkmkstart",
        "bkmkend",
        "wgrffmtfilter",
        "fcharset",
        "pgdsctbl",
        "colortbl",
        "pict",
    ];

    // Track whether each group produced text (mirrors extract_text_from_rtf)
    let mut group_has_text: Vec<bool> = Vec::new();
    let mut pending_boundary_space = false;

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                group_depth += 1;
                expect_destination = true;
                // Push current formatting state so it can be restored on `}`
                fmt_stack.push(fmt.clone());
                group_has_text.push(false);
                pending_boundary_space = false;
            }
            '}' => {
                group_depth -= 1;
                expect_destination = false;
                ignorable_pending = false;
                // Restore formatting state from before this group opened.
                // If formatting changed inside the group, close the span and
                // revert to the parent state.
                if let Some(parent) = fmt_stack.pop() {
                    let changed = fmt.bold != parent.bold
                        || fmt.italic != parent.italic
                        || fmt.underline != parent.underline
                        || fmt.strikethrough != parent.strikethrough
                        || fmt.color_idx != parent.color_idx;
                    if changed {
                        if text_offset > span_start {
                            spans.push(RtfFormattingSpan {
                                start: span_start,
                                end: text_offset,
                                bold: fmt.bold,
                                italic: fmt.italic,
                                underline: fmt.underline,
                                strikethrough: fmt.strikethrough,
                                color_index: fmt.color_idx,
                            });
                        }
                        span_start = text_offset;
                        fmt = parent;
                    }
                }
                if skip_depth > 0 && group_depth < skip_depth {
                    skip_depth = 0;
                }
                if in_header && group_depth < header_depth {
                    in_header = false;
                }
                if in_footer && group_depth < footer_depth {
                    in_footer = false;
                }
                if in_fldinst && group_depth < fldinst_depth {
                    in_fldinst = false;
                    // Parse the HYPERLINK URL from fldinst content
                    let trimmed = fldinst_content.trim();
                    if let Some(rest) = trimmed.strip_prefix("HYPERLINK") {
                        let url = rest.trim().trim_matches('"').trim().to_string();
                        // Handle bookmark-style links: HYPERLINK \l "bookmark_name"
                        let url = if let Some(bookmark) = url.strip_prefix("\\l ") {
                            format!("#{}", bookmark.trim().trim_matches('"'))
                        } else if let Some(bookmark) = url.strip_prefix("\\l\"") {
                            format!("#{}", bookmark.trim_matches('"'))
                        } else {
                            url
                        };
                        if !url.is_empty() {
                            pending_hyperlink_url = Some(url);
                        }
                    }
                    fldinst_content.clear();
                }
                if in_fldrslt && group_depth < fldrslt_depth {
                    in_fldrslt = false;
                    if let Some(url) = pending_hyperlink_url.take() {
                        hyperlinks.push((fldrslt_start, text_offset, url));
                    }
                }
                // Mirror boundary-space logic from extract_text_from_rtf
                let produced_text = group_has_text.pop().unwrap_or(false);
                if produced_text && skip_depth == 0 {
                    pending_boundary_space = true;
                }
            }
            '\\' => {
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        '\\' | '{' | '}' => {
                            chars.next();
                            expect_destination = false;
                            // Capture escaped chars in fldinst buffer even when skipping
                            if in_fldinst {
                                fldinst_content.push(next_ch);
                            }
                            if skip_depth > 0 {
                                continue;
                            }
                            // Flush deferred boundary space
                            if pending_boundary_space && text_offset > 0 {
                                text_offset += 1; // space
                            }
                            pending_boundary_space = false;
                            text_offset += next_ch.len_utf8();
                            if let Some(flag) = group_has_text.last_mut() {
                                *flag = true;
                            }
                            if in_header {
                                header_buf.push(next_ch);
                            }
                            if in_footer {
                                footer_buf.push(next_ch);
                            }
                        }
                        '\'' => {
                            chars.next();
                            expect_destination = false;
                            let _ = chars.next();
                            let _ = chars.next();
                            if skip_depth > 0 {
                                continue;
                            }
                            // Flush deferred boundary space
                            if pending_boundary_space && text_offset > 0 {
                                text_offset += 1;
                            }
                            pending_boundary_space = false;
                            // Count 1 byte for the decoded char
                            text_offset += 1;
                            if let Some(flag) = group_has_text.last_mut() {
                                *flag = true;
                            }
                        }
                        '*' => {
                            chars.next();
                            ignorable_pending = true;
                        }
                        _ => {
                            let (word, param) = parse_rtf_control_word(&mut chars);

                            if expect_destination || ignorable_pending {
                                expect_destination = false;

                                if ignorable_pending {
                                    ignorable_pending = false;
                                    // Allow \*\fldinst through for hyperlink parsing
                                    if word == "fldinst" {
                                        in_fldinst = true;
                                        fldinst_depth = group_depth;
                                        if skip_depth == 0 {
                                            skip_depth = group_depth;
                                        }
                                        continue;
                                    }
                                    if skip_depth == 0 {
                                        skip_depth = group_depth;
                                    }
                                    continue;
                                }

                                // Handle special destinations
                                match word.as_str() {
                                    "fldinst" => {
                                        in_fldinst = true;
                                        fldinst_depth = group_depth;
                                        if skip_depth == 0 {
                                            skip_depth = group_depth;
                                        }
                                        continue;
                                    }
                                    "fldrslt" => {
                                        in_fldrslt = true;
                                        fldrslt_depth = group_depth;
                                        fldrslt_start = text_offset;
                                        continue;
                                    }
                                    _ => {}
                                }

                                if skip_dests.contains(&word.as_str()) {
                                    if skip_depth == 0 {
                                        skip_depth = group_depth;
                                    }
                                    continue;
                                }
                            }

                            // Capture control words in fldinst buffer even when skipping
                            if in_fldinst {
                                fldinst_content.push_str(&word);
                            }
                            if skip_depth > 0 {
                                continue;
                            }

                            // Helper macro to close the current span and update
                            // a single formatting field on `fmt`.
                            macro_rules! update_fmt_field {
                                ($field:ident, $new_val:expr) => {
                                    let new_val = $new_val;
                                    if new_val != fmt.$field {
                                        if text_offset > span_start {
                                            spans.push(RtfFormattingSpan {
                                                start: span_start,
                                                end: text_offset,
                                                bold: fmt.bold,
                                                italic: fmt.italic,
                                                underline: fmt.underline,
                                                strikethrough: fmt.strikethrough,
                                                color_index: fmt.color_idx,
                                            });
                                        }
                                        span_start = text_offset;
                                        fmt.$field = new_val;
                                    }
                                };
                            }

                            match word.as_str() {
                                "b" => {
                                    update_fmt_field!(bold, param.unwrap_or(1) != 0);
                                }
                                "i" => {
                                    update_fmt_field!(italic, param.unwrap_or(1) != 0);
                                }
                                "ul" => {
                                    update_fmt_field!(underline, param.unwrap_or(1) != 0);
                                }
                                "ulnone" => {
                                    update_fmt_field!(underline, false);
                                }
                                "strike" => {
                                    update_fmt_field!(strikethrough, param.unwrap_or(1) != 0);
                                }
                                "cf" => {
                                    update_fmt_field!(color_idx, param.unwrap_or(0) as u16);
                                }
                                "plain" => {
                                    // Reset all formatting
                                    if fmt.bold
                                        || fmt.italic
                                        || fmt.underline
                                        || fmt.strikethrough
                                        || fmt.color_idx != 0
                                    {
                                        if text_offset > span_start {
                                            spans.push(RtfFormattingSpan {
                                                start: span_start,
                                                end: text_offset,
                                                bold: fmt.bold,
                                                italic: fmt.italic,
                                                underline: fmt.underline,
                                                strikethrough: fmt.strikethrough,
                                                color_index: fmt.color_idx,
                                            });
                                        }
                                        span_start = text_offset;
                                        fmt.bold = false;
                                        fmt.italic = false;
                                        fmt.underline = false;
                                        fmt.strikethrough = false;
                                        fmt.color_idx = 0;
                                    }
                                }
                                "header" | "headerl" | "headerr" | "headerf" => {
                                    in_header = true;
                                    header_depth = group_depth;
                                }
                                "footer" | "footerl" | "footerr" | "footerf" => {
                                    in_footer = true;
                                    footer_depth = group_depth;
                                }
                                "par" | "line" => {
                                    text_offset += 1; // newline
                                    if in_header {
                                        header_buf.push('\n');
                                    }
                                    if in_footer {
                                        footer_buf.push('\n');
                                    }
                                }
                                // Text-producing control words: advance text_offset to
                                // stay synchronised with extract_text_from_rtf.
                                "tab" => {
                                    text_offset += 1;
                                }
                                "bullet" => {
                                    text_offset += '\u{2022}'.len_utf8();
                                }
                                "lquote" => {
                                    text_offset += '\u{2018}'.len_utf8();
                                }
                                "rquote" => {
                                    text_offset += '\u{2019}'.len_utf8();
                                }
                                "ldblquote" => {
                                    text_offset += '\u{201C}'.len_utf8();
                                }
                                "rdblquote" => {
                                    text_offset += '\u{201D}'.len_utf8();
                                }
                                "endash" => {
                                    text_offset += '\u{2013}'.len_utf8();
                                }
                                "emdash" => {
                                    text_offset += '\u{2014}'.len_utf8();
                                }
                                "u" => {
                                    // Unicode char
                                    if let Some(code_num) = param {
                                        let code_u = if code_num < 0 {
                                            (code_num + 65536) as u32
                                        } else {
                                            code_num as u32
                                        };
                                        if let Some(c) = char::from_u32(code_u) {
                                            text_offset += c.len_utf8();
                                            if in_header {
                                                header_buf.push(c);
                                            }
                                            if in_footer {
                                                footer_buf.push(c);
                                            }
                                        }
                                    }
                                    // Skip replacement char
                                    if let Some(&next) = chars.peek()
                                        && next != '\\'
                                        && next != '{'
                                        && next != '}'
                                    {
                                        chars.next();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            '\n' | '\r' => {}
            ' ' | '\t' => {
                if in_fldinst {
                    fldinst_content.push(' ');
                }
                if skip_depth > 0 {
                    continue;
                }
                // Mirror space-dedup from extract_text_from_rtf:
                // only count a space if the previous output isn't already
                // a space or newline.
                if text_offset > 0 {
                    // We approximate: always count 1 space since exact
                    // dedup state isn't tracked here. The normalize_whitespace
                    // pass collapses duplicates, so counting 1 per space char
                    // is correct for non-adjacent spaces.
                    text_offset += 1;
                    if let Some(flag) = group_has_text.last_mut() {
                        *flag = true;
                    }
                }
            }
            _ => {
                // Capture field instruction content for HYPERLINK parsing
                if in_fldinst {
                    fldinst_content.push(ch);
                    continue;
                }
                if skip_depth > 0 {
                    continue;
                }
                // Flush deferred boundary space
                if pending_boundary_space && text_offset > 0 {
                    text_offset += 1;
                }
                pending_boundary_space = false;
                text_offset += ch.len_utf8();
                if let Some(flag) = group_has_text.last_mut() {
                    *flag = true;
                }
                if in_header {
                    header_buf.push(ch);
                }
                if in_footer {
                    footer_buf.push(ch);
                }
            }
        }
    }

    // Close final span
    if text_offset > span_start && (fmt.bold || fmt.italic || fmt.underline || fmt.strikethrough || fmt.color_idx != 0)
    {
        spans.push(RtfFormattingSpan {
            start: span_start,
            end: text_offset,
            bold: fmt.bold,
            italic: fmt.italic,
            underline: fmt.underline,
            strikethrough: fmt.strikethrough,
            color_index: fmt.color_idx,
        });
    }

    let header_trimmed = header_buf.trim().to_string();
    let footer_trimmed = footer_buf.trim().to_string();

    RtfFormattingData {
        spans,
        color_table,
        header_text: if header_trimmed.is_empty() {
            None
        } else {
            Some(header_trimmed)
        },
        footer_text: if footer_trimmed.is_empty() {
            None
        } else {
            Some(footer_trimmed)
        },
        hyperlinks,
    }
}

/// Convert RTF formatting spans into `TextAnnotation` vectors for a paragraph.
///
/// Given the byte range of a paragraph within the full extracted text,
/// produces annotations from the formatting spans that overlap.
pub fn spans_to_annotations(para_start: usize, para_end: usize, formatting: &RtfFormattingData) -> Vec<TextAnnotation> {
    let mut annotations = Vec::new();
    for span in &formatting.spans {
        // Check overlap
        if span.end <= para_start || span.start >= para_end {
            continue;
        }
        let ann_start = span.start.max(para_start) - para_start;
        let ann_end = span.end.min(para_end) - para_start;
        if ann_start >= ann_end {
            continue;
        }
        let s = ann_start as u32;
        let e = ann_end as u32;
        if span.bold {
            annotations.push(TextAnnotation {
                start: s,
                end: e,
                kind: AnnotationKind::Bold,
            });
        }
        if span.italic {
            annotations.push(TextAnnotation {
                start: s,
                end: e,
                kind: AnnotationKind::Italic,
            });
        }
        if span.underline {
            annotations.push(TextAnnotation {
                start: s,
                end: e,
                kind: AnnotationKind::Underline,
            });
        }
        if span.strikethrough {
            annotations.push(TextAnnotation {
                start: s,
                end: e,
                kind: AnnotationKind::Strikethrough,
            });
        }
        if span.color_index > 0
            && let Some(color) = formatting.color_table.get(span.color_index as usize)
            && !color.is_empty()
            && color != "#000000"
        {
            annotations.push(TextAnnotation {
                start: s,
                end: e,
                kind: AnnotationKind::Color { value: color.clone() },
            });
        }
    }

    // Add hyperlink annotations
    for (link_start, link_end, url) in &formatting.hyperlinks {
        if *link_end <= para_start || *link_start >= para_end {
            continue;
        }
        let s = (link_start.max(&para_start) - para_start) as u32;
        let e = (link_end.min(&para_end) - para_start) as u32;
        if s < e {
            annotations.push(TextAnnotation {
                start: s,
                end: e,
                kind: AnnotationKind::Link {
                    url: url.clone(),
                    title: None,
                },
            });
        }
    }

    annotations
}

/// Known RTF destination groups whose content should be skipped entirely.
///
/// These are groups that start with a control word and contain metadata,
/// font tables, style sheets, or binary data — not document body text.
///
/// Note: `field` and `fldinst` are NOT in this list — they are handled
/// specially so that hyperlink text (`\fldrslt`) is extracted.
const SKIP_DESTINATIONS: &[&str] = &[
    "fonttbl",
    "colortbl",
    "stylesheet",
    "info",
    "listtable",
    "listoverridetable",
    "generator",
    "filetbl",
    "revtbl",
    "rsidtbl",
    "xmlnstbl",
    "mmathPr",
    "themedata",
    "colorschememapping",
    "datastore",
    "latentstyles",
    "datafield",
    "objdata",
    "objclass",
    "panose",
    "bkmkstart",
    "bkmkend",
    "wgrffmtfilter",
    "fcharset",
    "pgdsctbl",
];

/// Extract text and image metadata from RTF document.
///
/// This function extracts plain text from an RTF document by:
/// 1. Tracking group nesting depth with a state stack
/// 2. Skipping known destination groups (fonttbl, stylesheet, info, etc.)
/// 3. Skipping `{\*\...}` ignorable destination groups
/// 4. Converting encoded characters to Unicode
/// 5. Extracting text while skipping formatting groups
/// 6. Detecting and extracting image metadata (\pict sections)
/// 7. Normalizing whitespace
pub fn extract_text_from_rtf(
    content: &str,
    plain: bool,
) -> (String, Vec<Table>, Vec<RtfImage>, Vec<ParagraphMeta>, RtfFormattingData) {
    let color_table = parse_rtf_color_table(content);
    let mut fmt_tracker = FormattingTracker::new();

    let mut result = String::new();
    let mut chars = content.chars().peekable();
    let mut tables: Vec<Table> = Vec::new();
    let mut images: Vec<RtfImage> = Vec::new();
    let mut table_state: Option<TableState> = None;

    // Per-paragraph metadata: one entry per paragraph separated by \n\n.
    let mut para_metas: Vec<ParagraphMeta> = Vec::new();
    // Current paragraph's metadata (accumulated between \pard and \par).
    let mut cur_heading_level: u8 = 0;
    let mut cur_list_level: Option<u8> = None;
    let mut cur_list_id: Option<u16> = None;
    // Track \listtext destination to skip bullet/number prefix text
    let mut in_listtext = false;
    let mut listtext_depth: i32 = 0;
    // Buffer to capture listtext content for ordered/unordered detection
    let mut listtext_buf = String::new();
    // Whether the current paragraph's list item is ordered (detected from listtext)
    let mut cur_ordered = false;
    // Flag to prevent double-emitting metadata when \par and \pard both occur
    let mut para_meta_emitted = false;

    // Unicode skip count (\ucN): how many replacement bytes follow \uN.
    // Scoped per group — push on '{', pop on '}'.
    let mut uc_stack: Vec<u8> = vec![1]; // default \uc1

    // Hyperlink field tracking for \field{\*\fldinst HYPERLINK "url"}{\fldrslt text}
    let mut in_fldinst = false;
    let mut fldinst_depth: i32 = 0;
    let mut fldinst_content = String::new();
    let mut in_fldrslt = false;
    let mut fldrslt_depth: i32 = 0;
    let mut fldrslt_start: usize = 0;
    let mut pending_hyperlink_url: Option<String> = None;
    let mut hyperlinks: Vec<(usize, usize, String)> = Vec::new();

    // Footnote tracking
    let mut in_footnote = false;
    let mut footnote_depth: i32 = 0;
    let mut footnote_buf = String::new();
    let mut footnote_count: usize = 0;
    let mut footnotes: Vec<String> = Vec::new();

    // Group state stack: each entry tracks whether the group should be skipped.
    // When skip_depth > 0, all content is suppressed until we return to the
    // enclosing depth.
    let mut group_depth: i32 = 0;
    let mut skip_depth: i32 = 0; // 0 = not skipping; >0 = skip until depth drops below this

    // Track whether the next group is an ignorable destination (\*)
    let mut ignorable_pending = false;
    // Track whether we just entered a new group and the first control word decides skip
    let mut expect_destination = false;

    // Track whether each group produced text output. Used to avoid inserting
    // spurious spaces at `}` when the group only contained font directives
    // (e.g. `\loch`, `\hich`, `\dbch`).
    let mut group_has_text: Vec<bool> = Vec::new();

    // Deferred boundary space: set to true when a text-producing group closes.
    // The space is only emitted when actual text follows (not another `{`).
    let mut pending_boundary_space = false;

    // Hidden text tracking: \v enables, \v0 or \plain disables.
    // Stack tracks hidden state per group depth for proper scoping.
    let mut hidden_stack: Vec<bool> = vec![false];

    let ensure_table = |table_state: &mut Option<TableState>| {
        if table_state.is_none() {
            *table_state = Some(TableState::new());
        }
    };

    let finalize_table = move |state_opt: &mut Option<TableState>, tables: &mut Vec<Table>| {
        if let Some(state) = state_opt.take()
            && let Some(table) = state.finalize_with_format(plain)
        {
            tables.push(table);
        }
    };

    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                group_depth += 1;
                expect_destination = true;
                group_has_text.push(false);
                // Inherit current uc value into new group scope
                let current_uc = uc_stack.last().copied().unwrap_or(1);
                uc_stack.push(current_uc);
                // Inherit hidden state into new group scope
                let current_hidden = hidden_stack.last().copied().unwrap_or(false);
                hidden_stack.push(current_hidden);
                // Push formatting state so it's restored on `}`
                fmt_tracker.push();
                // Adjacent group open `}{`: clear pending boundary space so that
                // `x}{\super superscript}` produces `xsuperscript` not `x superscript`.
                pending_boundary_space = false;
            }
            '}' => {
                group_depth -= 1;
                expect_destination = false;
                ignorable_pending = false;
                // Pop formatting state — closes span if formatting changed
                fmt_tracker.pop(result.len());
                // Pop uc_stack and hidden_stack for this group
                if uc_stack.len() > 1 {
                    uc_stack.pop();
                }
                if hidden_stack.len() > 1 {
                    hidden_stack.pop();
                }
                // If we were skipping and just exited the skipped group, stop skipping
                if skip_depth > 0 && group_depth < skip_depth {
                    skip_depth = 0;
                }
                // Exit listtext destination and detect ordered vs unordered
                if in_listtext && group_depth < listtext_depth {
                    in_listtext = false;
                    let lt = listtext_buf.trim();
                    // Detect ordered prefixes: "1.", "1)", "a.", "a)", "i.", "i)", "A.", etc.
                    // Also handles multi-digit numbers like "12." or Roman numerals "iv."
                    let is_ordered = lt
                        .strip_suffix('.')
                        .or_else(|| lt.strip_suffix(')'))
                        .is_some_and(|prefix| {
                            let p = prefix.trim();
                            // Numeric: "1", "12", etc.
                            if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() {
                                return true;
                            }
                            // Alphabetic: "a", "b", "A", "B", "iv", "III", etc.
                            if p.chars().all(|c| c.is_ascii_alphabetic()) && !p.is_empty() {
                                return true;
                            }
                            false
                        });
                    if is_ordered {
                        cur_ordered = true;
                    }
                    listtext_buf.clear();
                }
                // Handle \fldinst group closing — parse HYPERLINK URL
                if in_fldinst && group_depth < fldinst_depth {
                    in_fldinst = false;
                    let trimmed = fldinst_content.trim();
                    if let Some(rest) = trimmed.strip_prefix("HYPERLINK") {
                        let url = rest.trim().trim_matches('"').trim().to_string();
                        // Handle bookmark-style links: HYPERLINK \l "bookmark_name"
                        let url = if let Some(bookmark) = url.strip_prefix("\\l ") {
                            format!("#{}", bookmark.trim().trim_matches('"'))
                        } else if let Some(bookmark) = url.strip_prefix("\\l\"") {
                            format!("#{}", bookmark.trim_matches('"'))
                        } else {
                            url
                        };
                        if !url.is_empty() {
                            pending_hyperlink_url = Some(url);
                        }
                    }
                    fldinst_content.clear();
                }
                // Handle \fldrslt group closing
                if in_fldrslt && group_depth < fldrslt_depth {
                    in_fldrslt = false;
                    if let Some(url) = pending_hyperlink_url.take() {
                        hyperlinks.push((fldrslt_start, result.len(), url));
                    }
                }
                // Handle \footnote group closing — store footnote text
                if in_footnote && group_depth < footnote_depth {
                    in_footnote = false;
                    let note = footnote_buf.trim().to_string();
                    if !note.is_empty() {
                        footnotes.push(note);
                    }
                    footnote_buf.clear();
                }
                // Defer space insertion at group boundary. If the group produced
                // text and the next token is also text (not an adjacent group open),
                // a space is needed. But if `}{` appears with no intervening text,
                // the groups are adjacent and no space should be inserted (e.g.
                // `x}{\super superscript}` means `xsuperscript`).
                let produced_text = group_has_text.pop().unwrap_or(false);
                if produced_text && skip_depth == 0 {
                    pending_boundary_space = true;
                }
            }
            '\\' => {
                if let Some(&next_ch) = chars.peek() {
                    match next_ch {
                        // \<newline> is equivalent to \par in RTF
                        '\n' | '\r' => {
                            chars.next();
                            // Also consume \r\n pair
                            if next_ch == '\r'
                                && let Some(&'\n') = chars.peek()
                            {
                                chars.next();
                            }
                            expect_destination = false;
                            if skip_depth > 0 {
                                continue;
                            }
                            // Treat as \par: emit paragraph break
                            handle_control_word(
                                "par",
                                None,
                                &mut chars,
                                &mut result,
                                &mut table_state,
                                &mut tables,
                                &mut images,
                                &ensure_table,
                                &finalize_table,
                                plain,
                                &mut group_has_text,
                                &mut cur_heading_level,
                                &mut cur_list_level,
                                &mut cur_list_id,
                                &mut cur_ordered,
                                &mut para_metas,
                                &mut para_meta_emitted,
                                &mut uc_stack,
                                &mut footnote_count,
                                in_footnote,
                                &mut footnote_buf,
                                &mut pending_boundary_space,
                                &mut hidden_stack,
                                &mut fmt_tracker,
                            );
                        }
                        '\\' | '{' | '}' => {
                            chars.next();
                            expect_destination = false;
                            // Capture literal chars in fldinst/footnote buffers
                            if in_fldinst {
                                fldinst_content.push(next_ch);
                            }
                            if in_footnote {
                                footnote_buf.push(next_ch);
                            }
                            if skip_depth > 0 {
                                continue;
                            }
                            // Skip hidden text
                            if hidden_stack.last().copied().unwrap_or(false) {
                                continue;
                            }
                            // Flush deferred boundary space
                            if pending_boundary_space
                                && !result.is_empty()
                                && !result.ends_with(' ')
                                && !result.ends_with('\n')
                            {
                                result.push(' ');
                            }
                            pending_boundary_space = false;
                            para_meta_emitted = false;
                            result.push(next_ch);
                            if let Some(flag) = group_has_text.last_mut() {
                                *flag = true;
                            }
                        }
                        '\'' => {
                            chars.next();
                            expect_destination = false;
                            let hex1 = chars.next();
                            let hex2 = chars.next();
                            // Capture hex-encoded chars in footnote buffer even when skipping
                            if in_footnote
                                && let (Some(h1), Some(h2)) = (hex1, hex2)
                                && let Some(byte) = parse_hex_byte(h1, h2)
                            {
                                footnote_buf.push(decode_windows_1252(byte));
                            }
                            if skip_depth > 0 {
                                continue;
                            }
                            // Skip hidden text
                            if hidden_stack.last().copied().unwrap_or(false) {
                                continue;
                            }
                            if let (Some(h1), Some(h2)) = (hex1, hex2)
                                && let Some(byte) = parse_hex_byte(h1, h2)
                            {
                                let decoded = decode_windows_1252(byte);
                                if let Some(state) = table_state.as_mut()
                                    && state.in_row
                                {
                                    state.current_cell.push(decoded);
                                } else {
                                    // Flush deferred boundary space
                                    if pending_boundary_space
                                        && !result.is_empty()
                                        && !result.ends_with(' ')
                                        && !result.ends_with('\n')
                                    {
                                        result.push(' ');
                                    }
                                    pending_boundary_space = false;
                                    para_meta_emitted = false;
                                    result.push(decoded);
                                    if let Some(flag) = group_has_text.last_mut() {
                                        *flag = true;
                                    }
                                }
                            }
                        }
                        '*' => {
                            chars.next();
                            // \* marks an ignorable destination — skip the entire group
                            // if we don't recognize the keyword
                            ignorable_pending = true;
                        }
                        _ => {
                            let (control_word, _param) = parse_rtf_control_word(&mut chars);

                            // Check if this control word starts a destination to skip
                            if expect_destination || ignorable_pending {
                                expect_destination = false;

                                if ignorable_pending {
                                    // \* destination: skip entire group unless we specifically handle it
                                    ignorable_pending = false;
                                    // Allow \shppict through — it contains \pict groups with image data
                                    // Allow \fldinst through — it contains HYPERLINK field instructions
                                    if control_word == "fldinst" {
                                        in_fldinst = true;
                                        fldinst_depth = group_depth;
                                        if skip_depth == 0 {
                                            skip_depth = group_depth;
                                        }
                                        continue;
                                    }
                                    // Allow \listtext/\pntext through — needed for ordered/unordered
                                    // list detection (capture marker text like "1.", "a.", etc.)
                                    if control_word == "listtext" || control_word == "pntext" {
                                        in_listtext = true;
                                        listtext_depth = group_depth;
                                        listtext_buf.clear();
                                        if skip_depth == 0 {
                                            skip_depth = group_depth;
                                        }
                                        continue;
                                    }
                                    if control_word != "shppict" {
                                        if skip_depth == 0 {
                                            skip_depth = group_depth;
                                        }
                                        continue;
                                    }
                                }

                                // Capture \listtext destination content for ordered/unordered
                                // detection, but skip output to result.
                                if control_word == "listtext" || control_word == "pntext" {
                                    in_listtext = true;
                                    listtext_depth = group_depth;
                                    listtext_buf.clear();
                                    if skip_depth == 0 {
                                        skip_depth = group_depth;
                                    }
                                    continue;
                                }

                                // Handle \fldinst destination (non-ignorable case)
                                if control_word == "fldinst" {
                                    in_fldinst = true;
                                    fldinst_depth = group_depth;
                                    if skip_depth == 0 {
                                        skip_depth = group_depth;
                                    }
                                    continue;
                                }

                                // Handle \fldrslt destination — link display text
                                if control_word == "fldrslt" {
                                    in_fldrslt = true;
                                    fldrslt_depth = group_depth;
                                    fldrslt_start = result.len();
                                    // Don't skip — we want the text extracted
                                    continue;
                                }

                                // Handle \footnote destination
                                if control_word == "footnote" {
                                    in_footnote = true;
                                    footnote_depth = group_depth;
                                    footnote_buf.clear();
                                    if skip_depth == 0 {
                                        skip_depth = group_depth;
                                    }
                                    continue;
                                }

                                if SKIP_DESTINATIONS.contains(&control_word.as_str()) {
                                    if skip_depth == 0 {
                                        skip_depth = group_depth;
                                    }
                                    continue;
                                }
                            }

                            if skip_depth > 0 {
                                // Even when skipping, handle \uc inside footnotes
                                if control_word == "uc"
                                    && let Some(val) = _param
                                    && let Some(uc) = uc_stack.last_mut()
                                {
                                    *uc = val.max(0) as u8;
                                }
                                // Capture unicode chars inside footnote buffers
                                if in_footnote
                                    && control_word == "u"
                                    && let Some(code_num) = _param
                                {
                                    let code_u = if code_num < 0 {
                                        (code_num + 65536) as u32
                                    } else {
                                        code_num as u32
                                    };
                                    if let Some(c) = char::from_u32(code_u) {
                                        footnote_buf.push(c);
                                    }
                                    // Skip replacement chars per uc count
                                    let uc_count = uc_stack.last().copied().unwrap_or(1);
                                    for _ in 0..uc_count {
                                        if let Some(&next) = chars.peek()
                                            && next != '\\'
                                            && next != '{'
                                            && next != '}'
                                        {
                                            chars.next();
                                        }
                                    }
                                }
                                // Handle \par inside footnotes
                                if in_footnote && (control_word == "par" || control_word == "line") {
                                    footnote_buf.push(' ');
                                }
                                continue;
                            }

                            handle_control_word(
                                &control_word,
                                _param,
                                &mut chars,
                                &mut result,
                                &mut table_state,
                                &mut tables,
                                &mut images,
                                &ensure_table,
                                &finalize_table,
                                plain,
                                &mut group_has_text,
                                &mut cur_heading_level,
                                &mut cur_list_level,
                                &mut cur_list_id,
                                &mut cur_ordered,
                                &mut para_metas,
                                &mut para_meta_emitted,
                                &mut uc_stack,
                                &mut footnote_count,
                                in_footnote,
                                &mut footnote_buf,
                                &mut pending_boundary_space,
                                &mut hidden_stack,
                                &mut fmt_tracker,
                            );
                        }
                    }
                }
            }
            '\n' | '\r' => {
                // RTF line breaks in the source are not significant
            }
            ' ' | '\t' => {
                // Capture spaces in fldinst/footnote buffers even when skipping
                if in_fldinst {
                    fldinst_content.push(' ');
                }
                if in_footnote {
                    footnote_buf.push(' ');
                }
                if skip_depth > 0 && !in_footnote {
                    continue;
                }
                if in_footnote {
                    continue; // Already captured to footnote_buf
                }
                if let Some(state) = table_state.as_mut()
                    && state.in_row
                {
                    if !state.current_cell.ends_with(' ') {
                        state.current_cell.push(' ');
                    }
                } else if !result.is_empty() && !result.ends_with(' ') && !result.ends_with('\n') {
                    result.push(' ');
                    if let Some(flag) = group_has_text.last_mut() {
                        *flag = true;
                    }
                }
            }
            _ => {
                expect_destination = false;
                // Capture content in fldinst/footnote/listtext buffers even when skipping
                if in_fldinst {
                    fldinst_content.push(ch);
                }
                if in_footnote {
                    footnote_buf.push(ch);
                }
                if in_listtext {
                    listtext_buf.push(ch);
                }
                if skip_depth > 0 {
                    continue;
                }
                // Skip hidden text (\v)
                if hidden_stack.last().copied().unwrap_or(false) {
                    continue;
                }
                if let Some(state) = table_state.as_ref()
                    && !state.in_row
                    && !state.rows.is_empty()
                {
                    finalize_table(&mut table_state, &mut tables);
                }
                if let Some(state) = table_state.as_mut()
                    && state.in_row
                {
                    state.current_cell.push(ch);
                } else {
                    // Flush deferred boundary space before pushing text
                    if pending_boundary_space && !result.is_empty() && !result.ends_with(' ') && !result.ends_with('\n')
                    {
                        result.push(' ');
                    }
                    pending_boundary_space = false;
                    para_meta_emitted = false;
                    result.push(ch);
                    if let Some(flag) = group_has_text.last_mut() {
                        *flag = true;
                    }
                }
            }
        }
    }

    if table_state.is_some() {
        finalize_table(&mut table_state, &mut tables);
    }

    // Finalize formatting tracker — close any open span
    fmt_tracker.finalize(result.len());

    // Finalize the last paragraph's metadata if there's text after the last \par
    let (normalized, mapping) = normalize_whitespace_with_mapping(&result);
    let final_text = normalized.trim_end();
    if !final_text.is_empty() {
        // Count how many paragraphs we have (split by \n\n)
        let para_count = normalized.split("\n\n").filter(|p| !p.trim().is_empty()).count();
        while para_metas.len() < para_count {
            para_metas.push(ParagraphMeta {
                heading_level: cur_heading_level,
                list_level: cur_list_level,
                list_id: cur_list_id,
                is_table: false,
                ordered: cur_ordered,
            });
        }
    }

    // Append footnote definitions at the end
    let mut final_result = normalized;
    if !footnotes.is_empty() {
        if !final_result.ends_with('\n') {
            final_result.push('\n');
            final_result.push('\n');
        }
        for (i, note) in footnotes.iter().enumerate() {
            final_result.push_str(&format!("[^{}]: {}", i + 1, note.trim()));
            final_result.push('\n');
            final_result.push('\n');
        }
    }

    // Remap formatting span byte offsets through the normalization mapping
    fmt_tracker.remap_spans(&mapping);

    // Also remap hyperlink byte offsets
    for link in &mut hyperlinks {
        link.0 = map_offset(&mapping, link.0);
        link.1 = map_offset(&mapping, link.1);
    }
    hyperlinks.retain(|l| l.0 < l.1);

    let formatting_data = RtfFormattingData {
        spans: fmt_tracker.spans,
        color_table,
        header_text: None, // Headers are extracted by extract_rtf_formatting
        footer_text: None, // Footers are extracted by extract_rtf_formatting
        hyperlinks,
    };

    (final_result, tables, images, para_metas, formatting_data)
}

/// Handle an RTF control word during parsing.
#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
fn handle_control_word(
    control_word: &str,
    param: Option<i32>,
    chars: &mut std::iter::Peekable<std::str::Chars>,
    result: &mut String,
    table_state: &mut Option<TableState>,
    tables: &mut Vec<Table>,
    images: &mut Vec<RtfImage>,
    ensure_table: &dyn Fn(&mut Option<TableState>),
    finalize_table: &dyn Fn(&mut Option<TableState>, &mut Vec<Table>),
    plain: bool,
    group_has_text: &mut [bool],
    cur_heading_level: &mut u8,
    cur_list_level: &mut Option<u8>,
    cur_list_id: &mut Option<u16>,
    cur_ordered: &mut bool,
    para_metas: &mut Vec<ParagraphMeta>,
    para_meta_emitted: &mut bool,
    uc_stack: &mut Vec<u8>,
    footnote_count: &mut usize,
    _in_footnote: bool,
    _footnote_buf: &mut String,
    pending_boundary_space: &mut bool,
    hidden_stack: &mut Vec<bool>,
    fmt_tracker: &mut FormattingTracker,
) {
    match control_word {
        // Hidden text: \v enables, \v0 disables
        "v" => {
            let hidden = param.unwrap_or(1) != 0;
            if let Some(h) = hidden_stack.last_mut() {
                *h = hidden;
            }
        }
        // Paragraph reset — start tracking new paragraph properties.
        // \pard starts a new paragraph definition. If there's already text,
        // emit a paragraph break and record metadata for the previous paragraph.
        "pard" => {
            // Inside a table row, \pard is just a cell-level formatting reset —
            // do NOT emit paragraph breaks or metadata.
            let in_table_row = table_state.as_ref().is_some_and(|s| s.in_row);
            if !in_table_row {
                // If there's content and we haven't already emitted metadata (from \par),
                // close the current paragraph.
                if !result.is_empty() && !result.ends_with('\n') && !*para_meta_emitted {
                    para_metas.push(ParagraphMeta {
                        heading_level: *cur_heading_level,
                        list_level: *cur_list_level,
                        list_id: *cur_list_id,
                        is_table: false,
                        ordered: *cur_ordered,
                    });
                    result.push('\n');
                    result.push('\n');
                    if let Some(flag) = group_has_text.last_mut() {
                        *flag = true;
                    }
                }
            }
            *para_meta_emitted = false;
            *cur_heading_level = 0;
            *cur_list_level = None;
            *cur_list_id = None;
            *cur_ordered = false;
        }
        // Outline level: \outlinelevel0 = H1, \outlinelevel1 = H2, etc.
        "outlinelevel" => {
            if let Some(level) = param {
                *cur_heading_level = (level as u8) + 1;
            }
        }
        // List nesting level: \ilvl0 = top level, \ilvl1 = nested, etc.
        "ilvl" => {
            *cur_list_level = Some(param.unwrap_or(0) as u8);
        }
        // List override ID: \lsN identifies which list
        "ls" => {
            *cur_list_id = Some(param.unwrap_or(0) as u16);
        }
        // Unicode skip count: \ucN sets how many replacement bytes follow \uN
        "uc" => {
            if let Some(val) = param
                && let Some(uc) = uc_stack.last_mut()
            {
                *uc = val.max(0) as u8;
            }
        }
        // Unicode escape: \u1234 (signed integer)
        "u" => {
            if let Some(code_num) = param {
                let code_u = if code_num < 0 {
                    (code_num + 65536) as u32
                } else {
                    code_num as u32
                };
                if let Some(c) = char::from_u32(code_u) {
                    if let Some(state) = table_state.as_mut()
                        && state.in_row
                    {
                        state.current_cell.push(c);
                    } else {
                        // Flush deferred boundary space
                        if *pending_boundary_space
                            && !result.is_empty()
                            && !result.ends_with(' ')
                            && !result.ends_with('\n')
                        {
                            result.push(' ');
                        }
                        *pending_boundary_space = false;
                        result.push(c);
                        if let Some(flag) = group_has_text.last_mut() {
                            *flag = true;
                        }
                    }
                }
                // Skip replacement characters per \uc count
                let uc_count = uc_stack.last().copied().unwrap_or(1);
                let mut skipped = 0u8;
                while skipped < uc_count {
                    if let Some(&next) = chars.peek() {
                        if next == '\\' {
                            // A \' hex escape counts as one replacement character
                            chars.next(); // consume '\'
                            if let Some(&apos) = chars.peek() {
                                if apos == '\'' {
                                    chars.next(); // consume '\''
                                    chars.next(); // consume hex digit 1
                                    chars.next(); // consume hex digit 2
                                    skipped += 1;
                                    continue;
                                }
                                // Other control word — don't consume it, break
                                // Put the backslash "back" conceptually — we can't un-consume,
                                // so we just break and let the main loop handle it
                                break;
                            }
                            break;
                        } else if next == '{' || next == '}' {
                            break;
                        } else {
                            chars.next();
                            skipped += 1;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        // Footnote reference marker
        "chftn" => {
            *footnote_count += 1;
            let marker = format!("[^{}]", *footnote_count);
            if let Some(state) = table_state.as_mut()
                && state.in_row
            {
                state.current_cell.push_str(&marker);
            } else {
                result.push_str(&marker);
                if let Some(flag) = group_has_text.last_mut() {
                    *flag = true;
                }
            }
        }
        "pict" => {
            let (image_metadata, rtf_image) = extract_pict_image(chars);
            if let Some(img) = rtf_image {
                images.push(img);
            }
            if !image_metadata.is_empty() && !plain {
                let img_md = format!("![image]({image_metadata}) ");
                if let Some(state) = table_state.as_mut()
                    && state.in_row
                {
                    state.current_cell.push_str(&img_md);
                } else {
                    if let Some(flag) = group_has_text.last_mut() {
                        *flag = true;
                    }
                    result.push_str(&img_md);
                }
            }
        }
        "par" | "line" => {
            *pending_boundary_space = false;
            let in_table_row = table_state.as_ref().is_some_and(|s| s.in_row);
            if in_table_row {
                // Inside a table row, \par is just a line break within a cell —
                // add a space to the cell content instead of a paragraph break.
                if let Some(state) = table_state.as_mut()
                    && !state.current_cell.is_empty()
                    && !state.current_cell.ends_with(' ')
                {
                    state.current_cell.push(' ');
                }
            } else {
                // Only finalize the table when we're sure no more rows follow.
                // If expecting_next_row is set, \par is just formatting between
                // rows (e.g. from \pard resets), not end-of-table.
                let still_in_table = table_state.as_ref().is_some_and(|s| s.expecting_next_row);
                if table_state.is_some() && !still_in_table {
                    finalize_table(table_state, tables);
                }
                // Record metadata for this paragraph before emitting the break.
                // Only push meta + line breaks when there's actual content to close.
                if !result.is_empty() && !result.ends_with('\n') {
                    if !*para_meta_emitted {
                        para_metas.push(ParagraphMeta {
                            heading_level: *cur_heading_level,
                            list_level: *cur_list_level,
                            list_id: *cur_list_id,
                            is_table: false,
                            ordered: *cur_ordered,
                        });
                        *para_meta_emitted = true;
                    }
                    result.push('\n');
                    result.push('\n');
                }
                if let Some(flag) = group_has_text.last_mut() {
                    *flag = true;
                }
            }
        }
        "tab" => {
            if let Some(state) = table_state.as_mut()
                && state.in_row
            {
                state.current_cell.push('\t');
            } else {
                result.push('\t');
                if let Some(flag) = group_has_text.last_mut() {
                    *flag = true;
                }
            }
        }
        "bullet" => {
            result.push('\u{2022}');
            if let Some(flag) = group_has_text.last_mut() {
                *flag = true;
            }
        }
        "lquote" => {
            result.push('\u{2018}');
            if let Some(flag) = group_has_text.last_mut() {
                *flag = true;
            }
        }
        "rquote" => {
            result.push('\u{2019}');
            if let Some(flag) = group_has_text.last_mut() {
                *flag = true;
            }
        }
        "ldblquote" => {
            result.push('\u{201C}');
            if let Some(flag) = group_has_text.last_mut() {
                *flag = true;
            }
        }
        "rdblquote" => {
            result.push('\u{201D}');
            if let Some(flag) = group_has_text.last_mut() {
                *flag = true;
            }
        }
        "endash" => {
            result.push('\u{2013}');
            if let Some(flag) = group_has_text.last_mut() {
                *flag = true;
            }
        }
        "emdash" => {
            result.push('\u{2014}');
            if let Some(flag) = group_has_text.last_mut() {
                *flag = true;
            }
        }
        "trowd" => {
            ensure_table(table_state);
            if let Some(state) = table_state.as_mut() {
                state.start_row();
            }
        }
        "cell" => {
            if let Some(state) = table_state.as_mut()
                && state.in_row
            {
                state.push_cell();
            }
        }
        "row" => {
            ensure_table(table_state);
            if let Some(state) = table_state.as_mut()
                && (state.in_row || !state.current_cell.is_empty())
            {
                state.push_row();
            }
            // Write a placeholder paragraph for this table row so that
            // para_metas stays aligned with the paragraphs in `result`.
            if !result.is_empty() && !result.ends_with('\n') {
                result.push('\n');
                result.push('\n');
            }
            result.push_str("[TABLE_ROW]");
            result.push('\n');
            result.push('\n');
            if let Some(flag) = group_has_text.last_mut() {
                *flag = true;
            }
            // Mark this as a table row in para metadata
            *para_meta_emitted = true;
            para_metas.push(ParagraphMeta {
                is_table: true,
                ..Default::default()
            });
        }
        // \intbl marks a paragraph as inside a table. If we have a table state
        // that just finished a row (expecting_next_row), start a new row so
        // that subsequent text goes into cells rather than leaking into the
        // result string. This handles RTFs where \trowd appears only once and
        // subsequent rows use \pard\intbl without repeating \trowd.
        "intbl" => {
            ensure_table(table_state);
            if let Some(state) = table_state.as_mut()
                && !state.in_row {
                    state.start_row();
                }
        }
        // Formatting control words — tracked for annotation spans
        "b" => {
            fmt_tracker.update_bold(result.len(), param.unwrap_or(1) != 0);
        }
        "i" => {
            fmt_tracker.update_italic(result.len(), param.unwrap_or(1) != 0);
        }
        "ul" => {
            fmt_tracker.update_underline(result.len(), param.unwrap_or(1) != 0);
        }
        "ulnone" => {
            fmt_tracker.update_underline(result.len(), false);
        }
        "strike" => {
            fmt_tracker.update_strikethrough(result.len(), param.unwrap_or(1) != 0);
        }
        "cf" => {
            fmt_tracker.update_color(result.len(), param.unwrap_or(0) as u16);
        }
        // \plain resets all character formatting including hidden text
        "plain" => {
            if let Some(h) = hidden_stack.last_mut() {
                *h = false;
            }
            fmt_tracker.reset_all(result.len());
        }
        _ => {}
    }
}
