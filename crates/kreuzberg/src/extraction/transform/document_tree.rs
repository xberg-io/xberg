//! Transform extraction results into a structured `DocumentStructure` tree.
//!
//! This module converts the flat/page-based extraction output into a hierarchical
//! document tree with heading-driven section nesting, table grids, and content
//! layer classification.

use crate::types::document_structure::GridCell;
use crate::types::{
    BoundingBox, ContentLayer, DocumentNode, DocumentStructure, ExtractionResult, NodeContent, NodeId, NodeIndex,
    TableGrid,
};

use super::elements::detect_list_items;
use super::types::ListType;

/// Transform an `ExtractionResult` into a `DocumentStructure`.
///
/// Processes pages (if available) or unified content to build a hierarchical tree:
/// - Heading-driven section nesting via `Group` nodes
/// - Table conversion from `Vec<Vec<String>>` to `TableGrid`
/// - List detection and grouping into `List` containers
/// - Image and page break nodes
/// - Body/furniture content layer classification
///
/// The resulting structure is validated before returning.
pub fn transform_to_document_structure(result: &ExtractionResult) -> DocumentStructure {
    let mut doc = DocumentStructure::with_capacity(estimate_node_count(result));
    let mut section_stack: Vec<(u8, NodeIndex)> = Vec::new();

    if let Some(ref pages) = result.pages {
        for page in pages {
            let page_num = page.page_number as u32;
            // Reset section stack for each new page (prevents cross-page nesting)
            section_stack.clear();

            // Process hierarchy blocks (headings) first — they create section groups
            if let Some(ref hierarchy) = page.hierarchy {
                for block in &hierarchy.blocks {
                    let level = parse_heading_level(&block.level);
                    let bbox = block.bbox.map(BoundingBox::from);

                    if let Some(level) = level {
                        push_heading_group(&mut doc, &mut section_stack, level, &block.text, Some(page_num), bbox);
                    } else if !block.text.trim().is_empty() {
                        push_content_node(
                            &mut doc,
                            &section_stack,
                            NodeContent::Paragraph {
                                text: block.text.clone(),
                            },
                            Some(page_num),
                            bbox,
                        );
                    }
                }
            }

            // Process tables
            for table_arc in &page.tables {
                let table = table_arc.as_ref();
                let grid = table_cells_to_grid(&table.cells);
                push_content_node(
                    &mut doc,
                    &section_stack,
                    NodeContent::Table { grid },
                    Some(page_num),
                    None,
                );
            }

            // Process images
            for (idx, image_arc) in page.images.iter().enumerate() {
                let image = image_arc.as_ref();
                push_content_node(
                    &mut doc,
                    &section_stack,
                    NodeContent::Image {
                        description: image.description.clone(),
                        image_index: Some(idx as u32),
                        src: None,
                    },
                    Some(page_num),
                    None,
                );
            }

            // Process page content text (paragraphs, list items) only if no hierarchy blocks
            // (hierarchy blocks already contain the structured body content)
            let has_hierarchy_blocks = page.hierarchy.as_ref().is_some_and(|h| !h.blocks.is_empty());
            if !has_hierarchy_blocks {
                process_text_content(&mut doc, &section_stack, &page.content, Some(page_num));
            }

            // Add PageBreak between pages (not after last)
            if result.pages.as_ref().is_some_and(|all| page.page_number < all.len()) {
                push_content_node(&mut doc, &section_stack, NodeContent::PageBreak, Some(page_num), None);
            }
        }
    } else {
        // No pages — process unified content
        process_text_content(&mut doc, &section_stack, &result.content, Some(1));

        // Process global tables
        for table in &result.tables {
            let grid = table_cells_to_grid(&table.cells);
            push_content_node(
                &mut doc,
                &section_stack,
                NodeContent::Table { grid },
                Some(table.page_number as u32),
                None,
            );
        }

        // Process global images
        if let Some(ref images) = result.images {
            for (idx, image) in images.iter().enumerate() {
                let page_num = image.page_number.map(|p| p as u32).unwrap_or(1);
                push_content_node(
                    &mut doc,
                    &section_stack,
                    NodeContent::Image {
                        description: image.description.clone(),
                        image_index: Some(idx as u32),
                        src: None,
                    },
                    Some(page_num),
                    None,
                );
            }
        }
    }

    // Validation — debug assert in dev, silent in release
    debug_assert!(
        doc.validate().is_ok(),
        "DocumentStructure validation failed: {:?}",
        doc.validate()
    );

    doc
}

// ============================================================================
// Section Nesting
// ============================================================================

/// Push a heading-driven `Group` node onto the tree, managing the section stack.
fn push_heading_group(
    doc: &mut DocumentStructure,
    section_stack: &mut Vec<(u8, NodeIndex)>,
    level: u8,
    text: &str,
    page: Option<u32>,
    bbox: Option<BoundingBox>,
) {
    // Pop sections at same or deeper level
    while section_stack.last().is_some_and(|(l, _)| *l >= level) {
        section_stack.pop();
    }

    let content = NodeContent::Group {
        label: None,
        heading_level: Some(level),
        heading_text: Some(text.to_string()),
    };

    let index = doc.len() as u32;
    let node = DocumentNode {
        id: NodeId::generate("group", text, page, index),
        content,
        parent: None,
        children: vec![],
        content_layer: ContentLayer::Body,
        page,
        page_end: None,
        bbox,
        annotations: vec![],
        attributes: None,
    };

    let group_idx = doc.push_node(node);

    // Wire parent → child using add_child
    if let Some((_, parent_idx)) = section_stack.last() {
        doc.add_child(*parent_idx, group_idx);
    }

    // Insert a Heading child node inside the Group so downstream consumers
    // can find headings via NodeContent::Heading (matches DOCX builder behavior).
    let heading_index = doc.len() as u32;
    let heading_node = DocumentNode {
        id: NodeId::generate("heading", text, page, heading_index),
        content: NodeContent::Heading {
            level,
            text: text.to_string(),
        },
        parent: Some(group_idx),
        children: vec![],
        content_layer: ContentLayer::Body,
        page,
        page_end: None,
        bbox,
        annotations: vec![],
        attributes: None,
    };
    let heading_idx = doc.push_node(heading_node);
    doc.nodes[group_idx.0 as usize].children.push(heading_idx);

    section_stack.push((level, group_idx));
}

/// Push a content node as a child of the current section (or root if no section).
/// PageBreak nodes are always added as root-level nodes (no parent).
fn push_content_node(
    doc: &mut DocumentStructure,
    section_stack: &[(u8, NodeIndex)],
    content: NodeContent,
    page: Option<u32>,
    bbox: Option<BoundingBox>,
) -> NodeIndex {
    let node_type = content.node_type_str();
    let text_for_id = content.text().unwrap_or("");
    let is_page_break = matches!(content, NodeContent::PageBreak);

    let index = doc.len() as u32;
    let node = DocumentNode {
        id: NodeId::generate(node_type, text_for_id, page, index),
        content,
        parent: None,
        children: vec![],
        content_layer: ContentLayer::Body,
        page,
        page_end: None,
        bbox,
        annotations: vec![],
        attributes: None,
    };

    let node_idx = doc.push_node(node);

    // Wire parent → child using add_child, EXCEPT for PageBreak nodes which are always root-level
    if !is_page_break && let Some((_, parent_idx)) = section_stack.last() {
        doc.add_child(*parent_idx, node_idx);
    }

    node_idx
}

// ============================================================================
// Text Content Processing
// ============================================================================

/// Process text content into paragraphs and list items.
fn process_text_content(
    doc: &mut DocumentStructure,
    section_stack: &[(u8, NodeIndex)],
    content: &str,
    page: Option<u32>,
) {
    if content.trim().is_empty() {
        return;
    }

    let list_items = detect_list_items(content);

    if list_items.is_empty() {
        // No list items — split into paragraphs
        add_paragraphs(doc, section_stack, content, page);
        return;
    }

    let mut current_offset = 0;

    // Group consecutive list items by type
    let mut list_groups: Vec<(ListType, Vec<(usize, usize)>)> = Vec::new();

    for item in &list_items {
        // Add paragraphs before list items
        if current_offset < item.byte_start {
            let text_before = &content[current_offset..item.byte_start];
            add_paragraphs(doc, section_stack, text_before, page);
        }

        // Group consecutive same-type list items
        if list_groups.last().is_some_and(|(t, _)| *t == item.list_type) {
            if let Some(group) = list_groups.last_mut() {
                group.1.push((item.byte_start, item.byte_end));
            }
        } else {
            list_groups.push((item.list_type, vec![(item.byte_start, item.byte_end)]));
        }

        current_offset = item.byte_end;
    }

    // Emit list groups
    for (list_type, items) in &list_groups {
        let ordered = matches!(list_type, ListType::Numbered | ListType::Lettered);

        // Create List container
        let list_content = NodeContent::List { ordered };
        let list_index = doc.len() as u32;
        let list_node = DocumentNode {
            id: NodeId::generate("list", &format!("{:?}_{}", list_type, items.len()), page, list_index),
            content: list_content,
            parent: None,
            children: vec![],
            content_layer: ContentLayer::Body,
            page,
            page_end: None,
            bbox: None,
            annotations: vec![],
            attributes: None,
        };
        let list_idx = doc.push_node(list_node);

        // Wire parent → list using add_child()
        if let Some((_, parent_idx)) = section_stack.last() {
            doc.add_child(*parent_idx, list_idx);
        }

        // Add list items as children
        for (start, end) in items {
            let item_text = content[*start..*end].trim();
            // Strip list marker
            let clean_text = strip_list_marker(item_text);

            let item_content = NodeContent::ListItem {
                text: clean_text.to_string(),
            };
            let item_index = doc.len() as u32;
            let item_node = DocumentNode {
                id: NodeId::generate("list_item", clean_text, page, item_index),
                content: item_content,
                parent: Some(list_idx),
                children: vec![],
                content_layer: ContentLayer::Body,
                page,
                page_end: None,
                bbox: None,
                annotations: vec![],
                attributes: None,
            };
            let item_idx = doc.push_node(item_node);
            doc.nodes[list_idx.0 as usize].children.push(item_idx);
        }
    }

    // Add remaining text after last list item
    if current_offset < content.len() {
        let text_after = &content[current_offset..];
        add_paragraphs(doc, section_stack, text_after, page);
    }
}

/// Add paragraphs split on double newlines.
///
/// Detects markdown heading markers (`#` through `######`) and creates
/// heading groups instead of plain paragraphs.
fn add_paragraphs(doc: &mut DocumentStructure, section_stack: &[(u8, NodeIndex)], text: &str, page: Option<u32>) {
    let mut local_stack: Vec<(u8, NodeIndex)> = section_stack.to_vec();

    for paragraph in text.split("\n\n").filter(|p| !p.trim().is_empty()) {
        let para_text = paragraph.trim();
        if para_text.is_empty() {
            continue;
        }

        if let Some((level, heading_text)) = parse_markdown_heading(para_text) {
            push_heading_group(doc, &mut local_stack, level, heading_text, page, None);
        } else {
            push_content_node(
                doc,
                &local_stack,
                NodeContent::Paragraph {
                    text: para_text.to_string(),
                },
                page,
                None,
            );
        }
    }
}

/// Parse a markdown heading line into (level, text).
fn parse_markdown_heading(line: &str) -> Option<(u8, &str)> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let text = rest.trim();
    if text.is_empty() {
        return None;
    }
    Some((hashes as u8, text))
}

// ============================================================================
// Table Conversion
// ============================================================================

/// Convert a `Vec<Vec<String>>` cell grid into a `TableGrid`.
fn table_cells_to_grid(cells: &[Vec<String>]) -> TableGrid {
    let rows = cells.len() as u32;
    let cols = cells.iter().map(|r| r.len()).max().unwrap_or(0) as u32;

    let mut grid_cells = Vec::new();
    for (row_idx, row) in cells.iter().enumerate() {
        for (col_idx, cell_content) in row.iter().enumerate() {
            grid_cells.push(GridCell {
                content: cell_content.clone(),
                row: row_idx as u32,
                col: col_idx as u32,
                row_span: 1,
                col_span: 1,
                is_header: row_idx == 0, // First row assumed header
                bbox: None,
            });
        }
    }

    TableGrid {
        rows,
        cols,
        cells: grid_cells,
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Parse heading level from "h1"-"h6" strings. Returns None for "body" or unknown.
fn parse_heading_level(level: &str) -> Option<u8> {
    match level {
        "h1" => Some(1),
        "h2" => Some(2),
        "h3" => Some(3),
        "h4" => Some(4),
        "h5" => Some(5),
        "h6" => Some(6),
        _ => None,
    }
}

/// Strip list marker from text (e.g., "- item" → "item", "1. item" → "item").
fn strip_list_marker(text: &str) -> &str {
    let trimmed = text.trim_start();
    // Bullet markers
    for prefix in &["- ", "* ", "• "] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest;
        }
    }
    // Numbered markers (e.g., "1. ", "12. ")
    if let Some(dot_pos) = trimmed.find('.') {
        let prefix = &trimmed[..dot_pos];
        if prefix.chars().all(|c| c.is_ascii_digit())
            && dot_pos > 0
            && dot_pos < 3
            && let Some(rest) = trimmed[dot_pos + 1..].strip_prefix(' ')
        {
            return rest;
        }
        // Lettered markers (e.g., "a. ", "B. ")
        if prefix.len() == 1
            && prefix.chars().all(|c| c.is_alphabetic())
            && let Some(rest) = trimmed[dot_pos + 1..].strip_prefix(' ')
        {
            return rest;
        }
    }
    trimmed
}

/// Estimate node count for pre-allocation.
fn estimate_node_count(result: &ExtractionResult) -> usize {
    let base = if let Some(ref pages) = result.pages {
        pages
            .iter()
            .map(|p| {
                let hierarchy_count = p.hierarchy.as_ref().map(|h| h.blocks.len()).unwrap_or(0);
                let table_count = p.tables.len();
                let image_count = p.images.len();
                // Rough estimate: hierarchy blocks + tables + images + ~5 paragraphs per page
                hierarchy_count + table_count + image_count + 5
            })
            .sum()
    } else {
        // Estimate from content length
        (result.content.len() / 200).max(4)
    };

    base + result.tables.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ExtractionResult, HierarchicalBlock, Metadata, PageContent, PageHierarchy, Table};
    use std::borrow::Cow;

    #[allow(dead_code)]
    fn test_metadata() -> Metadata {
        Metadata::default()
    }

    fn test_result(content: &str) -> ExtractionResult {
        ExtractionResult {
            content: content.to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        }
    }

    #[test]
    fn test_simple_paragraphs() {
        let result = test_result("First paragraph.\n\nSecond paragraph.\n\nThird paragraph.");
        let doc = transform_to_document_structure(&result);

        assert!(doc.validate().is_ok());
        assert_eq!(doc.len(), 3);

        // All root-level body nodes
        let body: Vec<_> = doc.body_roots().collect();
        assert_eq!(body.len(), 3);
    }

    #[test]
    fn test_list_detection() {
        let result = test_result("- First item\n- Second item\n- Third item");
        let doc = transform_to_document_structure(&result);

        assert!(doc.validate().is_ok());

        // Should have 1 List container + 3 ListItem children = 4 nodes
        assert_eq!(doc.len(), 4);

        // Root should be the List container
        let roots: Vec<_> = doc.body_roots().collect();
        assert_eq!(roots.len(), 1);
        match &roots[0].1.content {
            NodeContent::List { ordered } => assert!(!ordered),
            _ => panic!("Expected List node"),
        }

        // List should have 3 children
        assert_eq!(doc.nodes[0].children.len(), 3);
    }

    #[test]
    fn test_heading_driven_sections() {
        let result = ExtractionResult {
            pages: Some(vec![PageContent {
                page_number: 1,
                content: "Body text under heading.".to_string(),
                tables: vec![],
                images: vec![],
                hierarchy: Some(PageHierarchy {
                    block_count: 3,
                    blocks: vec![
                        HierarchicalBlock {
                            text: "Main Title".to_string(),
                            font_size: 24.0,
                            level: "h1".to_string(),
                            bbox: Some((10.0, 20.0, 500.0, 50.0)),
                        },
                        HierarchicalBlock {
                            text: "Subtitle".to_string(),
                            font_size: 18.0,
                            level: "h2".to_string(),
                            bbox: None,
                        },
                        HierarchicalBlock {
                            text: "Body text from hierarchy.".to_string(),
                            font_size: 12.0,
                            level: "body".to_string(),
                            bbox: None,
                        },
                    ],
                }),
                is_blank: None,
                layout_regions: None,
            }]),
            ..test_result("")
        };

        let doc = transform_to_document_structure(&result);
        assert!(doc.validate().is_ok());

        // Root: H1 Group
        let roots: Vec<_> = doc.body_roots().collect();
        assert_eq!(roots.len(), 1);
        match &roots[0].1.content {
            NodeContent::Group {
                heading_level,
                heading_text,
                ..
            } => {
                assert_eq!(*heading_level, Some(1));
                assert_eq!(heading_text.as_deref(), Some("Main Title"));
            }
            _ => panic!("Expected Group node"),
        }

        // H1 should have H2 Group as child
        let h1_children = &doc.nodes[0].children;
        assert!(!h1_children.is_empty());
    }

    #[test]
    fn test_multiple_h1_sections() {
        let result = ExtractionResult {
            pages: Some(vec![PageContent {
                page_number: 1,
                content: String::new(),
                tables: vec![],
                images: vec![],
                hierarchy: Some(PageHierarchy {
                    block_count: 2,
                    blocks: vec![
                        HierarchicalBlock {
                            text: "First Section".to_string(),
                            font_size: 24.0,
                            level: "h1".to_string(),
                            bbox: None,
                        },
                        HierarchicalBlock {
                            text: "Second Section".to_string(),
                            font_size: 24.0,
                            level: "h1".to_string(),
                            bbox: None,
                        },
                    ],
                }),
                is_blank: None,
                layout_regions: None,
            }]),
            ..test_result("")
        };

        let doc = transform_to_document_structure(&result);
        assert!(doc.validate().is_ok());

        // Two separate root-level H1 groups
        let roots: Vec<_> = doc.body_roots().collect();
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn test_skipped_heading_levels() {
        // H1 → H3 (no H2)
        let result = ExtractionResult {
            pages: Some(vec![PageContent {
                page_number: 1,
                content: String::new(),
                tables: vec![],
                images: vec![],
                hierarchy: Some(PageHierarchy {
                    block_count: 2,
                    blocks: vec![
                        HierarchicalBlock {
                            text: "Title".to_string(),
                            font_size: 24.0,
                            level: "h1".to_string(),
                            bbox: None,
                        },
                        HierarchicalBlock {
                            text: "Subsub".to_string(),
                            font_size: 14.0,
                            level: "h3".to_string(),
                            bbox: None,
                        },
                    ],
                }),
                is_blank: None,
                layout_regions: None,
            }]),
            ..test_result("")
        };

        let doc = transform_to_document_structure(&result);
        assert!(doc.validate().is_ok());

        // H1 Group should have 2 children: Heading("Title") + H3 Group
        assert_eq!(doc.nodes[0].children.len(), 2);
        let heading_idx = doc.nodes[0].children[0];
        assert!(matches!(
            doc.nodes[heading_idx.0 as usize].content,
            NodeContent::Heading { level: 1, .. }
        ));
        let h3_idx = doc.nodes[0].children[1];
        assert_eq!(doc.nodes[h3_idx.0 as usize].parent, Some(NodeIndex(0)));
    }

    #[test]
    fn test_no_headings_flat_paragraphs() {
        let result = test_result("Paragraph one.\n\nParagraph two.");
        let doc = transform_to_document_structure(&result);

        assert!(doc.validate().is_ok());
        assert_eq!(doc.len(), 2);

        // All nodes should be root-level (no parent)
        for node in &doc.nodes {
            assert!(node.parent.is_none());
        }
    }

    #[test]
    fn test_table_grid_conversion() {
        let result = ExtractionResult {
            tables: vec![Table {
                cells: vec![
                    vec!["Name".to_string(), "Age".to_string()],
                    vec!["Alice".to_string(), "30".to_string()],
                ],
                markdown: "| Name | Age |\n|---|---|\n| Alice | 30 |".to_string(),
                page_number: 1,
                bounding_box: None,
            }],
            ..test_result("Some content")
        };

        let doc = transform_to_document_structure(&result);
        assert!(doc.validate().is_ok());

        // Find Table node
        let table_node = doc
            .nodes
            .iter()
            .find(|n| matches!(n.content, NodeContent::Table { .. }));
        assert!(table_node.is_some());

        if let NodeContent::Table { ref grid } = table_node.unwrap().content {
            assert_eq!(grid.rows, 2);
            assert_eq!(grid.cols, 2);
            assert_eq!(grid.cells.len(), 4);
            assert!(grid.cells[0].is_header); // First row is header
            assert!(!grid.cells[2].is_header);
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        let result = test_result("Hello world.\n\n- Item 1\n- Item 2");
        let doc = transform_to_document_structure(&result);

        let json = serde_json::to_string(&doc).expect("serialize");
        let deserialized: DocumentStructure = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.len(), doc.len());
        assert!(deserialized.validate().is_ok());
    }

    #[test]
    fn test_strip_list_marker() {
        assert_eq!(strip_list_marker("- item"), "item");
        assert_eq!(strip_list_marker("* item"), "item");
        assert_eq!(strip_list_marker("• item"), "item");
        assert_eq!(strip_list_marker("1. item"), "item");
        assert_eq!(strip_list_marker("a. item"), "item");
        assert_eq!(strip_list_marker("plain text"), "plain text");
    }

    #[test]
    fn test_empty_content() {
        let result = test_result("");
        let doc = transform_to_document_structure(&result);
        assert!(doc.validate().is_ok());
        assert!(doc.is_empty());
    }
}
