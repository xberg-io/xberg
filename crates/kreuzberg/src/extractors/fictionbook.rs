//! FictionBook (FB2) document extractor supporting FictionBook 2.0 format.
//!
//! This extractor handles FictionBook XML documents (FB2), an XML-based e-book format
//! popular in Russian-speaking countries.
//!
//! It extracts:
//! - Document metadata (genre, language)
//! - Section hierarchy and content
//! - Paragraphs and text content with inline formatting
//! - Inline markup: emphasis, strong, strikethrough, subscript, superscript, code
//! - Blockquotes and notes
//! - Embedded images from `<binary>` elements (base64-encoded)
//! - Hyperlinks from `<a>` elements

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::extraction::cells_to_markdown;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::internal::InternalDocument;
use crate::types::internal_builder::InternalDocumentBuilder;
use crate::types::uri::Uri;
use crate::types::{ExtractedImage, Metadata, Table};
use async_trait::async_trait;
use base64::Engine;
use bytes::Bytes;
use quick_xml::Reader;
use quick_xml::events::Event;

/// Resolve an XML entity reference name to its character(s).
fn resolve_entity(name: &str) -> Option<&'static str> {
    match name {
        "amp" => Some("&"),
        "lt" => Some("<"),
        "gt" => Some(">"),
        "quot" => Some("\""),
        "apos" => Some("'"),
        "nbsp" => Some("\u{00A0}"),
        _ if name.starts_with('#') => None, // char refs handled separately
        _ => None,
    }
}

/// Resolve an XML general reference (entity or char ref) to a string.
fn resolve_general_ref(ref_bytes: &[u8]) -> String {
    let name = String::from_utf8_lossy(ref_bytes);
    if let Some(entity) = resolve_entity(&name) {
        return entity.to_string();
    }
    if let Some(num) = name.strip_prefix('#') {
        let code = if let Some(hex) = num.strip_prefix('x') {
            u32::from_str_radix(hex, 16).ok()
        } else {
            num.parse::<u32>().ok()
        };
        if let Some(ch) = code.and_then(char::from_u32) {
            return ch.to_string();
        }
    }
    String::new()
}

/// FictionBook document extractor.
///
/// Supports FictionBook 2.0 format with proper section hierarchy and inline formatting.
pub struct FictionBookExtractor;

impl Default for FictionBookExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FictionBookExtractor {
    pub fn new() -> Self {
        Self
    }

    /// Extract text content from a FictionBook element and its children.
    fn extract_text_content(reader: &mut Reader<&[u8]>) -> Result<String> {
        let mut text = String::new();
        let mut depth = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    match tag.as_ref() {
                        "emphasis" | "strong" | "strikethrough" | "code" | "sub" | "sup" => {}
                        "empty-line" => {
                            text.push('\n');
                        }
                        _ => {}
                    }
                    depth += 1;
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    if (tag == "p" || tag == "cite" || tag == "section") && !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                }
                Ok(Event::Text(t)) => {
                    let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                    let had_trailing_space = decoded.ends_with(char::is_whitespace);
                    let normalized = crate::utils::normalize_whitespace(&decoded);
                    let trimmed: &str = normalized.as_ref();
                    if !trimmed.is_empty() {
                        let starts_with_punct = trimmed.starts_with(['.', ',', ';', ':', '!', '?', ')', ']', '[']);
                        if !text.is_empty() && !text.ends_with(' ') && !text.ends_with('\n') && !starts_with_punct {
                            text.push(' ');
                        }
                        text.push_str(trimmed);
                        if had_trailing_space {
                            text.push(' ');
                        }
                    }
                }
                Ok(Event::CData(t)) => {
                    let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                    if !decoded.trim().is_empty() {
                        if !text.is_empty() && !text.ends_with('\n') {
                            text.push('\n');
                        }
                        text.push_str(&decoded);
                        text.push('\n');
                    }
                }
                Ok(Event::GeneralRef(r)) => {
                    let resolved = resolve_general_ref(r.as_ref());
                    if !resolved.is_empty() {
                        text.push_str(&resolved);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(crate::error::KreuzbergError::parsing(format!(
                        "XML parsing error: {}",
                        e
                    )));
                }
                _ => {}
            }
        }

        let text = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }

    /// Extract metadata from FictionBook document.
    fn extract_metadata(data: &[u8]) -> Result<Metadata> {
        let mut reader = Reader::from_reader(data);
        let mut metadata = Metadata::default();
        let mut additional = ahash::AHashMap::new();
        let mut in_title_info = false;
        let mut in_description = false;
        let mut in_author = false;
        let mut in_annotation = false;
        let mut genres: Vec<String> = Vec::new();
        let mut sequences: Vec<String> = Vec::new();
        let mut authors: Vec<String> = Vec::new();
        let mut annotation_text = String::new();

        // Author name parts
        let mut first_name = String::new();
        let mut middle_name = String::new();
        let mut last_name = String::new();
        let mut nickname = String::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());

                    match tag.as_ref() {
                        "description" => {
                            in_description = true;
                        }
                        "title-info" if in_description => {
                            in_title_info = true;
                        }
                        "author" if in_title_info => {
                            in_author = true;
                            first_name.clear();
                            middle_name.clear();
                            last_name.clear();
                            nickname.clear();
                        }
                        "first-name" if in_author => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                first_name = String::from_utf8_lossy(t.as_ref()).trim().to_string();
                            }
                        }
                        "middle-name" if in_author => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                middle_name = String::from_utf8_lossy(t.as_ref()).trim().to_string();
                            }
                        }
                        "last-name" if in_author => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                last_name = String::from_utf8_lossy(t.as_ref()).trim().to_string();
                            }
                        }
                        "nickname" if in_author => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                nickname = String::from_utf8_lossy(t.as_ref()).trim().to_string();
                            }
                        }
                        "annotation" if in_title_info => {
                            in_annotation = true;
                            annotation_text.clear();
                        }
                        "genre" if in_title_info => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                let genre = String::from_utf8_lossy(t.as_ref());
                                if !genre.trim().is_empty() && genre.trim() != "unrecognised" {
                                    genres.push(genre.trim().to_string());
                                }
                            }
                        }
                        "sequence" if in_title_info => {
                            let mut seq_name = String::new();
                            let mut seq_number = String::new();
                            for attr in e.attributes().flatten() {
                                let attr_name = String::from_utf8_lossy(attr.key.as_ref());
                                let attr_value = String::from_utf8_lossy(attr.value.as_ref());
                                if attr_name == "name" {
                                    seq_name = attr_value.to_string();
                                } else if attr_name == "number" {
                                    seq_number = attr_value.to_string();
                                }
                            }
                            if !seq_name.is_empty() {
                                let entry = if seq_number.is_empty() {
                                    seq_name
                                } else {
                                    format!("{} #{}", seq_name, seq_number)
                                };
                                sequences.push(entry);
                            }
                        }
                        "date" if in_title_info => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                let date = String::from_utf8_lossy(t.as_ref());
                                if !date.trim().is_empty() {
                                    metadata.created_at = Some(date.trim().to_string());
                                }
                            }
                        }
                        "lang" if in_title_info => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                let lang = String::from_utf8_lossy(t.as_ref());
                                if !lang.trim().is_empty() {
                                    metadata.language = Some(lang.trim().to_string());
                                }
                            }
                        }
                        "book-title" if in_title_info => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                let title = String::from_utf8_lossy(t.as_ref());
                                if !title.trim().is_empty() {
                                    metadata.title = Some(title.trim().to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    match tag.as_ref() {
                        "title-info" => {
                            in_title_info = false;
                        }
                        "description" => {
                            in_description = false;
                        }
                        "author" if in_author => {
                            in_author = false;
                            // Build full author name from parts
                            let mut parts = Vec::new();
                            if !first_name.is_empty() {
                                parts.push(first_name.clone());
                            }
                            if !middle_name.is_empty() {
                                parts.push(middle_name.clone());
                            }
                            if !last_name.is_empty() {
                                parts.push(last_name.clone());
                            }
                            let full_name = parts.join(" ");
                            if !full_name.is_empty() {
                                // Store individual name parts in additional metadata
                                let mut author_detail = serde_json::Map::new();
                                if !first_name.is_empty() {
                                    author_detail.insert("first_name".to_string(), serde_json::json!(first_name));
                                }
                                if !middle_name.is_empty() {
                                    author_detail.insert("middle_name".to_string(), serde_json::json!(middle_name));
                                }
                                if !last_name.is_empty() {
                                    author_detail.insert("last_name".to_string(), serde_json::json!(last_name));
                                }
                                if !nickname.is_empty() {
                                    author_detail.insert("nickname".to_string(), serde_json::json!(nickname));
                                }
                                authors.push(full_name);

                                // Store author details in additional metadata as array
                                let existing = additional
                                    .entry(std::borrow::Cow::Borrowed("author_details"))
                                    .or_insert_with(|| serde_json::json!([]));
                                if let serde_json::Value::Array(arr) = existing {
                                    arr.push(serde_json::Value::Object(author_detail));
                                }
                            } else if !nickname.is_empty() {
                                authors.push(nickname.clone());
                                let mut author_detail = serde_json::Map::new();
                                author_detail.insert("nickname".to_string(), serde_json::json!(nickname));
                                let existing = additional
                                    .entry(std::borrow::Cow::Borrowed("author_details"))
                                    .or_insert_with(|| serde_json::json!([]));
                                if let serde_json::Value::Array(arr) = existing {
                                    arr.push(serde_json::Value::Object(author_detail));
                                }
                            }
                        }
                        "annotation" if in_annotation => {
                            in_annotation = false;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(t)) if in_annotation => {
                    let decoded = String::from_utf8_lossy(t.as_ref());
                    let trimmed = decoded.trim();
                    if !trimmed.is_empty() {
                        if !annotation_text.is_empty() {
                            annotation_text.push(' ');
                        }
                        annotation_text.push_str(trimmed);
                    }
                }
                // Self-closing tags (e.g. <sequence ... />) produce Event::Empty, not Event::Start
                Ok(Event::Empty(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "sequence" && in_title_info {
                        let mut seq_name = String::new();
                        let mut seq_number = String::new();
                        for attr in e.attributes().flatten() {
                            let attr_name = String::from_utf8_lossy(attr.key.as_ref());
                            let attr_value = String::from_utf8_lossy(attr.value.as_ref());
                            if attr_name == "name" {
                                seq_name = attr_value.to_string();
                            } else if attr_name == "number" {
                                seq_number = attr_value.to_string();
                            }
                        }
                        if !seq_name.is_empty() {
                            let entry = if seq_number.is_empty() {
                                seq_name
                            } else {
                                format!("{} #{}", seq_name, seq_number)
                            };
                            sequences.push(entry);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        if !genres.is_empty() {
            metadata.subject = Some(genres.join(", "));
        }

        if !authors.is_empty() {
            metadata.authors = Some(authors);
        }

        let fb_metadata = crate::types::metadata::FictionBookMetadata {
            genres,
            sequences,
            annotation: if annotation_text.is_empty() {
                None
            } else {
                Some(annotation_text)
            },
        };
        metadata.format = Some(crate::types::metadata::FormatMetadata::FictionBook(fb_metadata));

        metadata.additional = additional;

        Ok(metadata)
    }

    /// Extract a single table from the XML reader (positioned just after `<table>`).
    fn extract_table(reader: &mut Reader<&[u8]>) -> Result<Vec<Vec<String>>> {
        let mut table: Vec<Vec<String>> = Vec::new();
        let mut current_row: Vec<String> = Vec::new();
        let mut in_row = false;
        let mut table_depth = 1;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    match tag.as_ref() {
                        "table" => table_depth += 1,
                        "tr" => {
                            in_row = true;
                            current_row.clear();
                        }
                        "td" | "th" if in_row => {
                            let cell_text = Self::extract_text_content(reader)?;
                            current_row.push(cell_text);
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    match tag.as_ref() {
                        "table" => {
                            table_depth -= 1;
                            if table_depth == 0 {
                                break;
                            }
                        }
                        "tr" if in_row => {
                            if !current_row.is_empty() {
                                table.push(std::mem::take(&mut current_row));
                            }
                            in_row = false;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(crate::error::KreuzbergError::parsing(format!(
                        "XML parsing error in table: {}",
                        e
                    )));
                }
                _ => {}
            }
        }

        Ok(table)
    }

    /// Extract all tables from the FictionBook body.
    fn extract_tables_from_body(data: &[u8]) -> Result<Vec<Table>> {
        let mut reader = Reader::from_reader(data);
        let mut tables = Vec::new();
        let mut table_index = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "table"
                        && let Ok(cells) = Self::extract_table(&mut reader)
                        && !cells.is_empty()
                    {
                        let markdown = cells_to_markdown(&cells);
                        tables.push(Table {
                            cells,
                            markdown,
                            page_number: table_index + 1,
                            bounding_box: None,
                        });
                        table_index += 1;
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(tables)
    }

    /// Extract embedded images from `<binary>` elements in FictionBook XML.
    ///
    /// FB2 embeds images as base64-encoded data inside `<binary>` elements with
    /// `content-type` and `id` attributes:
    /// ```xml
    /// <binary id="cover.jpg" content-type="image/jpeg">base64data...</binary>
    /// ```
    fn extract_binary_images(data: &[u8]) -> Vec<ExtractedImage> {
        let mut reader = Reader::from_reader(data);
        let mut images = Vec::new();
        let mut image_index = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "binary" {
                        let mut content_type = String::new();
                        let mut id = String::new();
                        for attr in e.attributes().flatten() {
                            let attr_name = String::from_utf8_lossy(attr.key.as_ref());
                            let attr_value = String::from_utf8_lossy(attr.value.as_ref());
                            if attr_name == "content-type" {
                                content_type = attr_value.to_string();
                            } else if attr_name == "id" {
                                id = attr_value.to_string();
                            }
                        }

                        // Read the base64 text content
                        let mut b64_text = String::new();
                        loop {
                            match reader.read_event() {
                                Ok(Event::Text(t)) => {
                                    b64_text.push_str(&String::from_utf8_lossy(t.as_ref()));
                                }
                                Ok(Event::End(_)) => break,
                                Ok(Event::Eof) => break,
                                Err(_) => break,
                                _ => {}
                            }
                        }

                        // Strip whitespace from base64 data and decode
                        let cleaned: String = b64_text.chars().filter(|c| !c.is_whitespace()).collect();
                        if cleaned.is_empty() {
                            continue;
                        }

                        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&cleaned) {
                            // Determine format from content-type or detect from bytes
                            let format = if let Some(subtype) = content_type.strip_prefix("image/") {
                                std::borrow::Cow::Owned(subtype.to_string())
                            } else {
                                crate::extraction::image_format::detect_image_format(&decoded)
                            };

                            let description = if id.is_empty() { None } else { Some(id) };

                            images.push(ExtractedImage {
                                data: Bytes::from(decoded),
                                format,
                                image_index,
                                page_number: None,
                                width: None,
                                height: None,
                                colorspace: None,
                                bits_per_component: None,
                                is_mask: false,
                                description,
                                ocr_result: None,
                                bounding_box: None,
                                source_path: None,
                            });
                            image_index += 1;
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        images
    }

    /// Extract hyperlinks from `<a>` elements in FictionBook XML body.
    ///
    /// FB2 uses `<a>` elements with `l:href` or `xlink:href` attributes for links:
    /// ```xml
    /// <a l:href="http://example.com">link text</a>
    /// <a xlink:href="#note1">footnote ref</a>
    /// ```
    fn extract_links(data: &[u8]) -> Vec<Uri> {
        let mut reader = Reader::from_reader(data);
        let mut uris = Vec::new();
        let mut in_body = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "body" {
                        in_body = true;
                    } else if tag == "a" && in_body {
                        let mut href = String::new();
                        for attr in e.attributes().flatten() {
                            let attr_name = String::from_utf8_lossy(attr.key.as_ref());
                            // FB2 uses l:href or xlink:href; also check plain href
                            if attr_name == "l:href" || attr_name == "xlink:href" || attr_name == "href" {
                                href = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                                break;
                            }
                        }

                        if href.is_empty() {
                            continue;
                        }

                        // Collect label text from the <a> element
                        let mut label_text = String::new();
                        let mut depth = 1;
                        loop {
                            match reader.read_event() {
                                Ok(Event::Start(_)) => depth += 1,
                                Ok(Event::End(_)) => {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                Ok(Event::Text(t)) => {
                                    let decoded = String::from_utf8_lossy(t.as_ref());
                                    let trimmed = decoded.trim();
                                    if !trimmed.is_empty() {
                                        if !label_text.is_empty() {
                                            label_text.push(' ');
                                        }
                                        label_text.push_str(trimmed);
                                    }
                                }
                                Ok(Event::Eof) => break,
                                Err(_) => break,
                                _ => {}
                            }
                        }

                        let label = if label_text.is_empty() { None } else { Some(label_text) };

                        uris.push(Uri::hyperlink(&href, label));
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "body" {
                        in_body = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        uris
    }

    /// Build an `InternalDocument` from FictionBook XML content.
    fn build_internal_document(data: &[u8]) -> Result<InternalDocument> {
        let mut reader = Reader::from_reader(data);
        let mut builder = InternalDocumentBuilder::new("fictionbook");

        let mut in_body = false;
        let mut is_notes_body = false;
        let mut section_depth: u8 = 0;
        let mut footnote_counter: u32 = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());

                    if tag == "body" {
                        let mut is_notes = false;
                        for a in e.attributes().flatten() {
                            let attr_name = String::from_utf8_lossy(a.key.as_ref());
                            if attr_name == "name" {
                                let val = String::from_utf8_lossy(a.value.as_ref());
                                if val == "notes" {
                                    is_notes = true;
                                    break;
                                }
                            }
                        }
                        if is_notes {
                            is_notes_body = true;
                        } else {
                            in_body = true;
                        }
                    } else if tag == "section" && in_body {
                        section_depth = section_depth.saturating_add(1);
                    } else if tag == "title" && in_body {
                        match Self::extract_text_content(&mut reader) {
                            Ok(text) if !text.is_empty() => {
                                let level = std::cmp::min(section_depth.max(1), 6);
                                builder.push_heading(level, &text, None, None);
                            }
                            _ => {}
                        }
                    } else if tag == "p" && in_body && !is_notes_body {
                        match Self::extract_paragraph_with_annotations(&mut reader) {
                            Ok((text, annotations)) if !text.is_empty() => {
                                builder.push_paragraph(&text, annotations, None, None);
                            }
                            _ => {}
                        }
                    } else if tag == "cite" && in_body {
                        match Self::extract_text_content(&mut reader) {
                            Ok(text) if !text.is_empty() => {
                                builder.push_quote_start();
                                builder.push_paragraph(&text, vec![], None, None);
                                builder.push_quote_end();
                            }
                            _ => {}
                        }
                    } else if (tag == "programlisting" || tag == "code") && in_body {
                        match Self::extract_text_content(&mut reader) {
                            Ok(text) if !text.is_empty() => {
                                builder.push_code(&text, None, None, None);
                            }
                            _ => {}
                        }
                    } else if tag == "section" && is_notes_body {
                        match Self::extract_footnote_text(&mut reader) {
                            Ok(text) if !text.is_empty() => {
                                footnote_counter += 1;
                                let key = format!("fn-{}", footnote_counter);
                                builder.push_footnote_definition(&text, &key, None);
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "body" {
                        if is_notes_body {
                            is_notes_body = false;
                        } else {
                            in_body = false;
                        }
                    } else if tag == "section" && in_body {
                        section_depth = section_depth.saturating_sub(1);
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(builder.build())
    }

    /// Extract paragraph text with annotation tracking for inline formatting.
    fn extract_paragraph_with_annotations(
        reader: &mut Reader<&[u8]>,
    ) -> Result<(String, Vec<crate::types::document_structure::TextAnnotation>)> {
        use crate::types::document_structure::{AnnotationKind, TextAnnotation};

        let mut text = String::new();
        let mut annotations = Vec::new();
        let mut depth = 0;
        let mut format_stack: Vec<(String, u32)> = Vec::new(); // (tag, start_byte_offset)

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    depth += 1;
                    match tag.as_ref() {
                        "emphasis" | "strong" | "strikethrough" | "code" => {
                            format_stack.push((tag.into_owned(), text.len() as u32));
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "p" && depth <= 1 {
                        break;
                    }
                    match tag.as_ref() {
                        "emphasis" | "strong" | "strikethrough" | "code" => {
                            if let Some((fmt_tag, start)) = format_stack.pop() {
                                let end = text.len() as u32;
                                if end > start
                                    && let Some(kind) = match fmt_tag.as_str() {
                                        "emphasis" => Some(AnnotationKind::Italic),
                                        "strong" => Some(AnnotationKind::Bold),
                                        "strikethrough" => Some(AnnotationKind::Strikethrough),
                                        "code" => Some(AnnotationKind::Code),
                                        _ => None,
                                    }
                                {
                                    annotations.push(TextAnnotation { start, end, kind });
                                }
                            }
                        }
                        _ => {}
                    }
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                Ok(Event::Text(t)) => {
                    let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                    let normalized = crate::utils::normalize_whitespace(&decoded);
                    let trimmed: &str = normalized.as_ref();
                    if !trimmed.is_empty() {
                        if !text.is_empty() && !text.ends_with(' ') {
                            text.push(' ');
                        }
                        text.push_str(trimmed);
                    }
                }
                Ok(Event::GeneralRef(r)) => {
                    let resolved = resolve_general_ref(r.as_ref());
                    if !resolved.is_empty() {
                        text.push_str(&resolved);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(crate::error::KreuzbergError::parsing(format!(
                        "XML parsing error: {}",
                        e
                    )));
                }
                _ => {}
            }
        }

        Ok((text.trim().to_string(), annotations))
    }

    /// Extract footnote text from a notes-body section.
    fn extract_footnote_text(reader: &mut Reader<&[u8]>) -> Result<String> {
        let mut text = String::new();
        let mut section_depth = 1;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "section" {
                        section_depth += 1;
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "section" {
                        section_depth -= 1;
                        if section_depth == 0 {
                            break;
                        }
                    }
                }
                Ok(Event::Text(t)) => {
                    let decoded = String::from_utf8_lossy(t.as_ref());
                    let trimmed = decoded.trim();
                    if !trimmed.is_empty() {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(trimmed);
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(text.trim().to_string())
    }
}

impl Plugin for FictionBookExtractor {
    fn name(&self) -> &str {
        "fictionbook-extractor"
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
        "Extracts content and metadata from FictionBook documents (FB2 format)"
    }

    fn author(&self) -> &str {
        "Kreuzberg Team"
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl DocumentExtractor for FictionBookExtractor {
    async fn extract_bytes(
        &self,
        content: &[u8],
        mime_type: &str,
        _config: &ExtractionConfig,
    ) -> Result<InternalDocument> {
        let metadata = Self::extract_metadata(content)?;

        let tables = Self::extract_tables_from_body(content)?;
        let images = Self::extract_binary_images(content);
        let links = Self::extract_links(content);

        let mut doc = Self::build_internal_document(content)?;
        doc.mime_type = std::borrow::Cow::Owned(mime_type.to_string());
        doc.metadata = metadata;

        // Add extracted tables
        for table in tables {
            doc.push_table(table);
        }

        // Add extracted images
        for image in images {
            doc.push_image(image);
        }

        // Add extracted links
        for uri in links {
            doc.push_uri(uri);
        }

        Ok(doc)
    }

    fn supported_mime_types(&self) -> &[&str] {
        &[
            "application/x-fictionbook+xml",
            "text/x-fictionbook",
            "application/x-fictionbook",
        ]
    }

    fn priority(&self) -> i32 {
        50
    }
}

#[cfg(all(test, feature = "office"))]
mod tests {
    use super::*;

    #[test]
    fn test_fictionbook_extractor_plugin_interface() {
        let extractor = FictionBookExtractor::new();
        assert_eq!(extractor.name(), "fictionbook-extractor");
        assert_eq!(extractor.priority(), 50);
        assert!(!extractor.supported_mime_types().is_empty());
    }

    #[test]
    fn test_fictionbook_extractor_default() {
        let extractor = FictionBookExtractor;
        assert_eq!(extractor.name(), "fictionbook-extractor");
    }

    #[test]
    fn test_fictionbook_extractor_supported_mime_types() {
        let extractor = FictionBookExtractor::new();
        let supported = extractor.supported_mime_types();
        assert!(supported.contains(&"application/x-fictionbook+xml"));
        assert!(supported.contains(&"text/x-fictionbook"));
    }

    #[tokio::test]
    async fn test_fictionbook_extractor_initialize_shutdown() {
        let extractor = FictionBookExtractor::new();
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    #[test]
    fn test_fictionbook_author_details() {
        let fb2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook>
  <description>
    <title-info>
      <author>
        <first-name>Leo</first-name>
        <middle-name>Nikolaevich</middle-name>
        <last-name>Tolstoy</last-name>
        <nickname>LNT</nickname>
      </author>
      <book-title>War and Peace</book-title>
      <genre>fiction</genre>
      <lang>ru</lang>
    </title-info>
  </description>
  <body><section><p>Content.</p></section></body>
</FictionBook>"#;

        let metadata = FictionBookExtractor::extract_metadata(fb2).expect("Metadata extraction failed");
        assert_eq!(metadata.title, Some("War and Peace".to_string()));
        assert!(metadata.authors.is_some());
        let authors = metadata.authors.expect("authors should be present");
        assert_eq!(authors.len(), 1);
        assert!(authors[0].contains("Leo"), "expected first name, got: {}", authors[0]);
        assert!(
            authors[0].contains("Tolstoy"),
            "expected last name, got: {}",
            authors[0]
        );

        // Check author details in additional metadata
        let details = metadata
            .additional
            .get("author_details")
            .expect("expected author_details");
        assert!(details.is_array());
        let arr = details.as_array().expect("author_details should be an array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["first_name"], "Leo");
        assert_eq!(arr[0]["middle_name"], "Nikolaevich");
        assert_eq!(arr[0]["last_name"], "Tolstoy");
        assert_eq!(arr[0]["nickname"], "LNT");
    }

    #[test]
    fn test_fictionbook_genre_metadata() {
        let fb2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook>
  <description>
    <title-info>
      <genre>sci_fi</genre>
      <genre>adventure</genre>
      <lang>en</lang>
    </title-info>
  </description>
  <body><section><p>Content.</p></section></body>
</FictionBook>"#;

        let metadata = FictionBookExtractor::extract_metadata(fb2).expect("Metadata extraction failed");
        assert!(metadata.subject.is_some());
        let subject = metadata.subject.expect("subject should be present");
        assert!(subject.contains("sci_fi"), "expected sci_fi genre, got: {subject}");
        assert!(
            subject.contains("adventure"),
            "expected adventure genre, got: {subject}"
        );

        let fb_meta = match &metadata.format {
            Some(crate::types::metadata::FormatMetadata::FictionBook(fb)) => fb,
            other => panic!("expected FictionBook format metadata, got: {:?}", other),
        };
        assert_eq!(fb_meta.genres.len(), 2);
    }

    #[test]
    fn test_fictionbook_annotation() {
        let fb2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook>
  <description>
    <title-info>
      <annotation>
        <p>This is the book annotation describing the plot.</p>
      </annotation>
      <lang>en</lang>
    </title-info>
  </description>
  <body><section><p>Content.</p></section></body>
</FictionBook>"#;

        let metadata = FictionBookExtractor::extract_metadata(fb2).expect("Metadata extraction failed");
        let fb_meta = match &metadata.format {
            Some(crate::types::metadata::FormatMetadata::FictionBook(fb)) => fb,
            other => panic!("expected FictionBook format metadata, got: {:?}", other),
        };
        let annotation = fb_meta.annotation.as_deref().expect("expected annotation");
        assert!(
            annotation.contains("book annotation"),
            "expected annotation text, got: {}",
            annotation
        );
    }

    #[test]
    fn test_fictionbook_sequence() {
        let fb2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook>
  <description>
    <title-info>
      <sequence name="Foundation Series" number="3"/>
      <lang>en</lang>
    </title-info>
  </description>
  <body><section><p>Content.</p></section></body>
</FictionBook>"#;

        let metadata = FictionBookExtractor::extract_metadata(fb2).expect("Metadata extraction failed");
        let fb_meta = match &metadata.format {
            Some(crate::types::metadata::FormatMetadata::FictionBook(fb)) => fb,
            other => panic!("expected FictionBook format metadata, got: {:?}", other),
        };
        assert_eq!(fb_meta.sequences.len(), 1);
        assert!(
            fb_meta.sequences[0].contains("Foundation Series"),
            "expected Foundation Series, got: {}",
            fb_meta.sequences[0]
        );
        assert!(
            fb_meta.sequences[0].contains("#3"),
            "expected #3, got: {}",
            fb_meta.sequences[0]
        );
    }

    #[test]
    fn test_fictionbook_binary_images() {
        // A minimal 1x1 red PNG as base64
        let png_b64 =
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwADhQGAWjR9awAAAABJRU5ErkJggg==";
        let fb2 = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook>
  <description>
    <title-info><lang>en</lang></title-info>
  </description>
  <body><section><p>Content with image.</p></section></body>
  <binary id="cover.png" content-type="image/png">{}</binary>
</FictionBook>"#,
            png_b64
        );

        let images = FictionBookExtractor::extract_binary_images(fb2.as_bytes());
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].format, "png");
        assert_eq!(images[0].image_index, 0);
        assert_eq!(images[0].description, Some("cover.png".to_string()));
        assert!(!images[0].data.is_empty(), "image data should not be empty");
        // Verify it starts with PNG magic bytes
        assert!(images[0].data.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
    }

    #[test]
    fn test_fictionbook_binary_images_multiple() {
        let fb2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook>
  <description>
    <title-info><lang>en</lang></title-info>
  </description>
  <body><section><p>Content.</p></section></body>
  <binary id="img1.jpg" content-type="image/jpeg">AAAA</binary>
  <binary id="img2.gif" content-type="image/gif">BBBB</binary>
</FictionBook>"#;

        let images = FictionBookExtractor::extract_binary_images(fb2);
        // Both entries are present (even if base64 decodes to short data)
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].image_index, 0);
        assert_eq!(images[1].image_index, 1);
        assert_eq!(images[0].format, "jpeg");
        assert_eq!(images[1].format, "gif");
    }

    #[test]
    fn test_fictionbook_binary_images_empty() {
        let fb2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook>
  <description>
    <title-info><lang>en</lang></title-info>
  </description>
  <body><section><p>No images here.</p></section></body>
</FictionBook>"#;

        let images = FictionBookExtractor::extract_binary_images(fb2);
        assert!(images.is_empty());
    }

    #[test]
    fn test_fictionbook_links() {
        let fb2 = br##"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook>
  <description>
    <title-info><lang>en</lang></title-info>
  </description>
  <body>
    <section>
      <p>Visit <a l:href="http://example.com">Example Site</a> for more.</p>
      <p>See <a xlink:href="#note1">footnote 1</a> below.</p>
      <p>Also <a href="https://rust-lang.org">Rust</a> is great.</p>
    </section>
  </body>
</FictionBook>"##;

        let links = FictionBookExtractor::extract_links(fb2);
        assert_eq!(links.len(), 3);

        assert_eq!(links[0].url, "http://example.com");
        assert_eq!(links[0].label, Some("Example Site".to_string()));
        assert_eq!(links[0].kind, crate::types::uri::UriKind::Hyperlink);

        assert_eq!(links[1].url, "#note1");
        assert_eq!(links[1].label, Some("footnote 1".to_string()));
        assert_eq!(links[1].kind, crate::types::uri::UriKind::Anchor);

        assert_eq!(links[2].url, "https://rust-lang.org");
        assert_eq!(links[2].label, Some("Rust".to_string()));
        assert_eq!(links[2].kind, crate::types::uri::UriKind::Hyperlink);
    }

    #[test]
    fn test_fictionbook_links_no_body() {
        let fb2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook>
  <description>
    <title-info>
      <annotation><a l:href="http://meta-link.com">meta</a></annotation>
      <lang>en</lang>
    </title-info>
  </description>
</FictionBook>"#;

        // Links outside <body> should not be extracted
        let links = FictionBookExtractor::extract_links(fb2);
        assert!(links.is_empty());
    }

    #[test]
    fn test_fictionbook_links_empty_href() {
        let fb2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook>
  <description>
    <title-info><lang>en</lang></title-info>
  </description>
  <body>
    <section>
      <p>A link with <a l:href="">no href</a>.</p>
    </section>
  </body>
</FictionBook>"#;

        let links = FictionBookExtractor::extract_links(fb2);
        assert!(links.is_empty());
    }

    #[test]
    fn test_fictionbook_tables() {
        let fb2 = br#"<?xml version="1.0" encoding="UTF-8"?>
<FictionBook>
  <description>
    <title-info><lang>en</lang></title-info>
  </description>
  <body>
    <section>
      <table>
        <tr><th>Name</th><th>Age</th></tr>
        <tr><td>Alice</td><td>30</td></tr>
        <tr><td>Bob</td><td>25</td></tr>
      </table>
    </section>
  </body>
</FictionBook>"#;

        let tables = FictionBookExtractor::extract_tables_from_body(fb2).expect("Table extraction failed");
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].cells.len(), 3);
        assert_eq!(tables[0].cells[0], vec!["Name", "Age"]);
        assert_eq!(tables[0].cells[1], vec!["Alice", "30"]);
    }
}
