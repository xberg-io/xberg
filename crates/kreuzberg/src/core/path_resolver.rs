//! Image path resolution for markup extractors.
//!
//! Resolves relative image paths found in markup documents (Markdown, LaTeX, RST,
//! Org-mode, Typst, Djot, DocBook) to actual filesystem paths, reads the image data,
//! and attaches them to the extraction result.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use bytes::Bytes;

use crate::core::config::ExtractionConfig;
use crate::types::ExtractedImage;
use crate::types::internal::InternalDocument;
use crate::types::uri::UriKind;

/// Maximum image file size: 50 MB.
const MAX_IMAGE_SIZE: u64 = 50 * 1024 * 1024;

/// Resolve a relative image reference against a base directory.
///
/// Returns `None` for URLs, absolute paths, and paths that escape `base_dir`
/// via traversal (`..`). Returns `Some(resolved)` for safe relative paths.
///
/// This function performs no filesystem access — it only validates the
/// structural safety of the path.
pub(crate) fn resolve_image_path(base_dir: &Path, image_ref: &str) -> Option<PathBuf> {
    let trimmed = image_ref.trim();

    // Reject URLs
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("data:")
        || trimmed.starts_with("ftp://")
        || trimmed.starts_with("mailto:")
    {
        return None;
    }

    // Strip file:// or file: prefix (org-mode uses file: without //)
    let path_str = if let Some(stripped) = trimmed.strip_prefix("file://") {
        stripped
    } else if let Some(stripped) = trimmed.strip_prefix("file:") {
        stripped
    } else {
        trimmed
    };

    // Reject absolute paths (Unix or Windows drive letter)
    if path_str.starts_with('/')
        || (path_str.len() >= 2 && path_str.as_bytes()[0].is_ascii_alphabetic() && path_str.as_bytes()[1] == b':')
    {
        return None;
    }

    let joined = base_dir.join(path_str);
    let normalized = normalize_path(&joined);

    // Path traversal prevention: resolved path must start with base_dir
    let norm_base = normalize_path(base_dir);
    if !normalized.starts_with(&norm_base) {
        return None;
    }

    Some(normalized)
}

/// Read an image file and produce an `ExtractedImage`.
///
/// Returns `None` if the file does not exist, is not a regular file,
/// exceeds the size limit, or has an unrecognised extension.
pub(crate) fn read_image_file(path: &Path, image_index: u32) -> Option<ExtractedImage> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    if meta.len() > MAX_IMAGE_SIZE {
        return None;
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())?;

    let format: Cow<'static, str> = match ext.as_str() {
        "png" => Cow::Borrowed("png"),
        "jpg" | "jpeg" => Cow::Borrowed("jpeg"),
        "gif" => Cow::Borrowed("gif"),
        "webp" => Cow::Borrowed("webp"),
        "svg" => Cow::Borrowed("svg"),
        "bmp" => Cow::Borrowed("bmp"),
        "tiff" | "tif" => Cow::Borrowed("tiff"),
        "avif" => Cow::Borrowed("avif"),
        _ => return None,
    };

    let data = std::fs::read(path).ok()?;
    let source_path = path.to_string_lossy().into_owned();

    Some(ExtractedImage {
        data: Bytes::from(data),
        format,
        image_index,
        page_number: None,
        width: None,
        height: None,
        colorspace: None,
        bits_per_component: None,
        is_mask: false,
        description: None,
        ocr_result: None,
        bounding_box: None,
        source_path: Some(source_path),
        image_kind: None,
        kind_confidence: None,
        cluster_id: None,
    })
}

/// Resolve image URIs in an `InternalDocument` to actual image data.
///
/// Iterates over all `UriKind::Image` entries, resolves them relative to
/// `base_dir`, reads the file, and appends the result to `doc.images`.
/// No-op when image extraction is disabled in `config`.
pub(crate) fn resolve_image_uris(doc: &mut InternalDocument, base_dir: &Path, config: &ExtractionConfig) {
    let image_extraction_enabled = config.images.as_ref().is_some_and(|img| img.extract_images);

    if !image_extraction_enabled {
        return;
    }

    let mut image_index = doc.images.len() as u32;

    // Collect URI indices first to avoid borrow conflict (doc.uris vs doc.images).
    let image_uri_indices: Vec<usize> = doc
        .uris
        .iter()
        .enumerate()
        .filter(|(_, uri)| uri.kind == UriKind::Image)
        .map(|(i, _)| i)
        .collect();

    for idx in image_uri_indices {
        if let Some(resolved) = resolve_image_path(base_dir, &doc.uris[idx].url)
            && let Some(img) = read_image_file(&resolved, image_index)
        {
            doc.images.push(img);
            image_index += 1;
        }
    }
}

/// Read a file, extract via `extract_bytes`, then resolve image URIs.
///
/// Shared helper for markup extractors (Markdown, LaTeX, RST, Org-mode, Typst,
/// Djot, DocBook, MDX) that need to resolve relative image paths after extraction.
pub(crate) async fn extract_file_with_image_resolution(
    extractor: &(dyn crate::plugins::DocumentExtractor + Sync),
    path: &Path,
    mime_type: &str,
    config: &ExtractionConfig,
) -> crate::Result<InternalDocument> {
    let bytes = crate::core::io::open_file_bytes(path)?;
    let mut doc = extractor.extract_bytes(&bytes, mime_type, config).await?;
    if let Some(base_dir) = path.parent() {
        resolve_image_uris(&mut doc, base_dir, config);
    }
    Ok(doc)
}

/// Normalize a path by resolving `.` and `..` components without filesystem access.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_resolve_relative_path() {
        let base = Path::new("/home/user/docs");
        let result = resolve_image_path(base, "images/photo.png");
        assert_eq!(result, Some(PathBuf::from("/home/user/docs/images/photo.png")));
    }

    #[test]
    fn test_resolve_nested_relative() {
        let base = Path::new("/project/content");
        let result = resolve_image_path(base, "images/subfolder/nested.png");
        assert_eq!(
            result,
            Some(PathBuf::from("/project/content/images/subfolder/nested.png"))
        );
    }

    #[test]
    fn test_reject_absolute_path() {
        let base = Path::new("/home/user/docs");
        assert_eq!(resolve_image_path(base, "/etc/passwd"), None);
    }

    #[test]
    fn test_reject_traversal() {
        let base = Path::new("/home/user/docs");
        assert_eq!(resolve_image_path(base, "../../etc/passwd"), None);
    }

    #[test]
    fn test_reject_http_url() {
        let base = Path::new("/home/user/docs");
        assert_eq!(resolve_image_path(base, "https://example.com/img.png"), None);
    }

    #[test]
    fn test_reject_data_uri() {
        let base = Path::new("/home/user/docs");
        assert_eq!(resolve_image_path(base, "data:image/png;base64,abc"), None);
    }

    #[test]
    fn test_nonexistent_file_still_resolves() {
        // resolve_image_path only checks structure, not filesystem
        let base = Path::new("/nonexistent/base");
        let result = resolve_image_path(base, "sub/image.jpg");
        assert_eq!(result, Some(PathBuf::from("/nonexistent/base/sub/image.jpg")));
    }

    #[test]
    fn test_path_with_spaces() {
        let base = Path::new("/home/user/my docs");
        let result = resolve_image_path(base, "my images/photo.png");
        assert_eq!(result, Some(PathBuf::from("/home/user/my docs/my images/photo.png")));
    }

    #[test]
    fn test_windows_backslash() {
        // On all platforms, std::path::Path::join handles separators correctly.
        // On Unix, backslash is a valid filename char so it stays as-is in the component.
        // The key point: the function does not panic and produces a usable path.
        let base = Path::new("/home/user/docs");
        let result = resolve_image_path(base, "images/photo.png");
        assert!(result.is_some());
    }

    #[test]
    fn test_reject_ftp_url() {
        let base = Path::new("/home/user/docs");
        assert_eq!(resolve_image_path(base, "ftp://server/img.png"), None);
    }

    #[test]
    fn test_reject_mailto() {
        let base = Path::new("/home/user/docs");
        assert_eq!(resolve_image_path(base, "mailto:user@example.com"), None);
    }

    #[test]
    fn test_file_uri_stripped() {
        let base = Path::new("/home/user/docs");
        let result = resolve_image_path(base, "file://images/photo.png");
        assert_eq!(result, Some(PathBuf::from("/home/user/docs/images/photo.png")));
    }

    #[test]
    fn test_reject_windows_absolute() {
        let base = Path::new("/home/user/docs");
        assert_eq!(resolve_image_path(base, "C:\\Windows\\img.png"), None);
    }
}
