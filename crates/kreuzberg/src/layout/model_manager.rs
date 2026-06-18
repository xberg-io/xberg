//! Model downloading and caching for layout detection.
//!
//! Downloads ONNX models from HuggingFace Hub and caches them locally.
//! Uses shared download/checksum utilities from [`crate::model_download`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::layout::error::LayoutError;
use crate::model_download;

/// Monotonic counter giving each `atomic_publish` call a unique temp filename so concurrent
/// publishes of the same model never collide on the staging path.
static PUBLISH_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Publish `src` to `dst` atomically.
///
/// Copies `src` into a per-call temp file inside `dst_dir` (same filesystem as `dst`),
/// re-verifies the staged copy's checksum, then renames it into place. The rename is atomic,
/// so concurrent callers — e.g. GLM-OCR paired mode rasterizing pages in parallel, each
/// constructing its own [`LayoutModelManager`] — each stage to a private temp and atomically
/// swap it in. Every observer sees either the old absent file or a complete, checksum-valid
/// model, never a torn copy from interleaved writes to a shared destination.
fn atomic_publish(src: &Path, dst: &Path, dst_dir: &Path, sha256: &str, label: &str) -> Result<(), String> {
    let stem = dst.file_name().and_then(|n| n.to_str()).unwrap_or("model");
    let tmp = dst_dir.join(format!(
        ".{stem}.{}.{}.tmp",
        std::process::id(),
        PUBLISH_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
    ));

    fs::copy(src, &tmp).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("Failed to stage model at {}: {e}", tmp.display())
    })?;

    // Re-verify the staged copy so a torn/partial copy is caught before it is published.
    if let Err(e) = model_download::verify_sha256(&tmp, sha256, label) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }

    fs::rename(&tmp, dst).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("Failed to publish model to {}: {e}", dst.display())
    })
}

#[cfg(feature = "paddle-ocr")]
use crate::paddle_ocr::ModelManifestEntry;

#[cfg(not(feature = "paddle-ocr"))]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelManifestEntry {
    pub relative_path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub source_url: String,
}

/// Model definition for a layout model.
#[derive(Debug, Clone)]
struct ModelDefinition {
    model_type: &'static str,
    hf_repo_id: &'static str,
    remote_filename: &'static str,
    local_filename: &'static str,
    sha256_checksum: &'static str,
    size_bytes: u64,
}

const MODELS: &[ModelDefinition] = &[
    ModelDefinition {
        model_type: "rtdetr",
        hf_repo_id: "Kreuzberg/layout-models",
        remote_filename: "rtdetr/model.onnx",
        local_filename: "model.onnx",
        sha256_checksum: "3bf2fb0ee6df87435b7ae47f0f3930ec3dc97ec56fd824acc6d57bc7a6b89ef2",
        size_bytes: 168_839_883,
    },
    ModelDefinition {
        model_type: "tatr",
        hf_repo_id: "Kreuzberg/layout-models",
        remote_filename: "tatr/model.onnx",
        local_filename: "tatr.onnx",
        sha256_checksum: "c11f4033da75e9c4d41c403ef356e89caa0a37a7d111b55461e7d5ba856bb6b6",
        size_bytes: 30_158_413,
    },
    ModelDefinition {
        model_type: "slanet_wired",
        hf_repo_id: "Kreuzberg/paddleocr-onnx-models",
        remote_filename: "v2/table/SLANeXt_wired.onnx",
        local_filename: "slanet_wired.onnx",
        sha256_checksum: "64990fa026a7e2e2c2d4ad2c810bc9c6992da76d5f91b54771dfc900927ca3d0",
        size_bytes: 365_355_622,
    },
    ModelDefinition {
        model_type: "slanet_wireless",
        hf_repo_id: "Kreuzberg/paddleocr-onnx-models",
        remote_filename: "v2/table/SLANeXt_wireless.onnx",
        local_filename: "slanet_wireless.onnx",
        sha256_checksum: "b29ae2b4fe0ff8bbf7efd73fda0951227eb1abaedcaa046ad016191c779b7766",
        size_bytes: 365_355_622,
    },
    ModelDefinition {
        model_type: "slanet_plus",
        hf_repo_id: "Kreuzberg/paddleocr-onnx-models",
        remote_filename: "v2/table/SLANet_plus.onnx",
        local_filename: "slanet_plus.onnx",
        sha256_checksum: "e48a401a4ebcddd47fe3822427db24d867a557324f58e438692f588bbe9231de",
        size_bytes: 7_781_309,
    },
    ModelDefinition {
        model_type: "table_classifier",
        hf_repo_id: "Kreuzberg/paddleocr-onnx-models",
        remote_filename: "v2/classifiers/PP-LCNet_x1_0_table_cls.onnx",
        local_filename: "table_cls.onnx",
        sha256_checksum: "f02bf087e924dadfb109e3b7887d7d56dc961b80e08c64cacf1030f97345b3c3",
        size_bytes: 6_775_213,
    },
    ModelDefinition {
        model_type: "pp_doclayout_v3",
        // ONNX export of PaddlePaddle/PP-DocLayoutV3 (PaddleDetection DETR), produced by the
        // kreuzberg-dev/paddle-to-onnx pipeline (opset 17) and mirrored in the Kreuzberg HF
        // layout-models repo alongside rtdetr/tatr. Upstream PaddlePaddle/PP-DocLayoutV3 ships
        // only Paddle-native weights (no ONNX), so it cannot be downloaded directly.
        hf_repo_id: "Kreuzberg/layout-models",
        remote_filename: "pp_doclayout_v3/model.onnx",
        local_filename: "pp_doclayout_v3.onnx",
        sha256_checksum: "93d1197e55f1c9cb6720275a89684e7ea61cd5830008a837d8c51b19d47926c1",
        size_bytes: 131_731_131,
    },
];

/// Manages layout model downloading, caching, and path resolution.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct LayoutModelManager {
    cache_dir: PathBuf,
}

impl LayoutModelManager {
    /// Creates a new model manager.
    ///
    /// If `cache_dir` is None, uses the default cache directory:
    /// 1. `KREUZBERG_CACHE_DIR` env var + `/layout`
    /// 2. `.kreuzberg/layout/` in current directory
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        let cache_dir = cache_dir.unwrap_or_else(|| model_download::resolve_cache_dir("layout"));
        Self { cache_dir }
    }

    /// Ensure the RT-DETR model (Docling Heron) exists locally, downloading if needed.
    pub fn ensure_rtdetr_model(&self) -> Result<PathBuf, LayoutError> {
        self.ensure_model("rtdetr")
    }

    fn ensure_model(&self, model_type: &str) -> Result<PathBuf, LayoutError> {
        let definition = MODELS
            .iter()
            .find(|m| m.model_type == model_type)
            .ok_or_else(|| LayoutError::ModelDownload(format!("Unknown model type: {model_type}")))?;

        let model_dir = self.cache_dir.join(model_type);
        let model_file = model_dir.join(definition.local_filename);

        if model_file.exists() {
            tracing::debug!(model_type, "Layout model found in cache");
            return Ok(model_file);
        }

        tracing::info!(
            model_type,
            repo = definition.hf_repo_id,
            "Downloading layout model from HuggingFace..."
        );
        fs::create_dir_all(&model_dir).map_err(|e| {
            LayoutError::ModelDownload(format!("Failed to create cache dir {}: {e}", model_dir.display()))
        })?;

        let cached_path = model_download::hf_download(definition.hf_repo_id, definition.remote_filename)
            .map_err(LayoutError::ModelDownload)?;

        model_download::verify_sha256(&cached_path, definition.sha256_checksum, model_type)
            .map_err(LayoutError::ModelDownload)?;

        atomic_publish(
            &cached_path,
            &model_file,
            &model_dir,
            definition.sha256_checksum,
            model_type,
        )
        .map_err(LayoutError::ModelDownload)?;

        tracing::info!(path = %model_file.display(), model_type, "Layout model saved to cache");
        Ok(model_file)
    }

    /// Check if the RT-DETR model is cached.
    pub fn is_rtdetr_cached(&self) -> bool {
        self.cache_dir.join("rtdetr").join("model.onnx").exists()
    }

    /// Ensure the TATR table structure recognition model exists locally, downloading if needed.
    pub fn ensure_tatr_model(&self) -> Result<PathBuf, LayoutError> {
        self.ensure_model("tatr")
    }

    /// Check if the TATR model is cached.
    pub fn is_tatr_cached(&self) -> bool {
        self.cache_dir.join("tatr").join("tatr.onnx").exists()
    }

    /// Ensure a SLANeXT table structure model exists locally, downloading if needed.
    ///
    /// `variant` must be one of: `"slanet_wired"`, `"slanet_wireless"`, `"slanet_plus"`.
    pub fn ensure_slanet_model(&self, variant: &str) -> Result<PathBuf, LayoutError> {
        self.ensure_model(variant)
    }

    /// Ensure the table classifier model exists locally, downloading if needed.
    pub fn ensure_table_classifier(&self) -> Result<PathBuf, LayoutError> {
        self.ensure_model("table_classifier")
    }

    /// Ensure the PP-DocLayout-V3 layout detection model exists locally, downloading if needed.
    pub fn ensure_pp_doclayout_v3_model(&self) -> Result<PathBuf, LayoutError> {
        self.ensure_model("pp_doclayout_v3")
    }

    /// Check if the PP-DocLayout-V3 model is cached.
    pub fn is_pp_doclayout_v3_cached(&self) -> bool {
        self.cache_dir
            .join("pp_doclayout_v3")
            .join("pp_doclayout_v3.onnx")
            .exists()
    }

    /// Get the cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Returns the manifest of all layout model files with checksums and sizes.
    ///
    /// Paths are relative to the cache root (prefixed with "layout/").
    pub fn manifest() -> Vec<ModelManifestEntry> {
        MODELS
            .iter()
            .map(|model| ModelManifestEntry {
                relative_path: format!("layout/{}/{}", model.model_type, model.local_filename),
                sha256: model.sha256_checksum.to_string(),
                size_bytes: model.size_bytes,
                source_url: format!(
                    "https://huggingface.co/{}/resolve/main/{}",
                    model.hf_repo_id, model.remote_filename
                ),
            })
            .collect()
    }

    /// Ensures the default layout models (RT-DETR + TATR) are downloaded and cached.
    ///
    /// This downloads only the core models needed for basic layout detection and table
    /// structure recognition. Use [`Self::ensure_all_models`] to also download the larger
    /// SLANeXT variants (~730MB).
    pub fn ensure_default_models(&self) -> Result<(), LayoutError> {
        self.ensure_model("rtdetr")?;
        self.ensure_model("tatr")?;
        tracing::info!("Default layout models (rtdetr, tatr) ready");
        Ok(())
    }

    /// Ensures all layout models are downloaded and cached.
    ///
    /// Downloads RT-DETR, TATR, and all SLANeXT table structure variants (~730MB).
    /// For a lighter download that omits SLANeXT, use [`Self::ensure_default_models`].
    pub fn ensure_all_models(&self) -> Result<(), LayoutError> {
        for model in MODELS {
            self.ensure_model(model.model_type)?;
        }
        tracing::info!("All layout models ready");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn atomic_publish_yields_complete_file_under_concurrency() {
        use std::sync::Arc;

        let dir = TempDir::new().unwrap();
        let dst_dir = dir.path().to_path_buf();
        let src = dst_dir.join("source.bin");
        // ~1 MiB payload so an interleaved/torn copy would be detectable.
        let payload: Vec<u8> = (0..1024 * 1024).map(|i| (i % 251) as u8).collect();
        fs::write(&src, &payload).unwrap();
        let sha = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&payload);
            hex::encode(hasher.finalize())
        };

        let src = Arc::new(src);
        let dst = Arc::new(dst_dir.join("published.onnx"));
        let dst_dir = Arc::new(dst_dir);
        let sha = Arc::new(sha);

        let handles: Vec<_> = (0..16)
            .map(|_| {
                let (src, dst, dst_dir, sha) = (src.clone(), dst.clone(), dst_dir.clone(), sha.clone());
                std::thread::spawn(move || {
                    atomic_publish(&src, &dst, &dst_dir, &sha, "test-model").expect("publish must succeed");
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        // The published file must be a complete, byte-identical copy — never torn.
        assert_eq!(fs::read(&*dst).unwrap(), payload, "published file must equal source");

        // No staging temp files may leak.
        let leftovers: Vec<_> = fs::read_dir(&*dst_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "no .tmp staging files should remain, found {leftovers:?}"
        );
    }

    #[test]
    fn atomic_publish_rejects_checksum_mismatch_and_leaves_no_partial() {
        let dir = TempDir::new().unwrap();
        let dst_dir = dir.path().to_path_buf();
        let src = dst_dir.join("source.bin");
        fs::write(&src, b"some model bytes").unwrap();
        let dst = dst_dir.join("published.onnx");

        let err = atomic_publish(&src, &dst, &dst_dir, &"0".repeat(64), "test-model")
            .expect_err("mismatched checksum must fail");
        assert!(err.contains("Checksum mismatch"), "unexpected error: {err}");
        assert!(!dst.exists(), "no file may be published on checksum failure");

        let leftovers: Vec<_> = fs::read_dir(&dst_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "staging temp must be cleaned up on failure, found {leftovers:?}"
        );
    }

    #[test]
    fn test_layout_model_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LayoutModelManager::new(Some(temp_dir.path().to_path_buf()));
        assert_eq!(manager.cache_dir(), temp_dir.path());
    }

    #[test]
    fn test_is_rtdetr_cached_empty() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LayoutModelManager::new(Some(temp_dir.path().to_path_buf()));
        assert!(!manager.is_rtdetr_cached());
    }

    #[test]
    fn test_is_rtdetr_cached_present() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LayoutModelManager::new(Some(temp_dir.path().to_path_buf()));

        let dir = temp_dir.path().join("rtdetr");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("model.onnx"), "fake").unwrap();

        assert!(manager.is_rtdetr_cached());
    }

    #[test]
    fn test_is_tatr_cached_empty() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LayoutModelManager::new(Some(temp_dir.path().to_path_buf()));
        assert!(!manager.is_tatr_cached());
    }

    #[test]
    fn test_is_tatr_cached_present() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LayoutModelManager::new(Some(temp_dir.path().to_path_buf()));

        let dir = temp_dir.path().join("tatr");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("tatr.onnx"), "fake").unwrap();

        assert!(manager.is_tatr_cached());
    }

    #[test]
    fn test_manifest_returns_all_layout_models() {
        let entries = LayoutModelManager::manifest();
        assert_eq!(entries.len(), 7);

        let paths: Vec<&str> = entries.iter().map(|e| e.relative_path.as_str()).collect();
        assert!(paths.contains(&"layout/rtdetr/model.onnx"));
        assert!(paths.contains(&"layout/tatr/tatr.onnx"));
        assert!(paths.contains(&"layout/slanet_wired/slanet_wired.onnx"));
        assert!(paths.contains(&"layout/slanet_wireless/slanet_wireless.onnx"));
        assert!(paths.contains(&"layout/slanet_plus/slanet_plus.onnx"));
        assert!(paths.contains(&"layout/table_classifier/table_cls.onnx"));
        assert!(paths.contains(&"layout/pp_doclayout_v3/pp_doclayout_v3.onnx"));
    }

    #[test]
    fn test_manifest_entries_have_valid_fields() {
        let entries = LayoutModelManager::manifest();

        for entry in &entries {
            assert!(
                !entry.sha256.is_empty(),
                "SHA256 should not be empty for {}",
                entry.relative_path
            );
            assert!(
                entry.size_bytes > 0,
                "Size should be non-zero for {}",
                entry.relative_path
            );
            assert!(
                entry.source_url.starts_with("https://huggingface.co/"),
                "Source URL should be a HuggingFace URL"
            );
            assert!(
                entry.relative_path.starts_with("layout/"),
                "Paths should be prefixed with layout/"
            );
        }
    }

    #[test]
    fn test_ensure_all_models_with_cache() {
        let temp_dir = TempDir::new().unwrap();
        let manager = LayoutModelManager::new(Some(temp_dir.path().to_path_buf()));

        // Pre-populate all models so ensure_all_models short-circuits without downloading.
        let models_and_files = [
            ("rtdetr", "model.onnx"),
            ("tatr", "tatr.onnx"),
            ("slanet_wired", "slanet_wired.onnx"),
            ("slanet_wireless", "slanet_wireless.onnx"),
            ("slanet_plus", "slanet_plus.onnx"),
            ("table_classifier", "table_cls.onnx"),
            ("pp_doclayout_v3", "pp_doclayout_v3.onnx"),
        ];
        for (model_dir, filename) in &models_and_files {
            let dir = temp_dir.path().join(model_dir);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(filename), "fake").unwrap();
        }

        assert!(manager.ensure_all_models().is_ok());
    }
}
