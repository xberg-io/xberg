//! Derivation pipeline: converts `InternalDocument` → `DocumentStructure` + `ExtractionResult`.
//!
//! This module bridges the internal flat document representation produced by extractors
//! and the public-facing types consumed by callers. It handles:
//!
//! - **Relationship resolution**: `RelationshipTarget::Key` → `RelationshipTarget::Index`
//! - **Tree reconstruction**: Flat elements → hierarchical `DocumentStructure`
//! - **Content string derivation**: Concatenation of text-carrying elements
//! - **ExtractionResult assembly**: Combining all outputs into the final result

use std::borrow::Cow;
use std::sync::Arc;

use ahash::AHashMap;

use crate::types::document_structure::{
    DocumentNode, DocumentRelationship, DocumentStructure, GridCell, NodeContent, NodeId, NodeIndex, TableGrid,
};
use crate::types::extraction::ExtractionResult;
use crate::types::internal::{ElementKind, InternalDocument, InternalElement, RelationshipTarget};
use crate::types::ocr_elements::{OcrConfidence, OcrElement};
use crate::types::page::PageContent;
use crate::types::tables::Table;

// ============================================================================
// 1. Relationship Resolution
// ============================================================================

/// Resolve `RelationshipTarget::Key` entries to `RelationshipTarget::Index`.
///
/// Builds an anchor index from elements with non-`None` anchors, then resolves
/// each key-based relationship target. Unresolvable keys are logged and skipped
/// (the relationship is left as `Key` — it will be excluded from the final
/// `DocumentStructure` relationships).
pub fn resolve_relationships(doc: &mut InternalDocument) {
    // Build anchor → element index map (first element with a given anchor wins).
    // Skip FootnoteRef elements so that refs resolve to definitions, not to themselves.
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

// ============================================================================
// 2. Document Structure Derivation
// ============================================================================

/// Derive a hierarchical `DocumentStructure` from the flat internal document.
///
/// Calls `resolve_relationships` first to resolve any key-based relationship targets,
/// then builds the tree.
///
/// # Algorithm
///
/// 1. Walk elements in reading order, maintaining a stack of `(depth, NodeIndex)`.
/// 2. Container start markers (`ListStart`, `QuoteStart`, `GroupStart`) push
///    onto the stack; their matching end markers pop.
/// 3. Headings pop the stack to a shallower depth, then create a `Group` node
///    with a `Heading` child and push the group.
/// 4. All other elements are parented under the current stack top.
/// 5. Resolved relationships are mapped from element indices to node indices.
pub fn derive_document_structure(doc: &mut InternalDocument) -> DocumentStructure {
    resolve_relationships(doc);
    derive_document_structure_inner(doc)
}

/// Inner implementation that assumes relationships are already resolved.
///
/// Takes `&mut` so it can move data out of elements via `std::mem::take`,
/// avoiding clones. Callers that still need `elem.text` (build_pages,
/// build_ocr_elements) must run before this function.
fn derive_document_structure_inner(doc: &mut InternalDocument) -> DocumentStructure {
    let mut ds = DocumentStructure::with_capacity(doc.elements.len());
    ds.source_format = Some(doc.source_format.to_string());

    // Stack: (depth, NodeIndex) — depth is the element depth that "owns" this level
    let mut stack: Vec<(u16, NodeIndex)> = Vec::new();

    // Map element index → node index (for relationship mapping).
    // Not every element produces a node (end markers, FootnoteRef are skipped).
    let mut elem_to_node: Vec<Option<NodeIndex>> = vec![None; doc.elements.len()];

    // Track which elements have been consumed by pairing (e.g. DefinitionDescription paired with preceding Term)
    let mut consumed: Vec<bool> = vec![false; doc.elements.len()];

    // Pre-compute definition term/description pairings:
    // When a DefinitionTerm is immediately followed by a DefinitionDescription, mark the description as consumed.
    let mut def_pairs: AHashMap<usize, usize> = AHashMap::new(); // term_idx -> desc_idx
    for i in 0..doc.elements.len().saturating_sub(1) {
        if matches!(doc.elements[i].kind, ElementKind::DefinitionTerm)
            && matches!(doc.elements[i + 1].kind, ElementKind::DefinitionDescription)
        {
            def_pairs.insert(i, i + 1);
            consumed[i + 1] = true;
        }
    }

    for elem_idx in 0..doc.elements.len() {
        // Skip elements consumed by pairing
        if consumed[elem_idx] {
            continue;
        }
        // Skip container end markers — they just pop the stack
        match doc.elements[elem_idx].kind {
            ElementKind::ListEnd | ElementKind::QuoteEnd | ElementKind::GroupEnd => {
                // Pop matching container from stack, but only if the top matches
                if let Some((_, top_idx)) = stack.last() {
                    let top_content = &ds.nodes[top_idx.0 as usize].content;
                    if matches!(
                        (&doc.elements[elem_idx].kind, top_content),
                        (ElementKind::ListEnd, NodeContent::List { .. })
                            | (ElementKind::QuoteEnd, NodeContent::Quote)
                            | (ElementKind::GroupEnd, NodeContent::Group { .. })
                    ) {
                        stack.pop();
                    }
                    // If it doesn't match, skip the end marker
                }
                continue;
            }
            ElementKind::FootnoteRef => {
                // Footnote refs are represented as annotations, not separate nodes
                continue;
            }
            _ => {}
        }

        let elem = &doc.elements[elem_idx];

        // Container start markers
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
                _ => unreachable!(),
            };
            let node_idx = push_node(&mut ds, &stack, content, elem, elem_idx as u32);
            elem_to_node[elem_idx] = Some(node_idx);
            stack.push((elem.depth, node_idx));
            continue;
        }

        // Headings create a Group + Heading child and push the group
        if let ElementKind::Heading { level } = elem.kind {
            // Pop stack until we find a shallower depth
            pop_stack_to_depth(&mut stack, elem.depth);

            // Take text and annotations — pages/OCR have already consumed what they need
            let text = std::mem::take(&mut doc.elements[elem_idx].text);
            let annotations = std::mem::take(&mut doc.elements[elem_idx].annotations);
            let elem = &doc.elements[elem_idx];

            let group_content = NodeContent::Group {
                label: None,
                heading_level: Some(level),
                heading_text: Some(text.clone()),
            };

            let group_idx = push_node(&mut ds, &stack, group_content, elem, elem_idx as u32);

            // Create Heading child node inside the group
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

            // The element maps to the group node (heading is a child detail)
            elem_to_node[elem_idx] = Some(group_idx);
            stack.push((elem.depth, group_idx));
            continue;
        }

        // DefinitionTerm with a paired DefinitionDescription: create a combined DefinitionItem
        // wrapped in a DefinitionList container.
        if let Some(&desc_idx) = def_pairs.get(&elem_idx) {
            pop_stack_to_depth(&mut stack, elem.depth);

            // Ensure a DefinitionList container is on the stack
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

        // Unpaired DefinitionTerm or DefinitionDescription: wrap in DefinitionList too
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

        // Close any open DefinitionList when a non-definition element is encountered
        if stack
            .last()
            .is_some_and(|(_, idx)| matches!(ds.nodes[idx.0 as usize].content, NodeContent::DefinitionList))
        {
            stack.pop();
        }

        // All other elements
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

    // Convert resolved relationships to DocumentRelationship
    for rel in &doc.relationships {
        if let RelationshipTarget::Index(target_elem_idx) = rel.target {
            // When source elem_to_node is None (e.g. FootnoteRef was skipped),
            // walk backwards to find the nearest mapped element as the source.
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

    ds
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
        // Intentional AHashMap → HashMap conversion: DocumentNode.attributes uses
        // std::collections::HashMap for utoipa/OpenAPI schema compatibility.
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
        // Container starts are handled separately above; these shouldn't be reached
        // but we handle them defensively.
        ElementKind::ListStart { ordered } => NodeContent::List { ordered },
        ElementKind::QuoteStart => NodeContent::Quote,
        ElementKind::GroupStart => NodeContent::Group {
            label: None,
            heading_level: None,
            heading_text: None,
        },
        // These should have been filtered out before calling this function
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

// ============================================================================
// 4. ExtractionResult Assembly
// ============================================================================

/// Derive a complete `ExtractionResult` from an `InternalDocument`.
///
/// This is the main entry point for the derivation pipeline. It:
/// 1. Resolves relationships (needed by renderers for footnotes)
/// 2. Renders plain-text content (for post-processors)
/// 3. Pre-renders formatted content if output_format != Plain
/// 4. Groups elements by page into `PageContent`
/// 5. Extracts OCR elements for backward compatibility
/// 6. Optionally derives `DocumentStructure` (assumes relationships resolved)
/// 7. Assembles the final `ExtractionResult`
pub fn derive_extraction_result(
    mut doc: InternalDocument,
    include_document_structure: bool,
    output_format: crate::core::config::OutputFormat,
) -> ExtractionResult {
    tracing::debug!(
        element_count = doc.elements.len(),
        source_format = %doc.source_format,
        include_document_structure,
        "derivation pipeline starting"
    );
    // 1. Resolve relationships first — renderers need resolved targets for footnotes.
    resolve_relationships(&mut doc);

    // 2. Always derive plain-text content (post-processors operate on this).
    let content = crate::rendering::render_plain(&doc);

    // Use the explicit mime_type from the doc if it was set, otherwise derive from source_format
    let mime_type = if doc.mime_type != "application/octet-stream" {
        std::mem::take(&mut doc.mime_type)
    } else {
        Cow::Borrowed(source_format_to_mime_type(&doc.source_format))
    };

    // 3. Pre-render formatted content if a non-plain output format is requested.
    //    This runs while the InternalDocument still owns its element data.
    //
    //    If the extractor already produced high-quality formatted output (stored in
    //    `pre_rendered_content`) and the requested format matches what the extractor
    //    produced (`metadata.output_format`), use it directly to avoid the lossy
    //    InternalDocument → renderer round-trip.
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

    // 4. Build pages and OCR elements BEFORE document structure derivation,
    //    so that derive_document_structure_inner can move (take) elem.text
    //    and elem.annotations instead of cloning them.
    //
    //    Prefer pre-built pages from the extractor (e.g. PDF native page tracking)
    //    over reconstructing from element-level page numbers.
    let pages = doc.prebuilt_pages.take().or_else(|| build_pages(&doc));
    let ocr_elements = build_ocr_elements(&doc);

    // 5. Optionally derive DocumentStructure (relationships already resolved above).
    let document = if include_document_structure {
        Some(derive_document_structure_inner(&mut doc))
    } else {
        None
    };

    // Convert images
    let images = if doc.images.is_empty() { None } else { Some(doc.images) };

    // Transfer URIs, deduplicating by (url, kind) pair
    let uris = if doc.uris.is_empty() {
        None
    } else {
        let mut seen = ahash::AHashSet::with_capacity(doc.uris.len());
        doc.uris.retain(|uri| seen.insert((uri.url.clone(), uri.kind)));
        Some(doc.uris)
    };

    // Extract code intelligence from FormatMetadata::Code if present.
    #[cfg(feature = "tree-sitter")]
    let code_intelligence = match &doc.metadata.format {
        Some(crate::types::metadata::FormatMetadata::Code(process_result)) => Some(process_result.clone()),
        _ => None,
    };

    tracing::debug!(
        content_length = content.len(),
        has_document_structure = document.is_some(),
        "derivation pipeline complete"
    );
    ExtractionResult {
        content,
        mime_type,
        metadata: doc.metadata,
        tables: doc.tables,
        images,
        pages,
        ocr_elements,
        document,
        processing_warnings: std::mem::take(&mut doc.processing_warnings),
        annotations: std::mem::take(&mut doc.annotations),
        children: std::mem::take(&mut doc.children),
        uris,
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
    // Group elements by page number
    let mut page_map: std::collections::BTreeMap<u32, Vec<&InternalElement>> = std::collections::BTreeMap::new();

    for elem in &doc.elements {
        if let Some(page) = elem.page {
            page_map.entry(page).or_default().push(elem);
        }
    }

    if page_map.is_empty() {
        return None;
    }

    // Pre-wrap tables and images in Arc once; clone the Arc (cheap) per page reference.
    let arc_tables: Vec<Arc<Table>> = doc.tables.iter().map(|t| Arc::new(t.clone())).collect();
    let arc_images: Vec<Arc<crate::types::ExtractedImage>> = doc.images.iter().map(|i| Arc::new(i.clone())).collect();

    let pages: Vec<PageContent> = page_map
        .into_iter()
        .map(|(page_num, elems)| {
            let mut content = String::new();
            let mut tables = Vec::new();
            let mut images = Vec::new();
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
                    ElementKind::Image { image_index } => {
                        if let Some(arc_image) = arc_images.get(image_index as usize) {
                            images.push(Arc::clone(arc_image));
                        }
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
                page_number: page_num as usize,
                content,
                tables,
                images,
                hierarchy: None,
                is_blank: None,
            }
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
                // Default confidence: 0.0 means "unknown, not measured" (not zero confidence).
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
                    // Default to page 1 when page info is absent (OCR always has at least one page).
                    page_number: elem.page.unwrap_or(1) as usize,
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

// ============================================================================
// Tests
// ============================================================================

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

    // -----------------------------------------------------------------------
    // Test 1: Simple flat document → correct tree
    // -----------------------------------------------------------------------

    #[test]
    fn test_flat_document_produces_flat_tree() {
        let mut doc = make_doc("markdown");
        doc.push_element(InternalElement::text(ElementKind::Title, "My Title", 0));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "First paragraph.", 0));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Second paragraph.", 0));

        let ds = derive_document_structure(&mut doc);
        assert!(ds.validate().is_ok(), "validation: {:?}", ds.validate());
        assert_eq!(ds.len(), 3);

        // All should be root-level
        let roots: Vec<_> = ds.body_roots().collect();
        assert_eq!(roots.len(), 3);

        // First node is Title
        match &roots[0].1.content {
            NodeContent::Title { text } => assert_eq!(text, "My Title"),
            other => panic!("Expected Title, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 2: Heading-based nesting → correct Group/Heading parent-child
    // -----------------------------------------------------------------------

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

        let ds = derive_document_structure(&mut doc);
        assert!(ds.validate().is_ok(), "validation: {:?}", ds.validate());

        // Root should have exactly 1 Group for H1
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

        // H1 group should have children: Heading, Paragraph, H2 Group
        assert_eq!(h1_group.children.len(), 3);

        // First child is the Heading node
        let heading_node = &ds.nodes[h1_group.children[0].0 as usize];
        assert!(matches!(&heading_node.content, NodeContent::Heading { level: 1, .. }));

        // Second child is the paragraph
        let para_node = &ds.nodes[h1_group.children[1].0 as usize];
        assert!(matches!(&para_node.content, NodeContent::Paragraph { .. }));

        // Third child is the H2 Group
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

        // H2 Group should have: Heading + Paragraph
        assert_eq!(h2_group.children.len(), 2);
    }

    // -----------------------------------------------------------------------
    // Test 3: Relationship resolution (footnote key matching)
    // -----------------------------------------------------------------------

    #[test]
    fn test_relationship_resolution() {
        let mut doc = make_doc("markdown");

        // Element 0: paragraph with footnote ref
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "See note [^fn1].", 0));

        // Element 1: footnote ref marker
        doc.push_element(InternalElement::text(ElementKind::FootnoteRef, "fn1", 0).with_anchor("fn1"));

        // Element 2: footnote definition
        doc.push_element(
            InternalElement::text(ElementKind::FootnoteDefinition, "This is the footnote.", 0).with_anchor("fn1"),
        );

        // Relationship: element 1 → key "fn1"
        doc.push_relationship(Relationship {
            source: 1,
            target: RelationshipTarget::Key("fn1".to_string()),
            kind: RelationshipKind::FootnoteReference,
        });

        resolve_relationships(&mut doc);

        // Should be resolved to Index(2) — the FootnoteDefinition, not the FootnoteRef itself
        // (FootnoteRef elements are excluded from the anchor map)
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

        // Should remain as Key (unresolvable)
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

        let ds = derive_document_structure(&mut doc);
        assert!(ds.validate().is_ok());
        assert_eq!(ds.relationships.len(), 1);
        assert_eq!(ds.relationships[0].kind, RelationshipKind::FootnoteReference);
    }

    // -----------------------------------------------------------------------
    // Container markers
    // -----------------------------------------------------------------------

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

        let ds = derive_document_structure(&mut doc);
        assert!(ds.validate().is_ok(), "validation: {:?}", ds.validate());

        // Root: List container
        let roots: Vec<_> = ds.body_roots().collect();
        assert_eq!(roots.len(), 1);
        assert!(matches!(&roots[0].1.content, NodeContent::List { ordered: false }));

        // List has 2 children
        assert_eq!(ds.nodes[roots[0].0.0 as usize].children.len(), 2);
    }

    // -----------------------------------------------------------------------
    // derive_extraction_result
    // -----------------------------------------------------------------------

    #[test]
    fn test_derive_extraction_result_basic() {
        let mut doc = make_doc("markdown");
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Hello world.", 0));

        let result = derive_extraction_result(doc, false, crate::core::config::OutputFormat::Plain);
        assert_eq!(result.content, "Hello world.");
        assert_eq!(result.mime_type, "text/markdown");
        assert!(result.document.is_none());
    }

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

    #[test]
    fn test_source_format_cow_owned_propagates() {
        // Regression test for #622: Cow::Owned variant must not fail type inference
        // when deriving document structure. Previously used unstable Cow::as_str().
        let owned: std::borrow::Cow<'static, str> = std::borrow::Cow::Owned("epub".to_string());
        let mut doc = InternalDocument::new(owned);
        doc.push_element(InternalElement::text(ElementKind::Heading { level: 1 }, "Ch1", 0).with_page(1));
        doc.push_element(InternalElement::text(ElementKind::Paragraph, "Body.", 1).with_page(1));

        let result = derive_extraction_result(doc, true, crate::core::config::OutputFormat::Plain);
        let ds = result.document.unwrap();
        assert_eq!(ds.source_format.as_deref(), Some("epub"));
    }
}
