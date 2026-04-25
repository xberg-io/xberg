//! Native Rust reStructuredText (RST) extractor.
//!
//! This extractor provides comprehensive RST document parsing.
//! It extracts:
//! - Document title and headings
//! - Field list metadata (:Author:, :Date:, :Version:, etc.)
//! - Paragraphs and text content
//! - Code blocks with language specifications
//! - Lists (bullet, numbered, definition lists)
//! - Tables (both simple and grid tables)
//! - Directives (image, code-block, note, math, etc.)
//! - Inline markup (emphasis, strong, code, links)
//! - Images and references

#[cfg(feature = "office")]
use crate::Result;
#[cfg(feature = "office")]
use crate::core::config::ExtractionConfig;
#[cfg(feature = "office")]
use crate::extractors::security::SecurityBudget;
#[cfg(feature = "office")]
use crate::plugins::{DocumentExtractor, Plugin};
#[cfg(feature = "office")]
use crate::types::document_structure::{AnnotationKind, TextAnnotation};
#[cfg(feature = "office")]
use crate::types::internal::InternalDocument;
#[cfg(feature = "office")]
use crate::types::internal::{RelationshipKind, RelationshipTarget};
#[cfg(feature = "office")]
use crate::types::internal_builder::InternalDocumentBuilder;
#[cfg(feature = "office")]
use crate::types::uri::Uri;
#[cfg(feature = "office")]
use crate::types::{Metadata, Table};
#[cfg(feature = "office")]
use ahash::AHashMap;
#[cfg(feature = "office")]
use async_trait::async_trait;
#[cfg(feature = "office")]
use std::borrow::Cow;

/// Native Rust reStructuredText extractor.
///
/// Parses RST documents using document tree parsing and extracts:
/// - Metadata from field lists
/// - Document structure (headings, sections)
/// - Text content and inline formatting
/// - Code blocks and directives
/// - Tables and lists
#[cfg(feature = "office")]
pub struct RstExtractor;

#[cfg(feature = "office")]
impl RstExtractor {
    /// Create a new RST extractor.
    pub(crate) fn new() -> Self {
        Self
    }

    /// Extract text content and metadata from RST document.
    ///
    /// Uses document tree parsing and fallback text extraction.
    fn extract_text_and_metadata(content: &str) -> (String, Metadata) {
        let mut metadata = Metadata::default();
        let mut additional: AHashMap<Cow<'static, str>, serde_json::Value> = AHashMap::new();

        let text = Self::extract_text_from_rst(content, &mut additional);

        // Map standard fields from additional to typed Metadata fields
        metadata.title = additional
            .remove(&Cow::Borrowed("title"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        metadata.authors = additional
            .remove(&Cow::Borrowed("author"))
            .and_then(|v| v.as_str().map(|s| vec![s.to_string()]));
        metadata.created_at = additional
            .remove(&Cow::Borrowed("date"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        metadata.additional = additional;
        (text, metadata)
    }

    /// Extract text and metadata from RST content.
    ///
    /// This is the main extraction engine that processes RST line-by-line
    /// and extracts all document content including headings, code blocks, lists, etc.
    fn extract_text_from_rst(content: &str, metadata: &mut AHashMap<Cow<'static, str>, serde_json::Value>) -> String {
        let mut output = String::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            if line.trim().starts_with(':')
                && line.contains(':')
                && let Some((key, value)) = Self::parse_field_list_line(line)
            {
                // Collect continuation lines (indented lines that follow)
                let mut full_value = value.clone();
                while i + 1 < lines.len() {
                    let next = lines[i + 1];
                    if !next.is_empty() && (next.starts_with("   ") || next.starts_with("\t")) {
                        full_value.push('\n');
                        full_value.push_str(next);
                        i += 1;
                    } else {
                        break;
                    }
                }
                Self::add_metadata_field(&key, &full_value, metadata);
                // Output the field list in preserved format
                output.push_str(&format!(":{}: {}\n", key, full_value));
                i += 1;
                continue;
            }

            // Overline+underline heading (document title): skip the overline,
            // emit the title text, skip the underline.
            if Self::is_section_underline(line.trim())
                && i + 2 < lines.len()
                && !lines[i + 1].trim().is_empty()
                && Self::is_section_underline(lines[i + 2])
            {
                let overline_char = line.trim().chars().next().unwrap_or('=');
                let underline_char = lines[i + 2].trim().chars().next().unwrap_or('=');
                if overline_char == underline_char {
                    output.push_str(lines[i + 1].trim());
                    output.push('\n');
                    i += 3;
                    continue;
                }
            }

            if i + 1 < lines.len() {
                let next_line = lines[i + 1];
                if Self::is_section_underline(next_line) && !line.trim().is_empty() {
                    output.push_str(line.trim());
                    output.push('\n');
                    i += 2;
                    continue;
                }
            }

            if line.trim().starts_with(".. code-block::") || line.trim().starts_with(".. code::") {
                // Preserve the directive line
                output.push_str(line.trim());
                output.push('\n');
                i += 1;
                // Preserve empty line after directive
                while i < lines.len() && lines[i].trim().is_empty() {
                    output.push('\n');
                    i += 1;
                }
                // Preserve indented content
                while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                    output.push_str(lines[i]);
                    output.push('\n');
                    i += 1;
                }
                continue;
            }

            if line.trim().starts_with(".. highlight::") {
                let lang = line.trim_start_matches(".. highlight::").trim().to_string();
                if !lang.is_empty() {
                    output.push_str("highlight: ");
                    output.push_str(&lang);
                    output.push('\n');
                }
                i += 1;
                continue;
            }

            if line.trim().ends_with("::") && !line.trim().starts_with(".. ") {
                if let Some(display_text) = line.strip_suffix("::")
                    && !display_text.trim().is_empty()
                {
                    output.push_str(display_text.trim());
                    output.push('\n');
                }
                i += 1;
                while i < lines.len() && (lines[i].starts_with("    ") || lines[i].is_empty()) {
                    if !lines[i].is_empty() {
                        output.push_str(lines[i].trim_start());
                        output.push('\n');
                    }
                    i += 1;
                }
                continue;
            }

            if Self::is_list_item(line) {
                output.push_str(line.trim());
                output.push('\n');
                i += 1;
                continue;
            }

            if line.trim().starts_with(".. ") || line.trim() == ".." {
                let trimmed = line.trim();
                let directive = if trimmed == ".." { "" } else { &trimmed[3..] };

                if directive.starts_with("image::") {
                    let uri = directive.strip_prefix("image::").unwrap_or("").trim();
                    output.push_str("image: ");
                    output.push_str(uri);
                    output.push('\n');
                    i += 1;
                    continue;
                }

                if directive.starts_with("note::")
                    || directive.starts_with("warning::")
                    || directive.starts_with("important::")
                    || directive.starts_with("caution::")
                    || directive.starts_with("hint::")
                    || directive.starts_with("tip::")
                {
                    // Preserve the directive marker
                    output.push_str(trimmed);
                    output.push('\n');
                    i += 1;
                    while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                        if !lines[i].is_empty() {
                            output.push_str(lines[i]);
                            output.push('\n');
                        }
                        i += 1;
                    }
                    continue;
                }

                if directive.starts_with("math::") {
                    let math = directive.strip_prefix("math::").unwrap_or("").trim();
                    if !math.is_empty() {
                        output.push_str("math: ");
                        output.push_str(math);
                        output.push('\n');
                    }
                    i += 1;
                    while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                        if !lines[i].is_empty() {
                            output.push_str(lines[i].trim());
                            output.push('\n');
                        }
                        i += 1;
                    }
                    continue;
                }

                i += 1;
                while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                    i += 1;
                }
                continue;
            }

            if !line.trim().is_empty() && !Self::is_markup_line(line) {
                output.push_str(line);
                output.push('\n');
            }

            i += 1;
        }

        output
    }

    /// Parse a field list line (e.g., ":Author: John Doe")
    fn parse_field_list_line(line: &str) -> Option<(String, String)> {
        let trimmed = line.trim();
        if !trimmed.starts_with(':') {
            return None;
        }

        let rest = &trimmed[1..];
        if let Some(end_pos) = rest.find(':') {
            let key = rest[..end_pos].to_string();
            // A valid field list key must be non-empty
            if key.is_empty() {
                return None;
            }
            let value = rest[end_pos + 1..].trim().to_string();
            return Some((key, value));
        }

        None
    }

    /// Add a metadata field from RST field list.
    fn add_metadata_field(key: &str, value: &str, metadata: &mut AHashMap<Cow<'static, str>, serde_json::Value>) {
        let key_lower = key.to_lowercase();
        match key_lower.as_str() {
            "author" | "authors" => {
                metadata.insert(Cow::Borrowed("author"), serde_json::Value::String(value.to_string()));
            }
            "date" => {
                metadata.insert(Cow::Borrowed("date"), serde_json::Value::String(value.to_string()));
            }
            "version" | "revision" => {
                metadata.insert(Cow::Borrowed("version"), serde_json::Value::String(value.to_string()));
            }
            "title" => {
                metadata.insert(Cow::Borrowed("title"), serde_json::Value::String(value.to_string()));
            }
            _ => {
                metadata.insert(
                    Cow::Owned(format!("field_{}", key_lower)),
                    serde_json::Value::String(value.to_string()),
                );
            }
        }
    }

    /// Check if a line is a section underline.
    fn is_section_underline(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.len() < 3 {
            return false;
        }
        let chars: Vec<char> = trimmed.chars().collect();
        let first = chars[0];
        matches!(first, '=' | '-' | '~' | '+' | '^' | '"' | '`' | '#' | '*') && chars.iter().all(|c| *c == first)
    }

    /// Check if a line is a list item.
    fn is_list_item(line: &str) -> bool {
        let trimmed = line.trim_start();
        // Bullet list markers: *, +, - followed by space or tab
        if trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || trimmed.starts_with("- ")
            || trimmed.starts_with("*\t")
            || trimmed.starts_with("+\t")
            || trimmed.starts_with("-\t")
        {
            return true;
        }
        // Auto-numbered list: #. or (#) item
        if trimmed.starts_with("#. ")
            || trimmed.starts_with("#.\t")
            || trimmed.starts_with("(#) ")
            || trimmed.starts_with("(#)\t")
        {
            return true;
        }
        // Parenthesized markers: (2), (A), (d) etc.
        if trimmed.starts_with('(')
            && let Some(close) = trimmed.find(')')
            && close > 1
            && close < 6
        {
            let inner = &trimmed[1..close];
            let after = &trimmed[close + 1..];
            if (after.starts_with(' ') || after.starts_with('\t'))
                && (inner.chars().all(|c| c.is_alphanumeric()) || inner == "#")
            {
                return true;
            }
        }
        // Find the first whitespace (space or tab) after the marker
        let sep_pos = trimmed.find([' ', '\t']);
        if let Some(space_pos) = sep_pos
            && space_pos > 0
            && space_pos < 6
        {
            let prefix = &trimmed[..space_pos];
            if prefix.ends_with('.') || prefix.ends_with(')') {
                let body = &prefix[..prefix.len() - 1];
                // Numeric: 1. 2. etc.
                if body.chars().all(|c| c.is_ascii_digit()) {
                    return true;
                }
                // Alpha: A. B. a. b. etc.
                if body.chars().all(|c| c.is_ascii_alphabetic()) && body.len() <= 3 {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a line is just markup (underlines, etc.)
    fn is_markup_line(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.len() < 3 {
            return false;
        }
        let first = trimmed.chars().next().unwrap();
        trimmed.chars().all(|c| c == first)
            && matches!(first, '=' | '-' | '~' | '+' | '^' | '"' | '`' | '#' | '*' | '/')
    }

    /// Extract tables from RST content.
    ///
    /// Identifies and extracts both simple and grid tables.
    fn extract_tables(content: &str) -> Vec<Table> {
        let mut tables = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            // Simple table (=====  ===== separator)
            if Self::is_simple_table_separator(trimmed) {
                let start = i;
                let mut table_lines = Vec::new();
                table_lines.push(lines[i]);
                i += 1;
                while i < lines.len() {
                    let tl = lines[i].trim();
                    if tl.is_empty() {
                        break;
                    }
                    table_lines.push(lines[i]);
                    i += 1;
                    if Self::is_simple_table_separator(tl) {
                        break;
                    }
                }
                let cells = Self::parse_simple_table_cells(&table_lines);
                if !cells.is_empty() {
                    let markdown = Self::cells_to_markdown(&cells);
                    tables.push(Table {
                        cells,
                        markdown,
                        page_number: 1,
                        bounding_box: None,
                    });
                }
                let _ = start;
                continue;
            }

            // Grid table (+-----+-----+)
            if trimmed.starts_with('+')
                && trimmed.ends_with('+')
                && trimmed.contains('-')
                && let Some(table) = Self::parse_grid_table(&lines, &mut i)
            {
                tables.push(table);
                continue;
            }

            i += 1;
        }

        tables
    }

    /// Parse a grid table from lines.
    fn parse_grid_table(lines: &[&str], i: &mut usize) -> Option<Table> {
        let mut cells = Vec::new();
        let mut row = Vec::new();

        while *i < lines.len() && (lines[*i].contains('|') || lines[*i].trim().starts_with('+')) {
            let line = lines[*i].trim_matches(|c: char| c == '|' || c == '+');
            if !line.is_empty() {
                let cell_content = line.split('|').map(|s| s.trim().to_string()).collect::<Vec<_>>();
                row.extend(cell_content);

                if !row.is_empty() {
                    cells.push(row.clone());
                    row.clear();
                }
            }
            *i += 1;
        }

        if cells.is_empty() {
            return None;
        }

        let markdown = Self::cells_to_markdown(&cells);
        Some(Table {
            cells,
            markdown,
            page_number: 1,
            bounding_box: None,
        })
    }

    /// Strip RST inline markup from text and produce annotations with byte offsets
    /// into the stripped text.
    ///
    /// Handles: `**strong**` (bold), `*emphasis*` (italic), ``` ``literal`` ``` (code),
    /// and `` `interpreted` `` (code).
    fn parse_inline_markup(raw: &str) -> (String, Vec<TextAnnotation>) {
        let mut out = String::with_capacity(raw.len());
        let mut annotations = Vec::new();
        let bytes = raw.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            // **strong emphasis**
            if i + 1 < len
                && bytes[i] == b'*'
                && bytes[i + 1] == b'*'
                && let Some(end) = Self::find_closing_marker(raw, i + 2, "**")
            {
                let inner = &raw[i + 2..end];
                let start = out.len() as u32;
                out.push_str(inner);
                let end_off = out.len() as u32;
                if start < end_off {
                    annotations.push(TextAnnotation {
                        start,
                        end: end_off,
                        kind: AnnotationKind::Bold,
                    });
                }
                i = end + 2;
                continue;
            }
            // *emphasis*  (single star, not followed by another star)
            if bytes[i] == b'*'
                && (i + 1 >= len || bytes[i + 1] != b'*')
                && let Some(end) = Self::find_closing_marker(raw, i + 1, "*")
            {
                // Make sure this isn't inside a ** pair
                if end + 1 >= len || bytes[end + 1] != b'*' {
                    let inner = &raw[i + 1..end];
                    let start = out.len() as u32;
                    out.push_str(inner);
                    let end_off = out.len() as u32;
                    if start < end_off {
                        annotations.push(TextAnnotation {
                            start,
                            end: end_off,
                            kind: AnnotationKind::Italic,
                        });
                    }
                    i = end + 1;
                    continue;
                }
            }
            // ``literal``
            if i + 1 < len
                && bytes[i] == b'`'
                && bytes[i + 1] == b'`'
                && let Some(end) = Self::find_closing_marker(raw, i + 2, "``")
            {
                let inner = &raw[i + 2..end];
                let start = out.len() as u32;
                out.push_str(inner);
                let end_off = out.len() as u32;
                if start < end_off {
                    annotations.push(TextAnnotation {
                        start,
                        end: end_off,
                        kind: AnnotationKind::Code,
                    });
                }
                i = end + 2;
                continue;
            }
            // `interpreted text` or `link text <url>`_  (RST inline hyperlink)
            if bytes[i] == b'`'
                && (i + 1 >= len || bytes[i + 1] != b'`')
                && let Some(end) = Self::find_closing_single_backtick(raw, i + 1)
            {
                let inner = &raw[i + 1..end];
                // Check for trailing `_ (hyperlink marker)
                let after_close = end + 1; // position after closing backtick
                if after_close < len && bytes[after_close] == b'_' {
                    // RST inline hyperlink: `link text <url>`_
                    if let Some(angle_start) = inner.rfind('<')
                        && let Some(angle_end) = inner.rfind('>')
                        && angle_end > angle_start
                    {
                        let url = inner[angle_start + 1..angle_end].trim().to_string();
                        let link_text = inner[..angle_start].trim();
                        let start = out.len() as u32;
                        out.push_str(link_text);
                        let end_off = out.len() as u32;
                        if start < end_off {
                            annotations.push(TextAnnotation {
                                start,
                                end: end_off,
                                kind: AnnotationKind::Link { url, title: None },
                            });
                        }
                        i = after_close + 1; // skip past the trailing _
                        continue;
                    }
                    // Plain reference like `Python`_ — treat as code/interpreted text
                    let start = out.len() as u32;
                    out.push_str(inner);
                    let end_off = out.len() as u32;
                    if start < end_off {
                        annotations.push(TextAnnotation {
                            start,
                            end: end_off,
                            kind: AnnotationKind::Code,
                        });
                    }
                    i = after_close + 1;
                    continue;
                }
                // Regular interpreted text (no trailing _)
                let start = out.len() as u32;
                out.push_str(inner);
                let end_off = out.len() as u32;
                if start < end_off {
                    annotations.push(TextAnnotation {
                        start,
                        end: end_off,
                        kind: AnnotationKind::Code,
                    });
                }
                i = end + 1;
                continue;
            }
            // RST citation/footnote reference: [label]_  ->  [label]
            // Strip the trailing underscore so that the brackets render unescaped.
            if bytes[i] == b'['
                && let Some(close) = raw[i + 1..].find(']')
            {
                let label_end = i + 1 + close;
                // Check for trailing _
                if label_end + 1 < len && bytes[label_end + 1] == b'_' {
                    let label = &raw[i + 1..label_end];
                    out.push('[');
                    out.push_str(label);
                    out.push(']');
                    i = label_end + 2; // skip past ]_
                    continue;
                }
            }
            let ch = raw[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }

        (out, annotations)
    }

    /// Find the position of a closing marker substring starting from `from`.
    fn find_closing_marker(text: &str, from: usize, marker: &str) -> Option<usize> {
        text[from..].find(marker).map(|pos| from + pos)
    }

    /// Find closing single backtick that is NOT part of a double backtick.
    fn find_closing_single_backtick(text: &str, from: usize) -> Option<usize> {
        let bytes = text.as_bytes();
        let mut j = from;
        while j < bytes.len() {
            if bytes[j] == b'`' {
                // Make sure it's not ``
                if j + 1 < bytes.len() && bytes[j + 1] == b'`' {
                    j += 2;
                    continue;
                }
                return Some(j);
            }
            j += 1;
        }
        None
    }

    /// Parse RST footnote references from a line.
    /// Returns footnote labels found (e.g. "1" from `[1]_` or "#" from `[#]_`).
    fn find_footnote_references(line: &str) -> Vec<String> {
        let mut refs = Vec::new();
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'['
                && let Some(close) = line[i + 1..].find(']')
            {
                let label_end = i + 1 + close;
                let label = &line[i + 1..label_end];
                // Check for trailing _
                if label_end + 1 < bytes.len() && bytes[label_end + 1] == b'_' {
                    // Valid footnote ref: numeric or #-prefixed
                    if label.chars().all(|c| c.is_ascii_digit()) || label.starts_with('#') {
                        refs.push(label.to_string());
                    }
                }
            }
            i += 1;
        }
        refs
    }

    /// Parse image directive options (`:alt:`, `:width:`, `:height:`) from indented lines.
    fn parse_image_options(lines: &[&str], start: &mut usize) -> AHashMap<String, String> {
        let mut opts = AHashMap::new();
        while *start < lines.len() {
            let line = lines[*start];
            if !line.starts_with("   ") && !line.starts_with("\t") {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                *start += 1;
                break;
            }
            if trimmed.starts_with(':')
                && let Some(colon2) = trimmed[1..].find(':')
            {
                let key = trimmed[1..1 + colon2].to_string();
                let value = trimmed[2 + colon2..].trim().to_string();
                opts.insert(key, value);
            }
            *start += 1;
        }
        opts
    }

    /// Build an `InternalDocument` from RST content.
    ///
    /// Handles sections, paragraphs, code blocks, tables, footnotes, citations,
    /// and cross-references.
    pub(crate) fn build_internal_document(content: &str, inject_placeholders: bool) -> InternalDocument {
        let mut b = InternalDocumentBuilder::new("rst");
        let lines: Vec<&str> = content.lines().collect();
        let mut heading_char_order: Vec<char> = Vec::new();
        let mut has_overline_heading = false;
        let mut highlight_lang: Option<String> = None;
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            // Field list metadata
            if trimmed.starts_with(':')
                && trimmed.len() > 1
                && let Some((key, value)) = Self::parse_field_list_line(trimmed)
            {
                let mut full_value = value;
                while i + 1 < lines.len() {
                    let next = lines[i + 1];
                    if !next.is_empty() && (next.starts_with("   ") || next.starts_with("\t")) {
                        if !full_value.is_empty() {
                            full_value.push(' ');
                        }
                        full_value.push_str(next.trim());
                        i += 1;
                    } else {
                        break;
                    }
                }
                b.push_metadata_block(&[(key, full_value)], None);
                i += 1;
                continue;
            }

            // Overline+underline heading (document title): markup line, then text,
            // then same markup line.  RST convention: this is the document title → H1.
            if Self::is_section_underline(trimmed)
                && i + 2 < lines.len()
                && !lines[i + 1].trim().is_empty()
                && Self::is_section_underline(lines[i + 2])
            {
                let overline_char = trimmed.chars().next().unwrap_or('=');
                let underline_char = lines[i + 2].trim().chars().next().unwrap_or('=');
                if overline_char == underline_char {
                    let title_text = lines[i + 1].trim();
                    has_overline_heading = true;
                    b.push_heading(1, title_text, None, None);
                    i += 3;
                    continue;
                }
            }

            // Heading: text line followed by underline
            // Section headings (underline only) start at level 2; the first
            // underline character seen is H2, the second is H3, etc.
            if i + 1 < lines.len() && !trimmed.is_empty() && Self::is_section_underline(lines[i + 1]) {
                let underline_char = lines[i + 1].trim().chars().next().unwrap_or('=');
                if !heading_char_order.contains(&underline_char) {
                    heading_char_order.push(underline_char);
                }
                // When an overline heading already claimed H1, underline headings
                // start at H2 (+2 offset).  Otherwise the first underline char is H1 (+1).
                let base = if has_overline_heading { 2 } else { 1 };
                let level = heading_char_order
                    .iter()
                    .position(|&c| c == underline_char)
                    .map(|p| (p + base) as u8)
                    .unwrap_or(base as u8);
                b.push_heading(level, trimmed, None, None);
                i += 2;
                continue;
            }

            // Code block directive
            if trimmed.starts_with(".. code-block::") || trimmed.starts_with(".. code::") {
                let language: Option<&str> = if let Some(rest) = trimmed.strip_prefix(".. code-block::") {
                    let lang = rest.trim();
                    if lang.is_empty() { None } else { Some(lang) }
                } else if let Some(rest) = trimmed.strip_prefix(".. code::") {
                    let lang = rest.trim();
                    if lang.is_empty() { None } else { Some(lang) }
                } else {
                    None
                };
                i += 1;
                while i < lines.len() && lines[i].trim().is_empty() {
                    i += 1;
                }
                let mut code_content = String::new();
                while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                    if !code_content.is_empty() {
                        code_content.push('\n');
                    }
                    if lines[i].starts_with("   ") {
                        code_content.push_str(&lines[i][3..]);
                    }
                    i += 1;
                }
                b.push_code(code_content.trim_end(), language, None, None);
                continue;
            }

            // Admonition directives
            if trimmed.starts_with(".. note::")
                || trimmed.starts_with(".. warning::")
                || trimmed.starts_with(".. important::")
                || trimmed.starts_with(".. caution::")
                || trimmed.starts_with(".. hint::")
                || trimmed.starts_with(".. tip::")
            {
                let kind = trimmed.strip_prefix(".. ").unwrap_or("").trim_end_matches("::").trim();
                let idx = b.push_admonition(kind, None, None);
                i += 1;
                let mut admonition_text = String::new();
                while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                    if !lines[i].is_empty() {
                        if !admonition_text.is_empty() {
                            admonition_text.push(' ');
                        }
                        admonition_text.push_str(lines[i].trim());
                    }
                    i += 1;
                }
                // Replace the kind-name text with the actual body text so the
                // rendering layer puts the content inside the alert block.
                if !admonition_text.is_empty() {
                    b.set_text(idx, &admonition_text);
                }
                continue;
            }

            // Image directive
            if trimmed.starts_with(".. image::") {
                let uri = trimmed.strip_prefix(".. image::").unwrap_or("").trim();
                i += 1;
                let opts = Self::parse_image_options(&lines, &mut i);
                let alt = opts.get("alt").cloned();
                let desc = alt.as_deref().unwrap_or(uri);
                if !uri.is_empty() {
                    b.push_uri(Uri::image(uri, alt.clone()));
                }
                if inject_placeholders {
                    let idx = b.push_paragraph(&format!("[image: {}]", desc), vec![], None, None);
                    if !uri.is_empty() {
                        let mut attrs = ahash::AHashMap::new();
                        attrs.insert("src".to_string(), uri.to_string());
                        b.set_attributes(idx, attrs);
                    }
                }
                continue;
            }

            // Math directive
            if trimmed.starts_with(".. math::") {
                let inline_math = trimmed.strip_prefix(".. math::").unwrap_or("").trim();
                i += 1;
                // Skip option lines (:label:, :nowrap:, etc.)
                while i < lines.len() {
                    let l = lines[i].trim();
                    if l.starts_with(':') && l.ends_with(':') || (l.starts_with(':') && l.contains(": ")) {
                        // Check if it looks like a directive option
                        if lines[i].starts_with("   ") || lines[i].starts_with("\t") {
                            i += 1;
                            continue;
                        }
                    }
                    break;
                }
                let mut math_content = if inline_math.is_empty() {
                    String::new()
                } else {
                    inline_math.to_string()
                };
                // Collect math content. Blank lines within separate into
                // multiple formula elements.
                while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                    if lines[i].is_empty() {
                        // Blank line: emit current formula and start a new one
                        if !math_content.is_empty() {
                            b.push_formula(&math_content, None, None);
                            math_content = String::new();
                        }
                    } else {
                        if !math_content.is_empty() {
                            math_content.push('\n');
                        }
                        math_content.push_str(lines[i].trim());
                    }
                    i += 1;
                }
                if !math_content.is_empty() {
                    b.push_formula(&math_content, None, None);
                }
                continue;
            }

            // Footnote definitions: .. [1] text  or  .. [#label] text
            if trimmed.starts_with(".. [")
                && let Some(close) = trimmed.find(']')
                && close > 4
            {
                let label = &trimmed[4..close];
                let footnote_text = trimmed[close + 1..].trim();
                let mut full_text = footnote_text.to_string();
                i += 1;
                while i < lines.len() && (lines[i].starts_with("   ") || lines[i].starts_with("\t")) {
                    if !full_text.is_empty() {
                        full_text.push(' ');
                    }
                    full_text.push_str(lines[i].trim());
                    i += 1;
                }
                // Determine if it's a citation or footnote
                let is_citation = label.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                    && !label.chars().all(|c| c.is_ascii_digit())
                    && !label.starts_with('#');
                if is_citation {
                    b.push_citation(&full_text, label, None);
                } else {
                    b.push_footnote_definition(&full_text, label, None);
                }
                continue;
            }

            // Reference target directives: .. _label: url
            if trimmed.starts_with(".. _")
                && let Some(colon_pos) = trimmed[4..].find(": ")
            {
                let label = &trimmed[4..4 + colon_pos];
                let url = trimmed[4 + colon_pos + 2..].trim();
                if !url.is_empty() && !label.is_empty() {
                    let idx = b.push_paragraph(
                        label,
                        vec![TextAnnotation {
                            start: 0,
                            end: label.len() as u32,
                            kind: AnnotationKind::Link {
                                url: url.to_string(),
                                title: None,
                            },
                        }],
                        None,
                        None,
                    );
                    let _ = idx;
                }
                i += 1;
                continue;
            }

            // Highlight directive: sets the default language for subsequent :: blocks.
            if trimmed.starts_with(".. highlight::") {
                let lang = trimmed.strip_prefix(".. highlight::").unwrap_or("").trim();
                highlight_lang = if lang.is_empty() { None } else { Some(lang.to_string()) };
                i += 1;
                // Skip any options block
                while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                    i += 1;
                }
                continue;
            }

            // Contents directive: emit the title text as a paragraph
            if trimmed.starts_with(".. contents::") {
                let title = trimmed.strip_prefix(".. contents::").unwrap_or("").trim();
                if !title.is_empty() {
                    b.push_paragraph(title, vec![], None, None);
                }
                i += 1;
                // Skip options block
                while i < lines.len() && (lines[i].starts_with("   ") || lines[i].is_empty()) {
                    i += 1;
                }
                continue;
            }

            // Other directives - skip (including their indented body, tabs or spaces)
            if trimmed.starts_with(".. ") || trimmed == ".." {
                i += 1;
                while i < lines.len()
                    && (lines[i].starts_with("   ") || lines[i].starts_with("\t") || lines[i].is_empty())
                {
                    i += 1;
                }
                continue;
            }

            // Simple RST table (=====  =====  ====== separator lines)
            // Collect all non-blank lines. The table ends at a blank line or at a
            // closing separator followed by a blank line / EOF.
            if Self::is_simple_table_separator(trimmed) {
                let mut table_lines = Vec::new();
                while i < lines.len() {
                    let tl = lines[i].trim();
                    if tl.is_empty() {
                        break;
                    }
                    table_lines.push(lines[i]);
                    i += 1;
                }
                let cells = Self::parse_simple_table_cells(&table_lines);
                if !cells.is_empty() {
                    b.push_table_from_cells(&cells, None, None);
                }
                continue;
            }

            // Grid table (+-----+-----+ border lines)
            if trimmed.starts_with('+') && trimmed.ends_with('+') && trimmed.contains('-') {
                let mut table_lines = Vec::new();
                while i < lines.len() && (lines[i].trim().starts_with('+') || lines[i].trim().starts_with('|')) {
                    table_lines.push(lines[i]);
                    i += 1;
                }
                let cells = Self::parse_grid_table_cells(&table_lines);
                if !cells.is_empty() {
                    b.push_table_from_cells(&cells, None, None);
                }
                continue;
            }

            // List items
            if Self::is_list_item(line) {
                let is_ordered = {
                    let t = trimmed.trim_start();
                    // Auto-numbered lists (#.) are ordered
                    if t.starts_with("#. ") || t.starts_with("#.\t") {
                        true
                    } else if let Some(space_pos) = t.find([' ', '\t']) {
                        let prefix = &t[..space_pos];
                        prefix.ends_with('.') || prefix.ends_with(')')
                    } else {
                        false
                    }
                };
                b.push_list(is_ordered);
                while i < lines.len() && Self::is_list_item(lines[i]) {
                    let item_trimmed = lines[i].trim();
                    let text = if let Some(rest) = item_trimmed
                        .strip_prefix("* ")
                        .or_else(|| item_trimmed.strip_prefix("*\t"))
                        .or_else(|| item_trimmed.strip_prefix("+ "))
                        .or_else(|| item_trimmed.strip_prefix("+\t"))
                        .or_else(|| item_trimmed.strip_prefix("- "))
                        .or_else(|| item_trimmed.strip_prefix("-\t"))
                        .or_else(|| item_trimmed.strip_prefix("#. "))
                        .or_else(|| item_trimmed.strip_prefix("#.\t"))
                    {
                        rest
                    } else if let Some(space_pos) = item_trimmed.find([' ', '\t']) {
                        &item_trimmed[space_pos + 1..]
                    } else {
                        item_trimmed
                    };
                    // Collect continuation lines (indented, not a new list item)
                    let mut full_text = text.to_string();
                    i += 1;
                    while i < lines.len()
                        && !lines[i].trim().is_empty()
                        && (lines[i].starts_with("   ") || lines[i].starts_with("\t"))
                        && !Self::is_list_item(lines[i])
                    {
                        full_text.push(' ');
                        full_text.push_str(lines[i].trim());
                        i += 1;
                    }
                    let (parsed_text, item_annotations) = Self::parse_inline_markup(&full_text);
                    b.push_list_item(&parsed_text, is_ordered, item_annotations, None, None);
                }
                b.end_list();
                continue;
            }

            // ``::`` literal block: a line ending with ``::`` introduces an
            // indented code block.  The ``.. highlight::`` directive, if any,
            // sets the default language.
            if trimmed.ends_with("::") && !trimmed.starts_with(".. ") {
                // Emit the introductory text (strip the trailing `::`)
                if let Some(display_text) = trimmed.strip_suffix("::")
                    && !display_text.is_empty()
                {
                    let (stripped, annotations) = Self::parse_inline_markup(display_text);
                    b.push_paragraph(&stripped, annotations, None, None);
                }
                i += 1;
                // Skip blank lines between intro and indented content
                while i < lines.len() && lines[i].trim().is_empty() {
                    i += 1;
                }
                // Collect indented content (spaces or tabs).
                // Detect indent level from the first non-blank content line.
                let indent = {
                    let mut indent_len = 3usize; // default
                    let mut j = i;
                    while j < lines.len() {
                        let l = lines[j];
                        if !l.trim().is_empty() {
                            indent_len = l.len() - l.trim_start().len();
                            if indent_len == 0 {
                                indent_len = 3;
                            }
                            break;
                        }
                        j += 1;
                    }
                    indent_len
                };
                let mut code_content = String::new();
                while i < lines.len() {
                    let l = lines[i];
                    // An indented line (by at least `indent` spaces or a tab)
                    let is_indented = l.starts_with("\t")
                        || (l.len() >= indent && l.as_bytes().iter().take(indent).all(|&b| b == b' '));
                    if !is_indented && !l.is_empty() {
                        break;
                    }
                    if !code_content.is_empty() {
                        code_content.push('\n');
                    }
                    if let Some(stripped) = l.strip_prefix('\t') {
                        code_content.push_str(stripped);
                    } else if is_indented && !l.is_empty() {
                        code_content.push_str(&l[indent..]);
                    }
                    i += 1;
                }
                if !code_content.is_empty() {
                    b.push_code(code_content.trim_end(), highlight_lang.as_deref(), None, None);
                }
                continue;
            }

            // Regular paragraph with footnote refs and cross-references.
            // Join consecutive non-blank, non-indented lines into a single paragraph
            // (RST hard-wrapping).
            if !trimmed.is_empty() && !Self::is_markup_line(line) {
                let mut para_text = trimmed.to_string();
                // Peek ahead: join continuation lines that are part of the same paragraph.
                // A continuation line is non-empty, not indented, not a list item,
                // not a directive, not a section underline, not a markup line.
                while i + 1 < lines.len() {
                    let next = lines[i + 1];
                    let next_trimmed = next.trim();
                    // Stop at blank lines
                    if next_trimmed.is_empty() {
                        break;
                    }
                    // Stop at indented lines (could be a directive body, code block, etc.)
                    if next.starts_with(' ') || next.starts_with('\t') {
                        break;
                    }
                    // Stop at section underlines
                    if Self::is_section_underline(next_trimmed) {
                        break;
                    }
                    // Stop at markup lines
                    if Self::is_markup_line(next) {
                        break;
                    }
                    // Stop at directives
                    if next_trimmed.starts_with(".. ") || next_trimmed == ".." {
                        break;
                    }
                    // Stop at list items
                    if Self::is_list_item(next) {
                        break;
                    }
                    // Stop at field lists
                    if next_trimmed.starts_with(':')
                        && next_trimmed.len() > 1
                        && Self::parse_field_list_line(next_trimmed).is_some()
                    {
                        break;
                    }
                    // Stop at table separators
                    if Self::is_simple_table_separator(next_trimmed) {
                        break;
                    }
                    if next_trimmed.starts_with('+') && next_trimmed.ends_with('+') && next_trimmed.contains('-') {
                        break;
                    }
                    para_text.push(' ');
                    para_text.push_str(next_trimmed);
                    i += 1;
                }
                let footnote_refs = Self::find_footnote_references(&para_text);
                let (stripped, annotations) = Self::parse_inline_markup(&para_text);
                let idx = b.push_paragraph(&stripped, annotations, None, None);

                // Emit footnote reference relationships
                for fref in &footnote_refs {
                    let ref_idx = b.push_footnote_ref(&format!("[{}]", fref), fref, None);
                    let _ = ref_idx;
                }

                // Check for cross-reference patterns like :ref:`target`
                Self::extract_rst_cross_refs(&para_text, idx, &mut b);
            }

            i += 1;
        }

        b.build()
    }

    /// Extract RST cross-reference roles (`:ref:`, `:doc:`, etc.) and emit relationships.
    fn extract_rst_cross_refs(line: &str, source_idx: u32, b: &mut InternalDocumentBuilder) {
        let roles = [":ref:", ":doc:", ":numref:"];
        for role in &roles {
            let mut search_from = 0;
            while let Some(pos) = line[search_from..].find(role) {
                let abs_pos = search_from + pos;
                let after = &line[abs_pos + role.len()..];
                if after.starts_with('`')
                    && let Some(close) = after[1..].find('`')
                {
                    let target = &after[1..1 + close];
                    // Handle <display text> patterns
                    let key = if let Some(angle_pos) = target.find('<') {
                        let end = target.find('>').unwrap_or(target.len());
                        &target[angle_pos + 1..end]
                    } else {
                        target
                    };
                    if !key.is_empty() {
                        b.push_relationship(
                            source_idx,
                            RelationshipTarget::Key(key.to_string()),
                            RelationshipKind::CrossReference,
                        );
                    }
                    search_from = abs_pos + role.len() + 1 + close + 1;
                    continue;
                }
                search_from = abs_pos + role.len();
            }
        }
    }

    /// Parse cells from grid table lines (for DocumentStructure).
    fn parse_grid_table_cells(lines: &[&str]) -> Vec<Vec<String>> {
        let mut cells = Vec::new();
        for line in lines {
            let content = line.trim().trim_matches('|');
            if content.is_empty() {
                continue;
            }
            // Skip separator lines (all dashes/equals)
            if content
                .chars()
                .all(|c| c == '-' || c == '=' || c == '+' || c == '|' || c == ' ')
            {
                continue;
            }
            let row: Vec<String> = content
                .split('|')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !row.is_empty() {
                cells.push(row);
            }
        }
        cells
    }

    /// Check if a line is a simple RST table separator (e.g. `=====  =====  =====`).
    fn is_simple_table_separator(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.len() < 3 {
            return false;
        }
        // Must consist only of '=' and spaces, with at least one '=' run
        if !trimmed.chars().all(|c| c == '=' || c == ' ') {
            return false;
        }
        // Must contain at least one run of '='
        trimmed.contains('=')
    }

    /// Parse a simple RST table into cell rows.
    ///
    /// Simple tables use `=====  =====` separator lines. Column boundaries
    /// are determined by the positions of whitespace gaps in the first separator.
    fn parse_simple_table_cells(lines: &[&str]) -> Vec<Vec<String>> {
        if lines.is_empty() {
            return Vec::new();
        }

        // Determine column boundaries from the first separator line
        let separator = lines[0];
        let col_ranges = Self::simple_table_column_ranges(separator);
        if col_ranges.is_empty() {
            return Vec::new();
        }

        let mut cells = Vec::new();
        for line in lines {
            let trimmed = line.trim();
            // Skip separator lines
            if Self::is_simple_table_separator(trimmed) {
                continue;
            }
            let row: Vec<String> = col_ranges
                .iter()
                .map(|&(start, end)| {
                    let end = end.min(line.len());
                    let start = start.min(line.len());
                    if start >= line.len() {
                        String::new()
                    } else {
                        line[start..end].trim().to_string()
                    }
                })
                .collect();
            if row.iter().any(|c| !c.is_empty()) {
                cells.push(row);
            }
        }
        cells
    }

    /// Determine column start/end byte positions from a simple table separator line.
    fn simple_table_column_ranges(separator: &str) -> Vec<(usize, usize)> {
        let mut ranges = Vec::new();
        let bytes = separator.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'=' {
                let start = i;
                while i < bytes.len() && bytes[i] == b'=' {
                    i += 1;
                }
                ranges.push((start, i));
            } else {
                i += 1;
            }
        }
        ranges
    }

    /// Convert table cells to markdown format.
    fn cells_to_markdown(cells: &[Vec<String>]) -> String {
        if cells.is_empty() {
            return String::new();
        }

        let mut md = String::new();

        md.push('|');
        for cell in &cells[0] {
            md.push(' ');
            md.push_str(cell);
            md.push_str(" |");
        }
        md.push('\n');

        md.push('|');
        for _ in &cells[0] {
            md.push_str(" --- |");
        }
        md.push('\n');

        for row in &cells[1..] {
            md.push('|');
            for cell in row {
                md.push(' ');
                md.push_str(cell);
                md.push_str(" |");
            }
            md.push('\n');
        }

        md
    }
}

#[cfg(feature = "office")]
impl Default for RstExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "office")]
impl Plugin for RstExtractor {
    fn name(&self) -> &str {
        "rst-extractor"
    }

    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    fn initialize(&self) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "Native Rust extractor for reStructuredText (RST) documents"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for RstExtractor {
    #[cfg_attr(
        feature = "otel",
        tracing::instrument(
            skip(self, content, config),
            fields(
                extractor.name = self.name(),
                content.size_bytes = content.len(),
            )
        )
    )]
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        tracing::debug!(format = "rst", size_bytes = content.len(), "extraction starting");
        let mut budget = SecurityBudget::from_config(config);
        budget.account_text(content.len())?;
        let inject_placeholders = config
            .images
            .as_ref()
            .map(|img| img.inject_placeholders)
            .unwrap_or(true);
        let text = String::from_utf8_lossy(content).into_owned();

        let (_extracted_text, metadata) = Self::extract_text_and_metadata(&text);

        let tables = Self::extract_tables(&text);

        let mut doc = Self::build_internal_document(&text, inject_placeholders);
        doc.mime_type = Cow::Owned(mime_type.to_string());
        doc.metadata = metadata;

        // Add tables to InternalDocument
        for table in tables {
            doc.push_table(table);
        }

        tracing::debug!(
            element_count = doc.elements.len(),
            format = "rst",
            "extraction complete"
        );
        Ok(doc)
    }

    async fn extract_file(
        &self,
        path: &std::path::Path,
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        crate::core::path_resolver::extract_file_with_image_resolution(self, path, mime_type, config).await
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["text/x-rst", "text/prs.fallenstein.rst"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    #[test]
    fn test_rst_extractor_plugin_interface() {
        let extractor = RstExtractor::new();
        assert_eq!(extractor.name(), "rst-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert!(!extractor.supported_mime_types().is_empty());
    }

    #[test]
    fn test_rst_extractor_supports_text_x_rst() {
        let extractor = RstExtractor::new();
        assert!(extractor.supported_mime_types().contains(&"text/x-rst"));
    }

    #[test]
    fn test_rst_extractor_supports_fallenstein_rst() {
        let extractor = RstExtractor::new();
        assert!(extractor.supported_mime_types().contains(&"text/prs.fallenstein.rst"));
    }

    #[test]
    fn test_extract_text_from_rst_simple_document() {
        let content = r#"
Title
=====

This is a paragraph.

Another paragraph.
"#;

        let mut metadata = AHashMap::new();
        let output = RstExtractor::extract_text_from_rst(content, &mut metadata);
        assert!(output.contains("Title"));
        assert!(output.contains("This is a paragraph"));
        assert!(output.contains("Another paragraph"));
    }

    #[test]
    fn test_extract_text_from_rst_with_code_block() {
        let content = r#"
.. code-block:: python

   def hello():
       print("world")

Some text after.
"#;

        let mut metadata = AHashMap::new();
        let output = RstExtractor::extract_text_from_rst(content, &mut metadata);
        assert!(output.contains("code-block"));
        assert!(output.contains("def hello"));
        assert!(output.contains("Some text after"));
    }

    #[test]
    fn test_extract_text_from_rst_with_metadata() {
        let content = r#"
:Author: John Doe
:Date: 2024-01-15

First paragraph.

Second paragraph.
"#;

        let mut metadata = AHashMap::new();
        let output = RstExtractor::extract_text_from_rst(content, &mut metadata);
        assert!(output.contains("First paragraph"));
        assert!(output.contains("Second paragraph"));
        assert!(metadata.contains_key("author"));
        assert_eq!(metadata.get("author").and_then(|v| v.as_str()), Some("John Doe"));
    }

    #[test]
    fn test_cells_to_markdown_format() {
        let cells = vec![
            vec!["Name".to_string(), "Age".to_string()],
            vec!["Alice".to_string(), "30".to_string()],
            vec!["Bob".to_string(), "25".to_string()],
        ];

        let markdown = RstExtractor::cells_to_markdown(&cells);
        assert!(markdown.contains("Name"));
        assert!(markdown.contains("Age"));
        assert!(markdown.contains("Alice"));
        assert!(markdown.contains("Bob"));
        assert!(markdown.contains("---"));
    }

    #[test]
    fn test_rst_extractor_default() {
        let extractor = RstExtractor;
        assert_eq!(extractor.name(), "rst-extractor");
    }

    #[test]
    fn test_rst_extractor_initialize_shutdown() {
        let extractor = RstExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_rst_inject_placeholders_true() {
        let rst = "Some text\n\n.. image:: photo.png\n   :alt: A photo\n\nMore text\n";
        let doc = RstExtractor::build_internal_document(rst, true);
        let has_image = doc.elements.iter().any(|e| e.text.contains("[image:"));
        assert!(has_image, "expected image placeholder with inject_placeholders=true");
    }

    #[test]
    fn test_rst_inject_placeholders_false() {
        let rst = "Some text\n\n.. image:: photo.png\n   :alt: A photo\n\nMore text\n";
        let doc = RstExtractor::build_internal_document(rst, false);
        let has_image = doc.elements.iter().any(|e| e.text.contains("[image:"));
        assert!(
            !has_image,
            "expected no image placeholder with inject_placeholders=false"
        );
    }
}
