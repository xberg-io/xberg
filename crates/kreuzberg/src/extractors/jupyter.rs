//! Jupyter Notebook extractor for .ipynb files.
//!
//! This extractor provides native Rust parsing of Jupyter notebooks,
//! extracting:
//! - Notebook metadata (kernelspec, language_info, nbformat)
//! - Cell content (markdown and code cells in order)
//! - Cell outputs (text, HTML, images)
//! - Cell metadata (execution_count, tags)
//!
//! Requires the `office` feature.

#[cfg(feature = "office")]
use crate::Result;
#[cfg(feature = "office")]
use crate::core::config::ExtractionConfig;
#[cfg(feature = "office")]
use crate::extractors::security::SecurityBudget;
#[cfg(feature = "office")]
use crate::plugins::{DocumentExtractor, Plugin};
#[cfg(feature = "office")]
use crate::types::internal::InternalDocument;
#[cfg(feature = "office")]
use crate::types::internal_builder::InternalDocumentBuilder;
#[cfg(feature = "office")]
use crate::types::uri::Uri;
#[cfg(feature = "office")]
use crate::types::{ExtractedImage, Metadata};
#[cfg(feature = "office")]
use ahash::AHashMap;
#[cfg(feature = "office")]
use async_trait::async_trait;
#[cfg(feature = "office")]
use base64::Engine;
#[cfg(feature = "office")]
use bytes::Bytes;
#[cfg(feature = "office")]
use serde_json::{Value, json};
#[cfg(feature = "office")]
use std::borrow::Cow;

#[cfg(feature = "office")]
type NotebookContent = (String, AHashMap<Cow<'static, str>, Value>, Vec<ExtractedImage>, Value);

/// Jupyter Notebook extractor.
///
/// Extracts content from Jupyter notebook JSON files, including:
/// - Notebook metadata (kernel, language, nbformat version)
/// - Cell content (code and markdown)
/// - Cell outputs (text, HTML, etc.)
/// - Cell-level metadata (tags, execution counts)
#[cfg(feature = "office")]
pub struct JupyterExtractor;

#[cfg(feature = "office")]
impl JupyterExtractor {
    /// Create a new Jupyter extractor.
    pub(crate) fn new() -> Self {
        Self
    }

    /// Extract content from a Jupyter notebook.
    fn extract_notebook(content: &[u8], plain: bool) -> Result<NotebookContent> {
        let notebook: Value = serde_json::from_slice(content)
            .map_err(|e| crate::KreuzbergError::parsing(format!("Failed to parse JSON: {}", e)))?;

        let mut extracted_content = String::new();
        let mut metadata = AHashMap::new();
        let mut images = Vec::new();

        if let Some(notebook_metadata) = notebook.get("metadata").and_then(|m| m.as_object()) {
            if let Some(kernelspec) = notebook_metadata.get("kernelspec") {
                metadata.insert(Cow::Borrowed("kernelspec"), kernelspec.clone());
            }

            if let Some(language_info) = notebook_metadata.get("language_info") {
                // Store the full language_info object
                metadata.insert(Cow::Borrowed("language_info"), language_info.clone());

                // Extract individual fields for convenience
                if let Some(obj) = language_info.as_object() {
                    if let Some(name) = obj.get("name") {
                        metadata.insert(Cow::Borrowed("language_name"), name.clone());
                    }
                    if let Some(version) = obj.get("version") {
                        metadata.insert(Cow::Borrowed("language_version"), version.clone());
                    }
                    if let Some(mimetype) = obj.get("mimetype") {
                        metadata.insert(Cow::Borrowed("language_mimetype"), mimetype.clone());
                    }
                }
            }
        }

        if let Some(nbformat) = notebook.get("nbformat") {
            metadata.insert(Cow::Borrowed("nbformat"), nbformat.clone());
        }
        if let Some(nbformat_minor) = notebook.get("nbformat_minor") {
            metadata.insert(Cow::Borrowed("nbformat_minor"), nbformat_minor.clone());
        }

        // Count cells by type
        if let Some(cells) = notebook.get("cells").and_then(|c| c.as_array()) {
            metadata.insert(Cow::Borrowed("cell_count"), json!(cells.len()));
        }

        if let Some(cells) = notebook.get("cells").and_then(|c| c.as_array()) {
            let mut cells_meta: Vec<Value> = Vec::with_capacity(cells.len());
            for (cell_idx, cell) in cells.iter().enumerate() {
                let cell_type = cell.get("cell_type").and_then(|t| t.as_str()).unwrap_or("unknown");
                let mut cell_entry = serde_json::Map::new();
                cell_entry.insert("index".into(), json!(cell_idx));
                cell_entry.insert("cell_type".into(), json!(cell_type));

                if cell_type == "code"
                    && let Some(exec_count) = cell.get("execution_count")
                {
                    cell_entry.insert("execution_count".into(), exec_count.clone());
                }
                if let Some(tags) = cell
                    .get("metadata")
                    .and_then(|m| m.get("tags"))
                    .and_then(|t| t.as_array())
                    && !tags.is_empty()
                {
                    cell_entry.insert("tags".into(), Value::Array(tags.clone()));
                }
                cells_meta.push(Value::Object(cell_entry));

                Self::extract_cell(cell, cell_idx, &mut extracted_content, &mut images, plain)?;
            }
            metadata.insert(Cow::Borrowed("cells"), json!(cells_meta));
        }

        Ok((extracted_content, metadata, images, notebook))
    }

    /// Extract content from a single cell.
    fn extract_cell(
        cell: &Value,
        cell_idx: usize,
        content: &mut String,
        images: &mut Vec<ExtractedImage>,
        plain: bool,
    ) -> Result<()> {
        let cell_type = cell.get("cell_type").and_then(|t| t.as_str()).unwrap_or("unknown");

        match cell_type {
            "markdown" => Self::extract_markdown_cell(cell, content)?,
            "code" => Self::extract_code_cell(cell, cell_idx, content, images, plain)?,
            "raw" => Self::extract_raw_cell(cell, content)?,
            _ => {}
        }

        // Separate cells with a blank line
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push('\n');
        Ok(())
    }

    /// Extract markdown cell content.
    fn extract_markdown_cell(cell: &Value, content: &mut String) -> Result<()> {
        if let Some(source) = cell.get("source") {
            let cell_text = Self::extract_source(source);
            content.push_str(&cell_text);
        }
        Ok(())
    }

    /// Extract code cell content and outputs.
    fn extract_code_cell(
        cell: &Value,
        cell_idx: usize,
        content: &mut String,
        images: &mut Vec<ExtractedImage>,
        plain: bool,
    ) -> Result<()> {
        if let Some(source) = cell.get("source") {
            let cell_text = Self::extract_source(source);
            content.push_str(&cell_text);
            if !cell_text.ends_with('\n') {
                content.push('\n');
            }
        }

        if let Some(outputs) = cell.get("outputs").and_then(|o| o.as_array()) {
            for output in outputs {
                Self::extract_output(output, cell_idx, content, images, plain)?;
            }
        }

        Ok(())
    }

    /// Extract raw cell content.
    fn extract_raw_cell(cell: &Value, content: &mut String) -> Result<()> {
        if let Some(source) = cell.get("source") {
            let cell_text = Self::extract_source(source);
            content.push_str(&cell_text);
        }
        Ok(())
    }

    /// Extract source content from various formats.
    ///
    /// Source can be either a string or an array of strings.
    fn extract_source(source: &Value) -> String {
        match source {
            Value::String(s) => s.clone(),
            Value::Array(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<String>(),
            _ => String::new(),
        }
    }

    /// Extract output from a cell.
    fn extract_output(
        output: &Value,
        cell_idx: usize,
        content: &mut String,
        images: &mut Vec<ExtractedImage>,
        plain: bool,
    ) -> Result<()> {
        let output_type = output.get("output_type").and_then(|t| t.as_str()).unwrap_or("unknown");

        match output_type {
            "stream" => Self::extract_stream_output(output, content)?,
            "execute_result" | "display_data" => {
                Self::extract_data_output(output, cell_idx, content, images, plain)?;
            }
            "error" => Self::extract_error_output(output, content)?,
            _ => {}
        }

        Ok(())
    }

    /// Extract stream output (stdout, stderr).
    fn extract_stream_output(output: &Value, content: &mut String) -> Result<()> {
        if let Some(text) = output.get("text") {
            let text_content = Self::extract_source(text);
            content.push_str(&text_content);
        }

        Ok(())
    }

    /// Extract data output (execute_result or display_data).
    ///
    /// Prioritizes text/plain for quality scoring. For raster image types,
    /// decodes base64 data and populates the images collection.
    fn extract_data_output(
        output: &Value,
        cell_idx: usize,
        content: &mut String,
        images: &mut Vec<ExtractedImage>,
        plain_mode: bool,
    ) -> Result<()> {
        if let Some(data) = output.get("data").and_then(|d| d.as_object()) {
            // Prefer text/plain first - it has the most readable tokens for quality scoring
            if let Some(plain) = data.get("text/plain") {
                let text = Self::extract_source(plain);
                if !text.is_empty() {
                    content.push_str(&text);
                    if !text.ends_with('\n') {
                        content.push('\n');
                    }
                }
            }

            // Also include markdown/HTML content — these often contain richer
            // semantic information than text/plain (e.g. descriptive fallback text).
            // Skip these for plain text output mode.
            if !plain_mode {
                for mime_type in &["text/markdown", "text/html"] {
                    if let Some(mime_content) = data.get(*mime_type) {
                        let mime_text = Self::extract_source(mime_content);
                        if !mime_text.is_empty() {
                            content.push_str(&mime_text);
                            if !mime_text.ends_with('\n') {
                                content.push('\n');
                            }
                        }
                    }
                }
            }

            // For raster image types, extract actual base64-encoded image data
            for mime_type in &["image/png", "image/jpeg", "image/gif", "image/webp"] {
                if let Some(image_value) = data.get(*mime_type) {
                    let base64_str = Self::extract_source(image_value);
                    let cleaned = base64_str.replace(['\n', '\r'], "");
                    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&cleaned) {
                        let format = match *mime_type {
                            "image/png" => "png",
                            "image/jpeg" => "jpeg",
                            "image/gif" => "gif",
                            "image/webp" => "webp",
                            _ => "unknown",
                        };

                        // Classify image based on metadata and visual properties
                        let (image_kind, kind_confidence) =
                            crate::extraction::image_kind::classify(&decoded, format, None, None, None, None, false);

                        images.push(ExtractedImage {
                            data: Bytes::from(decoded),
                            format: Cow::Borrowed(format),
                            image_index: images.len(),
                            page_number: Some(cell_idx + 1),
                            width: None,
                            height: None,
                            colorspace: None,
                            bits_per_component: None,
                            is_mask: false,
                            description: Some(format!("Notebook cell {} output", cell_idx)),
                            ocr_result: None,
                            bounding_box: None,
                            source_path: None,
                            image_kind: Some(image_kind),
                            kind_confidence: Some(kind_confidence),
                            cluster_id: None,
                        });
                        content.push_str(&format!("[Image: {}]\n", mime_type));
                    }
                }
            }

            // Handle SVG as text (not a raster image for OCR)
            if data.contains_key("image/svg+xml") {
                content.push_str("[Image: image/svg+xml]\n");
            }

            // Include JSON output as structured data
            if let Some(json_content) = data.get("application/json")
                && let Ok(formatted) = serde_json::to_string_pretty(json_content)
            {
                content.push_str(&formatted);
                content.push('\n');
            }
        }

        Ok(())
    }

    /// Scan markdown text for inline formatting patterns and produce
    /// stripped text with annotations.
    ///
    /// Recognizes: `**bold**`, `*italic*`, `` `code` ``
    fn scan_markdown_inline(text: &str) -> (String, Vec<crate::types::TextAnnotation>) {
        use crate::types::TextAnnotation;
        use crate::types::document_structure::AnnotationKind;

        let mut out = String::with_capacity(text.len());
        let mut annotations = Vec::new();
        let bytes = text.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'*' {
                // Bold: **...**
                if let Some(end) = Self::find_closing(bytes, i + 2, b"**") {
                    let inner = &text[i + 2..end];
                    let start = out.len() as u32;
                    out.push_str(inner);
                    let ann_end = out.len() as u32;
                    annotations.push(TextAnnotation {
                        start,
                        end: ann_end,
                        kind: AnnotationKind::Bold,
                    });
                    i = end + 2;
                    continue;
                }
            }

            if bytes[i] == b'*' && (i == 0 || bytes[i - 1] != b'*') {
                // Italic: *...*  (but not **)
                if i + 1 < len
                    && bytes[i + 1] != b'*'
                    && let Some(end) = Self::find_closing_single_star(bytes, i + 1)
                {
                    let inner = &text[i + 1..end];
                    let start = out.len() as u32;
                    out.push_str(inner);
                    let ann_end = out.len() as u32;
                    annotations.push(TextAnnotation {
                        start,
                        end: ann_end,
                        kind: AnnotationKind::Italic,
                    });
                    i = end + 1;
                    continue;
                }
            }

            if bytes[i] == b'`' && (i + 1 >= len || bytes[i + 1] != b'`') {
                // Inline code: `...`
                if let Some(end) = Self::find_closing_byte(bytes, i + 1, b'`') {
                    let inner = &text[i + 1..end];
                    let start = out.len() as u32;
                    out.push_str(inner);
                    let ann_end = out.len() as u32;
                    annotations.push(TextAnnotation {
                        start,
                        end: ann_end,
                        kind: AnnotationKind::Code,
                    });
                    i = end + 1;
                    continue;
                }
            }

            out.push(bytes[i] as char);
            i += 1;
        }

        (out, annotations)
    }

    /// Find position of a two-byte closing delimiter (e.g. `**`).
    fn find_closing(bytes: &[u8], start: usize, delim: &[u8; 2]) -> Option<usize> {
        let mut i = start;
        while i + 1 < bytes.len() {
            if bytes[i] == delim[0] && bytes[i + 1] == delim[1] {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    /// Find position of a closing single `*` that is not followed by another `*`.
    fn find_closing_single_star(bytes: &[u8], start: usize) -> Option<usize> {
        let mut i = start;
        while i < bytes.len() {
            if bytes[i] == b'*' && (i + 1 >= bytes.len() || bytes[i + 1] != b'*') {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    /// Find position of a closing single byte delimiter.
    fn find_closing_byte(bytes: &[u8], start: usize, delim: u8) -> Option<usize> {
        bytes[start..].iter().position(|&b| b == delim).map(|p| start + p)
    }

    /// Extract markdown-style links from text and return as URIs.
    ///
    /// Uses pulldown-cmark for robust parsing instead of hand-rolled byte scanning.
    /// Recognizes both `[text](url)` hyperlinks and `![alt](url)` image links.
    fn extract_markdown_links(text: &str) -> Vec<Uri> {
        use pulldown_cmark::{Event, Parser, Tag, TagEnd};

        let parser = Parser::new(text);
        let mut uris = Vec::new();
        let mut current_text = String::new();
        let mut current_url: Option<String> = None;
        let mut in_link = false;
        let mut in_image = false;

        for event in parser {
            match event {
                Event::Start(Tag::Link { dest_url, .. }) => {
                    in_link = true;
                    current_text.clear();
                    current_url = Some(dest_url.into_string());
                }
                Event::Start(Tag::Image { dest_url, .. }) => {
                    in_image = true;
                    current_text.clear();
                    current_url = Some(dest_url.into_string());
                }
                Event::Text(text) if in_link || in_image => {
                    current_text.push_str(&text);
                }
                Event::End(TagEnd::Link) => {
                    if let Some(url) = current_url.take()
                        && !url.is_empty()
                    {
                        let label_opt = if current_text.is_empty() {
                            None
                        } else {
                            Some(current_text.clone())
                        };
                        uris.push(Uri::hyperlink(&url, label_opt));
                    }
                    in_link = false;
                    current_text.clear();
                }
                Event::End(TagEnd::Image) => {
                    if let Some(url) = current_url.take()
                        && !url.is_empty()
                    {
                        let label_opt = if current_text.is_empty() {
                            None
                        } else {
                            Some(current_text.clone())
                        };
                        uris.push(Uri::image(&url, label_opt));
                    }
                    in_image = false;
                    current_text.clear();
                }
                _ => {}
            }
        }

        uris
    }

    /// Detect an ATX heading line (`# …`, `## …`, etc.) and return level + text.
    fn parse_heading_line(line: &str) -> Option<(u8, &str)> {
        let trimmed = line.trim_start();
        let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
        if hashes == 0 || hashes > 6 {
            return None;
        }
        let rest = &trimmed[hashes..];
        // ATX heading requires a space (or end-of-line) after the hashes
        if !rest.is_empty() && !rest.starts_with(' ') {
            return None;
        }
        Some((hashes as u8, rest.trim()))
    }

    /// Collect `text/plain` content from a single notebook output object.
    fn collect_output_text(output: &Value) -> String {
        let mut text = String::new();

        let output_type = output.get("output_type").and_then(|t| t.as_str()).unwrap_or("");

        match output_type {
            "stream" => {
                if let Some(t) = output.get("text") {
                    text.push_str(&Self::extract_source(t));
                }
            }
            "execute_result" | "display_data" => {
                if let Some(data) = output.get("data").and_then(|d| d.as_object())
                    && let Some(plain) = data.get("text/plain")
                {
                    text.push_str(&Self::extract_source(plain));
                }
            }
            "error" => {
                let ename = output.get("ename").and_then(|e| e.as_str()).unwrap_or("Unknown");
                let evalue = output.get("evalue").and_then(|e| e.as_str()).unwrap_or("");
                text.push_str(&format!("Error ({}): {}", ename, evalue));
            }
            _ => {}
        }

        text
    }

    /// Build an `InternalDocument` from the already-parsed notebook JSON.
    ///
    /// Markdown cells are split into headings and paragraphs. Code cells
    /// become code blocks followed by any output paragraphs.
    fn build_internal_document(notebook: &Value) -> Option<InternalDocument> {
        let cells = notebook.get("cells")?.as_array()?;

        let kernel_lang = notebook
            .get("metadata")
            .and_then(|m| m.get("kernelspec"))
            .and_then(|k| k.get("language"))
            .and_then(|l| l.as_str())
            .or_else(|| {
                notebook
                    .get("metadata")
                    .and_then(|m| m.get("language_info"))
                    .and_then(|l| l.get("name"))
                    .and_then(|n| n.as_str())
            });

        let mut builder = InternalDocumentBuilder::new("jupyter");

        // Emit kernel language at start of document
        if let Some(lang) = kernel_lang {
            builder.push_paragraph(&format!("[kernel_language: {}]", lang), vec![], None, None);
        }

        for cell in cells {
            let cell_type = cell.get("cell_type").and_then(|t| t.as_str()).unwrap_or("unknown");
            let source_text = Self::extract_source(cell.get("source").unwrap_or(&Value::Null));
            let trimmed = source_text.trim();

            // Emit cell ID and type markers at start of cell
            if let Some(cell_id) = cell.get("id").and_then(|id| id.as_str()) {
                builder.push_paragraph(&format!("[cell_id: {}]", cell_id), vec![], None, None);
            }

            // Emit tags if present
            if let Some(tags) = cell
                .get("metadata")
                .and_then(|m| m.get("tags"))
                .and_then(|t| t.as_array())
                && !tags.is_empty()
            {
                let tag_strs: Vec<String> = tags.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
                if !tag_strs.is_empty() {
                    builder.push_paragraph(&format!("[tags: {}]", tag_strs.join(",")), vec![], None, None);
                }
            }

            if trimmed.is_empty() {
                continue;
            }

            match cell_type {
                "markdown" => {
                    // Extract markdown links as URIs
                    let link_uris = Self::extract_markdown_links(trimmed);
                    for uri in link_uris {
                        builder.push_uri(uri);
                    }

                    // Parse line-by-line: headings become push_heading, other
                    // lines are accumulated and flushed as paragraphs.
                    let mut para_buf = String::new();
                    for line in trimmed.lines() {
                        if let Some((level, heading_text)) = Self::parse_heading_line(line) {
                            // Flush accumulated paragraph text first
                            let flushed = para_buf.trim();
                            if !flushed.is_empty() {
                                let (stripped, annotations) = Self::scan_markdown_inline(flushed);
                                builder.push_paragraph(&stripped, annotations, None, None);
                            }
                            para_buf.clear();

                            if !heading_text.is_empty() {
                                builder.push_heading(level, heading_text, None, None);
                            }
                        } else {
                            if !para_buf.is_empty() {
                                para_buf.push('\n');
                            }
                            para_buf.push_str(line);
                        }
                    }
                    // Flush remaining paragraph text
                    let flushed = para_buf.trim();
                    if !flushed.is_empty() {
                        let (stripped, annotations) = Self::scan_markdown_inline(flushed);
                        builder.push_paragraph(&stripped, annotations, None, None);
                    }
                }
                "code" => {
                    let idx = builder.push_code(trimmed, kernel_lang, None, None);
                    // Store execution_count and tags as element attributes
                    let mut attrs = AHashMap::new();
                    if let Some(exec_count) = cell.get("execution_count") {
                        match exec_count {
                            Value::Number(n) => {
                                attrs.insert("execution_count".to_string(), n.to_string());
                            }
                            Value::Null => {
                                attrs.insert("execution_count".to_string(), "null".to_string());
                            }
                            _ => {}
                        }
                    }
                    if let Some(tags) = cell
                        .get("metadata")
                        .and_then(|m| m.get("tags"))
                        .and_then(|t| t.as_array())
                        && !tags.is_empty()
                    {
                        let tag_strs: Vec<&str> = tags.iter().filter_map(|v| v.as_str()).collect();
                        attrs.insert("tags".to_string(), tag_strs.join(","));
                    }
                    if !attrs.is_empty() {
                        builder.set_attributes(idx, attrs);
                    }

                    // Emit execution_count metadata as paragraph
                    if let Some(exec_count) = cell.get("execution_count") {
                        match exec_count {
                            Value::Number(n) => {
                                builder.push_paragraph(&format!("execution_count: {}", n), vec![], None, None);
                            }
                            Value::Null => {
                                builder.push_paragraph("execution_count: null", vec![], None, None);
                            }
                            _ => {}
                        }
                    }

                    // Emit cell outputs as paragraphs with type markers
                    if let Some(outputs) = cell.get("outputs").and_then(|o| o.as_array()) {
                        for output in outputs {
                            let output_type = output.get("output_type").and_then(|t| t.as_str()).unwrap_or("unknown");

                            // Emit output type marker
                            builder.push_paragraph(&format!("[output_type: {}]", output_type), vec![], None, None);

                            // Emit MIME type markers if present
                            if let Some(data) = output.get("data").and_then(|d| d.as_object()) {
                                for mime_type in data.keys() {
                                    builder.push_paragraph(&format!("[mime: {}]", mime_type), vec![], None, None);
                                }
                            }

                            let output_text = Self::collect_output_text(output);
                            let output_trimmed = output_text.trim();
                            if !output_trimmed.is_empty() {
                                builder.push_paragraph(output_trimmed, vec![], None, None);
                            }
                        }
                    }
                }
                _ => {
                    builder.push_paragraph(trimmed, vec![], None, None);
                }
            }
        }

        Some(builder.build())
    }

    /// Extract error output, preserving ename, evalue, and traceback in content.
    fn extract_error_output(output: &Value, content: &mut String) -> Result<()> {
        let ename = output.get("ename").and_then(|e| e.as_str()).unwrap_or("Unknown");
        let evalue = output.get("evalue").and_then(|e| e.as_str()).unwrap_or("");

        content.push_str(&format!("Error ({}): {}\n", ename, evalue));

        if let Some(traceback) = output.get("traceback").and_then(|t| t.as_array()) {
            content.push_str("Traceback:\n");
            for line in traceback {
                if let Some(line_str) = line.as_str() {
                    content.push_str(line_str);
                    content.push('\n');
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "office")]
impl Default for JupyterExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "office")]
impl Plugin for JupyterExtractor {
    fn name(&self) -> &str {
        "jupyter-extractor"
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
        "Extracts content from Jupyter notebooks (.ipynb files)"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg(feature = "office")]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for JupyterExtractor {
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
        let mut budget = SecurityBudget::from_config(config);
        budget.account_text(content.len())?;
        let plain = matches!(
            config.output_format,
            crate::core::config::OutputFormat::Plain | crate::core::config::OutputFormat::Structured
        );
        let (_extracted_content, additional_metadata, extracted_images, notebook_json) =
            Self::extract_notebook(content, plain)?;

        let mut metadata_additional = AHashMap::new();
        // Extract language name for the standard Metadata.language field
        let meta_language = additional_metadata
            .get(&Cow::Borrowed("language_name"))
            .and_then(|v| v.as_str().map(|s| s.to_string()));
        for (key, value) in additional_metadata {
            metadata_additional.insert(key, json!(value));
        }

        let images = extracted_images;

        // Build InternalDocument from already-parsed notebook (no re-parse)
        let mut doc = Self::build_internal_document(&notebook_json)
            .unwrap_or_else(|| InternalDocumentBuilder::new("jupyter").build());
        doc.mime_type = Cow::Owned(mime_type.to_string());

        doc.metadata = Metadata {
            language: meta_language,
            additional: metadata_additional,
            ..Default::default()
        };
        doc.images = images;

        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &["application/x-ipynb+json"]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jupyter_extractor_plugin_interface() {
        let extractor = JupyterExtractor::new();
        assert_eq!(extractor.name(), "jupyter-extractor");
        assert_eq!(extractor.version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(extractor.priority(), 50);
        assert!(extractor.supported_mime_types().contains(&"application/x-ipynb+json"));
    }

    #[test]
    fn test_extract_execution_count_and_tags() {
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "source": ["print('hello')"],
                    "execution_count": 5,
                    "outputs": [],
                    "metadata": {"tags": ["test-tag", "important"]}
                }
            ],
            "metadata": {
                "kernelspec": {"name": "python3", "language": "python"},
                "language_info": {"name": "python", "version": "3.10.0", "mimetype": "text/x-python"}
            },
            "nbformat": 4,
            "nbformat_minor": 5
        }"#;

        let (_, metadata, _, _) = JupyterExtractor::extract_notebook(notebook_json.as_bytes(), false).unwrap();

        // Check cells array metadata
        let cells = metadata.get(&Cow::Borrowed("cells"));
        assert!(cells.is_some(), "Should have cells metadata array");
        let cells_arr = cells.unwrap().as_array().expect("cells should be an array");
        assert_eq!(cells_arr.len(), 1);
        let cell0 = &cells_arr[0];
        assert_eq!(cell0["index"], json!(0));
        assert_eq!(cell0["cell_type"], json!("code"));
        assert_eq!(cell0["execution_count"], json!(5));
        assert_eq!(cell0["tags"], json!(["test-tag", "important"]));

        // Check cell_count
        assert_eq!(metadata.get(&Cow::Borrowed("cell_count")), Some(&json!(1)));

        // Check language_info fields
        assert_eq!(metadata.get(&Cow::Borrowed("language_name")), Some(&json!("python")));
        assert_eq!(metadata.get(&Cow::Borrowed("language_version")), Some(&json!("3.10.0")));
        assert_eq!(
            metadata.get(&Cow::Borrowed("language_mimetype")),
            Some(&json!("text/x-python"))
        );

        // Check nbformat_minor
        assert_eq!(metadata.get(&Cow::Borrowed("nbformat_minor")), Some(&json!(5)));
    }

    #[test]
    fn test_extract_error_output_content() {
        let notebook_json = r#"{
            "cells": [
                {
                    "cell_type": "code",
                    "source": ["1/0"],
                    "execution_count": 1,
                    "outputs": [
                        {
                            "output_type": "error",
                            "ename": "ZeroDivisionError",
                            "evalue": "division by zero",
                            "traceback": ["Traceback line 1", "Traceback line 2"]
                        }
                    ],
                    "metadata": {}
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 0
        }"#;

        let (content, _, _, _) = JupyterExtractor::extract_notebook(notebook_json.as_bytes(), false).unwrap();

        assert!(
            content.contains("Error (ZeroDivisionError): division by zero"),
            "Should contain error name and value"
        );
        assert!(content.contains("Traceback:"), "Should contain traceback header");
        assert!(content.contains("Traceback line 1"), "Should contain traceback lines");
    }
}
