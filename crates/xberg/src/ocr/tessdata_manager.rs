//! Tessdata file downloading and cache management.
//!
//! Downloads `.traineddata` files from the pinned tessdata_fast or tessdata_best GitHub
//! revision into that model set's directory of the xberg cache (`<cache>/tessdata/<model>`).

use std::path::{Path, PathBuf};

#[cfg(paddle_ocr)]
use std::fs;

#[cfg(paddle_ocr)]
use super::error::OcrError;
#[cfg(paddle_ocr)]
use super::tessdata_download::{ensure_language_pack, pinned_packs, pinned_source_url};
use crate::core::config::TessdataModel;

/// Manages tessdata file downloading, caching, and manifest generation.
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
pub struct TessdataManager {
    cache_dir: PathBuf,
    model: TessdataModel,
    #[cfg(paddle_ocr)]
    adopt_from: Vec<PathBuf>,
}

/// The local directories a manager writing into `set_dir` may copy verified packs from.
#[cfg(paddle_ocr)]
fn adoption_sources(set_dir: &Path, model: TessdataModel) -> Vec<PathBuf> {
    let Some(flat) = set_dir
        .parent()
        .filter(|_| set_dir.file_name().is_some_and(|name| name == model.as_str()))
    else {
        return Vec::new();
    };
    let mut sources = Vec::new();
    if model == TessdataModel::Best {
        sources.push(flat.join(TessdataModel::Fast.as_str()));
    }
    sources.push(flat.to_path_buf());
    sources
}

impl TessdataManager {
    /// Creates a tessdata manager for the default model set.
    ///
    /// See [`Self::for_model`] for how `cache_dir` is used.
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        Self::for_model(cache_dir, TessdataModel::default())
    }

    /// Creates a tessdata manager for `model`.
    ///
    /// `cache_dir` is the directory the model files are written to. When `None`, it is the
    /// model set's own directory under the xberg cache, `<cache>/tessdata/<model>`, which is
    /// where OCR resolution looks for it.
    ///
    /// When `cache_dir` is such a `<root>/tessdata/<model>` directory, a missing pack is first
    /// copied, after its SHA-256 is checked against the pin, from the flat `<root>/tessdata`
    /// older releases filled and, for `best`, from `<root>/tessdata/fast` (`osd` and `equ` are
    /// the same bytes in both sets), before it is downloaded.
    pub fn for_model(cache_dir: Option<PathBuf>, model: TessdataModel) -> Self {
        let cache_dir =
            cache_dir.unwrap_or_else(|| crate::cache_dir::resolve_cache_dir("tessdata").join(model.as_str()));
        Self {
            #[cfg(paddle_ocr)]
            adopt_from: adoption_sources(&cache_dir, model),
            cache_dir,
            model,
        }
    }

    /// Get the cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// The model set this manager downloads.
    pub fn model(&self) -> TessdataModel {
        self.model
    }

    /// Check if a specific language traineddata file is cached.
    pub fn is_language_cached(&self, lang: &str) -> bool {
        self.cache_dir.join(format!("{lang}.traineddata")).exists()
    }

    /// Returns the manifest of every pinned tessdata file of both model sets.
    ///
    /// Paths are relative to the cache root (`tessdata/<model>/<lang>.traineddata`).
    #[cfg(paddle_ocr)]
    #[cfg_attr(alef, alef(skip))]
    pub fn manifest() -> Vec<crate::paddle_ocr::ModelManifestEntry> {
        [TessdataModel::Fast, TessdataModel::Best]
            .into_iter()
            .flat_map(Self::manifest_for)
            .collect()
    }

    /// Returns the manifest of the pinned tessdata files of `model`.
    #[cfg(paddle_ocr)]
    #[cfg_attr(alef, alef(skip))]
    pub fn manifest_for(model: TessdataModel) -> Vec<crate::paddle_ocr::ModelManifestEntry> {
        pinned_packs(model)
            .iter()
            .map(|&(lang, sha256, size_bytes)| crate::paddle_ocr::ModelManifestEntry {
                relative_path: format!("tessdata/{model}/{lang}.traineddata"),
                sha256: sha256.to_string(),
                size_bytes,
                source_url: pinned_source_url(lang, model),
            })
            .collect()
    }

    /// Downloads every pinned traineddata file of this manager's model set.
    ///
    /// Skips files that already exist. Returns the count of newly downloaded files.
    ///
    /// When the `paddle-ocr` feature is not enabled, no download URLs are available
    /// and this method always returns `Ok(0)`.
    #[cfg(not(paddle_ocr))]
    pub fn ensure_all_languages(&self) -> Result<usize, crate::error::XbergError> {
        Ok(0)
    }

    /// Downloads the pinned traineddata files of `languages` for this manager's model set.
    ///
    /// When the `paddle-ocr` feature is not enabled, no download URLs are available
    /// and this method always returns `Ok(0)`.
    #[cfg(not(paddle_ocr))]
    pub fn ensure_languages(&self, _languages: &[&str]) -> Result<usize, crate::error::XbergError> {
        Ok(0)
    }

    /// Downloads every pinned traineddata file of this manager's model set.
    ///
    /// Skips files that already exist. Returns the count of newly downloaded files.
    ///
    /// Requires the `paddle-ocr` feature for HTTP download support (ureq).
    #[cfg(paddle_ocr)]
    pub fn ensure_all_languages(&self) -> Result<usize, OcrError> {
        let languages: Vec<&str> = pinned_packs(self.model).iter().map(|&(lang, _, _)| lang).collect();
        self.ensure_languages(&languages)
    }

    /// Downloads the pinned traineddata files of `languages` for this manager's model set.
    ///
    /// Skips files that already exist and copies verified ones from the flat cache (see
    /// [`Self::for_model`]); a language with no pinned pack is an error. Returns the count of
    /// newly published files.
    #[cfg(paddle_ocr)]
    pub fn ensure_languages(&self, languages: &[&str]) -> Result<usize, OcrError> {
        fs::create_dir_all(&self.cache_dir).map_err(|e| {
            OcrError::TesseractInitializationFailed(format!(
                "Failed to create tessdata cache dir {}: {e}",
                self.cache_dir.display()
            ))
        })?;

        let total = languages.len();
        let mut downloaded = 0;

        for (i, &lang) in languages.iter().enumerate() {
            if self.is_language_cached(lang) {
                continue;
            }
            tracing::info!(
                lang,
                model = %self.model,
                progress = format!("{}/{}", i + 1, total),
                "Downloading tessdata"
            );
            downloaded += usize::from(ensure_language_pack(
                lang,
                self.model,
                &self.cache_dir,
                &self.adopt_from,
            )?);
        }

        tracing::info!(downloaded, total, model = %self.model, "Tessdata download complete");

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

    #[test]
    fn each_model_set_defaults_to_its_own_cache_directory() {
        let fast = TessdataManager::for_model(None, TessdataModel::Fast);
        let best = TessdataManager::for_model(None, TessdataModel::Best);

        assert!(fast.cache_dir().ends_with(Path::new("tessdata").join("fast")));
        assert!(best.cache_dir().ends_with(Path::new("tessdata").join("best")));
        assert_eq!(best.model(), TessdataModel::Best);
        assert_eq!(TessdataManager::new(None).model(), TessdataModel::default());
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn manifest_contains_pinned_english_and_orientation_models() {
        let entries = TessdataManager::manifest();
        let english = entries
            .iter()
            .find(|entry| entry.relative_path == "tessdata/fast/eng.traineddata")
            .unwrap();
        assert_eq!(
            english.sha256,
            "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2"
        );
        assert_eq!(english.size_bytes, 4_113_088);
        let best_english = entries
            .iter()
            .find(|entry| entry.relative_path == "tessdata/best/eng.traineddata")
            .unwrap();
        assert_eq!(best_english.size_bytes, 15_400_601);
        assert!(
            entries
                .iter()
                .any(|entry| entry.relative_path == "tessdata/fast/osd.traineddata")
        );
        let fraktur = entries
            .iter()
            .find(|entry| entry.relative_path == "tessdata/best/frk.traineddata")
            .unwrap();
        assert!(fraktur.source_url.ends_with("/deu_latf.traineddata"));
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_manifest_entries_have_valid_urls() {
        for model in [TessdataModel::Fast, TessdataModel::Best] {
            for entry in TessdataManager::manifest_for(model) {
                assert!(
                    entry
                        .source_url
                        .starts_with(&format!("https://github.com/tesseract-ocr/{}/", model.repository())),
                    "Source URL should point to {}: {}",
                    model.repository(),
                    entry.source_url
                );
                assert!(
                    entry.relative_path.starts_with(&format!("tessdata/{model}/")),
                    "Paths should be prefixed with tessdata/{model}/"
                );
            }
        }
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn test_ensure_all_languages_with_existing_files() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TessdataManager::for_model(Some(temp_dir.path().to_path_buf()), TessdataModel::Best);

        for &(code, _, _) in pinned_packs(TessdataModel::Best) {
            fs::write(temp_dir.path().join(format!("{code}.traineddata")), "fake").unwrap();
        }

        let downloaded = manager.ensure_all_languages().unwrap();
        assert_eq!(downloaded, 0);
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn a_set_directory_adopts_from_the_flat_cache_and_best_also_from_fast() {
        let root = Path::new("/cache/tessdata");

        assert_eq!(
            TessdataManager::for_model(Some(root.join("fast")), TessdataModel::Fast).adopt_from,
            [root.to_path_buf()]
        );
        assert_eq!(
            TessdataManager::for_model(Some(root.join("best")), TessdataModel::Best).adopt_from,
            [root.join("fast"), root.to_path_buf()]
        );
        assert!(
            TessdataManager::for_model(Some(PathBuf::from("/elsewhere")), TessdataModel::Best)
                .adopt_from
                .is_empty()
        );
    }

    #[cfg(paddle_ocr)]
    #[test]
    fn ensure_languages_rejects_a_language_with_no_pin() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TessdataManager::for_model(Some(temp_dir.path().to_path_buf()), TessdataModel::Fast);

        assert!(manager.ensure_languages(&["not_a_language"]).is_err());
        assert_eq!(manager.ensure_languages(&[]).unwrap(), 0);
    }
}
