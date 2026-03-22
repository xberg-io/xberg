//! Tessdata file downloading and cache management.
//!
//! Downloads `.traineddata` files from the tessdata_fast GitHub repository
//! and manages them in the kreuzberg cache directory.

use std::path::{Path, PathBuf};

#[cfg(feature = "paddle-ocr")]
use std::fs;

#[cfg(feature = "paddle-ocr")]
use super::error::OcrError;
#[cfg(feature = "paddle-ocr")]
use super::validation::TESSERACT_SUPPORTED_LANGUAGE_CODES;

#[cfg(feature = "paddle-ocr")]
const TESSDATA_FAST_BASE_URL: &str = "https://github.com/tesseract-ocr/tessdata_fast/raw/main";

/// All language codes to download (supported languages + osd for script detection).
#[cfg(feature = "paddle-ocr")]
fn all_download_codes() -> Vec<&'static str> {
    let mut codes: Vec<&str> = TESSERACT_SUPPORTED_LANGUAGE_CODES.iter().copied().collect();
    if !codes.contains(&"osd") {
        codes.push("osd");
    }
    codes.sort();
    codes
}

/// Manages tessdata file downloading, caching, and manifest generation.
#[derive(Debug, Clone)]
pub struct TessdataManager {
    cache_dir: PathBuf,
}

impl TessdataManager {
    /// Creates a new tessdata manager.
    ///
    /// If `cache_dir` is None, uses the default cache directory:
    /// 1. `KREUZBERG_CACHE_DIR` env var + `/tessdata`
    /// 2. `.kreuzberg/tessdata/` in current directory
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        let cache_dir = cache_dir.unwrap_or_else(|| {
            if let Ok(env_path) = std::env::var("KREUZBERG_CACHE_DIR") {
                PathBuf::from(env_path).join("tessdata")
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(".kreuzberg")
                    .join("tessdata")
            }
        });
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
    #[cfg(feature = "paddle-ocr")]
    pub fn manifest() -> Vec<crate::paddle_ocr::ModelManifestEntry> {
        all_download_codes()
            .into_iter()
            .map(|lang| crate::paddle_ocr::ModelManifestEntry {
                relative_path: format!("tessdata/{lang}.traineddata"),
                sha256: String::new(),
                size_bytes: 0,
                source_url: format!("{TESSDATA_FAST_BASE_URL}/{lang}.traineddata"),
            })
            .collect()
    }

    /// Downloads all tessdata_fast traineddata files to the cache directory.
    ///
    /// Skips files that already exist. Returns the count of newly downloaded files.
    ///
    /// Requires the `paddle-ocr` feature for HTTP download support (ureq).
    #[cfg(feature = "paddle-ocr")]
    pub fn ensure_all_languages(&self) -> Result<usize, OcrError> {
        fs::create_dir_all(&self.cache_dir).map_err(|e| {
            OcrError::TesseractInitializationFailed(format!(
                "Failed to create tessdata cache dir {}: {e}",
                self.cache_dir.display()
            ))
        })?;

        let codes = all_download_codes();
        let total = codes.len();
        let mut downloaded = 0usize;

        for (i, lang) in codes.iter().enumerate() {
            let dest = self.cache_dir.join(format!("{lang}.traineddata"));
            if dest.exists() {
                continue;
            }

            let url = format!("{TESSDATA_FAST_BASE_URL}/{lang}.traineddata");
            tracing::info!(lang, progress = format!("{}/{}", i + 1, total), "Downloading tessdata");

            download_traineddata(&url, &dest).map_err(|e| {
                OcrError::TesseractInitializationFailed(format!("Failed to download {lang}.traineddata: {e}"))
            })?;

            downloaded += 1;
        }

        tracing::info!(downloaded, total, "Tessdata download complete");

        Ok(downloaded)
    }
}

/// Download a single traineddata file with retries.
#[cfg(feature = "paddle-ocr")]
fn download_traineddata(url: &str, dest: &Path) -> Result<(), String> {
    let max_attempts = 3;

    for attempt in 1..=max_attempts {
        let result = (|| -> Result<Vec<u8>, String> {
            let agent = ureq::Agent::new_with_defaults();
            let response = agent.get(url).call().map_err(|e| format!("HTTP request failed: {e}"))?;

            let status = response.status();
            if status != 200 {
                return Err(format!("HTTP {status}"));
            }

            let body = response
                .into_body()
                .with_config()
                .limit(50 * 1024 * 1024)
                .read_to_vec()
                .map_err(|e| format!("Failed to read response body: {e}"))?;

            Ok(body)
        })();

        match result {
            Ok(bytes) => {
                fs::write(dest, &bytes).map_err(|e| format!("Failed to write {}: {e}", dest.display()))?;
                return Ok(());
            }
            Err(e) => {
                if attempt == max_attempts {
                    return Err(format!("Failed after {max_attempts} attempts: {e}"));
                }
                tracing::warn!(attempt, max_attempts, error = %e, "Download failed, retrying...");
                std::thread::sleep(std::time::Duration::from_secs(2u64.pow((attempt - 1).min(3))));
            }
        }
    }

    unreachable!()
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

    #[cfg(feature = "paddle-ocr")]
    #[test]
    fn test_all_download_codes_includes_osd() {
        let codes = all_download_codes();
        assert!(codes.contains(&"osd"));
        assert!(codes.contains(&"eng"));
        assert!(codes.contains(&"fra"));
    }

    #[cfg(feature = "paddle-ocr")]
    #[test]
    fn test_all_download_codes_sorted() {
        let codes = all_download_codes();
        let mut sorted = codes.clone();
        sorted.sort();
        assert_eq!(codes, sorted);
    }

    #[cfg(feature = "paddle-ocr")]
    #[test]
    fn test_manifest_returns_entries() {
        let entries = TessdataManager::manifest();
        assert!(!entries.is_empty());

        let paths: Vec<&str> = entries.iter().map(|e| e.relative_path.as_str()).collect();
        assert!(paths.contains(&"tessdata/eng.traineddata"));
        assert!(paths.contains(&"tessdata/osd.traineddata"));
    }

    #[cfg(feature = "paddle-ocr")]
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

    #[cfg(feature = "paddle-ocr")]
    #[test]
    fn test_ensure_all_languages_with_existing_files() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TessdataManager::new(Some(temp_dir.path().to_path_buf()));

        // Pre-populate all files so no downloads occur
        for code in all_download_codes() {
            fs::write(temp_dir.path().join(format!("{code}.traineddata")), "fake").unwrap();
        }

        let downloaded = manager.ensure_all_languages().unwrap();
        assert_eq!(downloaded, 0);
    }
}
