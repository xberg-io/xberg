/// Model downloading and caching for PaddleOCR.
///
/// This module handles PaddleOCR model path resolution, downloading, and caching operations.
/// Models are organized into shared models (detection, classification) and per-family
/// recognition models (one per script family).
///
/// # Model Download Flow
///
/// 1. Check if models exist in cache directory
/// 2. If not, download ONNX models from HuggingFace Hub via hf-hub
/// 3. Verify SHA256 checksums
/// 4. Copy models to local cache directory
///
/// # Cache Structure
///
/// ```text
/// cache_dir/
/// ├── det/
/// │   └── model.onnx
/// ├── cls/
/// │   └── model.onnx
/// └── rec/
///     ├── english/
///     │   ├── model.onnx
///     │   └── dict.txt
///     ├── chinese/
///     │   ├── model.onnx
///     │   └── dict.txt
///     └── ...
/// ```
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::KreuzbergError;
use sha2::{Digest, Sha256};

/// HuggingFace repository containing PaddleOCR ONNX models.
const HF_REPO_ID: &str = "Kreuzberg/paddleocr-onnx-models";

/// Shared model definition (detection and classification).
#[derive(Debug, Clone)]
struct SharedModelDefinition {
    model_type: &'static str,
    remote_filename: &'static str,
    local_filename: &'static str,
    sha256_checksum: &'static str,
    #[allow(dead_code)]
    size_bytes: u64,
}

/// Recognition model definition (per script family).
#[derive(Debug, Clone)]
struct RecModelDefinition {
    script_family: &'static str,
    model_sha256: &'static str,
    dict_sha256: &'static str,
    #[allow(dead_code)]
    model_size_bytes: u64,
}

/// Shared models: detection (PP-OCRv4) and classification (PPOCRv2).
/// These are language-agnostic and shared across all script families.
const SHARED_MODELS: &[SharedModelDefinition] = &[
    SharedModelDefinition {
        model_type: "det",
        remote_filename: "ch_PP-OCRv4_det_infer.onnx",
        local_filename: "model.onnx",
        sha256_checksum: "d2a7720d45a54257208b1e13e36a8479894cb74155a5efe29462512d42f49da9",
        size_bytes: 4_745_517,
    },
    SharedModelDefinition {
        model_type: "cls",
        remote_filename: "ch_ppocr_mobile_v2.0_cls_infer.onnx",
        local_filename: "model.onnx",
        sha256_checksum: "e47acedf663230f8863ff1ab0e64dd2d82b838fceb5957146dab185a89d6215c",
        size_bytes: 585_532,
    },
];

/// Recognition model definitions for 12 script families.
///
/// Each family has a recognition model (`rec/{family}/model.onnx`) and a character
/// dictionary (`rec/{family}/dict.txt`) hosted on HuggingFace.
///
/// Sources:
/// - PP-OCRv5 (7 families): english, chinese (server), latin, korean, eslav, thai, greek
/// - PP-OCRv3 (5 families): arabic, devanagari, tamil, telugu, kannada
const REC_MODELS: &[RecModelDefinition] = &[
    RecModelDefinition {
        script_family: "english",
        model_sha256: "4e16deb22c4da6468bdca539b2cd3c8687825538b67109177c47d359ab994cd7",
        dict_sha256: "0364294b29befa0dafb381b8a2cfa000337ff447728140b266459686f13fed4d",
        model_size_bytes: 7_830_888,
    },
    RecModelDefinition {
        script_family: "chinese",
        model_sha256: "26fa4f47060f58e25962b9af6beaee05c8182b90e026c4ecc6db165d9dfdc38a",
        dict_sha256: "d4f1e80e20cf72770b2fff3e825cd7fb5909bac4784677e307307b2fbdde4304",
        model_size_bytes: 84_468_836,
    },
    RecModelDefinition {
        script_family: "latin",
        model_sha256: "614ffc2d6d3902d360fad7f1b0dd455ee45e877069d14c4e51a99dc4ef144409",
        dict_sha256: "6230982f2773c40b10dc12a3346947a1a771f9be03fd891b294a023357378005",
        model_size_bytes: 7_862_832,
    },
    RecModelDefinition {
        script_family: "korean",
        model_sha256: "322f140154c820fcb83c3d24cfe42c9ec70dd1a1834163306a7338136e4f1eaa",
        dict_sha256: "086835d8f64802da9214d24e7aea3fda477a72d2df4716e9769117ca081059bb",
        model_size_bytes: 13_401_252,
    },
    RecModelDefinition {
        script_family: "eslav",
        model_sha256: "dc6bf0e855247decce214ba6dae5bc135fa0ad725a5918a7fcfb59fad6c9cdee",
        dict_sha256: "71e693f3f04afcd137ec0ce3bdc6732468f784f7f35168b9850e6ffe628a21c3",
        model_size_bytes: 7_870_092,
    },
    RecModelDefinition {
        script_family: "thai",
        model_sha256: "2b6e56b1872200349e227574c25aeb0e0f9af9b8356e9ff5f75ac543a535669a",
        dict_sha256: "40708ca7e0b6222320a5ba690201b77a6b39633273e3fd19e209613d18595d59",
        model_size_bytes: 7_873_480,
    },
    RecModelDefinition {
        script_family: "greek",
        model_sha256: "13373f736dbb229e96945fc41c2573403d91503b0775c7b7294839e0c5f3a7a3",
        dict_sha256: "c361caeae4e2b0e27a453390d65ca27be64fa04d4a6eddd79d91a8a6053141de",
        model_size_bytes: 7_791_200,
    },
    RecModelDefinition {
        script_family: "arabic",
        model_sha256: "7982d371612785238fd99080cff36354deaec84fdc6ff7da9c82af4243fa0c9a",
        dict_sha256: "ce4725178a558324f33ee967753bd4df865ab4dd2784c8fcdce2aad9825062ca",
        model_size_bytes: 8_978_664,
    },
    RecModelDefinition {
        script_family: "devanagari",
        model_sha256: "43df175fa3c877fbf7bcc4e5bd1e203e24ec450cd3ea96c9e802c86e39a4d4cf",
        dict_sha256: "d38810959dae4d0e16c4bdb6e1b000e645b7a43876f04cca746eac93c2b6c643",
        model_size_bytes: 8_980_224,
    },
    RecModelDefinition {
        script_family: "tamil",
        model_sha256: "fba9a00af746c8f3ef4f091cf966880222cd7245a60f6a953670593630c0ca4f",
        dict_sha256: "5530929b4be4221022d0cd35300a097d99157ddbe04e662d0c088d5f6672f265",
        model_size_bytes: 8_970_084,
    },
    RecModelDefinition {
        script_family: "telugu",
        model_sha256: "669946d9ad4de93e8d1b13f7e72c59cc2cc2de91bb24e22aad7503a6cc5757ee",
        dict_sha256: "c0998290b8bfed832b9a6eb4d4d16d88a410b89ab3277d53169b2c4df49c549f",
        model_size_bytes: 8_976_064,
    },
    RecModelDefinition {
        script_family: "kannada",
        model_sha256: "52a75cdf860878447a36b911299bf75419ad55b0e8e68f24904af507e19df958",
        dict_sha256: "baa474fdbeb74c42554f692601fe397610c0522c4240cb9d24feaaf0576eb4ef",
        model_size_bytes: 8_981_366,
    },
];

/// Paths to shared models (detection + classification).
#[derive(Debug, Clone)]
pub struct SharedModelPaths {
    /// Path to the detection model directory.
    pub det_model: PathBuf,
    /// Path to the classification model directory.
    pub cls_model: PathBuf,
}

/// Paths to a recognition model and its character dictionary.
#[derive(Debug, Clone)]
pub struct RecModelPaths {
    /// Path to the recognition model directory.
    pub rec_model: PathBuf,
    /// Path to the character dictionary file.
    pub dict_file: PathBuf,
}

/// Combined paths to all models needed for OCR (backward compatibility).
#[derive(Debug, Clone)]
pub struct ModelPaths {
    /// Path to the detection model directory.
    pub det_model: PathBuf,
    /// Path to the classification model directory.
    pub cls_model: PathBuf,
    /// Path to the recognition model directory.
    pub rec_model: PathBuf,
    /// Path to the character dictionary file.
    pub dict_file: PathBuf,
}

/// Statistics about the PaddleOCR model cache.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total size of cached models in bytes.
    pub total_size_bytes: u64,
    /// Number of models currently cached.
    pub model_count: usize,
    /// Path to the cache directory.
    pub cache_dir: PathBuf,
}

/// Manages PaddleOCR model downloading, caching, and path resolution.
///
/// The model manager ensures that PaddleOCR models are available locally,
/// organized by model type. Shared models (det, cls) are downloaded once,
/// while recognition models are downloaded per-script-family on demand.
#[derive(Debug, Clone)]
pub struct ModelManager {
    cache_dir: PathBuf,
}

impl ModelManager {
    /// Creates a new model manager with the specified cache directory.
    pub fn new(cache_dir: PathBuf) -> Self {
        ModelManager { cache_dir }
    }

    /// Gets the cache directory path.
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Ensures shared models (detection + classification) exist locally.
    ///
    /// Downloads them from HuggingFace if not cached.
    pub fn ensure_shared_models(&self) -> Result<SharedModelPaths, KreuzbergError> {
        fs::create_dir_all(&self.cache_dir)?;

        tracing::info!(cache_dir = ?self.cache_dir, "Checking shared PaddleOCR models");

        for model in SHARED_MODELS {
            let model_file = self.model_file_path(model.model_type);
            if !model_file.exists() {
                tracing::info!(model_type = model.model_type, "Downloading shared model...");
                self.download_shared_model(model)?;
            } else {
                tracing::debug!(model_type = model.model_type, "Shared model found in cache");
            }
        }

        Ok(SharedModelPaths {
            det_model: self.model_path("det"),
            cls_model: self.model_path("cls"),
        })
    }

    /// Ensures a recognition model for the given script family exists locally.
    ///
    /// Downloads the model and character dictionary from HuggingFace if not cached.
    ///
    /// # Arguments
    ///
    /// * `family` - Script family name (e.g., "english", "chinese", "latin")
    pub fn ensure_rec_model(&self, family: &str) -> Result<RecModelPaths, KreuzbergError> {
        let definition = Self::find_rec_definition(family).ok_or_else(|| KreuzbergError::Plugin {
            message: format!("Unsupported script family: {family}"),
            plugin_name: "paddle-ocr".to_string(),
        })?;

        let rec_dir = self.rec_family_path(family);
        let model_file = rec_dir.join("model.onnx");
        let dict_file = rec_dir.join("dict.txt");

        if !model_file.exists() || !dict_file.exists() {
            tracing::info!(family, "Downloading recognition model...");
            fs::create_dir_all(&rec_dir)?;
            self.download_rec_model(definition, &rec_dir)?;
        } else {
            tracing::debug!(family, "Recognition model found in cache");
        }

        Ok(RecModelPaths {
            rec_model: rec_dir,
            dict_file,
        })
    }

    /// Backward-compatible method that ensures all models for English exist.
    pub fn ensure_models_exist(&self) -> Result<ModelPaths, KreuzbergError> {
        let shared = self.ensure_shared_models()?;
        let rec = self.ensure_rec_model("english")?;

        tracing::info!("All PaddleOCR models ready (english)");

        Ok(ModelPaths {
            det_model: shared.det_model,
            cls_model: shared.cls_model,
            rec_model: rec.rec_model,
            dict_file: rec.dict_file,
        })
    }

    /// Find the recognition model definition for a script family.
    fn find_rec_definition(family: &str) -> Option<&'static RecModelDefinition> {
        REC_MODELS.iter().find(|d| d.script_family == family)
    }

    /// Returns the path for a model type directory (det, cls).
    pub fn model_path(&self, model_type: &str) -> PathBuf {
        self.cache_dir.join(model_type)
    }

    /// Returns the path for a recognition family directory.
    fn rec_family_path(&self, family: &str) -> PathBuf {
        self.cache_dir.join("rec").join(family)
    }

    /// Returns the full path to the ONNX model file for a given type.
    fn model_file_path(&self, model_type: &str) -> PathBuf {
        self.model_path(model_type).join("model.onnx")
    }

    /// Download a shared model (det or cls) from HuggingFace Hub.
    fn download_shared_model(&self, model: &SharedModelDefinition) -> Result<(), KreuzbergError> {
        let model_dir = self.model_path(model.model_type);
        let model_file = model_dir.join(model.local_filename);

        fs::create_dir_all(&model_dir)?;

        let cached_path = self.hf_download(model.remote_filename)?;

        if !model.sha256_checksum.is_empty() {
            Self::verify_checksum(&cached_path, model.sha256_checksum, model.model_type)?;
        }

        fs::copy(&cached_path, &model_file).map_err(|e| KreuzbergError::Plugin {
            message: format!("Failed to copy model to {}: {}", model_file.display(), e),
            plugin_name: "paddle-ocr".to_string(),
        })?;

        tracing::info!(path = ?model_file, model_type = model.model_type, "Shared model saved");
        Ok(())
    }

    /// Download a recognition model + dict for a script family.
    fn download_rec_model(&self, definition: &RecModelDefinition, rec_dir: &Path) -> Result<(), KreuzbergError> {
        let family = definition.script_family;

        // Download model
        let remote_model = format!("rec/{family}/model.onnx");
        let cached_model_path = self.hf_download(&remote_model)?;
        Self::verify_checksum(&cached_model_path, definition.model_sha256, &format!("rec/{family}"))?;
        let local_model = rec_dir.join("model.onnx");
        fs::copy(&cached_model_path, &local_model).map_err(|e| KreuzbergError::Plugin {
            message: format!("Failed to copy rec model to {}: {}", local_model.display(), e),
            plugin_name: "paddle-ocr".to_string(),
        })?;

        // Download dict
        let remote_dict = format!("rec/{family}/dict.txt");
        let cached_dict_path = self.hf_download(&remote_dict)?;
        Self::verify_checksum(&cached_dict_path, definition.dict_sha256, &format!("rec/{family}/dict"))?;
        let local_dict = rec_dir.join("dict.txt");
        fs::copy(&cached_dict_path, &local_dict).map_err(|e| KreuzbergError::Plugin {
            message: format!("Failed to copy dict to {}: {}", local_dict.display(), e),
            plugin_name: "paddle-ocr".to_string(),
        })?;

        tracing::info!(family, "Recognition model and dict saved");
        Ok(())
    }

    /// Download a file from the HuggingFace Hub.
    fn hf_download(&self, remote_filename: &str) -> Result<PathBuf, KreuzbergError> {
        tracing::info!(repo = HF_REPO_ID, filename = remote_filename, "Downloading via hf-hub");

        let api = hf_hub::api::sync::ApiBuilder::new()
            .with_progress(true)
            .build()
            .map_err(|e| KreuzbergError::Plugin {
                message: format!("Failed to initialize HuggingFace Hub API: {e}"),
                plugin_name: "paddle-ocr".to_string(),
            })?;

        let repo = api.model(HF_REPO_ID.to_string());
        let cached_path = repo.get(remote_filename).map_err(|e| KreuzbergError::Plugin {
            message: format!("Failed to download '{remote_filename}' from {HF_REPO_ID}: {e}"),
            plugin_name: "paddle-ocr".to_string(),
        })?;

        Ok(cached_path)
    }

    /// Verify SHA256 checksum of a downloaded file.
    fn verify_checksum(path: &Path, expected: &str, label: &str) -> Result<(), KreuzbergError> {
        if expected.is_empty() {
            return Ok(());
        }

        let bytes = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash_hex = hex::encode(hasher.finalize());

        if hash_hex != expected {
            return Err(KreuzbergError::Validation {
                message: format!("Checksum mismatch for {label}: expected {expected}, got {hash_hex}"),
                source: None,
            });
        }
        tracing::debug!(label, "Checksum verified");
        Ok(())
    }

    /// Checks if shared models (det + cls) are cached locally.
    pub fn are_shared_models_cached(&self) -> bool {
        SHARED_MODELS.iter().all(|model| {
            let f = self.model_file_path(model.model_type);
            f.exists() && f.is_file()
        })
    }

    /// Checks if a recognition model for the given family is cached.
    pub fn is_rec_model_cached(&self, family: &str) -> bool {
        let rec_dir = self.rec_family_path(family);
        rec_dir.join("model.onnx").exists() && rec_dir.join("dict.txt").exists()
    }

    /// Checks if all required models are cached (shared + English rec).
    pub fn are_models_cached(&self) -> bool {
        self.are_shared_models_cached() && self.is_rec_model_cached("english")
    }

    /// Clears all cached models from the cache directory.
    pub fn clear_cache(&self) -> Result<(), KreuzbergError> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            tracing::info!(?self.cache_dir, "Cache directory cleared");
        }
        Ok(())
    }

    /// Returns statistics about the current cache.
    pub fn cache_stats(&self) -> Result<CacheStats, KreuzbergError> {
        let mut total_size = 0u64;
        let mut model_count = 0usize;

        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir()
                    && let Ok(size) = Self::dir_size(&path)
                {
                    total_size += size;
                    if let Ok(entries) = fs::read_dir(&path) {
                        model_count += entries.count();
                    }
                }
            }
        }

        Ok(CacheStats {
            total_size_bytes: total_size,
            model_count,
            cache_dir: self.cache_dir.clone(),
        })
    }

    /// Recursively calculates the size of a directory in bytes.
    fn dir_size(path: &Path) -> std::io::Result<u64> {
        let mut size = 0u64;
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                size += Self::dir_size(&entry.path())?;
            } else {
                size += metadata.len();
            }
        }
        Ok(size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_model_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());
        assert_eq!(manager.cache_dir(), &temp_dir.path().to_path_buf());
    }

    #[test]
    fn test_model_path_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        let det_path = manager.model_path("det");
        assert!(det_path.to_string_lossy().contains("det"));

        let cls_path = manager.model_path("cls");
        assert!(cls_path.to_string_lossy().contains("cls"));
    }

    #[test]
    fn test_rec_family_path() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        let english_path = manager.rec_family_path("english");
        assert!(english_path.ends_with("rec/english"));

        let chinese_path = manager.rec_family_path("chinese");
        assert!(chinese_path.ends_with("rec/chinese"));
    }

    #[test]
    fn test_find_rec_definition_all_families() {
        let families = [
            "english",
            "chinese",
            "latin",
            "korean",
            "eslav",
            "thai",
            "greek",
            "arabic",
            "devanagari",
            "tamil",
            "telugu",
            "kannada",
        ];
        for family in families {
            let def = ModelManager::find_rec_definition(family);
            assert!(def.is_some(), "Should find definition for {family}");
            assert_eq!(def.unwrap().script_family, family);
            assert!(!def.unwrap().model_sha256.is_empty());
            assert!(!def.unwrap().dict_sha256.is_empty());
        }
    }

    #[test]
    fn test_find_rec_definition_unknown() {
        assert!(ModelManager::find_rec_definition("unknown").is_none());
        assert!(ModelManager::find_rec_definition("").is_none());
    }

    #[test]
    fn test_are_shared_models_cached_empty() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());
        assert!(!manager.are_shared_models_cached());
    }

    #[test]
    fn test_are_shared_models_cached_present() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        for model_type in &["det", "cls"] {
            let dir = manager.model_path(model_type);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("model.onnx"), "fake").unwrap();
        }

        assert!(manager.are_shared_models_cached());
    }

    #[test]
    fn test_is_rec_model_cached() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        assert!(!manager.is_rec_model_cached("english"));

        let rec_dir = manager.rec_family_path("english");
        fs::create_dir_all(&rec_dir).unwrap();
        fs::write(rec_dir.join("model.onnx"), "fake").unwrap();
        // Still false - dict missing
        assert!(!manager.is_rec_model_cached("english"));

        fs::write(rec_dir.join("dict.txt"), "#\na\n ").unwrap();
        assert!(manager.is_rec_model_cached("english"));
    }

    #[test]
    fn test_are_models_cached_requires_both() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        // Create shared models only
        for model_type in &["det", "cls"] {
            let dir = manager.model_path(model_type);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("model.onnx"), "fake").unwrap();
        }
        assert!(!manager.are_models_cached());

        // Add english rec
        let rec_dir = manager.rec_family_path("english");
        fs::create_dir_all(&rec_dir).unwrap();
        fs::write(rec_dir.join("model.onnx"), "fake").unwrap();
        fs::write(rec_dir.join("dict.txt"), "#\na\n ").unwrap();
        assert!(manager.are_models_cached());
    }

    #[test]
    fn test_clear_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("paddle_cache");
        let manager = ModelManager::new(cache_dir.clone());

        fs::create_dir_all(manager.model_path("det")).unwrap();
        fs::write(manager.model_path("det").join("model.onnx"), "test").unwrap();

        assert!(cache_dir.exists());
        manager.clear_cache().unwrap();
        assert!(!cache_dir.exists());
    }

    #[test]
    fn test_cache_stats_empty() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        let stats = manager.cache_stats().unwrap();
        assert_eq!(stats.total_size_bytes, 0);
        assert_eq!(stats.model_count, 0);
    }

    #[test]
    fn test_cache_stats_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        let det_path = manager.model_path("det");
        fs::create_dir_all(&det_path).unwrap();
        fs::write(det_path.join("model.onnx"), "x".repeat(1000)).unwrap();

        let cls_path = manager.model_path("cls");
        fs::create_dir_all(&cls_path).unwrap();
        fs::write(cls_path.join("model.onnx"), "y".repeat(2000)).unwrap();

        let stats = manager.cache_stats().unwrap();
        assert!(stats.total_size_bytes >= 3000);
    }

    #[test]
    fn test_shared_model_definitions() {
        assert_eq!(SHARED_MODELS.len(), 2);
        let types: Vec<_> = SHARED_MODELS.iter().map(|m| m.model_type).collect();
        assert!(types.contains(&"det"));
        assert!(types.contains(&"cls"));
    }

    #[test]
    fn test_rec_model_definitions() {
        assert_eq!(REC_MODELS.len(), 12);
        let families: Vec<_> = REC_MODELS.iter().map(|m| m.script_family).collect();
        assert!(families.contains(&"english"));
        assert!(families.contains(&"chinese"));
        assert!(families.contains(&"latin"));
        assert!(families.contains(&"korean"));
        assert!(families.contains(&"eslav"));
        assert!(families.contains(&"thai"));
        assert!(families.contains(&"greek"));
        assert!(families.contains(&"arabic"));
        assert!(families.contains(&"devanagari"));
        assert!(families.contains(&"tamil"));
        assert!(families.contains(&"telugu"));
        assert!(families.contains(&"kannada"));
    }

    #[test]
    fn test_model_paths_cloneable() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        // Pre-populate cache so ensure_models_exist doesn't try to download
        for model_type in &["det", "cls"] {
            let dir = manager.model_path(model_type);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("model.onnx"), "fake").unwrap();
        }
        let rec_dir = manager.rec_family_path("english");
        fs::create_dir_all(&rec_dir).unwrap();
        fs::write(rec_dir.join("model.onnx"), "fake").unwrap();
        fs::write(rec_dir.join("dict.txt"), "#\na\n ").unwrap();

        let paths1 = manager.ensure_models_exist().unwrap();
        let paths2 = paths1.clone();
        assert_eq!(paths1.det_model, paths2.det_model);
        assert_eq!(paths1.cls_model, paths2.cls_model);
        assert_eq!(paths1.rec_model, paths2.rec_model);
        assert_eq!(paths1.dict_file, paths2.dict_file);
    }

    #[test]
    fn test_ensure_shared_models_with_cache() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        // Pre-populate
        for model_type in &["det", "cls"] {
            let dir = manager.model_path(model_type);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("model.onnx"), "fake").unwrap();
        }

        let paths = manager.ensure_shared_models().unwrap();
        assert!(paths.det_model.ends_with("det"));
        assert!(paths.cls_model.ends_with("cls"));
    }

    #[test]
    fn test_ensure_rec_model_with_cache() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        let rec_dir = manager.rec_family_path("chinese");
        fs::create_dir_all(&rec_dir).unwrap();
        fs::write(rec_dir.join("model.onnx"), "fake").unwrap();
        fs::write(rec_dir.join("dict.txt"), "#\na\n ").unwrap();

        let paths = manager.ensure_rec_model("chinese").unwrap();
        assert!(paths.rec_model.ends_with("rec/chinese"));
        assert!(paths.dict_file.ends_with("rec/chinese/dict.txt"));
    }

    #[test]
    fn test_ensure_rec_model_unsupported_family() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        let result = manager.ensure_rec_model("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_checksum_correct() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.bin");
        fs::write(&file_path, b"hello").unwrap();

        // SHA256 of "hello"
        let expected = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(ModelManager::verify_checksum(&file_path, expected, "test").is_ok());
    }

    #[test]
    fn test_verify_checksum_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.bin");
        fs::write(&file_path, b"hello").unwrap();

        let result = ModelManager::verify_checksum(&file_path, "0000000000000000", "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_checksum_empty_skips() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.bin");
        fs::write(&file_path, b"hello").unwrap();

        assert!(ModelManager::verify_checksum(&file_path, "", "test").is_ok());
    }
}
