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
use crate::model_download;

/// HuggingFace repository containing PaddleOCR ONNX models.
const HF_REPO_ID: &str = "Kreuzberg/paddleocr-onnx-models";

/// Shared model definition (detection and classification).
#[derive(Debug, Clone)]
struct SharedModelDefinition {
    model_type: &'static str,
    remote_filename: &'static str,
    local_filename: &'static str,
    sha256_checksum: &'static str,
}

/// Recognition model definition (per script family).
#[derive(Debug, Clone)]
struct RecModelDefinition {
    script_family: &'static str,
    model_sha256: &'static str,
    dict_sha256: &'static str,
}

const SHARED_MODELS: &[SharedModelDefinition] = &[
    SharedModelDefinition {
        model_type: "det",
        remote_filename: "PP-OCRv5_server_det_infer.onnx",
        local_filename: "model.onnx",
        sha256_checksum: "127edf0182bb3d218ad59476377b02ca90296cfb4cc85df55042d671a3e53aeb",
    },
    SharedModelDefinition {
        model_type: "cls",
        remote_filename: "ch_ppocr_mobile_v2.0_cls_infer.onnx",
        local_filename: "model.onnx",
        sha256_checksum: "e47acedf663230f8863ff1ab0e64dd2d82b838fceb5957146dab185a89d6215c",
    },
];

/// Per-script-family recognition models (PP-OCRv5).
///
/// English and Chinese families are handled by v2 unified models.
/// These 9 families use per-script models for scripts not covered by the unified model.
const REC_MODELS: &[RecModelDefinition] = &[
    RecModelDefinition {
        script_family: "latin",
        model_sha256: "614ffc2d6d3902d360fad7f1b0dd455ee45e877069d14c4e51a99dc4ef144409",
        dict_sha256: "6230982f2773c40b10dc12a3346947a1a771f9be03fd891b294a023357378005",
    },
    RecModelDefinition {
        script_family: "korean",
        model_sha256: "322f140154c820fcb83c3d24cfe42c9ec70dd1a1834163306a7338136e4f1eaa",
        dict_sha256: "086835d8f64802da9214d24e7aea3fda477a72d2df4716e9769117ca081059bb",
    },
    RecModelDefinition {
        script_family: "eslav",
        model_sha256: "dc6bf0e855247decce214ba6dae5bc135fa0ad725a5918a7fcfb59fad6c9cdee",
        dict_sha256: "71e693f3f04afcd137ec0ce3bdc6732468f784f7f35168b9850e6ffe628a21c3",
    },
    RecModelDefinition {
        script_family: "thai",
        model_sha256: "2b6e56b1872200349e227574c25aeb0e0f9af9b8356e9ff5f75ac543a535669a",
        dict_sha256: "40708ca7e0b6222320a5ba690201b77a6b39633273e3fd19e209613d18595d59",
    },
    RecModelDefinition {
        script_family: "greek",
        model_sha256: "13373f736dbb229e96945fc41c2573403d91503b0775c7b7294839e0c5f3a7a3",
        dict_sha256: "c361caeae4e2b0e27a453390d65ca27be64fa04d4a6eddd79d91a8a6053141de",
    },
    RecModelDefinition {
        script_family: "arabic",
        model_sha256: "5b62055fc6209fa3bb247a9a2a7a9d5100c30868bad8a2fa49ed062f64b83021",
        dict_sha256: "7f92f7dbb9b75a4787a83bfb4f6d14a8ab515525130c9d40a9036f61cf6999e9",
    },
    RecModelDefinition {
        script_family: "devanagari",
        model_sha256: "2e895a63a7e08932c8b7b65d8bdb87f96b6f075a80c329ab98298ea0915ebf85",
        dict_sha256: "09c7440bfc5477e5c41052304b6b185aff8c4a5e8b2b4c23c1c706f6fe1ee9fc",
    },
    RecModelDefinition {
        script_family: "tamil",
        model_sha256: "1d3dd137f72273e13b03ad30c7abc55494d6aa723b441c21122479c0622105e0",
        dict_sha256: "85b541352ae18dc6ba6d47152d8bf8adff6b0266e605d2eef2990c1bf466117b",
    },
    RecModelDefinition {
        script_family: "telugu",
        model_sha256: "9ba6b6cd4f028f4e5eaa7e29c428b5ea52bd399c02844cddc5d412f139cf7793",
        dict_sha256: "42f83f5d3fdb50778e4fa5b66c58d99a59ab7792151c5e74f34b8ffd7b61c9d6",
    },
];

// ============================================================================
// V2 model definitions (tier-aware)
// ============================================================================

/// V2 detection model definition (tier-aware).
#[derive(Debug, Clone)]
struct V2DetModelDefinition {
    tier: &'static str,
    remote_filename: &'static str,
    sha256_checksum: &'static str,
}

/// V2 recognition model definition (unified multilingual models).
#[derive(Debug, Clone)]
struct V2RecModelDefinition {
    /// Engine pool key (e.g. "unified_server", "unified_mobile", "en_mobile").
    model_key: &'static str,
    remote_model: &'static str,
    remote_dict: &'static str,
    model_sha256: &'static str,
    dict_sha256: &'static str,
}

/// V2 detection models: server (PP-OCRv5, 88MB) and mobile (PP-OCRv5, 4.7MB).
const V2_DET_MODELS: &[V2DetModelDefinition] = &[
    V2DetModelDefinition {
        tier: "server",
        remote_filename: "v2/det/server.onnx",
        sha256_checksum: "d5f46afc7a2b7fe5773c4ce6ff05c9e23631eb5de0f59d7a90404d9c49678f3c",
    },
    V2DetModelDefinition {
        tier: "mobile",
        remote_filename: "v2/det/mobile.onnx",
        sha256_checksum: "c8d9b07063420ce5365c74e42532de48238feeeedcdb7a330b195708bc38a93f",
    },
];

/// V2 recognition models: unified server/mobile (CJK+English) and English-only mobile.
///
/// Note: `en_mobile` is kept for backward compatibility (direct `ensure_v2_rec_model("en_mobile")`
/// callers) but is not used by the default resolution matrix — both English and Chinese mobile
/// resolve to `unified_mobile`.
const V2_REC_MODELS: &[V2RecModelDefinition] = &[
    V2RecModelDefinition {
        model_key: "unified_server",
        remote_model: "v2/rec/unified_server/model.onnx",
        remote_dict: "v2/rec/unified_server/dict.txt",
        model_sha256: "00667becb28bcd49dfbcb8c7724aa8d6e8f01a1444db66e404182431e0fcbc14",
        dict_sha256: "74f75c9f414da39d503635e76c6871baf8ab8df3b5a47072d55b9344483086c9",
    },
    V2RecModelDefinition {
        model_key: "unified_mobile",
        remote_model: "v2/rec/unified_mobile/model.onnx",
        remote_dict: "v2/rec/unified_mobile/dict.txt",
        model_sha256: "bcb195e3463eb9e46ef419b8a01ea4729577de5fd63c64f0a762e43bd64256e7",
        dict_sha256: "74f75c9f414da39d503635e76c6871baf8ab8df3b5a47072d55b9344483086c9",
    },
    V2RecModelDefinition {
        model_key: "en_mobile",
        remote_model: "v2/rec/en_mobile/model.onnx",
        remote_dict: "v2/rec/en_mobile/dict.txt",
        model_sha256: "70b2450eed39599af6b996c27a2f1a0ef30eeb49f9f66dd3e74f28f652befc89",
        dict_sha256: "854c6bb3e5a9a8ceac81fa700927e86a8da0e9b329a2846c57fc686be9db93e5",
    },
];

/// V2 text line orientation model (PP-LCNet, replaces old PPOCRv2 angle classifier).
const V2_CLS_MODEL: SharedModelDefinition = SharedModelDefinition {
    model_type: "cls",
    remote_filename: "v2/classifiers/PP-LCNet_x1_0_textline_ori.onnx",
    local_filename: "model.onnx",
    sha256_checksum: "1090f9f483a115f904beefe04acc9d28edf0c0b7b08cf0dd8d0ea59a9e0f2735",
};

/// V2 document orientation model (PP-LCNet, for page-level auto_rotate).
const V2_DOC_ORI_MODEL: SharedModelDefinition = SharedModelDefinition {
    model_type: "doc_ori",
    remote_filename: "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx",
    local_filename: "model.onnx",
    sha256_checksum: "6b742aebce6f0f7f71f747931ac7becfc7c96c51641e14943b291eeb334e7947",
};

/// Resolved recognition model with engine pool key for sharing.
#[derive(Debug, Clone)]
pub struct ResolvedRecModel {
    /// Directory containing model.onnx.
    pub model_dir: PathBuf,
    /// Path to the character dictionary file.
    pub dict_file: PathBuf,
    /// Engine pool key for sharing engines across script families.
    /// Multiple families may share the same key (e.g. chinese and japanese
    /// both map to "v2:unified_server" when using server tier).
    pub model_key: String,
}

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

/// A single model file entry in the cache manifest.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelManifestEntry {
    /// Relative path within the cache directory (e.g., "paddle-ocr/det/model.onnx").
    pub relative_path: String,
    /// SHA256 checksum of the model file.
    pub sha256: String,
    /// Expected file size in bytes.
    pub size_bytes: u64,
    /// HuggingFace source URL for downloading.
    pub source_url: String,
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
    #[cfg(test)]
    pub(crate) fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Ensures a recognition model for the given script family exists locally.
    ///
    /// Downloads the model and character dictionary from HuggingFace if not cached.
    ///
    /// # Arguments
    ///
    /// * `family` - Script family name (e.g., "english", "chinese", "latin")
    pub(crate) fn ensure_rec_model(&self, family: &str) -> Result<RecModelPaths, KreuzbergError> {
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
    pub(crate) fn ensure_models_exist(&self) -> Result<ModelPaths, KreuzbergError> {
        let shared = self.ensure_shared_models("server")?;
        let rec = self.resolve_rec_model("english", "server")?;

        tracing::info!("All PaddleOCR models ready (english)");

        Ok(ModelPaths {
            det_model: shared.det_model,
            cls_model: shared.cls_model,
            rec_model: rec.model_dir,
            dict_file: rec.dict_file,
        })
    }

    /// Find the recognition model definition for a script family.
    fn find_rec_definition(family: &str) -> Option<&'static RecModelDefinition> {
        REC_MODELS.iter().find(|d| d.script_family == family)
    }

    /// Returns the path for a model type directory (det, cls).
    #[cfg(test)]
    pub(crate) fn model_path(&self, model_type: &str) -> PathBuf {
        self.cache_dir.join(model_type)
    }

    /// Returns the path for a recognition family directory.
    fn rec_family_path(&self, family: &str) -> PathBuf {
        self.cache_dir.join("rec").join(family)
    }

    /// Returns the full path to the ONNX model file for a given type.
    #[cfg(test)]
    fn model_file_path(&self, model_type: &str) -> PathBuf {
        self.model_path(model_type).join("model.onnx")
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
        model_download::hf_download(HF_REPO_ID, remote_filename).map_err(|e| KreuzbergError::Plugin {
            message: e,
            plugin_name: "paddle-ocr".to_string(),
        })
    }

    /// Verify SHA256 checksum of a downloaded file.
    fn verify_checksum(path: &Path, expected: &str, label: &str) -> Result<(), KreuzbergError> {
        model_download::verify_sha256(path, expected, label).map_err(|e| KreuzbergError::Validation {
            message: e,
            source: None,
        })
    }

    /// Checks if shared models (det + cls) are cached locally.
    #[cfg(test)]
    pub(crate) fn are_shared_models_cached(&self) -> bool {
        SHARED_MODELS.iter().all(|model| {
            let f = self.model_file_path(model.model_type);
            f.exists() && f.is_file()
        })
    }

    /// Checks if a recognition model for the given family is cached.
    #[cfg(test)]
    pub(crate) fn is_rec_model_cached(&self, family: &str) -> bool {
        let rec_dir = self.rec_family_path(family);
        rec_dir.join("model.onnx").exists() && rec_dir.join("dict.txt").exists()
    }

    /// Checks if all required models are cached (shared + English v2 rec).
    #[cfg(test)]
    pub(crate) fn are_models_cached(&self) -> bool {
        let v2_rec_dir = self.cache_dir.join("v2").join("rec").join("unified_server");
        self.are_shared_models_cached()
            && v2_rec_dir.join("model.onnx").exists()
            && v2_rec_dir.join("dict.txt").exists()
    }

    /// Clears all cached models from the cache directory.
    #[cfg(test)]
    pub(crate) fn clear_cache(&self) -> Result<(), KreuzbergError> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            tracing::info!(?self.cache_dir, "Cache directory cleared");
        }
        Ok(())
    }

    /// Returns statistics about the current cache.
    #[cfg(test)]
    pub(crate) fn cache_stats(&self) -> Result<CacheStats, KreuzbergError> {
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

    /// Returns the manifest of all PaddleOCR model files with checksums and sizes.
    ///
    /// This includes shared models (det, cls) and all 9 per-script recognition model families.
    /// Paths are relative to the cache root (prefixed with "paddle-ocr/").
    pub fn manifest() -> Vec<ModelManifestEntry> {
        let mut entries = Vec::new();

        for model in SHARED_MODELS {
            entries.push(ModelManifestEntry {
                relative_path: format!("paddle-ocr/{}/{}", model.model_type, model.local_filename),
                sha256: model.sha256_checksum.to_string(),
                size_bytes: 0,
                source_url: format!(
                    "https://huggingface.co/{}/resolve/main/{}",
                    HF_REPO_ID, model.remote_filename
                ),
            });
        }

        for rec in REC_MODELS {
            entries.push(ModelManifestEntry {
                relative_path: format!("paddle-ocr/rec/{}/model.onnx", rec.script_family),
                sha256: rec.model_sha256.to_string(),
                size_bytes: 0,
                source_url: format!(
                    "https://huggingface.co/{}/resolve/main/rec/{}/model.onnx",
                    HF_REPO_ID, rec.script_family
                ),
            });
            // Dict files don't have size_bytes tracked, use 0 as placeholder
            entries.push(ModelManifestEntry {
                relative_path: format!("paddle-ocr/rec/{}/dict.txt", rec.script_family),
                sha256: rec.dict_sha256.to_string(),
                size_bytes: 0,
                source_url: format!(
                    "https://huggingface.co/{}/resolve/main/rec/{}/dict.txt",
                    HF_REPO_ID, rec.script_family
                ),
            });
        }

        entries
    }

    /// Ensures all v2 models are downloaded and cached.
    ///
    /// Downloads:
    /// - Both detection tiers (server + mobile)
    /// - Classification model (PP-LCNet textline_ori)
    /// - Document orientation model (PP-LCNet doc_ori)
    /// - All v2 unified rec models (server, mobile, en_mobile)
    /// - All per-script rec models for uncovered scripts
    pub fn ensure_all_models(&self) -> Result<(), KreuzbergError> {
        // V2 shared models (both tiers)
        self.ensure_shared_models("server")?;
        self.ensure_v2_det_model("mobile")?; // cls is same for both tiers

        // Document orientation model
        self.ensure_doc_ori_model()?;

        // V2 unified rec models
        for v2_rec in V2_REC_MODELS {
            self.ensure_v2_rec_model(v2_rec.model_key)?;
        }

        // Per-script rec models for uncovered scripts
        for rec in REC_MODELS {
            self.ensure_rec_model(rec.script_family)?;
        }

        tracing::info!(
            "All PaddleOCR v2 models ready ({} v2 rec + {} per-script families)",
            V2_REC_MODELS.len(),
            REC_MODELS.len()
        );
        Ok(())
    }

    // ========================================================================
    // V2 tier-aware model resolution
    // ========================================================================

    /// Ensures the v2 detection model for the given tier is cached locally.
    ///
    /// Downloads from HuggingFace if not cached. Returns the path to the
    /// directory containing the ONNX model file.
    pub(crate) fn ensure_v2_det_model(&self, tier: &str) -> Result<PathBuf, KreuzbergError> {
        let definition = V2_DET_MODELS
            .iter()
            .find(|d| d.tier == tier)
            .ok_or_else(|| KreuzbergError::Plugin {
                message: format!("Invalid model_tier \"{tier}\". Valid values: \"server\", \"mobile\""),
                plugin_name: "paddle-ocr".to_string(),
            })?;

        let det_dir = self.cache_dir.join("v2").join("det").join(tier);
        let model_file = det_dir.join("model.onnx");

        if !model_file.exists() {
            tracing::info!(tier, "Downloading v2 detection model...");
            fs::create_dir_all(&det_dir)?;
            let cached_path = self.hf_download(definition.remote_filename)?;
            Self::verify_checksum(&cached_path, definition.sha256_checksum, &format!("v2/det/{tier}"))?;
            fs::copy(&cached_path, &model_file).map_err(|e| KreuzbergError::Plugin {
                message: format!("Failed to copy v2 det model: {e}"),
                plugin_name: "paddle-ocr".to_string(),
            })?;
            tracing::info!(tier, "V2 detection model saved");
        }

        Ok(det_dir)
    }

    /// Ensures the v2 classification model is cached locally.
    ///
    /// The cls model is the same for both tiers.
    pub(crate) fn ensure_v2_cls_model(&self) -> Result<PathBuf, KreuzbergError> {
        let cls_dir = self.cache_dir.join("v2").join("cls");
        let model_file = cls_dir.join("model.onnx");

        if !model_file.exists() {
            tracing::info!("Downloading v2 classification model...");
            fs::create_dir_all(&cls_dir)?;
            let cached_path = self.hf_download(V2_CLS_MODEL.remote_filename)?;
            Self::verify_checksum(&cached_path, V2_CLS_MODEL.sha256_checksum, "v2/cls")?;
            fs::copy(&cached_path, &model_file).map_err(|e| KreuzbergError::Plugin {
                message: format!("Failed to copy v2 cls model: {e}"),
                plugin_name: "paddle-ocr".to_string(),
            })?;
            tracing::info!("V2 classification model saved");
        }

        Ok(cls_dir)
    }

    /// Ensures the v2 document orientation model is cached locally.
    ///
    /// Used for page-level auto_rotate when PaddleOCR backend is active.
    pub(crate) fn ensure_doc_ori_model(&self) -> Result<PathBuf, KreuzbergError> {
        let ori_dir = self.cache_dir.join("v2").join("doc_ori");
        let model_file = ori_dir.join("model.onnx");

        if !model_file.exists() {
            tracing::info!("Downloading v2 document orientation model...");
            fs::create_dir_all(&ori_dir)?;
            let cached_path = self.hf_download(V2_DOC_ORI_MODEL.remote_filename)?;
            Self::verify_checksum(&cached_path, V2_DOC_ORI_MODEL.sha256_checksum, "v2/doc_ori")?;
            fs::copy(&cached_path, &model_file).map_err(|e| KreuzbergError::Plugin {
                message: format!("Failed to copy v2 doc_ori model: {e}"),
                plugin_name: "paddle-ocr".to_string(),
            })?;
            tracing::info!("V2 document orientation model saved");
        }

        Ok(ori_dir)
    }

    /// Ensures shared models (det + cls) are cached for the given tier.
    pub(crate) fn ensure_shared_models(&self, tier: &str) -> Result<SharedModelPaths, KreuzbergError> {
        let det_model = self.ensure_v2_det_model(tier)?;
        let cls_model = self.ensure_v2_cls_model()?;
        Ok(SharedModelPaths { det_model, cls_model })
    }

    /// Resolves the recognition model for a script family and tier.
    ///
    /// Returns the model directory, dict file path, and a model key for
    /// engine pool sharing. Multiple families may share the same model key
    /// (e.g. chinese and japanese both use "v2:unified_server").
    ///
    /// # Selection matrix
    ///
    /// | Family | Server | Mobile |
    /// |---|---|---|
    /// | english | v2 unified_server (84MB) | v2 unified_mobile (16.5MB) |
    /// | chinese (ch, jpn, chinese_cht) | v2 unified_server (84MB) | v2 unified_mobile (16.5MB) |
    /// | all others | per-script (unchanged) | per-script (unchanged) |
    pub(crate) fn resolve_rec_model(&self, family: &str, tier: &str) -> Result<ResolvedRecModel, KreuzbergError> {
        match (family, tier) {
            // English + Chinese families use v2 unified models
            ("english", "server") | ("chinese", "server") => self.ensure_v2_rec_model("unified_server"),
            // Both English and Chinese mobile use unified_mobile (CJK+English in one model)
            ("english", "mobile") | ("chinese", "mobile") => self.ensure_v2_rec_model("unified_mobile"),

            // All other scripts: per-script models (no tier distinction)
            _ => {
                let rec_paths = self.ensure_rec_model(family)?;
                Ok(ResolvedRecModel {
                    model_dir: rec_paths.rec_model,
                    dict_file: rec_paths.dict_file,
                    model_key: format!("v1:{family}"),
                })
            }
        }
    }

    /// Ensures a v2 recognition model is cached and returns resolved paths.
    fn ensure_v2_rec_model(&self, model_key: &str) -> Result<ResolvedRecModel, KreuzbergError> {
        let definition =
            V2_REC_MODELS
                .iter()
                .find(|d| d.model_key == model_key)
                .ok_or_else(|| KreuzbergError::Plugin {
                    message: format!("Unknown v2 rec model key: {model_key}"),
                    plugin_name: "paddle-ocr".to_string(),
                })?;

        let rec_dir = self.cache_dir.join("v2").join("rec").join(model_key);
        let model_file = rec_dir.join("model.onnx");
        let dict_file = rec_dir.join("dict.txt");

        if !model_file.exists() || !dict_file.exists() {
            tracing::info!(model_key, "Downloading v2 recognition model...");
            fs::create_dir_all(&rec_dir)?;

            // Download model
            let cached_model = self.hf_download(definition.remote_model)?;
            Self::verify_checksum(&cached_model, definition.model_sha256, &format!("v2/rec/{model_key}"))?;
            fs::copy(&cached_model, &model_file).map_err(|e| KreuzbergError::Plugin {
                message: format!("Failed to copy v2 rec model: {e}"),
                plugin_name: "paddle-ocr".to_string(),
            })?;

            // Download dict
            let cached_dict = self.hf_download(definition.remote_dict)?;
            Self::verify_checksum(
                &cached_dict,
                definition.dict_sha256,
                &format!("v2/rec/{model_key}/dict"),
            )?;
            fs::copy(&cached_dict, &dict_file).map_err(|e| KreuzbergError::Plugin {
                message: format!("Failed to copy v2 rec dict: {e}"),
                plugin_name: "paddle-ocr".to_string(),
            })?;

            tracing::info!(model_key, "V2 recognition model and dict saved");
        }

        Ok(ResolvedRecModel {
            model_dir: rec_dir,
            dict_file,
            model_key: format!("v2:{model_key}"),
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
            "latin",
            "korean",
            "eslav",
            "thai",
            "greek",
            "arabic",
            "devanagari",
            "tamil",
            "telugu",
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

        // Add v2 unified_server rec (used for english)
        let rec_dir = manager.cache_dir().join("v2").join("rec").join("unified_server");
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
        assert_eq!(REC_MODELS.len(), 9);
        let families: Vec<_> = REC_MODELS.iter().map(|m| m.script_family).collect();
        assert!(!families.contains(&"english"));
        assert!(!families.contains(&"chinese"));
        assert!(families.contains(&"latin"));
        assert!(families.contains(&"korean"));
        assert!(families.contains(&"eslav"));
        assert!(families.contains(&"thai"));
        assert!(families.contains(&"greek"));
        assert!(families.contains(&"arabic"));
        assert!(families.contains(&"devanagari"));
        assert!(families.contains(&"tamil"));
        assert!(families.contains(&"telugu"));
    }

    #[test]
    fn test_model_paths_cloneable() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        // Pre-populate cache so ensure_models_exist doesn't try to download
        // v2 shared models (det server, cls)
        let det_dir = temp_dir.path().join("v2").join("det").join("server");
        fs::create_dir_all(&det_dir).unwrap();
        fs::write(det_dir.join("model.onnx"), "fake").unwrap();
        let cls_dir = temp_dir.path().join("v2").join("cls");
        fs::create_dir_all(&cls_dir).unwrap();
        fs::write(cls_dir.join("model.onnx"), "fake").unwrap();
        // v2 unified_server rec model (used by ensure_models_exist for english)
        let rec_dir = temp_dir.path().join("v2").join("rec").join("unified_server");
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

        // Pre-populate v2 shared model paths
        let det_dir = temp_dir.path().join("v2").join("det").join("server");
        fs::create_dir_all(&det_dir).unwrap();
        fs::write(det_dir.join("model.onnx"), "fake").unwrap();
        let cls_dir = temp_dir.path().join("v2").join("cls");
        fs::create_dir_all(&cls_dir).unwrap();
        fs::write(cls_dir.join("model.onnx"), "fake").unwrap();

        let paths = manager.ensure_shared_models("server").unwrap();
        assert!(paths.det_model.ends_with("server"));
        assert!(paths.cls_model.ends_with("cls"));
    }

    #[test]
    fn test_ensure_rec_model_with_cache() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        let rec_dir = manager.rec_family_path("latin");
        fs::create_dir_all(&rec_dir).unwrap();
        fs::write(rec_dir.join("model.onnx"), "fake").unwrap();
        fs::write(rec_dir.join("dict.txt"), "#\na\n ").unwrap();

        let paths = manager.ensure_rec_model("latin").unwrap();
        assert!(paths.rec_model.ends_with("rec/latin"));
        assert!(paths.dict_file.ends_with("rec/latin/dict.txt"));
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

    #[test]
    fn test_manifest_returns_all_models() {
        let entries = ModelManager::manifest();

        // 2 shared (det, cls) + 9 rec families * 2 (model + dict) = 20
        assert_eq!(entries.len(), 2 + 9 * 2);

        // Check shared models present
        let paths: Vec<&str> = entries.iter().map(|e| e.relative_path.as_str()).collect();
        assert!(paths.contains(&"paddle-ocr/det/model.onnx"));
        assert!(paths.contains(&"paddle-ocr/cls/model.onnx"));

        // English and Chinese should NOT be in per-script manifest
        assert!(!paths.contains(&"paddle-ocr/rec/english/model.onnx"));
        assert!(!paths.contains(&"paddle-ocr/rec/chinese/model.onnx"));

        // Check all per-script rec families present
        for family in &[
            "latin",
            "korean",
            "eslav",
            "thai",
            "greek",
            "arabic",
            "devanagari",
            "tamil",
            "telugu",
        ] {
            let model_path = format!("paddle-ocr/rec/{family}/model.onnx");
            let dict_path = format!("paddle-ocr/rec/{family}/dict.txt");
            assert!(paths.contains(&model_path.as_str()), "Missing model for {family}");
            assert!(paths.contains(&dict_path.as_str()), "Missing dict for {family}");
        }
    }

    #[test]
    fn test_manifest_entries_have_valid_fields() {
        let entries = ModelManager::manifest();

        for entry in &entries {
            assert!(
                !entry.sha256.is_empty(),
                "SHA256 should not be empty for {}",
                entry.relative_path
            );
            assert!(
                entry.source_url.starts_with("https://huggingface.co/"),
                "Source URL should be a HuggingFace URL for {}",
                entry.relative_path
            );
            assert!(
                entry.relative_path.starts_with("paddle-ocr/"),
                "Paths should be prefixed with paddle-ocr/"
            );
        }
    }

    #[test]
    fn test_manifest_entry_serialization() {
        let entry = ModelManifestEntry {
            relative_path: "test/model.onnx".to_string(),
            sha256: "abc123".to_string(),
            size_bytes: 1024,
            source_url: "https://example.com/model.onnx".to_string(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("test/model.onnx"));
        assert!(json.contains("abc123"));
        assert!(json.contains("1024"));
    }

    #[test]
    fn test_ensure_all_models_with_cache() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf());

        let v2_dir = temp_dir.path().join("v2");

        // Pre-populate v2 det models (server + mobile)
        for tier in &["server", "mobile"] {
            let dir = v2_dir.join("det").join(tier);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("model.onnx"), "fake").unwrap();
        }

        // Pre-populate v2 cls model
        let cls_dir = v2_dir.join("cls");
        fs::create_dir_all(&cls_dir).unwrap();
        fs::write(cls_dir.join("model.onnx"), "fake").unwrap();

        // Pre-populate v2 doc_ori model
        let doc_ori_dir = v2_dir.join("doc_ori");
        fs::create_dir_all(&doc_ori_dir).unwrap();
        fs::write(doc_ori_dir.join("model.onnx"), "fake").unwrap();

        // Pre-populate v2 rec models (unified_server, unified_mobile, en_mobile)
        for model_key in &["unified_server", "unified_mobile", "en_mobile"] {
            let dir = v2_dir.join("rec").join(model_key);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("model.onnx"), "fake").unwrap();
            fs::write(dir.join("dict.txt"), "#\na\n ").unwrap();
        }

        // Pre-populate per-script rec families (9 families, no english/chinese)
        for family in &[
            "latin",
            "korean",
            "eslav",
            "thai",
            "greek",
            "arabic",
            "devanagari",
            "tamil",
            "telugu",
        ] {
            let rec_dir = manager.rec_family_path(family);
            fs::create_dir_all(&rec_dir).unwrap();
            fs::write(rec_dir.join("model.onnx"), "fake").unwrap();
            fs::write(rec_dir.join("dict.txt"), "#\na\n ").unwrap();
        }

        // Should succeed without downloading
        assert!(manager.ensure_all_models().is_ok());
    }
}
