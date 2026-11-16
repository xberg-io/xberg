//! Error text shown to remote callers of the REST API and the MCP HTTP transport.
//!
//! Errors backed by the server filesystem get a fixed public message, because their text can
//! name a path in a form no scrubber reliably finds. Other errors keep their text with server
//! paths scrubbed, as defense in depth. The full error is always logged server-side.

use crate::XbergError;

/// Placeholder for a filesystem path removed from an error message sent to a client.
pub(crate) const REDACTED_PATH: &str = "<path>";

const IO_NOT_FOUND_MESSAGE: &str = "IO error: file not found";
const IO_PERMISSION_DENIED_MESSAGE: &str = "IO error: permission denied";
const IO_ERROR_MESSAGE: &str = "IO error: a file could not be read or written";
const CACHE_ERROR_MESSAGE: &str = "Cache error: the cache could not be read or written";

/// Characters that may wrap a path inside a message token, such as quotes or a trailing colon.
const QUOTES: [char; 3] = ['\'', '"', '`'];
const LEADING_PATH_DELIMITERS: &[char] = &['\'', '"', '`', '(', '[', '{', '<'];
const TRAILING_PATH_DELIMITERS: &[char] = &['\'', '"', '`', ')', ']', '}', '>', ',', ';', ':', '.'];

/// The message a remote caller sees for `error`, whose full text is `full_message`.
pub(crate) fn public_error_message(error: &XbergError, full_message: &str) -> String {
    filesystem_error_message(error).map_or_else(|| redact_paths(full_message), str::to_string)
}

/// The fixed public message of an error backed by the server filesystem: an I/O error (missing
/// file, permission, any other failed read or write) or a cache error. `None` for other kinds.
pub(crate) fn filesystem_error_message(error: &XbergError) -> Option<&'static str> {
    match error {
        XbergError::Io(io) => Some(match io.kind() {
            std::io::ErrorKind::NotFound => IO_NOT_FOUND_MESSAGE,
            std::io::ErrorKind::PermissionDenied => IO_PERMISSION_DENIED_MESSAGE,
            _ => IO_ERROR_MESSAGE,
        }),
        XbergError::Cache { .. } => Some(CACHE_ERROR_MESSAGE),
        _ => None,
    }
}

/// [`filesystem_error_message`] for a per-input error, which carries only the kind's name from
/// `XbergError::extraction_error_type`.
fn filesystem_error_type_message(error_type: &str) -> Option<&'static str> {
    match error_type {
        "io" => Some(IO_ERROR_MESSAGE),
        "cache" => Some(CACHE_ERROR_MESSAGE),
        _ => None,
    }
}

/// Give per-input batch errors their public message, logging each original that changes.
pub(crate) fn redact_error_items(errors: &mut [super::ExtractionErrorItem]) {
    for item in errors {
        let message =
            filesystem_error_type_message(&item.error_type).map_or_else(|| redact_paths(&item.message), str::to_string);
        if message != item.message {
            tracing::warn!(
                index = item.index,
                error_type = %item.error_type,
                error = %item.message,
                "Extraction input failed"
            );
            item.message = message;
        }
    }
}

/// Replace every filesystem path in `message` with [`REDACTED_PATH`].
///
/// A quoted path is replaced up to its closing quote, spaces included. Otherwise a path is a
/// whitespace-separated token (or the value after `=` in one), stripped of wrapping quotes and
/// brackets, that is absolute (`/srv/models`), home-relative (`~/models`), a Windows drive or UNC
/// path, or a `file://` URI. URLs, MIME types and relative names are kept, so the message still
/// says what failed. An unquoted path containing spaces keeps the part after its first space.
pub(crate) fn redact_paths(message: &str) -> String {
    redact_path_tokens(&redact_quoted_paths(message))
}

fn redact_quoted_paths(message: &str) -> String {
    let mut redacted = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(open) = rest.find(QUOTES) {
        let quote = &rest[open..=open];
        let after = &rest[open + 1..];
        redacted.push_str(&rest[..=open]);
        match after.find(quote) {
            Some(close) if is_filesystem_path(&after[..close]) => {
                redacted.push_str(REDACTED_PATH);
                redacted.push_str(quote);
                rest = &after[close + 1..];
            }
            _ => rest = after,
        }
    }
    redacted.push_str(rest);
    redacted
}

fn redact_path_tokens(message: &str) -> String {
    let mut redacted = String::with_capacity(message.len());
    for piece in message.split_inclusive(char::is_whitespace) {
        let token = piece.trim_end_matches(char::is_whitespace);
        let value_start = token.find('=').map_or(0, |index| index + 1);
        let (prefix, value) = token.split_at(value_start);
        let start = value.len() - value.trim_start_matches(LEADING_PATH_DELIMITERS).len();
        let end = value.trim_end_matches(TRAILING_PATH_DELIMITERS).len().max(start);
        if is_filesystem_path(&value[start..end]) {
            redacted.push_str(prefix);
            redacted.push_str(&value[..start]);
            redacted.push_str(REDACTED_PATH);
            redacted.push_str(&value[end..]);
        } else {
            redacted.push_str(token);
        }
        redacted.push_str(&piece[token.len()..]);
    }
    redacted
}

fn is_filesystem_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    let unix_absolute = bytes.first() == Some(&b'/') && bytes.len() > 1 && bytes[1] != b'/';
    let windows_drive =
        bytes.len() > 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && matches!(bytes[2], b'\\' | b'/');
    let file_uri = value.len() > "file://".len() && value.starts_with("file://");
    unix_absolute || windows_drive || file_uri || value.starts_with("~/") || value.starts_with("\\\\")
}

#[cfg(test)]
mod tests {
    use super::*;

    const POSIX_PATH: &str = "/srv/my docs/a.pdf";
    const WINDOWS_PATH: &str = "C:\\Program Files\\x\\a.pdf";

    /// Every fragment of the synthetic paths that must not reach a client.
    const PATH_FRAGMENTS: &[&str] = &["srv", "my docs", "docs/", "a.pdf", "Program", "Files", "\\x"];

    fn assert_no_path_fragment(message: &str) {
        for fragment in PATH_FRAGMENTS {
            assert!(!message.contains(fragment), "{fragment:?} survived in {message:?}");
        }
    }

    #[test]
    fn should_give_filesystem_errors_a_fixed_message_without_any_path_fragment() {
        let cases = [
            (
                XbergError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("File does not exist: \"{POSIX_PATH}\""),
                )),
                IO_NOT_FOUND_MESSAGE,
            ),
            (
                XbergError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("cannot open '{WINDOWS_PATH}'"),
                )),
                IO_PERMISSION_DENIED_MESSAGE,
            ),
            (
                XbergError::Io(std::io::Error::other(format!(
                    "write failed: {POSIX_PATH} and {WINDOWS_PATH}"
                ))),
                IO_ERROR_MESSAGE,
            ),
            (
                XbergError::cache(format!("Failed to read cache directory {POSIX_PATH}")),
                CACHE_ERROR_MESSAGE,
            ),
        ];

        for (error, expected) in cases {
            let full = error.to_string();
            let message = public_error_message(&error, &full);
            assert_eq!(message, expected, "{full}");
            assert_no_path_fragment(&message);
        }
    }

    #[test]
    fn should_redact_quoted_posix_and_windows_paths_with_spaces() {
        let message = format!("Parsing error: cannot open \"{POSIX_PATH}\" or '{WINDOWS_PATH}': truncated");

        let redacted = redact_paths(&message);

        assert_eq!(redacted, "Parsing error: cannot open \"<path>\" or '<path>': truncated");
        assert_no_path_fragment(&redacted);
        let error = XbergError::parsing(format!("cannot open `{POSIX_PATH}`"));
        let public = public_error_message(&error, &error.to_string());
        assert_eq!(public, "Parsing error: cannot open `<path>`");
        assert_no_path_fragment(&public);
    }

    #[test]
    fn should_keep_apostrophes_and_quoted_non_paths() {
        for message in [
            "Plugin error in 'ocr': can't read the page",
            "Unknown embedding preset 'fast'. Available: balanced, quality",
        ] {
            assert_eq!(redact_paths(message), message);
        }
        assert_eq!(redact_paths("it's at '/srv/my docs/a.pdf'"), "it's at '<path>'");
    }

    #[test]
    fn should_redact_server_paths_in_per_input_batch_errors() {
        let item = |index, error_type: &str, message: &str| super::super::ExtractionErrorItem {
            index,
            code: 2,
            error_type: error_type.to_string(),
            source: "report.pdf".to_string(),
            message: message.to_string(),
        };
        let mut errors = vec![
            item(
                0,
                "parsing",
                "Parsing error: cannot open /srv/xberg/tmp/upload-7.pdf: truncated",
            ),
            item(
                1,
                "unsupported_format",
                "Unsupported format: application/x-unsupported-format",
            ),
            item(2, "io", &format!("IO error: cannot read \"{POSIX_PATH}\"")),
            item(3, "cache", &format!("Cache error: {WINDOWS_PATH} is full")),
        ];

        redact_error_items(&mut errors);

        assert_eq!(errors[0].message, "Parsing error: cannot open <path>: truncated");
        assert_eq!(errors[0].index, 0);
        assert_eq!(errors[0].error_type, "parsing");
        assert_eq!(errors[0].source, "report.pdf");
        assert_eq!(
            errors[1].message,
            "Unsupported format: application/x-unsupported-format"
        );
        assert_eq!(errors[2].message, IO_ERROR_MESSAGE);
        assert_eq!(errors[3].message, CACHE_ERROR_MESSAGE);
        for item in &errors {
            assert_no_path_fragment(&item.message);
        }
    }
}
