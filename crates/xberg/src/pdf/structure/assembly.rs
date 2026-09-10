//! Final structured document assembly from classified paragraphs, with optional table interleaving.
//!
//! Produces an `InternalDocument` from per-page `PdfParagraph` data with tables
//! interleaved at their correct reading-order positions.

use std::borrow::Cow;

use super::lines::{crosses_visual_line_break, needs_space_between, segments_need_space};
use super::text_repair::finalize_hyphens;
use super::types::{LayoutHintClass, LayoutRegionPath, LayoutRegionTag, PdfParagraph};
use crate::types::document_structure::{AnnotationKind, ContentLayer, TextAnnotation};
use crate::types::extraction::BoundingBox;
use crate::types::internal::{ElementKind, InternalDocument, RelationshipKind, RelationshipTarget};
use crate::types::internal_builder::InternalDocumentBuilder;

/// Assemble an `InternalDocument` from classified paragraphs with tables interleaved.
///
/// Builds a typed `InternalDocument` where each paragraph, heading, code block, formula,
/// list item, table, and image is represented as a distinct `InternalElement`.
pub(crate) fn assemble_internal_document(
    pages: Vec<Vec<PdfParagraph>>,
    tables: &[crate::types::Table],
    images: Option<&[crate::types::ExtractedImage]>,
    image_positions: &[(u32, u32)],
    hyphen_witnesses: &super::pipeline::HyphenWitnesses,
) -> InternalDocument {
    tracing::debug!(
        page_count = pages.len(),
        table_count = tables.len(),
        image_count = image_positions.len(),
        total_paragraphs = pages.iter().map(|p| p.len()).sum::<usize>(),
        "assemble_internal_document: start"
    );
    let mut builder = InternalDocumentBuilder::new("pdf");

    let mut tables_by_page: std::collections::BTreeMap<u32, Vec<&crate::types::Table>> =
        std::collections::BTreeMap::new();
    for table in tables {
        tables_by_page.entry(table.page_number).or_default().push(table);
    }

    let mut images_by_page: std::collections::BTreeMap<u32, Vec<u32>> = std::collections::BTreeMap::new();
    for &(page_idx, image_index) in image_positions {
        images_by_page.entry(page_idx).or_default().push(image_index);
    }

    let mut has_emitted_content = false;
    for (page_idx, paragraphs) in pages.iter().enumerate() {
        let page_num = Some((page_idx + 1) as u32);
        let page_tables = tables_by_page.remove(&((page_idx + 1) as u32));

        let page_has_content = !paragraphs.is_empty()
            || page_tables
                .as_ref()
                .is_some_and(|t| t.iter().any(|tb| !tb.markdown.trim().is_empty()))
            || images_by_page.contains_key(&((page_idx + 1) as u32));

        if page_has_content && has_emitted_content {
            builder.push_page_break();
        }

        if let Some(ref page_tables) = page_tables {
            tracing::debug!(
                page = page_idx + 1,
                tables = page_tables.len(),
                paragraphs = paragraphs.len(),
                "assembling page with tables"
            );
        }

        let (paragraph_elem_map, page_end_transition_index) = if let Some(page_tables) = page_tables {
            assemble_page_elements_with_tables(&mut builder, paragraphs, &page_tables, page_num, hyphen_witnesses)
        } else {
            assemble_page_elements(&mut builder, paragraphs, page_num, hyphen_witnesses)
        };

        if page_has_content {
            has_emitted_content = true;
        }

        if let Some(image_indices) = images_by_page.get(&((page_idx + 1) as u32)) {
            interleave_images_into_page(
                &mut builder,
                paragraphs,
                &paragraph_elem_map,
                page_end_transition_index,
                image_indices,
                images,
                (page_idx + 1) as u32,
            );
        }
    }

    for (&page_idx, page_tables) in &tables_by_page {
        let page_num = Some(page_idx);
        for &table in page_tables {
            if !table.markdown.trim().is_empty() {
                let bbox = table.bounding_box.map(|bb| BoundingBox {
                    x0: bb.x0,
                    y0: bb.y0,
                    x1: bb.x1,
                    y1: bb.y1,
                });
                builder.push_table(table.clone(), page_num, bbox);
            }
        }
    }

    if let Some(image_indices) = images_by_page.get(&0) {
        for &image_index in image_indices {
            let elem = crate::types::internal::InternalElement::text(ElementKind::Image { image_index }, "", 0);
            builder.push_element(elem);
        }
    }

    let doc = builder.build();
    tracing::debug!(
        output_elements = doc.elements.len(),
        "assemble_internal_document complete"
    );
    doc
}

#[derive(Clone, Copy)]
struct ParagraphElementPosition {
    paragraph_index: usize,
    element_index: u32,
    transition_index: u32,
}

/// Push paragraph elements for a page without tables.
///
/// Returns the element and structural-transition positions for every non-caption
/// paragraph in page order, used by [`interleave_images_into_page`].
fn assemble_page_elements(
    builder: &mut InternalDocumentBuilder,
    paragraphs: &[PdfParagraph],
    page: Option<u32>,
    hyphen_witnesses: &super::pipeline::HyphenWitnesses,
) -> (Vec<ParagraphElementPosition>, u32) {
    let mut in_list = false;
    let mut open_regions = Vec::new();
    let mut paragraph_elem_map = Vec::new();

    for (para_idx, para) in paragraphs.iter().enumerate() {
        if para.caption_for.is_some() {
            continue;
        }

        let transition_index = builder.element_count();
        transition_layout_path(
            builder,
            &mut open_regions,
            effective_layout_path(para),
            &mut in_list,
            page,
        );

        if para.is_list_item && !in_list {
            builder.push_list(list_item_is_ordered(para));
            in_list = true;
        } else if !para.is_list_item && in_list {
            builder.end_list();
            in_list = false;
        }

        let elem_idx = push_paragraph_element(builder, para, page, hyphen_witnesses);
        paragraph_elem_map.push(ParagraphElementPosition {
            paragraph_index: para_idx,
            element_index: elem_idx,
            transition_index,
        });

        emit_caption_elements(builder, paragraphs, para_idx, page, elem_idx);
    }

    let page_end_transition_index = builder.element_count();
    close_list(builder, &mut in_list);
    close_layout_path(builder, &mut open_regions);

    (paragraph_elem_map, page_end_transition_index)
}

/// Insert each page image at its correct reading-order position among the page's
/// already-pushed paragraph elements, so VLM captions and OCR text render inline
/// instead of trailing the whole page.
///
/// For every image with a `bounding_box` on a page that has at least one
/// positioned paragraph (`block_bbox.is_some()`), the image is inserted before the
/// first horizontally-overlapping paragraph whose top-y is below the image's top-y.
/// If no such paragraph exists, vertical order alone is used as a fallback. Images
/// without a `bounding_box`, or on pages with no positioned paragraphs, retain the
/// pre-existing append-after-text behavior.
fn interleave_images_into_page(
    builder: &mut InternalDocumentBuilder,
    paragraphs: &[PdfParagraph],
    paragraph_elem_map: &[ParagraphElementPosition],
    page_end_transition_index: u32,
    image_indices: &[u32],
    images: Option<&[crate::types::ExtractedImage]>,
    page_number: u32,
) {
    let page_has_positioned_paragraph = paragraph_elem_map
        .iter()
        .any(|position| paragraphs[position.paragraph_index].block_bbox.is_some());

    let mut positioned: Vec<(u32, usize, u32)> = Vec::new();
    let mut appended: Vec<u32> = Vec::new();

    for (source_order, &image_index) in image_indices.iter().enumerate() {
        let image_bbox = images
            .and_then(|imgs| imgs.get(image_index as usize))
            .and_then(|img| img.bounding_box);

        let target = image_bbox
            .filter(|_| page_has_positioned_paragraph)
            .and_then(|bbox| image_target_element(paragraphs, paragraph_elem_map, page_end_transition_index, bbox));

        match target {
            Some(elem_idx) => positioned.push((elem_idx, source_order, image_index)),
            None => appended.push(image_index),
        }
    }

    // Insert from the highest target index down so earlier insertions never shift
    // pending target indices. For an equal target, reverse source order preserves
    // the original image order despite repeated insert-before operations.
    positioned.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));
    for (elem_idx, _, image_index) in positioned {
        let elem = image_element(images, image_index, page_number);
        builder.insert_element_before(elem_idx, elem);
    }

    for image_index in appended {
        let elem = image_element(images, image_index, page_number);
        builder.push_element(elem);
    }
}

fn image_target_element(
    paragraphs: &[PdfParagraph],
    paragraph_elem_map: &[ParagraphElementPosition],
    page_end_transition_index: u32,
    image_bbox: crate::types::BoundingBox,
) -> Option<u32> {
    let image_top = image_bbox.y1 as f32;
    let below_image = |para_idx: usize| {
        paragraphs[para_idx]
            .block_bbox
            .is_some_and(|(_, _, _, top)| top < image_top)
    };
    let horizontally_overlaps = |para_idx: usize| {
        paragraphs[para_idx]
            .block_bbox
            .is_some_and(|(left, _, right, _)| left < image_bbox.x1 as f32 && right > image_bbox.x0 as f32)
    };

    if let Some(position) = paragraph_elem_map
        .iter()
        .find(|position| horizontally_overlaps(position.paragraph_index) && below_image(position.paragraph_index))
    {
        return Some(position.element_index);
    }

    if let Some(position) = paragraph_elem_map
        .iter()
        .rposition(|position| horizontally_overlaps(position.paragraph_index))
    {
        return Some(
            paragraph_elem_map
                .get(position + 1)
                .map_or(page_end_transition_index, |next| next.transition_index),
        );
    }

    paragraph_elem_map
        .iter()
        .find(|position| below_image(position.paragraph_index))
        .map(|position| position.element_index)
}

/// Build the `ElementKind::Image` element for an image, carrying its OCR text (if any).
fn image_element(
    images: Option<&[crate::types::ExtractedImage]>,
    image_index: u32,
    page_number: u32,
) -> crate::types::internal::InternalElement {
    let ocr_text = images
        .and_then(|imgs| imgs.get(image_index as usize))
        .and_then(|img| img.ocr_result.as_ref())
        .map(|res| res.content.as_str())
        .unwrap_or("");

    crate::types::internal::InternalElement::text(ElementKind::Image { image_index }, ocr_text, 0)
        .with_page(page_number)
}

/// Push paragraph elements in their established reading order, with tables interleaved.
///
/// Returns the element and structural-transition positions for every non-caption
/// paragraph in page order, used by [`interleave_images_into_page`].
fn assemble_page_elements_with_tables(
    builder: &mut InternalDocumentBuilder,
    paragraphs: &[PdfParagraph],
    tables: &[&crate::types::Table],
    page: Option<u32>,
    hyphen_witnesses: &super::pipeline::HyphenWitnesses,
) -> (Vec<ParagraphElementPosition>, u32) {
    let mut positioned: Vec<(f32, &crate::types::Table)> = Vec::new();
    let mut unpositioned: Vec<&crate::types::Table> = Vec::new();

    for table in tables {
        let md = table.markdown.trim();
        if md.is_empty() {
            continue;
        }
        if let Some(ref bbox) = table.bounding_box {
            let top = bbox.y1 as f32;
            if top.is_finite() {
                positioned.push((top, *table));
                continue;
            }
        }
        unpositioned.push(*table);
    }

    positioned.sort_by(|a, b| b.0.total_cmp(&a.0));

    let ordered_paragraphs: Vec<(usize, &PdfParagraph)> = paragraphs
        .iter()
        .enumerate()
        .filter(|(_, para)| para.caption_for.is_none())
        .collect();
    let mut tables_at_slot: Vec<Vec<&crate::types::Table>> =
        (0..=ordered_paragraphs.len()).map(|_| Vec::new()).collect();

    for (table_y, table) in positioned {
        let geometric_slot = table_insertion_slot(&ordered_paragraphs, table, table_y);
        let slot = top_level_table_slot(&ordered_paragraphs, geometric_slot, table_y);
        tables_at_slot[slot].push(table);
    }

    let mut in_list = false;
    let mut open_regions = Vec::new();
    let mut paragraph_elem_map = Vec::new();

    for (slot, slot_tables) in tables_at_slot.into_iter().enumerate() {
        for table in slot_tables {
            close_list(builder, &mut in_list);
            close_layout_path(builder, &mut open_regions);
            push_table_element(builder, table, page);
        }

        let Some(&(para_idx, para)) = ordered_paragraphs.get(slot) else {
            continue;
        };

        let transition_index = builder.element_count();
        transition_layout_path(
            builder,
            &mut open_regions,
            effective_layout_path(para),
            &mut in_list,
            page,
        );

        if para.is_list_item && !in_list {
            builder.push_list(list_item_is_ordered(para));
            in_list = true;
        } else if !para.is_list_item && in_list {
            builder.end_list();
            in_list = false;
        }

        let elem_idx = push_paragraph_element(builder, para, page, hyphen_witnesses);
        paragraph_elem_map.push(ParagraphElementPosition {
            paragraph_index: para_idx,
            element_index: elem_idx,
            transition_index,
        });
        emit_caption_elements(builder, paragraphs, para_idx, page, elem_idx);
    }

    let page_end_transition_index = builder.element_count();
    close_list(builder, &mut in_list);
    close_layout_path(builder, &mut open_regions);

    for table in unpositioned {
        push_table_element(builder, table, page);
    }

    (paragraph_elem_map, page_end_transition_index)
}

fn close_list(builder: &mut InternalDocumentBuilder, in_list: &mut bool) {
    if *in_list {
        builder.end_list();
        *in_list = false;
    }
}

fn close_layout_path(builder: &mut InternalDocumentBuilder, open_regions: &mut Vec<LayoutRegionTag>) {
    for _ in 0..open_regions.len() {
        builder.push_group_end();
    }
    open_regions.clear();
}

/// List structure already groups adjacent list items. Per-item `ListItem`
/// detections must not break that list into one-item containers. Remove only
/// that per-item region while preserving Text and wrapper ancestry, since those
/// regions distinguish independent lists in separate columns or containers.
fn effective_layout_path(paragraph: &PdfParagraph) -> Option<LayoutRegionPath> {
    let path = paragraph.layout_region_path?;
    if !paragraph.is_list_item {
        return Some(path);
    }

    if path.root.class_name == Some(LayoutHintClass::ListItem) {
        return path.child.map(|root| LayoutRegionPath { root, child: None });
    }
    if path
        .child
        .is_some_and(|child| child.class_name == Some(LayoutHintClass::ListItem))
    {
        return Some(LayoutRegionPath {
            root: path.root,
            child: None,
        });
    }
    Some(path)
}

fn transition_layout_path(
    builder: &mut InternalDocumentBuilder,
    open_regions: &mut Vec<LayoutRegionTag>,
    next_path: Option<LayoutRegionPath>,
    in_list: &mut bool,
    page: Option<u32>,
) {
    let next_regions = next_path
        .into_iter()
        .flat_map(LayoutRegionPath::tags)
        .collect::<Vec<_>>();
    let common_prefix = open_regions
        .iter()
        .zip(&next_regions)
        .take_while(|(open, next)| open == next)
        .count();
    if common_prefix == open_regions.len() && common_prefix == next_regions.len() {
        return;
    }

    close_list(builder, in_list);
    for _ in common_prefix..open_regions.len() {
        builder.push_group_end();
    }
    open_regions.truncate(common_prefix);

    for region in &next_regions[common_prefix..] {
        let class_label = region.class_name.map_or("uncovered", LayoutHintClass::label);
        let label = format!("pdf-layout:p{}:r{}:{class_label}", page.unwrap_or_default(), region.id);
        builder.push_group_start(Some(&label), page);
        open_regions.push(*region);
    }
}

/// Keep tables as top-level roots in the final page plan.
///
/// A geometric insertion point can fall between paragraphs that share one
/// layout root. Emitting there would duplicate that root around the table. Move
/// the table to the cheaper edge of the root run so every detected region and
/// table is emitted exactly once at the top level.
fn top_level_table_slot(paragraphs: &[(usize, &PdfParagraph)], slot: usize, table_y: f32) -> usize {
    if slot == 0 || slot >= paragraphs.len() {
        return slot;
    }
    let root_before = effective_layout_path(paragraphs[slot - 1].1).map(|path| path.root);
    let root_after = effective_layout_path(paragraphs[slot].1).map(|path| path.root);
    let Some(root) = root_before.filter(|before| Some(*before) == root_after) else {
        return slot;
    };

    let mut run_start = slot;
    while run_start > 0 && effective_layout_path(paragraphs[run_start - 1].1).map(|path| path.root) == Some(root) {
        run_start -= 1;
    }
    let mut run_end = slot;
    while run_end < paragraphs.len() && effective_layout_path(paragraphs[run_end].1).map(|path| path.root) == Some(root)
    {
        run_end += 1;
    }

    let before_cost = table_slot_displacement(&paragraphs[run_start..slot], table_y);
    let after_cost = table_slot_displacement(&paragraphs[slot..run_end], table_y);
    if before_cost <= after_cost { run_start } else { run_end }
}

fn table_slot_displacement(paragraphs: &[(usize, &PdfParagraph)], table_y: f32) -> f32 {
    paragraphs
        .iter()
        .filter_map(|(_, paragraph)| paragraph_vertical_anchor(paragraph))
        .map(|paragraph_y| (paragraph_y - table_y).abs())
        .sum()
}

/// Pick a reading-order boundary immediately before text below the table.
///
/// When every paragraph has horizontal geometry and the table overlaps only a
/// subset, the subset identifies the table's column. Full-width tables and pages
/// with incomplete geometry fall back to the complete paragraph sequence. Neither
/// path changes the established paragraph subsequence.
fn table_insertion_slot(paragraphs: &[(usize, &PdfParagraph)], table: &crate::types::Table, table_y: f32) -> usize {
    let fallback = || {
        vertical_insertion_slot(
            paragraphs
                .iter()
                .enumerate()
                .map(|(slot, &(_, paragraph))| (slot, paragraph)),
            table_y,
            paragraphs.len(),
        )
    };
    let Some(table_bbox) = table.bounding_box else {
        return fallback();
    };

    let mut overlapping = Vec::new();
    let mut has_non_overlapping_paragraph = false;
    for (slot, &(_, paragraph)) in paragraphs.iter().enumerate() {
        let Some((left, right)) = paragraph_horizontal_bounds(paragraph) else {
            return fallback();
        };
        if horizontal_ranges_overlap(table_bbox.x0 as f32, table_bbox.x1 as f32, left, right) {
            overlapping.push((slot, paragraph));
        } else {
            has_non_overlapping_paragraph = true;
        }
    }

    if overlapping.is_empty() || !has_non_overlapping_paragraph {
        return fallback();
    }

    let end_slot = overlapping.last().map_or(paragraphs.len(), |(slot, _)| slot + 1);
    vertical_insertion_slot(
        overlapping.iter().map(|(slot, paragraph)| (*slot, *paragraph)),
        table_y,
        end_slot,
    )
}

fn vertical_insertion_slot<'a>(
    mut paragraphs: impl Iterator<Item = (usize, &'a PdfParagraph)>,
    table_y: f32,
    end_slot: usize,
) -> usize {
    paragraphs
        .find_map(|(slot, paragraph)| {
            paragraph_vertical_anchor(paragraph)
                .is_some_and(|paragraph_y| paragraph_y < table_y)
                .then_some(slot)
        })
        .unwrap_or(end_slot)
}

fn paragraph_horizontal_bounds(paragraph: &PdfParagraph) -> Option<(f32, f32)> {
    if let Some((left, _, right, _)) = paragraph.block_bbox
        && left.is_finite()
        && right.is_finite()
        && right > left
    {
        return Some((left, right));
    }

    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;
    for segment in paragraph.lines.iter().flat_map(|line| &line.segments) {
        left = left.min(segment.x);
        right = right.max(segment.x + segment.width);
    }
    (left.is_finite() && right.is_finite() && right > left).then_some((left, right))
}

fn horizontal_ranges_overlap(first_left: f32, first_right: f32, second_left: f32, second_right: f32) -> bool {
    first_left.is_finite()
        && first_right.is_finite()
        && first_right > first_left
        && first_left < second_right
        && first_right > second_left
}

fn paragraph_vertical_anchor(paragraph: &PdfParagraph) -> Option<f32> {
    paragraph
        .block_bbox
        .map(|(_, _, _, top)| top)
        .or_else(|| paragraph.lines.first().map(|line| line.baseline_y))
        .filter(|anchor| anchor.is_finite())
}

fn push_table_element(builder: &mut InternalDocumentBuilder, table: &crate::types::Table, page: Option<u32>) -> u32 {
    let bbox = table.bounding_box.map(|bb| BoundingBox {
        x0: bb.x0,
        y0: bb.y0,
        x1: bb.x1,
        y1: bb.y1,
    });
    builder.push_table(table.clone(), page, bbox)
}

/// Convert a single PdfParagraph to the appropriate InternalElement and push it.
/// Returns the element index.
fn push_paragraph_element(
    builder: &mut InternalDocumentBuilder,
    para: &PdfParagraph,
    page: Option<u32>,
    hyphen_witnesses: &super::pipeline::HyphenWitnesses,
) -> u32 {
    let bbox = para.block_bbox.map(|bb| BoundingBox {
        x0: bb.0 as f64,
        y0: bb.1 as f64,
        x1: bb.2 as f64,
        y1: bb.3 as f64,
    });

    tracing::debug!(
        heading = ?para.heading_level,
        list = para.is_list_item,
        code = para.is_code_block,
        formula = para.is_formula,
        furniture = para.is_page_furniture,
        bold = para.is_bold,
        font_size = para.dominant_font_size,
        has_text = !para.text.is_empty(),
        page = ?page,
        "emitting element"
    );

    let get_text = |para: &PdfParagraph| -> String {
        let text = if !para.text.is_empty() {
            para.text.clone()
        } else {
            join_line_texts_plain(&para.lines, hyphen_witnesses)
        };
        finalize_hyphens(&text).into_owned()
    };

    if let Some(level) = para.heading_level {
        let text = get_text(para);
        return if para.layout_region_path.is_some() {
            builder.push_heading_in_current_container(level, &text, page, bbox)
        } else {
            builder.push_heading(level, &text, page, bbox)
        };
    }

    if para.is_code_block {
        let text = if !para.text.is_empty() {
            para.text.clone()
        } else {
            para.lines
                .iter()
                .map(|l| {
                    let line_text = l.segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
                    collapse_inner_spaces(&line_text).into_owned()
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        return builder.push_code(&text, None, page, bbox);
    }

    if para.is_formula {
        let text = get_text(para);
        return builder.push_formula(&text, page, bbox);
    }

    if para.is_list_item {
        let ordered = list_item_is_ordered(para);
        let text = get_text(para);
        let annotations = if para.text.is_empty() {
            let (annotated_text, annotations) = extract_text_and_annotations(para, hyphen_witnesses);
            if annotated_text == text {
                annotations
            } else {
                Vec::new()
            }
        } else {
            para.is_bold
                .then_some(TextAnnotation {
                    start: 0,
                    end: text.len() as u32,
                    kind: AnnotationKind::Bold,
                })
                .into_iter()
                .collect()
        };
        let (normalized, removed_prefix_len) = normalize_list_text(&text);
        let annotations = shift_annotations_after_prefix_removal(annotations, removed_prefix_len, normalized.len());
        let element_id = builder.push_list_item(normalized, ordered, annotations, page, bbox);
        // The marker just stripped off `text` is the document's own label. Keep it:
        // an ordinance is cross-referenced by printed clause number, so a
        // synthesized sequence position is not an acceptable substitute.
        if ordered && removed_prefix_len > 0 {
            builder.set_list_item_source_label(element_id, text[..removed_prefix_len].trim());
        }
        return element_id;
    }

    if para.is_page_furniture {
        let text = get_text(para);
        let layer = guess_furniture_layer(para);
        let elem_idx = builder.push_paragraph(&text, vec![], page, bbox);
        builder.set_layer(elem_idx, layer);
        return elem_idx;
    }

    if matches!(para.layout_class, Some(LayoutHintClass::Caption)) {
        let text = get_text(para);
        let annotations = vec![TextAnnotation {
            start: 0,
            end: text.len() as u32,
            kind: AnnotationKind::Italic,
        }];
        return builder.push_paragraph(&text, annotations, page, bbox);
    }

    if !para.text.is_empty() {
        let annotations = if para.is_bold {
            vec![TextAnnotation {
                start: 0,
                end: para.text.len() as u32,
                kind: AnnotationKind::Bold,
            }]
        } else {
            vec![]
        };
        builder.push_paragraph(&para.text, annotations, page, bbox)
    } else {
        let (text, annotations) = extract_text_and_annotations(para, hyphen_witnesses);
        builder.push_paragraph(&text, annotations, page, bbox)
    }
}

/// Emit caption elements as paragraphs with a Caption relationship to the parent.
fn emit_caption_elements(
    builder: &mut InternalDocumentBuilder,
    paragraphs: &[PdfParagraph],
    parent_idx: usize,
    page: Option<u32>,
    parent_elem_idx: u32,
) {
    for para in paragraphs {
        if para.caption_for == Some(parent_idx) {
            let text: String = para
                .lines
                .iter()
                .flat_map(|l| l.segments.iter())
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                let annotations = vec![TextAnnotation {
                    start: 0,
                    end: trimmed.len() as u32,
                    kind: AnnotationKind::Italic,
                }];
                let bbox = para.block_bbox.map(|bb| BoundingBox {
                    x0: bb.0 as f64,
                    y0: bb.1 as f64,
                    x1: bb.2 as f64,
                    y1: bb.3 as f64,
                });
                let caption_idx = builder.push_paragraph(trimmed, annotations, page, bbox);
                builder.push_relationship(
                    caption_idx,
                    RelationshipTarget::Index(parent_elem_idx),
                    RelationshipKind::Caption,
                );
            }
        }
    }
}

/// Extract plain text and inline annotations (bold/italic) from a paragraph.
///
/// Walks segments, groups consecutive runs of the same bold/italic state,
/// and produces `TextAnnotation` spans for formatting changes.
fn extract_text_and_annotations(
    para: &PdfParagraph,
    hyphen_witnesses: &super::pipeline::HyphenWitnesses,
) -> (String, Vec<TextAnnotation>) {
    let all_segments: Vec<&crate::pdf::hierarchy::SegmentData> = para.lines.iter().flat_map(|l| &l.segments).collect();

    if all_segments.is_empty() {
        return (String::new(), Vec::new());
    }

    let mut text = String::new();
    let mut annotations = Vec::new();
    let mut i = 0;

    while i < all_segments.len() {
        let bold = all_segments[i].is_bold;
        let italic = all_segments[i].is_italic;

        let run_start = i;
        while i < all_segments.len() && all_segments[i].is_bold == bold && all_segments[i].is_italic == italic {
            i += 1;
        }

        let mut run_words: Vec<(&str, usize)> = Vec::new();
        for (seg_offset, seg) in all_segments[run_start..i].iter().enumerate() {
            let seg_idx = run_start + seg_offset;
            for word in seg.text.split_whitespace() {
                run_words.push((word, seg_idx));
            }
        }

        if !text.is_empty() && !run_words.is_empty() {
            let prev_seg = all_segments[run_start - 1];
            let next_seg = all_segments[run_start];
            let prev_last = prev_seg.text.split_whitespace().next_back().unwrap_or("");
            let next_first = next_seg.text.split_whitespace().next().unwrap_or("");

            match classify_line_break_hyphen(prev_last, next_first, prev_seg, next_seg, hyphen_witnesses) {
                LineBreakHyphen::Weld => {
                    text.pop();
                }
                LineBreakHyphen::KeepHyphen => {}
                LineBreakHyphen::NotApplicable => {
                    if segments_need_space(prev_seg, prev_last, next_seg, next_first) {
                        text.push(' ');
                    }
                }
            }
        }

        let span_start = text.len();

        for (wi, &(word, seg_idx)) in run_words.iter().enumerate() {
            if wi > 0 {
                let (prev, prev_seg_idx) = run_words[wi - 1];
                match classify_line_break_hyphen(
                    prev,
                    word,
                    all_segments[prev_seg_idx],
                    all_segments[seg_idx],
                    hyphen_witnesses,
                ) {
                    LineBreakHyphen::Weld => {
                        text.pop();
                    }
                    LineBreakHyphen::KeepHyphen => {}
                    LineBreakHyphen::NotApplicable => {
                        if prev_seg_idx == seg_idx {
                            if needs_space_between(prev, word) {
                                text.push(' ');
                            }
                        } else if segments_need_space(all_segments[prev_seg_idx], prev, all_segments[seg_idx], word) {
                            text.push(' ');
                        }
                    }
                }
            }
            text.push_str(word);
        }

        let span_end = text.len();

        if span_start < span_end {
            if bold {
                annotations.push(TextAnnotation {
                    start: span_start as u32,
                    end: span_end as u32,
                    kind: AnnotationKind::Bold,
                });
            }
            if italic {
                annotations.push(TextAnnotation {
                    start: span_start as u32,
                    end: span_end as u32,
                    kind: AnnotationKind::Italic,
                });
            }
        }
    }

    (text, annotations)
}

/// Join line texts into a single plain string (no markup).
fn join_line_texts_plain(
    lines: &[super::types::PdfLine],
    hyphen_witnesses: &super::pipeline::HyphenWitnesses,
) -> String {
    if lines.is_empty() {
        return String::new();
    }

    let words_per_line: Vec<Vec<(&str, &crate::pdf::hierarchy::SegmentData)>> = lines
        .iter()
        .map(|line| {
            line.segments
                .iter()
                .flat_map(|segment| segment.text.split_whitespace().map(move |word| (word, segment)))
                .collect()
        })
        .collect();

    let mut result = String::new();
    for (line_idx, line_words) in words_per_line.iter().enumerate() {
        for (word_idx, &(word, seg)) in line_words.iter().enumerate() {
            if result.is_empty() {
                result.push_str(word);
                continue;
            }

            let prev = if word_idx > 0 {
                Some(line_words[word_idx - 1])
            } else {
                words_per_line[..line_idx]
                    .iter()
                    .rev()
                    .find_map(|lw| lw.last().copied())
            };

            let Some((prev_word, prev_seg)) = prev else {
                result.push_str(word);
                continue;
            };

            match classify_line_break_hyphen(prev_word, word, prev_seg, seg, hyphen_witnesses) {
                LineBreakHyphen::Weld => {
                    result.pop();
                    result.push_str(word);
                    continue;
                }
                LineBreakHyphen::KeepHyphen => {
                    result.push_str(word);
                    continue;
                }
                LineBreakHyphen::NotApplicable => {}
            }

            let insert_space = if std::ptr::eq(prev_seg, seg) {
                needs_space_between(prev_word, word)
            } else {
                segments_need_space(prev_seg, prev_word, seg, word)
            };
            if insert_space {
                result.push(' ');
            }
            result.push_str(word);
        }
    }
    result
}

/// Check if a line-ending hyphen should be removed and words joined.
///
/// Requires `prev_seg`/`next_seg` to actually cross a visual line break (xberg-io/xberg#1581):
/// without that check, a suspended hyphen mid-line ("onderhouds- en") matches the same text
/// pattern as a genuine wrapped-line hyphen and gets welded regardless of position.
/// How a hyphen that ends a visual line joins to the fragment after the break.
///
/// Two different things happen at a line break and the positional signals cannot tell them
/// apart: a wrap that split a word (`Mon-` + `tanide`), and a compound whose own hyphen simply
/// landed there (`long-` + `term`). Collapsing them into one boolean is what welded real
/// compounds into tokens that do not exist (xberg-io/xberg#1613). `KeepHyphen` is a distinct
/// outcome from `NotApplicable`: the pair still joins with no space between them, it is only the
/// hyphen that survives. ~keep
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineBreakHyphen {
    /// Not a hyphen-at-a-break pair; normal spacing rules apply.
    NotApplicable,
    /// A wrap split one word: drop the hyphen and join the halves.
    Weld,
    /// The compound's own hyphen: keep it, and join without inserting a space.
    KeepHyphen,
}

fn classify_line_break_hyphen(
    prev: &str,
    next: &str,
    prev_seg: &crate::pdf::hierarchy::SegmentData,
    next_seg: &crate::pdf::hierarchy::SegmentData,
    hyphen_witnesses: &super::pipeline::HyphenWitnesses,
) -> LineBreakHyphen {
    if prev.len() < 2 || !prev.ends_with('-') {
        return LineBreakHyphen::NotApplicable;
    }
    let before_hyphen = prev[..prev.len() - 1].chars().next_back();
    if !before_hyphen.is_some_and(|c| c.is_alphabetic()) {
        return LineBreakHyphen::NotApplicable;
    }
    if !next.chars().next().is_some_and(|c| c.is_lowercase()) {
        return LineBreakHyphen::NotApplicable;
    }
    if !crosses_visual_line_break(prev_seg, next_seg) {
        return LineBreakHyphen::NotApplicable;
    }
    // The four conditions above are positional and typographic; they separate a hyphen at the
    // break from one mid-line, but cannot separate the two things that both happen at a break:
    // a wrap that split a word, and a compound whose own hyphen landed there. That is what
    // `should_preserve_lexical_hyphen` weighs, and this site used to decide without it
    // (xberg-io/xberg#1613). Arguments are prepared exactly as `dehyphenate_paragraph_lines`
    // prepares them -- the guard trims non-lexical characters but deliberately keeps `-`, so
    // the trailing hyphen must come off here. ~keep
    let trailing_word = prev.trim_end_matches('-').split_whitespace().last().unwrap_or("");
    let leading_word = next.split_whitespace().next().unwrap_or("");
    if super::pipeline::should_preserve_lexical_hyphen(trailing_word, leading_word, hyphen_witnesses) {
        LineBreakHyphen::KeepHyphen
    } else {
        LineBreakHyphen::Weld
    }
}

/// Collapse runs of 2+ spaces inside a line while preserving leading indentation.
fn collapse_inner_spaces(line: &str) -> Cow<'_, str> {
    let leading = line.len() - line.trim_start_matches(' ').len();
    let prefix = &line[..leading];
    let rest = &line[leading..];
    if !rest.contains("  ") {
        return Cow::Borrowed(line);
    }
    let mut result = String::with_capacity(line.len());
    result.push_str(prefix);
    let mut prev_space = false;
    for ch in rest.chars() {
        if ch == ' ' {
            if !prev_space {
                result.push(ch);
            }
            prev_space = true;
        } else {
            prev_space = false;
            result.push(ch);
        }
    }
    Cow::Owned(result)
}

/// True when a list-item paragraph carries a numbered marker ("1." / "3)").
///
/// Determines both the item's `ordered` flag and the ordered-ness of the list
/// container opened for a run of items, so numbered lists render as "1." /
/// "2." instead of degrading to bullets.
fn list_item_is_ordered(para: &PdfParagraph) -> bool {
    let first_line_text;
    let text = if !para.text.is_empty() {
        para.text.as_str()
    } else {
        first_line_text = para
            .lines
            .first()
            .and_then(|l| l.segments.first())
            .map(|s| s.text.clone())
            .unwrap_or_default();
        first_line_text.as_str()
    };
    super::list_marker::parse_ordered_list_marker(text).is_some()
}

/// Strip a bullet/number prefix and return the clean suffix plus its byte offset.
fn normalize_list_text(text: &str) -> (&str, usize) {
    if let Some(marker) = super::list_marker::parse_ordered_list_marker(text) {
        let normalized = &text[marker.content_start..];
        return (normalized, marker.content_start);
    }
    let trimmed = text.trim_start();
    const BULLET_CHARS: &[char] = &['\u{2022}', '\u{00B7}'];
    let mut normalized = trimmed;
    for &ch in BULLET_CHARS {
        if trimmed.starts_with(ch) {
            normalized = trimmed[ch.len_utf8()..].trim_start();
            return (normalized, text.len() - normalized.len());
        }
    }
    if let Some(stripped) = trimmed.strip_prefix("* ") {
        normalized = stripped.trim_start();
        return (normalized, text.len() - normalized.len());
    }
    if let Some(stripped) = trimmed.strip_prefix("- ") {
        normalized = stripped;
        return (normalized, text.len() - normalized.len());
    }
    const DASH_BULLETS: &[char] = &['–', '—', '−', '‐', '‑', '‒', '―', '➤', '►', '▶', '○', '●', '◦'];
    for &ch in DASH_BULLETS {
        if trimmed.starts_with(ch) {
            normalized = trimmed[ch.len_utf8()..].trim_start();
            return (normalized, text.len() - normalized.len());
        }
    }
    let bytes = trimmed.as_bytes();
    let digit_end = bytes.iter().position(|&b| !b.is_ascii_digit()).unwrap_or(0);
    if digit_end > 0 && digit_end < bytes.len() {
        let suffix = bytes[digit_end];
        if suffix == b'.' || suffix == b')' {
            let after = &trimmed[digit_end + 1..];
            normalized = after.trim_start();
            return (normalized, text.len() - normalized.len());
        }
    }
    (normalized, text.len() - normalized.len())
}

fn shift_annotations_after_prefix_removal(
    annotations: Vec<TextAnnotation>,
    removed_prefix_len: usize,
    normalized_len: usize,
) -> Vec<TextAnnotation> {
    let removed_prefix_len = removed_prefix_len.min(u32::MAX as usize) as u32;
    let normalized_len = normalized_len.min(u32::MAX as usize) as u32;
    annotations
        .into_iter()
        .filter_map(|mut annotation| {
            annotation.start = annotation.start.saturating_sub(removed_prefix_len).min(normalized_len);
            annotation.end = annotation.end.saturating_sub(removed_prefix_len).min(normalized_len);
            (annotation.start < annotation.end).then_some(annotation)
        })
        .collect()
}

/// Guess whether page furniture is a header or footer based on vertical position.
fn guess_furniture_layer(para: &PdfParagraph) -> ContentLayer {
    match para.layout_class {
        Some(LayoutHintClass::PageHeader) => ContentLayer::Header,
        Some(LayoutHintClass::PageFooter) => ContentLayer::Footer,
        Some(LayoutHintClass::Footnote) => ContentLayer::Footnote,
        _ => {
            if let Some(first_line) = para.lines.first() {
                if first_line.baseline_y > 700.0 {
                    ContentLayer::Header
                } else if first_line.baseline_y < 100.0 {
                    ContentLayer::Footer
                } else {
                    ContentLayer::Header
                }
            } else {
                ContentLayer::Header
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::pdf::hierarchy::SegmentData;

    use super::super::types::PdfLine;
    use super::*;

    fn plain_segment(text: &str) -> SegmentData {
        SegmentData {
            text: text.to_string(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 12.0,
            font_size: 12.0,
            is_bold: false,
            is_italic: false,
            is_monospace: false,
            baseline_y: 700.0,
            rotation_degrees: 0.0,
            assigned_role: None,
        }
    }

    fn make_paragraph(text: &str, heading_level: Option<u8>) -> PdfParagraph {
        make_paragraph_at(text, heading_level, 700.0)
    }

    fn make_paragraph_at(text: &str, heading_level: Option<u8>, baseline_y: f32) -> PdfParagraph {
        let lines = vec![PdfLine {
            segments: vec![SegmentData {
                baseline_y,
                ..plain_segment(text)
            }],
            baseline_y,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        }];
        let word_count = PdfParagraph::compute_word_count("", &lines);
        PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: 12.0,
            heading_level,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        }
    }

    fn make_paragraph_in_box(text: &str, baseline_y: f32, left: f32, right: f32) -> PdfParagraph {
        let mut paragraph = make_paragraph_at(text, None, baseline_y);
        paragraph.block_bbox = Some((left, baseline_y - 12.0, right, baseline_y));
        paragraph
    }

    fn make_table_at(markdown: &str, top_y: f64) -> crate::types::Table {
        make_table_in_box(markdown, 40.0, 560.0, top_y)
    }

    fn make_table_in_box(markdown: &str, left: f64, right: f64, top_y: f64) -> crate::types::Table {
        crate::types::Table {
            cells: vec![],
            markdown: markdown.to_string(),
            page_number: 1,
            bounding_box: Some(crate::types::BoundingBox {
                x0: left,
                y0: top_y - 80.0,
                x1: right,
                y1: top_y,
            }),
            ..Default::default()
        }
    }

    fn page_element_labels(document: &InternalDocument) -> Vec<&str> {
        document
            .elements
            .iter()
            .filter_map(|element| match &element.kind {
                ElementKind::Paragraph => Some(element.text.as_str()),
                ElementKind::Table { .. } => Some("<table>"),
                _ => None,
            })
            .collect()
    }

    fn nested_layout_path() -> LayoutRegionPath {
        LayoutRegionPath {
            root: LayoutRegionTag {
                id: 3,
                class_name: Some(LayoutHintClass::Form),
            },
            child: Some(LayoutRegionTag {
                id: 7,
                class_name: Some(LayoutHintClass::Text),
            }),
        }
    }

    #[test]
    fn layout_paths_emit_balanced_nested_groups() {
        let mut first = make_paragraph("first", None);
        first.layout_region_path = Some(nested_layout_path());
        let mut second = make_paragraph("second", None);
        second.layout_region_path = Some(nested_layout_path());

        let document = assemble_internal_document(vec![vec![first, second]], &[], None, &[], &Default::default());
        let starts = document
            .elements
            .iter()
            .filter(|element| element.kind == ElementKind::GroupStart)
            .collect::<Vec<_>>();
        let ends = document
            .elements
            .iter()
            .filter(|element| element.kind == ElementKind::GroupEnd)
            .collect::<Vec<_>>();
        assert_eq!(starts.len(), 2);
        assert_eq!(ends.len(), 2);
        assert_eq!(starts[0].depth, 0);
        assert_eq!(starts[1].depth, 1);
        assert_eq!(page_element_labels(&document), ["first", "second"]);
    }

    #[test]
    fn heading_levels_remain_relative_inside_nested_layout_groups() {
        let paragraphs = [
            make_paragraph("h1", Some(1)),
            make_paragraph("h2", Some(2)),
            make_paragraph("h3", Some(3)),
        ]
        .into_iter()
        .map(|mut paragraph| {
            paragraph.layout_region_path = Some(nested_layout_path());
            paragraph
        })
        .collect::<Vec<_>>();

        let document = assemble_internal_document(vec![paragraphs], &[], None, &[], &Default::default());
        let heading_depths = document
            .elements
            .iter()
            .filter_map(|element| matches!(element.kind, ElementKind::Heading { .. }).then_some(element.depth))
            .collect::<Vec<_>>();
        assert_eq!(heading_depths, [2, 3, 4]);
    }

    #[test]
    fn adjacent_layout_list_items_share_one_list() {
        let mut first = make_paragraph("- first", None);
        first.is_list_item = true;
        first.layout_region_path = Some(LayoutRegionPath {
            root: LayoutRegionTag {
                id: 10,
                class_name: Some(LayoutHintClass::ListItem),
            },
            child: None,
        });
        let mut second = make_paragraph("- second", None);
        second.is_list_item = true;
        second.layout_region_path = Some(LayoutRegionPath {
            root: LayoutRegionTag {
                id: 11,
                class_name: Some(LayoutHintClass::ListItem),
            },
            child: None,
        });

        let document = assemble_internal_document(vec![vec![first, second]], &[], None, &[], &Default::default());
        assert_eq!(
            document
                .elements
                .iter()
                .filter(|element| matches!(element.kind, ElementKind::ListStart { .. }))
                .count(),
            1
        );
        assert_eq!(
            document
                .elements
                .iter()
                .filter(|element| element.kind == ElementKind::ListEnd)
                .count(),
            1
        );
        assert_eq!(
            document
                .elements
                .iter()
                .filter(|element| matches!(element.kind, ElementKind::ListItem { .. }))
                .count(),
            2
        );
        assert!(
            document
                .elements
                .iter()
                .all(|element| !matches!(element.kind, ElementKind::GroupStart | ElementKind::GroupEnd))
        );
    }

    #[test]
    fn distinct_text_regions_keep_layout_lists_separate() {
        let paragraphs = [("- left", 20), ("- right", 21)]
            .into_iter()
            .map(|(text, id)| {
                let mut paragraph = make_paragraph(text, None);
                paragraph.is_list_item = true;
                paragraph.layout_region_path = Some(LayoutRegionPath {
                    root: LayoutRegionTag {
                        id: 3,
                        class_name: Some(LayoutHintClass::Form),
                    },
                    child: Some(LayoutRegionTag {
                        id,
                        class_name: Some(LayoutHintClass::Text),
                    }),
                });
                paragraph
            })
            .collect::<Vec<_>>();

        let document = assemble_internal_document(vec![paragraphs], &[], None, &[], &Default::default());
        assert_eq!(
            document
                .elements
                .iter()
                .filter(|element| matches!(element.kind, ElementKind::ListStart { .. }))
                .count(),
            2
        );
        assert_eq!(
            document
                .elements
                .iter()
                .filter(|element| element.kind == ElementKind::ListEnd)
                .count(),
            2
        );
    }

    #[test]
    fn table_is_an_exact_once_root_beside_a_repeated_region_path() {
        let mut above = make_paragraph_at("above", None, 700.0);
        above.layout_region_path = Some(nested_layout_path());
        let mut below = make_paragraph_at("below", None, 400.0);
        below.layout_region_path = Some(nested_layout_path());
        let table = make_table_at("| a |\n|---|", 600.0);

        let document = assemble_internal_document(vec![vec![above, below]], &[table], None, &[], &Default::default());
        assert_eq!(page_element_labels(&document), ["<table>", "above", "below"]);
        let table_element = document
            .elements
            .iter()
            .find(|element| matches!(element.kind, ElementKind::Table { .. }))
            .unwrap();
        assert_eq!(table_element.depth, 0);
        assert_eq!(
            document
                .elements
                .iter()
                .filter(|element| element.kind == ElementKind::GroupStart)
                .count(),
            2
        );
        assert_eq!(
            document
                .elements
                .iter()
                .filter(|element| element.kind == ElementKind::GroupEnd)
                .count(),
            2
        );
    }

    #[test]
    fn no_layout_path_emits_no_group_markers() {
        let document = assemble_internal_document(
            vec![vec![make_paragraph("legacy", None)]],
            &[],
            None,
            &[],
            &Default::default(),
        );
        assert!(
            document
                .elements
                .iter()
                .all(|element| !matches!(element.kind, ElementKind::GroupStart | ElementKind::GroupEnd))
        );
        assert_eq!(page_element_labels(&document), ["legacy"]);
    }

    #[test]
    fn test_assemble_internal_document_basic() {
        let pages = vec![vec![
            make_paragraph("Title", Some(1)),
            make_paragraph("Body text", None),
        ]];
        let doc = assemble_internal_document(pages, &[], None, &[], &Default::default());
        assert_eq!(doc.elements.len(), 2);
        assert!(matches!(doc.elements[0].kind, ElementKind::Heading { level: 1 }));
        assert_eq!(doc.elements[0].text, "Title");
        assert!(matches!(doc.elements[1].kind, ElementKind::Paragraph));
        assert_eq!(doc.elements[1].text, "Body text");
    }

    #[test]
    fn test_assemble_internal_document_empty() {
        let doc = assemble_internal_document(vec![], &[], None, &[], &Default::default());
        assert!(doc.elements.is_empty());
    }

    #[test]
    fn test_assemble_internal_document_multiple_pages() {
        let pages = vec![
            vec![make_paragraph("Page 1", None)],
            vec![make_paragraph("Page 2", None)],
        ];
        let doc = assemble_internal_document(pages, &[], None, &[], &Default::default());
        let paragraphs: Vec<_> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Paragraph))
            .collect();
        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraphs[0].text, "Page 1");
        assert_eq!(paragraphs[1].text, "Page 2");
    }

    #[test]
    fn test_assemble_with_tables_no_bbox() {
        let pages = vec![vec![make_paragraph("Before", None)]];
        let tables = vec![crate::types::Table {
            cells: vec![],
            markdown: "| A | B |\n|---|---|\n| 1 | 2 |".to_string(),
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        }];
        let doc = assemble_internal_document(pages, &tables, None, &[], &Default::default());
        assert!(doc.elements.iter().any(|e| e.text == "Before"));
        assert!(doc.tables.iter().any(|t| t.markdown.contains("| A | B |")));
    }

    #[test]
    fn test_assemble_with_tables_multipage() {
        let pages = vec![
            vec![make_paragraph("Page 1", None)],
            vec![make_paragraph("Page 2", None)],
        ];
        let tables = vec![crate::types::Table {
            cells: vec![],
            markdown: "| Table |".to_string(),
            page_number: 2,
            bounding_box: None,
            ..Default::default()
        }];
        let doc = assemble_internal_document(pages, &tables, None, &[], &Default::default());
        assert!(doc.elements.iter().any(|e| e.text == "Page 1"));
        assert!(doc.elements.iter().any(|e| e.text == "Page 2"));
        assert!(doc.tables.iter().any(|t| t.markdown.contains("| Table |")));
    }

    #[test]
    fn test_single_column_table_is_interleaved_by_vertical_position() {
        let pages = vec![vec![
            make_paragraph_in_box("Before", 900.0, 40.0, 560.0),
            make_paragraph_in_box("After", 700.0, 40.0, 560.0),
        ]];
        let tables = vec![make_table_in_box("| Between |", 80.0, 520.0, 800.0)];

        let document = assemble_internal_document(pages, &tables, None, &[], &Default::default());

        assert_eq!(page_element_labels(&document), ["Before", "<table>", "After"]);
    }

    #[test]
    fn test_full_width_table_preserves_two_column_paragraph_order() {
        let pages = vec![vec![
            make_paragraph_in_box("Left top", 900.0, 40.0, 260.0),
            make_paragraph_in_box("Left bottom", 700.0, 40.0, 260.0),
            make_paragraph_in_box("Right top", 880.0, 340.0, 560.0),
            make_paragraph_in_box("Right bottom", 680.0, 340.0, 560.0),
        ]];
        let tables = vec![make_table_at("| Full width |", 800.0)];

        let document = assemble_internal_document(pages, &tables, None, &[], &Default::default());

        assert_eq!(
            page_element_labels(&document),
            ["Left top", "<table>", "Left bottom", "Right top", "Right bottom"]
        );
    }

    #[test]
    fn test_right_column_table_uses_right_column_vertical_boundary() {
        let pages = vec![vec![
            make_paragraph_in_box("Left top", 900.0, 40.0, 260.0),
            make_paragraph_in_box("Left bottom", 700.0, 40.0, 260.0),
            make_paragraph_in_box("Right top", 880.0, 340.0, 560.0),
            make_paragraph_in_box("Right bottom", 680.0, 340.0, 560.0),
        ]];
        let tables = vec![make_table_in_box("| Right column |", 350.0, 550.0, 800.0)];

        let document = assemble_internal_document(pages, &tables, None, &[], &Default::default());

        assert_eq!(
            page_element_labels(&document),
            ["Left top", "Left bottom", "Right top", "<table>", "Right bottom"]
        );
    }

    #[test]
    fn test_incomplete_paragraph_geometry_uses_conservative_page_boundary() {
        let pages = vec![vec![
            make_paragraph_in_box("Left top", 900.0, 40.0, 260.0),
            make_paragraph_at("Left bottom", None, 700.0),
            make_paragraph_in_box("Right top", 880.0, 340.0, 560.0),
            make_paragraph_in_box("Right bottom", 680.0, 340.0, 560.0),
        ]];
        let tables = vec![make_table_in_box("| Right column |", 350.0, 550.0, 800.0)];

        let document = assemble_internal_document(pages, &tables, None, &[], &Default::default());

        assert_eq!(
            page_element_labels(&document),
            ["Left top", "<table>", "Left bottom", "Right top", "Right bottom"]
        );
    }

    #[test]
    fn test_tables_beyond_page_count_appended() {
        let pages = vec![vec![make_paragraph("Page 1", None)]];
        let tables = vec![crate::types::Table {
            cells: vec![],
            markdown: "| Extra |".to_string(),
            page_number: 5,
            bounding_box: None,
            ..Default::default()
        }];
        let doc = assemble_internal_document(pages, &tables, None, &[], &Default::default());
        assert!(doc.elements.iter().any(|e| e.text == "Page 1"));
        assert!(doc.tables.iter().any(|t| t.markdown.contains("| Extra |")));
    }

    #[test]
    fn test_empty_table_markdown_not_rendered() {
        let pages = vec![vec![make_paragraph("Text", None)]];
        let tables = vec![crate::types::Table {
            cells: vec![],
            markdown: "   ".to_string(),
            page_number: 1,
            bounding_box: None,
            ..Default::default()
        }];
        let doc = assemble_internal_document(pages, &tables, None, &[], &Default::default());
        assert!(doc.tables.is_empty() || doc.tables.iter().all(|t| t.markdown.trim().is_empty()));
    }

    #[test]
    fn test_no_page_break_when_leading_page_empty() {
        let pages = vec![vec![], vec![make_paragraph("Content on page 2", None)]];
        let doc = assemble_internal_document(pages, &[], None, &[], &Default::default());
        assert!(
            !doc.elements.iter().any(|e| matches!(e.kind, ElementKind::PageBreak)),
            "Blank leading page should not produce a page break"
        );
        assert_eq!(
            doc.elements
                .iter()
                .filter(|e| matches!(e.kind, ElementKind::Paragraph))
                .count(),
            1
        );
    }

    #[test]
    fn test_no_page_break_when_trailing_page_empty() {
        let pages = vec![vec![make_paragraph("Content on page 1", None)], vec![]];
        let doc = assemble_internal_document(pages, &[], None, &[], &Default::default());
        assert!(
            !doc.elements.iter().any(|e| matches!(e.kind, ElementKind::PageBreak)),
            "Blank trailing page should not produce a page break"
        );
    }

    #[test]
    fn test_page_break_between_content_pages() {
        let pages = vec![
            vec![make_paragraph("Page 1", None)],
            vec![make_paragraph("Page 2", None)],
        ];
        let doc = assemble_internal_document(pages, &[], None, &[], &Default::default());
        assert!(
            doc.elements.iter().any(|e| matches!(e.kind, ElementKind::PageBreak)),
            "PageBreak should separate two content pages"
        );
    }

    #[test]
    fn test_no_page_break_single_page() {
        let pages = vec![vec![make_paragraph("Only page", None)]];
        let doc = assemble_internal_document(pages, &[], None, &[], &Default::default());
        assert!(
            !doc.elements.iter().any(|e| matches!(e.kind, ElementKind::PageBreak)),
            "Single page should not produce a page break"
        );
    }

    #[test]
    fn test_image_elements_injected_with_positions() {
        let pages = vec![vec![make_paragraph("Page with image", None)]];
        let image_positions = vec![(1u32, 0u32)];
        let doc = assemble_internal_document(pages, &[], None, &image_positions, &Default::default());

        let image_elems: Vec<_> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Image { .. }))
            .collect();
        assert_eq!(image_elems.len(), 1, "one image element should be injected");
        assert!(
            matches!(image_elems[0].kind, ElementKind::Image { image_index: 0 }),
            "image_index must match the position provided"
        );
    }

    #[test]
    fn test_image_ocr_text_appears_in_element() {
        use crate::types::ExtractedImage;
        use bytes::Bytes;
        use std::borrow::Cow;

        let pages = vec![vec![make_paragraph("Page with OCR image", None)]];
        let image_positions = vec![(1u32, 0u32)];
        let ocr_result = Box::new(crate::types::ExtractedDocument {
            content: "OCR extracted text".to_string(),
            mime_type: Cow::Borrowed("text/plain"),
            ..Default::default()
        });
        let images = vec![ExtractedImage {
            data: Bytes::new(),
            format: Cow::Borrowed("png"),
            image_index: 0,
            page_number: Some(1),
            width: None,
            height: None,
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description: None,
            ocr_result: Some(ocr_result),
            bounding_box: None,
            source_path: None,
            cluster_id: None,
            caption: None,
            qr_codes: None,
            image_kind: None,
            kind_confidence: None,
            data_base64: None,
        }];
        let doc = assemble_internal_document(pages, &[], Some(&images), &image_positions, &Default::default());
        let img_elem = doc
            .elements
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Image { .. }))
            .unwrap();
        assert_eq!(img_elem.text, "OCR extracted text");
    }

    #[test]
    fn test_no_image_elements_with_empty_positions() {
        let pages = vec![vec![make_paragraph("No images here", None)]];
        let doc = assemble_internal_document(pages, &[], None, &[], &Default::default());

        let image_count = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Image { .. }))
            .count();
        assert_eq!(image_count, 0, "no image elements when positions is empty");
    }

    /// Build a minimal `ExtractedImage` with (or without) a bounding box, for
    /// exercising `interleave_images_into_page` via `assemble_internal_document`.
    fn make_image_at(
        image_index: u32,
        page_number: u32,
        bounding_box: Option<crate::types::BoundingBox>,
    ) -> crate::types::ExtractedImage {
        crate::types::ExtractedImage {
            data: bytes::Bytes::new(),
            format: std::borrow::Cow::Borrowed("png"),
            image_index,
            page_number: Some(page_number),
            width: None,
            height: None,
            colorspace: None,
            bits_per_component: None,
            is_mask: false,
            description: None,
            ocr_result: None,
            bounding_box,
            source_path: None,
            cluster_id: None,
            caption: None,
            qr_codes: None,
            image_kind: None,
            kind_confidence: None,
            data_base64: None,
        }
    }

    /// Element sequence including images, in reading order.
    fn page_elements_with_images(document: &InternalDocument) -> Vec<&str> {
        document
            .elements
            .iter()
            .filter_map(|element| match &element.kind {
                ElementKind::Paragraph => Some(element.text.as_str()),
                ElementKind::Image { .. } => Some("<image>"),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn test_positioned_image_is_interleaved_between_paragraphs_by_bbox() {
        // "Top" paragraph sits high on the page (top-y 900), "Bottom" sits low (top-y 300).
        // The image's top-y (600) falls strictly between them, so it must land between
        // the two paragraph elements in the assembled sequence, not after both.
        let pages = vec![vec![
            make_paragraph_in_box("Top", 900.0, 40.0, 560.0),
            make_paragraph_in_box("Bottom", 300.0, 40.0, 560.0),
        ]];
        let image_bbox = crate::types::BoundingBox {
            x0: 40.0,
            y0: 550.0,
            x1: 560.0,
            y1: 600.0,
        };
        let images = vec![make_image_at(0, 1, Some(image_bbox))];
        let image_positions = vec![(1u32, 0u32)];

        let document = assemble_internal_document(pages, &[], Some(&images), &image_positions, &Default::default());

        let labels = page_elements_with_images(&document);
        assert_eq!(
            labels,
            vec!["Top", "<image>", "Bottom"],
            "image must be interleaved strictly between the two paragraphs, got {labels:?}"
        );

        let top_idx = document
            .elements
            .iter()
            .position(|e| e.text == "Top")
            .expect("Top paragraph must be present");
        let image_idx = document
            .elements
            .iter()
            .position(|e| matches!(e.kind, ElementKind::Image { .. }))
            .expect("image element must be present");
        let bottom_idx = document
            .elements
            .iter()
            .position(|e| e.text == "Bottom")
            .expect("Bottom paragraph must be present");
        assert!(
            top_idx < image_idx && image_idx < bottom_idx,
            "image index ({image_idx}) must be strictly between Top ({top_idx}) and Bottom ({bottom_idx})"
        );
    }

    #[test]
    fn test_image_without_bbox_falls_back_to_append_after_text() {
        // No bounding_box on the image (as when xberg_native_pdf's capped fast path is used, or on
        // the pure-heuristic path with no spatial data) must reproduce the pre-existing
        // append-after-text behavior exactly, and must never panic.
        let pages = vec![vec![
            make_paragraph_in_box("Top", 900.0, 40.0, 560.0),
            make_paragraph_in_box("Bottom", 300.0, 40.0, 560.0),
        ]];
        let images = vec![make_image_at(0, 1, None)];
        let image_positions = vec![(1u32, 0u32)];

        let document = assemble_internal_document(pages, &[], Some(&images), &image_positions, &Default::default());

        let labels = page_elements_with_images(&document);
        assert_eq!(
            labels,
            vec!["Top", "Bottom", "<image>"],
            "image without a bounding_box must append after all page text, got {labels:?}"
        );
    }

    #[test]
    fn test_positioned_image_prefers_paragraph_in_same_column() {
        let pages = vec![vec![
            make_paragraph_in_box("Left top", 900.0, 40.0, 280.0),
            make_paragraph_in_box("Left bottom", 300.0, 40.0, 280.0),
            make_paragraph_in_box("Right top", 880.0, 320.0, 560.0),
            make_paragraph_in_box("Right bottom", 280.0, 320.0, 560.0),
        ]];
        let image_bbox = crate::types::BoundingBox {
            x0: 320.0,
            y0: 550.0,
            x1: 560.0,
            y1: 600.0,
        };
        let images = vec![make_image_at(0, 1, Some(image_bbox))];

        let document = assemble_internal_document(pages, &[], Some(&images), &[(1, 0)], &Default::default());

        assert_eq!(
            page_elements_with_images(&document),
            vec!["Left top", "Left bottom", "Right top", "<image>", "Right bottom"]
        );
    }

    #[test]
    fn test_positioned_images_preserve_source_order_at_same_target() {
        let pages = vec![vec![
            make_paragraph_in_box("Top", 900.0, 40.0, 560.0),
            make_paragraph_in_box("Bottom", 300.0, 40.0, 560.0),
        ]];
        let image_bbox = crate::types::BoundingBox {
            x0: 40.0,
            y0: 550.0,
            x1: 560.0,
            y1: 600.0,
        };
        let images = vec![
            make_image_at(0, 1, Some(image_bbox)),
            make_image_at(1, 1, Some(image_bbox)),
        ];

        let document = assemble_internal_document(pages, &[], Some(&images), &[(1, 0), (1, 1)], &Default::default());
        let image_indices: Vec<u32> = document
            .elements
            .iter()
            .filter_map(|element| match element.kind {
                ElementKind::Image { image_index } => Some(image_index),
                _ => None,
            })
            .collect();

        assert_eq!(image_indices, vec![0, 1]);
    }

    #[test]
    fn test_image_below_left_column_stays_before_next_column() {
        let pages = vec![vec![
            make_paragraph_in_box("Left top", 900.0, 40.0, 280.0),
            make_paragraph_in_box("Left bottom", 500.0, 40.0, 280.0),
            make_paragraph_in_box("Right top", 880.0, 320.0, 560.0),
            make_paragraph_in_box("Right bottom", 480.0, 320.0, 560.0),
        ]];
        let image_bbox = crate::types::BoundingBox {
            x0: 40.0,
            y0: 320.0,
            x1: 280.0,
            y1: 380.0,
        };
        let images = vec![make_image_at(0, 1, Some(image_bbox))];

        let document = assemble_internal_document(pages, &[], Some(&images), &[(1, 0)], &Default::default());

        assert_eq!(
            page_elements_with_images(&document),
            vec!["Left top", "Left bottom", "<image>", "Right top", "Right bottom"]
        );
    }

    #[test]
    fn test_image_below_right_column_appends_after_that_column() {
        let pages = vec![vec![
            make_paragraph_in_box("Left top", 900.0, 40.0, 280.0),
            make_paragraph_in_box("Left bottom", 500.0, 40.0, 280.0),
            make_paragraph_in_box("Right top", 880.0, 320.0, 560.0),
            make_paragraph_in_box("Right bottom", 480.0, 320.0, 560.0),
        ]];
        let image_bbox = crate::types::BoundingBox {
            x0: 320.0,
            y0: 320.0,
            x1: 560.0,
            y1: 380.0,
        };
        let images = vec![make_image_at(0, 1, Some(image_bbox))];

        let document = assemble_internal_document(pages, &[], Some(&images), &[(1, 0)], &Default::default());

        assert_eq!(
            page_elements_with_images(&document),
            vec!["Left top", "Left bottom", "Right top", "Right bottom", "<image>"]
        );
    }

    #[test]
    fn test_image_between_layout_columns_stays_outside_next_group() {
        let column_path = |id| LayoutRegionPath {
            root: LayoutRegionTag {
                id,
                class_name: Some(LayoutHintClass::Text),
            },
            child: None,
        };
        let mut left_top = make_paragraph_in_box("Left top", 900.0, 40.0, 280.0);
        left_top.layout_region_path = Some(column_path(1));
        let mut left_bottom = make_paragraph_in_box("Left bottom", 500.0, 40.0, 280.0);
        left_bottom.layout_region_path = Some(column_path(1));
        let mut right_top = make_paragraph_in_box("Right top", 880.0, 320.0, 560.0);
        right_top.layout_region_path = Some(column_path(2));
        let mut right_bottom = make_paragraph_in_box("Right bottom", 480.0, 320.0, 560.0);
        right_bottom.layout_region_path = Some(column_path(2));
        let pages = vec![vec![left_top, left_bottom, right_top, right_bottom]];
        let left_image_bbox = crate::types::BoundingBox {
            x0: 40.0,
            y0: 320.0,
            x1: 280.0,
            y1: 380.0,
        };
        let right_image_bbox = crate::types::BoundingBox {
            x0: 320.0,
            y0: 320.0,
            x1: 560.0,
            y1: 380.0,
        };
        let images = vec![
            make_image_at(0, 1, Some(left_image_bbox)),
            make_image_at(1, 1, Some(right_image_bbox)),
        ];

        let document = assemble_internal_document(pages, &[], Some(&images), &[(1, 0), (1, 1)], &Default::default());
        let index_of_text = |text: &str| {
            document
                .elements
                .iter()
                .position(|element| element.text == text)
                .expect("paragraph must be present")
        };
        let image_element_index = |expected_image_index| {
            document
                .elements
                .iter()
                .position(|element| {
                    matches!(
                        element.kind,
                        ElementKind::Image { image_index } if image_index == expected_image_index
                    )
                })
                .expect("image must be present")
        };
        let left_image_index = image_element_index(0);
        let right_image_index = image_element_index(1);
        let group_ends = document
            .elements
            .iter()
            .enumerate()
            .filter_map(|(index, element)| (element.kind == ElementKind::GroupEnd).then_some(index))
            .collect::<Vec<_>>();
        let group_starts = document
            .elements
            .iter()
            .enumerate()
            .filter_map(|(index, element)| (element.kind == ElementKind::GroupStart).then_some(index))
            .collect::<Vec<_>>();

        assert_eq!(group_starts.len(), 2);
        assert_eq!(group_ends.len(), 2);
        assert!(
            index_of_text("Left bottom") < left_image_index
                && left_image_index < group_ends[0]
                && group_ends[0] < group_starts[1]
                && group_starts[1] < index_of_text("Right top"),
            "left-column image must remain inside the left group and before the right group: {:?}",
            document.elements.iter().map(|element| element.kind).collect::<Vec<_>>()
        );
        assert!(
            index_of_text("Right bottom") < right_image_index && right_image_index < group_ends[1],
            "right-column image must remain inside the final group: {:?}",
            document.elements.iter().map(|element| element.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_caption_skipped_in_main_flow() {
        let para1 = make_paragraph("Main text", None);
        let mut caption = make_paragraph("Caption text", None);
        caption.caption_for = Some(0);
        let pages = vec![vec![para1, caption]];
        let doc = assemble_internal_document(pages, &[], None, &[], &Default::default());
        assert!(doc.elements.iter().any(|e| e.text == "Main text"));
        assert!(doc.elements.iter().any(|e| e.text == "Caption text"));
    }

    fn bold_segment(text: &str) -> SegmentData {
        SegmentData {
            is_bold: true,
            ..plain_segment(text)
        }
    }

    fn italic_segment(text: &str) -> SegmentData {
        SegmentData {
            is_italic: true,
            ..plain_segment(text)
        }
    }

    #[test]
    fn test_bold_list_annotation_is_shifted_after_unicode_bullet() {
        let lines = vec![PdfLine {
            segments: vec![bold_segment("• Bold item")],
            baseline_y: 700.0,
            dominant_font_size: 12.0,
            is_bold: true,
            is_monospace: false,
        }];
        let word_count = PdfParagraph::compute_word_count("", &lines);
        let paragraph = PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: true,
            is_list_item: true,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        };

        let document = assemble_internal_document(vec![vec![paragraph]], &[], None, &[], &Default::default());
        let item = document
            .elements
            .iter()
            .find(|element| matches!(element.kind, ElementKind::ListItem { .. }))
            .expect("list item should be emitted");

        assert_eq!(item.text, "Bold item");
        assert!(!item.annotations.is_empty(), "bold annotation should be preserved");
        for annotation in &item.annotations {
            let start = annotation.start as usize;
            let end = annotation.end as usize;
            assert!(
                start < end && end <= item.text.len(),
                "invalid annotation: {annotation:?}"
            );
            assert!(item.text.is_char_boundary(start) && item.text.is_char_boundary(end));
        }
        let bold = item
            .annotations
            .iter()
            .find(|annotation| matches!(annotation.kind, AnnotationKind::Bold))
            .expect("bold annotation should be present");
        assert_eq!(&item.text[bold.start as usize..bold.end as usize], "Bold item");
    }

    #[test]
    fn ordered_marker_families_render_without_source_prefixes() {
        for (source, expected) in [
            ("a. alpha item", "alpha item"),
            ("I. Roman item", "Roman item"),
            ("(1) parenthesized item", "parenthesized item"),
            ("[1] bracketed item", "bracketed item"),
        ] {
            let mut paragraph = make_paragraph(source, None);
            paragraph.is_list_item = true;
            let document = assemble_internal_document(vec![vec![paragraph]], &[], None, &[], &Default::default());
            let item = document
                .elements
                .iter()
                .find(|element| matches!(element.kind, ElementKind::ListItem { .. }))
                .expect("list item should be emitted");

            assert_eq!(item.kind, ElementKind::ListItem { ordered: true }, "source: {source}");
            assert_eq!(item.text, expected, "source: {source}");
        }
    }

    #[test]
    fn split_ordered_marker_and_body_render_as_one_clean_item() {
        let lines = vec![PdfLine {
            segments: vec![plain_segment("I."), plain_segment("Split body")],
            baseline_y: 700.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        }];
        let mut paragraph = make_paragraph("", None);
        paragraph.lines = lines;
        paragraph.is_list_item = true;

        let document = assemble_internal_document(vec![vec![paragraph]], &[], None, &[], &Default::default());
        let item = document
            .elements
            .iter()
            .find(|element| matches!(element.kind, ElementKind::ListItem { .. }))
            .expect("list item should be emitted");

        assert_eq!(item.kind, ElementKind::ListItem { ordered: true });
        assert_eq!(item.text, "Split body");
    }

    #[test]
    fn test_w2a_inline_bold_and_italic_annotations_preserved() {
        let segments = vec![
            plain_segment("Normal "),
            bold_segment("bold"),
            plain_segment(" normal "),
            italic_segment("italic"),
            plain_segment(" normal"),
        ];

        let line = PdfLine {
            segments,
            baseline_y: 700.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        };

        let lines = vec![line];
        let word_count = PdfParagraph::compute_word_count("", &lines);
        let para = PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        };

        let doc = assemble_internal_document(vec![vec![para]], &[], None, &[], &Default::default());
        assert_eq!(doc.elements.len(), 1);

        let elem = &doc.elements[0];
        assert!(!elem.text.is_empty(), "Paragraph text should be populated");

        let has_bold = elem.annotations.iter().any(|a| matches!(a.kind, AnnotationKind::Bold));
        let has_italic = elem
            .annotations
            .iter()
            .any(|a| matches!(a.kind, AnnotationKind::Italic));

        assert!(
            has_bold,
            "Should have bold annotation; text: {}, annotations: {:?}",
            elem.text, elem.annotations
        );
        assert!(
            has_italic,
            "Should have italic annotation; text: {}, annotations: {:?}",
            elem.text, elem.annotations
        );
    }

    /// The producer half of the literal-list-label fix. `normalize_list_text`
    /// strips `"B."` off the item text so the text reads as content; without
    /// the `set_list_item_source_label` call in `push_paragraph_element` the
    /// label is gone for good and a renderer can only synthesize a position,
    /// silently renumbering clauses that the document cross-references by their
    /// printed label. Neutralise that call and this fails with `None`.
    #[test]
    fn ordered_list_item_keeps_its_literal_source_marker() {
        let mut para = make_paragraph("B. The Property shall be developed in substantial conformance.", None);
        para.is_list_item = true;

        let doc = assemble_internal_document(vec![vec![para]], &[], None, &[], &Default::default());

        let item = doc
            .elements
            .iter()
            .find(|element| matches!(element.kind, ElementKind::ListItem { .. }))
            .expect("the paragraph should assemble into a ListItem element");

        assert_eq!(
            item.list_item_source_label(),
            Some("B."),
            "the stripped marker must be recorded, not discarded; text is {:?}",
            item.text
        );
        assert!(
            !item.text.starts_with("B."),
            "the marker must still be stripped from the item text; got {:?}",
            item.text
        );
    }

    #[test]
    fn test_w2a_consecutive_bold_segments_grouped() {
        let segments = vec![bold_segment("This"), bold_segment(" is"), bold_segment(" bold")];

        let line = PdfLine {
            segments,
            baseline_y: 700.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        };

        let lines = vec![line];
        let word_count = PdfParagraph::compute_word_count("", &lines);
        let para = PdfParagraph {
            text: String::new(),
            lines,
            dominant_font_size: 12.0,
            heading_level: None,
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        };

        let doc = assemble_internal_document(vec![vec![para]], &[], None, &[], &Default::default());
        let elem = &doc.elements[0];

        let bold_anns: Vec<_> = elem
            .annotations
            .iter()
            .filter(|a| matches!(a.kind, AnnotationKind::Bold))
            .collect();

        assert!(
            !bold_anns.is_empty(),
            "Should have at least one bold annotation; text: {}, annotations: {:?}",
            elem.text,
            elem.annotations
        );

        if !bold_anns.is_empty() {
            let bold = bold_anns[0];
            let coverage = (bold.end - bold.start) as usize;
            let text_len = elem.text.len();
            assert!(
                coverage >= text_len / 2,
                "Bold annotation should cover at least half the text; coverage: {}, text_len: {}",
                coverage,
                text_len
            );
        }
    }

    /// Build a heading paragraph that mirrors the production pipeline: both `text`
    /// and `lines` are populated.  `push_paragraph_element` prefers `para.text` when
    /// non-empty, so this exercises the code path that previously silently discarded
    /// merged content.
    fn make_production_h1(text: &str) -> PdfParagraph {
        let lines = text
            .split_whitespace()
            .enumerate()
            .map(|(i, word)| PdfLine {
                segments: vec![SegmentData {
                    text: word.to_string(),
                    x: i as f32 * 50.0,
                    y: 700.0,
                    width: 40.0,
                    height: 24.0,
                    font_size: 24.0,
                    is_bold: false,
                    is_italic: false,
                    is_monospace: false,
                    baseline_y: 700.0,
                    rotation_degrees: 0.0,
                    assigned_role: None,
                }],
                baseline_y: 700.0,
                dominant_font_size: 24.0,
                is_bold: false,
                is_monospace: false,
            })
            .collect::<Vec<_>>();
        let word_count = PdfParagraph::compute_word_count(text, &lines);
        PdfParagraph {
            text: text.to_string(),
            lines,
            dominant_font_size: 24.0,
            heading_level: Some(1),
            is_bold: false,
            is_list_item: false,
            is_code_block: false,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        }
    }

    #[test]
    fn test_merged_h1_text_appears_in_assembled_document() {
        let merged_para = make_production_h1("KAISUN HOLDINGS LIMITED");
        let pages = vec![vec![merged_para]];
        let doc = assemble_internal_document(pages, &[], None, &[], &Default::default());

        assert_eq!(doc.elements.len(), 1);
        let heading = &doc.elements[0];
        assert!(
            matches!(heading.kind, ElementKind::Heading { level: 1 }),
            "expected Heading(1), got {:?}",
            heading.kind
        );
        assert!(
            heading.text.contains("KAISUN HOLDINGS"),
            "heading must contain first fragment; got: {:?}",
            heading.text
        );
        assert!(
            heading.text.contains("LIMITED"),
            "heading must contain second fragment; got: {:?}",
            heading.text
        );
    }

    #[test]
    fn test_separate_h1s_each_appear_in_assembled_document() {
        let pages = vec![vec![
            make_production_h1("HR 22"),
            make_production_h1("HR 28"),
            make_production_h1("HR 28/24"),
            make_production_h1("HR 36/30"),
        ]];
        let doc = assemble_internal_document(pages, &[], None, &[], &Default::default());

        let headings: Vec<_> = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Heading { level: 1 }))
            .collect();
        assert_eq!(headings.len(), 4, "all four model-code headings must be present");
        let texts: Vec<&str> = headings.iter().map(|h| h.text.as_str()).collect();
        assert!(texts.contains(&"HR 22"), "HR 22 missing; headings: {texts:?}");
        assert!(texts.contains(&"HR 28"), "HR 28 missing; headings: {texts:?}");
        assert!(texts.contains(&"HR 28/24"), "HR 28/24 missing; headings: {texts:?}");
        assert!(texts.contains(&"HR 36/30"), "HR 36/30 missing; headings: {texts:?}");
    }

    #[test]
    fn collapse_inner_spaces_borrows_when_no_double_space() {
        let line = "  no double spaces here";
        let result = collapse_inner_spaces(line);
        assert!(matches!(result, Cow::Borrowed(_)), "should not allocate when unchanged");
        assert_eq!(result, line);
    }

    #[test]
    fn collapse_inner_spaces_allocates_and_collapses_when_double_space_present() {
        let line = "  has   extra    spaces";
        let result = collapse_inner_spaces(line);
        assert!(
            matches!(result, Cow::Owned(_)),
            "should allocate when spaces are collapsed"
        );
        assert_eq!(result, "  has extra spaces");
    }

    fn paragraph_text(document: &InternalDocument) -> &str {
        document
            .elements
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Paragraph))
            .map(|e| e.text.as_str())
            .expect("a paragraph element should be emitted")
    }

    // xberg-io/xberg#1581: a suspended Dutch hyphen ("CV- en") welded into "CVen" because
    // `classify_line_break_hyphen` fired at the run boundary purely from the text pattern, without
    // checking whether the two runs actually sit on different visual lines.
    #[test]
    fn suspended_hyphen_across_style_run_boundary_is_not_welded() {
        let segments = vec![plain_segment("CV- "), bold_segment("en boiler")];
        let line = PdfLine {
            segments,
            baseline_y: 700.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        };
        let mut para = make_paragraph("", None);
        para.lines = vec![line];

        let document = assemble_internal_document(vec![vec![para]], &[], None, &[], &Default::default());
        assert_eq!(paragraph_text(&document), "CV- en boiler");
    }

    // xberg-io/xberg#1581: the same weld, but for two segments split mid-line within one
    // style run (`onderhouds- en`), the site the issue traces to `join_line_texts_plain`'s
    // preceding-word lookup and the same-run branch of `extract_text_and_annotations`.
    #[test]
    fn suspended_hyphen_within_one_style_run_is_not_welded() {
        let segments = vec![
            plain_segment("onderhouds- "),
            plain_segment("en installatiewerkzaamheden"),
        ];
        let line = PdfLine {
            segments,
            baseline_y: 700.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        };
        let mut para = make_paragraph("", None);
        para.lines = vec![line];

        let document = assemble_internal_document(vec![vec![para]], &[], None, &[], &Default::default());
        assert_eq!(paragraph_text(&document), "onderhouds- en installatiewerkzaamheden");
    }

    // xberg-io/xberg#1613: the line-break hyphen decision was purely typographic, with no access
    // to the lexical evidence `should_preserve_lexical_hyphen` exists to weigh -- so a compound
    // whose own hyphen lands on a line break was welded into a token that does not exist.
    // `long-term` and `cost-effective` are entries in `PRESERVED_LEXICAL_COMPOUNDS`, so these
    // need no witness collection and no corpus: the extractor already lists them as
    // must-preserve, and both lost the hyphen on this path.
    #[test]
    fn a_static_compound_hyphen_at_a_line_break_survives() {
        for (trailing, leading, expected) in [
            ("a long-", "term contract", "a long-term contract"),
            ("a cost-", "effective option", "a cost-effective option"),
        ] {
            let first = PdfLine {
                segments: vec![SegmentData {
                    baseline_y: 700.0,
                    ..plain_segment(trailing)
                }],
                baseline_y: 700.0,
                dominant_font_size: 12.0,
                is_bold: false,
                is_monospace: false,
            };
            let second = PdfLine {
                segments: vec![SegmentData {
                    baseline_y: 685.0,
                    ..plain_segment(leading)
                }],
                baseline_y: 685.0,
                dominant_font_size: 12.0,
                is_bold: false,
                is_monospace: false,
            };
            let mut para = make_paragraph("", None);
            para.lines = vec![first, second];

            let document = assemble_internal_document(vec![vec![para]], &[], None, &[], &Default::default());
            assert_eq!(paragraph_text(&document), expected);
        }
    }

    // The other half of the rule: a word the wrap genuinely broke must still be rejoined.
    // Without this, "preserve the hyphen" degenerates into "never dehyphenate".
    #[test]
    fn a_genuine_wrap_hyphen_at_a_line_break_is_still_joined() {
        let first = PdfLine {
            segments: vec![SegmentData {
                baseline_y: 700.0,
                ..plain_segment("adjuvant Mon-")
            }],
            baseline_y: 700.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        };
        let second = PdfLine {
            segments: vec![SegmentData {
                baseline_y: 685.0,
                ..plain_segment("tanide was used")
            }],
            baseline_y: 685.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        };
        let mut para = make_paragraph("", None);
        para.lines = vec![first, second];

        let document = assemble_internal_document(vec![vec![para]], &[], None, &[], &Default::default());
        assert_eq!(paragraph_text(&document), "adjuvant Montanide was used");
    }

    // Same defect via `join_line_texts_plain` (list items get their text from that path,
    // not `extract_text_and_annotations`).
    #[test]
    fn suspended_hyphen_is_not_welded_in_list_item_plain_join() {
        let segments = vec![plain_segment("montage- "), plain_segment("en installatiehandleiding")];
        let line = PdfLine {
            segments,
            baseline_y: 700.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        };
        let mut para = make_paragraph("", None);
        para.lines = vec![line];
        para.is_list_item = true;

        let document = assemble_internal_document(vec![vec![para]], &[], None, &[], &Default::default());
        let item = document
            .elements
            .iter()
            .find(|e| matches!(e.kind, ElementKind::ListItem { .. }))
            .expect("list item should be emitted");
        assert_eq!(item.text, "montage- en installatiehandleiding");
    }

    // The control from the issue's own reproducer: a genuine line-wrap hyphen, where the
    // trailing and leading runs sit on different visual lines (baselines differ by more than
    // the inline-style tolerance), must still be rejoined. A fix that stops all dehyphenation
    // would pass the three tests above for the wrong reason.
    #[test]
    fn genuine_line_wrap_hyphen_still_joins_across_visual_line_break() {
        let line1 = PdfLine {
            segments: vec![SegmentData {
                baseline_y: 700.0,
                ..plain_segment("Zie de installatie-")
            }],
            baseline_y: 700.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        };
        let line2 = PdfLine {
            segments: vec![SegmentData {
                baseline_y: 686.0,
                ..plain_segment("handleiding voor details")
            }],
            baseline_y: 686.0,
            dominant_font_size: 12.0,
            is_bold: false,
            is_monospace: false,
        };
        let mut para = make_paragraph("", None);
        para.lines = vec![line1, line2];

        let document = assemble_internal_document(vec![vec![para]], &[], None, &[], &Default::default());
        assert_eq!(paragraph_text(&document), "Zie de installatiehandleiding voor details");
    }
}
