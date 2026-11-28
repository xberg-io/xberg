//! Structure tree traversal for extracting reading order.
//!
//! Implements pre-order traversal of structure trees to determine correct reading order.

use super::types::{ActualTextIndex, McidScope, StructChild, StructElem, StructTreeRoot, StructType};
use crate::error::Error;
use std::sync::Arc;

/// Role this content plays inside a List (PDF spec §14.8.4.3).
///
/// MCRs nested under list-context ancestors carry their role so the
/// markdown converter can emit `- item` / `1. item` correctly even when
/// the immediate parent of the MCR is a Span or P (the common Word /
/// Acrobat output shape `LI → LBody → Span → MCR`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListRole {
    /// Inside an LI (list item) but not under Lbl/LBody yet (or LI
    /// itself holds the MCR directly).
    LI,
    /// Inside the Lbl (label) sub-element of an LI — the bullet/number.
    Lbl,
    /// Inside the LBody (body) sub-element of an LI — the item text.
    LBody,
}

/// Represents an ordered content item extracted from structure tree.
#[derive(Debug, Clone)]
pub struct OrderedContent {
    /// Page number
    pub page: u32,

    /// Marked Content ID (None for word break markers)
    pub mcid: Option<u32>,

    /// Structure type (for semantic information)
    pub struct_type: String,

    /// Pre-parsed structure type for efficient access
    pub parsed_type: StructType,

    /// Is this a heading?
    ///
    /// True when the MCR is nested under any heading ancestor (H, H1..H6),
    /// not just when the immediate parent is a heading. Word-generated
    /// tagged PDFs commonly wrap heading text in `H1 → Span → MCR`, where
    /// the heading semantic must still be recovered.
    pub is_heading: bool,

    /// If the MCR is nested under any heading ancestor, the level of that
    /// ancestor (H1 → 1, …, H6 → 6, generic H → 1). None otherwise.
    pub heading_level: Option<u8>,

    /// Role inside a list, when nested under any L/LI ancestor. None if
    /// this MCR has no list ancestor.
    pub list_role: Option<ListRole>,

    /// Is this a block-level element?
    pub is_block: bool,

    /// Is this a word break marker (WB element)?
    ///
    /// When true, a space should be inserted at this position during
    /// text assembly. This supports CJK text that uses WB elements
    /// to mark word boundaries.
    pub is_word_break: bool,

    /// Identifier of the nearest block-level ancestor (P, H*, LI, Sect,
    /// Div, Art, …) — increments each time the traversal enters a new
    /// block element. Two MCRs that share a `block_id` belong to the
    /// same logical paragraph; a change in `block_id` between adjacent
    /// MCRs is the structure-tree-authoritative paragraph boundary
    /// (PDF spec ISO 32000-1:2008 §14.8.4). The markdown / HTML
    /// converters rely on this to split paragraphs when a tagged PDF's
    /// inter-paragraph gap is too small for the geometric heuristic.
    /// 0 means "no enclosing block element seen" (root-level Span).
    pub block_id: u32,

    /// True when this MCR is nested under a table grouping element
    /// (Table / THead / TBody / TFoot / TR / TH / TD). The plain-text
    /// assembler separates consecutive table rows with a single newline
    /// rather than the geometric multi-line gap.
    pub in_table: bool,

    /// True when this MCR is nested under a `Code` element. Preformatted —
    /// its line breaks are significant and converters must not reflow them.
    pub preformatted: bool,

    /// Identifier of the nearest `Sect` / `Art` / `Part` grouping-element
    /// ancestor (ISO 32000-1:2008 §14.8.4.2), or `None` at the document level.
    /// Two MCRs that share a `section_id` belong to the same logical section —
    /// the spec-authoritative, page-independent grouping that
    /// `extract_structured` surfaces as a per-region section index, so chapters
    /// stay grouped across pages without geometric guessing.
    pub section_id: Option<u32>,

    /// Actual text replacement from /ActualText (optional)
    /// Per PDF spec Section 14.9.4, when present this replaces all
    /// descendant content with the specified text.
    pub actual_text: Option<String>,

    /// Content-stream scope of the MCID (ISO 32000-1:2008 §14.7.4.3).
    ///
    /// `McidScope::Page(page)` for MCIDs drawn directly by the page's
    /// content stream (the dominant case). `Form(_)` / `Pattern(_)`
    /// when the structure tree's MCR carried a `/Stm` reference into
    /// a Form XObject or Tiling Pattern, so the ActualText applier can
    /// look up `(scope, mcid)` without colliding with same-mcid keys
    /// in other namespaces. None when this `OrderedContent` is a word-
    /// break marker (no MCID).
    pub mcid_scope: Option<McidScope>,
}

/// Inheritable context propagated down the structure tree during traversal.
///
/// Tracks the nearest heading and list ancestors so deeply nested MCRs
/// (`H1 → Span → MCR`, `LI → LBody → Span → MCR`) carry the correct
/// semantic role on the resulting `OrderedContent`. Without this, the
/// markdown converter saw the immediate parent (Span / P) and lost the
/// heading / list-item information altogether.
#[derive(Debug, Clone, Copy, Default)]
struct InheritedContext {
    heading_level: Option<u8>,
    list_role: Option<ListRole>,
    /// Identifier of the nearest block-level ancestor — see
    /// `OrderedContent::block_id`.
    block_id: u32,
    /// Identifier of the nearest `Sect`/`Art`/`Part` ancestor — see
    /// `OrderedContent::section_id`.
    section_id: Option<u32>,
    /// True when the MCR is nested under a table grouping element
    /// (Table / THead / TBody / TFoot / TR / TH / TD). Used by the
    /// plain-text assembler to separate table rows with a single newline
    /// instead of the geometric multi-line gap (ISO 32000-1 §14.8.4.3.4:
    /// table rows are stacked block-level rows, not free-leading paragraphs).
    in_table: bool,
    /// True when the MCR is nested under a `Code` element — preformatted
    /// content whose line breaks are significant. The converters must NOT
    /// reflow such lines into a single paragraph.
    preformatted: bool,
}

impl InheritedContext {
    /// Returns true when `t` is a block-level element that should bump
    /// the paragraph counter on entry. Spans, links, and similar inline
    /// elements do not.
    fn is_paragraph_block(t: &StructType) -> bool {
        matches!(
            t,
            StructType::P
                | StructType::H
                | StructType::H1
                | StructType::H2
                | StructType::H3
                | StructType::H4
                | StructType::H5
                | StructType::H6
                | StructType::LI
                | StructType::Lbl
                | StructType::LBody
                | StructType::Sect
                | StructType::Div
                | StructType::Art
                | StructType::Part
                | StructType::Note
                | StructType::Reference
                | StructType::BibEntry
                | StructType::Code
                | StructType::TR
                | StructType::TH
                | StructType::TD
        )
    }

    fn descend(self, child: &StructType, counter: &mut u32) -> Self {
        let heading_level = match child {
            StructType::H1 => Some(1),
            StructType::H2 => Some(2),
            StructType::H3 => Some(3),
            StructType::H4 => Some(4),
            StructType::H5 => Some(5),
            StructType::H6 => Some(6),
            // Generic /H carries no level on its own. ~keep
            StructType::H => Some(self.heading_level.unwrap_or(1)),
            _ => self.heading_level,
        };
        let list_role = match child {
            StructType::Lbl => Some(ListRole::Lbl),
            StructType::LBody => Some(ListRole::LBody),
            StructType::LI => Some(self.list_role.unwrap_or(ListRole::LI)),
            // L starts list context but doesn't itself hold MCRs as items;
            // its LI children promote to ListRole::LI on descent. ~keep
            StructType::L => self.list_role,
            _ => self.list_role,
        };
        let block_id = if Self::is_paragraph_block(child) {
            *counter += 1;
            *counter
        } else {
            self.block_id
        };
        // A Sect/Art/Part opens a new logical section (§14.8.4.2); its own
        // block_id (just bumped above) becomes the section id its descendants
        // inherit. Other elements keep the enclosing section. ~keep
        let section_id = match child {
            StructType::Sect | StructType::Art | StructType::Part => Some(block_id),
            _ => self.section_id,
        };
        let in_table = self.in_table
            || matches!(
                child,
                StructType::Table
                    | StructType::THead
                    | StructType::TBody
                    | StructType::TFoot
                    | StructType::TR
                    | StructType::TH
                    | StructType::TD
            );
        let preformatted = self.preformatted || matches!(child, StructType::Code);
        Self {
            heading_level,
            list_role,
            block_id,
            section_id,
            in_table,
            preformatted,
        }
    }
}

/// Traverse the structure tree and extract ordered content for a specific page.
///
/// This performs a pre-order traversal of the structure tree, extracting
/// marked content references in document order.
///
/// # Arguments
/// * `struct_tree` - The structure tree root
/// * `page_num` - The page number to extract content for
///
/// # Returns
/// * Vector of ordered content items for the specified page
pub fn traverse_structure_tree(struct_tree: &StructTreeRoot, page_num: u32) -> Result<Vec<OrderedContent>, Error> {
    let mut result = Vec::new();
    let mut block_counter = 0u32;

    for root_elem in &struct_tree.root_elements {
        traverse_element(
            root_elem,
            page_num,
            InheritedContext::default(),
            &mut block_counter,
            &mut result,
        )?;
    }

    Ok(result)
}

/// Traverse the structure tree once and build content for ALL pages.
///
/// This is much more efficient than calling `traverse_structure_tree` once per page,
/// which would walk the entire tree N times. Instead, we walk the tree once and
/// collect content items into per-page buckets.
///
/// Returns a HashMap mapping page numbers to their ordered content items.
pub fn traverse_structure_tree_all_pages(
    struct_tree: &StructTreeRoot,
) -> std::collections::HashMap<u32, Vec<OrderedContent>> {
    let span = tracing::debug_span!(
        "xberg_native_pdf::structure_tree_traverse",
        root_count = struct_tree.root_elements.len(),
        pages = tracing::field::Empty
    );
    let _entered = span.enter();
    let mut result: std::collections::HashMap<u32, Vec<OrderedContent>> = std::collections::HashMap::new();

    let mut block_counter = 0u32;
    for root_elem in &struct_tree.root_elements {
        traverse_element_all_pages(root_elem, InheritedContext::default(), &mut block_counter, &mut result);
    }

    span.record("pages", result.len());
    result
}

/// Recursively traverse a structure element, collecting content for all pages.
///
/// `ctx` carries inherited semantics from heading and list ancestors so deeply
/// nested MCRs (e.g. `H1 → Span → MCR`, `LI → LBody → Span → MCR`) emit
/// content tagged with the right role, not just the immediate parent's role.
fn traverse_element_all_pages(
    elem: &StructElem,
    ctx: InheritedContext,
    block_counter: &mut u32,
    result: &mut std::collections::HashMap<u32, Vec<OrderedContent>>,
) {
    let struct_type_str = format!("{:?}", elem.struct_type);
    let parsed_type = elem.struct_type.clone();
    let descended = ctx.descend(&parsed_type, block_counter);
    let is_heading_inherited = descended.heading_level.is_some();
    let is_block = elem.struct_type.is_block();
    let is_word_break = elem.struct_type.is_word_break();

    // /ActualText is resolved separately via `build_actualtext_index`
    // — assemblers consult the index to position the replacement and to
    // suppress descendant MCIDs (per ISO 32000-1:2008 §14.9.4 the
    // replacement covers the entire subtree, but emitting it has to
    // respect the multi-page emit-once rule which a per-page traversal
    // cannot enforce). The traversal therefore continues to record
    // descendant MCIDs so the structure-order MCID list stays complete;
    // the assembler drops the suppressed ones at emit time. ~keep

    for child in &elem.children {
        match child {
            StructChild::MarkedContentRef {
                mcid,
                page,
                scope: mcid_scope,
            } => {
                result.entry(*page).or_default().push(OrderedContent {
                    page: *page,
                    mcid: Some(*mcid),
                    struct_type: struct_type_str.clone(),
                    parsed_type: parsed_type.clone(),
                    is_heading: is_heading_inherited,
                    heading_level: descended.heading_level,
                    list_role: descended.list_role,
                    is_block,
                    is_word_break: false,
                    block_id: descended.block_id,
                    section_id: descended.section_id,
                    in_table: descended.in_table,
                    preformatted: descended.preformatted,
                    actual_text: None,
                    mcid_scope: Some(mcid_scope.clone()),
                });
            }

            StructChild::StructElem(child_elem) => {
                // If parent is WB, emit word break markers before processing child ~keep
                if is_word_break {
                    let child_pages = collect_pages(child_elem);
                    for page in child_pages {
                        result.entry(page).or_default().push(OrderedContent {
                            page,
                            mcid: None,
                            struct_type: struct_type_str.clone(),
                            parsed_type: parsed_type.clone(),
                            is_heading: false,
                            heading_level: None,
                            list_role: descended.list_role,
                            is_block: false,
                            is_word_break: true,
                            block_id: descended.block_id,
                            section_id: descended.section_id,
                            in_table: descended.in_table,
                            preformatted: descended.preformatted,
                            actual_text: None,
                            mcid_scope: None,
                        });
                    }
                }
                traverse_element_all_pages(child_elem, descended, block_counter, result);
            }

            StructChild::ObjectRef(_obj_num, _gen) => {
                tracing::warn!(object_id = _obj_num, generation = _gen, "skipping unresolved ObjectRef");
            }
        }
    }
}

/// Collect all page numbers that a structure element has content on.
fn collect_pages(elem: &StructElem) -> Vec<u32> {
    let mut pages = Vec::new();
    collect_pages_recursive(elem, &mut pages);
    pages.sort_unstable();
    pages.dedup();
    pages
}

fn collect_pages_recursive(elem: &StructElem, pages: &mut Vec<u32>) {
    if let Some(page) = elem.page {
        pages.push(page);
    }
    for child in &elem.children {
        match child {
            StructChild::MarkedContentRef { page, .. } => {
                pages.push(*page);
            }
            StructChild::StructElem(child_elem) => {
                collect_pages_recursive(child_elem, pages);
            }
            _ => {}
        }
    }
}

/// Recursively traverse a structure element.
///
/// Performs pre-order traversal:
/// 1. Process current element's marked content (if on target page)
/// 2. Recursively process children in order
/// 3. Handle WB (word break) elements by emitting markers
fn traverse_element(
    elem: &StructElem,
    target_page: u32,
    ctx: InheritedContext,
    block_counter: &mut u32,
    result: &mut Vec<OrderedContent>,
) -> Result<(), Error> {
    let struct_type_str = format!("{:?}", elem.struct_type);
    let parsed_type = elem.struct_type.clone();
    let descended = ctx.descend(&parsed_type, block_counter);
    let is_heading_inherited = descended.heading_level.is_some();
    let is_block = elem.struct_type.is_block();
    let is_word_break = elem.struct_type.is_word_break();

    // /ActualText is resolved separately via `build_actualtext_index`;
    // see `traverse_element_all_pages` for the rationale. ~keep

    if is_word_break {
        result.push(OrderedContent {
            page: target_page,
            mcid: None,
            struct_type: struct_type_str.clone(),
            parsed_type: parsed_type.clone(),
            is_heading: false,
            heading_level: None,
            list_role: descended.list_role,
            is_block: false,
            is_word_break: true,
            block_id: descended.block_id,
            section_id: descended.section_id,
            in_table: descended.in_table,
            preformatted: descended.preformatted,
            actual_text: None,
            mcid_scope: None,
        });
        // WB elements typically have no children, but process any just in case ~keep
    }

    for child in &elem.children {
        match child {
            StructChild::MarkedContentRef {
                mcid,
                page,
                scope: mcid_scope,
            } => {
                if *page == target_page {
                    result.push(OrderedContent {
                        page: *page,
                        mcid: Some(*mcid),
                        struct_type: struct_type_str.clone(),
                        parsed_type: parsed_type.clone(),
                        is_heading: is_heading_inherited,
                        heading_level: descended.heading_level,
                        list_role: descended.list_role,
                        is_block,
                        is_word_break: false,
                        block_id: descended.block_id,
                        section_id: descended.section_id,
                        in_table: descended.in_table,
                        preformatted: descended.preformatted,
                        actual_text: None,
                        mcid_scope: Some(mcid_scope.clone()),
                    });
                }
            }

            StructChild::StructElem(child_elem) => {
                traverse_element(child_elem, target_page, descended, block_counter, result)?;
            }

            StructChild::ObjectRef(_obj_num, _gen) => {
                // ObjectRef should be resolved at parse time (structure/parser.rs).
                // If we encounter one here, it means the reference couldn't be resolved. ~keep
                tracing::warn!(object_id = _obj_num, generation = _gen, "skipping unresolved ObjectRef");
            }
        }
    }

    Ok(())
}

/// Check if a structure element has any content on the target page.
///
/// Used only by tests since per-element ActualText gating moved into
/// the [`ActualTextIndex`] (which records per-emission `first_page`).
#[cfg(test)]
fn has_content_on_page(elem: &StructElem, target_page: u32) -> bool {
    if elem.page == Some(target_page) {
        return true;
    }
    for child in &elem.children {
        match child {
            StructChild::MarkedContentRef { page, .. } => {
                if *page == target_page {
                    return true;
                }
            }
            StructChild::StructElem(child_elem) => {
                if has_content_on_page(child_elem, target_page) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Build an [`ActualTextIndex`] resolving every structure-tree
/// `/ActualText` declaration in a single pre-order traversal.
///
/// Per ISO 32000-1:2008 §14.9.4, a structure element may carry an
/// `/ActualText` entry that replaces all of its descendant content for
/// text-extraction purposes. The replacement scope is the bearing
/// element's subtree. When ActualText scopes nest, the inner
/// replacement wins for the `(page, mcid)` pairs the inner element
/// covers.
///
/// The returned index lets every extraction surface apply ActualText
/// consistently:
///   - `covered_mcids` lists `(page, mcid)` pairs whose raw glyph spans
///     must be suppressed.
///   - `mcid_to_actual_text` resolves the innermost replacement for
///     each covered `(page, mcid)` whose pair sits on the bearing
///     element's first page (or the inner scope's first page when
///     nested) — consumers iterate per-page MCIDs in structure-tree
///     order and emit `mcid_to_actual_text[(page, mcid)]` whenever the
///     value changes across consecutive covered MCIDs, giving
///     correct one-emission-per-replacement output.
///   - `suppress_only` is the rest: `(page, mcid)` pairs on a non-first
///     page of a multi-page subtree, where the replacement has already
///     fired on the bearing element's first page; the raw glyphs are
///     still suppressed but no second emission is produced.
///
/// Empty `/ActualText` strings and elements with no descendant MCID
/// contribute nothing.
pub fn build_actualtext_index(struct_tree: &StructTreeRoot) -> ActualTextIndex {
    let mut idx = ActualTextIndex::new();
    for root in &struct_tree.root_elements {
        walk_actualtext(root, None, &mut idx);
    }
    idx
}

/// One ActualText scope, threaded down the traversal so descendant
/// `(scope, mcid)` pairs know which scope to attribute them to.
#[derive(Clone)]
struct ActiveScope {
    /// Innermost active replacement text.
    text: Arc<str>,
    /// First page (in pre-order) on which a Page-scoped descendant
    /// MCID of this ActualText scope appears. The emit-once-across-
    /// pages rule applies *only* to Page-scoped descendants: a
    /// multi-page subtree emits once on `first_page` and `suppress_only`
    /// covers the rest. Form- and Pattern-scoped descendants live in
    /// their own per-stream namespace (ISO 32000-1:2008 §14.7.4.3); each
    /// one emits at its own anchor.
    ///
    /// `None` when the subtree has no Page-scoped MCR descendant — in
    /// which case the suppress-only fallback is irrelevant.
    first_page: Option<u32>,
}

/// Pre-order walker for [`build_actualtext_index`].
///
/// `inherited` carries the innermost active scope from our ancestors.
/// For each element bearing `/ActualText` we pre-scan our own subtree
/// to find the first Page-scoped page (so the across-pages emit-once
/// rule still works), then walk children with our scope active.
fn walk_actualtext(elem: &StructElem, inherited: Option<ActiveScope>, idx: &mut ActualTextIndex) {
    let own_text: Option<Arc<str>> = elem.actual_text.as_deref().filter(|s| !s.is_empty()).map(Arc::from);

    let active = if let Some(text) = own_text {
        // Pre-scan to find this scope's first Page-scoped descendant.
        // Subtrees with *only* Form/Pattern descendants get
        // `first_page = None` — the per-stream namespaces don't share
        // an emit-once rule (there is no "first page" for a Form
        // XObject's content stream from the structure tree's
        // perspective).
        //
        // When the subtree has no descendant MCR of any kind, drop
        // the scope: nothing to attach to. ~keep
        if has_any_mcr(elem) {
            Some(ActiveScope {
                text,
                first_page: first_page_in_subtree(elem),
            })
        } else {
            None
        }
    } else {
        None
    };

    // The active scope for our subtree: our own if any, else inherited.
    // Inner-wins: when our own scope exists, we override inherited for
    // every descendant. ~keep
    let scope = active.clone().or(inherited.clone());

    for child in &elem.children {
        match child {
            StructChild::MarkedContentRef {
                mcid,
                page,
                scope: mcid_scope,
            } => {
                if let Some(ref s) = scope {
                    let key = (mcid_scope.clone(), *mcid);
                    idx.covered_mcids.insert(key.clone());

                    // Emit-once rule:
                    // - Page-scoped: emit on the first page seen, suppress
                    //   on the others (cross-page subtrees).
                    // - Form/Pattern-scoped: emit on every covered key;
                    //   each form/pattern is its own namespace and the
                    //   StructElem covers one such stream at most for
                    //   each contained MCID. ~keep
                    let should_emit = match mcid_scope {
                        crate::structure::McidScope::Page(_) => s.first_page == Some(*page),
                        crate::structure::McidScope::Form(_) | crate::structure::McidScope::Pattern(_) => true,
                    };

                    if should_emit {
                        idx.mcid_to_actual_text.insert(key, s.text.clone());
                    } else {
                        // Non-first-page coverage for a multi-page Page
                        // subtree: suppress raw glyphs but do not
                        // re-emit; the replacement already fired on
                        // `s.first_page`. ~keep
                        idx.suppress_only.insert(key);
                    }
                }
            }
            StructChild::StructElem(child_elem) => {
                walk_actualtext(child_elem, scope.clone(), idx);
            }
            StructChild::ObjectRef(_, _) => {
                // Unresolved external reference — consistent with the
                // rest of the traversal, we skip. ~keep
            }
        }
    }
}

/// Find the first Page-scoped page (in pre-order) on which any
/// descendant MCR inside `elem`'s subtree sits. `None` when no
/// descendant is Page-scoped (the subtree may still have Form- or
/// Pattern-scoped descendants).
fn first_page_in_subtree(elem: &StructElem) -> Option<u32> {
    for child in &elem.children {
        match child {
            StructChild::MarkedContentRef { page, scope, .. } => {
                if matches!(scope, crate::structure::McidScope::Page(_)) {
                    return Some(*page);
                }
            }
            StructChild::StructElem(c) => {
                if let Some(p) = first_page_in_subtree(c) {
                    return Some(p);
                }
            }
            StructChild::ObjectRef(_, _) => {}
        }
    }
    None
}

/// Returns true when `elem`'s subtree contains at least one
/// `MarkedContentRef` of any scope.
fn has_any_mcr(elem: &StructElem) -> bool {
    for child in &elem.children {
        match child {
            StructChild::MarkedContentRef { .. } => return true,
            StructChild::StructElem(c) => {
                if has_any_mcr(c) {
                    return true;
                }
            }
            StructChild::ObjectRef(_, _) => {}
        }
    }
    false
}

/// Extract all marked content IDs in reading order for a page.
///
/// This is a simpler interface that just returns the MCIDs in order,
/// which can be used to reorder extracted text blocks.
///
/// Note: Word break (WB) markers are filtered out since they don't have MCIDs.
/// Use `traverse_structure_tree` directly if you need word break information.
///
/// # Arguments
/// * `struct_tree` - The structure tree root
/// * `page_num` - The page number
///
/// # Returns
/// * Vector of MCIDs in reading order
pub fn extract_reading_order(struct_tree: &StructTreeRoot, page_num: u32) -> Result<Vec<u32>, Error> {
    let ordered_content = traverse_structure_tree(struct_tree, page_num)?;
    Ok(ordered_content
        .into_iter()
        .filter_map(|c| c.mcid) // Filter out word break markers (mcid=None) ~keep
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::types::{StructChild, StructElem, StructType};

    #[test]
    fn test_simple_traversal() {
        // Create a simple structure tree:
        // Document
        //   ├─ P (MCID=0, page=0)
        //   └─ P (MCID=1, page=0) ~keep
        let mut root = StructElem::new(StructType::Document);

        let mut p1 = StructElem::new(StructType::P);
        p1.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        let mut p2 = StructElem::new(StructType::P);
        p2.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        root.add_child(StructChild::StructElem(Box::new(p1)));
        root.add_child(StructChild::StructElem(Box::new(p2)));

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        let order = extract_reading_order(&struct_tree, 0).unwrap();
        assert_eq!(order, vec![0, 1]);
    }

    #[test]
    fn test_page_filtering() {
        let mut root = StructElem::new(StructType::Document);

        let mut p1 = StructElem::new(StructType::P);
        p1.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        let mut p2 = StructElem::new(StructType::P);
        p2.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 1,
            scope: crate::structure::McidScope::Page(1),
        });

        root.add_child(StructChild::StructElem(Box::new(p1)));
        root.add_child(StructChild::StructElem(Box::new(p2)));

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        let order_page_0 = extract_reading_order(&struct_tree, 0).unwrap();
        assert_eq!(order_page_0, vec![0]);

        let order_page_1 = extract_reading_order(&struct_tree, 1).unwrap();
        assert_eq!(order_page_1, vec![1]);
    }

    #[test]
    fn test_nested_structure() {
        // Create nested structure:
        // Document
        //   └─ Sect
        //       ├─ H1 (MCID=0)
        //       └─ P (MCID=1) ~keep
        let mut root = StructElem::new(StructType::Document);

        let mut sect = StructElem::new(StructType::Sect);

        let mut h1 = StructElem::new(StructType::H1);
        h1.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        let mut p = StructElem::new(StructType::P);
        p.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        sect.add_child(StructChild::StructElem(Box::new(h1)));
        sect.add_child(StructChild::StructElem(Box::new(p)));

        root.add_child(StructChild::StructElem(Box::new(sect)));

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        let order = extract_reading_order(&struct_tree, 0).unwrap();
        assert_eq!(order, vec![0, 1]);
    }

    #[test]
    fn test_word_break_elements() {
        // Create structure with WB (word break) elements for CJK text:
        // P
        //   ├─ Span (MCID=0) - "你好"
        //   ├─ WB             - word boundary marker
        //   └─ Span (MCID=1) - "世界" ~keep
        let mut root = StructElem::new(StructType::P);

        let mut span1 = StructElem::new(StructType::Span);
        span1.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        let wb = StructElem::new(StructType::WB);

        let mut span2 = StructElem::new(StructType::Span);
        span2.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        root.add_child(StructChild::StructElem(Box::new(span1)));
        root.add_child(StructChild::StructElem(Box::new(wb)));
        root.add_child(StructChild::StructElem(Box::new(span2)));

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        let ordered = traverse_structure_tree(&struct_tree, 0).unwrap();
        assert_eq!(ordered.len(), 3); // MCID 0, WB, MCID 1 ~keep
        assert_eq!(ordered[0].mcid, Some(0));
        assert!(!ordered[0].is_word_break);
        assert_eq!(ordered[1].mcid, None); // WB has no MCID ~keep
        assert!(ordered[1].is_word_break);
        assert_eq!(ordered[2].mcid, Some(1));
        assert!(!ordered[2].is_word_break);

        let mcids = extract_reading_order(&struct_tree, 0).unwrap();
        assert_eq!(mcids, vec![0, 1]); // Only MCIDs, no WB ~keep
    }

    #[test]
    fn test_empty_tree() {
        let struct_tree = StructTreeRoot::new();
        let order = extract_reading_order(&struct_tree, 0).unwrap();
        assert!(order.is_empty());
    }

    #[test]
    fn test_empty_page() {
        let mut root = StructElem::new(StructType::Document);
        let mut p = StructElem::new(StructType::P);
        p.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        root.add_child(StructChild::StructElem(Box::new(p)));

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        // Page 5 has no content ~keep
        let order = extract_reading_order(&struct_tree, 5).unwrap();
        assert!(order.is_empty());
    }

    #[test]
    fn test_nested_heading_propagates_is_heading_to_inner_mcr() {
        // Word365 / docling pattern: H1 wraps Span which holds the actual MCR.
        // The MCR must inherit is_heading from its H1 ancestor, not from
        // the immediate Span parent (Span.is_heading() == false). ~keep
        let mut h1 = StructElem::new(StructType::H1);
        let mut span = StructElem::new(StructType::Span);
        span.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        h1.add_child(StructChild::StructElem(Box::new(span)));

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(h1);

        let ordered = traverse_structure_tree(&struct_tree, 0).unwrap();
        let heading_mcrs: Vec<_> = ordered.iter().filter(|c| c.is_heading).collect();
        assert_eq!(
            heading_mcrs.len(),
            1,
            "H1 → Span → MCR must propagate is_heading=true to the inner MCR"
        );
        assert_eq!(heading_mcrs[0].mcid, Some(0));
        let by_page = traverse_structure_tree_all_pages(&struct_tree);
        let heading_mcrs_all: Vec<_> = by_page.get(&0).unwrap().iter().filter(|c| c.is_heading).collect();
        assert_eq!(heading_mcrs_all.len(), 1);
    }

    #[test]
    fn test_nested_li_lbody_keeps_list_context() {
        // word365 / pdfa pattern: LI → LBody → MCR. LBody is the list-item
        // body and must be tagged as such; LI ancestry must be discoverable
        // when emitting markdown bullets. ~keep
        let mut li = StructElem::new(StructType::LI);
        let mut lbody = StructElem::new(StructType::LBody);
        lbody.add_child(StructChild::MarkedContentRef {
            mcid: 7,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        li.add_child(StructChild::StructElem(Box::new(lbody)));
        let mut l = StructElem::new(StructType::L);
        l.add_child(StructChild::StructElem(Box::new(li)));

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(l);

        let ordered = traverse_structure_tree(&struct_tree, 0).unwrap();
        let li_mcrs: Vec<_> = ordered
            .iter()
            .filter(|c| matches!(c.list_role, Some(crate::structure::ListRole::LBody)))
            .collect();
        assert_eq!(
            li_mcrs.len(),
            1,
            "LI → LBody → MCR must carry list_role=LBody on the inner MCR"
        );
    }

    /// D8b coverage — every standard heading level (H1..H6) propagates
    /// to a deeply nested MCR. Parametrised over all six levels in the
    /// same test to keep the lock-in compact.
    #[test]
    fn test_nested_heading_propagates_for_h1_through_h6() {
        let levels = [
            (StructType::H1, 1u8),
            (StructType::H2, 2),
            (StructType::H3, 3),
            (StructType::H4, 4),
            (StructType::H5, 5),
            (StructType::H6, 6),
        ];
        for (h_type, expected_level) in levels {
            // H? → Sect → Span → MCR (3-level nesting, reflects the
            // worst-case shape seen in word365_structure-class fixtures). ~keep
            let mut head = StructElem::new(h_type.clone());
            let mut sect = StructElem::new(StructType::Sect);
            let mut span = StructElem::new(StructType::Span);
            span.add_child(StructChild::MarkedContentRef {
                mcid: 42,
                page: 0,
                scope: crate::structure::McidScope::Page(0),
            });
            sect.add_child(StructChild::StructElem(Box::new(span)));
            head.add_child(StructChild::StructElem(Box::new(sect)));
            let mut tree = StructTreeRoot::new();
            tree.add_root_element(head);

            let ordered = traverse_structure_tree(&tree, 0).unwrap();
            let item = ordered.iter().find(|c| c.mcid == Some(42)).unwrap();
            assert!(
                item.is_heading,
                "H{} → Sect → Span → MCR must carry is_heading=true",
                expected_level
            );
            assert_eq!(
                item.heading_level,
                Some(expected_level),
                "H{} ancestor must propagate heading_level={}",
                expected_level,
                expected_level
            );
        }
    }

    /// D8b coverage — generic /H without an explicit level reports
    /// heading_level=Some(1) (the only sensible default per spec
    /// §14.8.4.2 when no surrounding heading exists).
    #[test]
    fn test_generic_h_without_level_defaults_to_h1() {
        let mut h = StructElem::new(StructType::H);
        let mut span = StructElem::new(StructType::Span);
        span.add_child(StructChild::MarkedContentRef {
            mcid: 9,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        h.add_child(StructChild::StructElem(Box::new(span)));
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(h);
        let ordered = traverse_structure_tree(&tree, 0).unwrap();
        let item = ordered.iter().find(|c| c.mcid == Some(9)).unwrap();
        assert!(item.is_heading);
        assert_eq!(item.heading_level, Some(1));
    }

    /// D8b negative case — adjacent heading and body MCRs at the same
    /// nesting level must keep their respective roles. A bug that
    /// "leaked" heading flag from a prior sibling into the next would
    /// flip every body paragraph after a heading into a heading.
    #[test]
    fn test_heading_role_does_not_bleed_into_following_paragraph() {
        let mut doc = StructElem::new(StructType::Document);
        let mut h1 = StructElem::new(StructType::H1);
        h1.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut p = StructElem::new(StructType::P);
        p.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        doc.add_child(StructChild::StructElem(Box::new(h1)));
        doc.add_child(StructChild::StructElem(Box::new(p)));
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(doc);

        let ordered = traverse_structure_tree(&tree, 0).unwrap();
        let h_item = ordered.iter().find(|c| c.mcid == Some(0)).unwrap();
        let p_item = ordered.iter().find(|c| c.mcid == Some(1)).unwrap();
        assert!(h_item.is_heading);
        assert!(!p_item.is_heading, "sibling P must not inherit H1's flag");
        assert_eq!(p_item.heading_level, None);
    }

    /// D8b coverage — list role variants on direct MCRs (LI carrying
    /// its own MCR without LBody/Lbl wrappers) and LBody siblings
    /// inside one LI.
    #[test]
    fn test_list_role_variants() {
        // Tree:
        // L
        //   ├─ LI (mcid=0, direct)         → role = LI
        //   └─ LI
        //        ├─ Lbl  (mcid=1)          → role = Lbl
        //        └─ LBody (mcid=2)         → role = LBody ~keep
        let mut l = StructElem::new(StructType::L);
        let mut li_a = StructElem::new(StructType::LI);
        li_a.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut li_b = StructElem::new(StructType::LI);
        let mut lbl = StructElem::new(StructType::Lbl);
        lbl.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut lbody = StructElem::new(StructType::LBody);
        lbody.add_child(StructChild::MarkedContentRef {
            mcid: 2,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        li_b.add_child(StructChild::StructElem(Box::new(lbl)));
        li_b.add_child(StructChild::StructElem(Box::new(lbody)));
        l.add_child(StructChild::StructElem(Box::new(li_a)));
        l.add_child(StructChild::StructElem(Box::new(li_b)));
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(l);

        let ordered = traverse_structure_tree(&tree, 0).unwrap();
        let m0 = ordered.iter().find(|c| c.mcid == Some(0)).unwrap();
        let m1 = ordered.iter().find(|c| c.mcid == Some(1)).unwrap();
        let m2 = ordered.iter().find(|c| c.mcid == Some(2)).unwrap();
        assert!(matches!(m0.list_role, Some(ListRole::LI)));
        assert!(matches!(m1.list_role, Some(ListRole::Lbl)));
        assert!(matches!(m2.list_role, Some(ListRole::LBody)));
        assert!(!m0.is_heading && !m1.is_heading && !m2.is_heading);
    }

    /// D5 coverage at the traversal layer — block_id must increment
    /// across sibling block elements but stay constant inside one
    /// block, even when the block contains multiple Span children.
    #[test]
    fn test_block_id_groups_within_block_and_changes_across() {
        let mut doc = StructElem::new(StructType::Document);
        let mut p1 = StructElem::new(StructType::P);
        let mut span_a = StructElem::new(StructType::Span);
        span_a.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut span_b = StructElem::new(StructType::Span);
        span_b.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        p1.add_child(StructChild::StructElem(Box::new(span_a)));
        p1.add_child(StructChild::StructElem(Box::new(span_b)));
        let mut p2 = StructElem::new(StructType::P);
        p2.add_child(StructChild::MarkedContentRef {
            mcid: 2,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        doc.add_child(StructChild::StructElem(Box::new(p1)));
        doc.add_child(StructChild::StructElem(Box::new(p2)));
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(doc);

        let ordered = traverse_structure_tree(&tree, 0).unwrap();
        let m0 = ordered.iter().find(|c| c.mcid == Some(0)).unwrap();
        let m1 = ordered.iter().find(|c| c.mcid == Some(1)).unwrap();
        let m2 = ordered.iter().find(|c| c.mcid == Some(2)).unwrap();
        assert_eq!(
            m0.block_id, m1.block_id,
            "two MCRs inside the same /P must share block_id"
        );
        assert_ne!(
            m0.block_id, m2.block_id,
            "MCRs in different /P elements must have different block_id"
        );
        assert!(m0.block_id > 0, "block_id should be positive once any block is entered");
    }

    /// D5 coverage — Span elements at the root (no enclosing block)
    /// keep block_id=0 so the converter's "Some, Some, equal" check
    /// stays well-defined.
    #[test]
    fn test_root_span_has_block_id_zero() {
        let mut span = StructElem::new(StructType::Span);
        span.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(span);
        let ordered = traverse_structure_tree(&tree, 0).unwrap();
        assert_eq!(ordered[0].block_id, 0);
    }

    #[test]
    fn test_object_ref_skipped() {
        let mut root = StructElem::new(StructType::Document);
        root.add_child(StructChild::ObjectRef(42, 0));
        root.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        let order = extract_reading_order(&struct_tree, 0).unwrap();
        assert_eq!(order, vec![0]);
    }

    #[test]
    fn test_traverse_all_pages() {
        let mut root = StructElem::new(StructType::Document);

        let mut p1 = StructElem::new(StructType::P);
        p1.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        let mut p2 = StructElem::new(StructType::P);
        p2.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 1,
            scope: crate::structure::McidScope::Page(1),
        });

        let mut p3 = StructElem::new(StructType::P);
        p3.add_child(StructChild::MarkedContentRef {
            mcid: 2,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        root.add_child(StructChild::StructElem(Box::new(p1)));
        root.add_child(StructChild::StructElem(Box::new(p2)));
        root.add_child(StructChild::StructElem(Box::new(p3)));

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        let all_pages = traverse_structure_tree_all_pages(&struct_tree);
        assert_eq!(all_pages.len(), 2); // pages 0 and 1 ~keep
        assert_eq!(all_pages[&0].len(), 2); // MCIDs 0 and 2 ~keep
        assert_eq!(all_pages[&1].len(), 1); // MCID 1 ~keep
    }

    #[test]
    fn test_actual_text_descendants_recorded_for_assembler_suppression() {
        // The per-page traversal continues to record descendant MCIDs
        // when their ancestor carries /ActualText. The replacement
        // itself is resolved separately via `build_actualtext_index`
        // (so multi-page emit-once stays consistent across paths). The
        // assembler then suppresses the descendant MCID and emits the
        // replacement at the anchor's position. ~keep
        let mut root = StructElem::new(StructType::Document);
        let mut elem = StructElem::new(StructType::Span);
        elem.actual_text = Some("Replacement text".to_string());
        elem.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        root.add_child(StructChild::StructElem(Box::new(elem)));
        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        let ordered = traverse_structure_tree(&struct_tree, 0).unwrap();
        // Descendant MCID still present and uncoated; assembler drops
        // it via covered_mcids from the index. ~keep
        assert_eq!(ordered.len(), 1);
        assert_eq!(ordered[0].mcid, Some(0));
        assert_eq!(ordered[0].actual_text, None);

        let idx = build_actualtext_index(&struct_tree);
        assert!(idx.covered_mcids.contains(&(crate::structure::McidScope::Page(0), 0)));
        assert_eq!(
            idx.mcid_to_actual_text
                .get(&(crate::structure::McidScope::Page(0), 0))
                .map(|s| &**s),
            Some("Replacement text")
        );
    }

    #[test]
    fn test_actual_text_wrong_page() {
        let mut root = StructElem::new(StructType::Document);

        let mut elem = StructElem::new(StructType::Span);
        elem.actual_text = Some("Replacement".to_string());
        elem.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 1,
            scope: crate::structure::McidScope::Page(1),
        });

        root.add_child(StructChild::StructElem(Box::new(elem)));

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        // Page 0 has no descendant MCID, so per-page traversal returns
        // empty. The index records the (page-1, MCID-0) coverage. ~keep
        let ordered = traverse_structure_tree(&struct_tree, 0).unwrap();
        assert!(ordered.is_empty());
        let idx = build_actualtext_index(&struct_tree);
        assert!(idx.covered_mcids.contains(&(crate::structure::McidScope::Page(1), 0)));
        assert_eq!(
            idx.mcid_to_actual_text
                .get(&(crate::structure::McidScope::Page(1), 0))
                .map(|s| &**s),
            Some("Replacement")
        );
    }

    #[test]
    fn test_heading_and_block_flags() {
        let mut root = StructElem::new(StructType::Document);

        let mut h1 = StructElem::new(StructType::H1);
        h1.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        let mut span = StructElem::new(StructType::Span);
        span.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        root.add_child(StructChild::StructElem(Box::new(h1)));
        root.add_child(StructChild::StructElem(Box::new(span)));

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        let ordered = traverse_structure_tree(&struct_tree, 0).unwrap();
        assert_eq!(ordered.len(), 2);
        assert!(ordered[0].is_heading);
        assert!(ordered[0].is_block);
        assert!(!ordered[1].is_heading);
        assert!(!ordered[1].is_block);
    }

    #[test]
    fn test_collect_pages() {
        let mut elem = StructElem::new(StructType::Document);
        elem.page = Some(0);

        let mut child = StructElem::new(StructType::P);
        child.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 1,
            scope: crate::structure::McidScope::Page(1),
        });
        child.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 2,
            scope: crate::structure::McidScope::Page(2),
        });

        elem.add_child(StructChild::StructElem(Box::new(child)));

        let pages = collect_pages(&elem);
        assert_eq!(pages, vec![0, 1, 2]);
    }

    #[test]
    fn test_traverse_all_pages_with_actual_text_does_not_repeat_per_page() {
        let mut root = StructElem::new(StructType::Document);
        let mut elem = StructElem::new(StructType::Span);
        elem.actual_text = Some("Hello".to_string());
        elem.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        elem.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 1,
            scope: crate::structure::McidScope::Page(1),
        });
        root.add_child(StructChild::StructElem(Box::new(elem)));
        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        let all_pages = traverse_structure_tree_all_pages(&struct_tree);
        assert!(all_pages.contains_key(&0));
        assert!(all_pages.contains_key(&1));
        // Descendant MCIDs surface on their own page, with no
        // actual_text on the OrderedContent itself. ~keep
        for items in all_pages.values() {
            for item in items {
                assert!(item.actual_text.is_none());
            }
        }
        let idx = build_actualtext_index(&struct_tree);
        // The bearing element covers both pages; first page wins for
        // emission, the second is suppress-only. ~keep
        assert!(idx.covered_mcids.contains(&(crate::structure::McidScope::Page(0), 0)));
        assert!(idx.covered_mcids.contains(&(crate::structure::McidScope::Page(1), 1)));
        assert!(
            idx.mcid_to_actual_text
                .contains_key(&(crate::structure::McidScope::Page(0), 0))
        );
        assert!(idx.suppress_only.contains(&(crate::structure::McidScope::Page(1), 1)));
    }

    #[test]
    fn test_traverse_all_pages_word_break_with_children() {
        let mut root = StructElem::new(StructType::P);

        let mut wb = StructElem::new(StructType::WB);
        let mut child = StructElem::new(StructType::Span);
        child.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        wb.add_child(StructChild::StructElem(Box::new(child)));

        root.add_child(StructChild::StructElem(Box::new(wb)));

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        let all_pages = traverse_structure_tree_all_pages(&struct_tree);
        let page0 = &all_pages[&0];
        assert!(page0.iter().any(|c| c.is_word_break));
        assert!(page0.iter().any(|c| c.mcid == Some(0)));
    }

    #[test]
    fn test_traverse_all_pages_object_ref() {
        let mut root = StructElem::new(StructType::Document);
        root.add_child(StructChild::ObjectRef(99, 0));
        root.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        let mut struct_tree = StructTreeRoot::new();
        struct_tree.add_root_element(root);

        let all_pages = traverse_structure_tree_all_pages(&struct_tree);
        assert_eq!(all_pages[&0].len(), 1);
        assert_eq!(all_pages[&0][0].mcid, Some(0));
    }

    #[test]
    fn test_has_content_on_page_deep() {
        let mut root = StructElem::new(StructType::Document);
        let mut sect = StructElem::new(StructType::Sect);
        let mut p = StructElem::new(StructType::P);
        p.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 3,
            scope: crate::structure::McidScope::Page(3),
        });
        sect.add_child(StructChild::StructElem(Box::new(p)));
        root.add_child(StructChild::StructElem(Box::new(sect)));

        assert!(has_content_on_page(&root, 3));
        assert!(!has_content_on_page(&root, 0));
    }

    // === ActualTextIndex builder tests ===
    //
    // The builder satisfies these invariants per ISO 32000-1:2008 §14.9.4:
    //   - Every (page, MCID) under an ActualText-bearing element is recorded
    //     in `covered_mcids`.
    //   - The bearing element's first page (min pre-order page of any
    //     descendant MCR) is the emission page; that page's (page, mcid)
    //     pairs land in `mcid_to_actual_text` with the innermost active
    //     replacement.
    //   - Pairs on non-first pages land in `suppress_only` to suppress
    //     raw glyphs without re-emitting (emit-once across pages).
    //   - When ActualText scopes nest, the inner replacement wins for
    //     `(page, mcid)` keys the inner scope covers (recorded under
    //     the inner scope's text in `mcid_to_actual_text`). ~keep

    #[test]
    fn test_actualtext_index_simple_single_mcid() {
        // Span /ActualText "fi" /K 0 on page 0. ~keep
        let mut span = StructElem::new(StructType::Span);
        span.actual_text = Some("fi".to_string());
        span.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        let mut tree = StructTreeRoot::new();
        tree.add_root_element(span);

        let idx = build_actualtext_index(&tree);
        assert!(idx.covered_mcids.contains(&(crate::structure::McidScope::Page(0), 0)));
        assert_eq!(
            idx.mcid_to_actual_text
                .get(&(crate::structure::McidScope::Page(0), 0))
                .map(|s| &**s),
            Some("fi")
        );
        assert!(idx.suppress_only.is_empty());
    }

    #[test]
    fn test_actualtext_index_nested_inner_wins() {
        // Outer Span /ActualText "outer" wrapping inner Span /ActualText
        // "inner" wrapping MCID 5. Inner replacement must win for MCID 5. ~keep
        let mut inner = StructElem::new(StructType::Span);
        inner.actual_text = Some("inner".to_string());
        inner.add_child(StructChild::MarkedContentRef {
            mcid: 5,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        let mut outer = StructElem::new(StructType::Span);
        outer.actual_text = Some("outer".to_string());
        outer.add_child(StructChild::StructElem(Box::new(inner)));

        let mut tree = StructTreeRoot::new();
        tree.add_root_element(outer);

        let idx = build_actualtext_index(&tree);
        // The leaf MCID is covered by the INNER text (inner-wins). ~keep
        assert_eq!(
            idx.mcid_to_actual_text
                .get(&(crate::structure::McidScope::Page(0), 5))
                .map(|s| &**s),
            Some("inner")
        );
        assert!(idx.covered_mcids.contains(&(crate::structure::McidScope::Page(0), 5)));
    }

    #[test]
    fn test_actualtext_index_nested_outer_sibling_with_inner_subtree() {
        // CRITICAL-1 shape:
        //   Outer Span /ActualText "O" /K [Inner /K 0, MCID 1]
        //   Inner Span /ActualText "I" /K 0
        // Expected: (page 0, MCID 0) → "I"; (page 0, MCID 1) → "O".
        // Both are covered; both must emit (the outer is NOT shadowed
        // even though the inner exists, because MCID 1 belongs to the
        // outer scope only). ~keep
        let mut inner = StructElem::new(StructType::Span);
        inner.actual_text = Some("I".to_string());
        inner.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        let mut outer = StructElem::new(StructType::Span);
        outer.actual_text = Some("O".to_string());
        outer.add_child(StructChild::StructElem(Box::new(inner)));
        outer.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });

        let mut tree = StructTreeRoot::new();
        tree.add_root_element(outer);
        let idx = build_actualtext_index(&tree);

        assert_eq!(
            idx.mcid_to_actual_text
                .get(&(crate::structure::McidScope::Page(0), 0))
                .map(|s| &**s),
            Some("I")
        );
        assert_eq!(
            idx.mcid_to_actual_text
                .get(&(crate::structure::McidScope::Page(0), 1))
                .map(|s| &**s),
            Some("O")
        );
        assert!(idx.covered_mcids.contains(&(crate::structure::McidScope::Page(0), 0)));
        assert!(idx.covered_mcids.contains(&(crate::structure::McidScope::Page(0), 1)));
    }

    #[test]
    fn test_actualtext_index_multi_page_first_page_emits_others_suppress() {
        // /H1 /ActualText "Heading X" covering MCIDs on pages 0 AND 1.
        // The bearing element's first descendant in pre-order sits on
        // page 1 first, then page 0 — the first descendant wins (page 1). ~keep
        let mut h1 = StructElem::new(StructType::H1);
        h1.actual_text = Some("Heading X".to_string());
        h1.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 1,
            scope: crate::structure::McidScope::Page(1),
        });
        h1.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(h1);
        let idx = build_actualtext_index(&tree);
        assert!(idx.covered_mcids.contains(&(crate::structure::McidScope::Page(1), 0)));
        assert!(idx.covered_mcids.contains(&(crate::structure::McidScope::Page(0), 1)));
        assert_eq!(
            idx.mcid_to_actual_text
                .get(&(crate::structure::McidScope::Page(1), 0))
                .map(|s| &**s),
            Some("Heading X")
        );
        assert!(idx.suppress_only.contains(&(crate::structure::McidScope::Page(0), 1)));
        assert!(
            !idx.mcid_to_actual_text
                .contains_key(&(crate::structure::McidScope::Page(0), 1))
        );
    }

    #[test]
    fn test_actualtext_index_multi_mcid_subtree() {
        // Span /ActualText "expanded" /K [7 8 9]. All three MCIDs
        // suppressed; all three share the same replacement on page 0. ~keep
        let mut span = StructElem::new(StructType::Span);
        span.actual_text = Some("expanded".to_string());
        for m in [7, 8, 9] {
            span.add_child(StructChild::MarkedContentRef {
                mcid: m,
                page: 0,
                scope: crate::structure::McidScope::Page(0),
            });
        }
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(span);
        let idx = build_actualtext_index(&tree);
        for m in [7, 8, 9] {
            assert!(idx.covered_mcids.contains(&(crate::structure::McidScope::Page(0), m)));
            assert_eq!(
                idx.mcid_to_actual_text
                    .get(&(crate::structure::McidScope::Page(0), m))
                    .map(|s| &**s),
                Some("expanded")
            );
        }
    }

    #[test]
    fn test_actualtext_index_no_actualtext_yields_empty() {
        let mut p = StructElem::new(StructType::P);
        p.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(p);
        let idx = build_actualtext_index(&tree);
        assert!(idx.is_empty());
        assert!(idx.mcid_to_actual_text.is_empty());
        assert!(idx.covered_mcids.is_empty());
    }

    #[test]
    fn test_actualtext_index_empty_actualtext_is_ignored() {
        // An empty /ActualText string MUST be ignored: a producer that
        // wrote it likely means "no replacement". ~keep
        let mut span = StructElem::new(StructType::Span);
        span.actual_text = Some(String::new());
        span.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(span);
        let idx = build_actualtext_index(&tree);
        assert!(idx.is_empty());
    }

    #[test]
    fn test_actualtext_index_no_descendant_mcid_drops_scope() {
        // /ActualText with no descendant MCID has nothing to attach
        // to and contributes no entries. ~keep
        let mut span = StructElem::new(StructType::Span);
        span.actual_text = Some("ghost".to_string());
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(span);
        let idx = build_actualtext_index(&tree);
        assert!(idx.is_empty());
    }

    #[test]
    fn test_actualtext_index_figure_with_actualtext() {
        // Figure /ActualText "logo text". Same shape as a Span. ~keep
        let mut fig = StructElem::new(StructType::Figure);
        fig.actual_text = Some("logo text".to_string());
        fig.add_child(StructChild::MarkedContentRef {
            mcid: 4,
            page: 2,
            scope: crate::structure::McidScope::Page(2),
        });
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(fig);
        let idx = build_actualtext_index(&tree);
        assert_eq!(
            idx.mcid_to_actual_text
                .get(&(crate::structure::McidScope::Page(2), 4))
                .map(|s| &**s),
            Some("logo text")
        );
    }

    #[test]
    fn test_actualtext_index_cross_page_mcid_collision() {
        // CRITICAL-2 shape: page 0 has /H1 /ActualText "Heading" /K MCID 0
        // (covered); page 1 has a plain /P /K MCID 0 (NOT covered).
        // The (page, mcid) keying must keep them independent. ~keep
        let mut h1 = StructElem::new(StructType::H1);
        h1.actual_text = Some("Heading".to_string());
        h1.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        let mut p = StructElem::new(StructType::P);
        p.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 1,
            scope: crate::structure::McidScope::Page(1),
        });

        let mut doc = StructElem::new(StructType::Document);
        doc.add_child(StructChild::StructElem(Box::new(h1)));
        doc.add_child(StructChild::StructElem(Box::new(p)));
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(doc);

        let idx = build_actualtext_index(&tree);
        assert!(idx.covered_mcids.contains(&(crate::structure::McidScope::Page(0), 0)));
        assert!(!idx.covered_mcids.contains(&(crate::structure::McidScope::Page(1), 0)));
        assert!(!idx.suppress_only.contains(&(crate::structure::McidScope::Page(1), 0)));
        assert_eq!(
            idx.mcid_to_actual_text
                .get(&(crate::structure::McidScope::Page(0), 0))
                .map(|s| &**s),
            Some("Heading")
        );
    }

    // ============================================================================
    // McidScope (ISO 32000-1:2008 §14.7.4.3) — per-content-stream MCID namespaces.
    //
    // The earlier `(page, mcid)` keying silently merged MCIDs that
    // came from distinct content streams on the same page. Per spec,
    // page content / Form XObject content / Tiling Pattern content
    // each define their own MCID namespace. These tests lock in that
    // the builder keeps them apart.
    // ============================================================================ ~keep

    /// The canonical bug shape: two Form XObjects on the same page,
    /// both emitting MCID 0, each wrapped by an ActualText-bearing
    /// StructElem. The pre-fix `(page, mcid)` keying would have
    /// collapsed them onto `(0, 0) → "Y"` (last-writer-wins). The
    /// fix keys by `(McidScope::Form(form_ref), mcid)` and keeps
    /// both replacements distinct.
    #[test]
    fn two_forms_with_same_mcid_on_same_page_do_not_collide() {
        let form_a = crate::object::ObjectRef::new(100, 0);
        let form_b = crate::object::ObjectRef::new(101, 0);

        let mut span_a = StructElem::new(StructType::Span);
        span_a.actual_text = Some("X".to_string());
        span_a.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Form(form_a),
        });

        let mut span_b = StructElem::new(StructType::Span);
        span_b.actual_text = Some("Y".to_string());
        span_b.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Form(form_b),
        });

        let mut doc = StructElem::new(StructType::Document);
        doc.add_child(StructChild::StructElem(Box::new(span_a)));
        doc.add_child(StructChild::StructElem(Box::new(span_b)));
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(doc);

        let idx = build_actualtext_index(&tree);

        let key_a = (crate::structure::McidScope::Form(form_a), 0);
        let key_b = (crate::structure::McidScope::Form(form_b), 0);
        assert!(idx.covered_mcids.contains(&key_a));
        assert!(idx.covered_mcids.contains(&key_b));

        assert_eq!(idx.mcid_to_actual_text.get(&key_a).map(|s| &**s), Some("X"));
        assert_eq!(idx.mcid_to_actual_text.get(&key_b).map(|s| &**s), Some("Y"));
    }

    /// Form-scoped MCID lookup uses `McidScope::Form` regardless of
    /// the page number recorded on the MCR (`/Pg`) — the form's
    /// content stream is the namespace.
    #[test]
    fn actualtext_with_stm_form_resolves_to_form_scope() {
        let form_ref = crate::object::ObjectRef::new(42, 0);
        let mut span = StructElem::new(StructType::Span);
        span.actual_text = Some("alt".to_string());
        span.add_child(StructChild::MarkedContentRef {
            mcid: 3,
            page: 0,
            scope: crate::structure::McidScope::Form(form_ref),
        });
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(span);

        let idx = build_actualtext_index(&tree);
        let key = (crate::structure::McidScope::Form(form_ref), 3);
        assert!(idx.covered_mcids.contains(&key));
        assert_eq!(idx.mcid_to_actual_text.get(&key).map(|s| &**s), Some("alt"));
        assert!(!idx.covered_mcids.contains(&(crate::structure::McidScope::Page(0), 3)));
    }

    /// Same as above but for Tiling Patterns (§8.7.3.3 + §14.7.4.3).
    #[test]
    fn actualtext_with_stm_pattern_resolves_to_pattern_scope() {
        let pattern_ref = crate::object::ObjectRef::new(7, 0);
        let mut span = StructElem::new(StructType::Span);
        span.actual_text = Some("dec".to_string());
        span.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 0,
            scope: crate::structure::McidScope::Pattern(pattern_ref),
        });
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(span);

        let idx = build_actualtext_index(&tree);
        let key = (crate::structure::McidScope::Pattern(pattern_ref), 1);
        assert!(idx.covered_mcids.contains(&key));
        assert_eq!(idx.mcid_to_actual_text.get(&key).map(|s| &**s), Some("dec"));
    }

    /// Two Tiling Patterns on the same page emit MCID 0 in their
    /// own streams — the index keeps them distinct.
    #[test]
    fn pattern_with_actualtext_keys_under_pattern_scope() {
        let pat_a = crate::object::ObjectRef::new(70, 0);
        let pat_b = crate::object::ObjectRef::new(71, 0);

        let mut span_a = StructElem::new(StructType::Span);
        span_a.actual_text = Some("alpha".to_string());
        span_a.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Pattern(pat_a),
        });

        let mut span_b = StructElem::new(StructType::Span);
        span_b.actual_text = Some("beta".to_string());
        span_b.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Pattern(pat_b),
        });

        let mut doc = StructElem::new(StructType::Document);
        doc.add_child(StructChild::StructElem(Box::new(span_a)));
        doc.add_child(StructChild::StructElem(Box::new(span_b)));
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(doc);

        let idx = build_actualtext_index(&tree);
        let ka = (crate::structure::McidScope::Pattern(pat_a), 0);
        let kb = (crate::structure::McidScope::Pattern(pat_b), 0);
        assert_eq!(idx.mcid_to_actual_text.get(&ka).map(|s| &**s), Some("alpha"));
        assert_eq!(idx.mcid_to_actual_text.get(&kb).map(|s| &**s), Some("beta"));
    }

    /// When the MCR omits `/Stm` (the parser hands the builder a
    /// `McidScope::Page(p)`), the page namespace is used.
    #[test]
    fn actualtext_without_stm_falls_back_to_page_scope() {
        let mut span = StructElem::new(StructType::Span);
        span.actual_text = Some("plain".to_string());
        span.add_child(StructChild::MarkedContentRef {
            mcid: 5,
            page: 2,
            scope: crate::structure::McidScope::Page(2),
        });
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(span);

        let idx = build_actualtext_index(&tree);
        let key = (crate::structure::McidScope::Page(2), 5);
        assert!(idx.covered_mcids.contains(&key));
        assert_eq!(idx.mcid_to_actual_text.get(&key).map(|s| &**s), Some("plain"));
    }

    /// Robustness: a malformed parent_tree / cycle should not panic
    /// the builder. Tests the no-MCR case (the rest of the builder
    /// is exercised by other tests).
    #[test]
    fn malformed_mcr_dict_does_not_panic_in_builder() {
        // No descendants at all — drops the scope, returns empty index. ~keep
        let mut span = StructElem::new(StructType::Span);
        span.actual_text = Some("ghost".to_string());
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(span);
        let idx = build_actualtext_index(&tree);
        assert!(idx.is_empty());
    }

    /// Mixed scopes under one ActualText: Page-scoped descendants
    /// follow the cross-page first-page rule; Form-scoped descendants
    /// emit at every covered key (each form is its own namespace).
    #[test]
    fn mixed_scopes_under_one_actualtext_use_per_namespace_rules() {
        let form_ref = crate::object::ObjectRef::new(50, 0);
        let mut outer = StructElem::new(StructType::Span);
        outer.actual_text = Some("alt".to_string());
        // Page-scoped, page 0: this is the "first page" → emits. ~keep
        outer.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Page(0),
        });
        // Page-scoped, page 1: not the first page → suppress-only. ~keep
        outer.add_child(StructChild::MarkedContentRef {
            mcid: 1,
            page: 1,
            scope: crate::structure::McidScope::Page(1),
        });
        // Form-scoped: independent namespace, emits regardless of
        // page-scope first-page logic. ~keep
        outer.add_child(StructChild::MarkedContentRef {
            mcid: 0,
            page: 0,
            scope: crate::structure::McidScope::Form(form_ref),
        });
        let mut tree = StructTreeRoot::new();
        tree.add_root_element(outer);

        let idx = build_actualtext_index(&tree);

        let page0 = (crate::structure::McidScope::Page(0), 0);
        let page1 = (crate::structure::McidScope::Page(1), 1);
        let formk = (crate::structure::McidScope::Form(form_ref), 0);

        assert_eq!(idx.mcid_to_actual_text.get(&page0).map(|s| &**s), Some("alt"));
        assert!(idx.suppress_only.contains(&page1));
        assert!(!idx.mcid_to_actual_text.contains_key(&page1));
        assert_eq!(idx.mcid_to_actual_text.get(&formk).map(|s| &**s), Some("alt"));
    }
}
