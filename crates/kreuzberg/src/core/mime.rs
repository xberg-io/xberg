//! MIME type detection and validation.
//!
//! This module provides utilities for detecting MIME types from file extensions
//! and validating them against supported types.
//!
//! Format information is centralized in the [`FORMATS`] registry. All extension-to-MIME
//! mappings and supported MIME type validation are derived from this single source of truth.

use crate::{KreuzbergError, Result};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// A supported document format entry.
///
/// Represents a file extension and its corresponding MIME type that Kreuzberg can process.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "api", derive(utoipa::ToSchema))]
pub struct SupportedFormat {
    /// File extension (without leading dot), e.g., "pdf", "docx"
    pub extension: String,
    /// MIME type string, e.g., "application/pdf"
    pub mime_type: String,
}

pub const HTML_MIME_TYPE: &str = "text/html";
pub const MARKDOWN_MIME_TYPE: &str = "text/markdown";
pub const PDF_MIME_TYPE: &str = "application/pdf";
pub const PLAIN_TEXT_MIME_TYPE: &str = "text/plain";
pub const POWER_POINT_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";
pub const DOCX_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
pub const LEGACY_WORD_MIME_TYPE: &str = "application/msword";
pub const LEGACY_POWERPOINT_MIME_TYPE: &str = "application/vnd.ms-powerpoint";

pub const EML_MIME_TYPE: &str = "message/rfc822";
pub const MSG_MIME_TYPE: &str = "application/vnd.ms-outlook";
pub const PST_MIME_TYPE: &str = "application/vnd.ms-outlook-pst";
pub const JSON_MIME_TYPE: &str = "application/json";
pub const JSONL_MIME_TYPE: &str = "application/x-ndjson";
pub const YAML_MIME_TYPE: &str = "application/x-yaml";
pub const TOML_MIME_TYPE: &str = "application/toml";
pub const XML_MIME_TYPE: &str = "application/xml";
pub const XML_TEXT_MIME_TYPE: &str = "text/xml";
pub const SVG_MIME_TYPE: &str = "image/svg+xml";
pub const SOURCE_CODE_MIME_TYPE: &str = "text/x-source-code";

pub const EXCEL_MIME_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
pub const EXCEL_BINARY_MIME_TYPE: &str = "application/vnd.ms-excel";
pub const EXCEL_MACRO_MIME_TYPE: &str = "application/vnd.ms-excel.sheet.macroEnabled.12";
pub const EXCEL_BINARY_2007_MIME_TYPE: &str = "application/vnd.ms-excel.sheet.binary.macroEnabled.12";
pub const EXCEL_ADDON_MIME_TYPE: &str = "application/vnd.ms-excel.addin.macroEnabled.12";
pub const EXCEL_TEMPLATE_MIME_TYPE: &str = "application/vnd.ms-excel.template.macroEnabled.12";

pub const OPENDOC_SPREADSHEET_MIME_TYPE: &str = "application/vnd.oasis.opendocument.spreadsheet";

pub const IWORK_PAGES_MIME_TYPE: &str = "application/x-iwork-pages-sffpages";
pub const IWORK_NUMBERS_MIME_TYPE: &str = "application/x-iwork-numbers-sffnumbers";
pub const IWORK_KEYNOTE_MIME_TYPE: &str = "application/x-iwork-keynote-sffkey";

/// A format definition in the centralized registry.
///
/// Each entry defines a document format with its file extensions, primary MIME type,
/// and any MIME type aliases that should also be accepted for this format.
struct FormatEntry {
    /// File extensions (without leading dot). First is canonical.
    extensions: &'static [&'static str],
    /// Primary MIME type for this format.
    mime_type: &'static str,
    /// Additional MIME type aliases that should also be accepted.
    aliases: &'static [&'static str],
}

/// Centralized format registry - the single source of truth for all supported formats.
///
/// Adding a new format requires only adding a single entry here. Both `EXT_TO_MIME`
/// (extension-to-MIME mapping) and `SUPPORTED_MIME_TYPES` (validation set) are
/// derived from this array automatically.
static FORMATS: &[FormatEntry] = &[
    // ── Plain text ──────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["txt"],
        mime_type: "text/plain",
        aliases: &[],
    },
    // Plain text variants handled by extractors (no file extension mapping)
    FormatEntry {
        extensions: &[],
        mime_type: "text/troff",
        aliases: &[],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "text/x-mdoc",
        aliases: &[],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "text/x-pod",
        aliases: &[],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "text/x-dokuwiki",
        aliases: &[],
    },
    // ── Markdown ────────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["md", "markdown"],
        mime_type: "text/markdown",
        aliases: &["text/x-markdown"],
    },
    FormatEntry {
        extensions: &["commonmark"],
        mime_type: "text/x-commonmark",
        aliases: &[],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "text/x-gfm",
        aliases: &[],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "text/x-markdown-extra",
        aliases: &[],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "text/x-multimarkdown",
        aliases: &[],
    },
    // ── MDX ─────────────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["mdx"],
        mime_type: "text/mdx",
        aliases: &["text/x-mdx"],
    },
    // ── Djot ────────────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["djot"],
        mime_type: "text/x-djot",
        aliases: &["text/djot"],
    },
    // ── PDF ─────────────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["pdf"],
        mime_type: "application/pdf",
        aliases: &[],
    },
    // ── HTML ────────────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["html", "htm"],
        mime_type: "text/html",
        aliases: &["application/xhtml+xml"],
    },
    // ── Word processing ─────────────────────────────────────────────────
    FormatEntry {
        extensions: &["docx"],
        mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["docm"],
        mime_type: "application/vnd.ms-word.document.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["dotx"],
        mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["dotm"],
        mime_type: "application/vnd.ms-word.template.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["doc", "dot"],
        mime_type: "application/msword",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["odt"],
        mime_type: "application/vnd.oasis.opendocument.text",
        aliases: &[],
    },
    // ── Presentations ───────────────────────────────────────────────────
    FormatEntry {
        extensions: &["pptx"],
        mime_type: "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["ppsx"],
        mime_type: "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["pptm"],
        mime_type: "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["potx"],
        mime_type: "application/vnd.openxmlformats-officedocument.presentationml.template",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["potm"],
        mime_type: "application/vnd.ms-powerpoint.template.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["ppt", "pot"],
        mime_type: "application/vnd.ms-powerpoint",
        aliases: &[],
    },
    // ── Spreadsheets ────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["xlsx"],
        mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xltx"],
        mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.template",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xls", "xlt"],
        mime_type: "application/vnd.ms-excel",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xlsm"],
        mime_type: "application/vnd.ms-excel.sheet.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xlsb"],
        mime_type: "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xlam"],
        mime_type: "application/vnd.ms-excel.addin.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["xla"],
        mime_type: "application/vnd.ms-excel.template.macroEnabled.12",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["ods"],
        mime_type: "application/vnd.oasis.opendocument.spreadsheet",
        aliases: &[],
    },
    // ── dBASE ──────────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["dbf"],
        mime_type: "application/x-dbf",
        aliases: &["application/dbase"],
    },
    // ── Hangul ─────────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["hwp"],
        mime_type: "application/x-hwp",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["hwpx"],
        mime_type: "application/haansofthwpx",
        aliases: &[],
    },
    // ── Images ──────────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["bmp"],
        mime_type: "image/bmp",
        aliases: &["image/x-bmp", "image/x-ms-bmp"],
    },
    FormatEntry {
        extensions: &["gif"],
        mime_type: "image/gif",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["jpg", "jpeg"],
        mime_type: "image/jpeg",
        aliases: &["image/pjpeg", "image/jpg"],
    },
    FormatEntry {
        extensions: &["png"],
        mime_type: "image/png",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["tiff", "tif"],
        mime_type: "image/tiff",
        aliases: &["image/x-tiff"],
    },
    FormatEntry {
        extensions: &["webp"],
        mime_type: "image/webp",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["jp2", "j2k", "j2c"],
        mime_type: "image/jp2",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["jpx"],
        mime_type: "image/jpx",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["jpm"],
        mime_type: "image/jpm",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["mj2"],
        mime_type: "image/mj2",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["jbig2", "jb2"],
        mime_type: "image/x-jbig2",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["pnm"],
        mime_type: "image/x-portable-anymap",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["pbm"],
        mime_type: "image/x-portable-bitmap",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["pgm"],
        mime_type: "image/x-portable-graymap",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["ppm"],
        mime_type: "image/x-portable-pixmap",
        aliases: &[],
    },
    // ── Data formats ────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["csv"],
        mime_type: "text/csv",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["tsv"],
        mime_type: "text/tab-separated-values",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["json"],
        mime_type: "application/json",
        aliases: &["text/json"],
    },
    FormatEntry {
        extensions: &[],
        mime_type: "application/csl+json",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["jsonl", "ndjson"],
        mime_type: "application/x-ndjson",
        aliases: &["application/jsonl", "application/x-jsonlines"],
    },
    FormatEntry {
        extensions: &["yaml", "yml"],
        mime_type: "application/x-yaml",
        aliases: &["text/yaml", "text/x-yaml", "application/yaml"],
    },
    FormatEntry {
        extensions: &["toml"],
        mime_type: "application/toml",
        aliases: &["text/toml"],
    },
    FormatEntry {
        extensions: &["xml"],
        mime_type: "application/xml",
        aliases: &["text/xml"],
    },
    FormatEntry {
        extensions: &["svg"],
        mime_type: "image/svg+xml",
        aliases: &[],
    },
    // ── Email ───────────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["eml"],
        mime_type: "message/rfc822",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["msg"],
        mime_type: "application/vnd.ms-outlook",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["pst"],
        mime_type: "application/vnd.ms-outlook-pst",
        aliases: &[],
    },
    // ── Archives ────────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["zip"],
        mime_type: "application/zip",
        aliases: &["application/x-zip-compressed"],
    },
    FormatEntry {
        extensions: &["tar"],
        mime_type: "application/x-tar",
        aliases: &["application/tar", "application/x-gtar", "application/x-ustar"],
    },
    FormatEntry {
        extensions: &["gz", "tgz"],
        mime_type: "application/gzip",
        aliases: &["application/x-gzip"],
    },
    FormatEntry {
        extensions: &["7z"],
        mime_type: "application/x-7z-compressed",
        aliases: &[],
    },
    // ── Document / academic formats ─────────────────────────────────────
    FormatEntry {
        extensions: &["rst"],
        mime_type: "text/x-rst",
        aliases: &["text/prs.fallenstein.rst"],
    },
    FormatEntry {
        extensions: &["org"],
        mime_type: "text/x-org",
        aliases: &["text/org", "application/x-org"],
    },
    FormatEntry {
        extensions: &["epub"],
        mime_type: "application/epub+zip",
        aliases: &["application/x-epub+zip", "application/vnd.epub+zip"],
    },
    FormatEntry {
        extensions: &["rtf"],
        mime_type: "application/rtf",
        aliases: &["text/rtf"],
    },
    FormatEntry {
        extensions: &["bib"],
        mime_type: "application/x-bibtex",
        aliases: &["text/x-bibtex", "application/x-biblatex"],
    },
    FormatEntry {
        extensions: &["ris"],
        mime_type: "application/x-research-info-systems",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["nbib"],
        mime_type: "application/x-pubmed",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["enw"],
        mime_type: "application/x-endnote+xml",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["fb2"],
        mime_type: "application/x-fictionbook+xml",
        aliases: &["application/x-fictionbook", "text/x-fictionbook"],
    },
    FormatEntry {
        extensions: &["opml"],
        mime_type: "application/xml+opml",
        aliases: &["application/x-opml+xml", "text/x-opml"],
    },
    FormatEntry {
        extensions: &["dbk", "docbook", "docbook4", "docbook5"],
        mime_type: "application/docbook+xml",
        aliases: &["text/docbook"],
    },
    FormatEntry {
        extensions: &["jats"],
        mime_type: "application/x-jats+xml",
        aliases: &["text/jats"],
    },
    FormatEntry {
        extensions: &["ipynb"],
        mime_type: "application/x-ipynb+json",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["tex", "latex"],
        mime_type: "application/x-latex",
        aliases: &["text/x-tex"],
    },
    FormatEntry {
        extensions: &["typst", "typ"],
        mime_type: "application/x-typst",
        aliases: &["text/x-typst"],
    },
    // ── Apple iWork ─────────────────────────────────────────────────────
    FormatEntry {
        extensions: &["pages"],
        mime_type: "application/x-iwork-pages-sffpages",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["numbers"],
        mime_type: "application/x-iwork-numbers-sffnumbers",
        aliases: &[],
    },
    FormatEntry {
        extensions: &["key"],
        mime_type: "application/x-iwork-keynote-sffkey",
        aliases: &[],
    },
    // ── Source code (tree-sitter) ──────────────────────────────────────
    // No file extension mapping — detection is dynamic via TSLP's
    // detect_language_from_extension() as a fallback in detect_mime_type().
    FormatEntry {
        extensions: &[],
        mime_type: "text/x-source-code",
        aliases: &[],
    },
];

/// Extension to MIME type mapping, derived from [`FORMATS`].
static EXT_TO_MIME: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for entry in FORMATS {
        for ext in entry.extensions {
            m.insert(*ext, entry.mime_type);
        }
    }
    m
});

/// All supported MIME types (primary + aliases), derived from [`FORMATS`].
static SUPPORTED_MIME_TYPES: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();
    for entry in FORMATS {
        set.insert(entry.mime_type);
        for alias in entry.aliases {
            set.insert(*alias);
        }
    }
    set
});

/// Detect MIME type from a file path.
///
/// Uses file extension to determine MIME type. Falls back to `mime_guess` crate
/// if extension-based detection fails.
///
/// # Arguments
///
/// * `path` - Path to the file
/// * `check_exists` - Whether to verify file existence
///
/// # Returns
///
/// The detected MIME type string.
///
/// # Errors
///
/// Returns `KreuzbergError::Io` if file doesn't exist (when `check_exists` is true).
/// Returns `KreuzbergError::UnsupportedFormat` if MIME type cannot be determined.
pub fn detect_mime_type(path: impl AsRef<Path>, check_exists: bool) -> Result<String> {
    let path = path.as_ref();

    if check_exists && !path.exists() {
        return Err(KreuzbergError::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File does not exist: {}", path.display()),
        )));
    }

    let extension = path.extension().and_then(|ext| ext.to_str()).map(|s| s.to_lowercase());
    tracing::debug!(path = %path.display(), extension = ?extension, "detecting MIME type from path");

    if let Some(ext) = &extension
        && let Some(mime_type) = EXT_TO_MIME.get(ext.as_str())
    {
        tracing::debug!(ext = %ext, mime_type = %mime_type, "matched via EXT_TO_MIME");
        return Ok(mime_type.to_string());
    }

    // Tree-sitter detection: check if the extension belongs to a known
    // programming language *before* falling back to mime_guess, which returns
    // language-specific MIME types (e.g. "text/x-python") that are not in our
    // supported set.
    #[cfg(feature = "tree-sitter")]
    {
        if let Some(ext) = &extension {
            let lang = tree_sitter_language_pack::detect_language_from_extension(ext);
            tracing::debug!(ext = %ext, detected_language = ?lang, "tree-sitter extension detection");
            if lang.is_some() {
                return Ok(SOURCE_CODE_MIME_TYPE.to_string());
            }
        }
    }

    let guess = mime_guess::from_path(path).first();
    tracing::debug!(guess = ?guess, "mime_guess fallback");
    if let Some(mime) = guess {
        return Ok(mime.to_string());
    }

    if let Some(ext) = extension {
        return Err(KreuzbergError::UnsupportedFormat(format!(
            "Unknown extension: .{}",
            ext
        )));
    }

    Err(KreuzbergError::validation(format!(
        "Could not determine MIME type from file path: {}",
        path.display()
    )))
}

/// Validate that a MIME type is supported.
///
/// # Arguments
///
/// * `mime_type` - The MIME type to validate
///
/// # Returns
///
/// The validated MIME type (may be normalized).
///
/// # Errors
///
/// Returns `KreuzbergError::UnsupportedFormat` if not supported.
pub fn validate_mime_type(mime_type: &str) -> Result<String> {
    if SUPPORTED_MIME_TYPES.contains(mime_type) {
        tracing::trace!(mime_type = %mime_type, "MIME type validated (exact match)");
        return Ok(mime_type.to_string());
    }

    if mime_type.starts_with("image/") {
        tracing::trace!(mime_type = %mime_type, "MIME type validated (image prefix)");
        return Ok(mime_type.to_string());
    }

    // Case-insensitive fallback: MIME types are case-insensitive per RFC 2045.
    // This handles common mismatches like "macroEnabled" vs "macroenabled".
    let lower = mime_type.to_ascii_lowercase();
    for supported in SUPPORTED_MIME_TYPES.iter() {
        if supported.to_ascii_lowercase() == lower {
            tracing::trace!(mime_type = %mime_type, matched = %supported, "MIME type validated (case-insensitive)");
            return Ok(supported.to_string());
        }
    }

    tracing::debug!(mime_type = %mime_type, "MIME type not in supported set");
    Err(KreuzbergError::UnsupportedFormat(mime_type.to_string()))
}

/// Detect or validate MIME type.
///
/// If `mime_type` is provided, validates it. Otherwise, detects from `path`.
///
/// # Arguments
///
/// * `path` - Optional path to detect MIME type from
/// * `mime_type` - Optional explicit MIME type to validate
///
/// # Returns
///
/// The validated MIME type string.
pub fn detect_or_validate(path: Option<&Path>, mime_type: Option<&str>) -> Result<String> {
    if let Some(mime) = mime_type {
        tracing::debug!(mime_type = %mime, "validating caller-provided MIME type");
        validate_mime_type(mime)
    } else if let Some(p) = path {
        let detected = detect_mime_type(p, true)?;
        tracing::debug!(path = %p.display(), detected = %detected, "detected MIME, now validating");
        validate_mime_type(&detected)
    } else {
        Err(KreuzbergError::validation(
            "Must provide either path or mime_type".to_string(),
        ))
    }
}

/// Detect MIME type from raw file bytes.
///
/// Uses magic byte signatures to detect file type from content.
/// Falls back to `infer` crate for comprehensive detection.
///
/// For ZIP-based files, inspects contents to distinguish Office Open XML
/// formats (DOCX, XLSX, PPTX) from plain ZIP archives.
///
/// # Arguments
///
/// * `content` - Raw file bytes
///
/// # Returns
///
/// The detected MIME type string.
///
/// # Errors
///
/// Returns `KreuzbergError::UnsupportedFormat` if MIME type cannot be determined.
pub fn detect_mime_type_from_bytes(content: &[u8]) -> Result<String> {
    if let Some(kind) = infer::get(content) {
        let mime_type = kind.mime_type();

        // Check if ZIP is actually an Office Open XML format
        if mime_type == "application/zip"
            && let Some(office_mime) = detect_office_format_from_zip(content)
        {
            return Ok(office_mime.to_string());
        }

        if SUPPORTED_MIME_TYPES.contains(mime_type) || mime_type.starts_with("image/") {
            return Ok(mime_type.to_string());
        }
    }

    // PST (Outlook Personal Folders) magic signature: "!BDN" at offset 0
    if content.len() >= 4 && content[..4] == [0x21, 0x42, 0x44, 0x4E] {
        return Ok(PST_MIME_TYPE.to_string());
    }

    if let Ok(text) = std::str::from_utf8(content) {
        let trimmed = text.trim_start();

        if (trimmed.starts_with('{') || trimmed.starts_with('['))
            && serde_json::from_str::<serde_json::Value>(text).is_ok()
        {
            return Ok(JSON_MIME_TYPE.to_string());
        }

        if trimmed.starts_with("<?xml") || trimmed.starts_with('<') {
            return Ok(XML_MIME_TYPE.to_string());
        }

        if trimmed.starts_with("<!DOCTYPE html") || trimmed.starts_with("<html") {
            return Ok(HTML_MIME_TYPE.to_string());
        }

        if trimmed.starts_with("%PDF") {
            return Ok(PDF_MIME_TYPE.to_string());
        }

        // Tree-sitter fallback: detect language from shebang line.
        #[cfg(feature = "tree-sitter")]
        if tree_sitter_language_pack::detect_language_from_content(trimmed).is_some() {
            return Ok(SOURCE_CODE_MIME_TYPE.to_string());
        }

        return Ok(PLAIN_TEXT_MIME_TYPE.to_string());
    }

    Err(KreuzbergError::UnsupportedFormat(
        "Could not determine MIME type from bytes".to_string(),
    ))
}

/// Detect Office Open XML format from ZIP content by scanning for marker files.
///
/// Office Open XML formats (DOCX, XLSX, PPTX) are ZIP archives containing specific
/// XML files that identify the format:
/// - DOCX: contains `word/document.xml`
/// - XLSX: contains `xl/workbook.xml`
/// - PPTX: contains `ppt/presentation.xml`
///
/// Apple iWork formats (2013+) also use ZIP with IWA files:
/// - Pages: contains `Index/Document.iwa`
/// - Numbers: contains `Index/CalculationEngine.iwa`
/// - Keynote: contains `Index/Presentation.iwa`
///
/// This function scans the ZIP's local file headers without fully parsing the archive,
/// making it efficient for MIME type detection.
fn detect_office_format_from_zip(content: &[u8]) -> Option<&'static str> {
    // Office format markers - these are file paths within the ZIP that identify the format
    const DOCX_MARKER: &[u8] = b"word/document.xml";
    const XLSX_MARKER: &[u8] = b"xl/workbook.xml";
    const PPTX_MARKER: &[u8] = b"ppt/presentation.xml";

    // Apple iWork markers
    const PAGES_MARKER: &[u8] = b"Index/Document.iwa";
    const NUMBERS_MARKER: &[u8] = b"Index/CalculationEngine.iwa";
    const KEYNOTE_MARKER: &[u8] = b"Index/Presentation.iwa";

    // Check iWork first (before generic Office) since iWork ZIPs also contain XML
    if contains_subsequence(content, PAGES_MARKER) {
        return Some(IWORK_PAGES_MIME_TYPE);
    }
    if contains_subsequence(content, NUMBERS_MARKER) {
        return Some(IWORK_NUMBERS_MIME_TYPE);
    }
    if contains_subsequence(content, KEYNOTE_MARKER) {
        return Some(IWORK_KEYNOTE_MIME_TYPE);
    }

    // Check for each Office marker using a sliding window search
    if contains_subsequence(content, DOCX_MARKER) {
        return Some(DOCX_MIME_TYPE);
    }
    if contains_subsequence(content, XLSX_MARKER) {
        return Some(EXCEL_MIME_TYPE);
    }
    if contains_subsequence(content, PPTX_MARKER) {
        return Some(POWER_POINT_MIME_TYPE);
    }

    None
}

/// Check if `haystack` contains `needle` as a subsequence.
#[inline]
fn contains_subsequence(haystack: &[u8], needle: &[u8]) -> bool {
    memchr::memmem::find(haystack, needle).is_some()
}

/// Get file extensions for a given MIME type.
///
/// Returns all known file extensions that map to the specified MIME type.
///
/// # Arguments
///
/// * `mime_type` - The MIME type to look up
///
/// # Returns
///
/// A vector of file extensions (without leading dot) for the MIME type.
///
/// # Example
///
/// ```
/// use kreuzberg::core::mime::get_extensions_for_mime;
///
/// let extensions = get_extensions_for_mime("application/pdf").unwrap();
/// assert_eq!(extensions, vec!["pdf"]);
///
/// let doc_extensions = get_extensions_for_mime("application/vnd.openxmlformats-officedocument.wordprocessingml.document").unwrap();
/// assert!(doc_extensions.contains(&"docx".to_string()));
/// ```
pub fn get_extensions_for_mime(mime_type: &str) -> Result<Vec<String>> {
    let mut extensions = Vec::new();

    for (ext, mime) in EXT_TO_MIME.iter() {
        if *mime == mime_type {
            extensions.push(ext.to_string());
        }
    }

    if !extensions.is_empty() {
        return Ok(extensions);
    }

    let guessed = mime_guess::get_mime_extensions_str(mime_type);
    if let Some(exts) = guessed {
        return Ok(exts.iter().map(|s| s.to_string()).collect());
    }

    Err(KreuzbergError::UnsupportedFormat(format!(
        "No known extensions for MIME type: {}",
        mime_type
    )))
}

/// List all supported document formats.
///
/// Returns a list of all file extensions and their corresponding MIME types
/// that Kreuzberg can process. Derived from the centralized [`FORMATS`] registry.
///
/// The list is sorted alphabetically by file extension.
///
/// # Example
///
/// ```
/// use kreuzberg::core::mime::list_supported_formats;
///
/// let formats = list_supported_formats();
/// assert!(!formats.is_empty());
/// assert!(formats.iter().any(|f| f.extension == "pdf"));
/// ```
pub fn list_supported_formats() -> Vec<SupportedFormat> {
    let mut formats: Vec<SupportedFormat> = EXT_TO_MIME
        .iter()
        .map(|(ext, mime)| SupportedFormat {
            extension: ext.to_string(),
            mime_type: mime.to_string(),
        })
        .collect();
    formats.sort_by(|a, b| a.extension.cmp(&b.extension));
    formats
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_detect_mime_type_pdf() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.pdf");
        File::create(&file_path).unwrap();

        let mime = detect_mime_type(&file_path, true).unwrap();
        assert_eq!(mime, "application/pdf");
    }

    #[test]
    fn test_detect_mime_type_images() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.png", "image/png"),
            ("test.jpg", "image/jpeg"),
            ("test.jpeg", "image/jpeg"),
            ("test.gif", "image/gif"),
            ("test.bmp", "image/bmp"),
            ("test.webp", "image/webp"),
            ("test.tiff", "image/tiff"),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_detect_mime_type_office() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.xlsx", EXCEL_MIME_TYPE),
            ("test.xls", EXCEL_BINARY_MIME_TYPE),
            ("test.pptx", POWER_POINT_MIME_TYPE),
            (
                "test.ppsx",
                "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
            ),
            (
                "test.pptm",
                "application/vnd.ms-powerpoint.presentation.macroEnabled.12",
            ),
            ("test.ppt", LEGACY_POWERPOINT_MIME_TYPE),
            ("test.docx", DOCX_MIME_TYPE),
            ("test.doc", LEGACY_WORD_MIME_TYPE),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_detect_mime_type_data_formats() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.json", JSON_MIME_TYPE),
            ("test.yaml", YAML_MIME_TYPE),
            ("test.toml", TOML_MIME_TYPE),
            ("test.xml", XML_MIME_TYPE),
            ("test.csv", "text/csv"),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_detect_mime_type_text_formats() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.txt", PLAIN_TEXT_MIME_TYPE),
            ("test.md", MARKDOWN_MIME_TYPE),
            ("test.html", HTML_MIME_TYPE),
            ("test.htm", HTML_MIME_TYPE),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_detect_mime_type_email() {
        let dir = tempdir().unwrap();

        let test_cases = vec![
            ("test.eml", EML_MIME_TYPE),
            ("test.msg", MSG_MIME_TYPE),
            ("test.pst", PST_MIME_TYPE),
        ];

        for (filename, expected_mime) in test_cases {
            let file_path = dir.path().join(filename);
            File::create(&file_path).unwrap();
            let mime = detect_mime_type(&file_path, true).unwrap();
            assert_eq!(mime, expected_mime, "Failed for {}", filename);
        }
    }

    #[test]
    fn test_validate_mime_type_exact() {
        assert!(validate_mime_type("application/pdf").is_ok());
        assert!(validate_mime_type("text/plain").is_ok());
        assert!(validate_mime_type("text/html").is_ok());
    }

    #[test]
    fn test_validate_mime_type_images() {
        assert!(validate_mime_type("image/jpeg").is_ok());
        assert!(validate_mime_type("image/png").is_ok());
        assert!(validate_mime_type("image/gif").is_ok());
        assert!(validate_mime_type("image/webp").is_ok());

        assert!(validate_mime_type("image/custom-format").is_ok());
    }

    #[test]
    fn test_validate_mime_type_unsupported() {
        assert!(validate_mime_type("application/unknown").is_err());
        assert!(validate_mime_type("video/mp4").is_err());
    }

    #[test]
    fn test_file_not_exists() {
        let result = detect_mime_type("/nonexistent/file.pdf", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_no_extension() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("testfile");
        File::create(&file_path).unwrap();

        let _result = detect_mime_type(&file_path, true);
        // Files without extensions may or may not be detected via mime_guess fallback
    }

    #[test]
    fn test_detect_or_validate_with_mime() {
        let result = detect_or_validate(None, Some("application/pdf"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "application/pdf");
    }

    #[test]
    fn test_detect_or_validate_with_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.pdf");
        File::create(&file_path).unwrap();

        let result = detect_or_validate(Some(&file_path), None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "application/pdf");
    }

    #[test]
    fn test_detect_or_validate_neither() {
        let result = detect_or_validate(None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_case_insensitive_extensions() {
        let dir = tempdir().unwrap();

        let file_path = dir.path().join("test.PDF");
        File::create(&file_path).unwrap();
        let mime = detect_mime_type(&file_path, true).unwrap();
        assert_eq!(mime, "application/pdf");

        let file_path2 = dir.path().join("test.XLSX");
        File::create(&file_path2).unwrap();
        let mime2 = detect_mime_type(&file_path2, true).unwrap();
        assert_eq!(mime2, EXCEL_MIME_TYPE);
    }

    #[test]
    fn test_detect_office_format_from_zip_bytes() {
        // Test DOCX detection - minimal ZIP with word/document.xml marker
        // This is a valid ZIP local file header with "word/document.xml" as filename
        let docx_bytes: &[u8] = &[
            0x50, 0x4b, 0x03, 0x04, // ZIP local file header signature
            0x14, 0x00, // version needed
            0x00, 0x00, // general purpose bit flag
            0x00, 0x00, // compression method (stored)
            0x00, 0x00, // last mod time
            0x00, 0x00, // last mod date
            0x00, 0x00, 0x00, 0x00, // crc-32
            0x00, 0x00, 0x00, 0x00, // compressed size
            0x00, 0x00, 0x00, 0x00, // uncompressed size
            0x11, 0x00, // file name length (17)
            0x00, 0x00, // extra field length
            b'w', b'o', b'r', b'd', b'/', b'd', b'o', b'c', b'u', b'm', b'e', b'n', b't', b'.', b'x', b'm',
            b'l', // "word/document.xml"
        ];
        let mime = detect_mime_type_from_bytes(docx_bytes).unwrap();
        assert_eq!(
            mime, DOCX_MIME_TYPE,
            "Should detect DOCX from ZIP with word/document.xml"
        );

        // Test XLSX detection
        let xlsx_bytes: &[u8] = &[
            0x50, 0x4b, 0x03, 0x04, // ZIP signature
            0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x0f, 0x00, // file name length (15)
            0x00, 0x00, // extra field length
            b'x', b'l', b'/', b'w', b'o', b'r', b'k', b'b', b'o', b'o', b'k', b'.', b'x', b'm',
            b'l', // "xl/workbook.xml"
        ];
        let mime = detect_mime_type_from_bytes(xlsx_bytes).unwrap();
        assert_eq!(
            mime, EXCEL_MIME_TYPE,
            "Should detect XLSX from ZIP with xl/workbook.xml"
        );

        // Test PPTX detection
        let pptx_bytes: &[u8] = &[
            0x50, 0x4b, 0x03, 0x04, // ZIP signature
            0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x14, 0x00, // file name length (20)
            0x00, 0x00, // extra field length
            b'p', b'p', b't', b'/', b'p', b'r', b'e', b's', b'e', b'n', b't', b'a', b't', b'i', b'o', b'n', b'.', b'x',
            b'm', b'l', // "ppt/presentation.xml"
        ];
        let mime = detect_mime_type_from_bytes(pptx_bytes).unwrap();
        assert_eq!(
            mime, POWER_POINT_MIME_TYPE,
            "Should detect PPTX from ZIP with ppt/presentation.xml"
        );

        // Test plain ZIP (no Office markers)
        let plain_zip_bytes: &[u8] = &[
            0x50, 0x4b, 0x03, 0x04, // ZIP signature
            0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x08, 0x00, // file name length (8)
            0x00, 0x00, // extra field length
            b't', b'e', b's', b't', b'.', b't', b'x', b't', // "test.txt"
        ];
        let mime = detect_mime_type_from_bytes(plain_zip_bytes).unwrap();
        assert_eq!(mime, "application/zip", "Plain ZIP should remain as application/zip");
    }

    #[test]
    fn test_detect_pst_from_bytes() {
        // PST magic signature: "!BDN" followed by format-specific bytes
        let pst_bytes: &[u8] = &[
            0x21, 0x42, 0x44, 0x4E, // "!BDN" magic signature
            0x00, 0x00, 0x00, 0x00, // padding (real PST files have more header data)
        ];
        let mime = detect_mime_type_from_bytes(pst_bytes).unwrap();
        assert_eq!(mime, PST_MIME_TYPE, "Should detect PST from magic bytes");
    }

    #[test]
    fn test_list_supported_formats_not_empty() {
        let formats = list_supported_formats();
        assert!(!formats.is_empty(), "Supported formats list should not be empty");
    }

    #[test]
    fn test_list_supported_formats_sorted() {
        let formats = list_supported_formats();
        let extensions: Vec<&str> = formats.iter().map(|f| f.extension.as_str()).collect();
        let mut sorted = extensions.clone();
        sorted.sort();
        assert_eq!(extensions, sorted, "Formats should be sorted by extension");
    }

    #[test]
    fn test_list_supported_formats_includes_common_formats() {
        let formats = list_supported_formats();
        let extensions: Vec<&str> = formats.iter().map(|f| f.extension.as_str()).collect();

        assert!(extensions.contains(&"pdf"), "Should include pdf");
        assert!(extensions.contains(&"md"), "Should include md");
        assert!(extensions.contains(&"docx"), "Should include docx");
        assert!(extensions.contains(&"html"), "Should include html");
        assert!(extensions.contains(&"txt"), "Should include txt");
        assert!(extensions.contains(&"csv"), "Should include csv");
        assert!(extensions.contains(&"json"), "Should include json");
        assert!(extensions.contains(&"xlsx"), "Should include xlsx");
    }

    #[test]
    fn test_list_supported_formats_has_valid_mime_types() {
        let formats = list_supported_formats();
        for format in &formats {
            assert!(!format.extension.is_empty(), "Extension should not be empty");
            assert!(!format.mime_type.is_empty(), "MIME type should not be empty");
            assert!(format.mime_type.contains('/'), "MIME type should contain '/'");
        }
    }

    #[test]
    fn test_formats_registry_consistency() {
        // Every extension in EXT_TO_MIME should map to a MIME type that is in SUPPORTED_MIME_TYPES
        for (ext, mime) in EXT_TO_MIME.iter() {
            assert!(
                SUPPORTED_MIME_TYPES.contains(mime),
                "Extension '{}' maps to MIME '{}' which is not in SUPPORTED_MIME_TYPES",
                ext,
                mime
            );
        }
    }

    #[test]
    fn test_formats_registry_mdx() {
        // MDX extension mapping
        assert_eq!(EXT_TO_MIME.get("mdx"), Some(&"text/mdx"));
        // MDX MIME types are valid
        assert!(SUPPORTED_MIME_TYPES.contains("text/mdx"));
        assert!(SUPPORTED_MIME_TYPES.contains("text/x-mdx"));
    }

    #[test]
    fn test_formats_registry_aliases() {
        // Verify key aliases are in SUPPORTED_MIME_TYPES
        assert!(
            SUPPORTED_MIME_TYPES.contains("text/x-markdown"),
            "text/x-markdown alias"
        );
        assert!(SUPPORTED_MIME_TYPES.contains("text/json"), "text/json alias");
        assert!(SUPPORTED_MIME_TYPES.contains("text/yaml"), "text/yaml alias");
        assert!(SUPPORTED_MIME_TYPES.contains("text/xml"), "text/xml alias");
        assert!(SUPPORTED_MIME_TYPES.contains("application/xhtml+xml"), "xhtml alias");
        assert!(SUPPORTED_MIME_TYPES.contains("image/pjpeg"), "pjpeg alias");
        assert!(SUPPORTED_MIME_TYPES.contains("image/x-bmp"), "x-bmp alias");
        assert!(
            SUPPORTED_MIME_TYPES.contains("application/x-zip-compressed"),
            "zip alias"
        );
        assert!(SUPPORTED_MIME_TYPES.contains("text/rtf"), "rtf alias");
        assert!(SUPPORTED_MIME_TYPES.contains("text/x-typst"), "typst alias");
    }
}
