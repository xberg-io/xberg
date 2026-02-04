/// Model downloading and caching for PaddleOCR.
///
/// This module handles PaddleOCR model path resolution and caching operations.
/// Models are organized into three types: detection, classification, and recognition.
///
/// # Examples
///
/// ```no_run
/// use kreuzberg::ocr::paddle::ModelManager;
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

/// Base URL for PaddleOCR model downloads.
#[allow(dead_code)]
const MODEL_BASE_URL: &str = "https://paddleocr.bj.bcebos.com/";

/// Model definitions: (model_type, relative_path, sha256_checksum).
/// Note: Checksums are placeholders and should be verified with actual model downloads.
const MODELS: &[(&str, &str, &str)] = &[
    (
        "det",
        "PP-OCRv4/en_PP-OCRv4_det_infer.tar",
        "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b",
    ),
    (
        "cls",
        "dygraph_v2.0/ch/ch_ppocr_mobile_v2.0_cls_infer.tar",
        "b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c",
    ),
    (
        "rec",
        "PP-OCRv4/en_PP-OCRv4_rec_infer.tar",
        "c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d",
    ),
];

/// Paths to all three required PaddleOCR models.
#[derive(Debug, Clone)]
pub struct ModelPaths {
    /// Path to the detection (text location) model.
    pub det_model: PathBuf,
    /// Path to the classification (text orientation) model.
    pub cls_model: PathBuf,
    /// Path to the recognition (text reading) model.
    pub rec_model: PathBuf,
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
    /// use kreuzberg::ocr::paddle::ModelManager;
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
    /// use kreuzberg::ocr::paddle::ModelManager;
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
    /// are cached locally. If any are missing, they will be downloaded.
    ///
    /// # Returns
    ///
    /// `Ok(ModelPaths)` containing paths to all three models if successful.
    /// `Err(KreuzbergError)` if the cache directory cannot be created or models cannot be verified.
    ///
    /// # TODO
    ///
    /// Implement actual model downloading from PaddleOCR servers with:
    /// - HTTP client integration
    /// - Checksum verification
    /// - Progress reporting
    /// - Automatic tar extraction
    /// - Fallback to CPU model variants
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kreuzberg::ocr::paddle::ModelManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = ModelManager::new(PathBuf::from("/tmp/paddle_models"));
    /// let paths = manager.ensure_models_exist()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn ensure_models_exist(&self) -> Result<ModelPaths, KreuzbergError> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&self.cache_dir)?;

        let det_model = self.model_path("det");
        let cls_model = self.model_path("cls");
        let rec_model = self.model_path("rec");

        tracing::info!(
            cache_dir = ?self.cache_dir,
            "Checking for cached PaddleOCR models"
        );

        // TODO: Implement model downloading
        // For now, just return the paths if they exist or would exist
        // In a real implementation, we would:
        // 1. Check if models exist locally
        // 2. If not, download from MODEL_BASE_URL using the paths from MODELS
        // 3. Verify checksums match the constants
        // 4. Extract tar archives
        // 5. Report progress via tracing

        if self.are_models_cached() {
            tracing::info!("All PaddleOCR models found in cache");
        } else {
            tracing::info!("Some models missing; would download in full implementation");
        }

        Ok(ModelPaths {
            det_model,
            cls_model,
            rec_model,
        })
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
    /// use kreuzberg::ocr::paddle::ModelManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = ModelManager::new(PathBuf::from("/tmp/paddle_models"));
    /// let det_path = manager.model_path("det");
    /// assert!(det_path.starts_with("/tmp/paddle_models/det"));
    /// ```
    pub fn model_path(&self, model_type: &str) -> PathBuf {
        let model_dir = MODELS
            .iter()
            .find(|(t, _, _)| t == &model_type)
            .map(|(_, path, _)| {
                // Extract the model name from the path (last component without .tar)
                Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or("model")
            })
            .unwrap_or("model");

        self.cache_dir.join(model_type).join(model_dir)
    }

    /// Checks if all required models are cached locally.
    ///
    /// This performs a basic check for the existence of model directories.
    /// It does not verify model integrity or completeness.
    ///
    /// # Returns
    ///
    /// `true` if all three models appear to be cached, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use kreuzberg::ocr::paddle::ModelManager;
    /// use std::path::PathBuf;
    ///
    /// let manager = ModelManager::new(PathBuf::from("/tmp/paddle_models"));
    /// if manager.are_models_cached() {
    ///     println!("All models are cached");
    /// }
    /// ```
    pub fn are_models_cached(&self) -> bool {
        MODELS.iter().all(|(model_type, _, _)| {
            let path = self.model_path(model_type);
            path.exists() && path.is_dir()
        })
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
    /// use kreuzberg::ocr::paddle::ModelManager;
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
    /// use kreuzberg::ocr::paddle::ModelManager;
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
    use std::fs;
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
        assert!(det_path.to_string_lossy().contains("PP-OCRv4_det_infer"));

        let cls_path = manager.model_path("cls");
        assert!(cls_path.to_string_lossy().contains("cls"));
        assert!(cls_path.to_string_lossy().contains("ppocr_mobile_v2.0_cls_infer"));

        let rec_path = manager.model_path("rec");
        assert!(rec_path.to_string_lossy().contains("rec"));
        assert!(rec_path.to_string_lossy().contains("PP-OCRv4_rec_infer"));
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

        // Create only det model directory
        let det_path = manager.model_path("det");
        fs::create_dir_all(&det_path).unwrap();

        // Should return false when only some models are cached
        assert!(!manager.are_models_cached());
    }

    #[test]
    fn test_are_models_cached_all_present() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        // Create all model directories
        fs::create_dir_all(manager.model_path("det")).unwrap();
        fs::create_dir_all(manager.model_path("cls")).unwrap();
        fs::create_dir_all(manager.model_path("rec")).unwrap();

        // Should return true when all models are present
        assert!(manager.are_models_cached());
    }

    #[test]
    fn test_ensure_models_exist_creates_cache_dir() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("paddle_cache");

        let manager = ModelManager::new(cache_dir.clone());

        // Cache directory should not exist yet
        assert!(!cache_dir.exists());

        // Call ensure_models_exist
        let result = manager.ensure_models_exist();

        // Cache directory should now exist
        assert!(cache_dir.exists());
        assert!(result.is_ok());
    }

    #[test]
    fn test_ensure_models_exist_returns_paths() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        let paths = manager.ensure_models_exist().unwrap();

        assert!(paths.det_model.to_string_lossy().contains("det"));
        assert!(paths.cls_model.to_string_lossy().contains("cls"));
        assert!(paths.rec_model.to_string_lossy().contains("rec"));
    }

    #[test]
    fn test_clear_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join("paddle_cache");
        let manager = ModelManager::new(cache_dir.clone());

        // Create some dummy files
        fs::create_dir_all(manager.model_path("det")).unwrap();
        fs::write(manager.model_path("det").join("test.txt"), "test content").unwrap();

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
        fs::write(det_path.join("model.bin"), "x".repeat(1000)).unwrap();

        let cls_path = manager.model_path("cls");
        fs::create_dir_all(&cls_path).unwrap();
        fs::write(cls_path.join("model.bin"), "y".repeat(2000)).unwrap();

        let stats = manager.cache_stats().unwrap();

        // Should have at least 3000 bytes (1000 + 2000)
        assert!(stats.total_size_bytes >= 3000);
        assert_eq!(stats.model_count, 2);
    }

    #[test]
    fn test_model_paths_struct_cloneable() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

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
}
