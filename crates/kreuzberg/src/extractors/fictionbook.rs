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

use crate::Result;
use crate::core::config::ExtractionConfig;
use crate::plugins::{DocumentExtractor, Plugin};
use crate::types::{ExtractionResult, Metadata};
use async_trait::async_trait;
use quick_xml::Reader;
use quick_xml::events::Event;

const HORIZONTAL_RULE: &str = "------------------------------------------------------------------------";

/// Convert a character to its Unicode superscript equivalent, if available.
fn char_to_superscript(c: char) -> Option<char> {
    match c {
        '0' => Some('\u{2070}'),
        '1' => Some('\u{00B9}'),
        '2' => Some('\u{00B2}'),
        '3' => Some('\u{00B3}'),
        '4' => Some('\u{2074}'),
        '5' => Some('\u{2075}'),
        '6' => Some('\u{2076}'),
        '7' => Some('\u{2077}'),
        '8' => Some('\u{2078}'),
        '9' => Some('\u{2079}'),
        '+' => Some('\u{207A}'),
        '-' => Some('\u{207B}'),
        '=' => Some('\u{207C}'),
        '(' => Some('\u{207D}'),
        ')' => Some('\u{207E}'),
        'n' => Some('\u{207F}'),
        'i' => Some('\u{2071}'),
        _ => None,
    }
}

/// Convert a character to its Unicode subscript equivalent, if available.
fn char_to_subscript(c: char) -> Option<char> {
    match c {
        '0' => Some('\u{2080}'),
        '1' => Some('\u{2081}'),
        '2' => Some('\u{2082}'),
        '3' => Some('\u{2083}'),
        '4' => Some('\u{2084}'),
        '5' => Some('\u{2085}'),
        '6' => Some('\u{2086}'),
        '7' => Some('\u{2087}'),
        '8' => Some('\u{2088}'),
        '9' => Some('\u{2089}'),
        '+' => Some('\u{208A}'),
        '-' => Some('\u{208B}'),
        '=' => Some('\u{208C}'),
        '(' => Some('\u{208D}'),
        ')' => Some('\u{208E}'),
        _ => None,
    }
}

/// Convert a string to Unicode superscript characters where possible.
fn to_superscript(s: &str) -> String {
    s.chars().map(|c| char_to_superscript(c).unwrap_or(c)).collect()
}

/// Convert a string to Unicode subscript characters where possible.
fn to_subscript(s: &str) -> String {
    s.chars().map(|c| char_to_subscript(c).unwrap_or(c)).collect()
}

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

    /// Extract paragraph content with optional markdown formatting preservation.
    /// When `plain` is true, skips most inline formatting markers but keeps
    /// strikethrough and converts sub/superscript to Unicode.
    fn extract_paragraph_content(reader: &mut Reader<&[u8]>, plain: bool) -> Result<String> {
        let mut text = String::new();
        let mut depth = 0;
        let mut in_sub = false;
        let mut in_sup = false;
        let mut sub_buf = String::new();
        let mut sup_buf = String::new();
        let mut last_was_open_marker = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    depth += 1;
                    match tag.as_ref() {
                        "emphasis" if !plain => {
                            text.push('*');
                            last_was_open_marker = true;
                        }
                        "strong" if !plain => {
                            text.push_str("**");
                            last_was_open_marker = true;
                        }
                        "strikethrough" => {
                            text.push_str("~~");
                            last_was_open_marker = true;
                        }
                        "code" if !plain => {
                            text.push('`');
                            last_was_open_marker = true;
                        }
                        "sub" if plain => {
                            in_sub = true;
                            sub_buf.clear();
                        }
                        "sup" if plain => {
                            in_sup = true;
                            sup_buf.clear();
                        }
                        "sub" => {
                            text.push('~');
                            last_was_open_marker = true;
                        }
                        "sup" => {
                            text.push('^');
                            last_was_open_marker = true;
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
                    last_was_open_marker = false;
                    match tag.as_ref() {
                        "emphasis" if !plain => text.push('*'),
                        "strong" if !plain => text.push_str("**"),
                        "strikethrough" => text.push_str("~~"),
                        "code" if !plain => text.push('`'),
                        "sub" if plain => {
                            in_sub = false;
                            text.push_str(&to_subscript(&sub_buf));
                        }
                        "sup" if plain => {
                            in_sup = false;
                            text.push_str(&to_superscript(&sup_buf));
                        }
                        "sub" => text.push('~'),
                        "sup" => text.push('^'),
                        _ => {}
                    }
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                Ok(Event::Text(t)) => {
                    let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                    let had_leading_space = decoded.starts_with(char::is_whitespace);
                    let had_trailing_space = decoded.ends_with(char::is_whitespace);
                    let normalized = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
                    let trimmed = normalized.as_str();
                    if !trimmed.is_empty() {
                        if in_sub {
                            sub_buf.push_str(trimmed);
                        } else if in_sup {
                            sup_buf.push_str(trimmed);
                        } else {
                            let starts_with_punct = trimmed.starts_with(['.', ',', ';', ':', '!', '?', ')', ']', '[']);
                            let needs_space = !text.is_empty()
                                && !text.ends_with(' ')
                                && !last_was_open_marker
                                && !starts_with_punct
                                && (had_leading_space
                                    || !text.ends_with(|c: char| {
                                        c == '&'
                                            || c == '<'
                                            || c == '>'
                                            || c == '"'
                                            || c == '\''
                                            || ('\u{2070}'..='\u{209F}').contains(&c)
                                            || c == '\u{00B9}'
                                            || c == '\u{00B2}'
                                            || c == '\u{00B3}'
                                    }));
                            if needs_space {
                                text.push(' ');
                            }
                            text.push_str(trimmed);
                            if had_trailing_space && !text.ends_with(' ') {
                                text.push(' ');
                            }
                        }
                        last_was_open_marker = false;
                    }
                }
                Ok(Event::GeneralRef(r)) => {
                    let resolved = resolve_general_ref(r.as_ref());
                    if !resolved.is_empty() {
                        if in_sub {
                            sub_buf.push_str(&resolved);
                        } else if in_sup {
                            sup_buf.push_str(&resolved);
                        } else {
                            text.push_str(&resolved);
                        }
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

        Ok(text.trim().to_string())
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
                    let normalized = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
                    let trimmed = normalized.as_str();
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
        let mut in_title_info = false;
        let mut in_description = false;

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
                        "genre" if in_title_info => {
                            if let Ok(Event::Text(t)) = reader.read_event() {
                                let genre = String::from_utf8_lossy(t.as_ref());
                                if !genre.trim().is_empty() && genre.trim() != "unrecognised" {
                                    metadata.subject = Some(genre.trim().to_string());
                                }
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
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "title-info" {
                        in_title_info = false;
                    } else if tag == "description" {
                        in_description = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(metadata)
    }

    /// Extract content from FictionBook document body sections.
    fn extract_body_content(data: &[u8], plain: bool) -> Result<String> {
        let mut reader = Reader::from_reader(data);
        let mut content = String::new();
        let mut in_body = false;
        let mut is_notes_body = false;
        let mut footnotes = Vec::new();

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
                    } else if tag == "section" && is_notes_body {
                        // Extract footnote from notes body
                        if let Ok(note) = Self::extract_footnote_section(&mut reader, plain)
                            && !note.is_empty()
                        {
                            footnotes.push(note);
                        }
                    } else if tag == "section" && in_body {
                        match Self::extract_section_content(&mut reader, plain) {
                            Ok(section_content) if !section_content.is_empty() => {
                                if !content.is_empty() && !content.ends_with("\n\n") {
                                    content.push('\n');
                                }
                                content.push_str(&section_content);
                                content.push('\n');
                            }
                            _ => {}
                        }
                    } else if tag == "poem" && in_body {
                        match Self::extract_poem_content(&mut reader, plain) {
                            Ok(poem_content) if !poem_content.is_empty() => {
                                if !content.is_empty() && !content.ends_with("\n\n") {
                                    content.push('\n');
                                }
                                content.push_str(&poem_content);
                                content.push('\n');
                            }
                            _ => {}
                        }
                    } else if tag == "p" && in_body {
                        match Self::extract_paragraph_content(&mut reader, plain) {
                            Ok(para) if !para.is_empty() => {
                                content.push_str(&para);
                                content.push_str("\n\n");
                            }
                            _ => {}
                        }
                    } else if tag == "title" && in_body {
                        match Self::extract_text_content(&mut reader) {
                            Ok(title_content) if !title_content.is_empty() => {
                                if plain {
                                    content.push_str(&format!("{}\n\n", title_content));
                                } else {
                                    content.push_str(&format!("# {}\n\n", title_content));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Event::Empty(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "empty-line" && in_body {
                        content.push_str(HORIZONTAL_RULE);
                        content.push_str("\n\n");
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
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        // Append footnotes at the end
        if !footnotes.is_empty() {
            if !content.ends_with('\n') {
                content.push('\n');
            }
            for note in &footnotes {
                content.push_str(note);
                content.push('\n');
            }
        }

        Ok(content.trim().to_string())
    }

    /// Extract a footnote section from the notes body.
    fn extract_footnote_section(reader: &mut Reader<&[u8]>, plain: bool) -> Result<String> {
        let mut text = String::new();
        let mut section_depth = 1;
        let mut note_id = String::new();

        // Try to get the note ID from the section attributes (already consumed by caller)
        // We'll just extract the content

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    match tag.as_ref() {
                        "section" => section_depth += 1,
                        "title" => {
                            if let Ok(title) = Self::extract_text_content(reader)
                                && !title.is_empty()
                            {
                                note_id = title;
                            }
                        }
                        "p" => {
                            if let Ok(para) = Self::extract_paragraph_content(reader, plain)
                                && !para.is_empty()
                            {
                                if !text.is_empty() {
                                    text.push(' ');
                                }
                                text.push_str(&para);
                            }
                        }
                        _ => {}
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
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        if text.is_empty() {
            return Ok(String::new());
        }

        if !note_id.is_empty() {
            Ok(format!("[{}] {}", note_id, text))
        } else {
            Ok(text)
        }
    }

    /// Extract content from a poem element.
    fn extract_poem_content(reader: &mut Reader<&[u8]>, plain: bool) -> Result<String> {
        let mut content = String::new();
        let mut poem_depth = 1;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());

                    match tag.as_ref() {
                        "poem" => {
                            poem_depth += 1;
                        }
                        "title" => match Self::extract_text_content(reader) {
                            Ok(title_text) if !title_text.is_empty() => {
                                if plain {
                                    content.push_str(&format!("{}\n\n", title_text));
                                } else {
                                    content.push_str(&format!("# {}\n\n", title_text));
                                }
                            }
                            _ => {}
                        },
                        "epigraph" => match Self::extract_text_content(reader) {
                            Ok(epigraph_text) if !epigraph_text.is_empty() => {
                                if !plain {
                                    content.push_str("> ");
                                }
                                content.push_str(&epigraph_text);
                                content.push_str("\n\n");
                            }
                            _ => {}
                        },
                        "stanza" => match Self::extract_stanza_content(reader, plain) {
                            Ok(stanza_text) if !stanza_text.is_empty() => {
                                content.push_str(&stanza_text);
                                content.push_str("\n\n");
                            }
                            _ => {}
                        },
                        "text-author" => match Self::extract_text_content(reader) {
                            Ok(author) if !author.is_empty() => {
                                content.push_str(&author);
                                content.push_str("\n\n");
                            }
                            _ => {}
                        },
                        "date" => match Self::extract_text_content(reader) {
                            Ok(date) if !date.is_empty() => {
                                content.push_str(&date);
                                content.push_str("\n\n");
                            }
                            _ => {}
                        },
                        "p" => match Self::extract_paragraph_content(reader, plain) {
                            Ok(para) if !para.is_empty() => {
                                content.push_str(&para);
                                content.push_str("\n\n");
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "poem" {
                        poem_depth -= 1;
                        if poem_depth == 0 {
                            break;
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(content.trim().to_string())
    }

    /// Extract a single verse line (content of <v> tag).
    fn extract_verse_line(reader: &mut Reader<&[u8]>, plain: bool) -> Result<String> {
        let mut text = String::new();
        let mut depth = 0;
        let mut in_sub = false;
        let mut in_sup = false;
        let mut sub_buf = String::new();
        let mut sup_buf = String::new();
        let mut last_was_open_marker = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    depth += 1;
                    match tag.as_ref() {
                        "emphasis" if !plain => {
                            text.push('*');
                            last_was_open_marker = true;
                        }
                        "strong" if !plain => {
                            text.push_str("**");
                            last_was_open_marker = true;
                        }
                        "strikethrough" => {
                            text.push_str("~~");
                            last_was_open_marker = true;
                        }
                        "code" if !plain => {
                            text.push('`');
                            last_was_open_marker = true;
                        }
                        "sub" if plain => {
                            in_sub = true;
                            sub_buf.clear();
                        }
                        "sup" if plain => {
                            in_sup = true;
                            sup_buf.clear();
                        }
                        "sub" => {
                            text.push('~');
                            last_was_open_marker = true;
                        }
                        "sup" => {
                            text.push('^');
                            last_was_open_marker = true;
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "v" && depth == 0 {
                        break;
                    }
                    last_was_open_marker = false;
                    match tag.as_ref() {
                        "emphasis" if !plain => text.push('*'),
                        "strong" if !plain => text.push_str("**"),
                        "strikethrough" => text.push_str("~~"),
                        "code" if !plain => text.push('`'),
                        "sub" if plain => {
                            in_sub = false;
                            text.push_str(&to_subscript(&sub_buf));
                        }
                        "sup" if plain => {
                            in_sup = false;
                            text.push_str(&to_superscript(&sup_buf));
                        }
                        "sub" => text.push('~'),
                        "sup" => text.push('^'),
                        _ => {}
                    }
                    if depth > 0 {
                        depth -= 1;
                    }
                }
                Ok(Event::Text(t)) => {
                    let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                    let had_leading_space = decoded.starts_with(char::is_whitespace);
                    let had_trailing_space = decoded.ends_with(char::is_whitespace);
                    let normalized = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
                    let trimmed = normalized.as_str();
                    if !trimmed.is_empty() {
                        if in_sub {
                            sub_buf.push_str(trimmed);
                        } else if in_sup {
                            sup_buf.push_str(trimmed);
                        } else {
                            let starts_with_punct = trimmed.starts_with(['.', ',', ';', ':', '!', '?', ')', ']', '[']);
                            let needs_space = !text.is_empty()
                                && !text.ends_with(' ')
                                && !last_was_open_marker
                                && !starts_with_punct
                                && (had_leading_space
                                    || !text.ends_with(|c: char| {
                                        c == '&'
                                            || c == '<'
                                            || c == '>'
                                            || c == '"'
                                            || c == '\''
                                            || ('\u{2070}'..='\u{209F}').contains(&c)
                                            || c == '\u{00B9}'
                                            || c == '\u{00B2}'
                                            || c == '\u{00B3}'
                                    }));
                            if needs_space {
                                text.push(' ');
                            }
                            text.push_str(trimmed);
                            if had_trailing_space && !text.ends_with(' ') {
                                text.push(' ');
                            }
                        }
                        last_was_open_marker = false;
                    }
                }
                Ok(Event::GeneralRef(r)) => {
                    let resolved = resolve_general_ref(r.as_ref());
                    if !resolved.is_empty() {
                        if in_sub {
                            sub_buf.push_str(&resolved);
                        } else if in_sup {
                            sup_buf.push_str(&resolved);
                        } else {
                            text.push_str(&resolved);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(text.trim().to_string())
    }

    /// Extract content from a stanza element (contains verse lines).
    fn extract_stanza_content(reader: &mut Reader<&[u8]>, plain: bool) -> Result<String> {
        let mut content = String::new();
        let mut stanza_depth = 1;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());

                    match tag.as_ref() {
                        "stanza" => {
                            stanza_depth += 1;
                        }
                        "subtitle" => match Self::extract_text_content(reader) {
                            Ok(subtitle_text) if !subtitle_text.is_empty() => {
                                if plain {
                                    content.push_str(&format!("{}\n\n", subtitle_text));
                                } else {
                                    content.push_str(&format!("*{}*\n\n", subtitle_text));
                                }
                            }
                            _ => {}
                        },
                        "title" => match Self::extract_text_content(reader) {
                            Ok(title_text) if !title_text.is_empty() => {
                                if plain {
                                    content.push_str(&format!("{}\n\n", title_text));
                                } else {
                                    content.push_str(&format!("## {}\n\n", title_text));
                                }
                            }
                            _ => {}
                        },
                        "v" => match Self::extract_verse_line(reader, plain) {
                            Ok(verse) if !verse.is_empty() => {
                                content.push_str(&verse);
                                content.push('\n');
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "stanza" {
                        stanza_depth -= 1;
                        if stanza_depth == 0 {
                            break;
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(content.trim().to_string())
    }

    /// Extract content from a section with proper hierarchy.
    fn extract_section_content(reader: &mut Reader<&[u8]>, plain: bool) -> Result<String> {
        let mut content = String::new();
        let mut section_depth = 1;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());

                    match tag.as_ref() {
                        "section" => {
                            section_depth += 1;
                        }
                        "title" => match Self::extract_text_content(reader) {
                            Ok(title_text) if !title_text.is_empty() => {
                                if plain {
                                    content.push_str(&format!("{}\n\n", title_text));
                                } else {
                                    let heading_level = std::cmp::min(section_depth + 1, 6);
                                    let heading = "#".repeat(heading_level);
                                    content.push_str(&format!("{} {}\n\n", heading, title_text));
                                }
                            }
                            _ => {}
                        },
                        "p" => match Self::extract_paragraph_content(reader, plain) {
                            Ok(para) if !para.is_empty() => {
                                content.push_str(&para);
                                content.push_str("\n\n");
                            }
                            _ => {}
                        },
                        "poem" => match Self::extract_poem_content(reader, plain) {
                            Ok(poem_text) if !poem_text.is_empty() => {
                                content.push_str(&poem_text);
                                content.push_str("\n\n");
                            }
                            _ => {}
                        },
                        "cite" => match Self::extract_text_content(reader) {
                            Ok(cite_content) if !cite_content.is_empty() => {
                                if !plain {
                                    content.push_str("> ");
                                } else {
                                    content.push_str("  ");
                                }
                                content.push_str(&cite_content);
                                content.push_str("\n\n");
                            }
                            _ => {}
                        },
                        "empty-line" => {
                            content.push_str(HORIZONTAL_RULE);
                            content.push_str("\n\n");
                        }
                        _ => {}
                    }
                }
                Ok(Event::Empty(e)) => {
                    let name = e.name();
                    let tag = crate::utils::xml_tag_name(name.as_ref());
                    if tag == "empty-line" {
                        content.push_str(HORIZONTAL_RULE);
                        content.push_str("\n\n");
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
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        Ok(content.trim().to_string())
    }

    /// Build a `DocumentStructure` from FictionBook XML content.
    fn build_document_structure(data: &[u8]) -> Result<crate::types::document_structure::DocumentStructure> {
        use crate::types::builder::DocumentStructureBuilder;

        let mut reader = Reader::from_reader(data);
        let mut builder = DocumentStructureBuilder::new().source_format("fictionbook");

        let mut in_body = false;
        let mut is_notes_body = false;
        let mut section_depth: u8 = 0;

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
                        // Extract paragraph with inline formatting info for annotations
                        match Self::extract_paragraph_with_annotations(&mut reader) {
                            Ok((text, annotations)) if !text.is_empty() => {
                                builder.push_paragraph(&text, annotations, None, None);
                            }
                            _ => {}
                        }
                    } else if tag == "cite" && in_body {
                        match Self::extract_text_content(&mut reader) {
                            Ok(text) if !text.is_empty() => {
                                builder.push_quote(None);
                                builder.push_paragraph(&text, vec![], None, None);
                                builder.exit_container();
                            }
                            _ => {}
                        }
                    } else if (tag == "programlisting" || tag == "code") && in_body {
                        match Self::extract_text_content(&mut reader) {
                            Ok(text) if !text.is_empty() => {
                                builder.push_code(&text, None, None);
                            }
                            _ => {}
                        }
                    } else if tag == "section" && is_notes_body {
                        // Extract footnote
                        match Self::extract_footnote_text(&mut reader) {
                            Ok(text) if !text.is_empty() => {
                                builder.push_footnote(&text, None);
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
                    let normalized = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
                    let trimmed = normalized.as_str();
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
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let metadata = Self::extract_metadata(content)?;
        let plain = matches!(
            config.output_format,
            crate::core::config::OutputFormat::Plain | crate::core::config::OutputFormat::Structured
        );

        let extracted_content = Self::extract_body_content(content, plain)?;

        let document = if config.include_document_structure {
            Some(Self::build_document_structure(content)?)
        } else {
            None
        };

        Ok(ExtractionResult {
            content: extracted_content,
            mime_type: mime_type.to_string().into(),
            metadata,
            tables: vec![],
            detected_languages: None,
            chunks: None,
            images: None,
            djot_content: None,
            pages: None,
            elements: None,
            ocr_elements: None,
            document,
            #[cfg(any(feature = "keywords-yake", feature = "keywords-rake"))]
            extracted_keywords: None,
            quality_score: None,
            processing_warnings: Vec::new(),
            annotations: None,
        })
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
}
