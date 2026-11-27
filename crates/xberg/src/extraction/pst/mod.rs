//! PST (Outlook Personal Folders) file extraction.
//!
//! This module handles extraction of emails from Microsoft Outlook PST files
//! using the `outlook-pst` crate.
//!
//! # Features
//!
//! - **Unicode and ANSI PST support**: Handles both modern and legacy PST formats
//! - **Folder hierarchy traversal**: Extracts messages from all folders recursively
//! - **Message properties**: Extracts subject, sender, recipients, body
//!
//! # Example
//!
//! Not run as a doctest: `pub(crate)`, so it is unreachable from a downstream crate.
//!
//! ```ignore
//! use xberg::extraction::pst::extract_pst_messages;
//!
//! # fn example() -> xberg::Result<()> {
//! let pst_bytes = std::fs::read("archive.pst")?;
//! let (messages, _warnings) = extract_pst_messages(&pst_bytes)?;
//!
//! for msg in &messages {
//!     println!("Subject: {:?}", msg.subject);
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::{Result, XbergError};
use crate::types::{EmailAttachment, EmailExtractionResult, ProcessingWarning};
use std::borrow::Cow;
use std::collections::HashMap;

#[cfg(feature = "email")]
use outlook_pst::{
    ltp::{prop_context::PropertyValue, table_context::TableContext},
    messaging::{folder::Folder as PstFolder, message::Message as PstMessage, store::EntryId},
    ndb::node_id::{NID_ROOT_FOLDER, NodeId},
};
#[cfg(feature = "email")]
use std::rc::Rc;

/// A folder queued for traversal: the folder handle itself, its recursion depth
/// (0 at the top level), and its display path (used only for warning messages).
///
/// Named to keep this shape out of `walk_folder_tree`'s signature and the seed
/// lists built by [`extract_from_store`] and [`discover_non_ipm_top_level_folders`]
/// (see issue #162), which otherwise trips clippy's `type_complexity` lint.
#[cfg(feature = "email")]
type PstFolderSeed = (Rc<dyn PstFolder>, u32, String);

/// Safety cap on rows read from a single PST contents/hierarchy table in one
/// pass.
///
/// PST folder/table structures are attacker-controllable input: a corrupt or
/// hostile table can report (or its row iterator can yield) an effectively
/// unbounded number of rows. Before issue #162 the traversal fully
/// materialized a table's rows via `.collect()` with no bound, so such a
/// table hung extraction forever — the existing per-folder recursion-depth
/// cap (`depth > 50`) never even came into play, because it protects against
/// deep/cyclic *folder* nesting, not an unbounded row iterator *within* a
/// single table. Reading rows one at a time and stopping at this cap fixes
/// that regardless of which table (IPM or non-IPM) is misbehaving.
#[cfg(feature = "email")]
const MAX_TABLE_ROWS: usize = 100_000;

/// Read node ids from a table's `rows_matrix()`, stopping after
/// [`MAX_TABLE_ROWS`] rows without ever materializing the rest of the
/// iterator. Returns `(ids, true)` when reading stopped only because the cap
/// was hit, so callers can surface a `ProcessingWarning` about truncation.
#[cfg(feature = "email")]
fn collect_row_ids(table: &dyn TableContext) -> (Vec<u32>, bool) {
    let mut ids = Vec::new();
    for row in table.rows_matrix() {
        if ids.len() >= MAX_TABLE_ROWS {
            return (ids, true);
        }
        ids.push(u32::from(row.id()));
    }
    (ids, false)
}

/// From the node ids returned by `Store::root_hierarchy_table()` (the true
/// PST root's direct children), determine which ones are non-IPM (non-mail)
/// top-level folders that still need to be traversed — i.e. every id except
/// the one already covered by the IPM (mail) sub-tree walk.
///
/// Pure and dependency-free so the enumeration decision (issue #162) can be
/// unit-tested without a real PST `Store`/`Folder`.
#[cfg(feature = "email")]
fn non_ipm_top_level_ids(top_level_ids: &[u32], ipm_node_id: Option<u32>) -> Vec<u32> {
    top_level_ids
        .iter()
        .copied()
        .filter(|id| Some(*id) != ipm_node_id)
        .collect()
}

/// Extract all email messages from a PST file.
///
/// Opens the PST file and traverses the full folder hierarchy, extracting
/// every message including subject, sender, recipients, and body text.
///
/// # Arguments
///
/// * `pst_data` - Raw bytes of the PST file
///
/// # Returns
///
/// A vector of `EmailExtractionResult`, one per message found.
///
/// # Errors
///
/// Returns an error if the PST data cannot be written to a temporary file,
/// or if the PST format is invalid.
#[cfg(all(feature = "email", not(target_arch = "wasm32")))]
pub(crate) fn extract_pst_messages(pst_data: &[u8]) -> Result<(Vec<EmailExtractionResult>, Vec<ProcessingWarning>)> {
    use std::io::Write;

    let mut temp_file = tempfile::Builder::new()
        .prefix("xberg_pst_")
        .suffix(".tmp")
        .tempfile()
        .map_err(crate::XbergError::from)?;

    temp_file.write_all(pst_data).map_err(crate::XbergError::from)?;
    temp_file.flush().map_err(crate::XbergError::from)?;

    let (messages, warnings) = extract_from_path(temp_file.path())?;
    Ok((messages, warnings))
}

/// WASM-safe fallback: PST extraction is not available on WASM due to tempfile incompatibility.
#[cfg(all(feature = "email", target_arch = "wasm32"))]
pub(crate) fn extract_pst_messages(_pst_data: &[u8]) -> Result<(Vec<EmailExtractionResult>, Vec<ProcessingWarning>)> {
    Err(XbergError::Validation {
        message: "PST extraction is not supported on WebAssembly targets".to_string(),
        source: None,
    })
}

/// Extract PST messages directly from a file path, bypassing the in-memory copy.
///
/// Used by `PstExtractor::extract_file` to avoid the double-allocation that
/// occurs when the full PST is first read into a `Vec<u8>` and then written
/// back out to a tempfile before parsing.
#[cfg(all(feature = "email", feature = "tokio-runtime"))]
pub(crate) fn extract_pst_from_path(
    path: &std::path::Path,
) -> Result<(Vec<EmailExtractionResult>, Vec<ProcessingWarning>)> {
    extract_from_path(path)
}

#[cfg(feature = "email")]
fn extract_from_path(path: &std::path::Path) -> Result<(Vec<EmailExtractionResult>, Vec<ProcessingWarning>)> {
    let store = outlook_pst::open_store(path).map_err(|e| XbergError::Validation {
        message: format!("Failed to open PST file: {e}"),
        source: None,
    })?;

    Ok(extract_from_store(store.as_ref()))
}

/// Walk an already-opened PST `Store` and extract all messages, collecting
/// non-fatal `ProcessingWarning`s along the way.
///
/// Split out from `extract_from_path` so the traversal logic can be unit
/// tested against a synthetic `Store` implementation without needing a real
/// PST file on disk (see issue #162).
#[cfg(feature = "email")]
fn extract_from_store(
    store: &dyn outlook_pst::messaging::store::Store,
) -> (Vec<EmailExtractionResult>, Vec<ProcessingWarning>) {
    let mut warnings = Vec::new();

    let ipm_entry = match store.properties().ipm_sub_tree_entry_id() {
        Ok(e) => e,
        Err(e) => {
            warnings.push(ProcessingWarning {
                source: Cow::Borrowed("pst_extraction"),
                message: Cow::Owned(format!("Failed to locate IPM (mail) sub-tree in PST store: {e}")),
            });
            return (Vec::new(), warnings);
        }
    };

    let root_folder = match store.open_folder(&ipm_entry) {
        Ok(f) => f,
        Err(e) => {
            warnings.push(ProcessingWarning {
                source: Cow::Borrowed("pst_extraction"),
                message: Cow::Owned(format!("Failed to open IPM (mail) sub-tree root folder: {e}")),
            });
            return (Vec::new(), warnings);
        }
    };

    let root_name = root_folder
        .properties()
        .display_name()
        .unwrap_or_else(|_| "Top of Personal Folders".to_string());
    let ipm_node_id = u32::from(ipm_entry.node_id());
    let mut seeds: Vec<PstFolderSeed> = vec![(root_folder, 0, root_name)];

    let (non_ipm_seeds, mut discovery_warnings) = discover_non_ipm_top_level_folders(store, ipm_node_id);
    seeds.extend(non_ipm_seeds);
    warnings.append(&mut discovery_warnings);

    let (messages, mut traversal_warnings) = walk_folder_tree(store, seeds);
    warnings.append(&mut traversal_warnings);

    (messages, warnings)
}

/// Enumerate the PST store's true top-level folders and return every one
/// that is *not* the already-handled IPM (mail) sub-tree, ready to seed
/// [`walk_folder_tree`] alongside it.
///
/// Split out from `extract_from_store` (issue #162) so the enumeration can be
/// exercised without a fully-populated `StoreProperties` — every id is either
/// opened as a seed folder or reported via a `ProcessingWarning`, traversal
/// never aborts because one non-IPM folder failed to open.
///
/// # Reaches the root hierarchy table via a workaround, not `Store::root_hierarchy_table()`
///
/// `outlook_pst::messaging::store::Store::root_hierarchy_table()` deadlocks
/// unconditionally in `outlook-pst` 1.2.0 (the version on crates.io, and the
/// version this crate depends on): its default implementation locks the PST
/// file-reader `Mutex` to resolve the root folder's B-tree node, keeps that
/// `MutexGuard` alive, and then calls `TableContextInner::read`, which tries
/// to lock the *same* `Mutex` again on the same thread. `std::sync::Mutex` is
/// not reentrant, so the second `.lock()` call blocks forever — this happens
/// before a single row is read, on every PST file, not only malformed ones.
/// Upstream fixed this on `main` (PR #55) by scoping the lock guard to a block
/// that ends before `TableContext::read` runs, but nothing has been published:
/// crates.io tops out at 1.2.0 and the `outlook-pst` release workflow has no
/// `release-pr` job, so no 1.2.1 appears without a maintainer manually
/// bumping the version.
///
/// Instead of calling `root_hierarchy_table()`, this function reaches the
/// identical node (`NodeId::new(HierarchyTable, NID_ROOT_FOLDER.index())`)
/// through a path that is *already* correctly scoped in 1.2.0:
/// `FolderInner::read_table` binds its B-tree node inside a block, so the
/// file-reader lock is dropped before `TableContext::read` is called. Opening
/// the root folder through the public API —
/// `store.properties().make_entry_id(NID_ROOT_FOLDER)` ->
/// `store.open_folder(&entry_id)` -> `folder.hierarchy_table()` — is exactly
/// how upstream's own `FolderInner::read` expects the root to be opened:
/// `NID_ROOT_FOLDER`'s type bits (`0x122 & 0x1F == 0x02`) satisfy the
/// `NormalFolder | SearchFolder` gate, and `read` even special-cases
/// `entry_id.node_id() == NID_ROOT_FOLDER` when computing the folder type.
///
/// `Folder::hierarchy_table()` returns `Option`, not `Result` (it swallows
/// the underlying read error via `.ok()` internally), so its `None` case is
/// reported below as its own `ProcessingWarning` distinct from the two error
/// arms above it — never silently treated as "no folders found".
#[cfg(feature = "email")]
/// Resolve the PST store's true root folder's hierarchy table, the starting
/// point for enumerating non-IPM top-level folders. Each of the three ways
/// this can fail (entry ID, folder open, missing table) is reported as its
/// own distinct warning by the caller.
fn resolve_root_hierarchy_table(
    store: &dyn outlook_pst::messaging::store::Store,
) -> std::result::Result<Rc<dyn TableContext>, ProcessingWarning> {
    let root_entry_id = store
        .properties()
        .make_entry_id(NID_ROOT_FOLDER)
        .map_err(|e| ProcessingWarning {
            source: Cow::Borrowed("pst_extraction"),
            message: Cow::Owned(format!(
                "Failed to build entry ID for PST root folder while enumerating non-IPM top-level folders: {e}"
            )),
        })?;
    let root_folder = store.open_folder(&root_entry_id).map_err(|e| ProcessingWarning {
        source: Cow::Borrowed("pst_extraction"),
        message: Cow::Owned(format!(
            "Failed to open PST root folder while enumerating non-IPM top-level folders: {e}"
        )),
    })?;
    root_folder.hierarchy_table().cloned().ok_or_else(|| ProcessingWarning {
        source: Cow::Borrowed("pst_extraction"),
        message: Cow::Owned(
            "PST root folder has no hierarchy table; cannot enumerate non-IPM top-level folders".to_string(),
        ),
    })
}

/// Open one non-IPM top-level folder node as a depth-0 traversal seed, or
/// report why it could not be opened.
fn open_non_ipm_top_level_folder(
    store: &dyn outlook_pst::messaging::store::Store,
    id: u32,
) -> std::result::Result<PstFolderSeed, ProcessingWarning> {
    let node = NodeId::from(id);
    let entry_id = store.properties().make_entry_id(node).map_err(|e| ProcessingWarning {
        source: Cow::Borrowed("pst_extraction"),
        message: Cow::Owned(format!(
            "Failed to create entry ID for non-IPM top-level folder node {:?}: {}; folder skipped",
            node, e
        )),
    })?;
    let top_folder = store.open_folder(&entry_id).map_err(|e| ProcessingWarning {
        source: Cow::Borrowed("pst_extraction"),
        message: Cow::Owned(format!(
            "Failed to open non-IPM top-level folder (node {:?}): {}; folder skipped",
            node, e
        )),
    })?;
    let top_name = top_folder
        .properties()
        .display_name()
        .unwrap_or_else(|_| format!("(unnamed non-IPM folder, node {node:?})"));
    Ok((top_folder, 0, top_name))
}

fn discover_non_ipm_top_level_folders(
    store: &dyn outlook_pst::messaging::store::Store,
    ipm_node_id: u32,
) -> (Vec<PstFolderSeed>, Vec<ProcessingWarning>) {
    let mut seeds = Vec::new();
    let mut warnings = Vec::new();

    let root_table = match resolve_root_hierarchy_table(store) {
        Ok(t) => t,
        Err(w) => {
            warnings.push(w);
            return (seeds, warnings);
        }
    };

    let (top_level_ids, truncated) = collect_row_ids(root_table.as_ref());
    if truncated {
        warnings.push(ProcessingWarning {
            source: Cow::Borrowed("pst_extraction"),
            message: Cow::Owned(format!(
                "PST store root exceeds the maximum top-level folder limit ({MAX_TABLE_ROWS}); remaining top-level folders skipped"
            )),
        });
    }

    for id in non_ipm_top_level_ids(&top_level_ids, Some(ipm_node_id)) {
        match open_non_ipm_top_level_folder(store, id) {
            Ok(seed) => seeds.push(seed),
            Err(w) => warnings.push(w),
        }
    }

    (seeds, warnings)
}

/// Walk a set of already-opened top-level folders (and their subtrees),
/// extracting every message and collecting non-fatal `ProcessingWarning`s.
///
/// Split out from `extract_from_store` (issue #162) so the traversal itself
/// — including its termination guarantees — can be unit tested against a
/// synthetic folder tree, independent of how the seed folders were
/// discovered (IPM sub-tree vs. non-IPM top-level folders).
///
/// Termination is guaranteed by two independent bounds: `depth > 50` caps how
/// deep (or how many times, for a cyclic tree) folders are nested, and
/// [`collect_row_ids`] caps how many rows are read from any single table —
/// without the latter, a table whose row iterator never terminates hangs this
/// function forever regardless of the depth cap, because the hang happens
/// while reading rows *within* one folder, before depth is ever considered.
///
/// # No de-duplication of messages across folders (investigated for issue #162)
///
/// [`discover_non_ipm_top_level_folders`] can seed genuine PST *search*
/// folders (`NodeIdType::SearchFolder`), whose contents tables, per
/// [MS-PST] 2.4.8.6, are specified to reference messages that physically live
/// in another (non-search) folder — a real aliasing hazard for a naive
/// per-folder walk. This was investigated against the vendored `outlook-pst`
/// 1.2.0 source rather than assumed: `Folder::contents_table()`
/// (`FolderInner::contents_table` in `messaging/folder.rs`) is *not*
/// type-aware — for every folder, search or normal, it unconditionally reads
/// `NodeIdType::ContentsTable` (nid type `0x0E`). A search folder's actual
/// linked-message rows live under the distinct `NodeIdType::SearchContentsTable`
/// (nid type `0x10`, same node index, different type tag per `NodeId::new`'s
/// bit layout in `ndb/node_id.rs`) — a variant that exists only as an enum
/// case in `ndb/node_id.rs` and is never referenced anywhere else in the
/// crate. So `contents_table()` on a search folder looks up a node id that a
/// search folder never has, and returns `None` (`read_table` maps a missing
/// B-tree entry to `Ok(None)`, not an error). No messages are ever read back
/// from a search folder through this API today, so no message can be emitted
/// twice — de-duplication would guard against a code path that cannot
/// currently execute. If a future `outlook-pst` upgrade adds real
/// `SearchContentsTable` support to `contents_table()`, this analysis must be
/// redone and de-duplication (preferring the real IPM `folder_path` over the
/// search folder's) added at that point.
#[cfg(feature = "email")]
/// Extract every message in one folder's contents table, appending any
/// non-fatal failures (bad entry id, message open failure, row-limit
/// truncation) to `warnings` rather than aborting the folder.
fn extract_folder_messages(
    store: &dyn outlook_pst::messaging::store::Store,
    contents: &dyn TableContext,
    folder_path: &str,
    warnings: &mut Vec<ProcessingWarning>,
) -> Vec<EmailExtractionResult> {
    let mut messages = Vec::new();

    let (ids, truncated) = collect_row_ids(contents);
    if truncated {
        warnings.push(ProcessingWarning {
            source: Cow::Borrowed("pst_extraction"),
            message: Cow::Owned(format!(
                "Folder '{folder_path}' contents table exceeds the maximum row limit ({MAX_TABLE_ROWS}); remaining messages skipped"
            )),
        });
    }
    for id in ids {
        let node = NodeId::from(id);
        let entry_id = match store.properties().make_entry_id(node) {
            Ok(e) => e,
            Err(e) => {
                warnings.push(ProcessingWarning {
                    source: Cow::Borrowed("pst_extraction"),
                    message: Cow::Owned(format!("Failed to create entry ID for message node {:?}: {}", node, e)),
                });
                continue;
            }
        };
        let msg = match store.open_message(&entry_id, None) {
            Ok(m) => m,
            Err(e) => {
                warnings.push(ProcessingWarning {
                    source: Cow::Borrowed("pst_extraction"),
                    message: Cow::Owned(format!("Failed to open message {:?}: {}", entry_id, e)),
                });
                continue;
            }
        };
        messages.push(extract_message_content(msg.as_ref(), &entry_id, folder_path));
    }

    messages
}

/// Enqueue every subfolder in one folder's hierarchy table onto
/// `folder_stack` at `depth + 1`, appending any non-fatal failures (bad entry
/// id, folder open failure, row-limit truncation) to `warnings`.
fn enqueue_subfolders(
    store: &dyn outlook_pst::messaging::store::Store,
    hierarchy: &dyn TableContext,
    folder_path: &str,
    depth: u32,
    folder_stack: &mut Vec<PstFolderSeed>,
    warnings: &mut Vec<ProcessingWarning>,
) {
    let (ids, truncated) = collect_row_ids(hierarchy);
    if truncated {
        warnings.push(ProcessingWarning {
            source: Cow::Borrowed("pst_extraction"),
            message: Cow::Owned(format!(
                "Folder '{folder_path}' hierarchy table exceeds the maximum row limit ({MAX_TABLE_ROWS}); remaining subfolders skipped"
            )),
        });
    }
    for id in ids {
        let node = NodeId::from(id);
        let entry_id = match store.properties().make_entry_id(node) {
            Ok(e) => e,
            Err(e) => {
                warnings.push(ProcessingWarning {
                    source: Cow::Borrowed("pst_extraction"),
                    message: Cow::Owned(format!("Failed to create entry ID for folder node {:?}: {}", node, e)),
                });
                continue;
            }
        };
        let sub_folder = match store.open_folder(&entry_id) {
            Ok(f) => f,
            Err(e) => {
                warnings.push(ProcessingWarning {
                    source: Cow::Borrowed("pst_extraction"),
                    message: Cow::Owned(format!("Failed to open folder {:?}: {}", entry_id, e)),
                });
                continue;
            }
        };
        let sub_name = sub_folder
            .properties()
            .display_name()
            .unwrap_or_else(|_| format!("(unnamed folder, node {node:?})"));
        let sub_path = format!("{folder_path}/{sub_name}");
        folder_stack.push((sub_folder, depth + 1, sub_path));
    }
}

fn walk_folder_tree(
    store: &dyn outlook_pst::messaging::store::Store,
    mut folder_stack: Vec<PstFolderSeed>,
) -> (Vec<EmailExtractionResult>, Vec<ProcessingWarning>) {
    let mut messages = Vec::new();
    let mut warnings = Vec::new();

    while let Some((folder, depth, folder_path)) = folder_stack.pop() {
        if depth > 50 {
            warnings.push(ProcessingWarning {
                source: Cow::Borrowed("pst_extraction"),
                message: Cow::Owned(format!(
                    "Folder '{folder_path}' exceeds maximum traversal depth (50); subtree truncated"
                )),
            });
            continue;
        }

        if let Some(contents) = folder.contents_table() {
            messages.extend(extract_folder_messages(
                store,
                contents.as_ref(),
                &folder_path,
                &mut warnings,
            ));
        }

        if let Some(hierarchy) = folder.hierarchy_table() {
            enqueue_subfolders(
                store,
                hierarchy.as_ref(),
                &folder_path,
                depth,
                &mut folder_stack,
                &mut warnings,
            );
        }
    }

    (messages, warnings)
}

/// Format an `EntryId` as the 48-char MAPI hex string (4 zero bytes + 16-byte
/// record key + 4-byte little-endian node id), not the Rust `Debug` form.
#[cfg(feature = "email")]
fn format_entry_id_hex(entry_id: &EntryId) -> String {
    let record_key = entry_id.record_key();
    let node_id_bytes = u32::from(entry_id.node_id()).to_le_bytes();
    std::iter::repeat_n(0u8, 4)
        .chain(record_key.iter().copied())
        .chain(node_id_bytes.iter().copied())
        .map(|b| format!("{b:02X}"))
        .collect()
}

/// Extract the message send/receive date (`PR_MESSAGE_DELIVERY_TIME`, 0x0E06)
/// as an RFC 3339 string, if present and time-typed.
#[cfg(feature = "email")]
fn extract_message_date(props: &outlook_pst::messaging::message::MessageProperties) -> Option<String> {
    props.get(0x0E06).and_then(|v| {
        if let PropertyValue::Time(ft) = v {
            Some(windows_filetime_to_string(*ft))
        } else {
            None
        }
    })
}

/// Read one recipient row's relevant columns: MAPI recipient type (`0x0C15`,
/// default `1` = To), display name (`0x3001`), and SMTP address (`0x39FE` or
/// `0x3003`, first one seen wins).
#[cfg(feature = "email")]
fn scan_recipient_row(
    table: &dyn TableContext,
    col_defs: &[(u16, outlook_pst::ltp::prop_type::PropertyType)],
    col_values: &[Option<outlook_pst::ltp::table_context::TableRowColumnValue>],
) -> (i32, Option<String>, Option<String>) {
    let mut recipient_type: i32 = 1;
    let mut display_name: Option<String> = None;
    let mut smtp_email: Option<String> = None;

    for ((prop_id, prop_type), value_opt) in col_defs.iter().zip(col_values.iter()) {
        let Some(value_record) = value_opt else {
            continue;
        };
        let Ok(value) = table.read_column(value_record, *prop_type) else {
            continue;
        };

        match prop_id {
            0x0C15 => {
                if let PropertyValue::Integer32(v) = value {
                    recipient_type = v;
                }
            }
            0x3001 => {
                display_name = prop_value_to_string(&value);
            }
            0x39FE | 0x3003 if smtp_email.is_none() => {
                smtp_email = prop_value_to_string(&value);
            }
            _ => {}
        }
    }

    (recipient_type, display_name, smtp_email)
}

/// Extract the To/Cc/Bcc recipient lists from a message's recipient table, in
/// that order. A row with no usable display name or SMTP address is skipped
/// (matches the original single-function behaviour).
#[cfg(feature = "email")]
fn extract_recipients(message: &dyn PstMessage) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut to_emails: Vec<String> = Vec::new();
    let mut cc_emails: Vec<String> = Vec::new();
    let mut bcc_emails: Vec<String> = Vec::new();

    let Some(recipient_table) = message.recipient_table() else {
        return (to_emails, cc_emails, bcc_emails);
    };

    let context = recipient_table.context();
    let col_defs: Vec<(u16, _)> = context.columns().iter().map(|c| (c.prop_id(), c.prop_type())).collect();

    for row in recipient_table.rows_matrix() {
        let Ok(col_values) = row.columns(context) else {
            continue;
        };

        let (recipient_type, display_name, smtp_email) =
            scan_recipient_row(recipient_table.as_ref(), &col_defs, &col_values);

        let recipient = smtp_email.or(display_name).unwrap_or_default();
        if recipient.is_empty() {
            continue;
        }
        match recipient_type {
            1 => to_emails.push(recipient),
            2 => cc_emails.push(recipient),
            3 => bcc_emails.push(recipient),
            _ => {
                tracing::warn!(recipient_type, "Unknown MAPI recipient type; skipping recipient");
            }
        }
    }

    (to_emails, cc_emails, bcc_emails)
}

/// Read one attachment row's relevant columns: long/short filename (`0x3707`/
/// `0x3704`) and raw attachment data (`0x3701`).
#[cfg(feature = "email")]
fn scan_attachment_row(
    table: &dyn TableContext,
    col_defs: &[(u16, outlook_pst::ltp::prop_type::PropertyType)],
    col_values: &[Option<outlook_pst::ltp::table_context::TableRowColumnValue>],
) -> (Option<String>, Option<String>, Option<Vec<u8>>) {
    let mut long_filename: Option<String> = None;
    let mut short_filename: Option<String> = None;
    let mut attach_data: Option<Vec<u8>> = None;

    for ((prop_id, prop_type), value_opt) in col_defs.iter().zip(col_values.iter()) {
        let Some(value_record) = value_opt else {
            continue;
        };
        let Ok(value) = table.read_column(value_record, *prop_type) else {
            continue;
        };

        match prop_id {
            0x3707 => long_filename = prop_value_to_string(&value),
            0x3704 => short_filename = prop_value_to_string(&value),
            0x3701 => {
                if let PropertyValue::Binary(v) = value {
                    attach_data = Some(v.buffer().to_vec());
                }
            }
            _ => {}
        }
    }

    (long_filename, short_filename, attach_data)
}

/// Extract every attachment from a message's attachment table.
#[cfg(feature = "email")]
fn extract_attachments(message: &dyn PstMessage) -> Vec<EmailAttachment> {
    let mut attachments: Vec<EmailAttachment> = Vec::new();

    let Some(attach_table) = message.attachment_table() else {
        return attachments;
    };

    let context = attach_table.context();
    let col_defs: Vec<(u16, _)> = context.columns().iter().map(|c| (c.prop_id(), c.prop_type())).collect();

    for row in attach_table.rows_matrix() {
        let Ok(col_values) = row.columns(context) else {
            continue;
        };

        let (long_filename, short_filename, attach_data) =
            scan_attachment_row(attach_table.as_ref(), &col_defs, &col_values);

        let filename = long_filename.or(short_filename);
        let size = attach_data.as_ref().map(|d| d.len());
        let mime_type = filename
            .as_deref()
            .and_then(|f| mime_guess::from_path(f).first())
            .map(|m| m.to_string());
        let is_image = mime_type.as_deref().is_some_and(|m| m.starts_with("image/"));

        attachments.push(EmailAttachment {
            name: filename.clone(),
            filename,
            mime_type,
            size,
            is_image,
            data: attach_data.map(bytes::Bytes::from),
        });
    }

    attachments
}

#[cfg(feature = "email")]
fn extract_message_content(message: &dyn PstMessage, entry_id: &EntryId, folder_path: &str) -> EmailExtractionResult {
    let props = message.properties();

    let subject = get_str_prop(props, 0x0037);
    let sender_name = get_str_prop(props, 0x0C1A);
    let sender_email = get_str_prop(props, 0x0C1F);
    let from_email = sender_email.or(sender_name);

    let plain_text = get_str_prop(props, 0x1000);
    let html_content = get_str_prop(props, 0x1013);
    // PR_RTF_COMPRESSED (0x1009): fallback body source when neither plain text
    // nor HTML is present. Decompressed and stripped via the same MS-OXRTFCP
    // helpers the MSG extraction path uses (extraction/email.rs).
    let rtf_body = get_binary_prop(props, 0x1009)
        .and_then(|data| super::email::decompress_rtf_compressed(&data))
        .map(|rtf| super::email::strip_rtf_to_plain_text(&rtf))
        .filter(|s| !s.is_empty());

    let content = resolve_pst_body(plain_text.as_deref(), html_content.as_deref(), rtf_body.as_deref());

    let date = extract_message_date(props);
    let entry_id_hex = format_entry_id_hex(entry_id);
    let (to_emails, cc_emails, bcc_emails) = extract_recipients(message);
    let attachments = extract_attachments(message);

    EmailExtractionResult {
        subject,
        from_email,
        to_emails,
        cc_emails,
        bcc_emails,
        date,
        message_id: None,
        plain_text,
        html_content,
        content,
        attachments,
        metadata: HashMap::from([
            ("entry_id".to_string(), entry_id_hex),
            ("folder_path".to_string(), folder_path.to_string()),
        ]),
    }
}

/// Get a string value from message properties by property ID.
#[cfg(feature = "email")]
fn get_str_prop(props: &outlook_pst::messaging::message::MessageProperties, prop_id: u16) -> Option<String> {
    prop_value_to_string(props.get(prop_id)?)
}

/// Read a binary property (e.g. `PR_RTF_COMPRESSED`) verbatim, without string conversion.
#[cfg(feature = "email")]
fn get_binary_prop(props: &outlook_pst::messaging::message::MessageProperties, prop_id: u16) -> Option<Vec<u8>> {
    match props.get(prop_id)? {
        PropertyValue::Binary(v) => Some(v.buffer().to_vec()),
        _ => None,
    }
}

/// Resolve the message body from the available sources, in the same precedence
/// order the MSG extraction path uses (extraction/email.rs): plain text first,
/// then cleaned HTML, then RTF-decompressed plain text, else empty.
///
/// Pure and dependency-free so it can be unit-tested without a real PST file.
fn resolve_pst_body(plain_text: Option<&str>, html_content: Option<&str>, rtf_body: Option<&str>) -> String {
    if let Some(plain) = plain_text.filter(|s| !s.is_empty()) {
        plain.to_string()
    } else if let Some(html) = html_content.filter(|s| !s.is_empty()) {
        super::email::clean_html_content(html)
    } else if let Some(rtf) = rtf_body.filter(|s| !s.is_empty()) {
        rtf.to_string()
    } else {
        String::new()
    }
}

/// Convert a `PropertyValue` to a `String`, if it holds a string type.
#[cfg(feature = "email")]
fn prop_value_to_string(value: &PropertyValue) -> Option<String> {
    match value {
        PropertyValue::String8(v) => Some(v.to_string()),
        PropertyValue::Unicode(v) => Some(v.to_string()),
        PropertyValue::Binary(v) => Some(String::from_utf8_lossy(v.buffer()).into_owned()),
        _ => None,
    }
}

#[cfg(feature = "email")]
fn windows_filetime_to_string(filetime: i64) -> String {
    const EPOCH_DIFF_100NS: i64 = 116_444_736_000_000_000;
    if filetime < EPOCH_DIFF_100NS {
        return format!("(invalid timestamp: {})", filetime);
    }
    let unix_100ns = filetime - EPOCH_DIFF_100NS;
    let unix_secs = unix_100ns / 10_000_000;
    let nsecs = (unix_100ns % 10_000_000) * 100;

    jiff::Timestamp::new(unix_secs, nsecs as i32)
        .map(|ts| ts.strftime("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|_| format!("(invalid timestamp: {})", filetime))
}

#[cfg(not(feature = "email"))]
pub(crate) fn extract_pst_messages(_pst_data: &[u8]) -> Result<(Vec<EmailExtractionResult>, Vec<ProcessingWarning>)> {
    Err(XbergError::FeatureNotEnabled {
        feature: "email".to_string(),
        context: Some("PST extraction requires the 'email' feature to be enabled".to_string()),
    })
}

#[cfg(test)]
#[cfg(feature = "email")]
mod tests;
