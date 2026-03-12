//! ODT (OpenDocument Text) extractor using native Rust parsing.
//!
//! Supports: OpenDocument Text (.odt)

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::{cells_to_markdown, office_metadata};
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata, Table};
use ahash::AHashMap;
use async_trait::async_trait;
use roxmltree::Document;
use std::borrow::Cow;
use std::io::Cursor;

/// High-performance ODT extractor using native Rust XML parsing.
///
/// This extractor provides:
/// - Fast text extraction via roxmltree XML parsing
/// - Comprehensive metadata extraction from meta.xml
/// - Table extraction with row and cell support
/// - Formatting preservation (bold, italic, strikeout)
/// - Support for headings, paragraphs, and special elements
pub struct OdtExtractor;

impl OdtExtractor {
    /// Create a new ODT extractor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for OdtExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for OdtExtractor {
    fn name(&self) -> &str {
        "odt-extractor"
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
        "Native Rust ODT (OpenDocument Text) extractor with metadata and table support"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

/// Replace a word in a string only if it appears as a whole word
/// (not as a substring of a larger word).
fn replace_whole_word(input: &str, word: &str, replacement: &str) -> String {
    let mut result = String::new();
    let mut remaining = input;

    while let Some(pos) = remaining.find(word) {
        // Check character before the match
        let before_ok = if pos == 0 {
            true
        } else {
            let prev_char = remaining[..pos].chars().next_back().unwrap();
            !prev_char.is_alphanumeric()
        };

        // Check character after the match
        let after_pos = pos + word.len();
        let after_ok = if after_pos >= remaining.len() {
            true
        } else {
            let next_char = remaining[after_pos..].chars().next().unwrap();
            !next_char.is_alphanumeric()
        };

        if before_ok && after_ok {
            result.push_str(&remaining[..pos]);
            result.push_str(replacement);
            remaining = &remaining[after_pos..];
        } else {
            result.push_str(&remaining[..after_pos]);
            remaining = &remaining[after_pos..];
        }
    }

    result.push_str(remaining);
    result
}

/// Convert StarMath notation to Unicode text.
///
/// Handles common StarMath operators and superscript/subscript notation,
/// converting them to their Unicode equivalents.
fn starmath_to_unicode(formula: &str) -> String {
    let mut result = formula.to_string();

    // Replace StarMath operators with Unicode equivalents
    let replacements = [
        ("cdot", "\u{22C5}"),    // ⋅
        ("times", "\u{00D7}"),   // ×
        ("div", "\u{00F7}"),     // ÷
        ("pm", "\u{00B1}"),      // ±
        ("mp", "\u{2213}"),      // ∓
        ("le", "\u{2264}"),      // ≤
        ("ge", "\u{2265}"),      // ≥
        ("ne", "\u{2260}"),      // ≠
        ("approx", "\u{2248}"),  // ≈
        ("equiv", "\u{2261}"),   // ≡
        ("inf", "\u{221E}"),     // ∞
        ("partial", "\u{2202}"), // ∂
        ("nabla", "\u{2207}"),   // ∇
        ("sum", "\u{2211}"),     // ∑
        ("prod", "\u{220F}"),    // ∏
        ("int", "\u{222B}"),     // ∫
        ("sqrt", "\u{221A}"),    // √
        ("alpha", "\u{03B1}"),   // α
        ("beta", "\u{03B2}"),    // β
        ("gamma", "\u{03B3}"),   // γ
        ("delta", "\u{03B4}"),   // δ
        ("pi", "\u{03C0}"),      // π
        ("sigma", "\u{03C3}"),   // σ
        ("theta", "\u{03B8}"),   // θ
        ("lambda", "\u{03BB}"),  // λ
        ("mu", "\u{03BC}"),      // μ
        ("omega", "\u{03C9}"),   // ω
    ];

    for (from, to) in &replacements {
        // Replace only whole words (not substrings within words)
        result = replace_whole_word(&result, from, to);
    }

    // Handle superscripts: ^{...} or ^N (single digit)
    result = convert_superscripts(&result);
    // Handle subscripts: _{...} or _N (single digit)
    result = convert_subscripts(&result);

    result
}

/// Convert superscript notation (^2, ^{10}, etc.) to Unicode superscript characters.
fn convert_superscripts(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '^' {
            if chars.peek() == Some(&'{') {
                chars.next(); // consume '{'
                let mut content = String::new();
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    content.push(c);
                }
                for c in content.chars() {
                    result.push(char_to_superscript(c));
                }
            } else if let Some(&next) = chars.peek() {
                chars.next();
                result.push(char_to_superscript(next));
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Convert subscript notation (_2, _{10}, etc.) to Unicode subscript characters.
fn convert_subscripts(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '_' {
            if chars.peek() == Some(&'{') {
                chars.next(); // consume '{'
                let mut content = String::new();
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    content.push(c);
                }
                for c in content.chars() {
                    result.push(char_to_subscript(c));
                }
            } else if let Some(&next) = chars.peek() {
                chars.next();
                result.push(char_to_subscript(next));
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Convert a single character to its Unicode superscript equivalent.
fn char_to_superscript(c: char) -> char {
    match c {
        '0' => '\u{2070}',
        '1' => '\u{00B9}',
        '2' => '\u{00B2}',
        '3' => '\u{00B3}',
        '4' => '\u{2074}',
        '5' => '\u{2075}',
        '6' => '\u{2076}',
        '7' => '\u{2077}',
        '8' => '\u{2078}',
        '9' => '\u{2079}',
        '+' => '\u{207A}',
        '-' => '\u{207B}',
        '=' => '\u{207C}',
        '(' => '\u{207D}',
        ')' => '\u{207E}',
        'n' => '\u{207F}',
        'i' => '\u{2071}',
        _ => c,
    }
}

/// Convert a single character to its Unicode subscript equivalent.
fn char_to_subscript(c: char) -> char {
    match c {
        '0' => '\u{2080}',
        '1' => '\u{2081}',
        '2' => '\u{2082}',
        '3' => '\u{2083}',
        '4' => '\u{2084}',
        '5' => '\u{2085}',
        '6' => '\u{2086}',
        '7' => '\u{2087}',
        '8' => '\u{2088}',
        '9' => '\u{2089}',
        '+' => '\u{208A}',
        '-' => '\u{208B}',
        '=' => '\u{208C}',
        '(' => '\u{208D}',
        ')' => '\u{208E}',
        _ => c,
    }
}

/// Extract text from MathML formula element
///
/// # Arguments
/// * `math_node` - The math XML node
///
/// # Returns
/// * `Option<String>` - The extracted formula text
fn extract_mathml_text(math_node: roxmltree::Node) -> Option<String> {
    for node in math_node.descendants() {
        if node.tag_name().name() == "annotation"
            && let Some(encoding) = node.attribute("encoding")
            && encoding.contains("StarMath")
            && let Some(text) = node.text()
        {
            return Some(starmath_to_unicode(text));
        }
    }

    let mut formula_parts = Vec::new();
    for node in math_node.descendants() {
        match node.tag_name().name() {
            "mi" | "mo" | "mn" | "ms" | "mtext" => {
                if let Some(text) = node.text() {
                    formula_parts.push(text.to_string());
                }
            }
            _ => {}
        }
    }

    if !formula_parts.is_empty() {
        Some(formula_parts.join(" "))
    } else {
        None
    }
}

/// Extract text from embedded formula objects
///
/// # Arguments
/// * `archive` - ZIP archive containing the ODT document
///
/// # Returns
/// * `String` - Extracted formula content from embedded objects
fn extract_embedded_formulas(archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>) -> crate::error::Result<String> {
    use std::io::Read;
    let mut formula_parts = Vec::new();

    let file_names: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();

    for file_name in file_names {
        if file_name.contains("Object")
            && file_name.ends_with("content.xml")
            && let Ok(mut file) = archive.by_name(&file_name)
        {
            let mut xml_content = String::new();
            if file.read_to_string(&mut xml_content).is_ok()
                && let Ok(doc) = Document::parse(&xml_content)
            {
                let root = doc.root_element();

                if root.tag_name().name() == "math" {
                    if let Some(formula_text) = extract_mathml_text(root) {
                        formula_parts.push(formula_text);
                    }
                } else {
                    for node in root.descendants() {
                        if node.tag_name().name() == "math"
                            && let Some(formula_text) = extract_mathml_text(node)
                        {
                            formula_parts.push(formula_text);
                        }
                    }
                }
            }
        }
    }

    Ok(formula_parts.join("\n"))
}

/// Extract text content from ODT content.xml
///
/// # Arguments
/// * `archive` - ZIP archive containing the ODT document
///
/// # Returns
/// * `String` - Extracted text content
fn extract_content_text(archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>, plain: bool) -> crate::error::Result<String> {
    let mut xml_content = String::new();

    match archive.by_name("content.xml") {
        Ok(mut file) => {
            use std::io::Read;
            file.read_to_string(&mut xml_content)
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to read content.xml: {}", e)))?;
        }
        Err(_) => {
            return Ok(String::new());
        }
    }

    let doc = Document::parse(&xml_content)
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to parse content.xml: {}", e)))?;

    let root = doc.root_element();

    let mut text_parts: Vec<String> = Vec::new();

    for body_child in root.children() {
        if body_child.tag_name().name() == "body" {
            for text_elem in body_child.children() {
                if text_elem.tag_name().name() == "text" {
                    process_document_elements(text_elem, &mut text_parts, plain);
                }
            }
        }
    }

    Ok(text_parts.join("\n").trim().to_string())
}

/// Helper function to process document elements (paragraphs, headings, tables, lists)
/// Only processes direct children, avoiding nested content like table cells
fn process_document_elements(parent: roxmltree::Node, text_parts: &mut Vec<String>, plain: bool) {
    for node in parent.children() {
        match node.tag_name().name() {
            "h" => {
                if let Some(text) = extract_node_text(node)
                    && !text.trim().is_empty()
                {
                    if plain {
                        text_parts.push(text.trim().to_string());
                    } else {
                        text_parts.push(format!("# {}", text.trim()));
                    }
                    text_parts.push(String::new());
                }
            }
            "p" => {
                if let Some(text) = extract_node_text(node)
                    && !text.trim().is_empty()
                {
                    text_parts.push(text.trim().to_string());
                    text_parts.push(String::new());
                }
            }
            "table" => {
                if let Some(table_text) = extract_table_text(node, plain) {
                    text_parts.push(table_text);
                    text_parts.push(String::new());
                }
            }
            "list" => {
                process_list_elements(node, text_parts, 0, plain);
                text_parts.push(String::new());
            }
            "section" => {
                process_document_elements(node, text_parts, plain);
            }
            _ => {}
        }
    }
}

/// Process list elements recursively, handling nested lists with indentation
fn process_list_elements(list_node: roxmltree::Node, text_parts: &mut Vec<String>, depth: usize, plain: bool) {
    let indent = "  ".repeat(depth);
    for item in list_node.children() {
        if item.tag_name().name() == "list-item" {
            for child in item.children() {
                match child.tag_name().name() {
                    "p" => {
                        if let Some(text) = extract_node_text(child)
                            && !text.trim().is_empty()
                        {
                            if plain {
                                text_parts.push(text.trim().to_string());
                            } else {
                                text_parts.push(format!("{indent}- {}", text.trim()));
                            }
                        }
                    }
                    "h" => {
                        if let Some(text) = extract_node_text(child)
                            && !text.trim().is_empty()
                        {
                            if plain {
                                text_parts.push(text.trim().to_string());
                            } else {
                                text_parts.push(format!("{indent}- # {}", text.trim()));
                            }
                        }
                    }
                    "list" => {
                        process_list_elements(child, text_parts, depth + 1, plain);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Extract text from a single XML node, handling spans and formatting
///
/// # Arguments
/// * `node` - The XML node to extract text from
///
/// # Returns
/// * `Option<String>` - The extracted text with formatting preserved
fn extract_node_text(node: roxmltree::Node) -> Option<String> {
    let mut text_parts = Vec::new();

    for child in node.children() {
        match child.tag_name().name() {
            "span" => {
                if let Some(text) = child.text() {
                    text_parts.push(text.to_string());
                }
            }
            "tab" => {
                text_parts.push("\t".to_string());
            }
            "line-break" => {
                text_parts.push("\n".to_string());
            }
            _ => {
                if let Some(text) = child.text() {
                    text_parts.push(text.to_string());
                }
            }
        }
    }

    if text_parts.is_empty() {
        node.text().map(|s| s.to_string())
    } else {
        Some(text_parts.join(""))
    }
}

/// Extract table content as text with markdown formatting
///
/// # Arguments
/// * `table_node` - The table XML node
///
/// # Returns
/// * `Option<String>` - Markdown formatted table
fn extract_table_text(table_node: roxmltree::Node, plain: bool) -> Option<String> {
    let mut rows = Vec::new();
    let mut max_cols = 0;

    for row_node in table_node.children() {
        if row_node.tag_name().name() == "table-row" {
            let mut row_cells = Vec::new();

            for cell_node in row_node.children() {
                if cell_node.tag_name().name() == "table-cell" {
                    let cell_text = extract_node_text(cell_node).unwrap_or_default();
                    row_cells.push(cell_text.trim().to_string());
                }
            }

            if !row_cells.is_empty() {
                max_cols = max_cols.max(row_cells.len());
                rows.push(row_cells);
            }
        }
    }

    if rows.is_empty() {
        return None;
    }

    for row in &mut rows {
        while row.len() < max_cols {
            row.push(String::new());
        }
    }

    if plain {
        Some(crate::extraction::cells_to_text(&rows))
    } else {
        let mut markdown = String::new();

        if !rows.is_empty() {
            markdown.push('|');
            for cell in &rows[0] {
                markdown.push(' ');
                markdown.push_str(cell);
                markdown.push_str(" |");
            }
            markdown.push('\n');

            markdown.push('|');
            for _ in 0..rows[0].len() {
                markdown.push_str(" --- |");
            }
            markdown.push('\n');

            for row in rows.iter().skip(1) {
                markdown.push('|');
                for cell in row {
                    markdown.push(' ');
                    markdown.push_str(cell);
                    markdown.push_str(" |");
                }
                markdown.push('\n');
            }
        }

        Some(markdown)
    }
}

/// Extract tables from ODT content.xml
///
/// # Arguments
/// * `archive` - ZIP archive containing the ODT document
///
/// # Returns
/// * `Result<Vec<Table>>` - Extracted tables
fn extract_tables(archive: &mut zip::ZipArchive<Cursor<Vec<u8>>>) -> crate::error::Result<Vec<Table>> {
    let mut xml_content = String::new();

    match archive.by_name("content.xml") {
        Ok(mut file) => {
            use std::io::Read;
            file.read_to_string(&mut xml_content)
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to read content.xml: {}", e)))?;
        }
        Err(_) => {
            return Ok(Vec::new());
        }
    }

    let doc = Document::parse(&xml_content)
        .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to parse content.xml: {}", e)))?;

    let root = doc.root_element();
    let mut tables = Vec::new();
    let mut table_index = 0;

    for node in root.descendants() {
        if node.tag_name().name() == "table"
            && let Some(table) = parse_odt_table(node, table_index)
        {
            tables.push(table);
            table_index += 1;
        }
    }

    Ok(tables)
}

/// Parse a single ODT table element into a Table struct
///
/// # Arguments
/// * `table_node` - The table XML node
/// * `table_index` - Index of the table in the document
///
/// # Returns
/// * `Option<Table>` - Parsed table
fn parse_odt_table(table_node: roxmltree::Node, table_index: usize) -> Option<Table> {
    let mut cells: Vec<Vec<String>> = Vec::new();

    for row_node in table_node.children() {
        if row_node.tag_name().name() == "table-row" {
            let mut row_cells = Vec::new();

            for cell_node in row_node.children() {
                if cell_node.tag_name().name() == "table-cell" {
                    let cell_text = extract_node_text(cell_node).unwrap_or_default();
                    row_cells.push(cell_text.trim().to_string());
                }
            }

            if !row_cells.is_empty() {
                cells.push(row_cells);
            }
        }
    }

    if cells.is_empty() {
        return None;
    }

    let markdown = cells_to_markdown(&cells);

    Some(Table {
        cells,
        markdown,
        page_number: table_index + 1,
        bounding_box: None,
    })
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for OdtExtractor {
    #[cfg_attr(
        feature = "otel",
        tracing::instrument(
            skip(self, content, _config),
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
        _config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let content_owned = content.to_vec();
        let plain = matches!(
            _config.output_format,
            crate::core::config::OutputFormat::Plain | crate::core::config::OutputFormat::Structured
        );

        let (text, tables) = {
            #[cfg(feature = "tokio-runtime")]
            if crate::core::batch_mode::is_batch_mode() {
                let content_for_task = content_owned.clone();
                let span = tracing::Span::current();
                tokio::task::spawn_blocking(move || -> crate::error::Result<(String, Vec<Table>)> {
                    let _guard = span.entered();

                    let cursor = Cursor::new(content_for_task);
                    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
                        crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e))
                    })?;

                    let text = extract_content_text(&mut archive, plain)?;
                    let tables = extract_tables(&mut archive)?;
                    let embedded_formulas = extract_embedded_formulas(&mut archive)?;

                    let combined_text = if !embedded_formulas.is_empty() {
                        if !text.is_empty() {
                            format!("{}\n{}", text, embedded_formulas)
                        } else {
                            embedded_formulas
                        }
                    } else {
                        text
                    };

                    Ok((combined_text, tables))
                })
                .await
                .map_err(|e| crate::error::KreuzbergError::parsing(format!("ODT extraction task failed: {}", e)))??
            } else {
                let cursor = Cursor::new(content_owned.clone());
                let mut archive = zip::ZipArchive::new(cursor)
                    .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

                let text = extract_content_text(&mut archive, plain)?;
                let tables = extract_tables(&mut archive)?;
                let embedded_formulas = extract_embedded_formulas(&mut archive)?;

                let combined_text = if !embedded_formulas.is_empty() {
                    if !text.is_empty() {
                        format!("{}\n{}", text, embedded_formulas)
                    } else {
                        embedded_formulas
                    }
                } else {
                    text
                };

                (combined_text, tables)
            }

            #[cfg(not(feature = "tokio-runtime"))]
            {
                let cursor = Cursor::new(content_owned.clone());
                let mut archive = zip::ZipArchive::new(cursor)
                    .map_err(|e| crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive: {}", e)))?;

                let text = extract_content_text(&mut archive, plain)?;
                let tables = extract_tables(&mut archive)?;
                let embedded_formulas = extract_embedded_formulas(&mut archive)?;

                let combined_text = if !embedded_formulas.is_empty() {
                    if !text.is_empty() {
                        format!("{}\n{}", text, embedded_formulas)
                    } else {
                        embedded_formulas
                    }
                } else {
                    text
                };

                (combined_text, tables)
            }
        };

        let mut metadata_map = AHashMap::new();

        let cursor = Cursor::new(content_owned.clone());
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
            crate::error::KreuzbergError::parsing(format!("Failed to open ZIP archive for metadata: {}", e))
        })?;

        if let Ok(odt_props) = office_metadata::extract_odt_properties(&mut archive) {
            if let Some(title) = odt_props.title {
                metadata_map.insert(Cow::Borrowed("title"), serde_json::Value::String(title));
            }
            if let Some(creator) = odt_props.creator {
                metadata_map.insert(
                    Cow::Borrowed("authors"),
                    serde_json::Value::Array(vec![serde_json::Value::String(creator.clone())]),
                );
                metadata_map.insert(Cow::Borrowed("created_by"), serde_json::Value::String(creator));
            }
            if let Some(initial_creator) = odt_props.initial_creator {
                metadata_map.insert(
                    Cow::Borrowed("initial_creator"),
                    serde_json::Value::String(initial_creator),
                );
            }
            if let Some(subject) = odt_props.subject {
                metadata_map.insert(Cow::Borrowed("subject"), serde_json::Value::String(subject));
            }
            if let Some(keywords) = odt_props.keywords {
                metadata_map.insert(Cow::Borrowed("keywords"), serde_json::Value::String(keywords));
            }
            if let Some(description) = odt_props.description {
                metadata_map.insert(Cow::Borrowed("description"), serde_json::Value::String(description));
            }
            if let Some(creation_date) = odt_props.creation_date {
                metadata_map.insert(Cow::Borrowed("created_at"), serde_json::Value::String(creation_date));
            }
            if let Some(date) = odt_props.date {
                metadata_map.insert(Cow::Borrowed("modified_at"), serde_json::Value::String(date));
            }
            if let Some(language) = odt_props.language {
                metadata_map.insert(Cow::Borrowed("language"), serde_json::Value::String(language));
            }
            if let Some(generator) = odt_props.generator {
                metadata_map.insert(Cow::Borrowed("generator"), serde_json::Value::String(generator));
            }
            if let Some(editing_duration) = odt_props.editing_duration {
                metadata_map.insert(
                    Cow::Borrowed("editing_duration"),
                    serde_json::Value::String(editing_duration),
                );
            }
            if let Some(editing_cycles) = odt_props.editing_cycles {
                metadata_map.insert(
                    Cow::Borrowed("editing_cycles"),
                    serde_json::Value::String(editing_cycles),
                );
            }
            if let Some(page_count) = odt_props.page_count {
                metadata_map.insert(
                    Cow::Borrowed("page_count"),
                    serde_json::Value::Number(page_count.into()),
                );
            }
            if let Some(word_count) = odt_props.word_count {
                metadata_map.insert(
                    Cow::Borrowed("word_count"),
                    serde_json::Value::Number(word_count.into()),
                );
            }
            if let Some(character_count) = odt_props.character_count {
                metadata_map.insert(
                    Cow::Borrowed("character_count"),
                    serde_json::Value::Number(character_count.into()),
                );
            }
            if let Some(paragraph_count) = odt_props.paragraph_count {
                metadata_map.insert(
                    Cow::Borrowed("paragraph_count"),
                    serde_json::Value::Number(paragraph_count.into()),
                );
            }
            if let Some(table_count) = odt_props.table_count {
                metadata_map.insert(
                    Cow::Borrowed("table_count"),
                    serde_json::Value::Number(table_count.into()),
                );
            }
            if let Some(image_count) = odt_props.image_count {
                metadata_map.insert(
                    Cow::Borrowed("image_count"),
                    serde_json::Value::Number(image_count.into()),
                );
            }
        }

        Ok(ExtractionResult {
            content: text,
            mime_type: mime_type.to_string().into(),
            metadata: Metadata {
                additional: metadata_map,
                ..Default::default()
            },
            pages: None,
            tables,
            detected_languages: None,
            chunks: None,
            images: None,
            djot_content: None,
            elements: None,
            ocr_elements: None,
            document: None,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: Vec::new(),
            annotations: None,
        })
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/vnd.oasis.opendocument.text"]
    }

    fn priority(&self) -> i32 {
        60
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_odt_extractor_plugin_interface() {
        let extractor = OdtExtractor::new();
        assert_eq!(extractor.name(), "odt-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 60);
        assert_eq!(extractor.supported_mime_types().len(), 1);
    }

    #[tokio::test]
    async fn test_odt_extractor_supports_odt() {
        let extractor = OdtExtractor::new();
        assert!(
            extractor
                .supported_mime_types()
                .contains(&"application/vnd.oasis.opendocument.text")
        );
    }

    #[tokio::test]
    async fn test_odt_extractor_default() {
        let extractor = OdtExtractor;
        assert_eq!(extractor.name(), "odt-extractor");
    }

    #[tokio::test]
    async fn test_odt_extractor_initialize_shutdown() {
        let extractor = OdtExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_extract_node_text_simple() {
        let xml = r#"<p xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">Hello world</p>"#;
        let doc = roxmltree::Document::parse(xml).unwrap();
        let node = doc.root_element();

        let result = extract_node_text(node);
        assert!(result.is_some());
        assert!(!result.unwrap().is_empty());
    }
}
