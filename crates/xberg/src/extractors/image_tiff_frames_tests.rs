use super::*;
use crate::types::internal::InternalElement;
use crate::types::{ExtractedImage, Table};
use std::sync::Arc;

/// A frame as the per-image OCR path returns it: one paragraph, one table with its element, one image.
fn frame_with_table_and_image(text: &str) -> InternalDocument {
    let mut frame = InternalDocument::new("image");
    frame.push_element(InternalElement::text(ElementKind::Paragraph, text, 0));
    let table_index = frame.push_table(Table {
        cells: vec![vec![text.to_string()]],
        page_number: 1,
        ..Default::default()
    });
    frame.push_element(InternalElement::text(ElementKind::Table { table_index }, "", 0));
    let image_index = frame.push_image(ExtractedImage::default());
    frame.push_element(InternalElement::text(ElementKind::Image { image_index }, "", 0));
    let mut page = blank_page(1);
    page.content = text.to_string();
    page.tables = vec![Arc::new(frame.tables[0].clone())];
    page.image_indices = vec![image_index];
    frame.prebuilt_pages = Some(vec![page]);
    frame
}

fn merged(texts: &[&str]) -> (InternalDocument, Vec<crate::types::PageContent>) {
    let mut target = InternalDocument::new("image");
    let mut ocr_elements = Vec::new();
    let mut pages = Vec::new();
    for (index, text) in texts.iter().enumerate() {
        let number = index as u32 + 1;
        let mut page = append_frame(&mut target, frame_with_table_and_image(text), number, &mut ocr_elements);
        page.page_number = number;
        pages.push(page);
    }
    (target, pages)
}

#[test]
fn tables_of_later_frames_carry_their_frame_page_and_index() {
    let (target, pages) = merged(&["one", "two", "three"]);
    let table_pages: Vec<u32> = target.tables.iter().map(|table| table.page_number).collect();
    assert_eq!(table_pages, vec![1, 2, 3]);
    let element_tables: Vec<(u32, Option<u32>)> = target
        .elements
        .iter()
        .filter_map(|element| match element.kind {
            ElementKind::Table { table_index } => Some((table_index, element.page)),
            _ => None,
        })
        .collect();
    assert_eq!(element_tables, vec![(0, Some(1)), (1, Some(2)), (2, Some(3))]);
    for (page, expected) in pages.iter().zip(["one", "two", "three"]) {
        assert_eq!(page.tables.len(), 1);
        assert_eq!(page.tables[0].page_number, page.page_number);
        assert_eq!(page.tables[0].cells, vec![vec![expected.to_string()]]);
    }
}

#[test]
fn images_of_later_frames_are_offset_into_the_merged_image_list() {
    let (target, pages) = merged(&["one", "two", "three"]);
    assert_eq!(target.images.len(), 3);
    let element_images: Vec<u32> = target
        .elements
        .iter()
        .filter_map(|element| match element.kind {
            ElementKind::Image { image_index } => Some(image_index),
            _ => None,
        })
        .collect();
    assert_eq!(element_images, vec![0, 1, 2]);
    let page_images: Vec<Vec<u32>> = pages.iter().map(|page| page.image_indices.clone()).collect();
    assert_eq!(page_images, vec![vec![0], vec![1], vec![2]]);
}

#[test]
fn identical_text_on_two_frames_gets_distinct_element_ids() {
    let (target, _) = merged(&["same", "same"]);
    let paragraph_ids: Vec<String> = target
        .elements
        .iter()
        .filter(|element| matches!(element.kind, ElementKind::Paragraph))
        .map(|element| element.id.to_string())
        .collect();
    assert_eq!(paragraph_ids.len(), 2);
    assert_ne!(paragraph_ids[0], paragraph_ids[1]);
    let all: std::collections::HashSet<String> = target.elements.iter().map(|element| element.id.to_string()).collect();
    assert_eq!(all.len(), target.elements.len(), "every merged element id is unique");
}

#[test]
fn a_walk_with_one_frame_reads_by_frame_only_when_the_chain_is_broken() {
    let one = |end| TiffIfdWalk { offsets: vec![8], end };
    assert!(!reads_by_frame(&one(TiffWalkEnd::Complete)));
    assert!(reads_by_frame(&one(TiffWalkEnd::Broken("cut"))));
    assert!(reads_by_frame(&TiffIfdWalk {
        offsets: vec![8, 40],
        end: TiffWalkEnd::Complete,
    }));
    let warning = walk_warning(&one(TiffWalkEnd::Broken("cut"))).expect("broken chain warns");
    assert!(warning.message.contains("frame 2"), "{}", warning.message);
    assert!(walk_warning(&one(TiffWalkEnd::Complete)).is_none());
}

#[test]
fn a_frame_without_a_prebuilt_page_gets_its_text_from_its_elements() {
    let mut frame = InternalDocument::new("image");
    frame.push_element(InternalElement::text(ElementKind::Paragraph, "region text", 0));
    frame.formulas.push(crate::types::Formula {
        latex: "x".into(),
        bbox: None,
        page: Some(1),
    });
    let mut target = InternalDocument::new("image");
    let page = append_frame(&mut target, frame, 2, &mut Vec::new());
    assert_eq!(page.content, "region text");
    assert_eq!(page.is_blank, Some(false));
    assert_eq!(target.formulas[0].page, Some(2));
}
