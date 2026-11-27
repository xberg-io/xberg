//! Tessdata file downloading and cache management.
//!
//! Downloads `.traineddata` files from the pinned tessdata_fast or tessdata_best GitHub
//! revision into that model set's directory of the xberg cache (`<cache>/tessdata/<model>`).

use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use std::fs;

use super::error::OcrError;
#[cfg(paddle_ocr)]
use super::tessdata_download::pinned_source_url;
#[cfg(not(target_arch = "wasm32"))]
use super::tessdata_download::{ensure_language_pack, pinned_packs};
use crate::core::config::TessdataModel;

/// Manages tessdata file downloading, caching, and manifest generation.
#[derive(Debug, Clone)]
#[cfg_attr(alef, alef(skip))]
pub struct TessdataManager {
    cache_dir: PathBuf,
    model: TessdataModel,
    #[cfg(not(target_arch = "wasm32"))]
    adopt_from: Vec<PathBuf>,
}

/// The local directories a manager writing into `set_dir` may copy verified packs from: those
/// OCR resolution adopts from when `set_dir` is a `<root>/<model>` set directory, none otherwise.
#[cfg(not(target_arch = "wasm32"))]
fn adoption_sources(set_dir: &Path, model: TessdataModel) -> Vec<PathBuf> {
    match set_dir.parent() {
        Some(flat) if set_dir.file_name().is_some_and(|name| name == model.as_str()) => {
            super::processor::validation::adoption_sources_beside(flat, model)
        }
        _ => Vec::new(),
    }
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
    /// copied, after its SHA-256 is checked against the pin, from the directories OCR resolution
    /// adopts from: the flat `<root>/tessdata` older releases filled, `<root>/tessdata/fast`
    /// (`osd` and `equ` are the same bytes in both sets) and, unless
    /// `XBERG_TESSDATA_SYSTEM_PATHS` turns them off, the built-in `best` directory (for `best`)
    /// and the system tessdata directories. Only then is it downloaded.
    pub fn for_model(cache_dir: Option<PathBuf>, model: TessdataModel) -> Self {
        let cache_dir =
            cache_dir.unwrap_or_else(|| crate::cache_dir::resolve_cache_dir("tessdata").join(model.as_str()));
        Self {
            #[cfg(not(target_arch = "wasm32"))]
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
    ///
    /// A regular file counts. For the `best` set, a language with a pinned pack counts only when
    /// its file is that pack (size and SHA-256), so fast data, an empty file or a truncated one is
    /// reported as not cached and replaced by the next download.
    pub fn is_language_cached(&self, lang: &str) -> bool {
        let pack = self.cache_dir.join(format!("{lang}.traineddata"));
        #[cfg(not(target_arch = "wasm32"))]
        {
            let pins = &super::tessdata_download::PINNED;
            if self.model == TessdataModel::Best && pins.pin(lang, self.model).is_some() {
                return pins.classify(lang, &pack).is(self.model);
            }
        }
        pack.is_file()
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

    /// WebAssembly cannot download tessdata, so this is an error there rather than a count of
    /// zero, which would read as "already cached".
    #[cfg(target_arch = "wasm32")]
    pub fn ensure_all_languages(&self) -> Result<usize, OcrError> {
        self.ensure_languages(&[])
    }

    /// WebAssembly cannot download tessdata, so this is an error there rather than a count of
    /// zero, which would read as "already cached".
    #[cfg(target_arch = "wasm32")]
    pub fn ensure_languages(&self, _languages: &[&str]) -> Result<usize, OcrError> {
        Err(OcrError::TesseractInitializationFailed(format!(
            "tessdata_{} cannot be downloaded on WebAssembly; supply the models through OcrConfig::tessdata_bytes",
            self.model
        )))
    }

    /// Downloads every pinned traineddata file of this manager's model set.
    ///
    /// Skips files that already exist. Returns the count of newly downloaded files.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn ensure_all_languages(&self) -> Result<usize, OcrError> {
        let languages: Vec<&str> = pinned_packs(self.model).iter().map(|&(lang, _, _)| lang).collect();
        self.ensure_languages(&languages)
    }

    /// Downloads the pinned traineddata files of `languages` for this manager's model set.
    ///
    /// Skips files that already exist and copies verified ones from local directories (see
    /// [`Self::for_model`]). Entries are trimmed and empty ones ignored; every language is checked
    /// against the pins before anything is written, and one with no pinned pack is an error.
    /// Returns the count of newly published files.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn ensure_languages(&self, languages: &[&str]) -> Result<usize, OcrError> {
        let languages: Vec<&str> = languages
            .iter()
            .map(|lang| lang.trim())
            .filter(|lang| !lang.is_empty())
            .collect();
        let unknown: Vec<&str> = languages
            .iter()
            .copied()
            .filter(|lang| super::tessdata_download::PINNED.pin(lang, self.model).is_none())
            .collect();
        if !unknown.is_empty() {
            return Err(OcrError::InvalidLanguageCode(format!(
                "No pinned {} language pack for: {}",
                self.model.repository(),
                unknown.join(", ")
            )));
        }
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
            if self.model == TessdataModel::Best {
                let set_aside = super::tessdata_download::PINNED
                    .set_aside_unverified(lang, self.model, &self.cache_dir)
                    .map_err(|e| {
                        OcrError::TesseractInitializationFailed(format!(
                            "'{lang}.traineddata' in {} is not the pinned {} language pack and could not be \
                             moved aside: {e}. Remove it",
                            self.cache_dir.display(),
                            self.model.repository()
                        ))
                    })?;
                // ~keep The directory cannot be written, so nothing can be fetched into it either;
                // set_aside_unverified has warned that the file stays.
                if set_aside == super::tessdata_download::SetAside::Blocked {
                    continue;
                }
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

    #[test]
    fn a_best_manager_counts_only_the_pinned_pack_as_cached() {
        let temp_dir = TempDir::new().unwrap();
        let best = TessdataManager::for_model(Some(temp_dir.path().to_path_buf()), TessdataModel::Best);
        let fast = TessdataManager::for_model(Some(temp_dir.path().to_path_buf()), TessdataModel::Fast);

        std::fs::write(temp_dir.path().join("eng.traineddata"), "not the pinned best pack").unwrap();
        std::fs::write(temp_dir.path().join("my_custom.traineddata"), "user trained").unwrap();
        std::fs::create_dir(temp_dir.path().join("deu.traineddata")).unwrap();

        assert!(!best.is_language_cached("eng"));
        assert!(fast.is_language_cached("eng"), "the fast set keeps hand-placed packs");
        assert!(best.is_language_cached("my_custom"), "no pin to hold it to");
        assert!(!fast.is_language_cached("deu"), "a directory is not a pack");
    }

    #[test]
    fn test_ensure_all_languages_with_existing_files() {
        let temp_dir = TempDir::new().unwrap();
        // ~keep Fast: a best manager holds every pinned file to its pin and would download.
        let manager = TessdataManager::for_model(Some(temp_dir.path().to_path_buf()), TessdataModel::Fast);

        for &(code, _, _) in pinned_packs(TessdataModel::Fast) {
            fs::write(temp_dir.path().join(format!("{code}.traineddata")), "fake").unwrap();
        }

        let downloaded = manager.ensure_all_languages().unwrap();
        assert_eq!(downloaded, 0);
    }

    #[test]
    fn a_set_directory_adopts_from_the_directories_resolution_adopts_from() {
        let root = Path::new("/cache/tessdata");

        for model in [TessdataModel::Fast, TessdataModel::Best] {
            let manager = TessdataManager::for_model(Some(root.join(model.as_str())), model);
            assert_eq!(
                manager.adopt_from[..2],
                [root.to_path_buf(), root.join("fast")],
                "{model}"
            );
        }
        assert!(
            TessdataManager::for_model(Some(PathBuf::from("/elsewhere")), TessdataModel::Best)
                .adopt_from
                .is_empty()
        );
        assert!(
            TessdataManager::for_model(Some(root.join("fast")), TessdataModel::Best)
                .adopt_from
                .is_empty(),
            "the directory of the other set is not a best set directory"
        );
    }

    #[test]
    fn ensure_languages_rejects_a_language_with_no_pin() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TessdataManager::for_model(Some(temp_dir.path().to_path_buf()), TessdataModel::Fast);

        let error = manager
            .ensure_languages(&[" eng ", "not_a_language", " zz", " "])
            .expect_err("unknown languages must be rejected");
        assert!(error.to_string().contains("not_a_language, zz"), "{error}");
        assert_eq!(
            temp_dir.path().read_dir().unwrap().count(),
            0,
            "nothing is downloaded before every language is validated"
        );
        assert_eq!(manager.ensure_languages(&[]).unwrap(), 0);
    }
}
