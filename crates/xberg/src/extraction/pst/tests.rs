use super::*;
use outlook_pst::{
    ltp::prop_context::PropertyValue,
    messaging::folder::{Folder, FolderProperties},
    messaging::store::{EntryId, Store, StoreProperties, StoreRecordKey},
    ndb::node_id::NodeId,
};
use std::io;

/// Regression tests for issue #152: PST body resolution must fall back to
/// PR_RTF_COMPRESSED (decompressed+stripped) when plain text is absent, and
/// must clean raw HTML rather than dumping markup when only HTML is present.
#[test]
fn test_resolve_pst_body_prefers_plain_text_issue_152() {
    let result = resolve_pst_body(Some("plain body"), Some("<p>html body</p>"), Some("rtf body"));
    assert_eq!(result, "plain body");
}

#[test]
fn test_resolve_pst_body_cleans_html_when_plain_absent_issue_152() {
    let result = resolve_pst_body(None, Some("<p>Hello <b>World</b></p>"), Some("rtf body"));
    assert_eq!(result, "Hello World");
}

#[test]
fn test_resolve_pst_body_falls_back_to_rtf_when_only_rtf_present_issue_152() {
    let result = resolve_pst_body(None, None, Some("rtf-derived plain text"));
    assert_eq!(result, "rtf-derived plain text");
}

#[test]
fn test_resolve_pst_body_empty_when_all_absent_issue_152() {
    let result = resolve_pst_body(None, None, None);
    assert_eq!(result, "");
}

#[test]
fn test_resolve_pst_body_treats_empty_strings_as_absent_issue_152() {
    let result = resolve_pst_body(Some(""), Some(""), Some("rtf fallback"));
    assert_eq!(result, "rtf fallback");
}

#[test]
fn test_decompress_and_strip_rtf_via_shared_email_helpers_issue_152() {
    // End-to-end through the actual MS-OXRTFCP decoder shared with the MSG
    // path: build a minimal "uncompressed" (MELA-magic) RTF-compressed blob
    // and confirm extraction/pst.rs can decompress+strip it via the
    // extraction/email.rs helpers exactly as extract_message_content does.
    let rtf_plain = b"{\\rtf1 Hello RTF World\\par}";
    let comp_size = (rtf_plain.len() + 12) as u32;
    let mut data = Vec::new();
    data.extend_from_slice(&comp_size.to_le_bytes());
    data.extend_from_slice(&(rtf_plain.len() as u32).to_le_bytes());
    data.extend_from_slice(&0x414c_454du32.to_le_bytes()); // MELA = uncompressed
    data.extend_from_slice(&0u32.to_le_bytes()); // crc, unused for MELA
    data.extend_from_slice(rtf_plain);

    let decompressed = super::super::email::decompress_rtf_compressed(&data).expect("should decompress");
    let plain = super::super::email::strip_rtf_to_plain_text(&decompressed);
    assert_eq!(plain, "Hello RTF World");
}

/// Regression test for issue #764: entry_id must be the MAPI hex format,
/// not the Rust Debug representation of the EntryId struct.
#[test]
fn test_entry_id_hex_format_issue_764() {
    let record_key_bytes: [u8; 16] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
    ];
    let record_key = StoreRecordKey::new(record_key_bytes);

    let node_id = NodeId::from(0x04 | (1u32 << 5));
    let entry_id = EntryId::new(record_key, node_id);

    let node_id_u32 = u32::from(entry_id.node_id());
    let node_id_le = node_id_u32.to_le_bytes();
    let expected: String = std::iter::repeat_n(0u8, 4)
        .chain(record_key_bytes.iter().copied())
        .chain(node_id_le.iter().copied())
        .map(|b| format!("{b:02X}"))
        .collect();

    assert_eq!(expected.len(), 48, "MAPI EntryID must be 48 hex chars");

    assert!(expected.starts_with("00000000"), "EntryID must start with 00000000");

    assert!(!expected.contains("EntryId"), "must not be Debug representation");
    assert!(!expected.contains("record_key"), "must not be Debug representation");
    assert!(!expected.contains('{'), "must not be Debug representation");
}

#[test]
fn test_filetime_known_epoch() {
    let filetime: i64 = 116_444_736_000_000_000;
    let result = windows_filetime_to_string(filetime);
    assert_eq!(result, "1970-01-01T00:00:00Z");
}

#[test]
fn test_filetime_known_date() {
    let filetime: i64 = 133_549_776_000_000_000;
    let result = windows_filetime_to_string(filetime);
    assert_eq!(result, "2024-03-15T12:00:00Z");
}

#[test]
fn test_filetime_sub_second_part_is_truncated() {
    let filetime: i64 = 133_549_776_000_000_000 + 9_999_999;
    assert_eq!(windows_filetime_to_string(filetime), "2024-03-15T12:00:00Z");
}

#[test]
fn test_filetime_before_unix_epoch_is_invalid() {
    let filetime: i64 = 116_444_735_999_999_999;
    let result = windows_filetime_to_string(filetime);
    assert!(result.starts_with("(invalid timestamp:"));
}

#[test]
fn test_filetime_zero_is_invalid() {
    let result = windows_filetime_to_string(0);
    assert!(result.starts_with("(invalid timestamp:"));
}

#[test]
fn test_prop_value_integer32_returns_none() {
    let val = PropertyValue::Integer32(42);
    assert_eq!(prop_value_to_string(&val), None);
}

#[test]
fn test_prop_value_boolean_returns_none() {
    let val = PropertyValue::Boolean(true);
    assert_eq!(prop_value_to_string(&val), None);
}

#[test]
fn test_prop_value_time_returns_none() {
    let val = PropertyValue::Time(133_549_776_000_000_000);
    assert_eq!(prop_value_to_string(&val), None);
}

/// A `Store` whose `StoreProperties` are empty, so `ipm_sub_tree_entry_id()`
/// always fails, without needing a real PST file on disk.
struct FakeStoreWithoutIpmSubtree {
    properties: StoreProperties,
}

impl Store for FakeStoreWithoutIpmSubtree {
    fn properties(&self) -> &StoreProperties {
        &self.properties
    }

    fn root_hierarchy_table(&self) -> io::Result<Rc<dyn outlook_pst::ltp::table_context::TableContext>> {
        Err(io::Error::other("not implemented in test fake"))
    }

    fn unique_value(&self) -> u32 {
        0
    }

    fn open_folder(&self, _entry_id: &EntryId) -> io::Result<Rc<dyn outlook_pst::messaging::folder::Folder>> {
        Err(io::Error::other("not implemented in test fake"))
    }

    fn open_message(
        &self,
        _entry_id: &EntryId,
        _prop_ids: Option<&[u16]>,
    ) -> io::Result<Rc<dyn outlook_pst::messaging::message::Message>> {
        Err(io::Error::other("not implemented in test fake"))
    }

    fn named_property_map(&self) -> io::Result<Rc<dyn outlook_pst::messaging::named_prop::NamedPropertyMap>> {
        Err(io::Error::other("not implemented in test fake"))
    }

    fn search_update_queue(&self) -> io::Result<Rc<dyn outlook_pst::messaging::search::SearchUpdateQueue>> {
        Err(io::Error::other("not implemented in test fake"))
    }
}

/// Regression test for issue #162(a): a PST whose IPM (mail) sub-tree cannot
/// be located must surface a `ProcessingWarning`, not silently return an
/// empty result.
#[test]
fn test_extract_from_store_warns_when_ipm_subtree_missing_issue_162() {
    let store = FakeStoreWithoutIpmSubtree {
        properties: StoreProperties::default(),
    };

    let (messages, warnings) = extract_from_store(&store);

    assert!(
        messages.is_empty(),
        "no messages should be extracted when the IPM sub-tree cannot be located"
    );
    assert_eq!(warnings.len(), 1, "exactly one warning should be emitted");
    assert_eq!(warnings[0].source.as_ref(), "pst_extraction");
    assert!(
        warnings[0]
            .message
            .contains("Failed to locate IPM (mail) sub-tree in PST store"),
        "unexpected warning message: {}",
        warnings[0].message
    );
}

/// Regression test for issue #162(c): if the PST root folder's entry ID
/// cannot be built (e.g. the store's record key is missing),
/// `discover_non_ipm_top_level_folders` must report a `ProcessingWarning`
/// and return no seeds -- never panic or silently return nothing.
#[test]
fn test_discover_non_ipm_top_level_folders_warns_when_entry_id_creation_fails() {
    let store = FakeStoreWithoutIpmSubtree {
        properties: StoreProperties::default(),
    };

    let (seeds, warnings) = discover_non_ipm_top_level_folders(&store, 0);

    assert!(
        seeds.is_empty(),
        "no seeds should be produced when the root entry ID cannot be built"
    );
    assert_eq!(warnings.len(), 1, "exactly one warning should be emitted");
    assert_eq!(warnings[0].source.as_ref(), "pst_extraction");
    assert!(
        warnings[0]
            .message
            .contains("Failed to build entry ID for PST root folder"),
        "unexpected warning message: {}",
        warnings[0].message
    );
}

/// Absolute path to the only PST fixture checked into the repo
/// (`test_documents/email/empty.pst`), resolved the same way integration
/// tests under `crates/xberg/tests/helpers/mod.rs` do: two levels up from
/// this crate's manifest directory is the workspace root.
fn empty_pst_fixture_path() -> std::path::PathBuf {
    let workspace_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/xberg should have a parent directory")
        .parent()
        .expect("crates/xberg should have a workspace root two levels up")
        .to_path_buf();

    workspace_root.join("test_documents/email/empty.pst")
}

/// Wraps a real `Store` (opened from the `empty.pst` fixture, which has a
/// genuine record key that `StoreProperties` provides no public
/// constructor for outside `outlook_pst`) so a single method can be
/// overridden to simulate a failure while everything else -- crucially
/// `properties()` and its real record key -- still comes from the real
/// store.
struct StoreWithFailingOpenFolder {
    inner: Rc<dyn Store>,
}

impl Store for StoreWithFailingOpenFolder {
    fn properties(&self) -> &StoreProperties {
        self.inner.properties()
    }

    fn root_hierarchy_table(&self) -> io::Result<Rc<dyn outlook_pst::ltp::table_context::TableContext>> {
        self.inner.root_hierarchy_table()
    }

    fn unique_value(&self) -> u32 {
        self.inner.unique_value()
    }

    fn open_folder(&self, _entry_id: &EntryId) -> io::Result<Rc<dyn Folder>> {
        Err(io::Error::other("simulated open_folder failure for test"))
    }

    fn open_message(
        &self,
        entry_id: &EntryId,
        prop_ids: Option<&[u16]>,
    ) -> io::Result<Rc<dyn outlook_pst::messaging::message::Message>> {
        self.inner.open_message(entry_id, prop_ids)
    }

    fn named_property_map(&self) -> io::Result<Rc<dyn outlook_pst::messaging::named_prop::NamedPropertyMap>> {
        self.inner.named_property_map()
    }

    fn search_update_queue(&self) -> io::Result<Rc<dyn outlook_pst::messaging::search::SearchUpdateQueue>> {
        self.inner.search_update_queue()
    }
}

/// Regression test for issue #162(c): if the PST root folder cannot be
/// opened, `discover_non_ipm_top_level_folders` must report a
/// `ProcessingWarning` distinct from the entry-ID-creation failure above,
/// and still return no seeds. Wraps the real `empty.pst` fixture's `Store`
/// so `properties().make_entry_id(NID_ROOT_FOLDER)` succeeds for real and
/// only `open_folder` is made to fail.
#[test]
fn test_discover_non_ipm_top_level_folders_warns_when_root_folder_cannot_be_opened() {
    let fixture = empty_pst_fixture_path();
    assert!(fixture.exists(), "PST test fixture not found: {fixture:?}");
    let real_store = outlook_pst::open_store(&fixture).expect("should open empty.pst fixture");
    let store = StoreWithFailingOpenFolder { inner: real_store };

    let (seeds, warnings) = discover_non_ipm_top_level_folders(&store, 0);

    assert!(
        seeds.is_empty(),
        "no seeds should be produced when the root folder cannot be opened"
    );
    assert_eq!(warnings.len(), 1, "exactly one warning should be emitted");
    assert_eq!(warnings[0].source.as_ref(), "pst_extraction");
    assert!(
        warnings[0].message.contains("Failed to open PST root folder"),
        "unexpected warning message: {}",
        warnings[0].message
    );
}

/// A `Folder` whose `hierarchy_table()` always returns `None`, simulating
/// `FolderInner::read_table`'s internal `.ok()` swallowing a read error.
struct FolderWithNoHierarchyTable;

impl Folder for FolderWithNoHierarchyTable {
    fn store(&self) -> Rc<dyn Store> {
        unreachable!("discover_non_ipm_top_level_folders never calls Folder::store on the root")
    }

    fn properties(&self) -> &FolderProperties {
        unreachable!("discover_non_ipm_top_level_folders never calls Folder::properties on the root")
    }

    fn hierarchy_table(&self) -> Option<&Rc<dyn outlook_pst::ltp::table_context::TableContext>> {
        None
    }

    fn contents_table(&self) -> Option<&Rc<dyn outlook_pst::ltp::table_context::TableContext>> {
        None
    }

    fn associated_table(&self) -> Option<&Rc<dyn outlook_pst::ltp::table_context::TableContext>> {
        None
    }
}

/// Wraps a real `Store` so `open_folder` returns a `Folder` whose
/// `hierarchy_table()` is `None`, exercising the branch that
/// `Store::root_hierarchy_table()`'s plain `Result` return type never had.
struct StoreWithNoHierarchyTableRoot {
    inner: Rc<dyn Store>,
}

impl Store for StoreWithNoHierarchyTableRoot {
    fn properties(&self) -> &StoreProperties {
        self.inner.properties()
    }

    fn root_hierarchy_table(&self) -> io::Result<Rc<dyn outlook_pst::ltp::table_context::TableContext>> {
        self.inner.root_hierarchy_table()
    }

    fn unique_value(&self) -> u32 {
        self.inner.unique_value()
    }

    fn open_folder(&self, _entry_id: &EntryId) -> io::Result<Rc<dyn Folder>> {
        Ok(Rc::new(FolderWithNoHierarchyTable))
    }

    fn open_message(
        &self,
        entry_id: &EntryId,
        prop_ids: Option<&[u16]>,
    ) -> io::Result<Rc<dyn outlook_pst::messaging::message::Message>> {
        self.inner.open_message(entry_id, prop_ids)
    }

    fn named_property_map(&self) -> io::Result<Rc<dyn outlook_pst::messaging::named_prop::NamedPropertyMap>> {
        self.inner.named_property_map()
    }

    fn search_update_queue(&self) -> io::Result<Rc<dyn outlook_pst::messaging::search::SearchUpdateQueue>> {
        self.inner.search_update_queue()
    }
}

/// Regression test for issue #162(c): `Folder::hierarchy_table()` returns
/// `Option`, not `Result` -- its `None` case (the underlying read error is
/// swallowed inside `outlook_pst`) must surface its own distinct
/// `ProcessingWarning`, never be treated the same as "no non-IPM folders
/// found" (i.e. an empty, warning-free result).
#[test]
fn test_discover_non_ipm_top_level_folders_warns_when_hierarchy_table_is_none() {
    let fixture = empty_pst_fixture_path();
    assert!(fixture.exists(), "PST test fixture not found: {fixture:?}");
    let real_store = outlook_pst::open_store(&fixture).expect("should open empty.pst fixture");
    let store = StoreWithNoHierarchyTableRoot { inner: real_store };

    let (seeds, warnings) = discover_non_ipm_top_level_folders(&store, 0);

    assert!(
        seeds.is_empty(),
        "no seeds should be produced when the root has no hierarchy table"
    );
    assert_eq!(warnings.len(), 1, "exactly one warning should be emitted");
    assert_eq!(warnings[0].source.as_ref(), "pst_extraction");
    assert_eq!(
        warnings[0].message.as_ref(),
        "PST root folder has no hierarchy table; cannot enumerate non-IPM top-level folders"
    );
}

/// Proves the workaround does not reintroduce the deadlock a prior commit
/// hit (see `crates/xberg/tests/pst_warnings_and_structure_test.rs`) when
/// `discover_non_ipm_top_level_folders` was wired in using
/// `Store::root_hierarchy_table()` directly: this calls it against the
/// real, parsed `empty.pst` fixture. If the workaround reintroduced the
/// deadlock, this test would hang rather than fail an assertion --
/// running it under a timeout the first time is recommended.
///
/// `empty.pst` actually has three non-IPM top-level folders -- "Search
/// Root", "SPAM Search Folder 2", and "IPM_COMMON_VIEWS" -- all at depth
/// 0. This test's primary job is proving the absence of the deadlock
/// described above, not asserting an empty result; the exact folder set
/// is pinned so a regression in discovery is still caught.
#[test]
fn test_discover_non_ipm_top_level_folders_against_real_empty_pst_fixture_issue_162c() {
    let fixture = empty_pst_fixture_path();
    assert!(fixture.exists(), "PST test fixture not found: {fixture:?}");
    let store = outlook_pst::open_store(&fixture).expect("should open empty.pst fixture");

    let ipm_entry = store
        .properties()
        .ipm_sub_tree_entry_id()
        .expect("empty.pst fixture should have a locatable IPM sub-tree");
    let ipm_node_id = u32::from(ipm_entry.node_id());

    let (seeds, warnings) = discover_non_ipm_top_level_folders(store.as_ref(), ipm_node_id);

    assert_eq!(
        warnings.len(),
        0,
        "well-formed empty.pst fixture should not produce warnings, got: {warnings:?}"
    );

    let mut names: Vec<&str> = seeds.iter().map(|(_, _, name)| name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec!["IPM_COMMON_VIEWS", "SPAM Search Folder 2", "Search Root"],
        "unexpected non-IPM top-level folder set discovered in empty.pst"
    );
    assert_eq!(seeds.len(), 3, "expected exactly three non-IPM top-level folders");
    assert!(
        seeds.iter().all(|(_, depth, _)| *depth == 0),
        "all non-IPM top-level folders must be reported at depth 0, got: {:?}",
        seeds.iter().map(|(_, depth, name)| (depth, name)).collect::<Vec<_>>()
    );
}

/// [`non_ipm_top_level_ids`] must exclude exactly the id equal to
/// `ipm_node_id`, preserving the order and every other id (including
/// duplicates) from `top_level_ids`.
#[test]
fn test_non_ipm_top_level_ids_filters_out_ipm_node_only() {
    let top_level_ids = [10, 20, 30, 40];

    assert_eq!(
        non_ipm_top_level_ids(&top_level_ids, Some(20)),
        vec![10, 30, 40],
        "the id matching ipm_node_id must be removed and no others"
    );
}

/// When `ipm_node_id` is `None` (e.g. the caller has no IPM node to
/// exclude), every id must be returned unchanged.
#[test]
fn test_non_ipm_top_level_ids_returns_all_ids_when_ipm_node_id_is_none() {
    let top_level_ids = [1, 2, 3];

    assert_eq!(non_ipm_top_level_ids(&top_level_ids, None), vec![1, 2, 3]);
}

/// When `ipm_node_id` does not match any id in `top_level_ids`, every id
/// must be returned unchanged (no accidental over-filtering).
#[test]
fn test_non_ipm_top_level_ids_returns_all_ids_when_ipm_node_id_not_present() {
    let top_level_ids = [5, 6, 7];

    assert_eq!(non_ipm_top_level_ids(&top_level_ids, Some(999)), vec![5, 6, 7]);
}

/// An empty `top_level_ids` slice must yield an empty result regardless
/// of `ipm_node_id`.
#[test]
fn test_non_ipm_top_level_ids_empty_input_returns_empty() {
    let top_level_ids: [u32; 0] = [];

    assert_eq!(non_ipm_top_level_ids(&top_level_ids, Some(1)), Vec::<u32>::new());
}

/// If `top_level_ids` contains the IPM node id more than once (a
/// malformed/hostile hierarchy table), every occurrence must be filtered
/// out, not just the first.
#[test]
fn test_non_ipm_top_level_ids_filters_all_duplicate_ipm_occurrences() {
    let top_level_ids = [7, 7, 8, 7];

    assert_eq!(non_ipm_top_level_ids(&top_level_ids, Some(7)), vec![8]);
}

/// Pins the finding behind `walk_folder_tree`'s "no de-duplication needed"
/// doc comment: `outlook_pst::messaging::folder::Folder::contents_table()`
/// (`outlook-pst` 1.2.0) is not type-aware for search folders. It always
/// reads `NodeIdType::ContentsTable`, never the distinct
/// `NodeIdType::SearchContentsTable` a real search folder's linked-message
/// rows live under, so it returns `None` for a genuine search folder
/// rather than the aliased messages a naive walk would need to
/// de-duplicate. `empty.pst`'s "SPAM Search Folder 2" is exactly such a
/// search folder (discovered as a non-IPM top-level seed). If a future
/// `outlook-pst` upgrade starts returning `Some(_)` here, the "no
/// aliasing risk" analysis in `walk_folder_tree`'s doc comment no longer
/// holds and de-duplication must be added.
#[test]
fn test_search_folder_contents_table_is_none_no_message_aliasing_issue_162() {
    let fixture = empty_pst_fixture_path();
    assert!(fixture.exists(), "PST test fixture not found: {fixture:?}");
    let store = outlook_pst::open_store(&fixture).expect("should open empty.pst fixture");

    let ipm_entry = store
        .properties()
        .ipm_sub_tree_entry_id()
        .expect("empty.pst fixture should have a locatable IPM sub-tree");
    let ipm_node_id = u32::from(ipm_entry.node_id());

    let (seeds, warnings) = discover_non_ipm_top_level_folders(store.as_ref(), ipm_node_id);
    assert_eq!(warnings.len(), 0, "unexpected warnings: {warnings:?}");

    let (search_folder, _, _) = seeds
        .iter()
        .find(|(_, _, name)| name == "SPAM Search Folder 2")
        .expect("empty.pst fixture should contain the 'SPAM Search Folder 2' search folder");

    assert!(
        search_folder.contents_table().is_none(),
        "outlook-pst's Folder::contents_table() unexpectedly returned Some(_) for a search \
             folder -- this crate now exposes real search-folder contents, so walk_folder_tree's \
             'no aliasing risk' analysis is stale and de-duplication must be added"
    );
}
