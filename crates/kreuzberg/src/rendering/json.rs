//! JSON tree renderer for `InternalDocument`.
//!
//! Produces a heading-driven tree where headings create nested sections.
//! The output is a JSON object with a `title` (optional) and `body` array
//! of typed nodes (section, paragraph, table, code, formula, list, image, blockquote).

use serde::Serialize;

use crate::types::internal::{ElementKind, InternalDocument};

use super::common::{NestingKind, RenderState, get_language, handle_container_end, is_body_element, is_container_end};

// ============================================================================
// JSON Document Types
// ============================================================================

/// Top-level JSON document.
#[derive(Debug, Serialize)]
pub struct JsonDocument {
    /// Document title, if found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Body content nodes.
    pub body: Vec<JsonNode>,
}

/// A node in the JSON document tree.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum JsonNode {
    /// A heading-delimited section containing nested content.
    #[serde(rename = "section")]
    Section {
        heading: String,
        level: u8,
        body: Vec<JsonNode>,
    },
    /// A text paragraph.
    #[serde(rename = "paragraph")]
    Paragraph { text: String },
    /// A table with headers and rows.
    #[serde(rename = "table")]
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
    },
    /// A code block with optional language.
    #[serde(rename = "code")]
    Code {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    /// A mathematical formula.
    #[serde(rename = "formula")]
    Formula { text: String },
    /// An ordered or unordered list.
    #[serde(rename = "list")]
    List { ordered: bool, items: Vec<String> },
    /// An image reference.
    #[serde(rename = "image")]
    Image {
        #[serde(skip_serializing_if = "Option::is_none")]
        alt: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        src: Option<String>,
    },
    /// A blockquote containing nested content.
    #[serde(rename = "blockquote")]
    Blockquote { body: Vec<JsonNode> },
}

// ============================================================================
// Section Stack
// ============================================================================

/// An open section on the stack, accumulating child nodes.
struct OpenSection {
    heading: String,
    level: u8,
    body: Vec<JsonNode>,
}

// ============================================================================
// List Accumulator
// ============================================================================

/// Tracks an open list being accumulated from ListStart..ListEnd markers.
struct OpenList {
    ordered: bool,
    items: Vec<String>,
}

// ============================================================================
// Renderer
// ============================================================================

/// Render an `InternalDocument` as a JSON tree string.
///
/// Walks the flat element list and builds a heading-driven section tree.
/// Returns a JSON string (always valid JSON).
pub fn render_json(doc: &InternalDocument) -> String {
    let json_doc = build_json_document(doc);
    // serde_json::to_string should not fail on our types (no maps with non-string keys).
    serde_json::to_string(&json_doc).unwrap_or_else(|e| {
        tracing::error!(error = %e, "failed to serialize JSON document");
        r#"{"body":[]}"#.to_string()
    })
}

/// Build the `JsonDocument` from an `InternalDocument`.
fn build_json_document(doc: &InternalDocument) -> JsonDocument {
    let mut title: Option<String> = None;
    let mut section_stack: Vec<OpenSection> = Vec::new();
    let mut root_body: Vec<JsonNode> = Vec::new();
    let mut state = RenderState::default();
    let mut open_list: Option<OpenList> = None;
    let mut open_blockquote: Option<Vec<JsonNode>> = None;

    for elem in &doc.elements {
        if !is_body_element(elem) {
            continue;
        }

        if is_container_end(elem) {
            // Flush list/blockquote if ending
            match elem.kind {
                ElementKind::ListEnd => {
                    if let Some(list) = open_list.take() {
                        let node = JsonNode::List {
                            ordered: list.ordered,
                            items: list.items,
                        };
                        push_to_current(&mut root_body, &mut section_stack, &mut open_blockquote, node);
                    }
                }
                ElementKind::QuoteEnd => {
                    if let Some(bq_body) = open_blockquote.take() {
                        let node = JsonNode::Blockquote { body: bq_body };
                        push_to_current(&mut root_body, &mut section_stack, &mut None, node);
                    }
                }
                _ => {}
            }
            handle_container_end(&elem.kind, &mut state);
            continue;
        }

        match elem.kind {
            ElementKind::Title => {
                if title.is_none() && !elem.text.is_empty() {
                    title = Some(elem.text.clone());
                }
            }

            ElementKind::Heading { level } => {
                // Flush any open list before starting a new section.
                flush_list(&mut open_list, &mut root_body, &mut section_stack, &mut open_blockquote);

                // Close sections at same or deeper level.
                close_sections_to_level(&mut section_stack, &mut root_body, level);

                // Open a new section.
                section_stack.push(OpenSection {
                    heading: elem.text.clone(),
                    level,
                    body: Vec::new(),
                });
            }

            ElementKind::Paragraph => {
                if elem.text.is_empty() {
                    continue;
                }
                // If inside an open list (orphan list items without markers), skip.
                let node = JsonNode::Paragraph {
                    text: elem.text.clone(),
                };
                push_to_current(&mut root_body, &mut section_stack, &mut open_blockquote, node);
            }

            ElementKind::ListStart { ordered } => {
                // Flush any prior list.
                flush_list(&mut open_list, &mut root_body, &mut section_stack, &mut open_blockquote);
                state.push_container(NestingKind::List { ordered, item_count: 0 }, elem.depth);
                open_list = Some(OpenList {
                    ordered,
                    items: Vec::new(),
                });
            }

            ElementKind::ListItem { ordered } => {
                if let Some(ref mut list) = open_list {
                    list.items.push(elem.text.clone());
                } else {
                    // Orphan list item without ListStart — create an inline list node.
                    let node = JsonNode::List {
                        ordered,
                        items: vec![elem.text.clone()],
                    };
                    push_to_current(&mut root_body, &mut section_stack, &mut open_blockquote, node);
                }
            }

            ElementKind::Code => {
                flush_list(&mut open_list, &mut root_body, &mut section_stack, &mut open_blockquote);
                let language = get_language(elem).map(|s| s.to_string());
                let node = JsonNode::Code {
                    text: elem.text.clone(),
                    language,
                };
                push_to_current(&mut root_body, &mut section_stack, &mut open_blockquote, node);
            }

            ElementKind::Formula => {
                flush_list(&mut open_list, &mut root_body, &mut section_stack, &mut open_blockquote);
                let node = JsonNode::Formula {
                    text: elem.text.clone(),
                };
                push_to_current(&mut root_body, &mut section_stack, &mut open_blockquote, node);
            }

            ElementKind::Table { table_index } => {
                flush_list(&mut open_list, &mut root_body, &mut section_stack, &mut open_blockquote);
                if let Some(table) = doc.tables.get(table_index as usize) {
                    let (headers, rows) = if table.cells.is_empty() {
                        (Vec::new(), Vec::new())
                    } else {
                        let headers = table.cells[0].clone();
                        let rows = table.cells[1..].to_vec();
                        (headers, rows)
                    };
                    let node = JsonNode::Table {
                        headers,
                        rows,
                        caption: None,
                    };
                    push_to_current(&mut root_body, &mut section_stack, &mut open_blockquote, node);
                }
            }

            ElementKind::Image { image_index } => {
                flush_list(&mut open_list, &mut root_body, &mut section_stack, &mut open_blockquote);
                let image = doc.images.get(image_index as usize);
                let alt = image.and_then(|img| img.description.clone());
                let src = image.and_then(|img| {
                    if !img.data.is_empty() {
                        Some(format!("image_{}.{}", image_index, img.format))
                    } else {
                        img.source_path.clone()
                    }
                });
                let node = JsonNode::Image { alt, src };
                push_to_current(&mut root_body, &mut section_stack, &mut open_blockquote, node);
            }

            ElementKind::QuoteStart => {
                flush_list(&mut open_list, &mut root_body, &mut section_stack, &mut open_blockquote);
                state.push_container(NestingKind::BlockQuote, elem.depth);
                open_blockquote = Some(Vec::new());
            }

            ElementKind::OcrText { .. } => {
                if !elem.text.is_empty() {
                    let node = JsonNode::Paragraph {
                        text: elem.text.clone(),
                    };
                    push_to_current(&mut root_body, &mut section_stack, &mut open_blockquote, node);
                }
            }

            // Container end markers, page breaks, footnotes, etc. — skip.
            ElementKind::ListEnd
            | ElementKind::QuoteEnd
            | ElementKind::GroupStart
            | ElementKind::GroupEnd
            | ElementKind::PageBreak
            | ElementKind::FootnoteDefinition
            | ElementKind::FootnoteRef
            | ElementKind::Citation
            | ElementKind::Slide { .. }
            | ElementKind::DefinitionTerm
            | ElementKind::DefinitionDescription
            | ElementKind::Admonition
            | ElementKind::RawBlock
            | ElementKind::MetadataBlock => {}
        }
    }

    // Flush any remaining open list.
    flush_list(&mut open_list, &mut root_body, &mut section_stack, &mut open_blockquote);

    // Flush any remaining open blockquote.
    if let Some(bq_body) = open_blockquote.take() {
        let node = JsonNode::Blockquote { body: bq_body };
        push_to_current(&mut root_body, &mut section_stack, &mut None, node);
    }

    // Close all remaining open sections.
    close_sections_to_level(&mut section_stack, &mut root_body, 0);

    JsonDocument { title, body: root_body }
}

/// Push a node to the current target (innermost open section, or root body).
fn push_to_current(
    root_body: &mut Vec<JsonNode>,
    section_stack: &mut [OpenSection],
    open_blockquote: &mut Option<Vec<JsonNode>>,
    node: JsonNode,
) {
    // If inside a blockquote, push there.
    if let Some(bq) = open_blockquote {
        bq.push(node);
        return;
    }
    // Otherwise, push to innermost open section or root.
    if let Some(section) = section_stack.last_mut() {
        section.body.push(node);
    } else {
        root_body.push(node);
    }
}

/// Close all open sections whose level >= `target_level`.
/// Wraps each closed section as a `JsonNode::Section` and appends to its parent.
fn close_sections_to_level(section_stack: &mut Vec<OpenSection>, root_body: &mut Vec<JsonNode>, target_level: u8) {
    while let Some(top) = section_stack.last() {
        if top.level >= target_level {
            let section = section_stack.pop().expect("checked non-empty");
            let node = JsonNode::Section {
                heading: section.heading,
                level: section.level,
                body: section.body,
            };
            // Append to parent section or root.
            if let Some(parent) = section_stack.last_mut() {
                parent.body.push(node);
            } else {
                root_body.push(node);
            }
        } else {
            break;
        }
    }
}

/// Flush an open list accumulator into the current target.
fn flush_list(
    open_list: &mut Option<OpenList>,
    root_body: &mut Vec<JsonNode>,
    section_stack: &mut Vec<OpenSection>,
    open_blockquote: &mut Option<Vec<JsonNode>>,
) {
    if let Some(list) = open_list.take() {
        let node = JsonNode::List {
            ordered: list.ordered,
            items: list.items,
        };
        push_to_current(root_body, section_stack, open_blockquote, node);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::internal_builder::InternalDocumentBuilder;

    #[test]
    fn test_empty_document() {
        let b = InternalDocumentBuilder::new("test");
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(parsed.get("title").is_none() || parsed["title"].is_null());
        assert_eq!(parsed["body"], serde_json::json!([]));
    }

    #[test]
    fn test_single_paragraph() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_paragraph("Hello world", vec![], None, None);
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["body"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["body"][0]["type"], "paragraph");
        assert_eq!(parsed["body"][0]["text"], "Hello world");
    }

    #[test]
    fn test_heading_creates_section() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_heading(1, "Chapter 1", None, None);
        b.push_paragraph("Chapter content", vec![], None, None);
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["body"].as_array().unwrap().len(), 1);
        let section = &parsed["body"][0];
        assert_eq!(section["type"], "section");
        assert_eq!(section["heading"], "Chapter 1");
        assert_eq!(section["level"], 1);
        assert_eq!(section["body"][0]["type"], "paragraph");
        assert_eq!(section["body"][0]["text"], "Chapter content");
    }

    #[test]
    fn test_nested_sections() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_heading(1, "Chapter 1", None, None);
        b.push_paragraph("Intro", vec![], None, None);
        b.push_heading(2, "Section 1.1", None, None);
        b.push_paragraph("Sub content", vec![], None, None);
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let section = &parsed["body"][0];
        assert_eq!(section["type"], "section");
        assert_eq!(section["heading"], "Chapter 1");
        assert_eq!(section["level"], 1);
        // Body should have: paragraph "Intro" and a nested section
        assert_eq!(section["body"].as_array().unwrap().len(), 2);
        assert_eq!(section["body"][0]["type"], "paragraph");
        let sub_section = &section["body"][1];
        assert_eq!(sub_section["type"], "section");
        assert_eq!(sub_section["heading"], "Section 1.1");
        assert_eq!(sub_section["level"], 2);
        assert_eq!(sub_section["body"][0]["text"], "Sub content");
    }

    #[test]
    fn test_table_in_json() {
        let mut b = InternalDocumentBuilder::new("test");
        let cells = vec![
            vec!["A".to_string(), "B".to_string()],
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string(), "4".to_string()],
        ];
        b.push_table_from_cells(&cells, None, None);
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let table = &parsed["body"][0];
        assert_eq!(table["type"], "table");
        assert_eq!(table["headers"], serde_json::json!(["A", "B"]));
        assert_eq!(table["rows"], serde_json::json!([["1", "2"], ["3", "4"]]));
    }

    #[test]
    fn test_code_block() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_code("print('hello')", Some("python"), None, None);
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let code = &parsed["body"][0];
        assert_eq!(code["type"], "code");
        assert_eq!(code["text"], "print('hello')");
        assert_eq!(code["language"], "python");
    }

    #[test]
    fn test_code_block_no_language() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_code("some code", None, None, None);
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let code = &parsed["body"][0];
        assert_eq!(code["type"], "code");
        assert_eq!(code["text"], "some code");
        // language should be absent (skip_serializing_if)
        assert!(code.get("language").is_none() || code["language"].is_null());
    }

    #[test]
    fn test_list() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(false);
        b.push_list_item("Item 1", false, vec![], None, None);
        b.push_list_item("Item 2", false, vec![], None, None);
        b.end_list();
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let list = &parsed["body"][0];
        assert_eq!(list["type"], "list");
        assert_eq!(list["ordered"], false);
        assert_eq!(list["items"], serde_json::json!(["Item 1", "Item 2"]));
    }

    #[test]
    fn test_ordered_list() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_list(true);
        b.push_list_item("First", true, vec![], None, None);
        b.push_list_item("Second", true, vec![], None, None);
        b.end_list();
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let list = &parsed["body"][0];
        assert_eq!(list["type"], "list");
        assert_eq!(list["ordered"], true);
        assert_eq!(list["items"], serde_json::json!(["First", "Second"]));
    }

    #[test]
    fn test_formula() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_formula("E = mc^2", None, None);
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let formula = &parsed["body"][0];
        assert_eq!(formula["type"], "formula");
        assert_eq!(formula["text"], "E = mc^2");
    }

    #[test]
    fn test_title_from_title_element() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_title("My Document", None, None);
        b.push_paragraph("Content", vec![], None, None);
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["title"], "My Document");
        // Title should not appear as a body node.
        assert_eq!(parsed["body"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["body"][0]["type"], "paragraph");
    }

    #[test]
    fn test_blockquote() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_quote_start();
        b.push_paragraph("Quoted text", vec![], None, None);
        b.push_quote_end();
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        let bq = &parsed["body"][0];
        assert_eq!(bq["type"], "blockquote");
        assert_eq!(bq["body"][0]["type"], "paragraph");
        assert_eq!(bq["body"][0]["text"], "Quoted text");
    }

    #[test]
    fn test_sibling_sections() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_heading(1, "Chapter 1", None, None);
        b.push_paragraph("Content 1", vec![], None, None);
        b.push_heading(1, "Chapter 2", None, None);
        b.push_paragraph("Content 2", vec![], None, None);
        let doc = b.build();
        let json_str = render_json(&doc);
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["body"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["body"][0]["heading"], "Chapter 1");
        assert_eq!(parsed["body"][1]["heading"], "Chapter 2");
    }

    #[test]
    fn test_valid_json_output() {
        let mut b = InternalDocumentBuilder::new("test");
        b.push_title("Test", None, None);
        b.push_heading(1, "H1", None, None);
        b.push_paragraph("Para", vec![], None, None);
        b.push_heading(2, "H2", None, None);
        b.push_code("code", Some("rust"), None, None);
        b.push_formula("x^2", None, None);
        let doc = b.build();
        let json_str = render_json(&doc);
        // Must be valid JSON.
        let result: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
        assert!(result.is_ok(), "JSON output is not valid: {}", json_str);
    }
}
