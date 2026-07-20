//! Derivation pipeline: converts `InternalDocument` → `DocumentStructure` + `ExtractedDocument`.
//!
//! This module bridges the internal flat document representation produced by extractors
//! and the public-facing types consumed by callers. It handles:
//!
//! - **Relationship resolution**: `RelationshipTarget::Key` → `RelationshipTarget::Index`
//! - **Tree reconstruction**: Flat elements → hierarchical `DocumentStructure`
//! - **Content string derivation**: Concatenation of text-carrying elements
//! - **ExtractedDocument assembly**: Combining all outputs into the final result

use std::borrow::Cow;
use std::sync::Arc;

use ahash::AHashMap;

use crate::types::document_structure::{
    DocumentNode, DocumentRelationship, DocumentStructure, GridCell, NodeContent, NodeId, NodeIndex, TableGrid,
};
use crate::types::extraction::{ExtractedDocument, ExtractionMethod};
use crate::types::internal::{ElementKind, InternalDocument, InternalElement, RelationshipTarget};
use crate::types::ocr_elements::{OcrConfidence, OcrElement};
use crate::types::page::PageContent;
use crate::types::tables::Table;

/// Resolve `RelationshipTarget::Key` entries to `RelationshipTarget::Index`.
///
/// Builds an anchor index from elements with non-`None` anchors, then resolves
/// each key-based relationship target. Unresolvable keys are logged and skipped
/// (the relationship is left as `Key` — it will be excluded from the final
/// `DocumentStructure` relationships).
pub(crate) fn resolve_relationships(doc: &mut InternalDocument) {
    let mut anchor_map: AHashMap<&str, u32> = AHashMap::new();
    for (idx, elem) in doc.elements.iter().enumerate() {
        if matches!(elem.kind, ElementKind::FootnoteRef) {
            continue;
        }
        if let Some(anchor) = elem.anchor.as_deref() {
            anchor_map.entry(anchor).or_insert(idx as u32);
        }
    }

    for rel in &mut doc.relationships {
        if let RelationshipTarget::Key(ref key) = rel.target {
            match anchor_map.get(key.as_str()) {
                Some(&idx) => {
                    rel.target = RelationshipTarget::Index(idx);
                }
                None => {
                    log::debug!("Unresolvable relationship key: {}", key);
                }
            }
        }
    }
}

/// Inner implementation that assumes relationships are already resolved.
///
/// Takes `&mut` so it can move data out of elements via `std::mem::take`,
/// avoiding clones. Callers that still need `elem.text` (build_pages,
/// build_ocr_elements) must run before this function.
fn derive_document_structure_inner(doc: &mut InternalDocument) -> DocumentStructure {
    let mut ds = DocumentStructure::with_capacity(doc.elements.len());
    ds.source_format = Some(doc.source_format.to_string());

    let mut stack: Vec<(u16, NodeIndex)> = Vec::new();

    let mut elem_to_node: Vec<Option<NodeIndex>> = vec![None; doc.elements.len()];

    let mut consumed: Vec<bool> = vec![false; doc.elements.len()];

    let mut def_pairs: AHashMap<usize, usize> = AHashMap::new();
    for i in 0..doc.elements.len().saturating_sub(1) {
        if matches!(doc.elements[i].kind, ElementKind::DefinitionTerm)
            && matches!(doc.elements[i + 1].kind, ElementKind::DefinitionDescription)
        {
            def_pairs.insert(i, i + 1);
            consumed[i + 1] = true;
        }
    }

    for elem_idx in 0..doc.elements.len() {
        if consumed[elem_idx] {
            continue;
        }
        match doc.elements[elem_idx].kind {
            ElementKind::ListEnd | ElementKind::QuoteEnd | ElementKind::GroupEnd => {
                close_container(&mut stack, &ds, doc.elements[elem_idx].kind);
                continue;
            }
            ElementKind::FootnoteRef => {
                continue;
            }
            _ => {}
        }

        let elem = &doc.elements[elem_idx];

        if elem.kind.is_container_start() {
            pop_stack_to_depth(&mut stack, elem.depth);
            let content = match elem.kind {
                ElementKind::ListStart { ordered } => NodeContent::List { ordered },
                ElementKind::QuoteStart => NodeContent::Quote,
                ElementKind::GroupStart => NodeContent::Group {
                    label: elem.attributes.as_ref().and_then(|a| a.get("label").cloned()),
                    heading_level: None,
                    heading_text: None,
                },
                _ => unreachable!("variant already checked by is_container_start()"),
            };
            let node_idx = push_node(&mut ds, &stack, content, elem, elem_idx as u32);
            elem_to_node[elem_idx] = Some(node_idx);
            stack.push((elem.depth, node_idx));
            continue;
        }

        if let ElementKind::Heading { level } = elem.kind {
            pop_stack_to_depth(&mut stack, elem.depth);

            let text = std::mem::take(&mut doc.elements[elem_idx].text);
            let annotations = std::mem::take(&mut doc.elements[elem_idx].annotations);
            let elem = &doc.elements[elem_idx];

            let group_content = NodeContent::Group {
                label: None,
                heading_level: Some(level),
                heading_text: Some(text.clone()),
            };

            let group_idx = push_node(&mut ds, &stack, group_content, elem, elem_idx as u32);

            let heading_node_index = ds.len() as u32;
            let heading_node = DocumentNode {
                id: NodeId::generate("heading", &text, elem.page, heading_node_index),
                content: NodeContent::Heading { level, text },
                parent: Some(group_idx),
                children: vec![],
                content_layer: elem.layer,
                page: elem.page,
                page_end: None,
                bbox: elem.bbox,
                annotations,
                attributes: elem
                    .attributes
                    .as_ref()
                    .map(|a| a.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
            };
            let heading_idx = ds.push_node(heading_node);
            ds.nodes[group_idx.0 as usize].children.push(heading_idx);

            elem_to_node[elem_idx] = Some(group_idx);
            stack.push((elem.depth, group_idx));
            continue;
        }

        if let Some(&desc_idx) = def_pairs.get(&elem_idx) {
            pop_stack_to_depth(&mut stack, elem.depth);

            let is_in_def_list = stack
                .last()
                .is_some_and(|(_, idx)| matches!(ds.nodes[idx.0 as usize].content, NodeContent::DefinitionList));
            if !is_in_def_list {
                let dl_idx = push_node(&mut ds, &stack, NodeContent::DefinitionList, elem, elem_idx as u32);
                stack.push((elem.depth, dl_idx));
            }

            let term = std::mem::take(&mut doc.elements[elem_idx].text);
            let definition = std::mem::take(&mut doc.elements[desc_idx].text);
            let elem = &doc.elements[elem_idx];
            let content = NodeContent::DefinitionItem { term, definition };
            let node_idx = push_node(&mut ds, &stack, content, elem, elem_idx as u32);
            elem_to_node[elem_idx] = Some(node_idx);
            elem_to_node[desc_idx] = Some(node_idx);
            continue;
        }

        if matches!(
            elem.kind,
            ElementKind::DefinitionTerm | ElementKind::DefinitionDescription
        ) {
            pop_stack_to_depth(&mut stack, elem.depth);

            let is_in_def_list = stack
                .last()
                .is_some_and(|(_, idx)| matches!(ds.nodes[idx.0 as usize].content, NodeContent::DefinitionList));
            if !is_in_def_list {
                let dl_idx = push_node(&mut ds, &stack, NodeContent::DefinitionList, elem, elem_idx as u32);
                stack.push((elem.depth, dl_idx));
            }

            let content = element_to_node_content(&mut doc.elements[elem_idx], &doc.tables, &doc.images);
            let annotations = std::mem::take(&mut doc.elements[elem_idx].annotations);
            let node_idx = push_node_with_annotations(
                &mut ds,
                &stack,
                content,
                &doc.elements[elem_idx],
                annotations,
                elem_idx as u32,
            );
            elem_to_node[elem_idx] = Some(node_idx);
            continue;
        }

        if stack
            .last()
            .is_some_and(|(_, idx)| matches!(ds.nodes[idx.0 as usize].content, NodeContent::DefinitionList))
        {
            stack.pop();
        }

        pop_stack_to_depth(&mut stack, elem.depth);
        let content = element_to_node_content(&mut doc.elements[elem_idx], &doc.tables, &doc.images);
        let annotations = std::mem::take(&mut doc.elements[elem_idx].annotations);
        let node_idx = push_node_with_annotations(
            &mut ds,
            &stack,
            content,
            &doc.elements[elem_idx],
            annotations,
            elem_idx as u32,
        );
        elem_to_node[elem_idx] = Some(node_idx);
    }

    for rel in &doc.relationships {
        if let RelationshipTarget::Index(target_elem_idx) = rel.target {
            let source_node = elem_to_node
                .get(rel.source as usize)
                .and_then(|n| *n)
                .or_else(|| (0..rel.source as usize).rev().find_map(|i| elem_to_node[i]));
            let target_node = elem_to_node.get(target_elem_idx as usize).and_then(|n| *n);
            if let (Some(src), Some(tgt)) = (source_node, target_node) {
                ds.relationships.push(DocumentRelationship {
                    source: src,
                    target: tgt,
                    kind: rel.kind,
                });
            }
        }
    }

    debug_assert!(
        ds.validate().is_ok(),
        "DocumentStructure validation failed: {:?}",
        ds.validate()
    );

    ds.finalize_node_types();
    ds
}

/// Close the nearest explicit container matching an end marker.
///
/// Derived heading groups may sit above an explicit container on the stack. An
/// end marker closes both those derived groups and its matching container,
/// rather than mistaking the heading group for the explicit group itself.
fn close_container(stack: &mut Vec<(u16, NodeIndex)>, ds: &DocumentStructure, end_kind: ElementKind) {
    let Some(container_position) = stack.iter().rposition(|(_, node_idx)| {
        let content = &ds.nodes[node_idx.0 as usize].content;
        matches!(
            (end_kind, content),
            (ElementKind::ListEnd, NodeContent::List { .. })
                | (ElementKind::QuoteEnd, NodeContent::Quote)
                | (
                    ElementKind::GroupEnd,
                    NodeContent::Group {
                        heading_level: None,
                        ..
                    }
                )
        )
    }) else {
        return;
    };

    stack.truncate(container_position);
}

/// Pop the stack until the top has depth strictly less than `target_depth`.
fn pop_stack_to_depth(stack: &mut Vec<(u16, NodeIndex)>, target_depth: u16) {
    while stack.last().is_some_and(|(d, _)| *d >= target_depth) {
        stack.pop();
    }
}

/// Push a DocumentNode under the current stack top (or as root if stack is empty).
/// Clones annotations from the element. For cases where annotations have already
/// been taken, use `push_node_with_annotations` instead.
fn push_node(
    ds: &mut DocumentStructure,
    stack: &[(u16, NodeIndex)],
    content: NodeContent,
    elem: &InternalElement,
    _index: u32,
) -> NodeIndex {
    push_node_with_annotations(ds, stack, content, elem, elem.annotations.clone(), _index)
}

/// Push a DocumentNode with explicitly provided annotations (avoids cloning when
/// annotations have already been taken from the element).
fn push_node_with_annotations(
    ds: &mut DocumentStructure,
    stack: &[(u16, NodeIndex)],
    content: NodeContent,
    elem: &InternalElement,
    annotations: Vec<crate::types::document_structure::TextAnnotation>,
    _index: u32,
) -> NodeIndex {
    let node_type = content.node_type_str();
    let text_for_id = content.text().unwrap_or("");

    let node_index_val = ds.len() as u32;
    let node = DocumentNode {
        id: NodeId::generate(node_type, text_for_id, elem.page, node_index_val),
        content,
        parent: None,
        children: vec![],
        content_layer: elem.layer,
        page: elem.page,
        page_end: None,
        bbox: elem.bbox,
        annotations,
        attributes: elem
            .attributes
            .as_ref()
            .map(|a| a.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
    };

    let node_idx = ds.push_node(node);

    if let Some((_, parent_idx)) = stack.last() {
        ds.add_child(*parent_idx, node_idx);
    }

    node_idx
}

/// Convert an `InternalElement` + `ElementKind` into `NodeContent`.
///
/// Takes `&mut` so it can move text out via `std::mem::take` (pages/OCR have
/// already consumed what they need before this is called).
fn element_to_node_content(
    elem: &mut InternalElement,
    tables: &[Table],
    images: &[crate::types::ExtractedImage],
) -> NodeContent {
    match elem.kind {
        ElementKind::Title => NodeContent::Title {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::Paragraph => NodeContent::Paragraph {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::ListItem { .. } => NodeContent::ListItem {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::Code => NodeContent::Code {
            text: std::mem::take(&mut elem.text),
            language: elem.attributes.as_ref().and_then(|a| a.get("language").cloned()),
        },
        ElementKind::Formula => NodeContent::Formula {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::FootnoteDefinition => NodeContent::Footnote {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::Citation => NodeContent::Citation {
            key: elem.anchor.clone().unwrap_or_default(),
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::Table { table_index } => {
            let grid = if let Some(table) = tables.get(table_index as usize) {
                table_to_grid(table)
            } else {
                TableGrid {
                    rows: 0,
                    cols: 0,
                    cells: vec![],
                }
            };
            NodeContent::Table { grid }
        }
        ElementKind::Image { image_index } => {
            let description = images.get(image_index as usize).and_then(|img| img.description.clone());
            let src = elem.attributes.as_ref().and_then(|attrs| attrs.get("src").cloned());
            NodeContent::Image {
                description,
                image_index: Some(image_index),
                src,
            }
        }
        ElementKind::PageBreak => NodeContent::PageBreak,
        ElementKind::Slide { number } => NodeContent::Slide {
            number,
            title: if elem.text.is_empty() {
                None
            } else {
                Some(std::mem::take(&mut elem.text))
            },
        },
        ElementKind::DefinitionTerm | ElementKind::DefinitionDescription => {
            let text = std::mem::take(&mut elem.text);
            if matches!(elem.kind, ElementKind::DefinitionTerm) {
                NodeContent::DefinitionItem {
                    term: text,
                    definition: String::new(),
                }
            } else {
                NodeContent::DefinitionItem {
                    term: String::new(),
                    definition: text,
                }
            }
        }
        ElementKind::Admonition => {
            let attrs = elem.attributes.as_ref();
            NodeContent::Admonition {
                kind: attrs
                    .and_then(|a| a.get("kind").cloned())
                    .unwrap_or_else(|| "note".to_string()),
                title: attrs.and_then(|a| a.get("title").cloned()),
            }
        }
        ElementKind::RawBlock => {
            let attrs = elem.attributes.as_ref();
            NodeContent::RawBlock {
                format: attrs.and_then(|a| a.get("format").cloned()).unwrap_or_default(),
                content: std::mem::take(&mut elem.text),
            }
        }
        ElementKind::MetadataBlock => {
            let entries = parse_metadata_entries(&elem.text);
            NodeContent::MetadataBlock { entries }
        }
        ElementKind::OcrText { .. } => NodeContent::Paragraph {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::ListStart { ordered } => NodeContent::List { ordered },
        ElementKind::QuoteStart => NodeContent::Quote,
        ElementKind::GroupStart => NodeContent::Group {
            label: None,
            heading_level: None,
            heading_text: None,
        },
        ElementKind::Heading { level } => NodeContent::Heading {
            level,
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::FootnoteRef => NodeContent::Paragraph {
            text: std::mem::take(&mut elem.text),
        },
        ElementKind::ListEnd | ElementKind::QuoteEnd | ElementKind::GroupEnd => {
            unreachable!("container end markers should be filtered before this point")
        }
    }
}

/// Convert an internal `Table` to a `TableGrid`.
fn table_to_grid(table: &Table) -> TableGrid {
    let rows = table.cells.len() as u32;
    let cols = table.cells.iter().map(|r| r.len()).max().unwrap_or(0) as u32;

    let mut cells = Vec::new();
    for (row_idx, row) in table.cells.iter().enumerate() {
        for (col_idx, cell_content) in row.iter().enumerate() {
            cells.push(GridCell {
                content: cell_content.clone(),
                row: row_idx as u32,
                col: col_idx as u32,
                row_span: 1,
                col_span: 1,
                is_header: row_idx == 0,
                bbox: None,
            });
        }
    }

    TableGrid { rows, cols, cells }
}

/// Parse "key: value" lines from metadata text into `(key, value)` pairs.
fn parse_metadata_entries(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                Some((key, value))
            } else {
                Some((line.to_string(), String::new()))
            }
        })
        .collect()
}

/// Derive a complete `ExtractedDocument` from an `InternalDocument`.
///
/// This is the main entry point for the derivation pipeline. It:
/// 1. Resolves relationships (needed by renderers for footnotes)
/// 2. Renders plain-text content (for post-processors)
/// 3. Pre-renders formatted content if output_format != Plain
/// 4. Groups elements by page into `PageContent`
/// 5. Extracts OCR elements for backward compatibility
/// 6. Optionally derives `DocumentStructure` (assumes relationships resolved)
/// 7. Assembles the final `ExtractedDocument`
#[cfg_attr(alef, alef(skip))]
pub fn derive_extraction_result(
    mut doc: InternalDocument,
    include_document_structure: bool,
    output_format: crate::core::config::OutputFormat,
) -> ExtractedDocument {
    tracing::debug!(
        element_count = doc.elements.len(),
        source_format = %doc.source_format,
        include_document_structure,
        "derivation pipeline starting"
    );
    resolve_relationships(&mut doc);

    let content = crate::rendering::render_plain(&doc);

    let mime_type: Cow<'static, str> = if doc.mime_type != "application/octet-stream" {
        Cow::Owned(std::mem::take(&mut doc.mime_type))
    } else {
        Cow::Borrowed(source_format_to_mime_type(&doc.source_format))
    };

    let formatted_content = match output_format {
        crate::core::config::OutputFormat::Plain => None,
        crate::core::config::OutputFormat::Markdown => {
            if doc.pre_rendered_content.is_some() && doc.metadata.output_format.as_deref() == Some("markdown") {
                doc.pre_rendered_content.take()
            } else {
                Some(crate::rendering::render_markdown(&doc))
            }
        }
        crate::core::config::OutputFormat::Djot => {
            if doc.pre_rendered_content.is_some() && doc.metadata.output_format.as_deref() == Some("djot") {
                doc.pre_rendered_content.take()
            } else {
                Some(crate::rendering::render_djot(&doc))
            }
        }
        crate::core::config::OutputFormat::Html => Some(crate::rendering::render_html(&doc)),
        crate::core::config::OutputFormat::Json => Some(crate::rendering::render_json(&doc)),
        crate::core::config::OutputFormat::Structured => None,
        crate::core::config::OutputFormat::Custom(ref name) => {
            let registry = crate::plugins::registry::get_renderer_registry();
            let registry = registry.read();
            match registry.render(name, &doc) {
                Ok(rendered) => Some(rendered),
                Err(e) => {
                    tracing::warn!(renderer = %name, error = %e, "Custom renderer failed, falling back to plain");
                    None
                }
            }
        }
    };

    let raw_pages = doc.prebuilt_pages.take().or_else(|| build_pages(&doc));
    let pages = apply_page_content_format(raw_pages, &doc, &output_format);
    let ocr_elements = doc.prebuilt_ocr_elements.take().or_else(|| build_ocr_elements(&doc));

    let document = if include_document_structure {
        Some(derive_document_structure_inner(&mut doc))
    } else {
        None
    };

    let images = if doc.images.is_empty() { None } else { Some(doc.images) };

    let uris = if doc.uris.is_empty() {
        None
    } else {
        let mut seen = ahash::AHashSet::with_capacity(doc.uris.len());
        doc.uris.retain(|uri| seen.insert((uri.url.clone(), uri.kind)));
        Some(doc.uris)
    };

    #[cfg(feature = "tree-sitter")]
    let code_intelligence: Option<serde_json::Value> = None;

    let extraction_method = doc
        .metadata
        .additional
        .get("extraction_method")
        .and_then(serde_json::Value::as_str)
        .and_then(ExtractionMethod::from_metadata_value);

    tracing::debug!(
        content_length = content.len(),
        has_document_structure = document.is_some(),
        "derivation pipeline complete"
    );
    ExtractedDocument {
        content,
        mime_type,
        metadata: doc.metadata,
        extraction_method,
        tables: doc.tables,
        images,
        pages,
        ocr_elements,
        document,
        processing_warnings: std::mem::take(&mut doc.processing_warnings),
        annotations: std::mem::take(&mut doc.annotations),
        children: std::mem::take(&mut doc.children),
        uris,
        llm_usage: std::mem::take(&mut doc.llm_usage),
        revisions: std::mem::take(&mut doc.revisions),
        form_fields: std::mem::take(&mut doc.form_fields),
        formulas: std::mem::take(&mut doc.formulas),
        #[cfg(feature = "tree-sitter")]
        code_intelligence,
        formatted_content,
        ..Default::default()
    }
}

/// Map source format identifiers to MIME types.
fn source_format_to_mime_type(format: &str) -> &'static str {
    match format {
        "pdf" => "application/pdf",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "doc" => "application/msword",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "ppt" => "application/vnd.ms-powerpoint",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xls" => "application/vnd.ms-excel",
        "html" => "text/html",
        "markdown" | "md" => "text/markdown",
        "xml" => "application/xml",
        "json" => "application/json",
        "yaml" | "yml" => "application/yaml",
        "toml" => "application/toml",
        "csv" => "text/csv",
        "eml" | "msg" => "message/rfc822",
        "pst" => "application/vnd.ms-outlook-pst",
        "rtf" => "application/rtf",
        "txt" | "text" => "text/plain",
        "djot" => "text/djot",
        _ => "application/octet-stream",
    }
}

/// Build per-page `PageContent` from page-grouped elements.
fn build_pages(doc: &InternalDocument) -> Option<Vec<PageContent>> {
    let mut page_map: std::collections::BTreeMap<u32, Vec<&InternalElement>> = std::collections::BTreeMap::new();

    for elem in &doc.elements {
        if let Some(page) = elem.page {
            page_map.entry(page).or_default().push(elem);
        }
    }

    if page_map.is_empty() {
        return None;
    }

    let arc_tables: Vec<Arc<Table>> = doc.tables.iter().map(|t| Arc::new(t.clone())).collect();

    let pages: Vec<PageContent> = page_map
        .into_iter()
        .map(|(page_num, elems)| {
            let mut content = String::new();
            let mut tables = Vec::new();
            let mut image_indices = Vec::new();
            for elem in &elems {
                if elem.kind.is_container_start() || elem.kind.is_container_end() {
                    continue;
                }
                match elem.kind {
                    ElementKind::Table { table_index } => {
                        if let Some(arc_table) = arc_tables.get(table_index as usize) {
                            tables.push(Arc::clone(arc_table));
                        }
                    }
                    ElementKind::Image { image_index } if (image_index as usize) < doc.images.len() => {
                        image_indices.push(image_index);
                    }
                    _ => {}
                }
                if !elem.text.is_empty() {
                    if !content.is_empty() {
                        content.push_str("\n\n");
                    }
                    content.push_str(&elem.text);
                }
            }

            PageContent {
                page_number: page_num,
                content,
                tables,
                image_indices,
                hierarchy: None,
                is_blank: None,
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
            }
        })
        .collect();

    Some(pages)
}

/// Re-render each page's content using the requested output format.
///
/// Called after pages are built but before `derive_document_structure_inner` moves
/// element text out of the document. For Plain/Structured/Json/Custom formats this
/// is a no-op. For Markdown/Djot/Html, each page's element subset is rendered with
/// the same renderer used for the full document, so `pages[n].content` matches the
/// format of `result.content` after `apply_output_format`.
///
/// Pages whose `page_number` has no matching page-tagged elements (e.g., natively
/// extracted PDF pages where individual elements are not page-tracked) are returned
/// unchanged — their original content is preserved.
fn apply_page_content_format(
    pages: Option<Vec<PageContent>>,
    doc: &InternalDocument,
    output_format: &crate::core::config::OutputFormat,
) -> Option<Vec<PageContent>> {
    use crate::core::config::OutputFormat;

    let renderer: fn(&InternalDocument) -> String = match output_format {
        OutputFormat::Markdown => crate::rendering::render_markdown,
        OutputFormat::Djot => crate::rendering::render_djot,
        OutputFormat::Html => crate::rendering::render_html,
        OutputFormat::Plain | OutputFormat::Structured | OutputFormat::Json | OutputFormat::Custom(_) => {
            return pages;
        }
    };

    let pages = pages?;

    let mut elements_by_page: std::collections::BTreeMap<u32, Vec<usize>> = std::collections::BTreeMap::new();
    for (idx, elem) in doc.elements.iter().enumerate() {
        if let Some(page_num) = elem.page {
            elements_by_page.entry(page_num).or_default().push(idx);
        }
    }

    if elements_by_page.is_empty() {
        return Some(pages);
    }

    let pages = pages
        .into_iter()
        .map(|mut page| {
            let Some(elem_indices) = elements_by_page.get(&page.page_number) else {
                return page;
            };

            let mut table_remap: ahash::AHashMap<u32, u32> = ahash::AHashMap::new();
            let mut sub_tables: Vec<Table> = Vec::new();
            let mut image_remap: ahash::AHashMap<u32, u32> = ahash::AHashMap::new();
            let mut sub_images: Vec<crate::types::ExtractedImage> = Vec::new();
            for &i in elem_indices {
                match doc.elements[i].kind {
                    ElementKind::Table { table_index } if !table_remap.contains_key(&table_index) => {
                        let new_idx = sub_tables.len() as u32;
                        table_remap.insert(table_index, new_idx);
                        if let Some(t) = doc.tables.get(table_index as usize) {
                            sub_tables.push(t.clone());
                        }
                    }
                    ElementKind::Image { image_index } if !image_remap.contains_key(&image_index) => {
                        let new_idx = sub_images.len() as u32;
                        image_remap.insert(image_index, new_idx);
                        if let Some(img) = doc.images.get(image_index as usize) {
                            sub_images.push(img.clone());
                        }
                    }
                    _ => {}
                }
            }

            let elements: Vec<InternalElement> = elem_indices
                .iter()
                .map(|&i| {
                    let mut elem = doc.elements[i].clone();
                    match elem.kind {
                        ElementKind::Table { ref mut table_index } => {
                            if let Some(&new_idx) = table_remap.get(table_index) {
                                *table_index = new_idx;
                            }
                        }
                        ElementKind::Image { ref mut image_index } => {
                            if let Some(&new_idx) = image_remap.get(image_index) {
                                *image_index = new_idx;
                            }
                        }
                        _ => {}
                    }
                    elem
                })
                .collect();

            let mut sub_doc = InternalDocument::new(&doc.source_format);
            sub_doc.elements = elements;
            sub_doc.tables = sub_tables;
            sub_doc.images = sub_images;

            let rendered = renderer(&sub_doc);
            if !rendered.is_empty() {
                page.content = rendered;
            }
            page
        })
        .collect();

    Some(pages)
}

/// Extract `OcrElement` entries from OCR-typed internal elements.
fn build_ocr_elements(doc: &InternalDocument) -> Option<Vec<OcrElement>> {
    let ocr_elems: Vec<OcrElement> = doc
        .elements
        .iter()
        .filter_map(|elem| {
            if let ElementKind::OcrText { level } = elem.kind {
                let geometry = elem.ocr_geometry.clone()?;
                let confidence = elem.ocr_confidence.clone().unwrap_or(OcrConfidence {
                    detection: None,
                    recognition: 0.0,
                });
                Some(OcrElement {
                    text: elem.text.clone(),
                    geometry,
                    confidence,
                    level,
                    rotation: elem.ocr_rotation.clone(),
                    page_number: elem.page.unwrap_or(1),
                    parent_id: None,
                    backend_metadata: std::collections::HashMap::new(),
                })
            } else {
                None
            }
        })
        .collect();

    if ocr_elems.is_empty() { None } else { Some(ocr_elems) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::document_structure::NodeContent;
    use crate::types::internal::{
        ElementKind, InternalDocument, InternalElement, Relationship, RelationshipKind, RelationshipTarget,
    };

    /// Helper: create a minimal internal document.
    fn make_doc(source_format: &'static str) -> InternalDocument {
        InternalDocument::new(source_format)
    }

    #[test]
    fn test_flat_document_produces_flat_tree() {
        let mut doc = make_doc("markdown");
        doc.push_element(InternalElement::text(ElementKind::Title, "My Title", 0));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "First paragraph.", 0));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Second paragraph.", 0));

        resolve_relationships(&mut doc);
        let ds = derive_document_structure_inner(&mut doc);
        assert!(ds.validate().is_ok(), "validation: {:?}", ds.validate());
        assert_eq!(ds.len(), 3);

        let roots: Vec<_> = ds.body_roots().collect();
        assert_eq!(roots.len(), 3);

        match &roots[0].1.content {
            NodeContent::Title { text } => assert_eq!(text, "My Title"),
            other => panic!("Expected Title, got {:?}", other),
        }
    }

    #[test]
    fn test_heading_nesting() {
        let mut doc = make_doc("markdown");
        doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Chapter 1", 0));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Intro text.", 1));
        doc.push_element(InternalElement::text(
            ElementKind::Heading { level: 2 },
            "Section 1.1",
            1,
        ));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Section body.", 2));

        resolve_relationships(&mut doc);
        let ds = derive_document_structure_inner(&mut doc);
        assert!(ds.validate().is_ok(), "validation: {:?}", ds.validate());

        let roots: Vec<_> = ds.body_roots().collect();
        assert_eq!(roots.len(), 1);

        let h1_group = &ds.nodes[roots[0].0.0 as usize];
        match &h1_group.content {
            NodeContent::Group {
                heading_level,
                heading_text,
                ..
            } => {
                assert_eq!(*heading_level, Some(1));
                assert_eq!(heading_text.as_deref(), Some("Chapter 1"));
            }
            other => panic!("Expected Group, got {:?}", other),
        }

        assert_eq!(h1_group.children.len(), 3);

        let heading_node = &ds.nodes[h1_group.children[0].0 as usize];
        assert!(matches!(&heading_node.content, NodeContent::Heading { level: 1, .. }));

        let para_node = &ds.nodes[h1_group.children[1].0 as usize];
        assert!(matches!(&para_node.content, NodeContent::Paragraph { .. }));

        let h2_group = &ds.nodes[h1_group.children[2].0 as usize];
        match &h2_group.content {
            NodeContent::Group {
                heading_level,
                heading_text,
                ..
            } => {
                assert_eq!(*heading_level, Some(2));
                assert_eq!(heading_text.as_deref(), Some("Section 1.1"));
            }
            other => panic!("Expected H2 Group, got {:?}", other),
        }

        assert_eq!(h2_group.children.len(), 2);
    }

    #[test]
    fn test_group_end_closes_layout_group_beneath_heading() {
        let mut doc = make_doc("pdf");
        doc.push_element(InternalElement::text(ElementKind::GroupStart, "", 0));
        doc.push_element(InternalElement::text(
            ElementKind::Heading { level: 1 },
            "Region heading",
            1,
        ));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Region body", 2));
        doc.push_element(InternalElement::text(ElementKind::GroupEnd, "", 0));
        doc.push_element(InternalElement::text(ElementKind::Table { table_index: 0 }, "", 1));
        doc.push_element(InternalElement::text(ElementKind::PageBreak, "", 1));
        doc.push_element(InternalElement::text(ElementKind::GroupStart, "", 1));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Next region", 2));
        doc.push_element(InternalElement::text(ElementKind::GroupEnd, "", 1));

        resolve_relationships(&mut doc);
        let ds = derive_document_structure_inner(&mut doc);
        assert!(ds.validate().is_ok(), "validation: {:?}", ds.validate());

        let roots: Vec<_> = ds.body_roots().collect();
        assert_eq!(roots.len(), 4);
        assert!(matches!(
            &roots[0].1.content,
            NodeContent::Group {
                heading_level: None,
                ..
            }
        ));
        assert!(matches!(&roots[1].1.content, NodeContent::Table { .. }));
        assert!(matches!(&roots[2].1.content, NodeContent::PageBreak));
        assert!(matches!(
            &roots[3].1.content,
            NodeContent::Group {
                heading_level: None,
                ..
            }
        ));

        let first_group = &ds.nodes[roots[0].0.0 as usize];
        assert_eq!(first_group.children.len(), 1);
        let heading_group = &ds.nodes[first_group.children[0].0 as usize];
        assert!(matches!(
            &heading_group.content,
            NodeContent::Group {
                heading_level: Some(1),
                ..
            }
        ));

        let next_group = &ds.nodes[roots[3].0.0 as usize];
        assert_eq!(next_group.children.len(), 1);
        assert!(matches!(
            &ds.nodes[next_group.children[0].0 as usize].content,
            NodeContent::Paragraph { .. }
        ));
    }

    #[test]
    fn test_relationship_resolution() {
        let mut doc = make_doc("markdown");

        doc.push_element(InternalElement::text(ElementKind::Paragraph, "See note [^fn1].", 0));

        doc.push_element(InternalElement::text(ElementKind::FootnoteRef, "fn1", 0).with_anchor("fn1"));

        doc.push_element(
            InternalElement::text(ElementKind::FootnoteDefinition, "This is the footnote.", 0).with_anchor("fn1"),
        );

        doc.push_relationship(Relationship {
            source: 1,
            target: RelationshipTarget::Key("fn1".to_string()),
            kind: RelationshipKind::FootnoteReference,
        });

        resolve_relationships(&mut doc);

        match &doc.relationships[0].target {
            RelationshipTarget::Index(idx) => assert_eq!(*idx, 2),
            RelationshipTarget::Key(k) => panic!("Expected resolved Index, got Key({:?})", k),
        }
    }

    #[test]
    fn test_unresolvable_key_left_as_key() {
        let mut doc = make_doc("markdown");
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Ref.", 0));

        doc.push_relationship(Relationship {
            source: 0,
            target: RelationshipTarget::Key("nonexistent".to_string()),
            kind: RelationshipKind::InternalLink,
        });

        resolve_relationships(&mut doc);

        assert!(matches!(
            &doc.relationships[0].target,
            RelationshipTarget::Key(k) if k == "nonexistent"
        ));
    }

    #[test]
    fn test_relationships_in_document_structure() {
        let mut doc = make_doc("markdown");

        doc.push_element(InternalElement::text(ElementKind::Paragraph, "See note.", 0));
        doc.push_element(InternalElement::text(ElementKind::FootnoteDefinition, "The note.", 0).with_anchor("fn1"));

        doc.push_relationship(Relationship {
            source: 0,
            target: RelationshipTarget::Index(1),
            kind: RelationshipKind::FootnoteReference,
        });

        resolve_relationships(&mut doc);
        let ds = derive_document_structure_inner(&mut doc);
        assert!(ds.validate().is_ok());
        assert_eq!(ds.relationships.len(), 1);
        assert_eq!(ds.relationships[0].kind, RelationshipKind::FootnoteReference);
    }

    #[test]
    fn test_list_container() {
        let mut doc = make_doc("markdown");
        doc.push_element(InternalElement::text(ElementKind::ListStart { ordered: false }, "", 0));
        doc.push_element(InternalElement::text(
            ElementKind::ListItem { ordered: false },
            "Item A",
            1,
        ));
        doc.push_element(InternalElement::text(
            ElementKind::ListItem { ordered: false },
            "Item B",
            1,
        ));
        doc.push_element(InternalElement::text(ElementKind::ListEnd, "", 0));

        resolve_relationships(&mut doc);
        let ds = derive_document_structure_inner(&mut doc);
        assert!(ds.validate().is_ok(), "validation: {:?}", ds.validate());

        let roots: Vec<_> = ds.body_roots().collect();
        assert_eq!(roots.len(), 1);
        assert!(matches!(&roots[0].1.content, NodeContent::List { ordered: false }));

        assert_eq!(ds.nodes[roots[0].0.0 as usize].children.len(), 2);
    }

    #[test]
    fn test_derive_extraction_result_basic() {
        let mut doc = make_doc("markdown");
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Hello world.", 0));

        let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
        assert_eq!(result.content, "Hello world.");
        assert_eq!(result.mime_type, "text/markdown");
        assert!(result.document.is_none());
    }

    #[cfg(any(feature = "pdf", feature = "ocr"))]
    #[test]
    fn test_derive_extraction_result_with_structure() {
        let mut doc = make_doc("pdf");
        doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Title", 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body.", 1).with_page(1));

        let result = derive_extraction_result(doc, true, crate::core::config::OutputFormat::Plain);
        assert!(result.document.is_some());
        let ds = result.document.unwrap();
        assert!(ds.validate().is_ok());
        assert_eq!(ds.source_format.as_deref(), Some("pdf"));
    }

    #[cfg(any(feature = "pdf", feature = "ocr"))]
    #[test]
    fn test_source_format_cow_owned_propagates() {
        let owned: std::borrow::Cow<'static, str> = std::borrow::Cow::Owned("epub".to_string());
        let mut doc = InternalDocument::new(owned);
        doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Ch1", 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body.", 1).with_page(1));

        let result = derive_extraction_result(doc, true, crate::core::config::OutputFormat::Plain);
        let ds = result.document.unwrap();
        assert_eq!(ds.source_format.as_deref(), Some("epub"));
    }

    #[test]
    fn test_derive_extraction_result_promotes_extraction_method() {
        let mut doc = make_doc("pdf");
        doc.metadata.additional.insert(
            Cow::Borrowed("extraction_method"),
            serde_json::Value::String("mixed".to_string()),
        );
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Hello world.", 0));

        let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
        assert_eq!(result.extraction_method, Some(ExtractionMethod::Mixed));
    }

    #[test]
    fn test_derive_extraction_result_ignores_unknown_extraction_method() {
        let mut doc = make_doc("pdf");
        doc.metadata.additional.insert(
            Cow::Borrowed("extraction_method"),
            serde_json::Value::String("native_ole".to_string()),
        );
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Hello world.", 0));

        let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
        assert_eq!(result.extraction_method, None);
    }

    /// Pages with a heading element must render `# Heading` when output_format=Markdown.
    ///
    /// Before the fix, `PageContent.content` always contained raw element text ("Introduction"),
    /// not the formatted representation ("# Introduction"). This tests the full pipeline
    /// state as seen by callers (derive + apply_output_format).
    #[cfg(any(
        feature = "ocr",
        feature = "office",
        feature = "pdf",
        feature = "paddle-ocr",
        feature = "xml",
        feature = "hwpx",
        feature = "quality",
        feature = "chunking"
    ))]
    #[test]
    fn page_content_markdown_heading_is_formatted() {
        let mut doc = make_doc("docx");
        doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Introduction", 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body text here.", 0).with_page(1));

        let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Markdown);
        let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Markdown);

        let pages = result
            .pages
            .expect("pages must be populated when elements have page numbers");
        assert_eq!(pages.len(), 1);

        assert!(
            result.content.contains("# Introduction"),
            "full content must have markdown heading, got: {:?}",
            result.content,
        );
        assert!(
            pages[0].content.contains("# Introduction"),
            "page content must use markdown heading format, got: {:?}",
            pages[0].content,
        );
        assert!(
            !pages[0].content.trim_start().starts_with("Introduction"),
            "page content must not start with bare heading text without '#', got: {:?}",
            pages[0].content,
        );
    }

    /// Plain output must leave page content as raw element text — no regressions.
    #[cfg(any(
        feature = "ocr",
        feature = "office",
        feature = "pdf",
        feature = "paddle-ocr",
        feature = "xml",
        feature = "hwpx",
        feature = "quality",
        feature = "chunking"
    ))]
    #[test]
    fn page_content_plain_format_unchanged() {
        let mut doc = make_doc("docx");
        doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Introduction", 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body text here.", 0).with_page(1));

        let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
        let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Plain);

        let pages = result.pages.expect("pages must be populated");
        assert_eq!(pages.len(), 1);

        assert!(
            !pages[0].content.contains("# Introduction"),
            "plain-format page content must not contain markdown heading prefix, got: {:?}",
            pages[0].content,
        );
        assert!(
            pages[0].content.contains("Introduction"),
            "plain-format page content must still contain the heading text, got: {:?}",
            pages[0].content,
        );
    }

    /// Each page's formatted content must only contain that page's elements.
    #[cfg(any(
        feature = "ocr",
        feature = "office",
        feature = "pdf",
        feature = "paddle-ocr",
        feature = "xml",
        feature = "hwpx",
        feature = "quality",
        feature = "chunking"
    ))]
    #[test]
    fn page_content_per_page_isolation() {
        let mut doc = make_doc("docx");
        doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Chapter One", 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Content of page one.", 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Chapter Two", 0).with_page(2));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Content of page two.", 0).with_page(2));

        let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Markdown);
        let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Markdown);

        let pages = result.pages.expect("pages must be populated");
        assert_eq!(pages.len(), 2);

        let p1 = pages.iter().find(|p| p.page_number == 1).expect("page 1");
        let p2 = pages.iter().find(|p| p.page_number == 2).expect("page 2");

        assert!(
            !p1.content.contains("Chapter Two"),
            "page 1 must not bleed page 2's content, got: {:?}",
            p1.content,
        );
        assert!(
            !p2.content.contains("Chapter One"),
            "page 2 must not include page 1's content, got: {:?}",
            p2.content,
        );
        assert!(
            p1.content.contains("# Chapter One"),
            "page 1 heading must be markdown-formatted, got: {:?}",
            p1.content,
        );
        assert!(
            p2.content.contains("# Chapter Two"),
            "page 2 heading must be markdown-formatted, got: {:?}",
            p2.content,
        );
    }

    /// List items must render as `- item` in markdown output, not bare text.
    #[cfg(any(
        feature = "ocr",
        feature = "office",
        feature = "pdf",
        feature = "paddle-ocr",
        feature = "xml",
        feature = "hwpx",
        feature = "quality",
        feature = "chunking"
    ))]
    #[test]
    fn page_content_markdown_list_items_formatted() {
        let mut doc = make_doc("docx");
        doc.push_element(InternalElement::text(ElementKind::ListStart { ordered: false }, "", 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::ListItem { ordered: false }, "First item", 1).with_page(1));
        doc.push_element(
            InternalElement::text(ElementKind::ListItem { ordered: false }, "Second item", 1).with_page(1),
        );
        doc.push_element(InternalElement::text(ElementKind::ListEnd, "", 0).with_page(1));

        let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Markdown);
        let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Markdown);

        let pages = result.pages.expect("pages must be populated");
        assert_eq!(pages.len(), 1);

        assert!(
            pages[0].content.contains("- First item") || pages[0].content.contains("* First item"),
            "page content must use markdown list syntax, got: {:?}",
            pages[0].content,
        );
        assert!(
            pages[0].content.contains("- Second item") || pages[0].content.contains("* Second item"),
            "page content must use markdown list syntax for all items, got: {:?}",
            pages[0].content,
        );
    }

    /// HTML output must render headings as `<h1>` tags, not bare text.
    #[cfg(any(
        feature = "ocr",
        feature = "office",
        feature = "pdf",
        feature = "paddle-ocr",
        feature = "xml",
        feature = "hwpx",
        feature = "quality",
        feature = "chunking"
    ))]
    #[test]
    fn page_content_html_format_renders_headings() {
        let mut doc = make_doc("docx");
        doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Title", 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body.", 0).with_page(1));

        let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Html);
        let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Html);

        let pages = result.pages.expect("pages must be populated");
        assert_eq!(pages.len(), 1);

        assert!(
            pages[0].content.contains("<h1"),
            "html-format page content must contain heading markup, got: {:?}",
            pages[0].content,
        );
        assert!(
            !pages[0].content.trim_start().starts_with("Title\n"),
            "html-format page content must not be bare plain text, got: {:?}",
            pages[0].content,
        );
    }

    /// Prebuilt pages whose page_number has no matching page-tagged elements must
    /// be returned unchanged. This is the normal path for native PDF extraction,
    /// OCR on images, and Excel/PPTX where the extractor sets prebuilt_pages but
    /// does not attach page numbers to individual InternalElements.
    #[test]
    fn page_content_prebuilt_pages_no_page_elements_unchanged() {
        let mut doc = make_doc("pdf");
        doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Title", 0));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body.", 0));
        doc.prebuilt_pages = Some(vec![crate::types::page::PageContent {
            page_number: 1,
            content: "Native PDF page content.".to_string(),
            tables: vec![],
            image_indices: vec![],
            hierarchy: None,
            is_blank: None,
            layout_regions: None,
            speaker_notes: None,
            section_name: None,
            sheet_name: None,
        }]);

        let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Markdown);
        let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Markdown);

        let pages = result.pages.expect("pages must be populated");
        assert_eq!(pages.len(), 1);
        assert_eq!(
            pages[0].content, "Native PDF page content.",
            "prebuilt content must not be overwritten when no elements are page-tagged, got: {:?}",
            pages[0].content,
        );
    }

    /// A page whose page_number appears in prebuilt_pages but has no matching
    /// page-tagged elements must keep its original content unchanged. This covers the
    /// per-page early-return branch inside apply_page_content_format.
    #[cfg(any(
        feature = "ocr",
        feature = "office",
        feature = "pdf",
        feature = "paddle-ocr",
        feature = "xml",
        feature = "hwpx",
        feature = "quality",
        feature = "chunking"
    ))]
    #[test]
    fn page_content_page_without_matching_elements_unchanged() {
        let mut doc = make_doc("docx");
        doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Chapter", 0).with_page(1));
        doc.prebuilt_pages = Some(vec![
            crate::types::page::PageContent {
                page_number: 1,
                content: "Page 1 plain".to_string(),
                tables: vec![],
                image_indices: vec![],
                hierarchy: None,
                is_blank: None,
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
            },
            crate::types::page::PageContent {
                page_number: 2,
                content: "Page 2 native content.".to_string(),
                tables: vec![],
                image_indices: vec![],
                hierarchy: None,
                is_blank: None,
                layout_regions: None,
                speaker_notes: None,
                section_name: None,
                sheet_name: None,
            },
        ]);

        let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Markdown);
        let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Markdown);

        let pages = result.pages.expect("pages must be populated");
        assert_eq!(pages.len(), 2);

        let p2 = pages.iter().find(|p| p.page_number == 2).expect("page 2");
        assert_eq!(
            p2.content, "Page 2 native content.",
            "page with no matching elements must keep its original content, got: {:?}",
            p2.content,
        );
        let p1 = pages.iter().find(|p| p.page_number == 1).expect("page 1");
        assert!(
            p1.content.contains("# Chapter"),
            "page 1 must still be markdown-formatted, got: {:?}",
            p1.content,
        );
    }

    /// OutputFormat::Json: result.content is rendered JSON but pages keep raw extracted
    /// text. The asymmetry is intentional — splitting JSON into per-page sub-objects
    /// would produce malformed fragments. The comment in apply_page_content_format
    /// explains the rationale; this test locks the observable contract.
    #[cfg(any(
        feature = "ocr",
        feature = "office",
        feature = "pdf",
        feature = "paddle-ocr",
        feature = "xml",
        feature = "hwpx",
        feature = "quality",
        feature = "chunking"
    ))]
    #[test]
    fn page_content_json_format_pages_stay_raw() {
        let mut doc = make_doc("docx");
        doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Title", 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body.", 0).with_page(1));

        let raw = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Json);
        let result = crate::core::pipeline::apply_output_format(raw, crate::core::config::OutputFormat::Json);

        assert!(
            result.content.contains('"'),
            "json format must produce JSON-structured result.content, got: {:?}",
            result.content,
        );
        let pages = result
            .pages
            .expect("pages must be populated when elements have page numbers");
        assert_eq!(pages.len(), 1);
        assert!(
            !pages[0].content.starts_with('{'),
            "page content must not be JSON-structured, got: {:?}",
            pages[0].content,
        );
        assert!(
            pages[0].content.contains("Title") || pages[0].content.contains("Body"),
            "page content must contain raw extracted text, got: {:?}",
            pages[0].content,
        );
    }
}
