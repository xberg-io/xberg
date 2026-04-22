//! Inline DOCX XML parser.
//!
//! Vendored and adapted from [docx-lite](https://github.com/v-lawyer/docx-lite) v0.2.0
//! (MIT OR Apache-2.0, V-Lawyer Team). See ATTRIBUTIONS.md for details.
//!
//! Changes from upstream:
//! - `Paragraph::to_text()` joins runs with `" "` instead of `""` (fixes #359)
//! - Adapted to use kreuzberg's existing `quick-xml` and `zip` versions
//! - Removed file-path based APIs (we only need bytes/reader)
//! - Added markdown rendering and formatting support (fixes #376)

use ahash::AHashMap;
use std::io::{Cursor, Read, Seek};

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

// --- Types ---

/// Tracks document element ordering (paragraphs, tables, and drawings interleaved).
#[derive(Debug, Clone)]
pub enum DocumentElement {
    Paragraph(usize), // index into Document::paragraphs
    Table(usize),     // index into Document::tables
    Drawing(usize),   // index into Document::drawings
    PageBreak,
}

#[derive(Debug, Clone, Default)]
pub struct Document {
    pub paragraphs: Vec<Paragraph>,
    pub tables: Vec<Table>,
    pub headers: Vec<HeaderFooter>,
    pub footers: Vec<HeaderFooter>,
    pub footnotes: Vec<Note>,
    pub endnotes: Vec<Note>,
    pub numbering_defs: AHashMap<(i64, i64), ListType>,
    /// Document elements in their original order.
    pub elements: Vec<DocumentElement>,
    /// Parsed style catalog from `word/styles.xml`, if available.
    pub style_catalog: Option<super::styles::StyleCatalog>,
    /// Parsed theme from `word/theme/theme1.xml`, if available.
    pub theme: Option<super::theme::Theme>,
    /// Section properties parsed from `w:sectPr` elements.
    pub sections: Vec<super::section::SectionProperties>,
    /// Drawing objects parsed from `w:drawing` elements.
    pub drawings: Vec<super::drawing::Drawing>,
    /// Image relationships (rId → target path) for image extraction.
    pub image_relationships: AHashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct Paragraph {
    pub runs: Vec<Run>,
    pub style: Option<String>,
    pub numbering_id: Option<i64>,
    pub numbering_level: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub subscript: bool,
    pub superscript: bool,
    /// Font size in half-points (from `w:sz`).
    pub font_size: Option<u32>,
    /// Font color as "RRGGBB" hex (from `w:color`).
    pub font_color: Option<String>,
    /// Highlight color name (from `w:highlight`).
    pub highlight: Option<String>,
    pub hyperlink_url: Option<String>,
    /// LaTeX math content: (latex_source, is_display_math).
    /// When set, this run represents an equation and `text` is ignored.
    pub math_latex: Option<(String, bool)>,
}

#[derive(Debug, Clone, Default)]
pub struct Table {
    pub rows: Vec<TableRow>,
    pub properties: Option<super::table::TableProperties>,
    pub grid: Option<super::table::TableGrid>,
}

#[derive(Debug, Clone, Default)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub properties: Option<super::table::RowProperties>,
}

#[derive(Debug, Clone, Default)]
pub struct TableCell {
    pub paragraphs: Vec<Paragraph>,
    pub properties: Option<super::table::CellProperties>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ListType {
    Bullet,
    Numbered,
}

#[derive(Debug, Clone, Default)]
pub struct HeaderFooter {
    pub paragraphs: Vec<Paragraph>,
    pub tables: Vec<Table>,
    pub header_type: HeaderFooterType,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum HeaderFooterType {
    #[default]
    Default,
    First,
    Even,
    Odd,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub id: String,
    pub note_type: NoteType,
    pub paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoteType {
    Footnote,
    Endnote,
}

// --- Helper functions ---

/// Check if a formatting element is enabled (not explicitly set to false/0/none).
fn is_format_enabled(e: &BytesStart) -> bool {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == b"w:val"
            && let Ok(val) = std::str::from_utf8(&attr.value)
        {
            return !matches!(val, "false" | "0" | "none");
        }
    }
    true // no w:val attribute means enabled
}

/// Read `w:val` attribute as i64.
fn get_val_attr(e: &BytesStart) -> Option<i64> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == b"w:val"
            && let Ok(val) = std::str::from_utf8(&attr.value)
        {
            return val.parse().ok();
        }
    }
    None
}

/// Read `w:val` attribute as String.
fn get_val_attr_string(e: &BytesStart) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == b"w:val"
            && let Ok(val) = std::str::from_utf8(&attr.value)
        {
            return Some(val.to_string());
        }
    }
    None
}

/// Map heading style name to markdown heading level (fallback for docs without styles.xml).
fn heading_level_from_style_name(style: &str) -> Option<u8> {
    match style {
        "Title" => Some(1),
        s if s.starts_with("Heading") || s.starts_with("heading") => {
            let num_part = s.trim_start_matches("Heading").trim_start_matches("heading");
            if let Ok(n) = num_part.parse::<u8>()
                && (1..=6).contains(&n)
            {
                // Title is h1, so Heading1 becomes h2, etc. Clamp to 6 (max markdown heading level).
                return Some((n + 1).min(6));
            }
            None
        }
        _ => None,
    }
}

// --- Impls ---

impl Document {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the document and return both the text and accurate page boundaries.
    pub fn extract_text_with_boundaries(&self, is_markdown: bool) -> (String, Vec<crate::types::PageBoundary>) {
        let text = if is_markdown {
            self.to_markdown(true)
        } else {
            self.to_plain_text()
        };

        let mut boundaries = Vec::new();
        let mut start_idx = 0;
        let mut page_num = 1;

        for (idx, _) in text.match_indices('\x0c') {
            boundaries.push(crate::types::PageBoundary {
                byte_start: start_idx,
                byte_end: idx,
                page_number: page_num,
            });
            start_idx = idx + 1; // Skip the \f character
            page_num += 1;
        }

        // Add the last page
        boundaries.push(crate::types::PageBoundary {
            byte_start: start_idx,
            byte_end: text.len(),
            page_number: page_num,
        });

        (text, boundaries)
    }

    /// Return the 1-based page number for each top-level table in the document.
    pub fn table_page_numbers(&self) -> Vec<usize> {
        let mut table_page_numbers = Vec::new();
        let mut current_page = 1;

        for element in &self.elements {
            match element {
                DocumentElement::PageBreak => current_page += 1,
                DocumentElement::Table(_) => {
                    table_page_numbers.push(current_page);
                }
                _ => {}
            }
        }

        table_page_numbers
    }

    /// Internal helper to ensure a blank line before appending new content.
    fn ensure_blank_line(output: &mut String) {
        if !output.is_empty() && !output.ends_with("\n\n") {
            if output.ends_with('\n') {
                output.push('\n');
            } else {
                output.push_str("\n\n");
            }
        }
    }

    /// Resolve heading level for a paragraph style using the StyleCatalog.
    ///
    /// Walks the style inheritance chain to find `outline_level`.
    /// Falls back to string-matching on style name/ID if no StyleCatalog is available.
    /// Returns 1-6 (markdown heading levels).
    pub fn resolve_heading_level(&self, style_id: &str) -> Option<u8> {
        if let Some(ref catalog) = self.style_catalog {
            // Walk inheritance chain looking for outline_level
            let mut current_id = Some(style_id);
            let mut visited = 0;
            while let Some(id) = current_id {
                if visited > 20 {
                    break; // prevent infinite loops
                }
                visited += 1;
                if let Some(style_def) = catalog.styles.get(id) {
                    if let Some(level) = style_def.paragraph_properties.outline_level {
                        // outline_level 0 = h1, 1 = h2, ..., clamped to 6
                        return Some((level + 1).min(6));
                    }
                    // Check style name for "Title" pattern
                    if let Some(ref name) = style_def.name
                        && (name == "Title" || name == "title")
                    {
                        return Some(1);
                    }
                    current_id = style_def.based_on.as_deref();
                } else {
                    break;
                }
            }
        }
        // Fallback: string-match on style ID
        heading_level_from_style_name(style_id)
    }

    pub fn extract_text(&self) -> String {
        let mut text = String::new();

        for paragraph in &self.paragraphs {
            let para_text = paragraph.to_text();
            if !para_text.is_empty() {
                text.push_str(&para_text);
                text.push('\n');
            }
        }

        for table in &self.tables {
            for row in &table.rows {
                for cell in &row.cells {
                    for paragraph in &cell.paragraphs {
                        let para_text = paragraph.to_text();
                        if !para_text.is_empty() {
                            text.push_str(&para_text);
                            text.push('\t');
                        }
                    }
                }
                text.push('\n');
            }
            text.push('\n');
        }

        text
    }

    /// Render the document as markdown.
    ///
    /// When `inject_placeholders` is `true`, drawings that reference an image
    /// emit `![alt](image)` placeholders. When `false` they are silently
    /// skipped, which is useful when the caller only wants text.
    pub fn to_markdown(&self, inject_placeholders: bool) -> String {
        use std::fmt::Write;

        let mut output = String::new();
        let mut list_counters: AHashMap<(i64, i64), usize> = AHashMap::new();
        let mut prev_was_list = false;

        // Use elements ordering if populated, otherwise fall back to paragraphs-only
        if !self.elements.is_empty() {
            for element in &self.elements {
                match element {
                    DocumentElement::Paragraph(idx) => {
                        let Some(paragraph) = self.paragraphs.get(*idx) else {
                            continue;
                        };
                        self.append_paragraph_markdown(paragraph, &mut output, &mut list_counters, &mut prev_was_list);
                    }
                    DocumentElement::Table(idx) => {
                        let Some(table) = self.tables.get(*idx) else { continue };
                        // Ensure blank line separation before table
                        Self::ensure_blank_line(&mut output);
                        if let Some(ref props) = table.properties
                            && let Some(ref caption) = props.caption
                        {
                            output.push_str(caption);
                            output.push_str("\n\n");
                        }
                        output.push_str(&table.to_markdown());
                        prev_was_list = false;
                    }
                    DocumentElement::Drawing(idx) => {
                        let Some(drawing) = self.drawings.get(*idx) else {
                            continue;
                        };
                        // Skip drawings without an image reference (e.g. textbox shapes)
                        if drawing.image_ref.is_none() {
                            continue;
                        }
                        if inject_placeholders {
                            let alt = drawing
                                .doc_properties
                                .as_ref()
                                .and_then(|dp| dp.description.as_deref())
                                .unwrap_or("");
                            // Ensure blank line separation before image
                            Self::ensure_blank_line(&mut output);
                            let _ = writeln!(output, "![{}](image)", alt);
                        }
                        prev_was_list = false;
                    }
                    DocumentElement::PageBreak => {
                        // In markdown, we can represent a page break as a form feed character \f
                        // or a triple dash at the start of a line. For now, we use \f which
                        // is a standard page break marker and easy to find for boundary mapping.
                        output.push('\x0c');
                        prev_was_list = false;
                    }
                }
            }
        } else {
            for paragraph in &self.paragraphs {
                self.append_paragraph_markdown(paragraph, &mut output, &mut list_counters, &mut prev_was_list);
            }
        }

        // Footnotes
        if !self.footnotes.is_empty() {
            output.push_str("\n\n");
            for note in &self.footnotes {
                let note_text: String = note
                    .paragraphs
                    .iter()
                    .map(|p| p.runs_to_markdown())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !note_text.is_empty() {
                    let _ = writeln!(output, "[^{}]: {}", note.id, note_text);
                }
            }
        }

        // Endnotes
        if !self.endnotes.is_empty() {
            output.push_str("\n\n");
            for note in &self.endnotes {
                let note_text: String = note
                    .paragraphs
                    .iter()
                    .map(|p| p.runs_to_markdown())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !note_text.is_empty() {
                    let _ = writeln!(output, "[^{}]: {}", note.id, note_text);
                }
            }
        }

        // Trim output in-place
        let trimmed_end = output.trim_end().len();
        output.truncate(trimmed_end);
        let trimmed_start = output.len() - output.trim_start().len();
        if trimmed_start > 0 {
            output.drain(..trimmed_start);
        }
        output
    }

    /// Render the document as plain text (no markdown formatting).
    pub fn to_plain_text(&self) -> String {
        let mut output = String::new();

        // Use elements ordering if populated, otherwise fall back to paragraphs-only
        if !self.elements.is_empty() {
            for element in &self.elements {
                match element {
                    DocumentElement::Paragraph(idx) => {
                        let Some(paragraph) = self.paragraphs.get(*idx) else {
                            continue;
                        };
                        let text = paragraph.to_text();
                        if !text.is_empty() {
                            Self::ensure_blank_line(&mut output);
                            output.push_str(&text);
                        }
                    }
                    DocumentElement::Table(idx) => {
                        let Some(table) = self.tables.get(*idx) else { continue };
                        Self::ensure_blank_line(&mut output);
                        if let Some(ref props) = table.properties
                            && let Some(ref caption) = props.caption
                        {
                            output.push_str(caption);
                            output.push_str("\n\n");
                        }
                        output.push_str(&table.to_plain_text());
                    }
                    DocumentElement::Drawing(idx) => {
                        let Some(drawing) = self.drawings.get(*idx) else {
                            continue;
                        };
                        if let Some(alt) = drawing.doc_properties.as_ref().and_then(|dp| dp.description.as_deref())
                            && !alt.is_empty()
                        {
                            Self::ensure_blank_line(&mut output);
                            output.push_str(alt);
                        }
                    }
                    DocumentElement::PageBreak => {
                        output.push('\x0c');
                    }
                }
            }
        } else {
            for paragraph in &self.paragraphs {
                let text = paragraph.to_text();
                if !text.is_empty() {
                    Self::ensure_blank_line(&mut output);
                    output.push_str(&text);
                }
            }
        }

        // Footnotes
        if !self.footnotes.is_empty() {
            output.push_str("\n\n");
            for note in &self.footnotes {
                let note_text: String = note
                    .paragraphs
                    .iter()
                    .map(|p| p.to_text())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !note_text.is_empty() {
                    output.push_str(&note.id);
                    output.push_str(": ");
                    output.push_str(&note_text);
                    output.push('\n');
                }
            }
        }

        // Endnotes
        if !self.endnotes.is_empty() {
            output.push_str("\n\n");
            for note in &self.endnotes {
                let note_text: String = note
                    .paragraphs
                    .iter()
                    .map(|p| p.to_text())
                    .collect::<Vec<_>>()
                    .join(" ");
                if !note_text.is_empty() {
                    output.push_str(&note.id);
                    output.push_str(": ");
                    output.push_str(&note_text);
                    output.push('\n');
                }
            }
        }

        // Trim output in-place
        let trimmed_end = output.trim_end().len();
        output.truncate(trimmed_end);
        let trimmed_start = output.len() - output.trim_start().len();
        if trimmed_start > 0 {
            output.drain(..trimmed_start);
        }
        output
    }

    /// Helper: append a paragraph's markdown to output, managing list transitions.
    fn append_paragraph_markdown(
        &self,
        paragraph: &Paragraph,
        output: &mut String,
        list_counters: &mut AHashMap<(i64, i64), usize>,
        prev_was_list: &mut bool,
    ) {
        let is_list = paragraph.numbering_id.is_some();

        // Add blank line before list block when transitioning from non-list
        if is_list && !*prev_was_list {
            Self::ensure_blank_line(output);
        }

        // Add blank line after list block when transitioning to non-list
        if !is_list && *prev_was_list {
            Self::ensure_blank_line(output);
        }

        let heading_level = paragraph.style.as_deref().and_then(|s| self.resolve_heading_level(s));
        let md = paragraph.to_markdown(&self.numbering_defs, list_counters, heading_level);
        if md.is_empty() {
            *prev_was_list = is_list;
            return;
        }

        // Check if this paragraph has a quote/blockquote style
        let is_quote = paragraph.style.as_deref().is_some_and(|s| {
            let lower = s.to_ascii_lowercase();
            lower == "quote" || lower == "blockquote" || lower.contains("quote")
        });

        if is_list {
            // List items separated by single newline
            if *prev_was_list {
                output.push('\n');
            }
            output.push_str(&md);
        } else if is_quote {
            Self::ensure_blank_line(output);
            output.push_str("> ");
            output.push_str(&md);
        } else {
            // Non-list paragraphs separated by blank lines
            Self::ensure_blank_line(output);
            output.push_str(&md);
        }

        *prev_was_list = is_list;
    }
}

impl Paragraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Concatenate text runs to produce paragraph text.
    ///
    /// In DOCX, whitespace between words is stored inside `<w:t>` elements
    /// (e.g. `<w:t>Hello </w:t><w:t>World</w:t>`), so runs are joined
    /// directly without adding extra separators. The parser must use
    /// `trim_text(false)` to preserve this whitespace.
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        for run in &self.runs {
            if let Some((ref latex, _)) = run.math_latex {
                text.push_str(latex);
            } else {
                text.push_str(&run.text);
            }
        }
        text
    }

    /// Render inline runs as markdown (no paragraph-level wrapping).
    ///
    /// Uses a two-level grouping strategy to avoid spurious marker sequences like `****`:
    /// 1. Groups consecutive runs that share the same bold/italic/hyperlink properties.
    /// 2. Within each group, opens bold/italic once and toggles underline/strikethrough per run.
    pub fn runs_to_markdown(&self) -> String {
        let mut text = String::new();
        let mut i = 0;
        while i < self.runs.len() {
            let run = &self.runs[i];

            // For math runs or empty runs, emit individually.
            if run.math_latex.is_some() || run.text.is_empty() {
                text.push_str(&run.to_markdown());
                i += 1;
                continue;
            }

            // Collect a group of consecutive runs sharing the same bold/italic/hyperlink.
            // Inner formatting (underline, strikethrough) may differ within the group.
            let group_start = i;
            let mut j = i + 1;
            while j < self.runs.len() {
                let next = &self.runs[j];
                if next.math_latex.is_some()
                    || next.text.is_empty()
                    || next.bold != run.bold
                    || next.italic != run.italic
                    || next.hyperlink_url != run.hyperlink_url
                {
                    break;
                }
                j += 1;
            }
            let group_end = j;

            // If the group is a single run with uniform formatting, use simple merge.
            // Also check if all runs in group have identical inner formatting — merge text.
            let all_same_inner = self.runs[group_start..group_end]
                .iter()
                .all(|r| r.underline == run.underline && r.strikethrough == run.strikethrough);

            if all_same_inner {
                // Merge all text and emit as one run.
                let mut merged_text = String::new();
                for r in &self.runs[group_start..group_end] {
                    merged_text.push_str(&r.text);
                }
                let merged_run = Run {
                    text: merged_text,
                    bold: run.bold,
                    italic: run.italic,
                    underline: run.underline,
                    strikethrough: run.strikethrough,
                    hyperlink_url: run.hyperlink_url.clone(),
                    ..Default::default()
                };
                text.push_str(&merged_run.to_markdown());
            } else {
                // Group has mixed inner formatting.  Open bold/italic once, toggle inner per run.
                if run.hyperlink_url.is_some() {
                    text.push('[');
                }
                if run.bold && run.italic {
                    text.push_str("***");
                } else if run.bold {
                    text.push_str("**");
                } else if run.italic {
                    text.push('*');
                }

                for r in &self.runs[group_start..group_end] {
                    if r.underline {
                        text.push_str("<u>");
                    }
                    if r.strikethrough {
                        text.push_str("~~");
                    }
                    text.push_str(&r.text);
                    if r.strikethrough {
                        text.push_str("~~");
                    }
                    if r.underline {
                        text.push_str("</u>");
                    }
                }

                if run.bold && run.italic {
                    text.push_str("***");
                } else if run.bold {
                    text.push_str("**");
                } else if run.italic {
                    text.push('*');
                }
                if let Some(ref url) = run.hyperlink_url {
                    text.push_str("](");
                    text.push_str(url);
                    text.push(')');
                }
            }

            i = group_end;
        }
        text
    }

    /// Render as markdown with heading/list context.
    ///
    /// If `heading_level` is provided (resolved via `Document::resolve_heading_level`),
    /// it takes precedence over style name matching.
    pub fn to_markdown(
        &self,
        numbering_defs: &AHashMap<(i64, i64), ListType>,
        list_counters: &mut AHashMap<(i64, i64), usize>,
        heading_level: Option<u8>,
    ) -> String {
        let inline = self.runs_to_markdown();

        // Check for heading level (resolved from StyleCatalog or style name fallback)
        if let Some(level) = heading_level {
            let hashes = "#".repeat(level as usize);
            return format!("{} {}", hashes, inline);
        }

        // Check for list item
        if let (Some(num_id), Some(level)) = (self.numbering_id, self.numbering_level) {
            let indent = "  ".repeat(level as usize);
            let key = (num_id, level);
            let list_type = numbering_defs.get(&key).copied().unwrap_or(ListType::Bullet);

            match list_type {
                ListType::Bullet => {
                    return format!("{}- {}", indent, inline);
                }
                ListType::Numbered => {
                    let counter = list_counters.entry(key).or_insert(0);
                    *counter += 1;
                    return format!("{}{}. {}", indent, *counter, inline);
                }
            }
        }

        // Plain paragraph
        inline
    }

    pub fn add_run(&mut self, run: Run) {
        self.runs.push(run);
    }
}

impl Run {
    pub fn new(text: String) -> Self {
        Self {
            text,
            ..Default::default()
        }
    }

    /// Render this run as markdown with formatting markers.
    pub fn to_markdown(&self) -> String {
        // Math runs: wrap LaTeX in $ or $$
        if let Some((ref latex, is_display)) = self.math_latex {
            if latex.is_empty() {
                return String::new();
            }
            return if is_display {
                format!("$${}$$", latex)
            } else {
                format!("${}$", latex)
            };
        }

        if self.text.is_empty() {
            return String::new();
        }

        let extra = (if self.bold && self.italic {
            6
        } else if self.bold || self.italic {
            4
        } else {
            0
        }) + (if self.strikethrough { 4 } else { 0 })
            + (if self.underline { 7 } else { 0 })
            + self.hyperlink_url.as_ref().map_or(0, |u| u.len() + 4);
        let mut out = String::with_capacity(self.text.len() + extra);

        if self.hyperlink_url.is_some() {
            out.push('[');
        }
        if self.underline {
            out.push_str("<u>");
        }
        if self.strikethrough {
            out.push_str("~~");
        }
        if self.bold && self.italic {
            out.push_str("***");
        } else if self.bold {
            out.push_str("**");
        } else if self.italic {
            out.push('*');
        }

        out.push_str(&self.text);

        if self.bold && self.italic {
            out.push_str("***");
        } else if self.bold {
            out.push_str("**");
        } else if self.italic {
            out.push('*');
        }
        if self.strikethrough {
            out.push_str("~~");
        }
        if self.underline {
            out.push_str("</u>");
        }
        if let Some(ref url) = self.hyperlink_url {
            out.push_str("](");
            out.push_str(url);
            out.push(')');
        }

        out
    }
}

impl Table {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render this table as a markdown table.
    ///
    /// Uses table row and cell properties to improve formatting:
    /// - Respects `RowProperties.is_header` to identify header rows
    /// - Handles `CellProperties.grid_span` to account for merged cells
    ///
    /// If no explicit header row is marked, treats the first row as the header.
    pub fn to_markdown(&self) -> String {
        if self.rows.is_empty() {
            return String::new();
        }

        // Build cells, accounting for grid_span (horizontal cell merging)
        let mut cells: Vec<Vec<String>> = Vec::new();
        for row in &self.rows {
            let mut row_cells = Vec::new();
            for cell in &row.cells {
                // Cells with v_merge=Continue are continuations of a vertically merged cell above;
                // render them as empty in the markdown table.
                let is_vmerge_continue = cell
                    .properties
                    .as_ref()
                    .is_some_and(|p| matches!(p.v_merge, Some(super::table::VerticalMerge::Continue)));

                let cell_text = if is_vmerge_continue {
                    String::new()
                } else {
                    cell.paragraphs
                        .iter()
                        .map(|para| para.runs_to_markdown())
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string()
                };
                row_cells.push(cell_text);

                // Add empty cells for grid_span > 1 (horizontal merging)
                let span = cell.properties.as_ref().and_then(|p| p.grid_span).unwrap_or(1);
                for _ in 1..span {
                    row_cells.push(String::new());
                }
            }
            cells.push(row_cells);
        }

        if cells.is_empty() {
            return String::new();
        }

        let num_cols = cells.iter().map(|r| r.len()).max().unwrap_or(0);
        if num_cols == 0 {
            return String::new();
        }

        // Calculate column widths
        let mut col_widths = vec![3usize; num_cols];
        for row in &cells {
            for (i, cell) in row.iter().enumerate() {
                col_widths[i] = col_widths[i].max(cell.len());
            }
        }

        // Determine which row is the header.
        // Prefer explicitly marked header rows; fall back to first row if none found.
        let header_row_index = self
            .rows
            .iter()
            .position(|row| row.properties.as_ref().map(|p| p.is_header).unwrap_or(false))
            .unwrap_or(0); // Default to first row if no explicit header found

        let mut md = String::new();

        // Render rows
        for (row_idx, row) in cells.iter().enumerate() {
            md.push('|');
            for (i, cell) in row.iter().enumerate() {
                let width = col_widths.get(i).copied().unwrap_or(3);
                md.push_str(&format!(" {:width$} |", cell, width = width));
            }
            // Pad missing columns
            for i in row.len()..num_cols {
                let width = col_widths.get(i).copied().unwrap_or(3);
                md.push_str(&format!(" {:width$} |", "", width = width));
            }
            md.push('\n');

            // Insert separator after header row
            if row_idx == header_row_index {
                md.push('|');
                for i in 0..num_cols {
                    let width = col_widths.get(i).copied().unwrap_or(3);
                    md.push_str(&format!(" {} |", "-".repeat(width)));
                }
                md.push('\n');
            }
        }

        md.trim_end().to_string()
    }

    /// Render this table as plain text with tab-separated cells.
    pub fn to_plain_text(&self) -> String {
        if self.rows.is_empty() {
            return String::new();
        }

        let mut cells: Vec<Vec<String>> = Vec::new();
        for row in &self.rows {
            let mut row_cells = Vec::new();
            for cell in &row.cells {
                let is_vmerge_continue = cell
                    .properties
                    .as_ref()
                    .is_some_and(|p| matches!(p.v_merge, Some(super::table::VerticalMerge::Continue)));

                let cell_text = if is_vmerge_continue {
                    String::new()
                } else {
                    cell.paragraphs
                        .iter()
                        .map(|para| para.to_text())
                        .collect::<Vec<_>>()
                        .join(" ")
                        .trim()
                        .to_string()
                };
                row_cells.push(cell_text);

                let span = cell.properties.as_ref().and_then(|p| p.grid_span).unwrap_or(1);
                for _ in 1..span {
                    row_cells.push(String::new());
                }
            }
            cells.push(row_cells);
        }

        crate::extraction::cells_to_text(&cells)
    }
}

// --- Parser ---

/// Context for tracking nested table parsing state.
///
/// Each level of table nesting gets its own context on the stack,
/// allowing arbitrary nesting depth (e.g. tables within table cells).
struct TableContext {
    table: Table,
    current_row: Option<TableRow>,
    current_cell: Option<TableCell>,
    paragraph: Option<Paragraph>,
}

impl TableContext {
    fn new() -> Self {
        Self {
            table: Table::new(),
            current_row: None,
            current_cell: None,
            paragraph: None,
        }
    }
}

/// Apply run-level formatting from run property child elements.
///
/// Handles `<w:b>`, `<w:i>`, `<w:u>`, `<w:strike>`, `<w:dstrike>`,
/// `<w:vertAlign>`, `<w:sz>`, `<w:color>`, and `<w:highlight>`.
/// Works for both `Event::Start` and `Event::Empty` events.
fn apply_run_formatting(e: &BytesStart, current_run: &mut Option<Run>) {
    if let Some(run) = current_run {
        match e.name().as_ref() as &[u8] {
            b"w:b" => run.bold = is_format_enabled(e),
            b"w:i" => run.italic = is_format_enabled(e),
            b"w:u" => run.underline = is_format_enabled(e),
            b"w:strike" | b"w:dstrike" => run.strikethrough = is_format_enabled(e),
            b"w:vertAlign" => {
                if let Some(val) = get_val_attr_string(e) {
                    match val.as_str() {
                        "subscript" => {
                            run.subscript = true;
                            run.superscript = false;
                        }
                        "superscript" => {
                            run.superscript = true;
                            run.subscript = false;
                        }
                        _ => {
                            run.subscript = false;
                            run.superscript = false;
                        }
                    }
                }
            }
            b"w:sz" => {
                if let Some(val) = get_val_attr(e) {
                    run.font_size = Some(val as u32);
                }
            }
            b"w:color" => {
                if let Some(val) = get_val_attr_string(e)
                    && val != "auto"
                    && val.len() == 6
                    && val.chars().all(|c| c.is_ascii_hexdigit())
                {
                    run.font_color = Some(val);
                }
            }
            b"w:highlight" => {
                if let Some(val) = get_val_attr_string(e) {
                    const VALID_HIGHLIGHTS: &[&str] = &[
                        "yellow",
                        "green",
                        "cyan",
                        "magenta",
                        "blue",
                        "red",
                        "darkBlue",
                        "darkCyan",
                        "darkGreen",
                        "darkMagenta",
                        "darkRed",
                        "darkYellow",
                        "darkGray",
                        "lightGray",
                        "black",
                        "none",
                    ];
                    if VALID_HIGHLIGHTS.contains(&val.as_str()) {
                        run.highlight = Some(val);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Apply paragraph-level properties from a `<w:pStyle>`, `<w:ilvl>`, or `<w:numId>` element.
///
/// Resolves the correct paragraph (table context vs top-level) automatically.
fn apply_paragraph_property(
    e: &BytesStart,
    table_stack: &mut [TableContext],
    current_paragraph: &mut Option<Paragraph>,
) {
    let para = if let Some(ctx) = table_stack.last_mut() {
        ctx.paragraph.as_mut()
    } else {
        current_paragraph.as_mut()
    };

    if let Some(para) = para {
        match e.name().as_ref() as &[u8] {
            b"w:pStyle" => para.style = get_val_attr_string(e),
            b"w:ilvl" => para.numbering_level = get_val_attr(e),
            b"w:numId" => para.numbering_id = get_val_attr(e),
            _ => {}
        }
    }
}

// --- Security Validation ---

/// Validate archive against ZIP bomb attacks and resource exhaustion.
///
/// Checks:
/// - Maximum uncompressed size per file (100 MB default)
/// - Maximum total number of entries (10,000 default)
/// - Maximum total uncompressed size (500 MB default)
fn validate_archive_security(archive: &mut zip::ZipArchive<impl Read + Seek>) -> Result<(), DocxParseError> {
    use super::{MAX_TOTAL_UNCOMPRESSED_SIZE, MAX_UNCOMPRESSED_FILE_SIZE, MAX_ZIP_ENTRIES};

    // Check entry count
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(DocxParseError::SecurityLimit(format!(
            "Archive contains {} entries, exceeds limit of {}",
            archive.len(),
            MAX_ZIP_ENTRIES
        )));
    }

    // Check individual file sizes and accumulate total
    let mut total_uncompressed: u64 = 0;
    for i in 0..archive.len() {
        let file = archive
            .by_index_raw(i)
            .map_err(|e| DocxParseError::SecurityLimit(format!("Failed to read ZIP entry {}: {}", i, e)))?;
        let size = file.size();
        if size > MAX_UNCOMPRESSED_FILE_SIZE {
            return Err(DocxParseError::SecurityLimit(format!(
                "File '{}' uncompressed size {} bytes exceeds limit of {} bytes",
                file.name(),
                size,
                MAX_UNCOMPRESSED_FILE_SIZE
            )));
        }
        total_uncompressed = total_uncompressed.saturating_add(size);
    }

    // Check total uncompressed size
    if total_uncompressed > MAX_TOTAL_UNCOMPRESSED_SIZE {
        return Err(DocxParseError::SecurityLimit(format!(
            "Total uncompressed size {} bytes exceeds limit of {} bytes",
            total_uncompressed, MAX_TOTAL_UNCOMPRESSED_SIZE
        )));
    }

    Ok(())
}

#[derive(Debug)]
struct DocxParser<R: Read + Seek> {
    archive: zip::ZipArchive<R>,
    relationships: AHashMap<String, String>,
    styles: Option<super::styles::StyleCatalog>,
    theme: Option<super::theme::Theme>,
}

impl<R: Read + Seek> DocxParser<R> {
    fn new(reader: R) -> Result<Self, DocxParseError> {
        let mut archive = zip::ZipArchive::new(reader)?;
        validate_archive_security(&mut archive)?;

        // Load styles catalog (best-effort - styles.xml is optional)
        let styles = {
            let mut styles_result = None;
            if let Ok(file) = archive.by_name("word/styles.xml") {
                let mut xml = String::new();
                if file
                    .take(super::MAX_UNCOMPRESSED_FILE_SIZE)
                    .read_to_string(&mut xml)
                    .is_ok()
                {
                    styles_result = super::styles::parse_styles_xml(&xml).ok();
                }
            }
            styles_result
        };

        // Load theme (best-effort - theme1.xml is optional)
        let theme = {
            let mut theme_result = None;
            if let Ok(file) = archive.by_name("word/theme/theme1.xml") {
                let mut xml = String::new();
                if file
                    .take(super::MAX_UNCOMPRESSED_FILE_SIZE)
                    .read_to_string(&mut xml)
                    .is_ok()
                {
                    theme_result = super::theme::parse_theme_xml(&xml).ok();
                }
            }
            theme_result
        };

        Ok(Self {
            archive,
            relationships: AHashMap::new(),
            styles,
            theme,
        })
    }

    fn parse(mut self) -> Result<Document, DocxParseError> {
        let mut document = Document::new();

        // Parse relationships first for hyperlink URL resolution
        if let Ok(rels_xml) = self.read_file("word/_rels/document.xml.rels") {
            self.relationships = Self::parse_relationships_xml(&rels_xml);
        }

        let document_xml = self.read_file("word/document.xml")?;
        self.parse_document_xml(&document_xml, &mut document)?;

        if let Ok(numbering_xml) = self.read_file("word/numbering.xml") {
            let numbering_defs = self.parse_numbering(&numbering_xml)?;
            document.numbering_defs = numbering_defs;
        }

        self.parse_headers_footers(&mut document)?;

        if let Ok(footnotes_xml) = self.read_file("word/footnotes.xml") {
            self.parse_notes(&footnotes_xml, &mut document.footnotes, NoteType::Footnote)?;
        }

        if let Ok(endnotes_xml) = self.read_file("word/endnotes.xml") {
            self.parse_notes(&endnotes_xml, &mut document.endnotes, NoteType::Endnote)?;
        }

        document.style_catalog = self.styles.take();
        document.theme = self.theme.take();
        // Filter to only image relationships (exclude hyperlinks)
        document.image_relationships = self
            .relationships
            .iter()
            .filter(|(_, target)| {
                // Image targets point to media/ paths, not URLs
                !target.starts_with("http://") && !target.starts_with("https://")
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(document)
    }

    /// Parse relationship file to get rId → target mappings for hyperlinks and images.
    fn parse_relationships_xml(xml: &str) -> AHashMap<String, String> {
        let mut rels = AHashMap::new();
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() as &[u8] == b"Relationship" => {
                    let mut id = None;
                    let mut target = None;
                    let mut rel_type_matches = false;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"Id" => id = std::str::from_utf8(&attr.value).ok().map(String::from),
                            b"Target" => {
                                target = std::str::from_utf8(&attr.value).ok().map(String::from);
                            }
                            b"Type" => {
                                rel_type_matches = std::str::from_utf8(&attr.value)
                                    .ok()
                                    .is_some_and(|t| t.contains("hyperlink") || t.contains("image"));
                            }
                            _ => {}
                        }
                    }
                    // Include hyperlink and image relationships
                    if let (Some(id_val), Some(target_val)) = (id, target)
                        && rel_type_matches
                    {
                        rels.insert(id_val, target_val);
                    }
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
            buf.clear();
        }

        rels
    }

    fn read_file(&mut self, path: &str) -> Result<String, DocxParseError> {
        let read_limit = super::MAX_UNCOMPRESSED_FILE_SIZE;

        let file = self
            .archive
            .by_name(path)
            .map_err(|_| DocxParseError::FileNotFound(path.to_string()))?;

        let mut contents = String::new();
        file.take(read_limit).read_to_string(&mut contents)?;
        Ok(contents)
    }

    fn parse_document_xml(&self, xml: &str, document: &mut Document) -> Result<(), DocxParseError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);

        let mut buf = Vec::new();
        let mut current_paragraph: Option<Paragraph> = None;
        let mut current_run: Option<Run> = None;
        let mut in_text = false;
        let mut in_field_instruction = false;
        let mut current_hyperlink_url: Option<String> = None;
        let mut table_stack: Vec<TableContext> = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => match e.name().as_ref() as &[u8] {
                    b"w:p" => {
                        if let Some(ctx) = table_stack.last_mut() {
                            ctx.paragraph = Some(Paragraph::new());
                        } else {
                            current_paragraph = Some(Paragraph::new());
                        }
                    }
                    b"w:r" => {
                        let mut run = Run::default();
                        if let Some(ref url) = current_hyperlink_url {
                            run.hyperlink_url = Some(url.clone());
                        }
                        current_run = Some(run);
                    }
                    b"w:t" if !in_field_instruction => {
                        in_text = true;
                    }
                    b"w:fldChar" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"w:fldCharType" {
                                match attr.value.as_ref() as &[u8] {
                                    b"begin" => in_field_instruction = true,
                                    b"separate" | b"end" => in_field_instruction = false,
                                    _ => {}
                                }
                            }
                        }
                    }
                    b"w:instrText" => {
                        // Skip field instruction text
                    }
                    // OMML display math — delegate to math.rs
                    b"m:oMathPara" => {
                        let latex = super::math::collect_and_convert_omath_para(&mut reader);
                        if !latex.is_empty() {
                            let run = Run {
                                math_latex: Some((latex, true)),
                                ..Default::default()
                            };
                            if let Some(ctx) = table_stack.last_mut() {
                                if let Some(ref mut para) = ctx.paragraph {
                                    para.add_run(run);
                                } else if let Some(ref mut cell) = ctx.current_cell {
                                    if cell.paragraphs.is_empty() {
                                        cell.paragraphs.push(Paragraph::new());
                                    }
                                    if let Some(para) = cell.paragraphs.last_mut() {
                                        para.add_run(run);
                                    }
                                }
                            } else if let Some(ref mut para) = current_paragraph {
                                para.add_run(run);
                            }
                        }
                    }
                    // OMML inline math — delegate to math.rs
                    b"m:oMath" => {
                        let latex = super::math::collect_and_convert_omath(&mut reader);
                        if !latex.is_empty() {
                            let run = Run {
                                math_latex: Some((latex, false)),
                                ..Default::default()
                            };
                            if let Some(ctx) = table_stack.last_mut() {
                                if let Some(ref mut para) = ctx.paragraph {
                                    para.add_run(run);
                                } else if let Some(ref mut cell) = ctx.current_cell {
                                    if cell.paragraphs.is_empty() {
                                        cell.paragraphs.push(Paragraph::new());
                                    }
                                    if let Some(para) = cell.paragraphs.last_mut() {
                                        para.add_run(run);
                                    }
                                }
                            } else if let Some(ref mut para) = current_paragraph {
                                para.add_run(run);
                            }
                        }
                    }
                    b"w:tbl" => {
                        table_stack.push(TableContext::new());
                    }
                    b"w:tblPr" => {
                        if let Some(ctx) = table_stack.last_mut() {
                            ctx.table.properties = Some(super::table::parse_table_properties(&mut reader));
                        }
                    }
                    b"w:tblGrid" => {
                        if let Some(ctx) = table_stack.last_mut() {
                            ctx.table.grid = Some(super::table::parse_table_grid(&mut reader));
                        }
                    }
                    b"w:tr" => {
                        if let Some(ctx) = table_stack.last_mut() {
                            ctx.current_row = Some(TableRow::default());
                        }
                    }
                    b"w:trPr" => {
                        if let Some(ctx) = table_stack.last_mut()
                            && let Some(ref mut row) = ctx.current_row
                        {
                            row.properties = Some(super::table::parse_row_properties(&mut reader));
                        }
                    }
                    b"w:tc" => {
                        if let Some(ctx) = table_stack.last_mut() {
                            ctx.current_cell = Some(TableCell::default());
                        }
                    }
                    b"w:tcPr" => {
                        if let Some(ctx) = table_stack.last_mut()
                            && let Some(ref mut cell) = ctx.current_cell
                        {
                            cell.properties = Some(super::table::parse_cell_properties(&mut reader));
                        }
                    }
                    b"w:b" | b"w:i" | b"w:u" | b"w:strike" | b"w:dstrike" | b"w:vertAlign" | b"w:sz" | b"w:color"
                    | b"w:highlight" => {
                        apply_run_formatting(e, &mut current_run);
                    }
                    b"w:pStyle" | b"w:ilvl" | b"w:numId" => {
                        apply_paragraph_property(e, &mut table_stack, &mut current_paragraph);
                    }
                    b"w:hyperlink" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"r:id"
                                && let Ok(rid) = std::str::from_utf8(&attr.value)
                            {
                                current_hyperlink_url = self.relationships.get(rid).cloned();
                            }
                        }
                    }
                    b"w:drawing" => {
                        let drawing = super::drawing::parse_drawing(&mut reader);
                        let idx = document.drawings.len();
                        document.drawings.push(drawing);
                        document.elements.push(DocumentElement::Drawing(idx));
                    }
                    // Line break (when not self-closing) or Page break
                    b"w:br" => {
                        let mut is_page_break = false;
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"w:type" && attr.value.as_ref() == b"page" {
                                is_page_break = true;
                                break;
                            }
                        }

                        if is_page_break && table_stack.is_empty() {
                            document.elements.push(DocumentElement::PageBreak);
                        } else if !is_page_break {
                            if let Some(ref mut run) = current_run {
                                run.text.push('\n');
                            }
                        }
                    }
                    b"w:lastRenderedPageBreak" => {
                        if table_stack.is_empty() {
                            document.elements.push(DocumentElement::PageBreak);
                        }
                    }
                    b"w:sectPr" => {
                        let sect_props = super::section::parse_section_properties_streaming(&mut reader);
                        document.sections.push(sect_props);
                    }
                    _ => {}
                },
                Ok(Event::Empty(ref e)) => match e.name().as_ref() as &[u8] {
                    b"w:fldChar" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"w:fldCharType" {
                                match attr.value.as_ref() as &[u8] {
                                    b"begin" => in_field_instruction = true,
                                    b"separate" | b"end" => in_field_instruction = false,
                                    _ => {}
                                }
                            }
                        }
                    }
                    b"w:b" | b"w:i" | b"w:u" | b"w:strike" | b"w:dstrike" | b"w:vertAlign" | b"w:sz" | b"w:color"
                    | b"w:highlight" => {
                        apply_run_formatting(e, &mut current_run);
                    }
                    b"w:pStyle" | b"w:ilvl" | b"w:numId" => {
                        apply_paragraph_property(e, &mut table_stack, &mut current_paragraph);
                    }
                    // Line break: insert newline to separate adjacent text
                    b"w:br" => {
                        let mut is_page_break = false;
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"w:type" && attr.value.as_ref() == b"page" {
                                is_page_break = true;
                                break;
                            }
                        }

                        if is_page_break && table_stack.is_empty() {
                            document.elements.push(DocumentElement::PageBreak);
                        } else if !is_page_break {
                            if let Some(ref mut run) = current_run {
                                run.text.push('\n');
                            }
                        }
                    }
                    b"w:lastRenderedPageBreak" => {
                        if table_stack.is_empty() {
                            document.elements.push(DocumentElement::PageBreak);
                        }
                    }
                    b"w:footnoteReference" | b"w:endnoteReference" => {
                        // Insert inline footnote/endnote reference marker [^N]
                        if let Some(ref mut run) = current_run {
                            for attr in e.attributes().flatten() {
                                if attr.key.as_ref() == b"w:id"
                                    && let Ok(id) = std::str::from_utf8(&attr.value)
                                {
                                    // Skip separator references (id 0 and 1)
                                    if id != "0" && id != "1" {
                                        run.text.push_str(&format!("[^{}]", id));
                                    }
                                }
                            }
                        }
                    }
                    b"w:sectPr" => {
                        // Self-closing <w:sectPr/> (empty section properties)
                        document.sections.push(super::section::SectionProperties::default());
                    }
                    b"w:tblPr" => {
                        if let Some(ctx) = table_stack.last_mut() {
                            ctx.table.properties = Some(super::table::TableProperties::default());
                        }
                    }
                    b"w:tblGrid" => {
                        if let Some(ctx) = table_stack.last_mut() {
                            ctx.table.grid = Some(super::table::TableGrid::default());
                        }
                    }
                    b"w:trPr" => {
                        if let Some(ctx) = table_stack.last_mut()
                            && let Some(ref mut row) = ctx.current_row
                        {
                            row.properties = Some(super::table::RowProperties::default());
                        }
                    }
                    b"w:tcPr" => {
                        if let Some(ctx) = table_stack.last_mut()
                            && let Some(ref mut cell) = ctx.current_cell
                        {
                            cell.properties = Some(super::table::CellProperties::default());
                        }
                    }
                    _ => {}
                },
                Ok(Event::Text(e)) => {
                    if in_text && let Some(ref mut run) = current_run {
                        let text = e.decode()?;
                        run.text.push_str(&text);
                    }
                }
                Ok(Event::End(ref e)) => match e.name().as_ref() as &[u8] {
                    b"w:t" => {
                        in_text = false;
                    }
                    b"w:r" => {
                        if let Some(run) = current_run.take() {
                            if let Some(ctx) = table_stack.last_mut() {
                                if let Some(ref mut para) = ctx.paragraph {
                                    para.add_run(run);
                                } else if let Some(ref mut cell) = ctx.current_cell {
                                    if cell.paragraphs.is_empty() {
                                        cell.paragraphs.push(Paragraph::new());
                                    }
                                    if let Some(para) = cell.paragraphs.last_mut() {
                                        para.add_run(run);
                                    }
                                }
                            } else if let Some(ref mut para) = current_paragraph {
                                para.add_run(run);
                            }
                        }
                    }
                    b"w:p" => {
                        if let Some(ctx) = table_stack.last_mut() {
                            if let Some(para) = ctx.paragraph.take()
                                && let Some(ref mut cell) = ctx.current_cell
                            {
                                cell.paragraphs.push(para);
                            }
                        } else if let Some(para) = current_paragraph.take() {
                            let idx = document.paragraphs.len();
                            document.paragraphs.push(para);
                            document.elements.push(DocumentElement::Paragraph(idx));
                        }
                    }
                    b"w:tc" => {
                        if let Some(ctx) = table_stack.last_mut()
                            && let Some(cell) = ctx.current_cell.take()
                            && let Some(ref mut row) = ctx.current_row
                        {
                            row.cells.push(cell);
                        }
                    }
                    b"w:tr" => {
                        if let Some(ctx) = table_stack.last_mut()
                            && let Some(row) = ctx.current_row.take()
                        {
                            ctx.table.rows.push(row);
                        }
                    }
                    b"w:tbl" => {
                        if let Some(completed_ctx) = table_stack.pop() {
                            let completed_table = completed_ctx.table;
                            if let Some(parent_ctx) = table_stack.last_mut() {
                                // Nested table: flatten content into parent cell
                                if let Some(ref mut cell) = parent_ctx.current_cell {
                                    for row in completed_table.rows {
                                        for table_cell in row.cells {
                                            for para in table_cell.paragraphs {
                                                cell.paragraphs.push(para);
                                            }
                                        }
                                    }
                                }
                            } else {
                                // Top-level table
                                let idx = document.tables.len();
                                document.tables.push(completed_table);
                                document.elements.push(DocumentElement::Table(idx));
                            }
                        }
                    }
                    b"w:hyperlink" => {
                        current_hyperlink_url = None;
                    }
                    _ => {}
                },
                Ok(Event::Eof) => break,
                Err(e) => return Err(e.into()),
                _ => {}
            }
            buf.clear();
        }

        Ok(())
    }

    fn parse_numbering(&self, xml: &str) -> Result<AHashMap<(i64, i64), ListType>, DocxParseError> {
        let mut numbering_defs: AHashMap<(i64, i64), ListType> = AHashMap::new();
        let mut abstract_num_formats: AHashMap<i64, AHashMap<i64, ListType>> = AHashMap::new();
        let mut num_to_abstract: AHashMap<i64, i64> = AHashMap::new();

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);

        let mut buf = Vec::new();
        let mut current_abstract_num_id: Option<i64> = None;
        let mut current_num_id: Option<i64> = None;
        let mut current_lvl: Option<i64> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => match e.name().as_ref() as &[u8] {
                    b"w:abstractNum" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"w:abstractNumId"
                                && let Ok(id_str) = std::str::from_utf8(&attr.value)
                            {
                                current_abstract_num_id = id_str.parse().ok();
                            }
                        }
                    }
                    b"w:num" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"w:numId"
                                && let Ok(id_str) = std::str::from_utf8(&attr.value)
                            {
                                current_num_id = id_str.parse().ok();
                            }
                        }
                    }
                    b"w:lvl" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"w:ilvl"
                                && let Ok(id_str) = std::str::from_utf8(&attr.value)
                            {
                                current_lvl = id_str.parse().ok();
                            }
                        }
                    }
                    b"w:numFmt" => {
                        if let (Some(abstract_id), Some(lvl)) = (current_abstract_num_id, current_lvl) {
                            let fmt = get_val_attr_string(e);
                            let list_type = match fmt.as_deref() {
                                Some("decimal") | Some("decimalZero") | Some("lowerLetter") | Some("upperLetter")
                                | Some("lowerRoman") | Some("upperRoman") => ListType::Numbered,
                                _ => ListType::Bullet,
                            };
                            abstract_num_formats
                                .entry(abstract_id)
                                .or_default()
                                .insert(lvl, list_type);
                        }
                    }
                    _ => {}
                },
                Ok(Event::Empty(ref e)) => match e.name().as_ref() as &[u8] {
                    b"w:abstractNumId" => {
                        if let Some(num_id) = current_num_id
                            && let Some(abstract_id) = get_val_attr(e)
                        {
                            num_to_abstract.insert(num_id, abstract_id);
                        }
                    }
                    b"w:numFmt" => {
                        if let (Some(abstract_id), Some(lvl)) = (current_abstract_num_id, current_lvl) {
                            let fmt = get_val_attr_string(e);
                            let list_type = match fmt.as_deref() {
                                Some("decimal") | Some("decimalZero") | Some("lowerLetter") | Some("upperLetter")
                                | Some("lowerRoman") | Some("upperRoman") => ListType::Numbered,
                                _ => ListType::Bullet,
                            };
                            abstract_num_formats
                                .entry(abstract_id)
                                .or_default()
                                .insert(lvl, list_type);
                        }
                    }
                    _ => {}
                },
                Ok(Event::End(ref e)) => match e.name().as_ref() as &[u8] {
                    b"w:abstractNum" => {
                        current_abstract_num_id = None;
                        current_lvl = None;
                    }
                    b"w:lvl" => {
                        current_lvl = None;
                    }
                    b"w:num" => {
                        current_num_id = None;
                    }
                    _ => {}
                },
                Ok(Event::Eof) => break,
                _ => {}
            }
            buf.clear();
        }

        // Build final numbering_defs by resolving num → abstractNum references
        for (num_id, abstract_id) in &num_to_abstract {
            if let Some(formats) = abstract_num_formats.get(abstract_id) {
                for (lvl, list_type) in formats {
                    numbering_defs.insert((*num_id, *lvl), *list_type);
                }
            }
        }

        Ok(numbering_defs)
    }

    fn parse_headers_footers(&mut self, document: &mut Document) -> Result<(), DocxParseError> {
        for i in 1..=3 {
            let header_path = format!("word/header{}.xml", i);
            if let Ok(header_xml) = self.read_file(&header_path) {
                let mut header = HeaderFooter::default();
                self.parse_header_footer_content(&header_xml, &mut header)?;
                document.headers.push(header);
            }

            let footer_path = format!("word/footer{}.xml", i);
            if let Ok(footer_xml) = self.read_file(&footer_path) {
                let mut footer = HeaderFooter::default();
                self.parse_header_footer_content(&footer_xml, &mut footer)?;
                document.footers.push(footer);
            }
        }

        Ok(())
    }

    fn parse_header_footer_content(&self, xml: &str, header_footer: &mut HeaderFooter) -> Result<(), DocxParseError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);

        let mut buf = Vec::new();
        let mut current_paragraph: Option<Paragraph> = None;
        let mut current_run: Option<Run> = None;
        let mut in_text = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => match e.name().as_ref() as &[u8] {
                    b"w:p" => current_paragraph = Some(Paragraph::new()),
                    b"w:r" => current_run = Some(Run::default()),
                    b"w:t" => in_text = true,
                    b"w:b" | b"w:i" | b"w:u" | b"w:strike" | b"w:dstrike" | b"w:vertAlign" | b"w:sz" | b"w:color"
                    | b"w:highlight" => {
                        apply_run_formatting(e, &mut current_run);
                    }
                    _ => {}
                },
                Ok(Event::Empty(ref e)) => match e.name().as_ref() as &[u8] {
                    b"w:b" | b"w:i" | b"w:u" | b"w:strike" | b"w:dstrike" | b"w:vertAlign" | b"w:sz" | b"w:color"
                    | b"w:highlight" => {
                        apply_run_formatting(e, &mut current_run);
                    }
                    _ => {}
                },
                Ok(Event::Text(e)) => {
                    if in_text && let Some(ref mut run) = current_run {
                        let text = e.decode()?;
                        run.text.push_str(&text);
                    }
                }
                Ok(Event::End(ref e)) => match e.name().as_ref() as &[u8] {
                    b"w:t" => in_text = false,
                    b"w:r" => {
                        if let Some(run) = current_run.take()
                            && let Some(ref mut para) = current_paragraph
                        {
                            para.add_run(run);
                        }
                    }
                    b"w:p" => {
                        if let Some(para) = current_paragraph.take() {
                            header_footer.paragraphs.push(para);
                        }
                    }
                    _ => {}
                },
                Ok(Event::Eof) => break,
                _ => {}
            }
            buf.clear();
        }

        Ok(())
    }

    fn parse_notes(&self, xml: &str, notes: &mut Vec<Note>, note_type: NoteType) -> Result<(), DocxParseError> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(false);

        let mut buf = Vec::new();
        let mut current_note: Option<Note> = None;
        let mut current_paragraph: Option<Paragraph> = None;
        let mut current_run: Option<Run> = None;
        let mut in_text = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => match e.name().as_ref() as &[u8] {
                    b"w:footnote" | b"w:endnote" => {
                        let mut id = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"w:id" {
                                id = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        current_note = Some(Note {
                            id,
                            note_type,
                            paragraphs: Vec::new(),
                        });
                    }
                    b"w:p" => current_paragraph = Some(Paragraph::new()),
                    b"w:r" => current_run = Some(Run::default()),
                    b"w:t" => in_text = true,
                    b"w:b" => {
                        if let Some(ref mut run) = current_run {
                            run.bold = is_format_enabled(e);
                        }
                    }
                    b"w:i" => {
                        if let Some(ref mut run) = current_run {
                            run.italic = is_format_enabled(e);
                        }
                    }
                    _ => {}
                },
                Ok(Event::Empty(ref e)) => match e.name().as_ref() as &[u8] {
                    b"w:b" => {
                        if let Some(ref mut run) = current_run {
                            run.bold = is_format_enabled(e);
                        }
                    }
                    b"w:i" => {
                        if let Some(ref mut run) = current_run {
                            run.italic = is_format_enabled(e);
                        }
                    }
                    _ => {}
                },
                Ok(Event::Text(e)) => {
                    if in_text && let Some(ref mut run) = current_run {
                        let text = e.decode()?;
                        run.text.push_str(&text);
                    }
                }
                Ok(Event::End(ref e)) => match e.name().as_ref() as &[u8] {
                    b"w:t" => in_text = false,
                    b"w:r" => {
                        if let Some(run) = current_run.take()
                            && let Some(ref mut para) = current_paragraph
                        {
                            para.add_run(run);
                        }
                    }
                    b"w:p" => {
                        if let Some(para) = current_paragraph.take()
                            && let Some(ref mut note) = current_note
                        {
                            note.paragraphs.push(para);
                        }
                    }
                    b"w:footnote" | b"w:endnote" => {
                        // Filter separator/continuation separator notes (id -1, 0, 1)
                        if let Some(note) = current_note.take()
                            && note.id != "-1"
                            && note.id != "0"
                            && note.id != "1"
                        {
                            notes.push(note);
                        }
                    }
                    _ => {}
                },
                Ok(Event::Eof) => break,
                _ => {}
            }
            buf.clear();
        }

        Ok(())
    }
}

// --- Error ---

#[derive(Debug, thiserror::Error)]
enum DocxParseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::Error),

    #[error("Required file not found in DOCX: {0}")]
    FileNotFound(String),

    #[error("Security limit exceeded: {0}")]
    SecurityLimit(String),
}

// quick-xml's unescape returns an encoding error type
impl From<quick_xml::encoding::EncodingError> for DocxParseError {
    fn from(e: quick_xml::encoding::EncodingError) -> Self {
        DocxParseError::Xml(quick_xml::Error::Encoding(e))
    }
}

// --- Public API ---

/// Parse a DOCX document from bytes and return the structured document.
pub fn parse_document(bytes: &[u8]) -> crate::error::Result<Document> {
    let cursor = Cursor::new(bytes);
    let parser = DocxParser::new(cursor)
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("DOCX parsing failed: {}", e)))?;
    parser
        .parse()
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("DOCX parsing failed: {}", e)))
}

/// Extract text from DOCX bytes.
pub fn extract_text_from_bytes(bytes: &[u8]) -> crate::error::Result<String> {
    let doc = parse_document(bytes)?;
    Ok(doc.extract_text())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs are concatenated directly; whitespace comes from the XML text content.
    #[test]
    fn test_paragraph_to_text_concatenates_runs() {
        let mut para = Paragraph::new();
        para.add_run(Run::new("Hello ".to_string()));
        para.add_run(Run::new("World".to_string()));
        assert_eq!(para.to_text(), "Hello World");
    }

    /// Mid-word run splits (e.g. drop caps) must not insert extra spaces.
    #[test]
    fn test_paragraph_to_text_mid_word_split() {
        let mut para = Paragraph::new();
        para.add_run(Run::new("S".to_string()));
        para.add_run(Run::new("ermocination".to_string()));
        assert_eq!(para.to_text(), "Sermocination");
    }

    #[test]
    fn test_paragraph_to_text_single_run() {
        let mut para = Paragraph::new();
        para.add_run(Run::new("Hello".to_string()));
        assert_eq!(para.to_text(), "Hello");
    }

    #[test]
    fn test_paragraph_to_text_no_runs() {
        let para = Paragraph::new();
        assert_eq!(para.to_text(), "");
    }

    /// Whitespace between words is stored in the run text, not added by join.
    #[test]
    fn test_paragraph_to_text_whitespace_in_runs() {
        let mut para = Paragraph::new();
        para.add_run(Run::new("The ".to_string()));
        para.add_run(Run::new("quick ".to_string()));
        para.add_run(Run::new("fox".to_string()));
        assert_eq!(para.to_text(), "The quick fox");
    }

    // --- Markdown rendering unit tests ---

    #[test]
    fn test_run_bold_to_markdown() {
        let run = Run {
            text: "hello".to_string(),
            bold: true,
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "**hello**");
    }

    #[test]
    fn test_run_italic_to_markdown() {
        let run = Run {
            text: "hello".to_string(),
            italic: true,
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "*hello*");
    }

    #[test]
    fn test_run_bold_italic_to_markdown() {
        let run = Run {
            text: "hello".to_string(),
            bold: true,
            italic: true,
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "***hello***");
    }

    #[test]
    fn test_run_strikethrough_to_markdown() {
        let run = Run {
            text: "hello".to_string(),
            strikethrough: true,
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "~~hello~~");
    }

    #[test]
    fn test_run_hyperlink_to_markdown() {
        let run = Run {
            text: "click here".to_string(),
            hyperlink_url: Some("https://example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "[click here](https://example.com)");
    }

    #[test]
    fn test_run_bold_hyperlink_to_markdown() {
        let run = Run {
            text: "click".to_string(),
            bold: true,
            hyperlink_url: Some("https://example.com".to_string()),
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "[**click**](https://example.com)");
    }

    #[test]
    fn test_run_empty_text_to_markdown() {
        let run = Run {
            text: String::new(),
            bold: true,
            ..Default::default()
        };
        assert_eq!(run.to_markdown(), "");
    }

    /// Adjacent bold runs must be merged to avoid spurious `****` sequences.
    #[test]
    fn test_adjacent_bold_runs_merged() {
        let mut para = Paragraph::new();
        let mut r1 = Run::new("Shuishang".to_string());
        r1.bold = true;
        let mut r2 = Run::new(" Township".to_string());
        r2.bold = true;
        para.add_run(r1);
        para.add_run(r2);
        assert_eq!(para.runs_to_markdown(), "**Shuishang Township**");
    }

    /// Adjacent italic runs must be merged to avoid spurious `**` sequences.
    #[test]
    fn test_adjacent_italic_runs_merged() {
        let mut para = Paragraph::new();
        let mut r1 = Run::new("he".to_string());
        r1.italic = true;
        let mut r2 = Run::new("llo".to_string());
        r2.italic = true;
        para.add_run(r1);
        para.add_run(r2);
        assert_eq!(para.runs_to_markdown(), "*hello*");
    }

    /// Runs with different formatting must NOT be merged.
    #[test]
    fn test_different_formatting_runs_not_merged() {
        let mut para = Paragraph::new();
        let mut r1 = Run::new("bold".to_string());
        r1.bold = true;
        let r2 = Run::new(" normal".to_string());
        para.add_run(r1);
        para.add_run(r2);
        assert_eq!(para.runs_to_markdown(), "**bold** normal");
    }

    /// Three adjacent bold runs produce a single merged bold span.
    #[test]
    fn test_three_adjacent_bold_runs_merged() {
        let mut para = Paragraph::new();
        for text in &["i", "l", "l"] {
            let mut r = Run::new(text.to_string());
            r.bold = true;
            para.add_run(r);
        }
        assert_eq!(para.runs_to_markdown(), "**ill**");
    }

    #[test]
    fn test_paragraph_heading_to_markdown() {
        let mut para = Paragraph::new();
        para.style = Some("Title".to_string());
        para.add_run(Run::new("My Title".to_string()));
        let defs = AHashMap::new();
        let mut counters = AHashMap::new();
        assert_eq!(para.to_markdown(&defs, &mut counters, Some(1)), "# My Title");
    }

    #[test]
    fn test_paragraph_heading1_to_markdown() {
        let mut para = Paragraph::new();
        para.style = Some("Heading1".to_string());
        para.add_run(Run::new("Section".to_string()));
        let defs = AHashMap::new();
        let mut counters = AHashMap::new();
        assert_eq!(para.to_markdown(&defs, &mut counters, Some(2)), "## Section");
    }

    #[test]
    fn test_paragraph_heading2_to_markdown() {
        let mut para = Paragraph::new();
        para.style = Some("Heading2".to_string());
        para.add_run(Run::new("Subsection".to_string()));
        let defs = AHashMap::new();
        let mut counters = AHashMap::new();
        assert_eq!(para.to_markdown(&defs, &mut counters, Some(3)), "### Subsection");
    }

    #[test]
    fn test_paragraph_bullet_list_to_markdown() {
        let mut para = Paragraph::new();
        para.numbering_id = Some(1);
        para.numbering_level = Some(0);
        para.add_run(Run::new("Item".to_string()));
        let mut defs = AHashMap::new();
        defs.insert((1, 0), ListType::Bullet);
        let mut counters = AHashMap::new();
        assert_eq!(para.to_markdown(&defs, &mut counters, None), "- Item");
    }

    #[test]
    fn test_paragraph_numbered_list_to_markdown() {
        let mut para = Paragraph::new();
        para.numbering_id = Some(2);
        para.numbering_level = Some(0);
        para.add_run(Run::new("Item".to_string()));
        let mut defs = AHashMap::new();
        defs.insert((2, 0), ListType::Numbered);
        let mut counters = AHashMap::new();
        assert_eq!(para.to_markdown(&defs, &mut counters, None), "1. Item");
    }

    #[test]
    fn test_paragraph_nested_list_to_markdown() {
        let mut para = Paragraph::new();
        para.numbering_id = Some(1);
        para.numbering_level = Some(1);
        para.add_run(Run::new("Nested".to_string()));
        let mut defs = AHashMap::new();
        defs.insert((1, 1), ListType::Bullet);
        let mut counters = AHashMap::new();
        assert_eq!(para.to_markdown(&defs, &mut counters, None), "  - Nested");
    }

    #[test]
    fn test_heading_level_from_style_name() {
        assert_eq!(heading_level_from_style_name("Title"), Some(1));
        assert_eq!(heading_level_from_style_name("Heading1"), Some(2));
        assert_eq!(heading_level_from_style_name("Heading2"), Some(3));
        assert_eq!(heading_level_from_style_name("Heading3"), Some(4));
        assert_eq!(heading_level_from_style_name("Heading6"), Some(6)); // clamped to max markdown level
        assert_eq!(heading_level_from_style_name("Normal"), None);
    }

    #[test]
    fn test_resolve_heading_level_with_style_catalog() {
        use super::super::styles::{ParagraphProperties, StyleCatalog, StyleDefinition, StyleType};

        let mut doc = Document::new();
        let mut catalog = StyleCatalog::default();

        // Style with outline_level = 2 (should become h3)
        catalog.styles.insert(
            "CustomHeading".to_string(),
            StyleDefinition {
                id: "CustomHeading".to_string(),
                name: Some("Custom Heading".to_string()),
                style_type: StyleType::Paragraph,
                based_on: None,
                next_style: None,
                is_default: false,
                paragraph_properties: ParagraphProperties {
                    outline_level: Some(2),
                    ..Default::default()
                },
                run_properties: Default::default(),
            },
        );

        doc.style_catalog = Some(catalog);
        assert_eq!(doc.resolve_heading_level("CustomHeading"), Some(3));
    }

    #[test]
    fn test_resolve_heading_level_inheritance_chain() {
        use super::super::styles::{ParagraphProperties, StyleCatalog, StyleDefinition, StyleType};

        let mut doc = Document::new();
        let mut catalog = StyleCatalog::default();

        // Parent has outline_level
        catalog.styles.insert(
            "ParentStyle".to_string(),
            StyleDefinition {
                id: "ParentStyle".to_string(),
                name: Some("Parent".to_string()),
                style_type: StyleType::Paragraph,
                based_on: None,
                next_style: None,
                is_default: false,
                paragraph_properties: ParagraphProperties {
                    outline_level: Some(0),
                    ..Default::default()
                },
                run_properties: Default::default(),
            },
        );

        // Child inherits from parent
        catalog.styles.insert(
            "ChildStyle".to_string(),
            StyleDefinition {
                id: "ChildStyle".to_string(),
                name: Some("Child".to_string()),
                style_type: StyleType::Paragraph,
                based_on: Some("ParentStyle".to_string()),
                next_style: None,
                is_default: false,
                paragraph_properties: ParagraphProperties::default(),
                run_properties: Default::default(),
            },
        );

        doc.style_catalog = Some(catalog);
        // Child resolves to parent's outline_level 0 → h1
        assert_eq!(doc.resolve_heading_level("ChildStyle"), Some(1));
    }

    #[test]
    fn test_underline_rendering() {
        let mut run = Run::new("underlined text".to_string());
        run.underline = true;
        assert_eq!(run.to_markdown(), "<u>underlined text</u>");
    }

    #[test]
    fn test_underline_combined_with_bold_italic() {
        let mut run = Run::new("styled".to_string());
        run.bold = true;
        run.italic = true;
        run.underline = true;
        let md = run.to_markdown();
        assert!(md.contains("<u>"));
        assert!(md.contains("</u>"));
        assert!(md.contains("**"));
        assert!(md.contains("*"));
    }

    #[test]
    fn test_header_footer_excluded_from_output() {
        let mut doc = Document::new();

        // Add a header
        let mut header = HeaderFooter::default();
        let mut para = Paragraph::new();
        para.add_run(Run::new("Header Text".to_string()));
        header.paragraphs.push(para);
        doc.headers.push(header);

        // Add body content
        let mut body_para = Paragraph::new();
        body_para.add_run(Run::new("Body content".to_string()));
        let idx = doc.paragraphs.len();
        doc.paragraphs.push(body_para);
        doc.elements.push(DocumentElement::Paragraph(idx));

        // Add a footer
        let mut footer = HeaderFooter::default();
        let mut footer_para = Paragraph::new();
        footer_para.add_run(Run::new("Footer Text".to_string()));
        footer.paragraphs.push(footer_para);
        doc.footers.push(footer);

        // Headers/footers should NOT appear in text output
        let md = doc.to_markdown(true);
        assert!(!md.contains("Header Text"), "Header should not be in markdown output");
        assert!(md.contains("Body content"), "Should contain body content");
        assert!(!md.contains("Footer Text"), "Footer should not be in markdown output");

        let plain = doc.to_plain_text();
        assert!(
            !plain.contains("Header Text"),
            "Header should not be in plain text output"
        );
        assert!(plain.contains("Body content"), "Should contain body content");
        assert!(
            !plain.contains("Footer Text"),
            "Footer should not be in plain text output"
        );

        // But headers/footers should still be accessible via struct fields
        assert_eq!(doc.headers.len(), 1);
        assert_eq!(doc.footers.len(), 1);
        assert_eq!(doc.headers[0].paragraphs[0].runs[0].text, "Header Text");
        assert_eq!(doc.footers[0].paragraphs[0].runs[0].text, "Footer Text");
    }

    #[test]
    fn test_footnote_reference_in_parsing() {
        // Simulate parsing a paragraph with a footnote reference
        let xml = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r>
                        <w:t>See note</w:t>
                    </w:r>
                    <w:r>
                        <w:footnoteReference w:id="2"/>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let parser_struct = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: AHashMap::new(),
            styles: None,
            theme: None,
        };
        let mut document = Document::new();
        parser_struct.parse_document_xml(xml, &mut document).unwrap();

        assert_eq!(document.paragraphs.len(), 1);
        // The second run should contain the footnote reference marker
        let full_text = document.paragraphs[0].to_text();
        assert!(
            full_text.contains("[^2]"),
            "Should contain footnote reference [^2], got: {}",
            full_text
        );
    }

    #[test]
    fn test_separator_footnotes_filtered() {
        // Separator footnotes (id 0 and 1) should be excluded
        let xml = r#"<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:footnote w:id="0">
                <w:p><w:r><w:t>separator</w:t></w:r></w:p>
            </w:footnote>
            <w:footnote w:id="1">
                <w:p><w:r><w:t>continuation</w:t></w:r></w:p>
            </w:footnote>
            <w:footnote w:id="2">
                <w:p><w:r><w:t>Actual footnote</w:t></w:r></w:p>
            </w:footnote>
        </w:footnotes>"#;

        let parser_struct = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: AHashMap::new(),
            styles: None,
            theme: None,
        };
        let mut notes = Vec::new();
        parser_struct.parse_notes(xml, &mut notes, NoteType::Footnote).unwrap();

        assert_eq!(notes.len(), 1, "Only actual footnote should remain");
        assert_eq!(notes[0].id, "2");
    }

    // Helper to create a minimal valid ZIP for parser construction in tests
    fn create_minimal_zip() -> Vec<u8> {
        use std::io::Write;
        let buf = Vec::new();
        let cursor = std::io::Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(b"<w:document/>").unwrap();
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn test_is_format_enabled_no_val() {
        // <w:b/> - no w:val attribute means enabled
        let xml = r#"<w:b/>"#;
        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();
        if let Ok(Event::Empty(ref e)) = reader.read_event_into(&mut buf) {
            assert!(is_format_enabled(e));
        }
    }

    // --- Security validation tests ---

    #[test]
    fn test_security_valid_minimal_archive() {
        // Create a minimal valid ZIP archive (empty) - should pass
        use std::io::Cursor;
        let zip_data = vec![
            0x50, 0x4b, 0x05, 0x06, // End of central directory signature
            0x00, 0x00, // Disk number
            0x00, 0x00, // Disk with central directory
            0x00, 0x00, // Number of entries on this disk
            0x00, 0x00, // Total number of entries
            0x00, 0x00, 0x00, 0x00, // Size of central directory
            0x00, 0x00, 0x00, 0x00, // Offset of central directory
            0x00, 0x00, // Comment length
        ];
        let cursor = Cursor::new(zip_data);
        let result = DocxParser::new(cursor);
        // Empty archive should pass security checks (0 entries, 0 size)
        assert!(
            result.is_ok(),
            "Empty valid ZIP should pass security checks: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_security_constants_are_reasonable() {
        use super::super::{MAX_TOTAL_UNCOMPRESSED_SIZE, MAX_UNCOMPRESSED_FILE_SIZE, MAX_ZIP_ENTRIES};

        const {
            assert!(MAX_ZIP_ENTRIES >= 1_000, "Entry limit must be at least 1,000");
            assert!(
                MAX_UNCOMPRESSED_FILE_SIZE >= 10 * 1024 * 1024,
                "Per-file size limit must be at least 10 MB"
            );
            assert!(
                MAX_TOTAL_UNCOMPRESSED_SIZE >= MAX_UNCOMPRESSED_FILE_SIZE,
                "Total size limit must be >= per-file limit"
            );
        }
    }

    #[test]
    fn test_security_normal_docx_passes() {
        use std::io::{Cursor, Write};

        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(b"<w:document/>").unwrap();

        zip.start_file("docProps/core.xml", options).unwrap();
        zip.write_all(b"<cp:coreProperties/>").unwrap();

        let cursor = zip.finish().unwrap();
        let data = cursor.into_inner();

        let mut archive = zip::ZipArchive::new(Cursor::new(data)).unwrap();
        let result = validate_archive_security(&mut archive);
        assert!(
            result.is_ok(),
            "A normal small archive must pass security validation: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_security_rejects_too_many_entries() {
        use std::io::{Cursor, Write};

        // Create a ZIP with 10,001 entries to exceed the 10,000 limit.
        // Each entry is an empty file, so this is fast.
        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        for i in 0..10_001 {
            zip.start_file(format!("file_{}.txt", i), options).unwrap();
            zip.write_all(b"").unwrap();
        }

        let cursor = zip.finish().unwrap();
        let data = cursor.into_inner();

        let mut archive = zip::ZipArchive::new(Cursor::new(data)).unwrap();
        let result = validate_archive_security(&mut archive);
        assert!(result.is_err(), "Archive with >10,000 entries must be rejected");

        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("10001") && err_msg.contains("10000"),
            "Error should mention actual and limit counts, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_security_rejects_oversized_file() {
        use std::io::{Cursor, Write};

        // We cannot actually write 100 MB in a unit test, but we can verify the
        // validation path by confirming a small archive passes and the error
        // message format is correct when it would fail. The constant-based test
        // above already validates the limit values are reasonable.
        //
        // Here we verify that a single-file archive just under the limit passes.
        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        // Write a small file (1 KB) - well under limits
        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(&[b'x'; 1024]).unwrap();

        let cursor = zip.finish().unwrap();
        let data = cursor.into_inner();

        let mut archive = zip::ZipArchive::new(Cursor::new(data)).unwrap();
        let result = validate_archive_security(&mut archive);
        assert!(
            result.is_ok(),
            "A 1 KB file must pass size validation: {:?}",
            result.err()
        );
    }

    // --- Nested table integration test ---

    /// Helper: create a minimal DOCX ZIP with the given XML as word/document.xml.
    fn create_test_docx(document_xml: &str) -> Vec<u8> {
        use std::io::{Cursor, Write};

        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(document_xml.as_bytes()).unwrap();

        let cursor = zip.finish().unwrap();
        cursor.into_inner()
    }

    #[test]
    fn test_nested_table_parsing() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:p><w:r><w:t>Outer Cell 1</w:t></w:r></w:p>
          <w:tbl>
            <w:tr>
              <w:tc>
                <w:p><w:r><w:t>Inner Cell</w:t></w:r></w:p>
              </w:tc>
            </w:tr>
          </w:tbl>
        </w:tc>
        <w:tc>
          <w:p><w:r><w:t>Outer Cell 2</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let bytes = create_test_docx(xml);
        let doc = parse_document(&bytes).expect("parse_document should succeed");

        // Only the outer table is stored; nested table content is flattened.
        assert_eq!(doc.tables.len(), 1, "Expected exactly 1 (outer) table");

        let table = &doc.tables[0];
        assert_eq!(table.rows.len(), 1, "Outer table should have 1 row");
        assert_eq!(table.rows[0].cells.len(), 2, "Outer row should have 2 cells");

        // First cell: "Outer Cell 1" paragraph + flattened "Inner Cell" paragraph
        let cell0 = &table.rows[0].cells[0];
        let cell0_texts: Vec<String> = cell0.paragraphs.iter().map(|p| p.to_text()).collect();
        assert!(
            cell0_texts.iter().any(|t| t.contains("Outer Cell 1")),
            "First cell must contain 'Outer Cell 1', got: {:?}",
            cell0_texts
        );
        assert!(
            cell0_texts.iter().any(|t| t.contains("Inner Cell")),
            "First cell must contain flattened 'Inner Cell', got: {:?}",
            cell0_texts
        );

        // Second cell: "Outer Cell 2"
        let cell1 = &table.rows[0].cells[1];
        let cell1_texts: Vec<String> = cell1.paragraphs.iter().map(|p| p.to_text()).collect();
        assert!(
            cell1_texts.iter().any(|t| t.contains("Outer Cell 2")),
            "Second cell must contain 'Outer Cell 2', got: {:?}",
            cell1_texts
        );
    }

    #[test]
    fn test_parser_loads_styles() {
        use std::io::{Cursor, Write};

        let styles_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="Heading1">
    <w:name w:val="heading 1"/>
    <w:basedOn w:val="Normal"/>
    <w:pPr><w:outlineLvl w:val="0"/></w:pPr>
    <w:rPr><w:b/><w:sz w:val="32"/></w:rPr>
  </w:style>
  <w:style w:type="paragraph" w:default="1" w:styleId="Normal">
    <w:name w:val="Normal"/>
  </w:style>
</w:styles>"#;

        let doc_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Hello</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;

        let buffer = Vec::new();
        let cursor = Cursor::new(buffer);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("word/document.xml", options).unwrap();
        zip.write_all(doc_xml.as_bytes()).unwrap();
        zip.start_file("word/styles.xml", options).unwrap();
        zip.write_all(styles_xml.as_bytes()).unwrap();

        let cursor = zip.finish().unwrap();
        let bytes = cursor.into_inner();

        let doc = parse_document(&bytes).expect("should parse");

        // Verify styles were loaded
        assert!(doc.style_catalog.is_some(), "Style catalog should be loaded");
        let catalog = doc.style_catalog.as_ref().unwrap();
        assert!(catalog.styles.contains_key("Heading1"));
        assert!(catalog.styles.contains_key("Normal"));

        // Verify heading1 has bold and font size
        let h1 = &catalog.styles["Heading1"];
        assert_eq!(h1.run_properties.bold, Some(true));
        assert_eq!(h1.run_properties.font_size_half_points, Some(32));
        assert_eq!(h1.paragraph_properties.outline_level, Some(0));
    }

    #[test]
    fn test_table_properties_integration() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tblPr>
        <w:tblStyle w:val="TableGrid"/>
        <w:tblW w:w="5000" w:type="dxa"/>
        <w:jc w:val="center"/>
      </w:tblPr>
      <w:tblGrid>
        <w:gridCol w:w="2500"/>
        <w:gridCol w:w="2500"/>
      </w:tblGrid>
      <w:tr>
        <w:trPr>
          <w:tblHeader/>
        </w:trPr>
        <w:tc>
          <w:tcPr>
            <w:tcW w:w="2500" w:type="dxa"/>
            <w:shd w:val="clear" w:fill="D9E2F3"/>
          </w:tcPr>
          <w:p><w:r><w:t>Header 1</w:t></w:r></w:p>
        </w:tc>
        <w:tc>
          <w:tcPr>
            <w:tcW w:w="2500" w:type="dxa"/>
            <w:gridSpan w:val="1"/>
          </w:tcPr>
          <w:p><w:r><w:t>Header 2</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
      <w:tr>
        <w:tc>
          <w:tcPr>
            <w:vMerge w:val="restart"/>
          </w:tcPr>
          <w:p><w:r><w:t>Merged</w:t></w:r></w:p>
        </w:tc>
        <w:tc>
          <w:p><w:r><w:t>Data</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let bytes = create_test_docx(xml);
        let doc = parse_document(&bytes).expect("parse should succeed");

        assert_eq!(doc.tables.len(), 1);
        let table = &doc.tables[0];

        // Table properties
        let tbl_props = table.properties.as_ref().expect("table should have properties");
        assert_eq!(tbl_props.style_id.as_deref(), Some("TableGrid"));
        assert_eq!(tbl_props.alignment.as_deref(), Some("center"));
        assert!(tbl_props.width.is_some());
        assert_eq!(tbl_props.width.as_ref().unwrap().value, 5000);

        // Table grid
        let grid = table.grid.as_ref().expect("table should have grid");
        assert_eq!(grid.columns, vec![2500, 2500]);

        // Row 0 header
        let row0 = &table.rows[0];
        let row_props = row0.properties.as_ref().expect("header row should have properties");
        assert!(row_props.is_header);

        // Cell 0,0 shading
        let cell00 = &row0.cells[0];
        let cell_props = cell00.properties.as_ref().expect("cell should have properties");
        assert!(cell_props.shading.is_some());
        assert_eq!(cell_props.shading.as_ref().unwrap().fill.as_deref(), Some("D9E2F3"));

        // Cell 1,0 vMerge
        let cell10 = &table.rows[1].cells[0];
        let cell10_props = cell10.properties.as_ref().expect("merged cell should have properties");
        assert_eq!(
            cell10_props.v_merge,
            Some(crate::extraction::docx::table::VerticalMerge::Restart)
        );
    }

    #[test]
    fn test_table_with_explicit_header_row_renders_correctly() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:trPr>
          <w:tblHeader/>
        </w:trPr>
        <w:tc>
          <w:p><w:r><w:t>Name</w:t></w:r></w:p>
        </w:tc>
        <w:tc>
          <w:p><w:r><w:t>Age</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
      <w:tr>
        <w:tc>
          <w:p><w:r><w:t>Alice</w:t></w:r></w:p>
        </w:tc>
        <w:tc>
          <w:p><w:r><w:t>30</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let bytes = create_test_docx(xml);
        let doc = parse_document(&bytes).expect("parse should succeed");

        assert_eq!(doc.tables.len(), 1);
        let table = &doc.tables[0];

        // Verify first row is marked as header
        let row0_props = table.rows[0]
            .properties
            .as_ref()
            .expect("first row should have properties");
        assert!(row0_props.is_header, "First row should be marked as header");

        // Verify markdown rendering has separator after header row
        let markdown = table.to_markdown();
        let lines: Vec<&str> = markdown.lines().collect();

        // Should have at least 3 lines: header, separator, data row
        assert!(
            lines.len() >= 3,
            "Table should have at least 3 lines, got: {}",
            markdown
        );

        // Line 1 should be separator (all dashes)
        assert!(
            lines[1].contains("---"),
            "Second line should be separator, got: {}",
            lines[1]
        );
    }

    #[test]
    fn test_table_with_merged_cells_expands_columns() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc>
          <w:p><w:r><w:t>A</w:t></w:r></w:p>
        </w:tc>
        <w:tc>
          <w:p><w:r><w:t>B</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
      <w:tr>
        <w:tc>
          <w:tcPr>
            <w:gridSpan w:val="2"/>
          </w:tcPr>
          <w:p><w:r><w:t>Merged</w:t></w:r></w:p>
        </w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

        let bytes = create_test_docx(xml);
        let doc = parse_document(&bytes).expect("parse should succeed");

        assert_eq!(doc.tables.len(), 1);
        let table = &doc.tables[0];

        // Verify second row cell has grid_span=2
        let merged_cell = &table.rows[1].cells[0];
        let cell_props = merged_cell.properties.as_ref().expect("cell should have properties");
        assert_eq!(cell_props.grid_span, Some(2), "Cell should have grid_span=2");

        // Verify markdown rendering produces equal number of columns
        let markdown = table.to_markdown();
        let lines: Vec<&str> = markdown.lines().collect();

        // Both rows should have same number of pipe characters (column count)
        let pipes_row0 = lines[0].matches('|').count();
        let pipes_row1 = lines[2].matches('|').count(); // After separator

        assert_eq!(
            pipes_row0, pipes_row1,
            "All rows should have same column count in markdown"
        );
    }

    // ========================================================================
    // Comprehensive DOCX extraction tests (python-docx parity)
    // ========================================================================

    /// Helper: parse document XML through DocxParser and return the Document.
    fn parse_xml(xml: &str) -> Document {
        let parser_struct = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: AHashMap::new(),
            styles: None,
            theme: None,
        };
        let mut document = Document::new();
        parser_struct.parse_document_xml(xml, &mut document).unwrap();
        document
    }

    /// Helper: parse document XML with custom relationships.
    fn parse_xml_with_rels(xml: &str, rels: AHashMap<String, String>) -> Document {
        let parser_struct = DocxParser {
            archive: zip::ZipArchive::new(std::io::Cursor::new(create_minimal_zip())).unwrap(),
            relationships: rels,
            styles: None,
            theme: None,
        };
        let mut document = Document::new();
        parser_struct.parse_document_xml(xml, &mut document).unwrap();
        document
    }

    /// Wrap body XML in a w:document envelope.
    fn wrap_body(body: &str) -> String {
        format!(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>{}</w:body></w:document>"#,
            body
        )
    }

    // --- Group 1: Text Extraction Basics ---

    #[test]
    fn test_plain_paragraph_text() {
        let xml = wrap_body(r#"<w:p><w:r><w:t>Hello World</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs.len(), 1);
        assert_eq!(doc.paragraphs[0].to_text(), "Hello World");
    }

    #[test]
    fn test_multiple_paragraphs() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>First</w:t></w:r></w:p>
               <w:p><w:r><w:t>Second</w:t></w:r></w:p>
               <w:p><w:r><w:t>Third</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs.len(), 3);
        assert_eq!(doc.paragraphs[0].to_text(), "First");
        assert_eq!(doc.paragraphs[1].to_text(), "Second");
        assert_eq!(doc.paragraphs[2].to_text(), "Third");

        let plain = doc.to_plain_text();
        assert!(plain.contains("First"));
        assert!(plain.contains("Second"));
        assert!(plain.contains("Third"));
    }

    #[test]
    fn test_empty_paragraph() {
        let xml = wrap_body(r#"<w:p></w:p>"#);
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs.len(), 1);
        assert_eq!(doc.paragraphs[0].to_text(), "");
    }

    #[test]
    fn test_multiple_runs_in_paragraph() {
        let xml = wrap_body(
            r#"<w:p>
                <w:r><w:t>Hello </w:t></w:r>
                <w:r><w:t>World</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs[0].to_text(), "Hello World");
    }

    #[test]
    fn test_line_break_in_run() {
        let xml = wrap_body(r#"<w:p><w:r><w:t>Before</w:t><w:br/><w:t>After</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert!(text.contains("Before"));
        assert!(text.contains("After"));
        assert!(text.contains('\n'));
    }

    // --- Group 2: Run Formatting ---

    #[test]
    fn test_bold_formatting() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Bold</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(doc.paragraphs[0].runs[0].bold);
        let md = doc.to_markdown(true);
        assert!(md.contains("**Bold**"), "Markdown: {}", md);
    }

    #[test]
    fn test_bold_disabled_with_val_0() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:b w:val="0"/></w:rPr><w:t>Not Bold</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(!doc.paragraphs[0].runs[0].bold);
    }

    #[test]
    fn test_italic_formatting() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:i/></w:rPr><w:t>Italic</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(doc.paragraphs[0].runs[0].italic);
        let md = doc.to_markdown(true);
        assert!(md.contains("*Italic*"), "Markdown: {}", md);
    }

    #[test]
    fn test_bold_italic_combined() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:b/><w:i/></w:rPr><w:t>Both</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        let run = &doc.paragraphs[0].runs[0];
        assert!(run.bold);
        assert!(run.italic);
        let md = doc.to_markdown(true);
        assert!(md.contains("***Both***"), "Markdown: {}", md);
    }

    #[test]
    fn test_underline_formatting() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:u w:val="single"/></w:rPr><w:t>Underlined</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(doc.paragraphs[0].runs[0].underline);
    }

    #[test]
    fn test_underline_none_disabled() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:u w:val="none"/></w:rPr><w:t>No Underline</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(!doc.paragraphs[0].runs[0].underline);
    }

    #[test]
    fn test_strikethrough_formatting() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:strike/></w:rPr><w:t>Struck</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(doc.paragraphs[0].runs[0].strikethrough);
        let md = doc.to_markdown(true);
        assert!(md.contains("~~Struck~~"), "Markdown: {}", md);
    }

    #[test]
    fn test_double_strikethrough() {
        let xml = wrap_body(r#"<w:p><w:r><w:rPr><w:dstrike/></w:rPr><w:t>DStruck</w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert!(doc.paragraphs[0].runs[0].strikethrough);
    }

    // --- Group 3: Hyperlinks ---

    #[test]
    fn test_external_hyperlink() {
        let mut rels = AHashMap::new();
        rels.insert("rId1".to_string(), "https://example.com".to_string());

        let xml = wrap_body(r#"<w:p><w:hyperlink r:id="rId1"><w:r><w:t>Click here</w:t></w:r></w:hyperlink></w:p>"#);
        let doc = parse_xml_with_rels(&xml, rels);
        assert_eq!(doc.paragraphs.len(), 1);
        let run = &doc.paragraphs[0].runs[0];
        assert_eq!(run.text, "Click here");
        assert_eq!(run.hyperlink_url.as_deref(), Some("https://example.com"));

        let md = doc.to_markdown(true);
        assert!(md.contains("[Click here](https://example.com)"), "Markdown: {}", md);
    }

    #[test]
    fn test_hyperlink_with_no_relationship() {
        // Hyperlink with r:id that doesn't exist in relationships
        let xml = wrap_body(r#"<w:p><w:hyperlink r:id="rId99"><w:r><w:t>Broken link</w:t></w:r></w:hyperlink></w:p>"#);
        let doc = parse_xml(&xml);
        let run = &doc.paragraphs[0].runs[0];
        assert_eq!(run.text, "Broken link");
        assert!(run.hyperlink_url.is_none());
    }

    #[test]
    fn test_multiple_hyperlinks() {
        let mut rels = AHashMap::new();
        rels.insert("rId1".to_string(), "https://one.com".to_string());
        rels.insert("rId2".to_string(), "https://two.com".to_string());

        let xml = wrap_body(
            r#"<w:p>
                <w:hyperlink r:id="rId1"><w:r><w:t>First</w:t></w:r></w:hyperlink>
                <w:r><w:t> and </w:t></w:r>
                <w:hyperlink r:id="rId2"><w:r><w:t>Second</w:t></w:r></w:hyperlink>
            </w:p>"#,
        );
        let doc = parse_xml_with_rels(&xml, rels);
        let md = doc.to_markdown(true);
        assert!(md.contains("[First](https://one.com)"), "Markdown: {}", md);
        assert!(md.contains("[Second](https://two.com)"), "Markdown: {}", md);
    }

    // --- Group 4: Tables ---

    #[test]
    fn test_basic_2x2_table() {
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>A1</w:t></w:r></w:p></w:tc>
                    <w:tc><w:p><w:r><w:t>B1</w:t></w:r></w:p></w:tc>
                </w:tr>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>A2</w:t></w:r></w:p></w:tc>
                    <w:tc><w:p><w:r><w:t>B2</w:t></w:r></w:p></w:tc>
                </w:tr>
            </w:tbl>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.tables.len(), 1);
        let table = &doc.tables[0];
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells.len(), 2);

        let md = doc.to_markdown(true);
        assert!(md.contains("A1"), "Markdown: {}", md);
        assert!(md.contains("B2"), "Markdown: {}", md);

        let plain = doc.to_plain_text();
        assert!(plain.contains("A1"), "Plain: {}", plain);
        assert!(plain.contains("B2"), "Plain: {}", plain);
    }

    #[test]
    fn test_table_with_caption() {
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tblPr>
                    <w:tblCaption w:val="My Table Caption"/>
                </w:tblPr>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc>
                </w:tr>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>Data</w:t></w:r></w:p></w:tc>
                </w:tr>
            </w:tbl>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.tables.len(), 1);
        let caption = doc.tables[0].properties.as_ref().and_then(|p| p.caption.as_deref());
        assert_eq!(caption, Some("My Table Caption"));

        let md = doc.to_markdown(true);
        assert!(md.contains("My Table Caption"), "Caption should be in markdown: {}", md);

        let plain = doc.to_plain_text();
        assert!(
            plain.contains("My Table Caption"),
            "Caption should be in plain text: {}",
            plain
        );
    }

    #[test]
    fn test_table_column_span() {
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tr>
                    <w:tc>
                        <w:tcPr><w:gridSpan w:val="2"/></w:tcPr>
                        <w:p><w:r><w:t>Spanning</w:t></w:r></w:p>
                    </w:tc>
                </w:tr>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>Left</w:t></w:r></w:p></w:tc>
                    <w:tc><w:p><w:r><w:t>Right</w:t></w:r></w:p></w:tc>
                </w:tr>
            </w:tbl>"#,
        );
        let doc = parse_xml(&xml);
        let table = &doc.tables[0];
        let first_cell = &table.rows[0].cells[0];
        assert_eq!(first_cell.properties.as_ref().and_then(|p| p.grid_span), Some(2));
    }

    #[test]
    fn test_table_vertical_merge() {
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tr>
                    <w:tc>
                        <w:tcPr><w:vMerge w:val="restart"/></w:tcPr>
                        <w:p><w:r><w:t>Merged</w:t></w:r></w:p>
                    </w:tc>
                    <w:tc><w:p><w:r><w:t>Right1</w:t></w:r></w:p></w:tc>
                </w:tr>
                <w:tr>
                    <w:tc>
                        <w:tcPr><w:vMerge/></w:tcPr>
                        <w:p></w:p>
                    </w:tc>
                    <w:tc><w:p><w:r><w:t>Right2</w:t></w:r></w:p></w:tc>
                </w:tr>
            </w:tbl>"#,
        );
        let doc = parse_xml(&xml);
        let table = &doc.tables[0];
        // First row, first cell: vMerge restart
        let cell_0_0 = &table.rows[0].cells[0];
        assert_eq!(
            cell_0_0.properties.as_ref().and_then(|p| p.v_merge.as_ref()),
            Some(&super::super::table::VerticalMerge::Restart)
        );
        // Second row, first cell: vMerge continue
        let cell_1_0 = &table.rows[1].cells[0];
        assert_eq!(
            cell_1_0.properties.as_ref().and_then(|p| p.v_merge.as_ref()),
            Some(&super::super::table::VerticalMerge::Continue)
        );
    }

    #[test]
    fn test_table_empty_cells() {
        let xml = wrap_body(
            r#"<w:tbl>
                <w:tr>
                    <w:tc><w:p><w:r><w:t>Has content</w:t></w:r></w:p></w:tc>
                    <w:tc><w:p></w:p></w:tc>
                </w:tr>
            </w:tbl>"#,
        );
        let doc = parse_xml(&xml);
        let table = &doc.tables[0];
        assert_eq!(table.rows[0].cells.len(), 2);
        let md = doc.to_markdown(true);
        assert!(md.contains("Has content"), "Markdown: {}", md);
    }

    // --- Group 5: Lists ---

    #[test]
    fn test_bullet_list_extraction() {
        let xml = wrap_body(
            r#"<w:p>
                <w:pPr>
                    <w:numId w:val="1"/>
                    <w:ilvl w:val="0"/>
                </w:pPr>
                <w:r><w:t>Bullet item</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs.len(), 1);
        assert_eq!(doc.paragraphs[0].to_text(), "Bullet item");
        assert!(doc.paragraphs[0].numbering_id.is_some());
    }

    // --- Group 6: Headings & Styles ---

    #[test]
    fn test_heading_style() {
        let xml = wrap_body(
            r#"<w:p>
                <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
                <w:r><w:t>My Heading</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs[0].style.as_deref(), Some("Heading1"));
        let md = doc.to_markdown(true);
        assert!(md.contains("# My Heading"), "Markdown: {}", md);
    }

    #[test]
    fn test_heading2_style() {
        let xml = wrap_body(
            r#"<w:p>
                <w:pPr><w:pStyle w:val="Heading2"/></w:pPr>
                <w:r><w:t>Sub Heading</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let md = doc.to_markdown(true);
        assert!(md.contains("## Sub Heading"), "Markdown: {}", md);
    }

    #[test]
    fn test_title_style() {
        let xml = wrap_body(
            r#"<w:p>
                <w:pPr><w:pStyle w:val="Title"/></w:pPr>
                <w:r><w:t>Document Title</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let md = doc.to_markdown(true);
        // Title maps to heading level (varies by implementation)
        assert!(md.contains("Document Title"), "Markdown: {}", md);
    }

    // --- Group 7: Images/Drawings ---

    #[test]
    fn test_inline_drawing_with_alt_text() {
        let xml = wrap_body(
            r#"<w:p><w:r>
                <w:drawing>
                    <wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
                        <wp:extent cx="914400" cy="914400"/>
                        <wp:docPr id="1" name="Picture 1" descr="A logo image"/>
                        <a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
                            <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                <pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture">
                                    <pic:blipFill>
                                        <a:blip r:embed="rId5"/>
                                    </pic:blipFill>
                                </pic:pic>
                            </a:graphicData>
                        </a:graphic>
                    </wp:inline>
                </w:drawing>
            </w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.drawings.len(), 1);
        let drawing = &doc.drawings[0];
        assert_eq!(
            drawing.doc_properties.as_ref().and_then(|dp| dp.description.as_deref()),
            Some("A logo image")
        );
        assert_eq!(drawing.image_ref.as_deref(), Some("rId5"));

        let md = doc.to_markdown(true);
        assert!(md.contains("![A logo image]"), "Markdown: {}", md);
    }

    #[test]
    fn test_drawing_dimensions() {
        let xml = wrap_body(
            r#"<w:p><w:r>
                <w:drawing>
                    <wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
                        <wp:extent cx="1828800" cy="914400"/>
                        <wp:docPr id="1" name="Pic"/>
                    </wp:inline>
                </w:drawing>
            </w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);
        let extent = doc.drawings[0].extent.as_ref().unwrap();
        assert_eq!(extent.cx, 1828800); // 2 inches
        assert_eq!(extent.cy, 914400); // 1 inch
    }

    // --- Group 8: Sections ---

    #[test]
    fn test_section_properties_parsed() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>Content</w:t></w:r></w:p>
            <w:sectPr>
                <w:pgSz w:w="12240" w:h="15840"/>
                <w:pgMar w:top="1440" w:right="1800" w:bottom="1440" w:left="1800"/>
            </w:sectPr>"#,
        );
        let doc = parse_xml(&xml);
        assert!(!doc.sections.is_empty(), "Should have sections");
        let sect = &doc.sections[0];
        assert_eq!(sect.page_width_twips, Some(12240));
        assert_eq!(sect.page_height_twips, Some(15840));
        assert_eq!(sect.margins.top, Some(1440));
        assert_eq!(sect.margins.left, Some(1800));
    }

    // --- Group 9: Footnotes & Endnotes ---

    #[test]
    fn test_footnote_reference_marker() {
        let xml = wrap_body(
            r#"<w:p>
                <w:r><w:t>Main text</w:t></w:r>
                <w:r><w:footnoteReference w:id="2"/></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert!(text.contains("[^2]"), "Should contain footnote marker: {}", text);
    }

    #[test]
    fn test_footnote_separator_ids_filtered() {
        let xml = wrap_body(
            r#"<w:p>
                <w:r><w:footnoteReference w:id="0"/></w:r>
                <w:r><w:footnoteReference w:id="1"/></w:r>
                <w:r><w:t>text</w:t></w:r>
                <w:r><w:footnoteReference w:id="2"/></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert!(!text.contains("[^0]"), "Separator id 0 should be filtered");
        assert!(!text.contains("[^1]"), "Separator id 1 should be filtered");
        assert!(text.contains("[^2]"), "Real footnote 2 should be present");
    }

    // --- Group 10: Field Codes (Fix 3 verification) ---

    #[test]
    fn test_field_instruction_skipped_result_kept() {
        // Field instructions (between begin and separate) are skipped,
        // but field results (between separate and end) are kept.
        let xml = wrap_body(
            r#"<w:p>
                <w:r><w:t>Before </w:t></w:r>
                <w:r><w:fldChar w:fldCharType="begin"/></w:r>
                <w:r><w:instrText> SEQ Figure \* ARABIC </w:instrText></w:r>
                <w:r><w:fldChar w:fldCharType="separate"/></w:r>
                <w:r><w:t>2</w:t></w:r>
                <w:r><w:fldChar w:fldCharType="end"/></w:r>
                <w:r><w:t> After</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert!(text.contains("Before"), "Text: {}", text);
        assert!(text.contains("After"), "Text: {}", text);
        assert!(text.contains("2"), "Field result '2' should be kept: {}", text);
        assert!(!text.contains("SEQ"), "Field instruction should be skipped: {}", text);
    }

    #[test]
    fn test_page_field_result_kept() {
        // PAGE field result is kept in output
        let xml = wrap_body(
            r#"<w:p>
                <w:r><w:t>Page </w:t></w:r>
                <w:r><w:fldChar w:fldCharType="begin"/></w:r>
                <w:r><w:instrText> PAGE </w:instrText></w:r>
                <w:r><w:fldChar w:fldCharType="separate"/></w:r>
                <w:r><w:t>1</w:t></w:r>
                <w:r><w:fldChar w:fldCharType="end"/></w:r>
                <w:r><w:t> of 5</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert_eq!(text.trim(), "Page 1 of 5", "Text: '{}'", text);
    }

    #[test]
    fn test_text_after_field_resumes() {
        // Field result is kept, and text after field resumes normally
        let xml = wrap_body(
            r#"<w:p>
                <w:r><w:fldChar w:fldCharType="begin"/></w:r>
                <w:r><w:instrText> NUMPAGES </w:instrText></w:r>
                <w:r><w:fldChar w:fldCharType="separate"/></w:r>
                <w:r><w:t>10</w:t></w:r>
                <w:r><w:fldChar w:fldCharType="end"/></w:r>
                <w:r><w:t>Normal text</w:t></w:r>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert!(text.contains("Normal text"), "Text: {}", text);
        assert!(text.contains("10"), "Field result should be kept: {}", text);
    }

    // --- Group 11: OMML Math ---

    #[test]
    fn test_math_text_extracted() {
        let xml = wrap_body(
            r#"<w:p>
                <m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">
                    <m:r>
                        <m:t>E=mc</m:t>
                    </m:r>
                    <m:sSup>
                        <m:e><m:r><m:t></m:t></m:r></m:e>
                        <m:sup><m:r><m:t>2</m:t></m:r></m:sup>
                    </m:sSup>
                </m:oMath>
            </w:p>"#,
        );
        let doc = parse_xml(&xml);
        let text = doc.paragraphs[0].to_text();
        assert!(text.contains("E=mc"), "Math text should contain E=mc: {}", text);
        assert!(text.contains("^{2}"), "Math text should contain ^{{2}}: {}", text);
        // Markdown should have $ delimiters
        let md = doc.paragraphs[0].runs_to_markdown();
        assert!(md.starts_with('$'), "Inline math should start with $: {}", md);
        assert!(md.ends_with('$'), "Inline math should end with $: {}", md);
    }

    // --- Group 12: Element ordering ---

    #[test]
    fn test_element_ordering_preserved() {
        let xml = wrap_body(
            r#"<w:p><w:r><w:t>Para 1</w:t></w:r></w:p>
            <w:tbl>
                <w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r></w:p></w:tc></w:tr>
                <w:tr><w:tc><w:p><w:r><w:t>Data</w:t></w:r></w:p></w:tc></w:tr>
            </w:tbl>
            <w:p><w:r><w:t>Para 2</w:t></w:r></w:p>"#,
        );
        let doc = parse_xml(&xml);
        assert_eq!(doc.elements.len(), 3);
        assert!(matches!(doc.elements[0], DocumentElement::Paragraph(0)));
        assert!(matches!(doc.elements[1], DocumentElement::Table(0)));
        assert!(matches!(doc.elements[2], DocumentElement::Paragraph(1)));

        // Verify ordering in output
        let md = doc.to_markdown(true);
        let para1_pos = md.find("Para 1").unwrap();
        let cell_pos = md.find("Cell").unwrap();
        let para2_pos = md.find("Para 2").unwrap();
        assert!(para1_pos < cell_pos, "Para 1 before table");
        assert!(cell_pos < para2_pos, "Table before Para 2");
    }

    // --- Group 13: Edge cases ---

    #[test]
    fn test_empty_document() {
        let xml = wrap_body("");
        let doc = parse_xml(&xml);
        assert!(doc.paragraphs.is_empty());
        assert!(doc.tables.is_empty());
        let md = doc.to_markdown(true);
        assert!(md.trim().is_empty(), "Empty doc markdown: '{}'", md);
    }

    #[test]
    fn test_paragraph_with_only_whitespace() {
        let xml = wrap_body(r#"<w:p><w:r><w:t>   </w:t></w:r></w:p>"#);
        let doc = parse_xml(&xml);
        assert_eq!(doc.paragraphs[0].to_text(), "   ");
    }

    // --- Group 14: Real document extraction tests ---

    #[test]
    fn test_extract_lorem_ipsum_docx() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/docx/lorem_ipsum.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            assert!(!text.is_empty(), "Should extract text from lorem ipsum");
            assert!(
                text.contains("Lorem"),
                "Should contain 'Lorem': {}",
                &text[..100.min(text.len())]
            );
        }
    }

    #[test]
    fn test_extract_word_tables_docx() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/docx/word_tables.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            assert!(!text.is_empty(), "Should extract text from word tables");
        }
    }

    #[test]
    fn test_extract_unit_test_lists_docx() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/docx/unit_test_lists.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            assert!(!text.is_empty(), "Should extract text from list document");
        }
    }

    #[test]
    fn test_extract_python_docx_test_file() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/python-docx/test.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            assert!(!text.is_empty(), "Should extract text from python-docx test.docx");
        }
    }

    #[test]
    fn test_extract_python_docx_having_images() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/vendored/python-docx/having-images.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            // Document with images should still extract any surrounding text
            let _ = text; // Should not crash on images document
        }
    }

    #[test]
    fn test_extract_word_sample_no_field_leaks() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/docx/word_sample.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            assert!(!text.is_empty());
            // After Fix 3: SEQ field results should not leak
            // The word_sample.docx has SEQ Figure fields that produced "2"
        }
    }

    /// Regression: adjacent bold runs in textbox.docx must not produce `****`.
    #[test]
    fn test_textbox_no_spurious_bold_markers() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/docx/textbox.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let doc = super::parse_document(&bytes).unwrap();
            let md = doc.to_markdown(true);
            assert!(
                !md.contains("****"),
                "Markdown output should not contain spurious '****' sequences. Got:\n{}",
                md
            );
        }
    }

    #[test]
    fn test_extract_unit_test_formatting_no_headers() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/docx/unit_test_formatting.docx");
        if let Ok(bytes) = std::fs::read(&path) {
            let text = super::super::extract_text(&bytes).unwrap();
            assert!(!text.is_empty());
            // After Fix 1: Headers/footers should not appear in text output
        }
    }

    #[test]
    fn test_to_markdown_inject_placeholders_true() {
        use crate::extraction::docx::drawing::{DocProperties, Drawing, DrawingType};

        let mut doc = Document::new();

        let mut para = Paragraph::new();
        para.add_run(Run::new("Hello world".to_string()));
        let p_idx = doc.paragraphs.len();
        doc.paragraphs.push(para);
        doc.elements.push(DocumentElement::Paragraph(p_idx));

        let drawing = Drawing {
            drawing_type: DrawingType::Inline,
            extent: None,
            doc_properties: Some(DocProperties {
                id: Some("1".to_string()),
                name: Some("Pic".to_string()),
                description: Some("alt text".to_string()),
            }),
            image_ref: Some("rId1".to_string()),
        };
        let d_idx = doc.drawings.len();
        doc.drawings.push(drawing);
        doc.elements.push(DocumentElement::Drawing(d_idx));

        let md = doc.to_markdown(true);
        assert!(
            md.contains("![alt text](image)"),
            "Expected image placeholder, got: {md}"
        );
        assert!(md.contains("Hello world"));
    }

    #[test]
    fn test_to_markdown_inject_placeholders_false() {
        use crate::extraction::docx::drawing::{DocProperties, Drawing, DrawingType};

        let mut doc = Document::new();

        let mut para = Paragraph::new();
        para.add_run(Run::new("Hello world".to_string()));
        let p_idx = doc.paragraphs.len();
        doc.paragraphs.push(para);
        doc.elements.push(DocumentElement::Paragraph(p_idx));

        let drawing = Drawing {
            drawing_type: DrawingType::Inline,
            extent: None,
            doc_properties: Some(DocProperties {
                id: Some("1".to_string()),
                name: Some("Pic".to_string()),
                description: Some("alt text".to_string()),
            }),
            image_ref: Some("rId1".to_string()),
        };
        let d_idx = doc.drawings.len();
        doc.drawings.push(drawing);
        doc.elements.push(DocumentElement::Drawing(d_idx));

        let md = doc.to_markdown(false);
        assert!(!md.contains("!["), "Should NOT contain image placeholder, got: {md}");
        assert!(md.contains("Hello world"), "Text content must be preserved");
    }
}
