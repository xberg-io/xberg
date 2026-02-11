/// Model downloading and caching for PaddleOCR.
///
/// This module handles PaddleOCR model path resolution, downloading, and caching operations.
/// Models are organized into three types: detection, classification, and recognition.
///
/// # Model Download Flow
///
/// 1. Check if models exist in cache directory
/// 2. If not, download ONNX models from HuggingFace Hub via hf-hub
/// 3. Verify SHA256 checksums
/// 4. Copy models to local cache directory
///
/// # Examples
///
/// ```no_run
/// use kreuzberg::ModelManager;
/// use std::path::PathBuf;
///
/// let manager = ModelManager::new(PathBuf::from("/tmp/paddle_models"));
/// let paths = manager.ensure_models_exist()?;
/// println!("Detection model: {:?}", paths.det_model);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::KreuzbergError;
use sha2::{Digest, Sha256};

/// HuggingFace repository containing PaddleOCR ONNX models.
/// Must be a public repo (or user must set HF_TOKEN for private repos).
const HF_REPO_ID: &str = "Kreuzberg/paddleocr-onnx-models";

/// Model definition with metadata.
#[derive(Debug, Clone)]
struct ModelDefinition {
    /// Model type identifier (det, cls, rec)
    model_type: &'static str,
    /// Remote filename on the server
    remote_filename: &'static str,
    /// Local filename after download
    local_filename: &'static str,
    /// SHA256 checksum of the file (empty string skips verification)
    sha256_checksum: &'static str,
    /// Approximate size in bytes (for progress reporting)
    #[allow(dead_code)]
    size_bytes: u64,
}

/// Model definitions with ONNX model files.
/// These are pre-converted PP-OCRv4 models in ONNX format hosted on HuggingFace.
///
/// Sources:
/// - det: `ch_PP-OCRv4_det_infer.onnx` — PP-OCRv4 detection model (language-agnostic),
///   sourced from SWHL/RapidOCR on HuggingFace.
/// - cls: `ch_ppocr_mobile_v2.0_cls_infer.onnx` — PPOCRv2 text angle classifier,
///   sourced from SWHL/RapidOCR on HuggingFace.
/// - rec: `en_PP-OCRv4_rec_infer.onnx` — PP-OCRv4 English recognition model,
///   converted from PaddlePaddle format via paddle2onnx.
const MODELS: &[ModelDefinition] = &[
    ModelDefinition {
        model_type: "det",
        remote_filename: "ch_PP-OCRv4_det_infer.onnx",
        local_filename: "model.onnx",
        sha256_checksum: "d2a7720d45a54257208b1e13e36a8479894cb74155a5efe29462512d42f49da9",
        size_bytes: 4_745_517,
    },
    ModelDefinition {
        model_type: "cls",
        remote_filename: "ch_ppocr_mobile_v2.0_cls_infer.onnx",
        local_filename: "model.onnx",
        sha256_checksum: "e47acedf663230f8863ff1ab0e64dd2d82b838fceb5957146dab185a89d6215c",
        size_bytes: 585_532,
    },
    ModelDefinition {
        model_type: "rec",
        remote_filename: "en_PP-OCRv4_rec_infer.onnx",
        local_filename: "model.onnx",
        sha256_checksum: "c8f9b6f4d541991132f0971a4fbe879b79f226bb40174a385407e6be09099e6a",
        size_bytes: 7_684_265,
    },
];

/// Character dictionary for en_PP-OCRv4 recognition model.
///
/// The `ort` crate cannot read custom metadata from PaddlePaddle PIR-mode ONNX models,
/// so we ship the dictionary alongside the model files. This contains 97 entries:
/// CTC blank '#', 95 printable ASCII characters in model order, and trailing space.
const EN_PPOCRV4_DICT: &str = "#\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n:\n;\n<\n=\n>\n?\n@\nA\nB\nC\nD\nE\nF\nG\nH\nI\nJ\nK\nL\nM\nN\nO\nP\nQ\nR\nS\nT\nU\nV\nW\nX\nY\nZ\n[\n\\\n]\n^\n_\n`\na\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn\no\np\nq\nr\ns\nt\nu\nv\nw\nx\ny\nz\n{\n|\n}\n~\n!\n\"\n#\n$\n%\n&\n'\n(\n)\n*\n+\n,\n-\n.\n/\n \n ";

/// Paths to all three required PaddleOCR models.
#[derive(Debug, Clone)]
pub struct ModelPaths {
    /// Path to the detection (text location) model.
    pub det_model: PathBuf,
    /// Path to the classification (text orientation) model.
    pub cls_model: PathBuf,
    /// Path to the recognition (text reading) model.
    pub rec_model: PathBuf,
    /// Path to the character dictionary file for the recognition model.
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
/// organized by model type (detection, classification, recognition).
///
/// # Cache Structure
///
/// Models are cached in the following structure:
/// ```text
/// cache_dir/
/// ├── det/
/// │   └── en_PP-OCRv4_det_infer/
/// │       ├── inference.pdmodel
/// │       └── inference.pdiparams
/// ├── cls/
/// │   └── ch_ppocr_mobile_v2.0_cls_infer/
/// │       ├── inference.pdmodel
/// │       └── inference.pdiparams
/// └── rec/
///     └── en_PP-OCRv4_rec_infer/
///         ├── inference.pdmodel
///         └── inference.pdiparams
/// ```
#[derive(Debug, Clone)]
pub struct ModelManager {
    cache_dir: PathBuf,
}

impl ModelManager {
    /// Creates a new model manager with the specified cache directory.
    ///
    /// The cache directory will be created if it does not already exist.
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Path to the directory where models will be cached.
    ///
    /// # Examples
    ///
    /// ```
    /// use kreuzberg::ModelManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = ModelManager::new(PathBuf::from("/tmp/paddle_models"));
    /// ```
    pub fn new(cache_dir: PathBuf) -> Self {
        ModelManager { cache_dir }
    }

    /// Gets the cache directory path.
    ///
    /// # Examples
    ///
    /// ```
    /// use kreuzberg::ModelManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = ModelManager::new(PathBuf::from("/tmp/models"));
    /// assert_eq!(manager.cache_dir(), &PathBuf::from("/tmp/models"));
    /// ```
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Ensures that all required models exist locally, downloading if necessary.
    ///
    /// This method checks if all three models (detection, classification, recognition)
    /// are cached locally. If any are missing, they will be downloaded from the
    /// PaddleOCR model repository.
    ///
    /// # Returns
    ///
    /// `Ok(ModelPaths)` containing paths to all three models if successful.
    /// `Err(KreuzbergError)` if the cache directory cannot be created, models cannot be downloaded,
    /// or checksum verification fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kreuzberg::ModelManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = ModelManager::new(PathBuf::from("/tmp/paddle_models"));
    /// let paths = manager.ensure_models_exist()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn ensure_models_exist(&self) -> Result<ModelPaths, KreuzbergError> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&self.cache_dir)?;

        tracing::info!(
            cache_dir = ?self.cache_dir,
            "Checking for cached PaddleOCR models"
        );

        // Check and download each model if necessary
        for model in MODELS {
            if !self.is_model_cached(model.model_type) {
                tracing::info!(
                    model_type = model.model_type,
                    "Model not found in cache, downloading..."
                );
                self.download_model(model)?;
            } else {
                tracing::debug!(model_type = model.model_type, "Model found in cache");
            }
        }

        // Write character dictionary file for recognition model.
        // The ort crate cannot read custom metadata from PaddlePaddle PIR-mode ONNX models,
        // so we ship the dictionary as a separate file.
        let dict_file = self.dict_file_path();
        if !dict_file.exists() {
            let rec_dir = self.model_path("rec");
            fs::create_dir_all(&rec_dir)?;
            fs::write(&dict_file, EN_PPOCRV4_DICT)?;
            tracing::debug!("Character dictionary written to {:?}", dict_file);
        }

        tracing::info!("All PaddleOCR models ready");

        Ok(ModelPaths {
            det_model: self.model_path("det"),
            cls_model: self.model_path("cls"),
            rec_model: self.model_path("rec"),
            dict_file,
        })
    }

    /// Download a single model from HuggingFace Hub.
    ///
    /// Downloads the model file via hf-hub (which handles auth, caching, and CDN),
    /// verifies its checksum (if provided), and copies it to the appropriate cache directory.
    fn download_model(&self, model: &ModelDefinition) -> Result<(), KreuzbergError> {
        let model_dir = self.model_path(model.model_type);
        let model_file = model_dir.join(model.local_filename);

        tracing::info!(
            repo = HF_REPO_ID,
            filename = model.remote_filename,
            model_type = model.model_type,
            "Downloading PaddleOCR model via hf-hub"
        );

        // Create model directory
        fs::create_dir_all(&model_dir)?;

        // hf-hub handles auth (HF_TOKEN env), caching, CDN, retries
        let api = hf_hub::api::sync::ApiBuilder::new()
            .with_progress(true)
            .build()
            .map_err(|e| KreuzbergError::Plugin {
                message: format!("Failed to initialize HuggingFace Hub API: {}", e),
                plugin_name: "paddle-ocr".to_string(),
            })?;

        let repo = api.model(HF_REPO_ID.to_string());
        let cached_path = repo.get(model.remote_filename).map_err(|e| KreuzbergError::Plugin {
            message: format!(
                "Failed to download '{}' from {}: {}",
                model.remote_filename, HF_REPO_ID, e
            ),
            plugin_name: "paddle-ocr".to_string(),
        })?;

        // Verify checksum if provided
        if !model.sha256_checksum.is_empty() {
            let bytes = fs::read(&cached_path)?;
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let hash_hex = hex::encode(hasher.finalize());

            if hash_hex != model.sha256_checksum {
                return Err(KreuzbergError::Validation {
                    message: format!(
                        "Checksum mismatch for {} model: expected {}, got {}",
                        model.model_type, model.sha256_checksum, hash_hex
                    ),
                    source: None,
                });
            }
            tracing::debug!(model_type = model.model_type, "Checksum verified");
        }

        // Copy from hf-hub cache to our cache structure
        fs::copy(&cached_path, &model_file).map_err(|e| KreuzbergError::Plugin {
            message: format!("Failed to copy model to {}: {}", model_file.display(), e),
            plugin_name: "paddle-ocr".to_string(),
        })?;

        tracing::info!(
            path = ?model_file,
            model_type = model.model_type,
            "Model saved to cache"
        );

        Ok(())
    }

    /// Returns the path where a model of the given type should be cached.
    ///
    /// This returns the expected path for the model directory, regardless of
    /// whether the model actually exists on disk.
    ///
    /// # Arguments
    ///
    /// * `model_type` - One of "det" (detection), "cls" (classification), or "rec" (recognition).
    ///
    /// # Examples
    ///
    /// ```
    /// use kreuzberg::ModelManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = ModelManager::new(PathBuf::from("/tmp/paddle_models"));
    /// let det_path = manager.model_path("det");
    /// assert!(det_path.starts_with("/tmp/paddle_models/det"));
    /// ```
    pub fn model_path(&self, model_type: &str) -> PathBuf {
        // Model directory is organized by type
        self.cache_dir.join(model_type)
    }

    /// Returns the full path to the ONNX model file for a given type.
    fn model_file_path(&self, model_type: &str) -> PathBuf {
        self.model_path(model_type).join("model.onnx")
    }

    /// Returns the path to the character dictionary file.
    fn dict_file_path(&self) -> PathBuf {
        self.model_path("rec").join("dict.txt")
    }

    /// Checks if all required models are cached locally.
    ///
    /// This performs a basic check for the existence of model files.
    /// It does not verify model integrity or completeness.
    ///
    /// # Returns
    ///
    /// `true` if all three models appear to be cached, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kreuzberg::ModelManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = ModelManager::new(PathBuf::from("/tmp/paddle_models"));
    /// if manager.are_models_cached() {
    ///     println!("All models are cached");
    /// }
    /// ```
    pub fn are_models_cached(&self) -> bool {
        MODELS.iter().all(|model| {
            let model_file = self.model_file_path(model.model_type);
            model_file.exists() && model_file.is_file()
        })
    }

    /// Check if a specific model is cached.
    fn is_model_cached(&self, model_type: &str) -> bool {
        let model_file = self.model_file_path(model_type);
        model_file.exists() && model_file.is_file()
    }

    /// Clears all cached models from the cache directory.
    ///
    /// This deletes the entire cache directory and all its contents.
    /// Use with caution.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the cache was successfully cleared.
    /// `Err(KreuzbergError)` if deletion failed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kreuzberg::ModelManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = ModelManager::new(PathBuf::from("/tmp/paddle_models"));
    /// manager.clear_cache()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn clear_cache(&self) -> Result<(), KreuzbergError> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            tracing::info!(?self.cache_dir, "Cache directory cleared");
        }
        Ok(())
    }

    /// Returns statistics about the current cache.
    ///
    /// This recursively calculates the total size of all cached models.
    ///
    /// # Returns
    ///
    /// `Ok(CacheStats)` containing cache information.
    /// `Err(KreuzbergError)` if the cache directory cannot be read.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kreuzberg::ModelManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = ModelManager::new(PathBuf::from("/tmp/paddle_models"));
    /// let stats = manager.cache_stats()?;
    /// println!("Cache size: {} bytes", stats.total_size_bytes);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn cache_stats(&self) -> Result<CacheStats, KreuzbergError> {
        let mut total_size = 0u64;
        let mut model_count = 0usize;

        if self.cache_dir.exists() {
            for entry in fs::read_dir(&self.cache_dir)? {
                let entry = entry?;

                let path = entry.path();
                if path.is_dir() {
                    // Count this as a potential model type directory
                    if let Ok(size) = Self::dir_size(&path) {
                        total_size += size;
                        // Count subdirectories within type as model count
                        if let Ok(entries) = fs::read_dir(&path) {
                            model_count += entries.count();
                        }
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
    ///
    /// # Arguments
    ///
    /// * `path` - The directory path to measure.
    ///
    /// # Returns
    ///
    /// `Ok(u64)` with the total size in bytes.
    /// `Err(std::io::Error)` if the directory cannot be read.
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

        let rec_path = manager.model_path("rec");
        assert!(rec_path.to_string_lossy().contains("rec"));
    }

    #[test]
    fn test_model_file_path() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        let det_file = manager.model_file_path("det");
        assert!(det_file.to_string_lossy().ends_with("det/model.onnx"));

        let cls_file = manager.model_file_path("cls");
        assert!(cls_file.to_string_lossy().ends_with("cls/model.onnx"));

        let rec_file = manager.model_file_path("rec");
        assert!(rec_file.to_string_lossy().ends_with("rec/model.onnx"));
    }

    #[test]
    fn test_are_models_cached_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        // Should return false when cache is empty
        assert!(!manager.are_models_cached());
    }

    #[test]
    fn test_are_models_cached_partial() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        // Create only det model file
        let det_path = manager.model_path("det");
        fs::create_dir_all(&det_path).unwrap();
        fs::write(det_path.join("model.onnx"), "fake model data").unwrap();

        // Should return false when only some models are cached
        assert!(!manager.are_models_cached());
    }

    #[test]
    fn test_are_models_cached_all_present() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        // Create all model files
        for model_type in &["det", "cls", "rec"] {
            let model_dir = manager.model_path(model_type);
            fs::create_dir_all(&model_dir).unwrap();
            fs::write(model_dir.join("model.onnx"), "fake model data").unwrap();
        }

        // Should return true when all models are present
        assert!(manager.are_models_cached());
    }

    #[test]
    fn test_is_model_cached() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        // Initially not cached
        assert!(!manager.is_model_cached("det"));

        // Create model file
        let det_path = manager.model_path("det");
        fs::create_dir_all(&det_path).unwrap();
        fs::write(det_path.join("model.onnx"), "fake model data").unwrap();

        // Now cached
        assert!(manager.is_model_cached("det"));
    }

    #[test]
    fn test_clear_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("paddle_cache");
        let manager = ModelManager::new(cache_dir.clone());

        // Create some dummy files
        fs::create_dir_all(manager.model_path("det")).unwrap();
        fs::write(manager.model_path("det").join("model.onnx"), "test content").unwrap();

        assert!(cache_dir.exists());

        // Clear cache
        manager.clear_cache().unwrap();

        // Cache should be gone
        assert!(!cache_dir.exists());
    }

    #[test]
    fn test_cache_stats_empty_cache() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        let stats = manager.cache_stats().unwrap();

        assert_eq!(stats.total_size_bytes, 0);
        assert_eq!(stats.model_count, 0);
        assert_eq!(stats.cache_dir, temp_dir.path().to_path_buf());
    }

    #[test]
    fn test_cache_stats_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        // Create model directories with files
        let det_path = manager.model_path("det");
        fs::create_dir_all(&det_path).unwrap();
        fs::write(det_path.join("model.onnx"), "x".repeat(1000)).unwrap();

        let cls_path = manager.model_path("cls");
        fs::create_dir_all(&cls_path).unwrap();
        fs::write(cls_path.join("model.onnx"), "y".repeat(2000)).unwrap();

        let stats = manager.cache_stats().unwrap();

        // Should have at least 3000 bytes (1000 + 2000)
        assert!(stats.total_size_bytes >= 3000);
        // Note: model_count counts subdirectories within type directories
    }

    #[test]
    fn test_model_paths_struct_cloneable() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        // Create fake model files so ensure_models_exist doesn't try to download
        for model_type in &["det", "cls", "rec"] {
            let model_dir = manager.model_path(model_type);
            fs::create_dir_all(&model_dir).unwrap();
            fs::write(model_dir.join("model.onnx"), "fake model data").unwrap();
        }

        let paths1 = manager.ensure_models_exist().unwrap();
        let paths2 = paths1.clone();

        assert_eq!(paths1.det_model, paths2.det_model);
        assert_eq!(paths1.cls_model, paths2.cls_model);
        assert_eq!(paths1.rec_model, paths2.rec_model);
    }

    #[test]
    fn test_cache_stats_struct_cloneable() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        let stats1 = manager.cache_stats().unwrap();
        let stats2 = stats1.clone();

        assert_eq!(stats1.total_size_bytes, stats2.total_size_bytes);
        assert_eq!(stats1.model_count, stats2.model_count);
        assert_eq!(stats1.cache_dir, stats2.cache_dir);
    }

    #[test]
    fn test_model_definitions() {
        // Verify model definitions are well-formed
        assert_eq!(MODELS.len(), 3);

        let model_types: Vec<_> = MODELS.iter().map(|m| m.model_type).collect();
        assert!(model_types.contains(&"det"));
        assert!(model_types.contains(&"cls"));
        assert!(model_types.contains(&"rec"));

        // All should have remote filenames ending in .onnx
        for model in MODELS {
            assert!(
                model.remote_filename.ends_with(".onnx"),
                "Model {} should have .onnx extension",
                model.model_type
            );
        }
    }
}
