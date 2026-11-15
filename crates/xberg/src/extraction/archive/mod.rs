//! Archive extraction functionality.
//!
//! This module provides functions for extracting file lists and contents from archives.
//! Supported formats:
//! - ZIP archives
//! - TAR archives (including compressed TAR.GZ, TAR.BZ2)
//! - 7Z archives
//! - GZIP archives
//!
//! Each format has its own submodule with specialized extraction logic.

use std::io::{self, Read, Write};

mod gzip;
mod sevenz;
mod tar;
mod zip;

pub(crate) use gzip::extract_gzip_with_bytes;
#[cfg(test)]
pub(crate) use gzip::{decompress_gzip, extract_gzip, extract_gzip_metadata, extract_gzip_text_content};
pub(crate) use sevenz::{extract_7z_file_bytes, extract_7z_metadata, extract_7z_text_content};
pub(crate) use tar::{extract_tar_file_bytes, extract_tar_metadata, extract_tar_text_content};
pub(crate) use zip::{extract_zip_file_bytes, extract_zip_metadata, extract_zip_text_content};

/// Archive metadata extracted from an archive file.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct ArchiveMetadata {
    /// Archive format (e.g., "ZIP", "TAR")
    pub format: String,
    /// List of files in the archive
    pub file_list: Vec<ArchiveEntry>,
    /// Total number of files
    pub file_count: usize,
    /// Total uncompressed size in bytes
    pub total_size: u64,
}

/// Information about a single file in an archive.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// File path within the archive, copied **verbatim** from the archive's own entry
    /// name (`zip::read::ZipFile::name()`, `tar::Entry::path()`, ...).
    ///
    /// This value is untrusted and unnormalised: a hostile or malformed archive can make
    /// it an absolute path (`/etc/passwd`), a traversing relative path
    /// (`../../etc/passwd`), or a Windows drive-letter/UNC form. Nothing in this module
    /// rejects or rewrites those forms, because nothing here writes archive contents to a
    /// real filesystem path built from this string -- it is only ever used as an opaque
    /// map key (`extract_zip_text_content`/`extract_zip_file_bytes` and their TAR
    /// equivalents) or surfaced as informational metadata.
    ///
    /// A future caller that *does* want to write an entry to disk (a CLI "extract to
    /// directory" feature, an FFI binding, ...) must never join this value onto a real
    /// path directly. Use [`ArchiveEntry::confined_path`] instead, which returns a
    /// normalised path guaranteed not to escape the archive root, or `None` when the raw
    /// name cannot be confined at all.
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// Whether this is a directory
    pub is_dir: bool,
}

impl ArchiveEntry {
    /// Returns [`Self::path`] normalised and confined to the archive root, or `None` when
    /// it cannot be confined safely.
    ///
    /// This is the safe counterpart to the raw `path` field: it rejects (via `None`)
    /// exactly the cases that make `path` unsafe to use as a filesystem write target --
    /// a `..` that pops past the root, a NUL byte, and a Windows drive letter or UNC
    /// prefix -- and otherwise returns a `/`-joined, root-relative path with `.` and
    /// empty segments removed.
    ///
    /// Backslashes are normalised to `/` first, since a hostile archive can store a
    /// backslash-separated name that native path handling would treat as a single opaque
    /// segment on Unix rather than as traversal components.
    ///
    /// This function does not decide *what* to do with an unconfined entry (skip it, warn,
    /// abort the whole archive, ...); that policy belongs to the caller that would
    /// otherwise write the entry somewhere.
    pub fn confined_path(&self) -> Option<String> {
        if self.path.contains('\0') {
            return None;
        }

        let normalized = self.path.replace('\\', "/");
        if has_drive_or_unc_prefix(&normalized) {
            return None;
        }

        let mut stack: Vec<&str> = Vec::new();
        for segment in normalized.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    stack.pop()?;
                }
                other => stack.push(other),
            }
        }

        if stack.is_empty() {
            return None;
        }

        Some(stack.join("/"))
    }
}

/// `true` when `path` begins with a Windows drive letter (`C:`) or a UNC prefix (`//`,
/// which is what a backslash-normalised `\\server\share` becomes).
fn has_drive_or_unc_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with("//") || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
}

fn add_declared_size(total: u64, size: u64) -> crate::error::Result<u64> {
    total.checked_add(size).ok_or_else(|| {
        crate::error::XbergError::validation("Archive declared uncompressed size overflows u64".to_string())
    })
}

pub(crate) struct DecompressedBudget<'a> {
    remaining: u64,
    shared: Option<&'a crate::extractors::security::ArchiveBudget>,
}

impl<'a> DecompressedBudget<'a> {
    #[cfg(test)]
    fn new(limit: u64) -> Self {
        Self {
            remaining: limit,
            shared: None,
        }
    }

    pub(crate) fn shared(budget: &'a crate::extractors::security::ArchiveBudget) -> Self {
        Self {
            remaining: budget.remaining_decompressed() as u64,
            shared: Some(budget),
        }
    }

    fn read_member(
        &mut self,
        reader: impl Read,
        output: &mut impl Write,
        content_limit: Option<u64>,
        member: &str,
    ) -> crate::error::Result<io::Result<()>> {
        let cap = content_limit.unwrap_or(u64::MAX).min(self.remaining);
        let probe_limit = cap.saturating_add(1);
        let mut limited = reader.take(probe_limit);
        let result = io::copy(&mut limited, output);
        // Take tracks successful reads even if a subsequent read fails integrity validation. ~keep
        let consumed = probe_limit - limited.limit();
        // A rejected overflow probe spends the remaining budget without exceeding it. ~keep
        let charged = consumed.min(self.remaining);
        self.remaining -= charged;
        if let Some(shared) = self.shared {
            shared
                .charge_decompressed(charged as usize)
                .map_err(|error| crate::error::XbergError::validation(error.to_string()))?;
        }
        if consumed > cap {
            let limit_name = if content_limit.is_some_and(|limit| limit <= cap) {
                "max_content_size"
            } else {
                "max_archive_size"
            };
            return Err(crate::error::XbergError::validation(format!(
                "Archive member '{member}' exceeds {limit_name} while reading ({cap} bytes available)"
            )));
        }
        Ok(result.map(|_| ()))
    }
}

/// Common text file extensions that should be extracted from archives.
///
/// #113: the original list only covered a handful of formats, so real
/// plain-text members (source code, config files, alternate markup/data
/// formats) were treated as binary and skipped. Widened to cover the text
/// families xberg already extracts elsewhere in the pipeline.
pub(crate) const TEXT_EXTENSIONS: &[&str] = &[
    ".txt",
    ".md",
    ".markdown",
    ".json",
    ".jsonl",
    ".ndjson",
    ".xml",
    ".html",
    ".htm",
    ".csv",
    ".tsv",
    ".log",
    ".yaml",
    ".yml",
    ".toml",
    ".ini",
    ".cfg",
    ".conf",
    ".properties",
    ".env",
    ".rst",
    ".adoc",
    ".tex",
    ".sql",
    ".rs",
    ".py",
    ".js",
    ".mjs",
    ".cjs",
    ".ts",
    ".tsx",
    ".jsx",
    ".go",
    ".java",
    ".kt",
    ".rb",
    ".php",
    ".c",
    ".h",
    ".cpp",
    ".cc",
    ".hpp",
    ".cs",
    ".swift",
    ".sh",
    ".bash",
    ".zsh",
    ".ps1",
    ".css",
    ".scss",
    ".less",
    ".svg",
    ".gitignore",
];

/// Decode an archive text member's bytes to a string, detecting the charset
/// instead of dropping non-UTF-8 members. Warns when the bytes weren't clean
/// UTF-8 so a mojibake member is at least visible (xberg-io/xberg#1223).
///
/// `decode_with_provenance` (#395) reports whether the decode actually lost data --
/// via `replaced_characters` -- at the point the decision is made, which is folded
/// into this same warning rather than emitted separately: scanning the returned
/// `String` for U+FFFD afterwards would be blind to that under the `quality`
/// feature, whose mojibake cleanup strips replacement characters before this
/// function's caller ever sees the text.
pub(crate) fn decode_archive_text(bytes: &[u8], member: &str) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => crate::utils::strip_bom(s).to_string(),
        Err(_) => {
            let outcome = crate::utils::decode_with_provenance(bytes, None);
            tracing::warn!(
                member = %member,
                replaced_characters = outcome.replaced_characters,
                "archive member is not valid UTF-8; decoding with charset detection"
            );
            crate::utils::strip_bom(&outcome.text).to_string()
        }
    }
}

#[cfg(test)]
mod tests;
