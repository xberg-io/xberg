//! Parser for PDF structure trees.
//!
//! Parses StructTreeRoot and StructElem dictionaries according to PDF spec Section 14.7.

use super::types::{StructChild, StructElem, StructTreeRoot, StructType};
use crate::document::PdfDocument;
use crate::error::Error;
use crate::object::Object;
use std::collections::{HashMap, HashSet};

use std::time::Duration;

/// An optional bound on structure-tree parsing.
///
/// A parsing library returns the *complete* tree by default, so the unbounded
/// guard (`ParseBudget::unbounded`) applies no time limit and no element cap.
/// Callers that deliberately want to bound the work — e.g. the extraction
/// reading-order strategy, which can fall back to content-stream order — build a
/// bounded guard from a wall-clock `Duration`, which also caps the element count
/// at [`MAX_STRUCT_ELEMENTS`]. On `wasm32-unknown-unknown` `std::time::Instant`
/// panics, so the time limit is a no-op there and only the element cap applies.
#[derive(Clone, Copy, Default)]
struct ParseBudget {
    #[cfg(not(target_arch = "wasm32"))]
    deadline: Option<std::time::Instant>,
    max_elements: Option<usize>,
}

impl ParseBudget {
    /// No limit — parse the entire tree (the default for a general-purpose API).
    fn unbounded() -> Self {
        Self::default()
    }

    /// Bound parsing by `budget` wall-clock time, and cap the element count at
    /// [`MAX_STRUCT_ELEMENTS`] as a companion guard against pathological trees.
    fn from_option(budget: Option<Duration>) -> Self {
        match budget {
            None => Self::unbounded(),
            Some(_budget) => Self {
                #[cfg(not(target_arch = "wasm32"))]
                deadline: Some(std::time::Instant::now() + _budget),
                max_elements: Some(MAX_STRUCT_ELEMENTS),
            },
        }
    }

    /// Whether either bound has been exceeded at the current element count.
    /// Always `false` for an unbounded guard.
    #[inline]
    fn exceeded(&self, element_count: usize) -> bool {
        #[cfg(test)]
        if self.max_elements.is_some() && TEST_PARSE_CUTOFF.get().is_some_and(|cutoff| element_count >= cutoff) {
            return true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(deadline) = self.deadline
            && std::time::Instant::now() > deadline
        {
            return true;
        }
        self.max_elements.is_some_and(|max| element_count > max)
    }

    fn check(&self, element_count: usize) -> Result<(), ParseFailure> {
        if self.exceeded(element_count) {
            return Err(ParseFailure::BudgetExceeded);
        }
        Ok(())
    }
}

enum ParseFailure {
    BudgetExceeded,
    Pdf(Error),
}

impl From<Error> for ParseFailure {
    fn from(error: Error) -> Self {
        Self::Pdf(error)
    }
}

#[cfg(test)]
thread_local! {
    static TEST_PARSE_CUTOFF: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub(crate) fn with_structure_parse_cutoff<T>(after_elements: usize, run: impl FnOnce() -> T) -> T {
    struct Restore(Option<usize>);
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_PARSE_CUTOFF.set(self.0);
        }
    }
    let _restore = Restore(TEST_PARSE_CUTOFF.replace(Some(after_elements)));
    run()
}

/// A timer for measuring elapsed time, WASM-safe.
#[derive(Clone, Copy)]
struct Timer {
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
}

impl Timer {
    fn now() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                start: std::time::Instant::now(),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self {}
        }
    }

    fn elapsed_debug(&self) -> String {
        #[cfg(not(target_arch = "wasm32"))]
        {
            format!("{:?}", self.start.elapsed())
        }
        #[cfg(target_arch = "wasm32")]
        {
            "(time unavailable on wasm)".to_string()
        }
    }
}

/// Maximum number of structure elements to parse.
/// Trees larger than this cause expensive traversal (seconds for 50K+ elements).
/// 10K elements is sufficient for any normal document; larger trees indicate
/// deeply structured books where content-stream order works equally well.
const MAX_STRUCT_ELEMENTS: usize = 10_000;

/// Decode a PDF text string (UTF-16BE/LE with BOM, or PDFDocEncoding).
fn decode_pdf_text_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE with BOM ~keep
        let utf16_pairs: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&utf16_pairs).unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string())
    } else if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        // UTF-16LE with BOM ~keep
        let utf16_pairs: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&utf16_pairs).unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string())
    } else {
        bytes
            .iter()
            .filter_map(|&b| crate::fonts::font_dict::pdfdoc_encoding_lookup(b))
            .collect()
    }
}

/// Helper function to resolve an object (handles both direct objects and references).
fn resolve_object(document: &PdfDocument, obj: &Object) -> Result<Object, Error> {
    match obj {
        Object::Reference(obj_ref) => document.load_object(*obj_ref),
        _ => Ok(obj.clone()),
    }
}

/// Build a mapping from page object IDs to page indices.
/// This allows resolving /Pg references in marked content references.
///
/// Shares the page-tree walk of [`PdfDocument::all_page_refs`], which is
/// cycle- and depth-guarded and infers node kind when /Type is missing.
fn build_page_map(document: &PdfDocument) -> HashMap<u32, u32> {
    let mut page_map = HashMap::new();
    for (index, page_ref) in document.all_page_refs().unwrap_or_default().into_iter().enumerate() {
        page_map.entry(page_ref.id).or_insert(index as u32);
    }
    page_map
}

/// Parse the complete structure tree from a PDF document.
///
/// Reads the `/StructTreeRoot` from the catalog and recursively parses every
/// structure element. This is the general-purpose entry point: it applies **no**
/// time budget and **no** element cap, so it returns the whole tree however large
/// it is. Callers that need to bound the work (e.g. to keep an interactive
/// pipeline responsive and fall back to another strategy) should use
/// [`parse_structure_tree_with_budget`].
///
/// # Returns
/// * `Ok(Some(StructTreeRoot))` - the document's structure tree
/// * `Ok(None)` - the document is not tagged (no `/StructTreeRoot`)
/// * `Err(Error)` - parsing failed
pub fn parse_structure_tree(document: &PdfDocument) -> Result<Option<StructTreeRoot>, Error> {
    parse_structure_tree_inner(document, ParseBudget::unbounded())
}

/// Parse the structure tree with an optional bound on the work done.
///
/// `budget` is a wall-clock limit: `None` parses the complete tree (identical to
/// [`parse_structure_tree`]); `Some(duration)` stops once the deadline is
/// exceeded (or a companion element cap is hit) and returns `Ok(None)` so the
/// caller can fall back to another strategy. Prefer `None` unless you have a
/// concrete responsiveness requirement — a bound can drop the tree on a slow
/// machine even when the document is perfectly valid.
pub fn parse_structure_tree_with_budget(
    document: &PdfDocument,
    budget: Option<Duration>,
) -> Result<Option<StructTreeRoot>, Error> {
    parse_structure_tree_inner(document, ParseBudget::from_option(budget))
}

fn parse_structure_tree_inner(document: &PdfDocument, deadline: ParseBudget) -> Result<Option<StructTreeRoot>, Error> {
    match parse_structure_tree_checked(document, deadline) {
        Ok(tree) => Ok(tree),
        Err(ParseFailure::Pdf(error)) => Err(error),
        Err(ParseFailure::BudgetExceeded) => {
            tracing::warn!("structure tree parse budget exceeded; falling back to content order");
            Ok(None)
        }
    }
}

fn parse_structure_tree_checked(
    document: &PdfDocument,
    deadline: ParseBudget,
) -> Result<Option<StructTreeRoot>, ParseFailure> {
    let parse_start = Timer::now();
    deadline.check(0)?;

    let catalog = document.catalog()?;

    let catalog_dict = catalog
        .as_dict()
        .ok_or_else(|| Error::InvalidPdf("Catalog is not a dictionary".into()))?;

    let struct_tree_root_ref = match catalog_dict.get("StructTreeRoot") {
        Some(obj) => obj,
        None => return Ok(None), // Not a tagged PDF ~keep
    };

    let page_map = build_page_map(document);

    let struct_tree_root_obj = resolve_object(document, struct_tree_root_ref)?;

    // Parse StructTreeRoot dictionary — treat non-dict (e.g. Null from a
    // corrupted parse) as "no structure tree" rather than a hard error. ~keep
    let struct_tree_dict = match struct_tree_root_obj.as_dict() {
        Some(d) => d,
        None => {
            tracing::warn!(
                resolved_type = struct_tree_root_obj.type_name(),
                "StructTreeRoot did not resolve to a dictionary; treating as no structure tree"
            );
            return Ok(None);
        }
    };

    let mut struct_tree = StructTreeRoot::new();

    if let Some(role_map_obj) = struct_tree_dict.get("RoleMap") {
        let role_map_obj = resolve_object(document, role_map_obj)?;
        if let Some(role_map_dict) = role_map_obj.as_dict() {
            for (key, value) in role_map_dict.iter() {
                if let Some(name) = value.as_name() {
                    struct_tree.role_map.insert(key.clone(), name.to_string());
                }
            }
        }
    }

    // Skip ParentTree parsing — it's expensive (recursively loads/parses objects)
    // and not needed for text extraction. The forward traversal of /K children
    // provides reading order. ParentTree is only needed for reverse lookups
    // (MCID → StructElem), which are not used in the extraction pipeline. ~keep

    let mut element_count: usize = 0;
    let mut visited: HashSet<u32> = HashSet::new();

    if let Some(k_obj) = struct_tree_dict.get("K") {
        let k_obj = resolve_object(document, k_obj)?;

        match k_obj {
            Object::Array(arr) => {
                for elem_obj in arr {
                    deadline.check(element_count)?;
                    // Record root element IDs before descending so that a back-reference
                    // from any descendant to this root is detectable as a cycle. ~keep
                    if let Object::Reference(obj_ref) = &elem_obj
                        && !visited.insert(obj_ref.id)
                    {
                        tracing::warn!(
                            object_id = obj_ref.id,
                            "cycle in structure tree: root object already visited, skipping"
                        );
                        continue;
                    }
                    if let Some(elem) = parse_struct_elem(
                        document,
                        &elem_obj,
                        &struct_tree.role_map,
                        &page_map,
                        deadline,
                        &mut element_count,
                        &mut visited,
                        ElemScope::root(),
                    )? {
                        struct_tree.add_root_element(elem);
                    }
                }
            }
            _ => {
                if let Some(elem) = parse_struct_elem(
                    document,
                    &k_obj,
                    &struct_tree.role_map,
                    &page_map,
                    deadline,
                    &mut element_count,
                    &mut visited,
                    ElemScope::root(),
                )? {
                    struct_tree.add_root_element(elem);
                }
            }
        }
    }

    deadline.check(element_count)?;
    tracing::debug!(
        count = element_count,
        root_count = struct_tree.root_elements.len(),
        elapsed = %parse_start.elapsed_debug(),
        "structure tree parsed"
    );

    Ok(Some(struct_tree))
}

/// Maximum structure-element nesting depth; deeper subtrees are truncated so
/// recursive parsing, traversal and drop stay within the stack.
pub(crate) const MAX_STRUCT_DEPTH: usize = 128;

/// Position of an element in the tree: its nesting depth and the nearest
/// ancestor's resolved /Pg, which /Pg-less descendants inherit.
#[derive(Clone, Copy)]
struct ElemScope {
    depth: usize,
    page: Option<u32>,
}

impl ElemScope {
    fn root() -> Self {
        Self { depth: 0, page: None }
    }
}

/// Resolves a dictionary's own /Pg reference to a page index.
fn dict_page(dict: &HashMap<String, Object>, page_map: &HashMap<u32, u32>) -> Option<u32> {
    match dict.get("Pg") {
        Some(Object::Reference(pg_ref)) => page_map.get(&pg_ref.id).copied(),
        _ => None,
    }
}

/// Parse a structure element (StructElem) from a PDF object.
fn parse_struct_elem(
    document: &PdfDocument,
    obj: &Object,
    role_map: &HashMap<String, String>,
    page_map: &HashMap<u32, u32>,
    deadline: ParseBudget,
    element_count: &mut usize,
    visited: &mut HashSet<u32>,
    scope: ElemScope,
) -> Result<Option<StructElem>, ParseFailure> {
    deadline.check(*element_count)?;
    if scope.depth > MAX_STRUCT_DEPTH {
        tracing::warn!("structure tree deeper than {MAX_STRUCT_DEPTH} levels, truncating");
        return Ok(None);
    }
    *element_count += 1;

    let obj = resolve_object(document, obj)?;

    let dict = match obj.as_dict() {
        Some(d) => d,
        None => return Ok(None),
    };

    if let Some(type_obj) = dict.get("Type")
        && let Some(type_name) = type_obj.as_name()
    {
        if type_name == "OBJR" {
            // Per PDF spec §14.7.4 Table 323, an OBJR (Object Reference) dictionary
            // references a StructElem (or other PDF object) via its /Obj entry.
            // Transparently dereference it so the caller gets the real StructElem. ~keep
            if let Some(obj_ref_obj) = dict.get("Obj")
                && let Some(obj_ref) = obj_ref_obj.as_reference()
            {
                if !visited.insert(obj_ref.id) {
                    tracing::warn!(
                        object_id = obj_ref.id,
                        "OBJR: cycle detected, object already visited, skipping"
                    );
                    return Ok(None);
                }
                match document.load_object(obj_ref) {
                    Ok(resolved) => {
                        let objr_page = dict_page(dict, page_map).or(scope.page);
                        return parse_struct_elem(
                            document,
                            &resolved,
                            role_map,
                            page_map,
                            deadline,
                            element_count,
                            visited,
                            ElemScope {
                                depth: scope.depth + 1,
                                page: objr_page,
                            },
                        );
                    }
                    Err(error) => {
                        tracing::warn!(
                            object_id = obj_ref.id,
                            generation = obj_ref.generation,
                            error_code = error.telemetry_code(),
                            error_offset = ?error.telemetry_offset(),
                            "OBJR: failed to load /Obj"
                        );
                        return Ok(None);
                    }
                }
            }
            return Ok(None);
        }
        if type_name != "StructElem" {
            return Ok(None);
        }
    }

    let s_obj = match dict.get("S") {
        Some(obj) => obj,
        None => return Ok(None),
    };
    let s_name = match s_obj.as_name() {
        Some(name) => name,
        None => return Ok(None),
    };

    let mapped = role_map.get(s_name);
    let struct_type_str = mapped.map(|s| s.as_str()).unwrap_or(s_name);
    let struct_type = StructType::from_str(struct_type_str);

    let mut struct_elem = StructElem::new(struct_type);
    if mapped.is_some() {
        struct_elem.source_role = Some(s_name.to_string());
    }

    if let Some(page_num) = dict_page(dict, page_map) {
        struct_elem.page = Some(page_num);
    }
    let child_scope = ElemScope {
        depth: scope.depth,
        page: struct_elem.page.or(scope.page),
    };

    // Skip /A (attributes) during text extraction — not needed for reading order.
    // Skip /Alt (alternate description) — not needed for text extraction. ~keep

    // Get /ActualText (replacement text) - optional, per PDF spec Section 14.9.4
    // When present, this text replaces all descendant content for the element. ~keep
    if let Some(at_obj) = dict.get("ActualText") {
        let at_obj = resolve_object(document, at_obj)?;
        if let Some(at_bytes) = at_obj.as_string() {
            let text = decode_pdf_text_string(at_bytes);
            if !text.is_empty() {
                struct_elem.actual_text = Some(text);
            }
        }
    }

    if let Some(k_obj_raw) = dict.get("K") {
        // When /K is an indirect reference that resolves to a struct elem dictionary
        // (as opposed to an array), we lose the object ID after resolve_object and
        // the Dictionary arm of parse_k_children cannot check for cycles.
        // Load it here while we still have the reference ID, insert into visited,
        // and short-circuit if it has already been visited. ~keep
        if let Object::Reference(r) = k_obj_raw {
            let k_resolved = match document.load_object(*r) {
                Ok(obj) => obj,
                Err(error) => {
                    tracing::warn!(
                        object_id = r.id,
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "failed to load /K reference"
                    );
                    return Ok(Some(struct_elem));
                }
            };
            if k_resolved.as_dict().is_some() && !visited.insert(r.id) {
                tracing::warn!(
                    object_id = r.id,
                    "cycle in structure tree: /K object already visited, skipping children"
                );
                return Ok(Some(struct_elem));
            }
            parse_k_children(
                document,
                &k_resolved,
                &mut struct_elem,
                role_map,
                page_map,
                deadline,
                element_count,
                visited,
                child_scope,
            )?;
        } else {
            let k_obj = resolve_object(document, k_obj_raw)?;
            parse_k_children(
                document,
                &k_obj,
                &mut struct_elem,
                role_map,
                page_map,
                deadline,
                element_count,
                visited,
                child_scope,
            )?;
        }
    }

    Ok(Some(struct_elem))
}

/// Parse the /K entry (children) of a structure element.
fn parse_k_children(
    document: &PdfDocument,
    k_obj: &Object,
    parent: &mut StructElem,
    role_map: &HashMap<String, String>,
    page_map: &HashMap<u32, u32>,
    deadline: ParseBudget,
    element_count: &mut usize,
    visited: &mut HashSet<u32>,
    scope: ElemScope,
) -> Result<(), ParseFailure> {
    let page_ctx = parent.page.or(scope.page).unwrap_or(0);
    let child_scope = ElemScope {
        depth: scope.depth + 1,
        page: parent.page.or(scope.page),
    };
    match k_obj {
        Object::Integer(mcid) => {
            // Single MCID — bare integer child references the page's
            // own content stream (ISO 32000-1:2008 §14.7.5.4.2 "MCR"
            // form is reserved for cross-stream refs). ~keep
            if let Some(mcid) = checked_mcid(*mcid) {
                let page = page_ctx;
                parent.add_child(StructChild::MarkedContentRef {
                    mcid,
                    page,
                    scope: crate::structure::McidScope::Page(page),
                });
            }
        }

        Object::Array(arr) => {
            for child_obj in arr {
                deadline.check(*element_count)?;

                // Guard against cycles before resolving: an indirect reference that has
                // already been visited would resolve to a dictionary and slip through to
                // parse_struct_elem without any ID to check. Capture the ID now, while we
                // still have the unresolved Reference, so the check happens before loading. ~keep
                if let Object::Reference(obj_ref) = child_obj
                    && !visited.insert(obj_ref.id)
                {
                    tracing::warn!(
                        object_id = obj_ref.id,
                        "cycle in structure tree: object already visited, skipping"
                    );
                    continue;
                }

                let child_obj = resolve_object(document, child_obj)?;

                match &child_obj {
                    Object::Integer(mcid) => {
                        if let Some(mcid) = checked_mcid(*mcid) {
                            let page = page_ctx;
                            parent.add_child(StructChild::MarkedContentRef {
                                mcid,
                                page,
                                scope: crate::structure::McidScope::Page(page),
                            });
                        }
                    }

                    Object::Dictionary(_) => {
                        // Could be a StructElem or marked content reference ~keep
                        match parse_struct_elem(
                            document,
                            &child_obj,
                            role_map,
                            page_map,
                            deadline,
                            element_count,
                            visited,
                            child_scope,
                        )? {
                            Some(child_elem) => {
                                parent.add_child(StructChild::StructElem(Box::new(child_elem)));
                            }
                            _ => {
                                if let Some(mcr) = parse_marked_content_ref(document, &child_obj, page_map, page_ctx)? {
                                    parent.add_child(mcr);
                                }
                            }
                        }
                    }

                    Object::Reference(obj_ref) => {
                        if !visited.insert(obj_ref.id) {
                            tracing::warn!(
                                object_id = obj_ref.id,
                                "cycle in structure tree: object already visited, skipping"
                            );
                            continue;
                        }
                        match document.load_object(*obj_ref) {
                            Ok(resolved) => {
                                match parse_struct_elem(
                                    document,
                                    &resolved,
                                    role_map,
                                    page_map,
                                    deadline,
                                    element_count,
                                    visited,
                                    child_scope,
                                )? {
                                    Some(child_elem) => {
                                        parent.add_child(StructChild::StructElem(Box::new(child_elem)));
                                    }
                                    _ => {
                                        if let Some(mcr) =
                                            parse_marked_content_ref(document, &resolved, page_map, page_ctx)?
                                        {
                                            parent.add_child(mcr);
                                        }
                                    }
                                }
                            }
                            Err(error) => {
                                tracing::warn!(
                                    object_id = obj_ref.id,
                                    generation = obj_ref.generation,
                                    error_code = error.telemetry_code(),
                                    error_offset = ?error.telemetry_offset(),
                                    "failed to resolve ObjectRef"
                                );
                            }
                        }
                    }

                    _ => {}
                }
            }
        }

        Object::Dictionary(_) => {
            match parse_struct_elem(
                document,
                k_obj,
                role_map,
                page_map,
                deadline,
                element_count,
                visited,
                child_scope,
            )? {
                Some(child_elem) => {
                    parent.add_child(StructChild::StructElem(Box::new(child_elem)));
                }
                _ => {
                    if let Some(mcr) = parse_marked_content_ref(document, k_obj, page_map, page_ctx)? {
                        parent.add_child(mcr);
                    }
                }
            }
        }

        Object::Reference(obj_ref) => {
            if !visited.insert(obj_ref.id) {
                tracing::warn!(
                    object_id = obj_ref.id,
                    "cycle in structure tree: object already visited, skipping"
                );
                return Ok(());
            }
            match document.load_object(*obj_ref) {
                Ok(resolved) => {
                    match parse_struct_elem(
                        document,
                        &resolved,
                        role_map,
                        page_map,
                        deadline,
                        element_count,
                        visited,
                        child_scope,
                    )? {
                        Some(child_elem) => {
                            parent.add_child(StructChild::StructElem(Box::new(child_elem)));
                        }
                        _ => {
                            if let Some(mcr) = parse_marked_content_ref(document, &resolved, page_map, page_ctx)? {
                                parent.add_child(mcr);
                            }
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        object_id = obj_ref.id,
                        generation = obj_ref.generation,
                        error_code = error.telemetry_code(),
                        error_offset = ?error.telemetry_offset(),
                        "failed to resolve ObjectRef"
                    );
                }
            }
        }

        _ => {}
    }

    Ok(())
}

/// Parse a marked content reference dictionary.
///
/// According to ISO 32000-1:2008 §14.7.5.4.2, a marked content reference
/// (MCR) dictionary has:
/// - `/Type /MCR` (optional but, when present, fixes the dict's identity)
/// - `/Pg` — page reference (optional; inherits enclosing StructElem
///   `/Pg` when absent)
/// - `/MCID` — required marked-content identifier
/// - `/Stm` — optional reference to the content stream that holds the
///   MCID. When absent, the MCID lives in the page's own content
///   stream. When present, the stream's dictionary determines the
///   scope:
///     - `/Subtype /Form` → Form XObject scope
///     - `/PatternType 1` → Tiling Pattern scope
///     - anything else → falls back to Page scope with a warning,
///       since we can't classify the stream.
fn parse_marked_content_ref(
    document: &PdfDocument,
    obj: &Object,
    page_map: &HashMap<u32, u32>,
    inherited_page: u32,
) -> Result<Option<StructChild>, Error> {
    let dict = match obj.as_dict() {
        Some(d) => d,
        None => return Ok(None),
    };

    if let Some(type_obj) = dict.get("Type")
        && let Some(type_name) = type_obj.as_name()
        && type_name != "MCR"
    {
        return Ok(None);
    }

    let mcid = match dict.get("MCID").and_then(|obj| obj.as_integer()) {
        Some(mcid) => mcid,
        None => return Ok(None),
    };
    let Some(mcid) = checked_mcid(mcid) else {
        return Ok(None);
    };

    let page = dict_page(dict, page_map).unwrap_or(inherited_page);

    // Resolve /Stm to determine the MCID's content-stream scope.
    // `/StmOwn` is the "inline image" form (§14.7.5.4.2) — out of
    // scope here since inline images do not carry their own MCID
    // namespace. ~keep
    let scope = resolve_mcr_scope(document, dict, page);

    Ok(Some(StructChild::MarkedContentRef { mcid, page, scope }))
}

/// Narrow a `/K` or `/MCID` integer to the `u32` marked-content id space.
///
/// Wrapping collides a malformed id with a real one: `2^32 + 5` becomes MCID 5
/// and duplicates that entry in the page's reading order.
pub(crate) fn checked_mcid(raw: i64) -> Option<u32> {
    match u32::try_from(raw) {
        Ok(mcid) => Some(mcid),
        Err(_) => {
            tracing::warn!(raw_mcid = raw, "marked-content id is out of range, skipping");
            None
        }
    }
}

/// Determine the `McidScope` for a marked-content reference dict.
///
/// When `/Stm` is absent the MCID belongs to the enclosing page's
/// content stream; the scope is `Page(page)`. When `/Stm` is present
/// the referenced stream's dictionary classifies the scope: Form
/// XObjects produce `Form(ref)`, Tiling Patterns produce
/// `Pattern(ref)`. An unclassifiable `/Stm` (unresolved, missing
/// subtype, or unrecognised type) falls back to `Page(page)` with a
/// warning so the assembler still has *some* lookup key — at worst
/// the lookup misses and the raw glyphs flow through unchanged, which
/// is strictly safer than a collision.
fn resolve_mcr_scope(
    document: &PdfDocument,
    mcr_dict: &HashMap<String, Object>,
    page: u32,
) -> crate::structure::McidScope {
    use crate::structure::McidScope;

    let stm = match mcr_dict.get("Stm") {
        Some(s) => s,
        None => return McidScope::Page(page),
    };

    let stm_ref = match stm {
        Object::Reference(r) => *r,
        _ => {
            tracing::warn!("MCR /Stm is not an indirect reference; defaulting to page scope");
            return McidScope::Page(page);
        }
    };

    let stm_obj = match document.load_object(stm_ref) {
        Ok(o) => o,
        Err(error) => {
            tracing::warn!(
                object_id = stm_ref.id,
                generation = stm_ref.generation,
                error_code = error.telemetry_code(),
                error_offset = ?error.telemetry_offset(),
                "MCR /Stm could not be resolved; defaulting to page scope"
            );
            return McidScope::Page(page);
        }
    };

    // For both streams and dictionaries, locate the dict-half. ~keep
    let stream_dict: &HashMap<String, Object> = match &stm_obj {
        Object::Stream { dict, .. } => dict,
        Object::Dictionary(d) => d,
        _ => {
            tracing::warn!(
                object_id = stm_ref.id,
                generation = stm_ref.generation,
                "MCR /Stm resolves to non-stream/non-dict; defaulting to page scope"
            );
            return McidScope::Page(page);
        }
    };

    // Form XObject: /Type /XObject /Subtype /Form (or /Subtype /Form
    // alone — producers sometimes omit /Type because it is optional on
    // an XObject stream per §8.8). ~keep
    if let Some(subtype) = stream_dict.get("Subtype").and_then(|o| o.as_name())
        && subtype == "Form"
    {
        return McidScope::Form(stm_ref);
    }

    // Tiling Pattern: /PatternType 1 (per §8.7.3.3). Shading patterns
    // (PatternType 2) do not have a content stream of their own and
    // cannot host MCIDs. ~keep
    if let Some(pt) = stream_dict.get("PatternType").and_then(|o| o.as_integer())
        && pt == 1
    {
        return McidScope::Pattern(stm_ref);
    }

    tracing::warn!(
        object_id = stm_ref.id,
        generation = stm_ref.generation,
        "MCR /Stm has unknown subtype/pattern type; defaulting to page scope"
    );
    McidScope::Page(page)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_budget_unbounded_never_exceeds() {
        // The default (general-purpose) guard imposes no cap: a huge element
        // count must not trip it, so `parse_structure_tree` returns the whole tree. ~keep
        let guard = ParseBudget::unbounded();
        assert!(!guard.exceeded(0));
        assert!(!guard.exceeded(MAX_STRUCT_ELEMENTS + 1));
        assert!(!guard.exceeded(usize::MAX));
        assert!(!ParseBudget::from_option(None).exceeded(usize::MAX));
    }

    #[test]
    fn parse_budget_bounded_caps_element_count() {
        // A bounded guard (Some(duration)) still applies the companion element
        // cap, deterministically, independent of wall-clock timing. ~keep
        let guard = ParseBudget::from_option(Some(Duration::from_secs(3600)));
        assert!(!guard.exceeded(MAX_STRUCT_ELEMENTS));
        assert!(guard.exceeded(MAX_STRUCT_ELEMENTS + 1));
    }

    #[test]
    fn test_struct_type_mapping() {
        let role_map = {
            let mut map = HashMap::new();
            map.insert("Heading1".to_string(), "H1".to_string());
            map
        };

        let mapped = role_map.get("Heading1").map(|s| s.as_str()).unwrap_or("Heading1");
        assert_eq!(mapped, "H1");
    }

    #[test]
    fn test_decode_pdf_text_string_utf8() {
        let text = b"Hello World";
        assert_eq!(decode_pdf_text_string(text), "Hello World");
    }

    #[test]
    fn test_decode_pdf_text_string_utf16be() {
        // UTF-16BE BOM + "AB" ~keep
        let bytes = vec![0xFE, 0xFF, 0x00, 0x41, 0x00, 0x42];
        assert_eq!(decode_pdf_text_string(&bytes), "AB");
    }

    #[test]
    fn test_decode_pdf_text_string_utf16le() {
        // UTF-16LE BOM + "AB" ~keep
        let bytes = vec![0xFF, 0xFE, 0x41, 0x00, 0x42, 0x00];
        assert_eq!(decode_pdf_text_string(&bytes), "AB");
    }

    #[test]
    fn test_decode_pdf_text_string_pdfdoc_encoding() {
        // ASCII subset works as PDFDocEncoding ~keep
        let bytes = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]; // "Hello" ~keep
        let result = decode_pdf_text_string(&bytes);
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_resolve_object_direct() {
        let obj = Object::Integer(42);
        let doc = {
            let pdf = build_test_pdf();
            PdfDocument::from_bytes(pdf).unwrap()
        };
        let result = resolve_object(&doc, &obj).unwrap();
        assert_eq!(result, Object::Integer(42));
    }

    fn minimal_test_doc() -> PdfDocument {
        let pdf = build_test_pdf();
        PdfDocument::from_bytes(pdf).unwrap()
    }

    #[test]
    fn test_parse_marked_content_ref_not_dict() {
        let doc = minimal_test_doc();
        let obj = Object::Integer(5);
        let page_map = HashMap::new();
        let result = parse_marked_content_ref(&doc, &obj, &page_map, 0).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_marked_content_ref_wrong_type() {
        let doc = minimal_test_doc();
        let mut dict = HashMap::new();
        dict.insert("Type".to_string(), Object::Name("NotMCR".to_string()));
        dict.insert("MCID".to_string(), Object::Integer(5));
        let obj = Object::Dictionary(dict);
        let page_map = HashMap::new();
        let result = parse_marked_content_ref(&doc, &obj, &page_map, 0).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_marked_content_ref_missing_mcid() {
        let doc = minimal_test_doc();
        let mut dict = HashMap::new();
        dict.insert("Type".to_string(), Object::Name("MCR".to_string()));
        let obj = Object::Dictionary(dict);
        let page_map = HashMap::new();
        let result = parse_marked_content_ref(&doc, &obj, &page_map, 0).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_marked_content_ref_valid() {
        let doc = minimal_test_doc();
        let mut dict = HashMap::new();
        dict.insert("Type".to_string(), Object::Name("MCR".to_string()));
        dict.insert("MCID".to_string(), Object::Integer(7));
        let obj = Object::Dictionary(dict);
        let page_map = HashMap::new();
        let result = parse_marked_content_ref(&doc, &obj, &page_map, 0).unwrap();
        assert!(result.is_some());
        if let Some(StructChild::MarkedContentRef { mcid, page, scope }) = result {
            assert_eq!(mcid, 7);
            assert_eq!(page, 0);
            assert_eq!(scope, crate::structure::McidScope::Page(0));
        }
    }

    #[test]
    fn test_parse_marked_content_ref_with_page() {
        let doc = minimal_test_doc();
        let mut page_map = HashMap::new();
        page_map.insert(10, 2u32); // object 10 -> page 2 ~keep

        let mut dict = HashMap::new();
        dict.insert("Type".to_string(), Object::Name("MCR".to_string()));
        dict.insert("MCID".to_string(), Object::Integer(3));
        dict.insert(
            "Pg".to_string(),
            Object::Reference(crate::object::ObjectRef { id: 10, generation: 0 }),
        );
        let obj = Object::Dictionary(dict);
        let result = parse_marked_content_ref(&doc, &obj, &page_map, 0).unwrap();
        if let Some(StructChild::MarkedContentRef { mcid, page, scope }) = result {
            assert_eq!(mcid, 3);
            assert_eq!(page, 2);
            assert_eq!(scope, crate::structure::McidScope::Page(2));
        } else {
            panic!("Expected MarkedContentRef");
        }
    }

    #[test]
    fn test_parse_structure_tree_untagged_pdf() {
        let pdf = build_test_pdf();
        let doc = PdfDocument::from_bytes(pdf).unwrap();
        let result = parse_structure_tree(&doc).unwrap();
        assert!(result.is_none());
    }

    /// Build a minimal PDF for testing
    fn build_test_pdf() -> Vec<u8> {
        let mut pdf = b"%PDF-1.7\n".to_vec();
        let off1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let off2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
        let off3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n");

        let xref_offset = pdf.len();
        pdf.extend_from_slice(b"xref\n0 4\n");
        pdf.extend_from_slice(b"0000000000 65535 f \r\n");
        pdf.extend_from_slice(format!("{:010} 00000 n \r\n", off1).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \r\n", off2).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \r\n", off3).as_bytes());
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                xref_offset
            )
            .as_bytes(),
        );
        pdf
    }
}
