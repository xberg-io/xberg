//! Native Rust Typst document extractor.
//!
//! This extractor provides Typst document parsing and text extraction.
//! It uses a hybrid approach combining regex patterns and character-level parsing
//! to extract text while preserving document structure.
//!
//! Features:
//! - Metadata extraction: title, author, date, subject, keywords from `#set document()`
//! - Section hierarchy: `=`, `==`, `===`, etc. heading levels
//! - Inline formatting: `*bold*`, `_italic_`, `` `code` ``
//! - Lists: extraction of list content (both `+` and `-` markers)
//! - Links: extraction of URLs and link text from `#link("url")[text]` syntax
//! - Math: inline (`$...$`) and display math preservation
//! - Code blocks: triple-backtick code blocks with language specifiers
//! - Tables: extraction of `#table()` function content
//! - Complex formatting: handling of nested and combined formatting
//!
//! Requires the `office` feature.

#[cfg(feature = "office")]
use crate::Result;
#[cfg(feature = "office")]
use crate::core::config::ExtractionConfig;
#[cfg(feature = "office")]
use crate::plugins::{DocumentExtractor, Plugin};
#[cfg(feature = "office")]
use crate::types::Metadata;
#[cfg(feature = "office")]
use crate::types::builder;
#[cfg(feature = "office")]
use crate::types::document_structure::TextAnnotation;
#[cfg(feature = "office")]
use crate::types::internal::InternalDocument;
#[cfg(feature = "office")]
use crate::types::internal_builder::InternalDocumentBuilder;
#[cfg(feature = "office")]
use crate::types::uri::Uri;
#[cfg(feature = "office")]
use async_trait::async_trait;
#[cfg(feature = "office")]
use regex::Regex;
#[cfg(feature = "office")]
use std::sync::LazyLock;

#[cfg(feature = "office")]
static IMAGE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"#image\("([^"]*)""#).unwrap());
#[cfg(feature = "office")]
static COLUMNS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"columns:\s*(\d+)").unwrap());
#[cfg(feature = "office")]
static LINK_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^#link\("([^"]*)"\)\[([^\]]*)\]"#).unwrap());

/// Typst document extractor
#[cfg(feature = "office")]
pub struct TypstExtractor;

#[cfg(feature = "office")]
impl TypstExtractor {
    /// Create a new Typst extractor.
    pub(crate) fn new() -> Self {
        Self
    }

    /// Parse Typst content and extract text.
    fn extract_from_typst(content: &str) -> (String, Metadata) {
        let mut extractor = TypstParser::new(content);
        let text = extractor.parse();
        let metadata = extractor.metadata;

        (text, metadata)
    }

    /// Build an `InternalDocument` from Typst source text.
    ///
    /// Extracts headings, code blocks, tables, footnotes, images, and paragraphs
    /// with inline annotations.
    fn build_internal_document(content: &str) -> InternalDocument {
        let mut builder = InternalDocumentBuilder::new("typst");
        let mut in_code_block = false;
        let mut code_text = String::new();
        let mut code_lang: Option<String> = None;
        let mut in_set_document = false;
        let mut paren_depth: i32 = 0;
        let mut paragraph_buf = String::new();
        // Track multi-line #table() accumulation
        let mut in_table = false;
        let mut table_buf = String::new();
        let mut table_paren_depth: i32 = 0;
        let mut table_bracket_depth: i32 = 0;
        let mut footnote_counter: u32 = 0;
        // Track active list state: Some(is_ordered) when inside a list
        let mut active_list: Option<bool> = None;

        let lines: Vec<&str> = content.lines().collect();
        let mut line_idx = 0;

        while line_idx < lines.len() {
            let trimmed = lines[line_idx].trim();
            line_idx += 1;

            // Accumulate multi-line #table() blocks
            if in_table {
                table_buf.push('\n');
                table_buf.push_str(trimmed);
                for ch in trimmed.chars() {
                    match ch {
                        '(' => table_paren_depth += 1,
                        ')' => table_paren_depth -= 1,
                        '[' => table_bracket_depth += 1,
                        ']' => table_bracket_depth -= 1,
                        _ => {}
                    }
                }
                if table_paren_depth <= 0 && table_bracket_depth <= 0 {
                    in_table = false;
                    Self::emit_table_internal(&table_buf, &mut builder);
                    table_buf.clear();
                }
                continue;
            }

            // Skip multi-line #set document(...) blocks
            if in_set_document {
                for ch in trimmed.chars() {
                    match ch {
                        '(' => paren_depth += 1,
                        ')' => paren_depth -= 1,
                        _ => {}
                    }
                }
                if paren_depth <= 0 {
                    in_set_document = false;
                    paren_depth = 0;
                }
                continue;
            }

            // Code block handling
            if trimmed.starts_with("```") {
                if in_code_block {
                    if trimmed == "```" {
                        in_code_block = false;
                        let text = code_text.trim_end().to_string();
                        if !text.is_empty() {
                            builder.push_code(&text, code_lang.as_deref(), None, None);
                        }
                        code_text.clear();
                        code_lang = None;
                        continue;
                    }
                } else {
                    Self::flush_paragraph_internal(&mut paragraph_buf, &mut builder);
                    in_code_block = true;
                    code_text.clear();
                    code_lang = trimmed.strip_prefix("```").and_then(|l| {
                        let l = l.trim();
                        if l.is_empty() { None } else { Some(l.to_string()) }
                    });
                    continue;
                }
            }

            if in_code_block {
                code_text.push_str(lines[line_idx - 1]);
                code_text.push('\n');
                continue;
            }

            // Skip #set document(...)
            if trimmed.starts_with("#set document(") {
                Self::flush_paragraph_internal(&mut paragraph_buf, &mut builder);
                paren_depth = 0;
                for ch in trimmed.chars() {
                    match ch {
                        '(' => paren_depth += 1,
                        ')' => paren_depth -= 1,
                        _ => {}
                    }
                }
                if paren_depth > 0 {
                    in_set_document = true;
                }
                continue;
            }

            // Skip directives
            if trimmed.starts_with("#set ")
                || trimmed.starts_with("#let ")
                || trimmed.starts_with("#import ")
                || trimmed.starts_with("#include ")
                || trimmed.starts_with("#pagebreak")
                || trimmed.starts_with("#colbreak")
                || trimmed.starts_with("#v(")
                || trimmed.starts_with("#h(")
            {
                continue;
            }

            // List items
            if (trimmed.starts_with('+') || trimmed.starts_with('-'))
                && trimmed.len() > 1
                && trimmed.chars().nth(1).is_some_and(|c| !c.is_alphanumeric())
            {
                Self::flush_paragraph_internal(&mut paragraph_buf, &mut builder);
                let ordered = trimmed.starts_with('+');
                // Open a new list if none is active, or if the type changed
                match active_list {
                    Some(prev_ordered) if prev_ordered == ordered => {}
                    _ => {
                        // Close previous list if type changed
                        if active_list.is_some() {
                            builder.end_list();
                        }
                        builder.push_list(ordered);
                        active_list = Some(ordered);
                    }
                }
                builder.push_list_item(trimmed[1..].trim(), ordered, vec![], None, None);
                continue;
            }

            // Any non-list line ends the active list
            if active_list.is_some() {
                builder.end_list();
                active_list = None;
            }

            // Headings
            if trimmed.starts_with('=') {
                let heading_level = trimmed.chars().take_while(|&c| c == '=').count();
                let heading_text = trimmed[heading_level..].trim();
                if !heading_text.is_empty() {
                    Self::flush_paragraph_internal(&mut paragraph_buf, &mut builder);
                    // Preserve Typst heading markers in the extracted text (e.g., "= " prefix)
                    let markers = "=".repeat(heading_level);
                    let full_heading = format!("{} {}", markers, heading_text);
                    builder.push_heading(heading_level as u8, &full_heading, None, None);
                }
                continue;
            }

            // Math blocks
            if trimmed.starts_with('$') && trimmed.ends_with('$') && trimmed.len() > 1 {
                Self::flush_paragraph_internal(&mut paragraph_buf, &mut builder);
                let math = trimmed.trim_matches('$').trim();
                if !math.is_empty() {
                    builder.push_formula(math, None, None);
                }
                continue;
            }

            // Empty lines flush paragraph
            if trimmed.is_empty() {
                Self::flush_paragraph_internal(&mut paragraph_buf, &mut builder);
                continue;
            }

            // #table() — start accumulation (may span multiple lines)
            if trimmed.starts_with("#table(") {
                Self::flush_paragraph_internal(&mut paragraph_buf, &mut builder);
                table_buf.clear();
                table_buf.push_str(trimmed);
                table_paren_depth = 0;
                table_bracket_depth = 0;
                for ch in trimmed.chars() {
                    match ch {
                        '(' => table_paren_depth += 1,
                        ')' => table_paren_depth -= 1,
                        '[' => table_bracket_depth += 1,
                        ']' => table_bracket_depth -= 1,
                        _ => {}
                    }
                }
                if table_paren_depth > 0 || table_bracket_depth > 0 {
                    in_table = true;
                } else {
                    Self::emit_table_internal(&table_buf, &mut builder);
                    table_buf.clear();
                }
                continue;
            }

            // #footnote[text] — extract footnote
            if trimmed.starts_with("#footnote[") {
                Self::flush_paragraph_internal(&mut paragraph_buf, &mut builder);
                if let Some(text) = Self::extract_bracket_content(trimmed, "#footnote[") {
                    footnote_counter += 1;
                    let key = format!("fn-{}", footnote_counter);
                    builder.push_footnote_definition(&text, &key, None);
                }
                continue;
            }

            // #image("path") — extract image description
            if trimmed.starts_with("#image(") {
                Self::flush_paragraph_internal(&mut paragraph_buf, &mut builder);
                let image_path = IMAGE_RE.captures(trimmed).and_then(|c| c.get(1)).map(|m| m.as_str());
                if let Some(path) = image_path {
                    builder.push_uri(Uri::image(path, None));
                }
                let description = image_path.map(|p| format!("[Image: {}]", p));
                let desc_text = description.unwrap_or_else(|| "[Image]".to_string());
                builder.push_paragraph(&desc_text, vec![], None, None);
                continue;
            }

            // Regular text accumulation
            if !paragraph_buf.is_empty() {
                paragraph_buf.push(' ');
            }
            paragraph_buf.push_str(trimmed);
        }

        // Close any open list at end of processing
        if active_list.is_some() {
            builder.end_list();
        }
        Self::flush_paragraph_internal(&mut paragraph_buf, &mut builder);
        builder.build()
    }

    /// Flush accumulated paragraph text into the internal builder,
    /// parsing inline annotations (bold, italic, code, links).
    fn flush_paragraph_internal(buf: &mut String, builder: &mut InternalDocumentBuilder) {
        if !buf.is_empty() {
            let (text, annotations) = Self::parse_inline_annotations(buf.trim());
            // Extract URIs from link annotations
            for ann in &annotations {
                if let crate::types::document_structure::AnnotationKind::Link { url, .. } = &ann.kind
                    && !url.is_empty()
                {
                    let label = text.get(ann.start as usize..ann.end as usize).map(|s| s.to_string());
                    builder.push_uri(Uri::hyperlink(url, label));
                }
            }
            builder.push_paragraph(&text, annotations, None, None);
            buf.clear();
        }
    }

    /// Parse a `#table(...)` block and emit it as a table element in the internal builder.
    fn emit_table_internal(table_str: &str, builder: &mut InternalDocumentBuilder) {
        // Extract column count from `columns: N`
        let num_cols = COLUMNS_RE
            .captures(table_str)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse::<usize>().ok())
            .unwrap_or(0);

        // Collect all cell texts from [content] brackets
        let mut cells: Vec<String> = Vec::new();
        let mut in_bracket = false;
        let mut cell = String::new();
        for ch in table_str.chars() {
            match ch {
                '[' => {
                    in_bracket = true;
                    cell.clear();
                }
                ']' if in_bracket => {
                    cells.push(cell.trim().to_string());
                    in_bracket = false;
                    cell.clear();
                }
                _ if in_bracket => {
                    cell.push(ch);
                }
                _ => {}
            }
        }

        if cells.is_empty() {
            return;
        }

        // Arrange cells into rows
        let effective_cols = if num_cols > 0 { num_cols } else { cells.len() };
        let rows: Vec<Vec<String>> = cells.chunks(effective_cols).map(|chunk| chunk.to_vec()).collect();
        builder.push_table_from_cells(&rows, None, None);
    }

    /// Parse inline formatting markers in paragraph text, producing stripped text
    /// and annotations for bold (`*...*`), italic (`_..._`), code (`` `...` ``),
    /// and links (`#link("url")[text]`).
    fn parse_inline_annotations(raw: &str) -> (String, Vec<TextAnnotation>) {
        let mut text = String::with_capacity(raw.len());
        let mut annotations = Vec::new();
        let mut byte_pos = 0;

        while byte_pos < raw.len() {
            // Handle #link("url")[text]
            if raw.as_bytes()[byte_pos] == b'#'
                && raw[byte_pos..].starts_with("#link(\"")
                && let Some((url, display, consumed)) = Self::parse_link_at(&raw[byte_pos..])
            {
                let start = text.len() as u32;
                text.push_str(&display);
                let end = text.len() as u32;
                annotations.push(builder::link(start, end, &url, None));
                byte_pos += consumed;
                continue;
            }

            // Handle #footnote[text] inline (emit text in brackets as-is with a
            // footnote marker — but since footnotes are block-level, we skip
            // inline footnotes in paragraph text).

            // Decode the current character and its byte length
            let ch = &raw[byte_pos..];
            let c = ch.chars().next().unwrap();
            let c_len = c.len_utf8();

            match c {
                '*' => {
                    // Bold: find matching closing *
                    if let Some(close_byte) = Self::find_closing_marker_byte(raw, byte_pos + c_len, b'*') {
                        let start = text.len() as u32;
                        text.push_str(&raw[byte_pos + c_len..close_byte]);
                        let end = text.len() as u32;
                        if end > start {
                            annotations.push(builder::bold(start, end));
                        }
                        byte_pos = close_byte + 1; // skip closing '*'
                    } else {
                        text.push('*');
                        byte_pos += c_len;
                    }
                }
                '_' => {
                    // Italic: find matching closing _
                    if let Some(close_byte) = Self::find_closing_marker_byte(raw, byte_pos + c_len, b'_') {
                        let start = text.len() as u32;
                        text.push_str(&raw[byte_pos + c_len..close_byte]);
                        let end = text.len() as u32;
                        if end > start {
                            annotations.push(builder::italic(start, end));
                        }
                        byte_pos = close_byte + 1; // skip closing '_'
                    } else {
                        text.push('_');
                        byte_pos += c_len;
                    }
                }
                '`' => {
                    // Code: find matching closing `
                    if let Some(close_byte) = Self::find_closing_marker_byte(raw, byte_pos + c_len, b'`') {
                        let start = text.len() as u32;
                        text.push_str(&raw[byte_pos + c_len..close_byte]);
                        let end = text.len() as u32;
                        if end > start {
                            annotations.push(builder::code(start, end));
                        }
                        byte_pos = close_byte + 1; // skip closing '`'
                    } else {
                        text.push('`');
                        byte_pos += c_len;
                    }
                }
                _ => {
                    text.push(c);
                    byte_pos += c_len;
                }
            }
        }

        (text, annotations)
    }

    /// Find the byte index of a closing ASCII marker character starting from byte position `start`.
    fn find_closing_marker_byte(raw: &str, start: usize, marker: u8) -> Option<usize> {
        let bytes = raw.as_bytes();
        (start..bytes.len()).find(|&idx| bytes[idx] == marker)
    }

    /// Parse a `#link("url")[text]` pattern at the beginning of a string slice.
    /// Returns `(url, display_text, byte_count_consumed)` on success.
    fn parse_link_at(s: &str) -> Option<(String, String, usize)> {
        let caps = LINK_RE.captures(s)?;
        let url = caps.get(1)?.as_str().to_string();
        let display = caps.get(2)?.as_str().to_string();
        let consumed = caps.get(0)?.end();
        Some((url, display, consumed))
    }

    /// Extract content between the first `[` after the prefix and the matching `]`.
    fn extract_bracket_content(s: &str, prefix: &str) -> Option<String> {
        let after = s.strip_prefix(prefix)?;
        let end = after.find(']')?;
        Some(after[..end].to_string())
    }
}

#[cfg(feature = "office")]
impl Default for TypstExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "office")]
impl Plugin for TypstExtractor {
    fn name(&self) -> &str {
        "typst-extractor"
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
        "Native Rust Typst document extractor with metadata support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for TypstExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let _ = config;
        let typst_str = String::from_utf8_lossy(content).into_owned();
        let (_text, metadata) = Self::extract_from_typst(&typst_str);

        let mut doc = Self::build_internal_document(&typst_str);
        doc.mime_type = mime_type.to_string();
        doc.metadata = metadata;

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
        &["application/x-typst", "text/x-typst"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

/// Internal Typst parser
#[cfg(feature = "office")]
struct TypstParser {
    content: String,
    metadata: Metadata,
}

#[cfg(feature = "office")]
impl TypstParser {
    fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            metadata: Metadata::default(),
        }
    }

    fn parse(&mut self) -> String {
        self.extract_metadata();

        self.extract_content()
    }

    fn extract_metadata(&mut self) {
        if let Some(title) = self.extract_quoted_value("title") {
            self.metadata.title = Some(title);
        }

        if let Some(author) = self.extract_quoted_value("author") {
            self.metadata.authors = Some(vec![author]);
        }

        if let Some(date) = self.extract_quoted_value("date") {
            self.metadata.created_at = Some(date);
        }

        if let Some(subject) = self.extract_quoted_value("subject") {
            self.metadata.subject = Some(subject);
        }

        if let Some(keywords) = self.extract_keywords() {
            self.metadata.keywords = Some(keywords);
        }
    }

    fn extract_quoted_value(&self, field: &str) -> Option<String> {
        let pattern = format!(r#"{}:\s*"([^"]*)""#, regex::escape(field));
        if let Ok(re) = Regex::new(&pattern)
            && let Some(caps) = re.captures(&self.content)
        {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        None
    }

    fn extract_keywords(&self) -> Option<Vec<String>> {
        let pattern = r#"keywords:\s*(?:"([^"]*)"|(\([^)]*\)))"#;
        if let Ok(re) = Regex::new(pattern)
            && let Some(caps) = re.captures(&self.content)
        {
            // Single quoted string: split by comma
            if let Some(m) = caps.get(1) {
                let keywords: Vec<String> = m
                    .as_str()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !keywords.is_empty() {
                    return Some(keywords);
                }
            }
            // Array form: ("keyword1", "keyword2")
            if let Some(m) = caps.get(2) {
                let array_str = m.as_str();
                let mut keywords = Vec::new();
                let item_pattern = r#""([^"]*)""#;
                if let Ok(item_re) = Regex::new(item_pattern) {
                    for item_caps in item_re.captures_iter(array_str) {
                        if let Some(keyword) = item_caps.get(1) {
                            keywords.push(keyword.as_str().to_string());
                        }
                    }
                }
                if !keywords.is_empty() {
                    return Some(keywords);
                }
            }
        }
        None
    }

    fn extract_content(&self) -> String {
        let mut output = String::new();
        let mut lines = self.content.lines().peekable();
        let mut in_code_block = false;
        let mut code_block_fence = String::new();
        let mut in_set_document = false;
        let mut paren_depth: i32 = 0;

        while let Some(line) = lines.next() {
            let trimmed = line.trim();

            // Skip multi-line #set document(...) blocks
            if in_set_document {
                for ch in trimmed.chars() {
                    match ch {
                        '(' => paren_depth += 1,
                        ')' => paren_depth -= 1,
                        _ => {}
                    }
                }
                if paren_depth <= 0 {
                    in_set_document = false;
                    paren_depth = 0;
                }
                continue;
            }

            if trimmed.starts_with("```") {
                if in_code_block {
                    if trimmed == "```" {
                        in_code_block = false;
                        code_block_fence.clear();
                        output.push_str("```\n");
                        continue;
                    }
                } else {
                    in_code_block = true;
                    code_block_fence = "```".to_string();
                    output.push_str("```");
                    if let Some(lang) = trimmed.strip_prefix("```") {
                        let lang = lang.trim();
                        if !lang.is_empty() {
                            output.push_str(lang);
                        }
                    }
                    output.push('\n');
                    continue;
                }
            }

            if in_code_block {
                output.push_str(line);
                output.push('\n');
                continue;
            }

            // Skip #set document(...) - may span multiple lines
            if trimmed.starts_with("#set document(") {
                paren_depth = 0;
                for ch in trimmed.chars() {
                    match ch {
                        '(' => paren_depth += 1,
                        ')' => paren_depth -= 1,
                        _ => {}
                    }
                }
                if paren_depth > 0 {
                    in_set_document = true;
                }
                continue;
            }

            if trimmed.starts_with("#set ") || trimmed.starts_with("#let ") {
                continue;
            }

            if trimmed.starts_with("#import ") || trimmed.starts_with("#include ") {
                continue;
            }

            // Skip layout directives
            if trimmed.starts_with("#pagebreak")
                || trimmed.starts_with("#colbreak")
                || trimmed.starts_with("#v(")
                || trimmed.starts_with("#h(")
            {
                continue;
            }

            if trimmed.starts_with("#table(") {
                let table_content = self.extract_table_content(trimmed, &mut lines);
                output.push_str(&table_content);
                output.push('\n');
                continue;
            }

            if trimmed.starts_with('=') {
                let next_char_pos = trimmed.find(|c: char| c != '=');
                if next_char_pos.is_some() {
                    let heading_level = trimmed.chars().take_while(|&c| c == '=').count();
                    let heading_text = trimmed[heading_level..].trim();

                    output.push_str(heading_text);
                    let _ = heading_level;
                    output.push('\n');
                    continue;
                }
            }

            if (trimmed.starts_with('+') || trimmed.starts_with('-'))
                && trimmed.len() > 1
                && trimmed.chars().nth(1).is_some_and(|c| !c.is_alphanumeric())
            {
                output.push_str("- ");
                output.push_str(trimmed[1..].trim());
                output.push('\n');
                continue;
            }

            if trimmed.starts_with('#')
                && !trimmed.starts_with("#set")
                && !trimmed.starts_with("#let")
                && !trimmed.starts_with("#import")
                && !trimmed.starts_with("#include")
            {
                if trimmed.contains('[')
                    && trimmed.contains(']')
                    && let Some(content) = self.extract_text_from_brackets(trimmed)
                {
                    let processed = self.process_line(&content);
                    if !processed.is_empty() {
                        output.push_str(&processed);
                        output.push('\n');
                    }
                }
                continue;
            }

            if !trimmed.is_empty() {
                let processed = self.process_line(trimmed);
                if !processed.is_empty() {
                    output.push_str(&processed);
                    output.push('\n');
                }
            } else {
                output.push('\n');
            }
        }

        output
    }

    /// Extract content from #table() function calls
    fn extract_table_content<'a, I>(&self, first_line: &str, lines: &mut std::iter::Peekable<I>) -> String
    where
        I: Iterator<Item = &'a str>,
    {
        let mut content = first_line.to_string();
        let mut bracket_depth = 0;
        let mut paren_depth = 0;

        for ch in first_line.chars() {
            match ch {
                '(' => paren_depth += 1,
                ')' => paren_depth -= 1,
                '[' => bracket_depth += 1,
                ']' => bracket_depth -= 1,
                _ => {}
            }
        }

        while paren_depth > 0 || bracket_depth > 0 {
            if let Some(next_line) = lines.next() {
                content.push('\n');
                content.push_str(next_line);
                for ch in next_line.chars() {
                    match ch {
                        '(' => paren_depth += 1,
                        ')' => paren_depth -= 1,
                        '[' => bracket_depth += 1,
                        ']' => bracket_depth -= 1,
                        _ => {}
                    }
                }
            } else {
                break;
            }
        }

        // Extract column count from `columns: N`
        let num_cols = COLUMNS_RE
            .captures(&content)
            .and_then(|caps| caps.get(1))
            .and_then(|m| m.as_str().parse::<usize>().ok())
            .unwrap_or(0);

        // Collect all cell texts
        let mut cells: Vec<String> = Vec::new();
        let mut in_bracket = false;
        let mut cell = String::new();
        for ch in content.chars() {
            match ch {
                '[' => {
                    in_bracket = true;
                    cell.clear();
                }
                ']' if in_bracket => {
                    let trimmed = cell.trim().to_string();
                    cells.push(trimmed);
                    in_bracket = false;
                    cell.clear();
                }
                _ if in_bracket => {
                    cell.push(ch);
                }
                _ => {}
            }
        }

        // Arrange cells into rows
        let mut table_content = String::new();
        if num_cols > 0 && !cells.is_empty() {
            for (i, cell_text) in cells.iter().enumerate() {
                if i > 0 && i % num_cols == 0 {
                    table_content.push('\n');
                }
                if i % num_cols > 0 {
                    table_content.push('\t');
                }
                table_content.push_str(cell_text);
            }
        } else {
            // Fallback: join all cells with separator
            table_content = cells.join(" | ");
        }

        table_content
    }

    fn process_line(&self, line: &str) -> String {
        let mut result = String::new();
        let mut chars = line.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '`' => {
                    result.push('`');
                    for c in chars.by_ref() {
                        result.push(c);
                        if c == '`' {
                            break;
                        }
                    }
                }
                '$' => {
                    result.push('$');
                    for c in chars.by_ref() {
                        result.push(c);
                        if c == '$' {
                            break;
                        }
                    }
                }
                '*' => {
                    result.push('*');
                    for c in chars.by_ref() {
                        result.push(c);
                        if c == '*' {
                            break;
                        }
                    }
                }
                '_' => {
                    result.push('_');
                    for c in chars.by_ref() {
                        result.push(c);
                        if c == '_' {
                            break;
                        }
                    }
                }
                '#' => {
                    // Skip the # prefix for Typst function calls like #link
                    // The link extraction happens later in extract_link_text
                    result.push(ch);
                }
                _ => {
                    result.push(ch);
                }
            }
        }

        self.extract_link_text(&result)
    }

    fn extract_link_text(&self, line: &str) -> String {
        // Handle #link("url")[text] pattern - extract just the display text
        let pattern = r#"#?link\("([^"]*)"\)\[([^\]]*)\]"#;
        if let Ok(re) = Regex::new(pattern) {
            return re
                .replace_all(line, |caps: &regex::Captures| {
                    let text = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                    text.to_string()
                })
                .to_string();
        }
        line.to_string()
    }

    fn extract_text_from_brackets(&self, line: &str) -> Option<String> {
        if let Some(start) = line.find('[')
            && let Some(end) = line.rfind(']')
            && end > start
        {
            let text = &line[start + 1..end];
            return Some(text.to_string());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_metadata() {
        let content = r#"#set document(
            title: "Test Document",
            author: "Test Author"
        )

        = Heading
        Some text
        "#;

        let (_, metadata) = TypstExtractor::extract_from_typst(content);

        assert!(metadata.title.is_some(), "Title should be extracted");
        assert_eq!(metadata.title.as_deref(), Some("Test Document"));
        assert!(metadata.authors.is_some(), "Author should be extracted");
        assert_eq!(metadata.authors.as_deref(), Some(&["Test Author".to_string()][..]));
    }

    #[test]
    fn test_extract_headings() {
        let content = r#"= Level 1
Content

== Level 2
More content
"#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("Level 1"));
        assert!(output.contains("Level 2"));
    }

    #[test]
    fn test_extract_formatting() {
        let content = r#"Some *bold* and _italic_ text with `code`."#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("*bold*") || output.contains("bold"));
        assert!(output.contains("_italic_") || output.contains("italic"));
        assert!(output.contains("`code`") || output.contains("code"));
    }

    #[test]
    fn test_extract_code_blocks() {
        let content = r#"Here is code:

```python
def hello():
    print("world")
```

Done."#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("```python"));
        assert!(output.contains("def hello"));
        assert!(output.contains("print"));
    }

    #[test]
    fn test_extract_links() {
        let content = r#"Visit #link("https://example.com")[example site] for info."#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(
            output.contains("example.com")
                || output.contains("example site")
                || output.contains("[example site](https://example.com)")
        );
    }

    #[test]
    fn test_extract_list_items() {
        let content = r#"= Lists

+ First item
+ Second item
+ Third item"#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("First item"));
        assert!(output.contains("Second item"));
        assert!(output.contains("Third item"));
    }

    #[test]
    fn test_extract_tables() {
        let content = r#"== Tables

#table(
  columns: 2,
  [Name], [Age],
  [Alice], [30],
)"#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("TABLE:") || output.contains("Name") || output.contains("Alice"));
    }

    #[test]
    fn test_extract_math() {
        let content = r#"The formula $E = mc^2$ is important.

Display:
$ a^2 + b^2 = c^2 $"#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("$") && output.contains("mc"));
    }

    #[test]
    fn test_metadata_extraction_comprehensive() {
        let content = r#"#set document(
            title: "Advanced Document",
            author: "John Doe",
            date: "2024-12-06",
            subject: "Test Subject",
            keywords: ("test", "example", "rust")
        )

        Content here."#;

        let (_, metadata) = TypstExtractor::extract_from_typst(content);

        assert_eq!(
            metadata.title.as_deref(),
            Some("Advanced Document"),
            "Title should be extracted"
        );
        assert!(metadata.authors.is_some(), "Author should be extracted");
        assert_eq!(metadata.authors.as_deref(), Some(&["John Doe".to_string()][..]));
        assert!(metadata.created_at.is_some(), "Date should be extracted");
        assert_eq!(
            metadata.subject.as_deref(),
            Some("Test Subject"),
            "Subject should be extracted"
        );
        assert!(metadata.keywords.is_some(), "Keywords should be extracted");
        let keywords = metadata.keywords.unwrap();
        assert_eq!(keywords, vec!["test", "example", "rust"]);
    }

    #[test]
    fn test_skip_directives() {
        let content = r#"#set heading(numbering: "1.")
#let x = 5
#import "@preview/foo:1.0"
#include "other.typ"

= Heading
Actual content"#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(!output.contains("#set"));
        assert!(!output.contains("#let"));
        assert!(!output.contains("#import"));
        assert!(!output.contains("#include"));
        assert!(output.contains("Heading"));
        assert!(output.contains("content"));
    }

    #[test]
    fn test_combined_formatting() {
        let content = r#"This is *bold with _nested italic_* and more."#;

        let (output, _) = TypstExtractor::extract_from_typst(content);

        assert!(output.contains("*") || output.contains("_") || (output.contains("bold") && output.contains("italic")));
    }

    #[test]
    fn test_cjk_with_inline_formatting() {
        // CJK text with bold markers — must not panic on multi-byte chars
        let content = "これは*太字*テスト";
        let (output, _) = TypstExtractor::extract_from_typst(content);
        assert!(output.contains("太字"), "Bold content should be present");
        assert!(output.contains("これは"), "Leading CJK text preserved");
        assert!(output.contains("テスト"), "Trailing CJK text preserved");
    }

    #[test]
    fn test_emoji_with_inline_formatting() {
        let content = "Hello 🎉 *bold* world 🌍";
        let (output, _) = TypstExtractor::extract_from_typst(content);
        assert!(output.contains("🎉"), "Emoji preserved");
        assert!(output.contains("bold"), "Bold content present");
        assert!(output.contains("🌍"), "Trailing emoji preserved");
    }
}
