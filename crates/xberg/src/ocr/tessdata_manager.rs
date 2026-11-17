//! Tessdata file downloading and cache management.
//!
//! Downloads `.traineddata` files from the tessdata_fast GitHub repository
//! and manages them in the xberg cache directory.

use std::path::{Path, PathBuf};

#[cfg(paddle_ocr)]
use std::fs;

#[cfg(paddle_ocr)]
use super::error::OcrError;
#[cfg(paddle_ocr)]
use super::tessdata_download::{TESSDATA_FAST_PACKS, download_missing_language_pack, pinned_source_url};

/// Manages tessdata file downloading, caching, and manifest generation.
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
pub struct TessdataManager {
    cache_dir: PathBuf,
}

impl TessdataManager {
    /// Creates a new tessdata manager.
    ///
    /// If `cache_dir` is None, uses the default cache directory:
    /// 1. `XBERG_CACHE_DIR` env var + `/tessdata`
    /// 2. `.xberg/tessdata/` in current directory
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        let cache_dir = cache_dir.unwrap_or_else(|| crate::cache_dir::resolve_cache_dir("tessdata"));
        Self { cache_dir }
    }

    /// Get the cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Check if a specific language traineddata file is cached.
    pub fn is_language_cached(&self, lang: &str) -> bool {
        self.cache_dir.join(format!("{lang}.traineddata")).exists()
    }

    /// Returns the manifest of all tessdata files.
    ///
    /// Paths are relative to the cache root (prefixed with "tessdata/").
    #[cfg(paddle_ocr)]
    #[cfg_attr(alef, alef(skip))]
    pub fn manifest() -> Vec<crate::paddle_ocr::ModelManifestEntry> {
        TESSDATA_FAST_PACKS
            .iter()
            .map(|&(lang, sha256, size_bytes)| crate::paddle_ocr::ModelManifestEntry {
                relative_path: format!("tessdata/{lang}.traineddata"),
                sha256: sha256.to_string(),
                size_bytes,
                source_url: pinned_source_url(lang),
            })
            .collect()
    }

    /// Downloads all tessdata_fast traineddata files to the cache directory.
    ///
    /// Skips files that already exist. Returns the count of newly downloaded files.
    ///
    /// When the `paddle-ocr` feature is not enabled, no download URLs are available
    /// and this method always returns `Ok(0)`.
    #[cfg(not(paddle_ocr))]
    pub fn ensure_all_languages(&self) -> Result<usize, crate::error::XbergError> {
        Ok(0)
    }

    /// Downloads all tessdata_fast traineddata files to the cache directory.
    ///
    /// Skips files that already exist. Returns the count of newly downloaded files.
    ///
    /// Requires the `paddle-ocr` feature for HTTP download support (ureq).
    #[cfg(paddle_ocr)]
    pub fn ensure_all_languages(&self) -> Result<usize, OcrError> {
        fs::create_dir_all(&self.cache_dir).map_err(|e| {
            OcrError::TesseractInitializationFailed(format!(
                "Failed to create tessdata cache dir {}: {e}",
                self.cache_dir.display()
            ))
        })?;

        let total = TESSDATA_FAST_PACKS.len();
        let mut downloaded = 0;

        for (i, &(lang, _, _)) in TESSDATA_FAST_PACKS.iter().enumerate() {
            if self.is_language_cached(lang) {
                continue;
            }
            tracing::info!(lang, progress = format!("{}/{}", i + 1, total), "Downloading tessdata");
            downloaded += usize::from(download_missing_language_pack(lang, &self.cache_dir)?);
        }

        tracing::info!(downloaded, total, "Tessdata download complete");

        Ok(downloaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tessdata_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TessdataManager::new(Some(temp_dir.path().to_path_buf()));
        assert_eq!(manager.cache_dir(), temp_dir.path());
    }

    #[test]
    fn test_is_language_cached_empty() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TessdataManager::new(Some(temp_dir.path().to_path_buf()));
        assert!(!manager.is_language_cached("eng"));
    }

    #[test]
    fn test_is_language_cached_present() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TessdataManager::new(Some(temp_dir.path().to_path_buf()));
        std::fs::write(temp_dir.path().join("eng.traineddata"), "fake").unwrap();
        assert!(manager.is_language_cached("eng"));
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn manifest_contains_pinned_english_and_orientation_models() {
        let entries = TessdataManager::manifest();
        let english = entries
            .iter()
            .find(|entry| entry.relative_path == "tessdata/eng.traineddata")
            .unwrap();
        assert_eq!(
            english.sha256,
            "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
        );
        assert_eq!(english.size_bytes, 4_113_088);
        assert!(
            entries
                .iter()
                .any(|entry| entry.relative_path == "tessdata/osd.traineddata")
        );
        let fraktur = entries
            .iter()
            .find(|entry| entry.relative_path == "tessdata/frk.traineddata")
            .unwrap();
        assert!(fraktur.source_url.ends_with("/deu_latf.traineddata"));
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_manifest_entries_have_valid_urls() {
        let entries = TessdataManager::manifest();
        for entry in &entries {
            assert!(
                entry
                    .source_url
                    .starts_with("https://github.com/tesseract-ocr/tessdata_fast/"),
                "Source URL should point to tessdata_fast: {}",
                entry.source_url
            );
            assert!(
                entry.relative_path.starts_with("tessdata/"),
                "Paths should be prefixed with tessdata/"
            );
        }
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_ensure_all_languages_with_existing_files() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TessdataManager::new(Some(temp_dir.path().to_path_buf()));

        for &(code, _, _) in TESSDATA_FAST_PACKS {
            fs::write(temp_dir.path().join(format!("{code}.traineddata")), "fake").unwrap();
        }

        let downloaded = manager.ensure_all_languages().unwrap();
        assert_eq!(downloaded, 0);
    }
}
