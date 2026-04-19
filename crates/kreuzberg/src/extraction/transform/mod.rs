//! Transformation utilities for converting extraction results into semantic elements.
//!
//! This module provides post-processing functions to transform raw extraction results
//! into element-based output format, suitable for downstream processing and analysis.
//! Key functionality includes:
//!
//! - Semantic element generation from text content
//! - List item detection with support for multiple formats
//! - PageBreak interleaving with reverse byte-order processing
//! - Safe bounds checking for text ranges

mod content;
pub(crate) mod document_tree;
mod elements;
mod types;

// Re-export public API
pub use document_tree::transform_to_document_structure;
pub use elements::{detect_list_items, generate_element_id};
pub use types::{ListItemMetadata, ListType};

use crate::types::{Element, ExtractionResult};
use content::{
    add_page_break, format_table_as_text, process_content, process_hierarchy, process_images, process_tables,
};
#[cfg(test)]
use std::borrow::Cow;

/// Transform an extraction result into semantic elements.
///
/// This function takes a reference to an ExtractionResult and generates
/// a vector of Element structs representing semantic blocks in the document.
/// It detects content sections, list items, page breaks, and other structural
/// elements to create an Unstructured-compatible element-based output.
///
/// Handles:
/// - PDF hierarchy → Title/Heading elements
/// - Multi-page documents with correct page numbers
/// - Table and Image extraction
/// - PageBreak interleaving
/// - Bounding box coordinates
/// - Paragraph detection for NarrativeText
///
/// # Arguments
///
/// * `result` - Reference to the ExtractionResult to transform
///
/// # Returns
///
/// A vector of Elements with proper semantic types and metadata.
pub fn transform_extraction_result_to_elements(result: &ExtractionResult) -> Vec<Element> {
    let mut elements = Vec::new();

    // If pages are available, process per-page with hierarchy, tables, images
    if let Some(ref pages) = result.pages {
        for page in pages {
            let page_number = page.page_number;

            // 1. Process hierarchy blocks (headings + body text with coordinates).
            // Returns true when body blocks with bounding boxes were emitted, in
            // which case step 4 is skipped to avoid coordinate-less duplicates.
            let hierarchy_covered_body = if let Some(ref hierarchy) = page.hierarchy {
                process_hierarchy(&mut elements, hierarchy, page_number, &result.metadata.title)
            } else {
                false
            };

            // 2. Process tables on this page
            process_tables(&mut elements, &page.tables, page_number, &result.metadata.title);

            // 3. Process images on this page
            process_images(&mut elements, &page.images, page_number, &result.metadata.title);

            // 4. Process page content (body text, list items, paragraphs).
            // Skipped when hierarchy already emitted body elements with coordinates
            // to prevent producing duplicate coordinate-less elements.
            if !hierarchy_covered_body {
                process_content(&mut elements, &page.content, page_number, &result.metadata.title);
            }

            // 5. Add PageBreak after each page (except the last)
            if page_number < pages.len() {
                add_page_break(&mut elements, page_number, page_number + 1, &result.metadata.title);
            }
        }
    } else {
        // Fallback: No pages, process unified content with page 1
        process_content(&mut elements, &result.content, 1, &result.metadata.title);

        // Process global tables (if any)
        for table in &result.tables {
            let table_text = format_table_as_text(table);
            let element_id = elements::generate_element_id(&table_text, crate::types::ElementType::Table, Some(1));
            elements.push(Element {
                element_id,
                element_type: crate::types::ElementType::Table,
                text: table_text,
                metadata: crate::types::ElementMetadata {
                    page_number: Some(1),
                    filename: result.metadata.title.clone(),
                    coordinates: None,
                    element_index: Some(elements.len()),
                    additional: std::collections::HashMap::new(),
                },
            });
        }

        // Process global images (if any)
        if let Some(ref images) = result.images {
            for image in images {
                let image_text = format!(
                    "Image: {} ({}x{})",
                    image.format,
                    image.width.unwrap_or(0),
                    image.height.unwrap_or(0)
                );
                let page_num = image.page_number.unwrap_or(1);

                let element_id =
                    elements::generate_element_id(&image_text, crate::types::ElementType::Image, Some(page_num));
                elements.push(Element {
                    element_id,
                    element_type: crate::types::ElementType::Image,
                    text: image_text,
                    metadata: crate::types::ElementMetadata {
                        page_number: Some(page_num),
                        filename: result.metadata.title.clone(),
                        coordinates: None,
                        element_index: Some(elements.len()),
                        additional: {
                            let mut m = std::collections::HashMap::new();
                            m.insert("format".to_string(), image.format.to_string());
                            if let Some(width) = image.width {
                                m.insert("width".to_string(), width.to_string());
                            }
                            if let Some(height) = image.height {
                                m.insert("height".to_string(), height.to_string());
                            }
                            m
                        },
                    },
                });
            }
        }
    }

    elements
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_detect_bullet_items() {
        let text = "- First item\n- Second item\n- Third item";
        let items = detect_list_items(text);
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].list_type, ListType::Bullet);
        assert_eq!(items[1].list_type, ListType::Bullet);
        assert_eq!(items[2].list_type, ListType::Bullet);
    }

    #[test]
    fn test_detect_numbered_items() {
        let text = "1. First\n2. Second\n3. Third";
        let items = detect_list_items(text);
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|i| i.list_type == ListType::Numbered));
    }

    #[test]
    fn test_detect_lettered_items() {
        let text = "a. First\nb. Second\nc. Third";
        let items = detect_list_items(text);
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|i| i.list_type == ListType::Lettered));
    }

    #[test]
    fn test_detect_mixed_items() {
        let text = "Some text\n- Bullet\n1. Numbered\nMore text";
        let items = detect_list_items(text);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].list_type, ListType::Bullet);
        assert_eq!(items[1].list_type, ListType::Numbered);
    }

    #[test]
    fn test_element_id_generation() {
        use crate::types::ElementType;
        let id1 = generate_element_id("test", ElementType::Title, Some(1));
        let id2 = generate_element_id("test", ElementType::Title, Some(1));
        assert_eq!(id1.as_ref(), id2.as_ref());

        let id3 = generate_element_id("different", ElementType::Title, Some(1));
        assert_ne!(id1.as_ref(), id3.as_ref());
    }

    #[test]
    fn test_page_break_interleaving_reverse_order() {
        // Test that page breaks are processed in reverse byte order
        let page_breaks = vec![(100, "page_break_1"), (50, "page_break_2"), (75, "page_break_3")];

        // Sort in descending order by byte offset
        let mut sorted = page_breaks.clone();
        sorted.sort_by(|(offset_a, _), (offset_b, _)| offset_b.cmp(offset_a));

        // Verify reverse order: 100, 75, 50
        assert_eq!(sorted[0].0, 100);
        assert_eq!(sorted[1].0, 75);
        assert_eq!(sorted[2].0, 50);
    }

    #[test]
    fn test_bounds_checking() {
        let text = "Hello world";

        // Valid range
        let valid_item = ListItemMetadata {
            list_type: ListType::Bullet,
            byte_start: 0,
            byte_end: 5,
            indent_level: 0,
        };
        assert!(valid_item.byte_start <= text.len());
        assert!(valid_item.byte_end <= text.len());
        assert!(valid_item.byte_start <= valid_item.byte_end);

        // Invalid: end beyond string
        let invalid_item = ListItemMetadata {
            list_type: ListType::Bullet,
            byte_start: 0,
            byte_end: 100,
            indent_level: 0,
        };
        assert!(invalid_item.byte_end > text.len());
    }

    #[test]
    fn test_indent_level_detection() {
        let text = "    - Indented item";
        let items = detect_list_items(text);
        assert_eq!(items.len(), 1);
        assert!(items[0].indent_level >= 1);
    }

    // Helper to create minimal Metadata for tests
    fn test_metadata(title: Option<String>) -> crate::types::Metadata {
        crate::types::Metadata {
            title,
            ..Default::default()
        }
    }

    // Integration tests for full transformation
    #[test]
    fn test_transform_with_pages_and_hierarchy() {
        use crate::types::{ElementType, ExtractionResult, HierarchicalBlock, PageContent, PageHierarchy};

        // Create a mock result with pages and hierarchy
        let result = ExtractionResult {
            content: "Full document content".to_string(),
            mime_type: Cow::Borrowed("application/pdf"),
            metadata: test_metadata(Some("Test Document".to_string())),
            pages: Some(vec![
                PageContent {
                    page_number: 1,
                    content: "This is a test paragraph.\n\nAnother paragraph here.".to_string(),
                    tables: vec![],
                    images: vec![],
                    hierarchy: Some(PageHierarchy {
                        block_count: 2,
                        blocks: vec![
                            HierarchicalBlock {
                                text: "Main Title".to_string(),
                                font_size: 24.0,
                                level: "h1".to_string(),
                                bbox: Some((10.0, 20.0, 100.0, 50.0)),
                            },
                            HierarchicalBlock {
                                text: "Subtitle".to_string(),
                                font_size: 16.0,
                                level: "h2".to_string(),
                                bbox: Some((10.0, 60.0, 100.0, 80.0)),
                            },
                        ],
                    }),
                    is_blank: None,
                    layout_regions: None,
                },
                PageContent {
                    page_number: 2,
                    content: "- List item 1\n- List item 2".to_string(),
                    tables: vec![],
                    images: vec![],
                    hierarchy: None,
                    is_blank: None,
                    layout_regions: None,
                },
            ]),
            ..Default::default()
        };

        let elements = transform_extraction_result_to_elements(&result);

        // Verify we have elements
        assert!(!elements.is_empty());

        // Find Title elements from hierarchy
        let titles: Vec<_> = elements
            .iter()
            .filter(|e| e.element_type == ElementType::Title)
            .collect();
        assert_eq!(titles.len(), 2, "Should have 2 title elements from hierarchy");
        assert_eq!(titles[0].text, "Main Title");
        assert_eq!(titles[1].text, "Subtitle");

        // Verify page numbers
        assert_eq!(titles[0].metadata.page_number, Some(1));
        assert_eq!(titles[1].metadata.page_number, Some(1));

        // Verify coordinates were extracted
        assert!(titles[0].metadata.coordinates.is_some());
        assert!(titles[1].metadata.coordinates.is_some());

        // Find list items
        let list_items: Vec<_> = elements
            .iter()
            .filter(|e| e.element_type == ElementType::ListItem)
            .collect();
        assert_eq!(list_items.len(), 2, "Should have 2 list items");
        assert_eq!(list_items[0].metadata.page_number, Some(2));
        assert_eq!(list_items[1].metadata.page_number, Some(2));

        // Find PageBreak
        let page_breaks: Vec<_> = elements
            .iter()
            .filter(|e| e.element_type == ElementType::PageBreak)
            .collect();
        assert_eq!(page_breaks.len(), 1, "Should have 1 page break between pages");
    }

    #[test]
    fn test_transform_with_tables_and_images() {
        use crate::types::{ExtractedImage, ExtractionResult, PageContent, Table};
        use std::sync::Arc;

        let table = Table {
            cells: vec![
                vec!["Header1".to_string(), "Header2".to_string()],
                vec!["Cell1".to_string(), "Cell2".to_string()],
            ],
            markdown: "| Header1 | Header2 |\n| Cell1 | Cell2 |".to_string(),
            page_number: 1,
            bounding_box: None,
        };

        let image = ExtractedImage {
            data: Bytes::from_static(&[1, 2, 3, 4]),
            format: std::borrow::Cow::Borrowed("jpeg"),
            image_index: 0,
            page_number: Some(1),
            width: Some(640),
            height: Some(480),
            colorspace: Some("RGB".to_string()),
            bits_per_component: Some(8),
            is_mask: false,
            description: None,
            ocr_result: None,
            bounding_box: None,
            source_path: None,
        };

        let result = ExtractionResult {
            content: "Test content".to_string(),
            mime_type: Cow::Borrowed("application/pdf"),
            metadata: test_metadata(Some("Test".to_string())),
            pages: Some(vec![PageContent {
                page_number: 1,
                content: "Some text".to_string(),
                tables: vec![Arc::new(table)],
                images: vec![Arc::new(image)],
                hierarchy: None,
                is_blank: None,
                layout_regions: None,
            }]),
            ..Default::default()
        };

        let elements = transform_extraction_result_to_elements(&result);

        // Find table elements
        use crate::types::ElementType;
        let tables: Vec<_> = elements
            .iter()
            .filter(|e| e.element_type == ElementType::Table)
            .collect();
        assert_eq!(tables.len(), 1, "Should have 1 table element");
        assert!(tables[0].text.contains("Header1"));
        assert!(tables[0].text.contains("Cell2"));

        // Find image elements
        let images: Vec<_> = elements
            .iter()
            .filter(|e| e.element_type == ElementType::Image)
            .collect();
        assert_eq!(images.len(), 1, "Should have 1 image element");
        assert!(images[0].text.contains("jpeg"));
        assert!(images[0].text.contains("640"));
        assert!(images[0].text.contains("480"));
        assert_eq!(images[0].metadata.page_number, Some(1));
    }

    #[test]
    fn test_transform_fallback_no_pages() {
        use crate::types::{ElementType, ExtractionResult};

        // Create a result without pages
        let result = ExtractionResult {
            content: "Simple text content\n\nSecond paragraph".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata: test_metadata(Some("Simple Doc".to_string())),
            ..Default::default()
        };

        let elements = transform_extraction_result_to_elements(&result);

        // Should have narrative text elements
        let narratives: Vec<_> = elements
            .iter()
            .filter(|e| e.element_type == ElementType::NarrativeText)
            .collect();
        assert!(!narratives.is_empty(), "Should have narrative text elements");

        // All elements should have page_number = 1 (fallback)
        for element in &elements {
            assert_eq!(element.metadata.page_number, Some(1));
        }
    }

    #[test]
    fn test_detect_list_items_with_crlf() {
        // CRLF line endings must not cause byte offset drift
        let text = "- First item\r\n- Second item\r\n- Third item";
        let items = detect_list_items(text);
        assert_eq!(items.len(), 3);
        // Verify byte offsets are valid char boundaries
        assert!(text.is_char_boundary(items[0].byte_start));
        assert!(text.is_char_boundary(items[0].byte_end));
        assert!(text.is_char_boundary(items[1].byte_start));
        assert!(text.is_char_boundary(items[1].byte_end));
        assert!(text.is_char_boundary(items[2].byte_start));
        assert!(text.is_char_boundary(items[2].byte_end));
        // Verify actual content via slicing (should not panic)
        assert_eq!(&text[items[0].byte_start..items[0].byte_end], "- First item");
        assert_eq!(&text[items[1].byte_start..items[1].byte_end], "- Second item");
        assert_eq!(&text[items[2].byte_start..items[2].byte_end], "- Third item");
    }

    #[test]
    fn test_detect_list_items_with_multibyte_utf8() {
        // Multi-byte UTF-8 characters (right single quote U+2019, en-dash U+2013)
        // must not cause panics when used with byte offsets
        let text = "Some text with \u{2019}quotes\u{2019}\n- First item\n1. Second \u{2013} item";
        let items = detect_list_items(text);
        assert_eq!(items.len(), 2);
        for item in &items {
            assert!(
                text.is_char_boundary(item.byte_start),
                "byte_start {} is not a char boundary",
                item.byte_start
            );
            assert!(
                text.is_char_boundary(item.byte_end),
                "byte_end {} is not a char boundary",
                item.byte_end
            );
            // Slicing should not panic
            let _ = &text[item.byte_start..item.byte_end];
        }
    }

    #[test]
    fn test_detect_list_items_crlf_with_multibyte() {
        // Combination of CRLF and multi-byte characters
        let text = "Policy \u{2019}Administration\u{2019}\r\n- Item one\r\nSome \u{2013} text\r\n1. Item two";
        let items = detect_list_items(text);
        assert_eq!(items.len(), 2);
        for item in &items {
            assert!(text.is_char_boundary(item.byte_start));
            assert!(text.is_char_boundary(item.byte_end));
            let slice = &text[item.byte_start..item.byte_end];
            assert!(!slice.is_empty());
        }
    }

    #[test]
    fn test_process_content_multibyte_no_panic() {
        use crate::types::ElementType;

        // Simulate the scenario from issue #398: text with multi-byte chars and list items
        let content = "Number 1.0 \u{2013} POLICY MANUAL\r\n\r\nRevised: August 4, 2008\r\nThe State\u{2019}s policy:\r\n- First item\r\n- Second item";
        let mut elements = Vec::new();
        // This should not panic
        process_content(&mut elements, content, 1, &None);

        // Verify we got some elements
        assert!(!elements.is_empty());
        let list_items: Vec<_> = elements
            .iter()
            .filter(|e| e.element_type == ElementType::ListItem)
            .collect();
        assert_eq!(list_items.len(), 2);
    }

    #[test]
    fn test_process_content_pure_multibyte_text() {
        // Text with CJK characters and bullet points
        let content = "\u{4f60}\u{597d}\u{4e16}\u{754c}\n- \u{7b2c}\u{4e00}\u{9879}\n- \u{7b2c}\u{4e8c}\u{9879}";
        let mut elements = Vec::new();
        // Must not panic
        process_content(&mut elements, content, 1, &None);
        assert!(!elements.is_empty());
    }

    #[test]
    fn test_paragraph_splitting() {
        use crate::types::{ElementType, ExtractionResult};

        let result = ExtractionResult {
            content: "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            metadata: test_metadata(None),
            ..Default::default()
        };

        let elements = transform_extraction_result_to_elements(&result);

        let narratives: Vec<_> = elements
            .iter()
            .filter(|e| e.element_type == ElementType::NarrativeText)
            .collect();

        // Should split into 3 separate paragraphs
        assert_eq!(narratives.len(), 3, "Should split into 3 paragraphs");
        assert_eq!(narratives[0].text, "First paragraph.");
        assert_eq!(narratives[1].text, "Second paragraph.");
        assert_eq!(narratives[2].text, "Third paragraph.");
    }

    /// Body-level hierarchy blocks with bounding boxes must produce NarrativeText
    /// elements with populated coordinates (issue #566).
    #[test]
    fn test_body_hierarchy_blocks_get_coordinates() {
        use crate::types::{ElementType, ExtractionResult, HierarchicalBlock, PageContent, PageHierarchy};

        let result = ExtractionResult {
            content: "Some body text here.".to_string(),
            mime_type: Cow::Borrowed("application/pdf"),
            metadata: test_metadata(Some("Doc".to_string())),
            pages: Some(vec![PageContent {
                page_number: 1,
                content: "Some body text here.".to_string(),
                tables: vec![],
                images: vec![],
                hierarchy: Some(PageHierarchy {
                    block_count: 2,
                    blocks: vec![
                        HierarchicalBlock {
                            text: "Heading".to_string(),
                            font_size: 18.0,
                            level: "h1".to_string(),
                            bbox: Some((10.0, 20.0, 200.0, 40.0)),
                        },
                        HierarchicalBlock {
                            text: "Some body text here.".to_string(),
                            font_size: 12.0,
                            level: "body".to_string(),
                            bbox: Some((10.0, 50.0, 200.0, 65.0)),
                        },
                    ],
                }),
                is_blank: None,
                layout_regions: None,
            }]),
            ..Default::default()
        };

        let elements = transform_extraction_result_to_elements(&result);

        // Title element must have coordinates
        let titles: Vec<_> = elements
            .iter()
            .filter(|e| e.element_type == ElementType::Title)
            .collect();
        assert_eq!(titles.len(), 1);
        assert!(
            titles[0].metadata.coordinates.is_some(),
            "Title should have coordinates"
        );

        // Body text must be emitted as NarrativeText with coordinates
        let narratives: Vec<_> = elements
            .iter()
            .filter(|e| e.element_type == ElementType::NarrativeText)
            .collect();
        assert_eq!(
            narratives.len(),
            1,
            "Should have exactly 1 NarrativeText (no duplicate from process_content)"
        );
        assert_eq!(narratives[0].text, "Some body text here.");
        assert!(
            narratives[0].metadata.coordinates.is_some(),
            "Body text should have coordinates"
        );
        let coords = narratives[0].metadata.coordinates.unwrap();
        assert_eq!(coords.x0, 10.0);
        assert_eq!(coords.y0, 50.0);
        assert_eq!(coords.x1, 200.0);
        assert_eq!(coords.y1, 65.0);
    }

    /// Body blocks without bboxes are emitted once by process_hierarchy; process_content is skipped.
    #[test]
    fn test_body_hierarchy_without_bbox_emits_once_without_coordinates() {
        use crate::types::{ElementType, ExtractionResult, HierarchicalBlock, PageContent, PageHierarchy};

        let result = ExtractionResult {
            content: "Paragraph one.\n\nParagraph two.".to_string(),
            mime_type: Cow::Borrowed("application/pdf"),
            metadata: test_metadata(None),
            pages: Some(vec![PageContent {
                page_number: 1,
                content: "Paragraph one.\n\nParagraph two.".to_string(),
                tables: vec![],
                images: vec![],
                hierarchy: Some(PageHierarchy {
                    block_count: 1,
                    blocks: vec![HierarchicalBlock {
                        text: "Paragraph one.\n\nParagraph two.".to_string(),
                        font_size: 12.0,
                        level: "body".to_string(),
                        bbox: None, // no bbox — include_bbox was false
                    }],
                }),
                is_blank: None,
                layout_regions: None,
            }]),
            ..Default::default()
        };

        let elements = transform_extraction_result_to_elements(&result);

        // process_hierarchy emits the body block as NarrativeText (no bbox → no coords).
        // Because a body block was emitted, process_content is skipped entirely — no duplicates.
        let narratives: Vec<_> = elements
            .iter()
            .filter(|e| e.element_type == ElementType::NarrativeText)
            .collect();
        assert_eq!(
            narratives.len(),
            1,
            "bbox-less body block should produce exactly one NarrativeText element"
        );
        assert!(
            narratives[0].metadata.coordinates.is_none(),
            "NarrativeText without hierarchy bbox should have no coordinates"
        );
    }
}
