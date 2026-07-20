//! Final structured document assembly from classified paragraphs, with optional table interleaving.
//!
//! Produces an `InternalDocument` from per-page `PdfParagraph` data with tables
//! interleaved at their correct reading-order positions.

use super::lines::needs_space_between;
use super::text_repair::finalize_hyphens;
use super::types::{LayoutHintClass, PdfParagraph};
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

        if let Some(page_tables) = page_tables {
            assemble_page_elements_with_tables(&mut builder, paragraphs, &page_tables, page_num);
        } else {
            assemble_page_elements(&mut builder, paragraphs, page_num);
        }

        if page_has_content {
            has_emitted_content = true;
        }

        if let Some(image_indices) = images_by_page.get(&((page_idx + 1) as u32)) {
            for &image_index in image_indices {
                let ocr_text = images
                    .and_then(|imgs| imgs.get(image_index as usize))
                    .and_then(|img| img.ocr_result.as_ref())
                    .map(|res| res.content.as_str())
                    .unwrap_or("");

                let elem =
                    crate::types::internal::InternalElement::text(ElementKind::Image { image_index }, ocr_text, 0)
                        .with_page((page_idx + 1) as u32);
                builder.push_element(elem);
            }
        }
    }

    for (&page_idx, page_tables) in &tables_by_page {
        let page_num = Some(page_idx + 1);
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

/// Push paragraph elements for a page without tables.
fn assemble_page_elements(builder: &mut InternalDocumentBuilder, paragraphs: &[PdfParagraph], page: Option<u32>) {
    let mut in_list = false;

    for (para_idx, para) in paragraphs.iter().enumerate() {
        if para.caption_for.is_some() {
            continue;
        }

        if para.is_list_item && !in_list {
            builder.push_list(list_item_is_ordered(para));
            in_list = true;
        } else if !para.is_list_item && in_list {
            builder.end_list();
            in_list = false;
        }

        let elem_idx = push_paragraph_element(builder, para, page);

        emit_caption_elements(builder, paragraphs, para_idx, page, elem_idx);
    }

    if in_list {
        builder.end_list();
    }
}

/// Push paragraph elements in their established reading order, with tables interleaved.
fn assemble_page_elements_with_tables(
    builder: &mut InternalDocumentBuilder,
    paragraphs: &[PdfParagraph],
    tables: &[&crate::types::Table],
    page: Option<u32>,
) {
    let mut positioned: Vec<(f32, &crate::types::Table)> = Vec::new();
    let mut unpositioned: Vec<&crate::types::Table> = Vec::new();

    for table in tables {
        let md = table.markdown.trim();
        if md.is_empty() {
            continue;
        }
        if let Some(ref bbox) = table.bounding_box {
            positioned.push((bbox.y1 as f32, *table));
        } else {
            unpositioned.push(*table);
        }
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
        let slot = table_insertion_slot(&ordered_paragraphs, table, table_y);
        tables_at_slot[slot].push(table);
    }

    let mut in_list = false;

    for (slot, slot_tables) in tables_at_slot.into_iter().enumerate() {
        for table in slot_tables {
            if in_list {
                builder.end_list();
                in_list = false;
            }
            push_table_element(builder, table, page);
        }

        let Some(&(para_idx, para)) = ordered_paragraphs.get(slot) else {
            continue;
        };

        if para.is_list_item && !in_list {
            builder.push_list(list_item_is_ordered(para));
            in_list = true;
        } else if !para.is_list_item && in_list {
            builder.end_list();
            in_list = false;
        }

        let elem_idx = push_paragraph_element(builder, para, page);
        emit_caption_elements(builder, paragraphs, para_idx, page, elem_idx);
    }

    if in_list {
        builder.end_list();
    }

    for table in unpositioned {
        push_table_element(builder, table, page);
    }
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
fn push_paragraph_element(builder: &mut InternalDocumentBuilder, para: &PdfParagraph, page: Option<u32>) -> u32 {
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
            join_line_texts_plain(&para.lines)
        };
        finalize_hyphens(&text).into_owned()
    };

    if let Some(level) = para.heading_level {
        let text = get_text(para);
        return builder.push_heading(level, &text, page, bbox);
    }

    if para.is_code_block {
        let text = if !para.text.is_empty() {
            para.text.clone()
        } else {
            para.lines
                .iter()
                .map(|l| {
                    let line_text = l.segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ");
                    collapse_inner_spaces(&line_text)
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
            let (annotated_text, annotations) = extract_text_and_annotations(para);
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
        return builder.push_list_item(normalized, ordered, annotations, page, bbox);
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
        let (text, annotations) = extract_text_and_annotations(para);
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
fn extract_text_and_annotations(para: &PdfParagraph) -> (String, Vec<TextAnnotation>) {
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

        let mut run_words: Vec<&str> = Vec::new();
        for seg in &all_segments[run_start..i] {
            for word in seg.text.split_whitespace() {
                run_words.push(word);
            }
        }

        if !text.is_empty() && !run_words.is_empty() {
            let prev_last = all_segments[run_start - 1]
                .text
                .split_whitespace()
                .next_back()
                .unwrap_or("");
            let next_first = all_segments[run_start].text.split_whitespace().next().unwrap_or("");

            if should_dehyphenate(prev_last, next_first) {
                text.pop();
            } else if needs_space_between(prev_last, next_first) {
                text.push(' ');
            }
        }

        let span_start = text.len();

        for (wi, &word) in run_words.iter().enumerate() {
            if wi > 0 {
                let prev = run_words[wi - 1];
                if should_dehyphenate(prev, word) {
                    text.pop();
                } else if needs_space_between(prev, word) {
                    text.push(' ');
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
fn join_line_texts_plain(lines: &[super::types::PdfLine]) -> String {
    if lines.is_empty() {
        return String::new();
    }

    let words_per_line: Vec<Vec<&str>> = lines
        .iter()
        .map(|l| l.segments.iter().flat_map(|s| s.text.split_whitespace()).collect())
        .collect();

    let mut result = String::new();
    for (line_idx, line_words) in words_per_line.iter().enumerate() {
        for (word_idx, &word) in line_words.iter().enumerate() {
            if result.is_empty() {
                result.push_str(word);
                continue;
            }

            let prev_word = if word_idx > 0 {
                line_words[word_idx - 1]
            } else {
                words_per_line[..line_idx]
                    .iter()
                    .rev()
                    .find_map(|lw| lw.last().copied())
                    .unwrap_or("")
            };

            if should_dehyphenate(prev_word, word) {
                result.pop();
                result.push_str(word);
            } else if needs_space_between(prev_word, word) {
                result.push(' ');
                result.push_str(word);
            } else {
                result.push_str(word);
            }
        }
    }
    result
}

/// Check if a line-ending hyphen should be removed and words joined.
fn should_dehyphenate(prev: &str, next: &str) -> bool {
    if prev.len() < 2 || !prev.ends_with('-') {
        return false;
    }
    let before_hyphen = prev[..prev.len() - 1].chars().next_back();
    if !before_hyphen.is_some_and(|c| c.is_alphabetic()) {
        return false;
    }
    next.chars().next().is_some_and(|c| c.is_lowercase())
}

/// Collapse runs of 2+ spaces inside a line while preserving leading indentation.
fn collapse_inner_spaces(line: &str) -> String {
    let leading = line.len() - line.trim_start_matches(' ').len();
    let prefix = &line[..leading];
    let rest = &line[leading..];
    if !rest.contains("  ") {
        return line.to_string();
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
    result
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
    let t = text.trim_start();
    let digit_end = t.bytes().position(|b| !b.is_ascii_digit()).unwrap_or(0);
    digit_end > 0 && matches!(t.as_bytes().get(digit_end), Some(b'.') | Some(b')'))
}

/// Strip a bullet/number prefix and return the clean suffix plus its byte offset.
fn normalize_list_text(text: &str) -> (&str, usize) {
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

    #[test]
    fn test_assemble_internal_document_basic() {
        let pages = vec![vec![
            make_paragraph("Title", Some(1)),
            make_paragraph("Body text", None),
        ]];
        let doc = assemble_internal_document(pages, &[], None, &[]);
        assert_eq!(doc.elements.len(), 2);
        assert!(matches!(doc.elements[0].kind, ElementKind::Heading { level: 1 }));
        assert_eq!(doc.elements[0].text, "Title");
        assert!(matches!(doc.elements[1].kind, ElementKind::Paragraph));
        assert_eq!(doc.elements[1].text, "Body text");
    }

    #[test]
    fn test_assemble_internal_document_empty() {
        let doc = assemble_internal_document(vec![], &[], None, &[]);
        assert!(doc.elements.is_empty());
    }

    #[test]
    fn test_assemble_internal_document_multiple_pages() {
        let pages = vec![
            vec![make_paragraph("Page 1", None)],
            vec![make_paragraph("Page 2", None)],
        ];
        let doc = assemble_internal_document(pages, &[], None, &[]);
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
        }];
        let doc = assemble_internal_document(pages, &tables, None, &[]);
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
        }];
        let doc = assemble_internal_document(pages, &tables, None, &[]);
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

        let document = assemble_internal_document(pages, &tables, None, &[]);

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

        let document = assemble_internal_document(pages, &tables, None, &[]);

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

        let document = assemble_internal_document(pages, &tables, None, &[]);

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

        let document = assemble_internal_document(pages, &tables, None, &[]);

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
        }];
        let doc = assemble_internal_document(pages, &tables, None, &[]);
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
        }];
        let doc = assemble_internal_document(pages, &tables, None, &[]);
        assert!(doc.tables.is_empty() || doc.tables.iter().all(|t| t.markdown.trim().is_empty()));
    }

    #[test]
    fn test_no_page_break_when_leading_page_empty() {
        let pages = vec![vec![], vec![make_paragraph("Content on page 2", None)]];
        let doc = assemble_internal_document(pages, &[], None, &[]);
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
        let doc = assemble_internal_document(pages, &[], None, &[]);
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
        let doc = assemble_internal_document(pages, &[], None, &[]);
        assert!(
            doc.elements.iter().any(|e| matches!(e.kind, ElementKind::PageBreak)),
            "PageBreak should separate two content pages"
        );
    }

    #[test]
    fn test_no_page_break_single_page() {
        let pages = vec![vec![make_paragraph("Only page", None)]];
        let doc = assemble_internal_document(pages, &[], None, &[]);
        assert!(
            !doc.elements.iter().any(|e| matches!(e.kind, ElementKind::PageBreak)),
            "Single page should not produce a page break"
        );
    }

    #[test]
    fn test_image_elements_injected_with_positions() {
        let pages = vec![vec![make_paragraph("Page with image", None)]];
        let image_positions = vec![(1u32, 0u32)];
        let doc = assemble_internal_document(pages, &[], None, &image_positions);

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
        let doc = assemble_internal_document(pages, &[], Some(&images), &image_positions);
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
        let doc = assemble_internal_document(pages, &[], None, &[]);

        let image_count = doc
            .elements
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Image { .. }))
            .count();
        assert_eq!(image_count, 0, "no image elements when positions is empty");
    }

    #[test]
    fn test_caption_skipped_in_main_flow() {
        let para1 = make_paragraph("Main text", None);
        let mut caption = make_paragraph("Caption text", None);
        caption.caption_for = Some(0);
        let pages = vec![vec![para1, caption]];
        let doc = assemble_internal_document(pages, &[], None, &[]);
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
            caption_for: None,
            block_bbox: None,
            word_count,
        };

        let document = assemble_internal_document(vec![vec![paragraph]], &[], None, &[]);
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
            caption_for: None,
            block_bbox: None,
            word_count,
        };

        let doc = assemble_internal_document(vec![vec![para]], &[], None, &[]);
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
            caption_for: None,
            block_bbox: None,
            word_count,
        };

        let doc = assemble_internal_document(vec![vec![para]], &[], None, &[]);
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
            caption_for: None,
            block_bbox: None,
            word_count,
        }
    }

    #[test]
    fn test_merged_h1_text_appears_in_assembled_document() {
        let merged_para = make_production_h1("KAISUN HOLDINGS LIMITED");
        let pages = vec![vec![merged_para]];
        let doc = assemble_internal_document(pages, &[], None, &[]);

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
        let doc = assemble_internal_document(pages, &[], None, &[]);

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
}
