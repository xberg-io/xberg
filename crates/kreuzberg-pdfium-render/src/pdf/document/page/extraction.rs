//! Semantic content extraction from PDF pages.
//!
//! Provides a unified API for extracting structured content from PDF pages.
//! Tagged PDFs use the structure tree for semantic extraction; untagged PDFs
//! fall back to heuristic analysis of text objects.

use crate::error::PdfiumError;
use crate::pdf::document::page::PdfPage;
use crate::pdf::document::page::object::PdfPageObjectCommon;
use crate::pdf::document::page::objects::common::PdfPageObjectsCommon;
use crate::pdf::document::page::struct_element::{PdfStructElement, PdfStructElementType};
use crate::pdf::font::PdfFontWeight;
use crate::pdf::points::PdfPoints;
use crate::pdf::rect::PdfRect;
use std::collections::HashMap;

/// The method used for extracting content from a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageExtractionMethod {
    /// Content was extracted using the PDF structure tree (tagged PDF).
    StructureTree,
    /// Content was extracted using heuristic analysis of text objects.
    Heuristic,
}

/// The semantic role of an extracted content block.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentRole {
    /// A heading at the given level (1-6).
    Heading { level: u8 },
    /// A paragraph of body text.
    Paragraph,
    /// A list item, optionally with its label (bullet, number, etc.).
    ListItem { label: Option<String> },
    /// A table cell at the given row and column.
    TableCell { row: usize, col: usize, is_header: bool },
    /// A figure or image, optionally with alternative text.
    Figure { alt_text: Option<String> },
    /// A caption for a figure or table.
    Caption,
    /// A code block.
    Code,
    /// A block quote.
    BlockQuote,
    /// A link with optional URL.
    Link { url: Option<String> },
    /// Any other role not covered above.
    Other(String),
}

/// A single block of extracted content with its semantic role and properties.
#[derive(Debug, Clone)]
pub struct ExtractedBlock {
    /// The semantic role of this block.
    pub role: ContentRole,
    /// The text content of this block.
    pub text: String,
    /// The bounding rectangle, if available.
    pub bounds: Option<PdfRect>,
    /// The font size in points, if available.
    pub font_size: Option<f32>,
    /// Whether the text is bold.
    pub is_bold: bool,
    /// Whether the text is italic.
    pub is_italic: bool,
    /// Whether the font is monospace (fixed-pitch).
    pub is_monospace: bool,
    /// Child blocks (e.g., cells within a table row).
    pub children: Vec<ExtractedBlock>,
}

/// The result of extracting content from a page.
#[derive(Debug)]
pub struct PageExtraction {
    /// Which extraction method was used.
    pub method: PageExtractionMethod,
    /// The extracted content blocks in reading order.
    pub blocks: Vec<ExtractedBlock>,
}

/// Extracts structured content from a PDF page.
///
/// Tries the structure tree first (for tagged PDFs). Falls back to heuristic
/// extraction if the page is untagged or the structure tree yields insufficient
/// content.
pub fn extract_page_content(page: &PdfPage<'_>) -> Result<PageExtraction, PdfiumError> {
    // Try structure tree extraction first.
    if let Some(extraction) = extract_via_structure_tree(page)? {
        return Ok(extraction);
    }

    // Fall back to heuristic extraction.
    extract_via_heuristics(page)
}

/// Attempts extraction using the PDF structure tree.
/// Returns `None` if the page is untagged or the tree has no useful content.
fn extract_via_structure_tree(page: &PdfPage<'_>) -> Result<Option<PageExtraction>, PdfiumError> {
    let tree = match page.struct_tree() {
        Some(tree) => tree,
        None => return Ok(None),
    };

    if tree.children_count() == 0 {
        return Ok(None);
    }

    // Build maps from MCID → text content and MCID → style in a single pass
    // over all page objects (avoids iterating the object list twice).
    let (mcid_text_map, mcid_style_map) = build_mcid_maps(page)?;

    if mcid_text_map.is_empty() {
        return Ok(None);
    }

    // Walk the structure tree top-down (NOT using tree.iter() which would
    // cause double-processing since extract_element_block also recurses children).
    let mut blocks = Vec::new();
    let mut resolved = false;

    for child in tree.children() {
        if let Some(block) = extract_element_block(&child, &mcid_text_map, &mcid_style_map)
            && (!block.text.is_empty() || !block.children.is_empty())
        {
            resolved = true;
            blocks.push(block);
        }
    }

    if !resolved {
        return Ok(None);
    }

    // Flatten nested blocks that don't add semantic value (Document, Part, Div wrappers).
    let blocks = flatten_structural_wrappers(blocks);

    Ok(Some(PageExtraction {
        method: PageExtractionMethod::StructureTree,
        blocks,
    }))
}

/// Style information for a text object.
#[derive(Debug, Clone)]
struct TextStyle {
    font_size: f32,
    is_bold: bool,
    is_italic: bool,
    is_monospace: bool,
    bounds: Option<PdfRect>,
}

/// Builds both MCID → text and MCID → style maps in a single pass over page objects.
///
/// Uses the pre-loaded text page handle to avoid calling `FPDFText_LoadPage` per object
/// (which was the root cause of multi-second extraction times on complex pages).
type McidMaps = (HashMap<i32, String>, HashMap<i32, TextStyle>);

fn build_mcid_maps(page: &PdfPage<'_>) -> Result<McidMaps, PdfiumError> {
    let objects = page.objects();
    let text_page = page.text()?;

    let mut text_map: HashMap<i32, String> = HashMap::new();
    let mut style_map: HashMap<i32, TextStyle> = HashMap::new();

    for i in 0..objects.len() {
        let object = objects.get(i)?;

        if let Some(text_obj) = object.as_text_object()
            && let Some(mcid) = object.marked_content_id()
        {
            // Text: use pre-loaded text page handle (fast path).
            let text = text_page.for_object(text_obj);
            if !text.is_empty() {
                text_map
                    .entry(mcid)
                    .and_modify(|existing| existing.push_str(&text))
                    .or_insert(text);
            }

            // Style: only store the first text object's style per MCID.
            style_map.entry(mcid).or_insert_with(|| {
                let font = text_obj.font();
                let is_bold = font.weight().ok().is_some_and(|w| {
                    matches!(
                        w,
                        PdfFontWeight::Weight700Bold | PdfFontWeight::Weight800 | PdfFontWeight::Weight900
                    )
                }) || font.is_bold_reenforced()
                    || font.name().to_ascii_lowercase().contains("bold");
                let is_italic = font.is_italic();
                let is_monospace = font.is_fixed_pitch();
                let font_size = text_obj.scaled_font_size().value;
                let bounds = object.bounds().ok().map(|qp| qp.to_rect());
                TextStyle {
                    font_size,
                    is_bold,
                    is_italic,
                    is_monospace,
                    bounds,
                }
            });
        }
    }

    Ok((text_map, style_map))
}

/// Extracts a single block from a structure element, resolving text via MCID mapping.
fn extract_element_block(
    element: &PdfStructElement<'_>,
    mcid_text_map: &HashMap<i32, String>,
    mcid_style_map: &HashMap<i32, TextStyle>,
) -> Option<ExtractedBlock> {
    let element_type = element.element_type();

    // Skip pure structural wrappers that don't carry content directly —
    // their children will be processed separately by the tree iterator.
    // However, we still process them if they have actual text or alt text.
    let role = element_type_to_role(&element_type, element);

    // Collect text from all MCIDs associated with this element.
    let mcids = element.all_marked_content_ids();
    let mut text_parts: Vec<&str> = Vec::new();
    let mut style: Option<&TextStyle> = None;

    for mcid in &mcids {
        if let Some(t) = mcid_text_map.get(mcid) {
            text_parts.push(t);
        }
        if style.is_none() {
            style = mcid_style_map.get(mcid);
        }
    }

    // Also check for actual text and alt text on the element itself.
    let actual_text = element.actual_text();
    let alt_text = element.alt_text();

    let text = if !text_parts.is_empty() {
        text_parts.join("")
    } else if let Some(ref at) = actual_text {
        at.clone()
    } else if let Some(ref alt) = alt_text {
        alt.clone()
    } else {
        String::new()
    };

    // Process children for composite elements (tables, lists).
    let children = extract_children_blocks(element, mcid_text_map, mcid_style_map);

    // Skip elements with no text and no children.
    if text.is_empty() && children.is_empty() {
        return None;
    }

    Some(ExtractedBlock {
        role,
        text,
        bounds: style.and_then(|s| s.bounds),
        font_size: style.map(|s| s.font_size),
        is_bold: style.is_some_and(|s| s.is_bold),
        is_italic: style.is_some_and(|s| s.is_italic),
        is_monospace: style.is_some_and(|s| s.is_monospace),
        children,
    })
}

/// Extracts blocks from the direct children of a structure element.
fn extract_children_blocks(
    element: &PdfStructElement<'_>,
    mcid_text_map: &HashMap<i32, String>,
    mcid_style_map: &HashMap<i32, TextStyle>,
) -> Vec<ExtractedBlock> {
    let mut children = Vec::new();
    for child in element.children() {
        if let Some(block) = extract_element_block(&child, mcid_text_map, mcid_style_map)
            && (!block.text.is_empty() || !block.children.is_empty())
        {
            children.push(block);
        }
    }
    children
}

/// Maps a PDF structure element type to a semantic content role.
fn element_type_to_role(element_type: &PdfStructElementType, element: &PdfStructElement<'_>) -> ContentRole {
    match element_type {
        PdfStructElementType::H => ContentRole::Heading { level: 1 },
        PdfStructElementType::H1 => ContentRole::Heading { level: 1 },
        PdfStructElementType::H2 => ContentRole::Heading { level: 2 },
        PdfStructElementType::H3 => ContentRole::Heading { level: 3 },
        PdfStructElementType::H4 => ContentRole::Heading { level: 4 },
        PdfStructElementType::H5 => ContentRole::Heading { level: 5 },
        PdfStructElementType::H6 => ContentRole::Heading { level: 6 },
        PdfStructElementType::P | PdfStructElementType::Span => ContentRole::Paragraph,
        PdfStructElementType::LI => {
            // Try to find a label child.
            let label = find_child_text_by_type(element, &PdfStructElementType::Lbl);
            ContentRole::ListItem { label }
        }
        PdfStructElementType::Figure => {
            let alt = element.alt_text();
            ContentRole::Figure { alt_text: alt }
        }
        PdfStructElementType::Caption => ContentRole::Caption,
        PdfStructElementType::Code => ContentRole::Code,
        PdfStructElementType::BlockQuote => ContentRole::BlockQuote,
        PdfStructElementType::Link => {
            // Try to extract URL from element attributes.
            let url = element.string_attribute("O");
            ContentRole::Link { url }
        }
        PdfStructElementType::TD => ContentRole::TableCell {
            row: 0,
            col: 0,
            is_header: false,
        },
        PdfStructElementType::TH => ContentRole::TableCell {
            row: 0,
            col: 0,
            is_header: true,
        },
        _ => {
            let type_str = element.element_type_raw().unwrap_or_default();
            ContentRole::Other(type_str)
        }
    }
}

/// Finds the text content of the first child element with the given type.
fn find_child_text_by_type(element: &PdfStructElement<'_>, target_type: &PdfStructElementType) -> Option<String> {
    for child in element.children() {
        if child.element_type() == *target_type {
            if let Some(text) = child.actual_text() {
                return Some(text);
            }
            if let Some(alt) = child.alt_text() {
                return Some(alt);
            }
        }
    }
    None
}

/// Removes pure structural wrapper blocks (Document, Part, Div, Sect, Art, NonStruct)
/// that don't carry semantic meaning, lifting their children up.
fn flatten_structural_wrappers(blocks: Vec<ExtractedBlock>) -> Vec<ExtractedBlock> {
    let mut result = Vec::new();
    for block in blocks {
        if is_structural_wrapper(&block.role) && block.text.is_empty() {
            // Lift children up.
            let children = flatten_structural_wrappers(block.children);
            result.extend(children);
        } else {
            let children = flatten_structural_wrappers(block.children);
            result.push(ExtractedBlock { children, ..block });
        }
    }
    result
}

/// Returns `true` if a role is a pure structural wrapper with no semantic meaning.
fn is_structural_wrapper(role: &ContentRole) -> bool {
    matches!(
        role,
        ContentRole::Other(s) if matches!(s.as_str(), "Document" | "Part" | "Div" | "Sect" | "Art" | "NonStruct" | "")
    )
}

/// Extracts content using heuristic analysis of text objects.
///
/// Groups text objects into blocks based on spatial position and font properties.
fn extract_via_heuristics(page: &PdfPage<'_>) -> Result<PageExtraction, PdfiumError> {
    let objects = page.objects();
    let text_page = page.text()?;
    let mut text_entries: Vec<TextEntry> = Vec::new();

    // Collect all text objects with their properties.
    // Uses pre-loaded text_page handle to avoid calling FPDFText_LoadPage per object.
    for i in 0..objects.len() {
        let object = objects.get(i)?;

        if let Some(text_obj) = object.as_text_object() {
            let text = text_page.for_object(text_obj);
            if text.is_empty() {
                continue;
            }

            let font = text_obj.font();
            let font_size = text_obj.scaled_font_size().value;
            let is_bold = font.weight().ok().is_some_and(|w| {
                matches!(
                    w,
                    PdfFontWeight::Weight700Bold | PdfFontWeight::Weight800 | PdfFontWeight::Weight900
                )
            }) || font.is_bold_reenforced()
                || font.name().to_ascii_lowercase().contains("bold");
            let is_italic = font.is_italic();
            let is_monospace = font.is_fixed_pitch();

            let bounds = object.bounds().ok().map(|qp| qp.to_rect());

            text_entries.push(TextEntry {
                text,
                font_size,
                is_bold,
                is_italic,
                is_monospace,
                bounds,
            });
        }
    }

    if text_entries.is_empty() {
        return Ok(PageExtraction {
            method: PageExtractionMethod::Heuristic,
            blocks: Vec::new(),
        });
    }

    // Determine the body font size (most common font size).
    let body_font_size = find_body_font_size(&text_entries);

    // Group text entries into blocks based on vertical position.
    let blocks = group_text_into_blocks(text_entries, body_font_size, page.height());

    Ok(PageExtraction {
        method: PageExtractionMethod::Heuristic,
        blocks,
    })
}

/// Internal representation of a text object for heuristic extraction.
struct TextEntry {
    text: String,
    font_size: f32,
    is_bold: bool,
    is_italic: bool,
    is_monospace: bool,
    bounds: Option<PdfRect>,
}

/// Finds the most commonly occurring font size (the "body" font size).
fn find_body_font_size(entries: &[TextEntry]) -> f32 {
    let mut size_counts: HashMap<u32, usize> = HashMap::new();
    for entry in entries {
        // Round to nearest 0.5pt for grouping.
        let key = (entry.font_size * 2.0).round() as u32;
        *size_counts.entry(key).or_insert(0) += 1;
    }

    size_counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(key, _)| key as f32 / 2.0)
        .unwrap_or(12.0)
}

/// Groups text entries into content blocks using vertical gaps.
fn group_text_into_blocks(entries: Vec<TextEntry>, body_font_size: f32, page_height: PdfPoints) -> Vec<ExtractedBlock> {
    if entries.is_empty() {
        return Vec::new();
    }

    // Sort by vertical position (top-to-bottom), then left-to-right.
    let mut sorted = entries;
    sorted.sort_by(|a, b| {
        let a_top = a
            .bounds
            .as_ref()
            .map(|r| page_height.value - r.top().value)
            .unwrap_or(0.0);
        let b_top = b
            .bounds
            .as_ref()
            .map(|r| page_height.value - r.top().value)
            .unwrap_or(0.0);
        a_top.total_cmp(&b_top).then_with(|| {
            let a_left = a.bounds.as_ref().map(|r| r.left().value).unwrap_or(0.0);
            let b_left = b.bounds.as_ref().map(|r| r.left().value).unwrap_or(0.0);
            a_left.total_cmp(&b_left)
        })
    });

    // Group entries that are close together vertically.
    let mut blocks = Vec::new();
    let mut current_group: Vec<TextEntry> = vec![sorted.remove(0)];

    for entry in sorted {
        let should_break = {
            let last = current_group.last().unwrap();
            let gap = vertical_gap(last, &entry, page_height);
            // Break if gap is larger than the body font size.
            gap > body_font_size * 1.2
        };

        if should_break {
            blocks.push(finalize_block(current_group, body_font_size));
            current_group = vec![entry];
        } else {
            current_group.push(entry);
        }
    }

    if !current_group.is_empty() {
        blocks.push(finalize_block(current_group, body_font_size));
    }

    blocks
}

/// Computes the vertical gap between two text entries.
fn vertical_gap(a: &TextEntry, b: &TextEntry, page_height: PdfPoints) -> f32 {
    let a_bottom = a
        .bounds
        .as_ref()
        .map(|r| page_height.value - r.bottom().value)
        .unwrap_or(0.0);
    let b_top = b
        .bounds
        .as_ref()
        .map(|r| page_height.value - r.top().value)
        .unwrap_or(0.0);
    (b_top - a_bottom).abs()
}

/// Converts a group of text entries into a single ExtractedBlock.
fn finalize_block(group: Vec<TextEntry>, body_font_size: f32) -> ExtractedBlock {
    let text: String = group.iter().map(|e| e.text.as_str()).collect::<Vec<_>>().join(" ");

    // Use the first entry's style as representative.
    let first = &group[0];
    let font_size = first.font_size;
    let is_bold = first.is_bold;
    let is_italic = first.is_italic;
    let is_monospace = first.is_monospace;

    // Determine role based on font size relative to body.
    let role = if font_size > body_font_size * 1.3 {
        // Significantly larger than body → heading.
        let level = if font_size > body_font_size * 1.8 {
            1
        } else if font_size > body_font_size * 1.5 {
            2
        } else {
            3
        };
        ContentRole::Heading { level }
    } else {
        ContentRole::Paragraph
    };

    // Compute bounding box union.
    let bounds = compute_union_bounds(&group);

    ExtractedBlock {
        role,
        text,
        bounds,
        font_size: Some(font_size),
        is_bold,
        is_italic,
        is_monospace,
        children: Vec::new(),
    }
}

/// Computes the union of all bounding rectangles in a group.
fn compute_union_bounds(group: &[TextEntry]) -> Option<PdfRect> {
    let mut result: Option<PdfRect> = None;
    for entry in group {
        if let Some(bounds) = &entry.bounds {
            result = Some(match result {
                None => *bounds,
                Some(r) => union_rect(&r, bounds),
            });
        }
    }
    result
}

/// Returns the union of two PdfRects.
fn union_rect(a: &PdfRect, b: &PdfRect) -> PdfRect {
    PdfRect::new(
        PdfPoints::new(a.bottom().value.min(b.bottom().value)),
        PdfPoints::new(a.left().value.min(b.left().value)),
        PdfPoints::new(a.top().value.max(b.top().value)),
        PdfPoints::new(a.right().value.max(b.right().value)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(text: &str, font_size: f32, y_top: f32, y_bottom: f32) -> TextEntry {
        TextEntry {
            text: text.to_string(),
            font_size,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            bounds: Some(PdfRect::new(
                PdfPoints::new(y_bottom),
                PdfPoints::new(0.0),
                PdfPoints::new(y_top),
                PdfPoints::new(100.0),
            )),
        }
    }

    fn make_block(role: ContentRole, text: &str, children: Vec<ExtractedBlock>) -> ExtractedBlock {
        ExtractedBlock {
            role,
            text: text.to_string(),
            bounds: None,
            font_size: Some(12.0),
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            children,
        }
    }

    #[test]
    fn test_find_body_font_size_most_common() {
        let entries = vec![
            make_entry("a", 12.0, 100.0, 90.0),
            make_entry("b", 12.0, 90.0, 80.0),
            make_entry("c", 12.0, 80.0, 70.0),
            make_entry("d", 24.0, 60.0, 50.0),
        ];
        assert_eq!(find_body_font_size(&entries), 12.0);
    }

    #[test]
    fn test_find_body_font_size_single_entry() {
        let entries = vec![make_entry("a", 14.0, 100.0, 90.0)];
        assert_eq!(find_body_font_size(&entries), 14.0);
    }

    #[test]
    fn test_find_body_font_size_empty() {
        let entries: Vec<TextEntry> = vec![];
        assert_eq!(find_body_font_size(&entries), 12.0);
    }

    #[test]
    fn test_is_structural_wrapper() {
        assert!(is_structural_wrapper(&ContentRole::Other("Document".to_string())));
        assert!(is_structural_wrapper(&ContentRole::Other("Part".to_string())));
        assert!(is_structural_wrapper(&ContentRole::Other("Div".to_string())));
        assert!(is_structural_wrapper(&ContentRole::Other("Sect".to_string())));
        assert!(is_structural_wrapper(&ContentRole::Other("Art".to_string())));
        assert!(is_structural_wrapper(&ContentRole::Other("NonStruct".to_string())));
        assert!(is_structural_wrapper(&ContentRole::Other(String::new())));

        assert!(!is_structural_wrapper(&ContentRole::Paragraph));
        assert!(!is_structural_wrapper(&ContentRole::Heading { level: 1 }));
        assert!(!is_structural_wrapper(&ContentRole::Other("Table".to_string())));
    }

    #[test]
    fn test_flatten_structural_wrappers_lifts_children() {
        let blocks = vec![make_block(
            ContentRole::Other("Document".to_string()),
            "",
            vec![
                make_block(ContentRole::Heading { level: 1 }, "Title", vec![]),
                make_block(ContentRole::Paragraph, "Body text", vec![]),
            ],
        )];

        let flattened = flatten_structural_wrappers(blocks);
        assert_eq!(flattened.len(), 2);
        assert_eq!(flattened[0].text, "Title");
        assert_eq!(flattened[1].text, "Body text");
    }

    #[test]
    fn test_flatten_structural_wrappers_preserves_semantic_blocks() {
        let blocks = vec![
            make_block(ContentRole::Heading { level: 1 }, "Title", vec![]),
            make_block(ContentRole::Paragraph, "Body", vec![]),
        ];

        let flattened = flatten_structural_wrappers(blocks);
        assert_eq!(flattened.len(), 2);
        assert_eq!(flattened[0].text, "Title");
        assert_eq!(flattened[1].text, "Body");
    }

    #[test]
    fn test_flatten_structural_wrappers_nested() {
        let blocks = vec![make_block(
            ContentRole::Other("Document".to_string()),
            "",
            vec![make_block(
                ContentRole::Other("Sect".to_string()),
                "",
                vec![make_block(ContentRole::Paragraph, "Deep content", vec![])],
            )],
        )];

        let flattened = flatten_structural_wrappers(blocks);
        assert_eq!(flattened.len(), 1);
        assert_eq!(flattened[0].text, "Deep content");
    }

    #[test]
    fn test_flatten_keeps_wrapper_with_text() {
        let blocks = vec![make_block(
            ContentRole::Other("Div".to_string()),
            "Div with text",
            vec![],
        )];

        let flattened = flatten_structural_wrappers(blocks);
        assert_eq!(flattened.len(), 1);
        assert_eq!(flattened[0].text, "Div with text");
    }

    #[test]
    fn test_finalize_block_paragraph() {
        let group = vec![
            make_entry("Hello", 12.0, 100.0, 90.0),
            make_entry("world", 12.0, 100.0, 90.0),
        ];
        let block = finalize_block(group, 12.0);
        assert_eq!(block.role, ContentRole::Paragraph);
        assert_eq!(block.text, "Hello world");
    }

    #[test]
    fn test_finalize_block_heading_detection() {
        let group = vec![make_entry("Title", 24.0, 100.0, 80.0)];
        let block = finalize_block(group, 12.0);
        // 24 > 12 * 1.8 = 21.6, so should be H1
        assert_eq!(block.role, ContentRole::Heading { level: 1 });
    }

    #[test]
    fn test_finalize_block_h2_detection() {
        let group = vec![make_entry("Subtitle", 20.0, 100.0, 80.0)];
        let block = finalize_block(group, 12.0);
        // 20 > 12 * 1.5 = 18.0, but 20 < 12 * 1.8 = 21.6, so H2
        assert_eq!(block.role, ContentRole::Heading { level: 2 });
    }

    #[test]
    fn test_finalize_block_h3_detection() {
        let group = vec![make_entry("Section", 16.5, 100.0, 80.0)];
        let block = finalize_block(group, 12.0);
        // 16.5 > 12 * 1.3 = 15.6, but 16.5 < 12 * 1.5 = 18.0, so H3
        assert_eq!(block.role, ContentRole::Heading { level: 3 });
    }

    #[test]
    fn test_union_rect() {
        let a = PdfRect::new(
            PdfPoints::new(10.0),
            PdfPoints::new(5.0),
            PdfPoints::new(50.0),
            PdfPoints::new(100.0),
        );
        let b = PdfRect::new(
            PdfPoints::new(5.0),
            PdfPoints::new(10.0),
            PdfPoints::new(60.0),
            PdfPoints::new(80.0),
        );
        let u = union_rect(&a, &b);
        assert_eq!(u.bottom().value, 5.0);
        assert_eq!(u.left().value, 5.0);
        assert_eq!(u.top().value, 60.0);
        assert_eq!(u.right().value, 100.0);
    }

    #[test]
    fn test_compute_union_bounds_empty() {
        let group: Vec<TextEntry> = vec![];
        assert!(compute_union_bounds(&group).is_none());
    }

    #[test]
    fn test_compute_union_bounds_no_bounds() {
        let group = vec![TextEntry {
            text: "test".to_string(),
            font_size: 12.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            bounds: None,
        }];
        assert!(compute_union_bounds(&group).is_none());
    }

    #[test]
    fn test_group_text_into_blocks_empty() {
        let blocks = group_text_into_blocks(Vec::new(), 12.0, PdfPoints::new(800.0));
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_group_text_into_blocks_single() {
        let entries = vec![make_entry("Hello", 12.0, 100.0, 88.0)];
        let blocks = group_text_into_blocks(entries, 12.0, PdfPoints::new(800.0));
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].text, "Hello");
    }

    #[test]
    fn test_vertical_gap_calculation() {
        let a = make_entry("first", 12.0, 700.0, 688.0);
        let b = make_entry("second", 12.0, 680.0, 668.0);
        let gap = vertical_gap(&a, &b, PdfPoints::new(800.0));
        // a_bottom = 800 - 688 = 112
        // b_top = 800 - 680 = 120
        // gap = |120 - 112| = 8
        assert!((gap - 8.0).abs() < 0.01);
    }
}
