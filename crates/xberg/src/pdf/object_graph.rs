//! A read-only view of a PDF's object graph in lopdf's object model.
//!
//! The outline and revision readers were written against `lopdf::Document`, which parses the
//! whole file. Reading through [`ObjectGraph`] lets them run over the native engine's document,
//! which extraction has already opened, while the helpers here keep lopdf's resolution rules
//! (dereference limit, page-tree walk, catalog lookup) so both sources yield the same result.

use std::borrow::Cow;

use lopdf::{Dictionary, Object, ObjectId};

/// lopdf's `Document::DEREF_LIMIT`: how many references one dereference follows.
const DEREF_LIMIT: usize = 128;

/// lopdf's `PageTreeIter::PAGE_TREE_DEPTH_LIMIT`.
const PAGE_TREE_DEPTH_LIMIT: usize = 256;

/// lopdf looks for the final `%%EOF` within this many bytes of the end of the file.
const EOF_SEARCH_WINDOW: usize = 1024;

/// lopdf requires `startxref` to begin within this many bytes before that `%%EOF`.
const STARTXREF_SEARCH_WINDOW: usize = 25;

/// Objects of one PDF, in lopdf's object model.
pub(crate) trait ObjectGraph {
    /// The trailer entry `key`, not dereferenced.
    fn trailer_entry(&self, key: &[u8]) -> Option<Object>;
    /// The object stored under `id`, not dereferenced; `None` when there is none.
    fn stored_object(&self, id: ObjectId) -> Option<Object>;
    /// How many objects the document holds, which bounds the page-tree walk as in lopdf.
    fn object_count(&self) -> usize;
    /// The offset lopdf records as `Document::xref_start`.
    fn xref_start(&self) -> usize;
}

#[cfg(test)]
impl ObjectGraph for lopdf::Document {
    fn trailer_entry(&self, key: &[u8]) -> Option<Object> {
        self.trailer.get(key).ok().cloned()
    }

    fn stored_object(&self, id: ObjectId) -> Option<Object> {
        self.objects.get(&id).cloned()
    }

    fn object_count(&self) -> usize {
        self.objects.len()
    }

    fn xref_start(&self) -> usize {
        self.xref_start
    }
}

/// The native engine's open document seen as an [`ObjectGraph`].
///
/// Objects come from its object cache, so reading them costs no second parse of the file. A
/// missing object loads as `Null` there where lopdf reports an error; every reader in this crate
/// treats the two alike, since neither is a dictionary, array, string, name or reference.
pub(crate) struct NativeObjectGraph<'a> {
    document: &'a xberg_native_pdf::PdfDocument,
}

impl<'a> NativeObjectGraph<'a> {
    pub(crate) fn new(document: &'a xberg_native_pdf::PdfDocument) -> Self {
        Self { document }
    }
}

impl ObjectGraph for NativeObjectGraph<'_> {
    fn trailer_entry(&self, key: &[u8]) -> Option<Object> {
        let key = std::str::from_utf8(key).ok()?;
        self.document.trailer().as_dict()?.get(key).cloned().map(to_lopdf)
    }

    fn stored_object(&self, (number, generation): ObjectId) -> Option<Object> {
        let reference = xberg_native_pdf::object::ObjectRef::new(number, generation);
        self.document.load_object(reference).ok().map(to_lopdf)
    }

    fn object_count(&self) -> usize {
        self.document.all_object_ids().len()
    }

    fn xref_start(&self) -> usize {
        lopdf_xref_start(&self.document.source_bytes)
    }
}

/// `object` in lopdf's model. A stream becomes `Null`: no reader here looks at stream data, and
/// a stream is no dictionary to lopdf's `as_dict` either, so this skips copying its payload.
/// Dictionary entries are ordered by key, as the native dictionary has no order of its own.
fn to_lopdf(object: xberg_native_pdf::object::Object) -> Object {
    use xberg_native_pdf::object::Object as Native;
    match object {
        Native::Null | Native::Stream { .. } => Object::Null,
        Native::Boolean(value) => Object::Boolean(value),
        Native::Integer(value) => Object::Integer(value),
        Native::Real(value) => Object::Real(value as f32),
        Native::String(bytes) => Object::String(bytes, lopdf::StringFormat::Literal),
        Native::Name(name) => Object::Name(name.into_bytes()),
        Native::Array(items) => Object::Array(items.into_iter().map(to_lopdf).collect()),
        Native::Dictionary(entries) => {
            let mut entries: Vec<_> = entries.into_iter().collect();
            entries.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
            Object::Dictionary(
                entries
                    .into_iter()
                    .map(|(key, value)| (key.into_bytes(), to_lopdf(value)))
                    .collect(),
            )
        }
        Native::Reference(reference) => Object::Reference((reference.id, reference.generation)),
    }
}

/// lopdf's `Document::dereference`: follow references from `object`, returning the last
/// reference followed and the object it names. `None` where lopdf errs.
pub(crate) fn dereference<'a, G: ObjectGraph + ?Sized>(
    graph: &G,
    object: &'a Object,
) -> Option<(Option<ObjectId>, Cow<'a, Object>)> {
    let mut id = None;
    let mut current = Cow::Borrowed(object);
    let mut hops = 0usize;
    while let Ok(reference) = current.as_reference() {
        id = Some(reference);
        current = Cow::Owned(graph.stored_object(reference)?);
        hops += 1;
        if hops > DEREF_LIMIT {
            return None;
        }
    }
    Some((id, current))
}

/// lopdf's `Document::get_object`: the object `id` names, dereferenced.
pub(crate) fn get_object<G: ObjectGraph + ?Sized>(graph: &G, id: ObjectId) -> Option<Object> {
    let stored = graph.stored_object(id)?;
    if stored.as_reference().is_err() {
        return Some(stored);
    }
    dereference(graph, &stored).map(|(_, object)| object.into_owned())
}

/// lopdf's `Document::get_dictionary`.
pub(crate) fn get_dictionary<G: ObjectGraph + ?Sized>(graph: &G, id: ObjectId) -> Option<Dictionary> {
    match get_object(graph, id)? {
        Object::Dictionary(dictionary) => Some(dictionary),
        _ => None,
    }
}

/// lopdf's `Document::catalog`: the dictionary the trailer's `/Root` reference names.
pub(crate) fn catalog<G: ObjectGraph + ?Sized>(graph: &G) -> Option<Dictionary> {
    let root = graph.trailer_entry(b"Root")?.as_reference().ok()?;
    get_dictionary(graph, root)
}

/// The kids of the page-tree node `id`, as lopdf's `PageTreeIter::kids` reads them.
fn page_tree_kids<G: ObjectGraph + ?Sized>(graph: &G, id: ObjectId) -> Vec<Object> {
    let Some(node) = get_dictionary(graph, id) else {
        return Vec::new();
    };
    let Ok(kids) = node.get(b"Kids") else {
        return Vec::new();
    };
    match dereference(graph, kids).map(|(_, kids)| kids.into_owned()) {
        Some(Object::Array(kids)) => kids,
        _ => Vec::new(),
    }
}

/// lopdf's `Document::page_iter`: page object ids in page order.
///
/// A kid that does not load as a dictionary with a type is kept as a stand-in page, as lopdf
/// keeps it, so later page numbers do not shift.
pub(crate) fn page_ids<G: ObjectGraph + ?Sized>(graph: &G) -> Vec<ObjectId> {
    let mut pages = Vec::new();
    let Some(root) = catalog(graph).and_then(|catalog| catalog.get(b"Pages").ok()?.as_reference().ok()) else {
        return pages;
    };
    let mut iter_limit = graph.object_count();
    let mut kids = page_tree_kids(graph, root).into_iter();
    let mut stack = Vec::new();
    loop {
        while let Some(kid) = kids.next() {
            if iter_limit == 0 {
                return pages;
            }
            iter_limit -= 1;
            let Ok(kid_id) = kid.as_reference() else {
                continue;
            };
            let Some(node) = get_dictionary(graph, kid_id) else {
                pages.push(kid_id);
                continue;
            };
            match node.get_type() {
                Ok(b"Page") | Err(_) => pages.push(kid_id),
                Ok(b"Pages") if stack.len() < PAGE_TREE_DEPTH_LIMIT => {
                    stack.push(std::mem::replace(&mut kids, page_tree_kids(graph, kid_id).into_iter()));
                }
                Ok(_) => {}
            }
        }
        match stack.pop() {
            Some(parent) => kids = parent,
            None => return pages,
        }
    }
}

/// The offset lopdf records as `xref_start` for `content`: the number after the `startxref`
/// that just precedes the final `%%EOF`, or 0, which lopdf records when it has to rebuild the
/// cross-reference table instead.
///
/// lopdf also moves an offset that misses its section by a few bytes onto the section; the
/// revision walk that reads this offset scans forward to the section's trailer either way.
pub(crate) fn lopdf_xref_start(content: &[u8]) -> usize {
    let start = memchr::memmem::find(content, b"%PDF-").unwrap_or(0);
    let buffer = &content[start..];
    declared_xref_start(buffer)
        .filter(|&offset| offset <= buffer.len())
        .unwrap_or(0)
}

fn declared_xref_start(buffer: &[u8]) -> Option<usize> {
    let end = buffer
        .iter()
        .rposition(|byte| !b" \t\n\r\0\x0C".contains(byte))
        .map_or(0, |pos| pos + 1);
    let eof = rfind_from(&buffer[..end], b"%%EOF", end.saturating_sub(EOF_SEARCH_WINDOW))
        .filter(|&pos| pos > STARTXREF_SEARCH_WINDOW)?;
    let keyword = rfind_from(&buffer[..eof], b"startxref", eof - STARTXREF_SEARCH_WINDOW)?;
    parse_startxref(&buffer[keyword..])
}

/// The last occurrence of `pattern` in `buffer` that starts at or after `start`.
fn rfind_from(buffer: &[u8], pattern: &[u8], start: usize) -> Option<usize> {
    buffer
        .get(start..)?
        .windows(pattern.len())
        .rposition(|window| window == pattern)
        .map(|pos| start + pos)
}

/// lopdf's `parser::xref_start`: `startxref`, an optional space, an end of line, the offset
/// between spaces, an end of line and `%%EOF`. A negative offset is no offset.
fn parse_startxref(input: &[u8]) -> Option<usize> {
    let input = input.strip_prefix(b"startxref")?;
    let input = input.strip_prefix(b" ").unwrap_or(input);
    let input = strip_eol(input)?;
    let input = trim_spaces_start(input);
    let (negative, input) = match input.split_first() {
        Some((b'-', rest)) => (true, rest),
        Some((b'+', rest)) => (false, rest),
        _ => (false, input),
    };
    let digits = input.iter().take_while(|byte| byte.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let value = std::str::from_utf8(&input[..digits]).ok()?.parse::<u64>().ok()?;
    let input = trim_spaces_start(&input[digits..]);
    strip_eol(input)?.strip_prefix(b"%%EOF")?;
    if negative && value != 0 {
        return None;
    }
    usize::try_from(i64::try_from(value).ok()?).ok()
}

fn strip_eol(input: &[u8]) -> Option<&[u8]> {
    input
        .strip_prefix(b"\r\n")
        .or_else(|| input.strip_prefix(b"\n"))
        .or_else(|| input.strip_prefix(b"\r"))
}

fn trim_spaces_start(input: &[u8]) -> &[u8] {
    let spaces = input.iter().take_while(|&&byte| byte == b' ').count();
    &input[spaces..]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xref_start_reads_the_offset_before_the_final_eof() {
        let pdf = b"%PDF-1.4\nbody\nstartxref\n9\n%%EOF\n\0\0";
        assert_eq!(lopdf_xref_start(pdf), 9);
        let mut long = b"%PDF-1.4\n".to_vec();
        long.resize(4096, b' ');
        long.extend_from_slice(b"\nstartxref\r\n  2048 \r\n%%EOF\r\n");
        assert_eq!(lopdf_xref_start(&long), 2048);
    }

    #[test]
    fn xref_start_is_zero_without_a_readable_startxref() {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        pdf.resize(200, b' ');
        assert_eq!(lopdf_xref_start(&pdf), 0, "no %%EOF");
        let mut negative = pdf.clone();
        negative.extend_from_slice(b"\nstartxref\n-5\n%%EOF\n");
        assert_eq!(lopdf_xref_start(&negative), 0, "negative offset");
        let mut past_end = pdf.clone();
        past_end.extend_from_slice(b"\nstartxref\n999999\n%%EOF\n");
        assert_eq!(lopdf_xref_start(&past_end), 0, "offset past the end of the file");
    }

    #[test]
    fn xref_start_matches_lopdf_on_a_saved_document() {
        use lopdf::dictionary;

        let mut document = lopdf::Document::with_version("1.7");
        let pages = document.new_object_id();
        let page = document.add_object(dictionary! { "Type" => "Page", "Parent" => pages });
        document.objects.insert(
            pages,
            Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 }),
        );
        let catalog = document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        document.trailer.set("Root", catalog);
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).expect("save");

        let loaded = lopdf::Document::load_mem(&bytes).expect("load");
        assert_ne!(loaded.xref_start, 0);
        assert_eq!(lopdf_xref_start(&bytes), loaded.xref_start);
    }

    /// What extraction reports from a document's outline and update history.
    #[derive(Debug, PartialEq)]
    struct OutlineAndRevisions {
        entries: Vec<crate::pdf::bookmarks::PdfOutlineEntry>,
        uris: Vec<crate::types::ExtractedUri>,
        /// `DocumentRevision` has no `PartialEq`; its `Debug` form carries every field.
        revisions: Option<String>,
    }

    fn read_outline_and_revisions<G: ObjectGraph>(content: &[u8], graph: &G) -> OutlineAndRevisions {
        let entries = crate::pdf::bookmarks::extract_outline_entries(graph);
        let uris = crate::pdf::bookmarks::extract_bookmarks_from_entries(&entries);
        let revisions = crate::pdf::xref_revisions::extract_pdf_xref_revisions(content, graph)
            .map(|revisions| format!("{revisions:?}"));
        OutlineAndRevisions {
            entries,
            uris,
            revisions,
        }
    }

    /// The same document read by lopdf and through the native engine's open document, or `None`
    /// when either cannot open it.
    fn read_both_ways(bytes: &[u8]) -> Option<(OutlineAndRevisions, OutlineAndRevisions)> {
        let lopdf_document = lopdf::Document::load_mem(bytes).ok()?;
        let native = crate::pdf::native::NativeDocument::open_owned_bytes_with_passwords(
            bytes::Bytes::copy_from_slice(bytes),
            &[],
        )
        .ok()?;
        Some((
            read_outline_and_revisions(bytes, &lopdf_document),
            read_outline_and_revisions(&native.doc.source_bytes, &NativeObjectGraph::new(&native.doc)),
        ))
    }

    /// Corpus documents with outlines (direct, named and action destinations, items inside object
    /// streams, UTF-16 titles) and with incremental updates or linearization `/Prev` chains.
    const EQUIVALENCE_FIXTURES: &[&str] = &[
        "pdf/medium.pdf",
        "pdf/nougat_029.pdf",
        "pdf/nougat_026.pdf",
        "pdf/pdfa_031.pdf",
        "pdf/embedded_images_tables.pdf",
        "pdf/rtl_hebrew_native.pdf",
        "pdf/5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf",
        "pdf/a_comprehensive_study_of_convergent_and_commutative_replicated_data_types.pdf",
        "pdf/xerox_alta_link_series_mfp_sag_en_us_2.pdf",
    ];

    /// Extraction used to parse the whole file again with lopdf for the outline, its URIs and
    /// the revision list. Reading them from the native document must report exactly the same.
    #[test]
    fn native_graph_reports_the_outline_and_revisions_lopdf_reported() {
        let required = std::env::var_os("XBERG_REQUIRE_TEST_FIXTURES").is_some_and(|value| value == "1");
        let (mut compared, mut entries, mut uris, mut revisions) = (0usize, 0usize, 0usize, 0usize);
        for relative in EQUIVALENCE_FIXTURES {
            let Some(bytes) = crate::utils::read_test_fixture(relative) else {
                assert!(!required, "XBERG_REQUIRE_TEST_FIXTURES=1 but {relative} is missing");
                continue;
            };
            let (expected, actual) = read_both_ways(&bytes)
                .unwrap_or_else(|| panic!("{relative}: lopdf and the native engine must open it"));
            assert_eq!(actual, expected, "{relative}");
            compared += 1;
            entries += expected.entries.len();
            uris += expected.uris.len();
            revisions += usize::from(expected.revisions.is_some());
        }
        if compared < EQUIVALENCE_FIXTURES.len() {
            eprintln!(
                "SKIP: compared {compared} of {} outline/revision fixtures",
                EQUIVALENCE_FIXTURES.len()
            );
            return;
        }
        assert!(
            entries > 0,
            "no fixture produced an outline entry, so nothing was compared"
        );
        assert!(uris > 0, "no fixture produced an outline URI, so nothing was compared");
        assert!(revisions > 0, "no fixture produced a revision, so nothing was compared");
    }

    /// Every corpus PDF both parsers open: run with `--ignored` after changing either reader.
    #[test]
    #[ignore = "sweeps the whole corpus"]
    fn native_graph_matches_lopdf_across_the_corpus() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test_documents/pdf");
        let Ok(listing) = std::fs::read_dir(&root) else {
            assert!(
                std::env::var_os("XBERG_REQUIRE_TEST_FIXTURES").is_none_or(|value| value != "1"),
                "XBERG_REQUIRE_TEST_FIXTURES=1 but {} is missing",
                root.display()
            );
            eprintln!("SKIP: {} not available", root.display());
            return;
        };
        let mut paths: Vec<_> = listing
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|extension| extension == "pdf"))
            .collect();
        paths.sort();
        let (mut compared, mut mismatches) = (0usize, Vec::new());
        for path in &paths {
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            let Some((expected, actual)) = read_both_ways(&bytes) else {
                continue;
            };
            compared += 1;
            if actual != expected {
                mismatches.push(format!("{}: lopdf {expected:?}\nnative {actual:?}", path.display()));
            }
        }
        eprintln!("compared {compared} of {} corpus PDFs", paths.len());
        assert!(compared > 0, "no corpus PDF opened in both parsers");
        assert!(mismatches.is_empty(), "{}", mismatches.join("\n\n"));
    }
}
