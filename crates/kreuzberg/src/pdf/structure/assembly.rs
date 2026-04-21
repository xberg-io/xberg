//! Final structured document assembly from classified paragraphs, with optional table interleaving.
//!
//! Produces an `InternalDocument` from per-page `PdfParagraph` data with tables
//! interleaved at their correct reading-order positions.

use super::lines::needs_space_between;
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
    image_positions: &[(usize, usize)], // (page_idx, image_index) for image placeholders
) -> InternalDocument {
    tracing::debug!(
        page_count = pages.len(),
        table_count = tables.len(),
        image_count = image_positions.len(),
        total_paragraphs = pages.iter().map(|p| p.len()).sum::<usize>(),
        "assemble_internal_document: start"
    );
    let mut builder = InternalDocumentBuilder::new("pdf");

    // Group tables by page number (1-indexed → 0-indexed)
    let mut tables_by_page: std::collections::BTreeMap<usize, Vec<&crate::types::Table>> =
        std::collections::BTreeMap::new();
    for table in tables {
        let page_idx = if table.page_number > 0 {
            table.page_number - 1
        } else {
            0
        };
        tables_by_page.entry(page_idx).or_default().push(table);
    }

    // Group image positions by page
    let mut images_by_page: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for &(page_idx, image_index) in image_positions {
        images_by_page.entry(page_idx).or_default().push(image_index);
    }

    let mut has_emitted_content = false;
    for (page_idx, paragraphs) in pages.iter().enumerate() {
        let page_num = Some((page_idx + 1) as u32);
        let page_tables = tables_by_page.remove(&page_idx);

        // Check whether this page has any content (paragraphs, tables, or images).
        let page_has_content = !paragraphs.is_empty()
            || page_tables
                .as_ref()
                .is_some_and(|t| t.iter().any(|tb| !tb.markdown.trim().is_empty()))
            || images_by_page.contains_key(&(page_idx + 1));

        // Insert page break only between pages that both have content, so that
        // blank leading/trailing pages do not produce spurious thematic breaks.
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

        // Inject image placeholders for this page
        if let Some(image_indices) = images_by_page.get(&(page_idx + 1)) {
            for &image_index in image_indices {
                let elem = crate::types::internal::InternalElement::text(
                    ElementKind::Image {
                        image_index: image_index as u32,
                    },
                    "",
                    0,
                )
                .with_page((page_idx + 1) as u32);
                builder.push_element(elem);
            }
        }
    }

    // Append tables for pages beyond what we have paragraphs for
    for (&page_idx, page_tables) in &tables_by_page {
        let page_num = Some((page_idx + 1) as u32);
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

    // Inject image placeholders for page 0 (unknown page)
    if let Some(image_indices) = images_by_page.get(&0) {
        for &image_index in image_indices {
            let elem = crate::types::internal::InternalElement::text(
                ElementKind::Image {
                    image_index: image_index as u32,
                },
                "",
                0,
            );
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
        // Skip captions — they are emitted after their parent element
        if para.caption_for.is_some() {
            continue;
        }

        // Manage list container markers
        if para.is_list_item && !in_list {
            builder.push_list(false);
            in_list = true;
        } else if !para.is_list_item && in_list {
            builder.end_list();
            in_list = false;
        }

        let elem_idx = push_paragraph_element(builder, para, page);

        // Emit captions as relationships
        emit_caption_elements(builder, paragraphs, para_idx, page, elem_idx);
    }

    if in_list {
        builder.end_list();
    }
}

/// Push paragraph elements interleaved with tables sorted by vertical position.
fn assemble_page_elements_with_tables(
    builder: &mut InternalDocumentBuilder,
    paragraphs: &[PdfParagraph],
    tables: &[&crate::types::Table],
    page: Option<u32>,
) {
    // Split tables into positioned (have bounding box) and unpositioned
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

    // Sort positioned tables by y-position descending (top of page first in PDF coords)
    positioned.sort_by(|a, b| b.0.total_cmp(&a.0));

    // Build interleaved elements list
    enum PageElement<'a> {
        Paragraph(usize, &'a PdfParagraph),
        Table(&'a crate::types::Table),
    }

    let mut elements: Vec<(f32, PageElement)> = Vec::new();

    for (idx, para) in paragraphs.iter().enumerate() {
        if para.caption_for.is_some() {
            continue;
        }
        let y_pos = para.lines.first().map(|l| l.baseline_y).unwrap_or(0.0);
        elements.push((y_pos, PageElement::Paragraph(idx, para)));
    }

    for (y_pos, table) in &positioned {
        elements.push((*y_pos, PageElement::Table(table)));
    }

    // Sort by y descending (top of page first in PDF coordinates)
    elements.sort_by(|a, b| b.0.total_cmp(&a.0));

    let mut in_list = false;

    for (_, elem) in &elements {
        match elem {
            PageElement::Paragraph(para_idx, para) => {
                // Manage list container markers
                if para.is_list_item && !in_list {
                    builder.push_list(false);
                    in_list = true;
                } else if !para.is_list_item && in_list {
                    builder.end_list();
                    in_list = false;
                }

                let elem_idx = push_paragraph_element(builder, para, page);
                emit_caption_elements(builder, paragraphs, *para_idx, page, elem_idx);
            }
            PageElement::Table(table) => {
                if in_list {
                    builder.end_list();
                    in_list = false;
                }
                let bbox = table.bounding_box.map(|bb| BoundingBox {
                    x0: bb.x0,
                    y0: bb.y0,
                    x1: bb.x1,
                    y1: bb.y1,
                });
                builder.push_table((*table).clone(), page, bbox);
            }
        }
    }

    if in_list {
        builder.end_list();
    }

    // Append unpositioned tables at end of page
    for table in &unpositioned {
        let bbox = table.bounding_box.map(|bb| BoundingBox {
            x0: bb.x0,
            y0: bb.y0,
            x1: bb.x1,
            y1: bb.y1,
        });
        builder.push_table((*table).clone(), page, bbox);
    }
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

    // Log element classification for debugging.
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

    // Get text: prefer para.text (full-text path) over segment joining.
    let get_text = |para: &PdfParagraph| -> String {
        if !para.text.is_empty() {
            para.text.clone()
        } else {
            join_line_texts_plain(&para.lines)
        }
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
        let text = get_text(para);
        let normalized = normalize_list_text(&text);
        // For full-text path, block-level bold annotation; for structure tree, extract inline annotations
        let annotations = if !para.text.is_empty() && para.is_bold {
            vec![TextAnnotation {
                start: 0,
                end: normalized.len() as u32,
                kind: AnnotationKind::Bold,
            }]
        } else if para.text.is_empty() {
            let (_, anns) = extract_text_and_annotations(para);
            anns
        } else {
            vec![]
        };
        return builder.push_list_item(&normalized, false, annotations, page, bbox);
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

    // Default: body paragraph
    if !para.text.is_empty() {
        // Full-text path: text is already correct, add block-level bold if applicable
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
        // Structure tree path: extract inline annotations from segments
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

        // Find run of segments with the same formatting
        let run_start = i;
        while i < all_segments.len() && all_segments[i].is_bold == bold && all_segments[i].is_italic == italic {
            i += 1;
        }

        // Collect words for this run
        let mut run_words: Vec<&str> = Vec::new();
        for seg in &all_segments[run_start..i] {
            for word in seg.text.split_whitespace() {
                run_words.push(word);
            }
        }

        // Add space between previous text and this run
        if !text.is_empty() && !run_words.is_empty() {
            let prev_last = all_segments[run_start - 1]
                .text
                .split_whitespace()
                .next_back()
                .unwrap_or("");
            let next_first = all_segments[run_start].text.split_whitespace().next().unwrap_or("");

            if should_dehyphenate(prev_last, next_first) {
                // Remove trailing hyphen
                text.pop();
            } else if needs_space_between(prev_last, next_first) {
                text.push(' ');
            }
        }

        let span_start = text.len();

        // Build run text with CJK-aware joining
        for (wi, &word) in run_words.iter().enumerate() {
            if wi > 0 {
                let prev = run_words[wi - 1];
                if should_dehyphenate(prev, word) {
                    text.pop(); // remove '-'
                } else if needs_space_between(prev, word) {
                    text.push(' ');
                }
            }
            text.push_str(word);
        }

        let span_end = text.len();

        // Add annotations for this run
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

/// Normalize list item text: strip bullet/number prefixes and return clean text.
fn normalize_list_text(text: &str) -> String {
    let trimmed = text.trim_start();
    const BULLET_CHARS: &[char] = &[
        '\u{2022}', // • BULLET
        '\u{00B7}', // · MIDDLE DOT
    ];
    for &ch in BULLET_CHARS {
        if trimmed.starts_with(ch) {
            return trimmed[ch.len_utf8()..].trim_start().to_string();
        }
    }
    if let Some(stripped) = trimmed.strip_prefix("* ") {
        return stripped.trim_start().to_string();
    }
    if let Some(stripped) = trimmed.strip_prefix("- ") {
        return stripped.to_string();
    }
    const DASH_BULLETS: &[char] = &['–', '—', '−', '‐', '‑', '‒', '―', '➤', '►', '▶', '○', '●', '◦'];
    for &ch in DASH_BULLETS {
        if trimmed.starts_with(ch) {
            return trimmed[ch.len_utf8()..].trim_start().to_string();
        }
    }
    // Numbered prefix: strip "1. " or "1) "
    let bytes = trimmed.as_bytes();
    let digit_end = bytes.iter().position(|&b| !b.is_ascii_digit()).unwrap_or(0);
    if digit_end > 0 && digit_end < bytes.len() {
        let suffix = bytes[digit_end];
        if suffix == b'.' || suffix == b')' {
            let after = &trimmed[digit_end + 1..];
            return after.trim_start().to_string();
        }
    }
    trimmed.to_string()
}

/// Guess whether page furniture is a header or footer based on vertical position.
fn guess_furniture_layer(para: &PdfParagraph) -> ContentLayer {
    // Use the layout class hint if available
    match para.layout_class {
        Some(LayoutHintClass::PageHeader) => ContentLayer::Header,
        Some(LayoutHintClass::PageFooter) => ContentLayer::Footer,
        Some(LayoutHintClass::Footnote) => ContentLayer::Footnote,
        _ => {
            // Heuristic: if baseline_y is high on the page (>700 in PDF coords),
            // it's likely a header. If low (<100), it's a footer.
            if let Some(first_line) = para.lines.first() {
                if first_line.baseline_y > 700.0 {
                    ContentLayer::Header
                } else if first_line.baseline_y < 100.0 {
                    ContentLayer::Footer
                } else {
                    ContentLayer::Header // default for furniture
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
        PdfParagraph {
            text: String::new(),
            lines: vec![PdfLine {
                segments: vec![SegmentData {
                    baseline_y,
                    ..plain_segment(text)
                }],
                baseline_y,
                dominant_font_size: 12.0,
                is_bold: false,
                is_monospace: false,
            }],
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
        }
    }

    #[test]
    fn test_assemble_internal_document_basic() {
        let pages = vec![vec![
            make_paragraph("Title", Some(1)),
            make_paragraph("Body text", None),
        ]];
        let doc = assemble_internal_document(pages, &[], &[]);
        assert_eq!(doc.elements.len(), 2);
        assert!(matches!(doc.elements[0].kind, ElementKind::Heading { level: 1 }));
        assert_eq!(doc.elements[0].text, "Title");
        assert!(matches!(doc.elements[1].kind, ElementKind::Paragraph));
        assert_eq!(doc.elements[1].text, "Body text");
    }

    #[test]
    fn test_assemble_internal_document_empty() {
        let doc = assemble_internal_document(vec![], &[], &[]);
        assert!(doc.elements.is_empty());
    }

    #[test]
    fn test_assemble_internal_document_multiple_pages() {
        let pages = vec![
            vec![make_paragraph("Page 1", None)],
            vec![make_paragraph("Page 2", None)],
        ];
        let doc = assemble_internal_document(pages, &[], &[]);
        // Should have page break between pages + 2 paragraphs
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
        let doc = assemble_internal_document(pages, &tables, &[]);
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
        let doc = assemble_internal_document(pages, &tables, &[]);
        assert!(doc.elements.iter().any(|e| e.text == "Page 1"));
        assert!(doc.elements.iter().any(|e| e.text == "Page 2"));
        assert!(doc.tables.iter().any(|t| t.markdown.contains("| Table |")));
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
        let doc = assemble_internal_document(pages, &tables, &[]);
        assert!(doc.elements.iter().any(|e| e.text == "Page 1"));
        assert!(doc.tables.iter().any(|t| t.markdown.contains("| Extra |")));
    }

    #[test]
    fn test_empty_table_markdown_not_rendered() {
        let pages = vec![vec![make_paragraph("Text", None)]];
        let tables = vec![crate::types::Table {
            cells: vec![],
            markdown: "   ".to_string(), // Whitespace-only markdown
            page_number: 1,
            bounding_box: None,
        }];
        let doc = assemble_internal_document(pages, &tables, &[]);
        // Table with whitespace-only markdown should be skipped
        assert!(doc.tables.is_empty() || doc.tables.iter().all(|t| t.markdown.trim().is_empty()));
    }

    #[test]
    fn test_no_page_break_when_leading_page_empty() {
        // Blank first page, content on second page — no PageBreak should be emitted.
        let pages = vec![
            vec![], // empty page 1
            vec![make_paragraph("Content on page 2", None)],
        ];
        let doc = assemble_internal_document(pages, &[], &[]);
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
        // Content on first page, blank second page — no PageBreak should be emitted.
        let pages = vec![
            vec![make_paragraph("Content on page 1", None)],
            vec![], // empty page 2
        ];
        let doc = assemble_internal_document(pages, &[], &[]);
        assert!(
            !doc.elements.iter().any(|e| matches!(e.kind, ElementKind::PageBreak)),
            "Blank trailing page should not produce a page break"
        );
    }

    #[test]
    fn test_page_break_between_content_pages() {
        // Both pages have content — PageBreak should be inserted between them.
        let pages = vec![
            vec![make_paragraph("Page 1", None)],
            vec![make_paragraph("Page 2", None)],
        ];
        let doc = assemble_internal_document(pages, &[], &[]);
        assert!(
            doc.elements.iter().any(|e| matches!(e.kind, ElementKind::PageBreak)),
            "PageBreak should separate two content pages"
        );
    }

    #[test]
    fn test_no_page_break_single_page() {
        // Single page with content — no PageBreak.
        let pages = vec![vec![make_paragraph("Only page", None)]];
        let doc = assemble_internal_document(pages, &[], &[]);
        assert!(
            !doc.elements.iter().any(|e| matches!(e.kind, ElementKind::PageBreak)),
            "Single page should not produce a page break"
        );
    }

    #[test]
    fn test_image_elements_injected_with_positions() {
        let pages = vec![vec![make_paragraph("Page with image", None)]];
        // Image at page 1 (1-indexed), image_index = 0
        let image_positions = vec![(1usize, 0usize)];
        let doc = assemble_internal_document(pages, &[], &image_positions);

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
    fn test_no_image_elements_with_empty_positions() {
        let pages = vec![vec![make_paragraph("No images here", None)]];
        let doc = assemble_internal_document(pages, &[], &[]);

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
        caption.caption_for = Some(0); // Caption for para at index 0
        let pages = vec![vec![para1, caption]];
        let doc = assemble_internal_document(pages, &[], &[]);
        // Main text paragraph should be present
        assert!(doc.elements.iter().any(|e| e.text == "Main text"));
        // Caption should be present as a separate element with italic annotation
        assert!(doc.elements.iter().any(|e| e.text == "Caption text"));
    }
}
