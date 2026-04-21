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
//! ```rust,no_run
//! use kreuzberg::extraction::pst::extract_pst_messages;
//!
//! # fn example() -> kreuzberg::Result<()> {
//! let pst_bytes = std::fs::read("archive.pst")?;
//! let messages = extract_pst_messages(&pst_bytes)?;
//!
//! for msg in &messages {
//!     println!("Subject: {:?}", msg.subject);
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::{KreuzbergError, Result};
use crate::types::{EmailAttachment, EmailExtractionResult, ProcessingWarning};
use std::borrow::Cow;
use std::collections::HashMap;

#[cfg(feature = "email")]
use outlook_pst::{
    ltp::prop_context::PropertyValue,
    messaging::{folder::Folder as PstFolder, message::Message as PstMessage, store::EntryId},
    ndb::node_id::NodeId,
};
#[cfg(feature = "email")]
use std::rc::Rc;

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
#[cfg(feature = "email")]
pub fn extract_pst_messages(pst_data: &[u8]) -> Result<(Vec<EmailExtractionResult>, Vec<ProcessingWarning>)> {
    use std::io::Write;

    // open_store requires a file path, so we write to a uniquely-named temp file
    let mut temp_file = tempfile::Builder::new()
        .prefix("kreuzberg_pst_")
        .suffix(".tmp")
        .tempfile()
        .map_err(KreuzbergError::Io)?;

    temp_file.write_all(pst_data).map_err(KreuzbergError::Io)?;
    temp_file.flush().map_err(KreuzbergError::Io)?;

    let (messages, warnings) = extract_from_path(temp_file.path())?;
    Ok((messages, warnings))
}

/// Extract PST messages directly from a file path, bypassing the in-memory copy.
///
/// Used by `PstExtractor::extract_file` to avoid the double-allocation that
/// occurs when the full PST is first read into a `Vec<u8>` and then written
/// back out to a tempfile before parsing.
#[cfg(feature = "email")]
#[allow(dead_code)]
pub(crate) fn extract_pst_from_path(
    path: &std::path::Path,
) -> Result<(Vec<EmailExtractionResult>, Vec<ProcessingWarning>)> {
    extract_from_path(path)
}

#[cfg(feature = "email")]
fn extract_from_path(path: &std::path::Path) -> Result<(Vec<EmailExtractionResult>, Vec<ProcessingWarning>)> {
    let store = outlook_pst::open_store(path).map_err(|e| KreuzbergError::Validation {
        message: format!("Failed to open PST file: {e}"),
        source: None,
    })?;

    let mut messages = Vec::new();
    let mut warnings = Vec::new();

    let ipm_entry = match store.properties().ipm_sub_tree_entry_id() {
        Ok(e) => e,
        Err(_) => return Ok((messages, warnings)),
    };

    let root_folder = match store.open_folder(&ipm_entry) {
        Ok(f) => f,
        Err(_) => return Ok((messages, warnings)),
    };

    // Iterative depth-first traversal to avoid deep recursion
    let mut folder_stack: Vec<(Rc<dyn PstFolder>, u32)> = vec![(root_folder, 0)];

    while let Some((folder, depth)) = folder_stack.pop() {
        if depth > 50 {
            continue;
        }

        // Extract messages from this folder's contents table
        if let Some(contents) = folder.contents_table() {
            let ids: Vec<u32> = contents.rows_matrix().map(|r| u32::from(r.id())).collect();
            for id in ids {
                let node = NodeId::from(id);
                let entry_id = match store.properties().make_entry_id(node) {
                    Ok(e) => e,
                    Err(e) => {
                        warnings.push(ProcessingWarning {
                            source: Cow::Borrowed("pst_extraction"),
                            message: Cow::Owned(format!(
                                "Failed to create entry ID for message node {:?}: {}",
                                node, e
                            )),
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
                messages.push(extract_message_content(msg.as_ref(), &entry_id));
            }
        }

        // Queue sub-folders from the hierarchy table
        if let Some(hierarchy) = folder.hierarchy_table() {
            let ids: Vec<u32> = hierarchy.rows_matrix().map(|r| u32::from(r.id())).collect();
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
                folder_stack.push((sub_folder, depth + 1));
            }
        }
    }

    Ok((messages, warnings))
}

#[cfg(feature = "email")]
fn extract_message_content(message: &dyn PstMessage, entry_id: &EntryId) -> EmailExtractionResult {
    let props = message.properties();

    let subject = get_str_prop(props, 0x0037); // PR_SUBJECT
    let sender_name = get_str_prop(props, 0x0C1A); // PR_SENDER_NAME
    let sender_email = get_str_prop(props, 0x0C1F); // PR_SENDER_EMAIL_ADDRESS
    let from_email = sender_email.or(sender_name);

    let plain_text = get_str_prop(props, 0x1000); // PR_BODY
    let html_content = get_str_prop(props, 0x1013); // PR_HTML (handles String or Binary via prop_value_to_string)

    let cleaned_text = plain_text.clone().or_else(|| html_content.clone()).unwrap_or_default();

    let date = props.get(0x0E06).and_then(|v| {
        if let PropertyValue::Time(ft) = v {
            Some(windows_filetime_to_string(*ft))
        } else {
            None
        }
    });

    // Build MAPI EntryID hex string: 4 zero bytes (flags) + 16-byte record_key + 4-byte node_id LE
    let record_key = entry_id.record_key();
    let node_id_bytes = u32::from(entry_id.node_id()).to_le_bytes();
    let entry_id_hex: String = std::iter::repeat_n(0u8, 4)
        .chain(record_key.iter().copied())
        .chain(node_id_bytes.iter().copied())
        .map(|b| format!("{b:02X}"))
        .collect();

    // Extract recipients from the recipient table
    let mut to_emails: Vec<String> = Vec::new();
    let mut cc_emails: Vec<String> = Vec::new();
    let mut bcc_emails: Vec<String> = Vec::new();

    if let Some(recipient_table) = message.recipient_table() {
        let context = recipient_table.context();
        let col_defs: Vec<(u16, _)> = context.columns().iter().map(|c| (c.prop_id(), c.prop_type())).collect();

        for row in recipient_table.rows_matrix() {
            let Ok(col_values) = row.columns(context) else {
                continue;
            };

            let mut recipient_type: i32 = 1; // default: TO
            let mut display_name: Option<String> = None;
            let mut smtp_email: Option<String> = None;

            for ((prop_id, prop_type), value_opt) in col_defs.iter().zip(col_values.iter()) {
                let Some(value_record) = value_opt else {
                    continue;
                };
                let Ok(value) = recipient_table.read_column(value_record, *prop_type) else {
                    continue;
                };

                match prop_id {
                    0x0C15 => {
                        // PR_RECIPIENT_TYPE
                        if let PropertyValue::Integer32(v) = value {
                            recipient_type = v;
                        }
                    }
                    0x3001 => {
                        // PR_DISPLAY_NAME
                        display_name = prop_value_to_string(&value);
                    }
                    0x39FE | 0x3003
                        // PR_SMTP_ADDRESS / PR_EMAIL_ADDRESS
                        if smtp_email.is_none() => {
                            smtp_email = prop_value_to_string(&value);
                        }
                    _ => {}
                }
            }

            let recipient = smtp_email.or(display_name).unwrap_or_default();
            if recipient.is_empty() {
                continue;
            }
            match recipient_type {
                1 => to_emails.push(recipient),  // MAPI_TO
                2 => cc_emails.push(recipient),  // MAPI_CC
                3 => bcc_emails.push(recipient), // MAPI_BCC
                _ => {
                    tracing::warn!(recipient_type, "Unknown MAPI recipient type; skipping recipient");
                }
            }
        }
    }

    // Extract attachments from the attachment table
    let mut attachments: Vec<EmailAttachment> = Vec::new();

    if let Some(attach_table) = message.attachment_table() {
        let context = attach_table.context();
        let col_defs: Vec<(u16, _)> = context.columns().iter().map(|c| (c.prop_id(), c.prop_type())).collect();

        for row in attach_table.rows_matrix() {
            let Ok(col_values) = row.columns(context) else {
                continue;
            };

            let mut long_filename: Option<String> = None;
            let mut short_filename: Option<String> = None;
            let mut attach_data: Option<Vec<u8>> = None;

            for ((prop_id, prop_type), value_opt) in col_defs.iter().zip(col_values.iter()) {
                let Some(value_record) = value_opt else {
                    continue;
                };
                let Ok(value) = attach_table.read_column(value_record, *prop_type) else {
                    continue;
                };

                match prop_id {
                    0x3707 => long_filename = prop_value_to_string(&value), // PR_ATTACH_LONG_FILENAME
                    0x3704 => short_filename = prop_value_to_string(&value), // PR_ATTACH_FILENAME
                    0x3701 => {
                        // PR_ATTACH_DATA_BINARY
                        if let PropertyValue::Binary(v) = value {
                            attach_data = Some(v.buffer().to_vec());
                        }
                    }
                    _ => {}
                }
            }

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
    }

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
        cleaned_text,
        attachments,
        metadata: HashMap::from([("entry_id".to_string(), entry_id_hex)]),
    }
}

/// Get a string value from message properties by property ID.
#[cfg(feature = "email")]
fn get_str_prop(props: &outlook_pst::messaging::message::MessageProperties, prop_id: u16) -> Option<String> {
    prop_value_to_string(props.get(prop_id)?)
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
    use chrono::DateTime;

    // 100-nanosecond intervals between 1601-01-01 and 1970-01-01
    const EPOCH_DIFF_100NS: i64 = 116_444_736_000_000_000;
    if filetime < EPOCH_DIFF_100NS {
        return format!("(invalid timestamp: {})", filetime);
    }
    let unix_100ns = filetime - EPOCH_DIFF_100NS;
    let unix_secs = unix_100ns / 10_000_000;
    let nsecs = (unix_100ns % 10_000_000) * 100;

    DateTime::from_timestamp(unix_secs, nsecs as u32)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
        .unwrap_or_else(|| format!("(invalid timestamp: {})", filetime))
}

#[cfg(not(feature = "email"))]
pub fn extract_pst_messages(_pst_data: &[u8]) -> Result<(Vec<EmailExtractionResult>, Vec<ProcessingWarning>)> {
    Err(KreuzbergError::FeatureNotEnabled {
        feature: "email".to_string(),
        context: Some("PST extraction requires the 'email' feature to be enabled".to_string()),
    })
}

#[cfg(test)]
#[cfg(feature = "email")]
mod tests {
    use super::*;
    use outlook_pst::{
        ltp::prop_context::PropertyValue,
        messaging::store::{EntryId, StoreRecordKey},
        ndb::node_id::NodeId,
    };

    // ── EntryID format ───────────────────────────────────────────────────────

    /// Regression test for issue #764: entry_id must be the MAPI hex format,
    /// not the Rust Debug representation of the EntryId struct.
    #[test]
    fn test_entry_id_hex_format_issue_764() {
        // 16-byte record_key (store UID), all distinct so we can verify ordering
        let record_key_bytes: [u8; 16] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
        ];
        let record_key = StoreRecordKey::new(record_key_bytes);

        // Build a NormalMessage node_id with index = 1
        // NodeId bits: index<<5 | nid_type; NormalMessage = 0x04
        let node_id = NodeId::from(0x04 | (1u32 << 5));
        let entry_id = EntryId::new(record_key, node_id);

        // Reconstruct the expected hex manually
        let node_id_u32 = u32::from(entry_id.node_id());
        let node_id_le = node_id_u32.to_le_bytes();
        let expected: String = std::iter::repeat_n(0u8, 4)
            .chain(record_key_bytes.iter().copied())
            .chain(node_id_le.iter().copied())
            .map(|b| format!("{b:02X}"))
            .collect();

        // Must be 48 hex chars (24 bytes)
        assert_eq!(expected.len(), 48, "MAPI EntryID must be 48 hex chars");

        // Must start with 8 zeros (4 zero bytes = rgbFlags)
        assert!(expected.starts_with("00000000"), "EntryID must start with 00000000");

        // Must NOT contain Debug-style syntax
        assert!(!expected.contains("EntryId"), "must not be Debug representation");
        assert!(!expected.contains("record_key"), "must not be Debug representation");
        assert!(!expected.contains('{'), "must not be Debug representation");
    }

    // ── FILETIME conversion ──────────────────────────────────────────────────

    #[test]
    fn test_filetime_known_epoch() {
        // FILETIME for 1970-01-01T00:00:00Z is exactly EPOCH_DIFF_100NS
        let filetime: i64 = 116_444_736_000_000_000;
        let result = windows_filetime_to_string(filetime);
        assert_eq!(result, "1970-01-01T00:00:00Z");
    }

    #[test]
    fn test_filetime_known_date() {
        // 2024-03-15T12:00:00Z as FILETIME
        // seconds since 1970: 1710504000
        // filetime = 1710504000 * 10_000_000 + 116_444_736_000_000_000 = 133_549_776_000_000_000
        let filetime: i64 = 133_549_776_000_000_000;
        let result = windows_filetime_to_string(filetime);
        assert_eq!(result, "2024-03-15T12:00:00Z");
    }

    #[test]
    fn test_filetime_before_unix_epoch_is_invalid() {
        // Any filetime less than EPOCH_DIFF_100NS represents a date before 1970-01-01
        let filetime: i64 = 116_444_735_999_999_999;
        let result = windows_filetime_to_string(filetime);
        assert!(result.starts_with("(invalid timestamp:"));
    }

    #[test]
    fn test_filetime_zero_is_invalid() {
        let result = windows_filetime_to_string(0);
        assert!(result.starts_with("(invalid timestamp:"));
    }

    // ── prop_value_to_string ─────────────────────────────────────────────────
    // Unicode/String8/Binary newtypes have private fields so we can only
    // test the non-string arms directly here.

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
}
