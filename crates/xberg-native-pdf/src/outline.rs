//! PDF document outline (bookmarks) support.
//!
//! Provides access to the PDF document outline, also known as bookmarks,
//! which allow users to navigate a PDF document.

use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::{Object, ObjectRef};
use std::collections::HashSet;

/// Maximum outline nesting followed; deeper children are dropped.
const OUTLINE_MAX_DEPTH: usize = 64;

/// Maximum outline items visited in one walk, however the /First and /Next
/// links are wired.
const OUTLINE_MAX_ITEMS: usize = 100_000;

/// Maximum name-tree nodes visited in one lookup or collection; a wide
/// malformed tree cannot force unbounded work either.
pub(crate) const NAME_TREE_MAX_NODES: usize = 500;

/// Visit budget for a name-tree walk: each indirect node is entered at most
/// once and the total node count is capped. ~keep
#[derive(Default)]
pub(crate) struct NameTreeBudget {
    visited: HashSet<ObjectRef>,
    nodes: usize,
}

impl NameTreeBudget {
    pub(crate) fn admit(&mut self, node: &Object) -> bool {
        if self.nodes >= NAME_TREE_MAX_NODES {
            return false;
        }
        if let Object::Reference(r) = node
            && !self.visited.insert(*r)
        {
            return false;
        }
        self.nodes += 1;
        true
    }
}

/// Visit budget for an outline walk: each item is entered at most once.
#[derive(Default)]
struct OutlineWalk {
    visited: HashSet<ObjectRef>,
}

impl OutlineWalk {
    fn admit(&mut self, item: ObjectRef) -> bool {
        self.visited.len() < OUTLINE_MAX_ITEMS && self.visited.insert(item)
    }
}

/// A single outline item (bookmark) in the document hierarchy.
#[derive(Debug, Clone)]
pub struct OutlineItem {
    /// The title of this bookmark
    pub title: String,

    /// The destination (page number or named destination)
    /// None if destination cannot be determined
    pub dest: Option<Destination>,

    /// Child bookmarks under this item
    pub children: Vec<OutlineItem>,
}

/// Destination in the PDF
#[derive(Debug, Clone)]
pub enum Destination {
    /// Direct page reference (page index, 0-based)
    PageIndex(usize),

    /// Named destination (string identifier)
    Named(String),
}

impl PdfDocument {
    /// Get the document outline (bookmarks) if present.
    ///
    /// Returns a hierarchical structure of bookmarks that can be used
    /// for document navigation.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(Vec<OutlineItem>))` - Bookmarks found and parsed
    /// - `Ok(None)` - No bookmarks in document
    /// - `Err` - Error parsing bookmarks
    ///
    /// # Example
    ///
    /// ```no_run
    /// use xberg_native_pdf::PdfDocument;
    ///
    /// let mut doc = PdfDocument::open("sample.pdf")?;
    /// if let Some(outline) = doc.get_outline()? {
    ///     for item in outline {
    ///         println!("Bookmark: {}", item.title);
    ///     }
    /// }
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn get_outline(&self) -> Result<Option<Vec<OutlineItem>>> {
        let catalog = self.catalog()?;

        let outlines_ref = match catalog.as_dict() {
            Some(dict) => match dict.get("Outlines") {
                Some(Object::Reference(obj_ref)) => *obj_ref,
                _ => return Ok(None),
            },
            None => return Ok(None),
        };

        let outlines_dict = self.load_object(outlines_ref)?;

        let first_ref = match outlines_dict.as_dict() {
            Some(dict) => match dict.get("First") {
                Some(Object::Reference(obj_ref)) => *obj_ref,
                _ => return Ok(None),
            },
            None => return Ok(None),
        };

        let mut items = Vec::new();
        let mut walk = OutlineWalk::default();
        let mut current_ref = Some(first_ref);

        while let Some(item_ref) = current_ref {
            if !walk.admit(item_ref) {
                break;
            }
            if let Ok(item) = self.parse_outline_item(item_ref, 0, &mut walk) {
                items.push(item);
            }

            let item_obj = self.load_object(item_ref)?;
            current_ref = match item_obj.as_dict() {
                Some(dict) => match dict.get("Next") {
                    Some(Object::Reference(obj_ref)) => Some(*obj_ref),
                    _ => None,
                },
                None => None,
            };
        }

        if items.is_empty() { Ok(None) } else { Ok(Some(items)) }
    }

    /// Parse a single outline item and its children recursively.
    fn parse_outline_item(&self, item_ref: ObjectRef, depth: usize, walk: &mut OutlineWalk) -> Result<OutlineItem> {
        let item_obj = self.load_object(item_ref)?;

        // Extract title. `/Title` is a PDF text string (ISO 32000-1
        // §7.9.2.2): UTF-16BE/LE prefixed with a BOM, or PDFDocEncoding
        // for the BOM-less form. hyperref emits UTF-16BE-with-BOM by
        // default, so a bare `String::from_utf8_lossy` mangles it —
        // route every case through the shared text-string decoder. The
        // value may also be given as an indirect reference. ~keep
        let title = match item_obj.as_dict().and_then(|dict| dict.get("Title")) {
            Some(Object::String(s)) => crate::optional_content::decode_pdf_text_string(s),
            Some(Object::Reference(r)) => self
                .load_object(*r)
                .ok()
                .as_ref()
                .and_then(Object::as_string)
                .map(crate::optional_content::decode_pdf_text_string)
                .unwrap_or_else(|| "(No Title)".to_string()),
            _ => "(No Title)".to_string(),
        };

        let dest = self.parse_outline_destination(&item_obj)?;

        let mut children = Vec::new();
        if depth < OUTLINE_MAX_DEPTH
            && let Some(dict) = item_obj.as_dict()
            && let Some(Object::Reference(first_child_ref)) = dict.get("First")
        {
            let mut child_ref = Some(*first_child_ref);

            while let Some(c_ref) = child_ref {
                if !walk.admit(c_ref) {
                    break;
                }
                if let Ok(child) = self.parse_outline_item(c_ref, depth + 1, walk) {
                    children.push(child);
                }

                let child_obj = self.load_object(c_ref)?;
                child_ref = match child_obj.as_dict() {
                    Some(dict) => match dict.get("Next") {
                        Some(Object::Reference(obj_ref)) => Some(*obj_ref),
                        _ => None,
                    },
                    None => None,
                };
            }
        }

        Ok(OutlineItem { title, dest, children })
    }

    /// Parse destination from an outline item.
    fn parse_outline_destination(&self, item: &Object) -> Result<Option<Destination>> {
        let dict = match item.as_dict() {
            Some(d) => d,
            None => return Ok(None),
        };

        if let Some(dest_obj) = dict.get("Dest") {
            return self.resolve_destination(dest_obj);
        }

        let mut action = dict.get("A");

        let obj;
        if let Some(Object::Reference(obj_ref)) = action {
            obj = self.load_object(*obj_ref)?;
            action = Some(&obj);
        }

        if let Some(Object::Dictionary(action)) = action
            && let Some(dest_obj) = action.get("D")
        {
            return self.resolve_destination(dest_obj);
        }

        Ok(None)
    }

    /// Resolve a *named* destination (PDF 1.1 catalog `/Dests` dict, or
    /// PDF 1.2+ `/Names` → `/Dests` name tree, ISO 32000-1 §12.3.2.3 /
    /// §7.9.6) to a 0-based page index. `Ok(None)` when the name is
    /// genuinely unresolvable (caller keeps [`Destination::Named`]).
    ///
    /// Named destinations are referenced by string from Link/GoTo actions and
    /// outline items; a consumer that follows those references needs to turn a
    /// name into the page it targets.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use xberg_native_pdf::PdfDocument;
    ///
    /// let doc = PdfDocument::open("doc.pdf")?;
    /// if let Some(page) = doc.resolve_named_destination("section.1")? {
    ///     println!("named destination -> page {page}");
    /// }
    /// # Ok::<(), xberg_native_pdf::error::Error>(())
    /// ```
    pub fn resolve_named_destination(&self, name: &str) -> Result<Option<usize>> {
        self.resolve_named_destination_bytes(name.as_bytes())
    }

    /// Byte-based counterpart of [`resolve_named_destination`]. A
    /// `/Dest` name-tree key is a byte string ordered and compared *by
    /// byte* (ISO 32000-1 §7.9.6) — it is not a text string. A caller
    /// that already holds the raw key bytes (e.g. from an
    /// `Object::String`, which may be UTF-16BE-with-BOM — what Adobe
    /// Distiller emits) must look up with those bytes directly rather
    /// than decoding to `&str` first: for a byte sequence that is not
    /// valid UTF-8, `resolve_named_destination`'s lossy decode-then-
    /// re-encode mangles the key (e.g. the `FE FF` BOM becomes two
    /// U+FFFD replacement characters) and the lookup can never match. ~keep
    fn resolve_named_destination_bytes(&self, name: &[u8]) -> Result<Option<usize>> {
        let catalog = self.catalog()?;
        let Some(cat) = catalog.as_dict() else {
            return Ok(None);
        };
        let resolve = |r: crate::object::ObjectRef| self.load_object(r).ok();
        let Some(dest) = lookup_named_dest(cat, name, &resolve, 0) else {
            return Ok(None);
        };
        // The found value is a dest array (or was already normalised
        // from a `<< /D [...] >>` wrapper). Reuse the array path. ~keep
        match self.resolve_destination(&dest)? {
            Some(Destination::PageIndex(i)) => Ok(Some(i)),
            _ => Ok(None),
        }
    }

    /// Resolve a destination object to a Destination enum.
    fn resolve_destination(&self, dest_obj: &Object) -> Result<Option<Destination>> {
        match dest_obj {
            // Named destination — a byte string (`/Dest (name)`) or a
            // name object (`/Dest /name`). Resolve via the catalog
            // /Dests dict / /Names name tree; fall back to the
            // unresolved name for backward compatibility (the
            // `bookmarks` JSON still prints names when unresolvable).
            // The name-tree key is looked up by its RAW bytes — it is
            // not a text string (ISO 32000-1 §7.9.6) — so a UTF-16BE-
            // with-BOM key (Adobe Distiller's default) still resolves;
            // only the *display* fallback goes through a lossy decode. ~keep
            Object::String(name) => match self.resolve_named_destination_bytes(name)? {
                Some(idx) => Ok(Some(Destination::PageIndex(idx))),
                None => Ok(Some(Destination::Named(String::from_utf8_lossy(name).to_string()))),
            },
            Object::Name(name) => match self.resolve_named_destination(name)? {
                Some(idx) => Ok(Some(Destination::PageIndex(idx))),
                None => Ok(Some(Destination::Named(name.clone()))),
            },

            Object::Array(arr) if !arr.is_empty() => match &arr[0] {
                Object::Reference(page_ref) => {
                    if let Ok(page_index) = self.find_page_index(*page_ref) {
                        Ok(Some(Destination::PageIndex(page_index)))
                    } else {
                        Ok(None)
                    }
                }
                _ => Ok(None),
            },

            Object::Reference(dest_ref) => {
                let resolved = self.load_object(*dest_ref)?;
                self.resolve_destination(&resolved)
            }

            _ => Ok(None),
        }
    }

    /// Find the page index for a given page object reference.
    fn find_page_index(&self, page_ref: crate::object::ObjectRef) -> Result<usize> {
        // Single tree walk; otherwise per-call get_page_ref(i) is O(n) and the
        // outer loop becomes O(n²) — outlines with many bookmarks turn into n³. ~keep
        let refs = self.all_page_refs()?;
        refs.iter()
            .position(|r| *r == page_ref)
            .ok_or_else(|| Error::InvalidPdf(format!("Page reference {:?} not found", page_ref)))
    }
}

/// Max name-tree descent — guards malformed/cyclic `/Kids` trees from
/// unbounded recursion (foundation §6.3 untrusted-input limits).
const NAME_TREE_MAX_DEPTH: u8 = 32;

/// Follow one level of indirection. Pure given `resolve`.
fn deref_obj(o: &Object, resolve: &dyn Fn(crate::object::ObjectRef) -> Option<Object>) -> Option<Object> {
    match o {
        Object::Reference(r) => resolve(*r),
        other => Some(other.clone()),
    }
}

/// A name-tree / `/Dests` value is either the destination **array**
/// directly, or a dictionary with a `/D` entry holding it
/// (ISO 32000-1 §12.3.2.3). Normalise to the array object.
fn normalize_dest_value(v: &Object, resolve: &dyn Fn(crate::object::ObjectRef) -> Option<Object>) -> Option<Object> {
    let v = deref_obj(v, resolve)?;
    if let Some(d) = v.as_dict()
        && let Some(inner) = d.get("D")
    {
        return deref_obj(inner, resolve);
    }
    if v.as_array().is_some() {
        return Some(v);
    }
    None
}

/// Walk a name-tree node (`/Names` leaf or `/Kids` intermediate),
/// `/Limits`-guided per ISO 32000-1 §7.9.6. Pure given `resolve`.
fn walk_name_tree(
    node: &Object,
    target: &[u8],
    resolve: &dyn Fn(crate::object::ObjectRef) -> Option<Object>,
    depth: u8,
    budget: &mut NameTreeBudget,
) -> Option<Object> {
    if depth > NAME_TREE_MAX_DEPTH {
        return None;
    }
    let node = deref_obj(node, resolve)?;
    let dict = node.as_dict()?;

    if let Some(names) = dict
        .get("Names")
        .and_then(|n| deref_obj(n, resolve))
        .filter(|d| d.as_array().is_some())
    {
        let arr = names.as_array().expect("checked array above");
        let mut i = 0;
        while i + 1 < arr.len() {
            if arr[i].as_string() == Some(target) {
                return normalize_dest_value(&arr[i + 1], resolve);
            }
            i += 2;
        }
        // A pure-leaf node with no match is terminal. ~keep
        if !dict.contains_key("Kids") {
            return None;
        }
    }

    if let Some(kids) = dict
        .get("Kids")
        .and_then(|k| deref_obj(k, resolve))
        .filter(|k| k.as_array().is_some())
    {
        for kid in kids.as_array().expect("checked array above") {
            if !budget.admit(kid) {
                continue;
            }
            let Some(kid_node) = deref_obj(kid, resolve) else {
                continue;
            };
            if kid_limits_admit(&kid_node, target, resolve)
                && let Some(found) = walk_name_tree(&kid_node, target, resolve, depth + 1, budget)
            {
                return Some(found);
            }
        }
    }
    None
}

/// Whether `target` can lie under this name-tree kid, per its `/Limits`
/// entry. A missing, unresolvable or malformed `/Limits` admits the kid, so
/// the search stays robust against damaged trees. ~keep
fn kid_limits_admit(
    kid_node: &Object,
    target: &[u8],
    resolve: &dyn Fn(crate::object::ObjectRef) -> Option<Object>,
) -> bool {
    let Some(lim) = kid_node.as_dict().and_then(|d| d.get("Limits")) else {
        return true;
    };
    let Some(limits) = deref_obj(lim, resolve) else {
        return true;
    };
    match limits.as_array() {
        Some(l) if l.len() == 2 => match (l[0].as_string(), l[1].as_string()) {
            (Some(lo), Some(hi)) => lo <= target && target <= hi,
            _ => true,
        },
        _ => true,
    }
}

/// Resolve `target` to its destination object via the catalog
/// `/Dests` dictionary (PDF 1.1) then the `/Names` → `/Dests` name
/// tree (PDF 1.2+). Pure given `resolve`; bounded; returns the
/// normalised destination array object, or `None`.
fn lookup_named_dest(
    catalog: &std::collections::HashMap<String, Object>,
    target: &[u8],
    resolve: &dyn Fn(crate::object::ObjectRef) -> Option<Object>,
    depth: u8,
) -> Option<Object> {
    if let Some(dests) = catalog.get("Dests").and_then(|d| deref_obj(d, resolve))
        && let Some(dd) = dests.as_dict()
    {
        let key = String::from_utf8_lossy(target);
        if let Some(v) = dd.get(key.as_ref())
            && let Some(found) = normalize_dest_value(v, resolve)
        {
            return Some(found);
        }
    }
    let names = catalog.get("Names").and_then(|n| deref_obj(n, resolve))?;
    let root = names.as_dict()?.get("Dests").and_then(|d| deref_obj(d, resolve))?;
    walk_name_tree(&root, target, resolve, depth, &mut NameTreeBudget::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn s(b: &str) -> Object {
        Object::String(b.as_bytes().to_vec())
    }
    fn arr_dest(page_obj: u32) -> Object {
        Object::Array(vec![
            Object::Reference(crate::object::ObjectRef::new(page_obj, 0)),
            Object::Name("Fit".to_string()),
        ])
    }
    fn no_resolve() -> impl Fn(crate::object::ObjectRef) -> Option<Object> {
        |_r| None
    }

    #[test]
    fn test_outline_item_creation() {
        let item = OutlineItem {
            title: "Chapter 1".to_string(),
            dest: Some(Destination::PageIndex(0)),
            children: vec![],
        };

        assert_eq!(item.title, "Chapter 1");
        assert!(matches!(item.dest, Some(Destination::PageIndex(0))));
        assert!(item.children.is_empty());
    }

    #[test]
    fn test_outline_hierarchy() {
        let child = OutlineItem {
            title: "Section 1.1".to_string(),
            dest: Some(Destination::PageIndex(1)),
            children: vec![],
        };

        let parent = OutlineItem {
            title: "Chapter 1".to_string(),
            dest: Some(Destination::PageIndex(0)),
            children: vec![child],
        };

        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0].title, "Section 1.1");
    }

    #[test]
    fn name_tree_repeated_kid_is_not_resolved_again() {
        let node = crate::object::ObjectRef::new(7, 0);
        let mut dict = HashMap::new();
        dict.insert("Kids".to_string(), Object::Array(vec![Object::Reference(node); 1_000]));
        let root = Object::Dictionary(dict);
        let resolved = std::cell::Cell::new(0usize);
        let resolve = |r: crate::object::ObjectRef| {
            resolved.set(resolved.get() + 1);
            (r == node).then(|| root.clone())
        };
        let found = walk_name_tree(
            &Object::Reference(node),
            b"x",
            &resolve,
            0,
            &mut NameTreeBudget::default(),
        );
        assert!(found.is_none());
        assert!(resolved.get() <= 4, "resolved {} times", resolved.get());
    }

    #[test]
    fn name_tree_range_rejected_repeated_kid_is_not_resolved_again() {
        let node = crate::object::ObjectRef::new(7, 0);
        let root_ref = crate::object::ObjectRef::new(1, 0);
        let mut kid = HashMap::new();
        kid.insert(
            "Limits".to_string(),
            Object::Array(vec![Object::String(b"a".to_vec()), Object::String(b"b".to_vec())]),
        );
        let kid = Object::Dictionary(kid);
        let mut root = HashMap::new();
        root.insert("Kids".to_string(), Object::Array(vec![Object::Reference(node); 1_000]));
        let root = Object::Dictionary(root);
        let resolved = std::cell::Cell::new(0usize);
        let resolve = |r: crate::object::ObjectRef| {
            resolved.set(resolved.get() + 1);
            if r == node {
                Some(kid.clone())
            } else if r == root_ref {
                Some(root.clone())
            } else {
                None
            }
        };
        let found = walk_name_tree(
            &Object::Reference(root_ref),
            b"x",
            &resolve,
            0,
            &mut NameTreeBudget::default(),
        );
        assert!(found.is_none());
        assert!(resolved.get() <= 3, "resolved {} times", resolved.get());
    }

    /// Catalog `/Dests` dict, value is the destination array directly.
    #[test]
    fn dests_dict_direct_array() {
        let mut dests = HashMap::new();
        dests.insert("chap1".to_string(), arr_dest(7));
        let mut cat = HashMap::new();
        cat.insert("Dests".to_string(), Object::Dictionary(dests));
        let r = no_resolve();
        let got = lookup_named_dest(&cat, b"chap1", &r, 0).expect("found");
        assert_eq!(got.as_array().unwrap()[0].as_reference().unwrap().id, 7);
        assert!(lookup_named_dest(&cat, b"missing", &r, 0).is_none());
    }

    /// `/Dests` value is a `<< /D [...] >>` wrapper (must unwrap /D).
    #[test]
    fn dests_dict_d_wrapper() {
        let mut inner = HashMap::new();
        inner.insert("D".to_string(), arr_dest(9));
        let mut dests = HashMap::new();
        dests.insert("c".to_string(), Object::Dictionary(inner));
        let mut cat = HashMap::new();
        cat.insert("Dests".to_string(), Object::Dictionary(dests));
        let got = lookup_named_dest(&cat, b"c", &no_resolve(), 0).expect("found");
        assert_eq!(got.as_array().unwrap()[0].as_reference().unwrap().id, 9);
    }

    /// `/Names` → `/Dests` flat-`/Names`-array leaf.
    #[test]
    fn names_tree_flat_leaf() {
        let dests_root = Object::Dictionary(HashMap::from([(
            "Names".to_string(),
            Object::Array(vec![s("a"), arr_dest(1), s("b"), arr_dest(4), s("c"), arr_dest(6)]),
        )]));
        let names = Object::Dictionary(HashMap::from([("Dests".to_string(), dests_root)]));
        let cat = HashMap::from([("Names".to_string(), names)]);
        let r = no_resolve();
        assert_eq!(
            lookup_named_dest(&cat, b"b", &r, 0).unwrap().as_array().unwrap()[0]
                .as_reference()
                .unwrap()
                .id,
            4
        );
        assert!(lookup_named_dest(&cat, b"z", &r, 0).is_none());
    }

    /// `/Kids` intermediate node with `/Limits`-guided descent, the
    /// child nodes reached through indirect references (exercises the
    /// injected `resolve`).
    #[test]
    fn names_tree_kids_with_limits_and_indirection() {
        use crate::object::ObjectRef;
        let leaf_l = Object::Dictionary(HashMap::from([
            ("Limits".to_string(), Object::Array(vec![s("a"), s("b")])),
            (
                "Names".to_string(),
                Object::Array(vec![s("a"), arr_dest(1), s("b"), arr_dest(2)]),
            ),
        ]));
        let leaf_r = Object::Dictionary(HashMap::from([
            ("Limits".to_string(), Object::Array(vec![s("m"), s("z")])),
            (
                "Names".to_string(),
                Object::Array(vec![s("m"), arr_dest(3), s("z"), arr_dest(4)]),
            ),
        ]));
        let resolve = move |rf: ObjectRef| match rf.id {
            100 => Some(leaf_l.clone()),
            101 => Some(leaf_r.clone()),
            _ => None,
        };
        let root = Object::Dictionary(HashMap::from([(
            "Kids".to_string(),
            Object::Array(vec![
                Object::Reference(ObjectRef::new(100, 0)),
                Object::Reference(ObjectRef::new(101, 0)),
            ]),
        )]));
        let names = Object::Dictionary(HashMap::from([("Dests".to_string(), root)]));
        let cat = HashMap::from([("Names".to_string(), names)]);
        assert_eq!(
            lookup_named_dest(&cat, b"z", &resolve, 0).unwrap().as_array().unwrap()[0]
                .as_reference()
                .unwrap()
                .id,
            4
        );
        assert_eq!(
            lookup_named_dest(&cat, b"a", &resolve, 0).unwrap().as_array().unwrap()[0]
                .as_reference()
                .unwrap()
                .id,
            1
        );
        assert!(lookup_named_dest(&cat, b"c", &resolve, 0).is_none());
    }

    /// Cyclic `/Kids` must terminate via the depth guard (no panic /
    /// stack overflow) — adversarial-input safety (foundation §6.3).
    #[test]
    fn names_tree_cyclic_kids_is_bounded() {
        use crate::object::ObjectRef;
        let resolve = |rf: ObjectRef| {
            if rf.id == 200 {
                Some(Object::Dictionary(HashMap::from([(
                    "Kids".to_string(),
                    Object::Array(vec![Object::Reference(ObjectRef::new(200, 0))]),
                )])))
            } else {
                None
            }
        };
        let names = Object::Dictionary(HashMap::from([(
            "Dests".to_string(),
            Object::Reference(ObjectRef::new(200, 0)),
        )]));
        let cat = HashMap::from([("Names".to_string(), names)]);
        assert!(lookup_named_dest(&cat, b"x", &resolve, 0).is_none());
    }

    /// No /Dests and no /Names → cleanly None.
    #[test]
    fn no_dests_no_names_is_none() {
        let cat = HashMap::from([("Type".to_string(), Object::Name("Catalog".to_string()))]);
        assert!(lookup_named_dest(&cat, b"anything", &no_resolve(), 0).is_none());
    }

    /// GH #1589 root cause: a name-tree key is a byte string ordered
    /// and compared *by byte* (ISO 32000-1 §7.9.6), not a text
    /// string. A UTF-16BE-with-BOM key (what Adobe Distiller emits)
    /// resolves fine when looked up by its raw bytes — `lookup_named_dest`
    /// / `walk_name_tree` were never at fault — but the OLD
    /// `resolve_destination` decoded the `/Dest` byte string lossily to
    /// a `String` *before* calling into the lookup, then re-encoded
    /// that `String` back to bytes for the search. For a BOM-prefixed
    /// UTF-16BE key that round trip mangles `FE FF` into two U+FFFD
    /// replacement characters (`EF BF BD` each in UTF-8) and the
    /// lookup can never match — every named destination in such a
    /// document silently resolved to `None`. ~keep
    #[test]
    fn utf16be_key_requires_raw_byte_lookup_not_lossy_roundtrip() {
        // UTF-16BE-with-BOM encoding of the two-character name "A3".
        let utf16_key: &[u8] = &[0xFE, 0xFF, 0x00, 0x41, 0x00, 0x33];
        let dests_root = Object::Dictionary(HashMap::from([(
            "Names".to_string(),
            Object::Array(vec![Object::String(utf16_key.to_vec()), arr_dest(3)]),
        )]));
        let names = Object::Dictionary(HashMap::from([("Dests".to_string(), dests_root)]));
        let cat = HashMap::from([("Names".to_string(), names)]);
        let r = no_resolve();

        // Correct: raw bytes find the entry (proves walk_name_tree /
        // lookup_named_dest are byte-correct, as the reporter claimed).
        let found = lookup_named_dest(&cat, utf16_key, &r, 0).expect("raw bytes must resolve");
        assert_eq!(found.as_array().unwrap()[0].as_reference().unwrap().id, 3);

        // What the pre-fix `resolve_destination` effectively looked up:
        // the name-tree key round-tripped through a lossy UTF-8 decode.
        let lossy_roundtrip = String::from_utf8_lossy(utf16_key).to_string().into_bytes();
        assert_eq!(
            lossy_roundtrip,
            vec![0xEF, 0xBF, 0xBD, 0xEF, 0xBF, 0xBD, 0x00, 0x41, 0x00, 0x33],
            "sanity: the lossy round-trip must mangle the BOM exactly as reported"
        );
        assert!(lookup_named_dest(&cat, &lossy_roundtrip, &r, 0).is_none());
    }
}
