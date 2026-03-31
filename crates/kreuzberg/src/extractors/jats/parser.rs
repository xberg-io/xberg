//! XML parsing and document structure traversal for JATS documents.

use crate::Result;
use crate::text::utf8_validation;
use quick_xml::Reader;
use quick_xml::events::Event;

/// Extract text content from a JATS element and its children.
pub(super) fn extract_text_content(reader: &mut Reader<&[u8]>) -> Result<String> {
    let mut text = String::new();
    let mut depth = 0;

    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) => {
                depth += 1;
            }
            Ok(Event::End(_)) => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                if !text.is_empty() && !text.ends_with('\n') {
                    text.push(' ');
                }
            }
            Ok(Event::Text(t)) => {
                let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                if !decoded.trim().is_empty() {
                    text.push_str(&decoded);
                    text.push(' ');
                }
            }
            Ok(Event::CData(t)) => {
                let decoded = utf8_validation::from_utf8(t.as_ref()).unwrap_or("").to_string();
                if !decoded.trim().is_empty() {
                    text.push_str(&decoded);
                    text.push('\n');
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

/// Extract a formatted citation string from a `<ref>` element.
///
/// Parses structured `<element-citation>` children (person-group, article-title,
/// source, year, volume, fpage, lpage) into a conventional citation string like:
/// `Brown T, Davis K. Cognitive effects of caffeine. J Neurosci. 2002;15:234-241.`
///
/// Falls back to plain text extraction for `<mixed-citation>` or unrecognized structures.
pub(super) fn extract_citation_text(reader: &mut Reader<&[u8]>) -> Result<String> {
    let mut depth: u32 = 0;
    let mut in_element_citation = false;
    let mut in_mixed_citation = false;
    let mut in_person_group = false;
    let mut in_name = false;

    // Structured citation fields
    let mut authors: Vec<String> = Vec::new();
    let mut current_surname = String::new();
    let mut current_given = String::new();
    let mut article_title = String::new();
    let mut source = String::new();
    let mut year = String::new();
    let mut volume = String::new();
    let mut fpage = String::new();
    let mut lpage = String::new();

    // Current tag name for text collection
    let mut current_tag = String::new();

    // Fallback for mixed-citation
    let mut mixed_text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                depth += 1;
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag.as_str() {
                    "element-citation" => {
                        in_element_citation = true;
                    }
                    "mixed-citation" => {
                        in_mixed_citation = true;
                    }
                    "person-group" if in_element_citation => {
                        in_person_group = true;
                    }
                    "name" if in_person_group => {
                        in_name = true;
                        current_surname.clear();
                        current_given.clear();
                    }
                    "surname" | "given-names" | "article-title" | "source" | "year" | "volume" | "fpage" | "lpage"
                        if in_element_citation =>
                    {
                        current_tag = tag;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                if depth == 0 {
                    break;
                }
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag.as_str() {
                    "name" if in_name => {
                        in_name = false;
                        let mut author = String::new();
                        if !current_surname.is_empty() {
                            author.push_str(current_surname.trim());
                        }
                        if !current_given.is_empty() {
                            if !author.is_empty() {
                                author.push(' ');
                            }
                            author.push_str(current_given.trim());
                        }
                        if !author.is_empty() {
                            authors.push(author);
                        }
                    }
                    "person-group" => {
                        in_person_group = false;
                    }
                    "element-citation" => {
                        in_element_citation = false;
                    }
                    "mixed-citation" => {
                        in_mixed_citation = false;
                    }
                    _ => {}
                }

                current_tag.clear();
                depth -= 1;
            }
            Ok(Event::Text(t)) => {
                let decoded = String::from_utf8_lossy(t.as_ref()).to_string();
                let trimmed = decoded.trim();

                if !trimmed.is_empty() {
                    if in_mixed_citation {
                        if !mixed_text.is_empty() {
                            mixed_text.push(' ');
                        }
                        mixed_text.push_str(trimmed);
                    } else if in_element_citation {
                        match current_tag.as_str() {
                            "surname" => current_surname.push_str(trimmed),
                            "given-names" => current_given.push_str(trimmed),
                            "article-title" => {
                                if !article_title.is_empty() {
                                    article_title.push(' ');
                                }
                                article_title.push_str(trimmed);
                            }
                            "source" => source.push_str(trimmed),
                            "year" => year.push_str(trimmed),
                            "volume" => volume.push_str(trimmed),
                            "fpage" => fpage.push_str(trimmed),
                            "lpage" => lpage.push_str(trimmed),
                            _ => {}
                        }
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

    // If we parsed a mixed-citation, return its text directly
    if !mixed_text.is_empty() {
        return Ok(mixed_text);
    }

    // Build formatted citation from structured fields
    let mut citation = String::new();

    // Authors
    if !authors.is_empty() {
        citation.push_str(&authors.join(", "));
        citation.push_str(". ");
    }

    // Article title
    if !article_title.is_empty() {
        citation.push_str(&article_title);
        citation.push_str(". ");
    }

    // Source (journal name)
    if !source.is_empty() {
        citation.push_str(&source);
        citation.push('.');
    }

    // Year, volume, pages
    if !year.is_empty() {
        citation.push(' ');
        citation.push_str(&year);
    }
    if !volume.is_empty() {
        citation.push(';');
        citation.push_str(&volume);
    }
    if !fpage.is_empty() {
        citation.push(':');
        citation.push_str(&fpage);
        if !lpage.is_empty() {
            citation.push('-');
            citation.push_str(&lpage);
        }
    }
    if !citation.is_empty() && !citation.ends_with('.') {
        citation.push('.');
    }

    Ok(citation.trim().to_string())
}
