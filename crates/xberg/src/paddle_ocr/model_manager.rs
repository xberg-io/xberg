/// Model downloading and caching for PaddleOCR.
///
/// This module resolves PaddleOCR model artifacts directly from the standard Hugging Face
/// cache. Models are organized into shared models (detection, classification) and
/// recognition models selected by script family.
///
/// # Model Download Flow
///
/// 1. Resolve the immutable repository revision from the local Hub cache.
/// 2. Download on a cache miss unless Hugging Face offline mode is enabled.
/// 3. Verify SHA-256 on every warm or cold resolution and repair corrupt entries.
/// 4. Return the snapshot artifact path directly, without an Xberg-owned copy.
#[cfg(paddle_ocr)]
use std::fs;
#[cfg(paddle_ocr)]
use std::io::Read;
#[cfg(all(test, paddle_ocr))]
use std::path::Path;
use std::path::PathBuf;

#[cfg(paddle_ocr)]
use crate::error::XbergError;
#[cfg(paddle_ocr)]
use crate::model_download;

/// HuggingFace repository containing PaddleOCR ONNX models.
#[cfg(paddle_ocr)]
const HF_REPO_ID: &str = "xberg-io/paddleocr-onnx-models";
/// Immutable Hub revision containing the checksummed PaddleOCR model set.
#[cfg(paddle_ocr)]
pub(crate) const HF_REPO_REVISION: &str = "bc5ec866cf0e798e667808dfa51b0ba8ad0dafc8";

/// Shared model definition (detection and classification).
#[cfg(paddle_ocr)]
#[derive(Debug, Clone)]
struct SharedModelDefinition {
    remote_filename: &'static str,
    sha256_checksum: &'static str,
}

/// Recognition model definition (per script family).
#[cfg(paddle_ocr)]
#[derive(Debug, Clone)]
struct RecModelDefinition {
    script_family: &'static str,
    model_sha256: &'static str,
    dict_sha256: &'static str,
}

/// Per-script-family recognition models (PP-OCRv5).
///
/// English and Chinese families are handled by v2 unified models.
/// These 9 families use per-script models for scripts not covered by the unified model.
#[cfg(paddle_ocr)]
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

/// V2 detection model definition (tier-aware).
#[cfg(paddle_ocr)]
#[derive(Debug, Clone)]
struct V2DetModelDefinition {
    tier: &'static str,
    remote_filename: &'static str,
    sha256_checksum: &'static str,
}

/// V2 recognition model definition (unified multilingual models).
#[cfg(paddle_ocr)]
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
#[cfg(paddle_ocr)]
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
#[cfg(paddle_ocr)]
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
#[cfg(paddle_ocr)]
const V2_CLS_MODEL: SharedModelDefinition = SharedModelDefinition {
    remote_filename: "v2/classifiers/PP-LCNet_x1_0_textline_ori.onnx",
    sha256_checksum: "1090f9f483a115f904beefe04acc9d28edf0c0b7b08cf0dd8d0ea59a9e0f2735",
};

/// V2 document orientation model (PP-LCNet, for page-level auto_rotate).
#[cfg(paddle_ocr)]
const V2_DOC_ORI_MODEL: SharedModelDefinition = SharedModelDefinition {
    remote_filename: "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx",
    sha256_checksum: "6b742aebce6f0f7f71f747931ac7becfc7c96c51641e14943b291eeb334e7947",
};

/// PP-OCRv6 detection model definition (script-agnostic, one per tier).
#[cfg(paddle_ocr)]
#[derive(Debug, Clone)]
struct V6DetModelDefinition {
    tier: &'static str,
    remote_filename: &'static str,
    sha256_checksum: &'static str,
}

/// PP-OCRv6 recognition model definition (unified CJK+Latin+JA/KO, one per tier).
#[cfg(paddle_ocr)]
#[derive(Debug, Clone)]
struct V6RecModelDefinition {
    tier: &'static str,
    remote_model: &'static str,
    remote_dict: &'static str,
    model_sha256: &'static str,
    dict_sha256: &'static str,
}

/// PP-OCRv6 detection models: medium (62MB), small (9.9MB), tiny (1.8MB).
#[cfg(paddle_ocr)]
const V6_DET_MODELS: &[V6DetModelDefinition] = &[
    V6DetModelDefinition {
        tier: "medium",
        remote_filename: "v6/det/medium/model.onnx",
        sha256_checksum: "9d58088cce871cd690deae447f860df699f5db1d4e3ef21cc2a3229497e50ea2",
    },
    V6DetModelDefinition {
        tier: "small",
        remote_filename: "v6/det/small/model.onnx",
        sha256_checksum: "b1a4f07289eda88d29239890b94ea2f9e29f5635a33ff6e165bb1b27dcea25fc",
    },
    V6DetModelDefinition {
        tier: "tiny",
        remote_filename: "v6/det/tiny/model.onnx",
        sha256_checksum: "7603ac05a98aef4f7284517b9210c09f37352debd1183511645e9ee03c5a0406",
    },
];

/// PP-OCRv6 recognition models. medium/small share an 18,708-char CJK+Latin+JA/KO dict
/// (output `[B,T,18710]`); tiny uses a reduced 6,904-char ~zh/en dict (output `[B,T,6906]`).
/// CTC convention matches v5 (blank@0, dict@1..N, trailing space@N+1) — the decoder sizes
/// itself from the dict, so no xberg-side shim is required.
#[cfg(paddle_ocr)]
const V6_REC_MODELS: &[V6RecModelDefinition] = &[
    V6RecModelDefinition {
        tier: "medium",
        remote_model: "v6/rec/medium/model.onnx",
        remote_dict: "v6/rec/medium/dict.txt",
        model_sha256: "b05d174d4f47ebea755aa3b63b85c176b3145b0a1ea1fbcc774f8f4751236b64",
        dict_sha256: "b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d",
    },
    V6RecModelDefinition {
        tier: "small",
        remote_model: "v6/rec/small/model.onnx",
        remote_dict: "v6/rec/small/dict.txt",
        model_sha256: "0cc734c53747ea66968d286f6e6597fc1c91df121964ab4be5085c156fcac892",
        dict_sha256: "b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d",
    },
    V6RecModelDefinition {
        tier: "tiny",
        remote_model: "v6/rec/tiny/model.onnx",
        remote_dict: "v6/rec/tiny/dict.txt",
        model_sha256: "46c3df72875f0969a9a5e0bdc23ce685913f68ab96b9778e760962b0cb1b8ed4",
        dict_sha256: "c5cbe34ef40c29c4df07ed012bf96569cb69a2d2a01a07027e9f13cb832bd9cd",
    },
];

/// Script families covered by the PP-OCRv6 unified recognition model.
///
/// Korean uses its PP-OCRv5 script-specific recognizer because it retains
/// materially better Hangul accuracy than the unified v6 export. ~keep
/// Families outside this set fall back to the PP-OCRv5 per-script recognition models.
#[cfg(paddle_ocr)]
const V6_UNIFIED_FAMILIES: &[&str] = &["english", "chinese", "latin"];

/// Maps a configured tier to an effective PP-OCRv6 tier.
///
/// `mobile` maps to `small`, not `medium`. `mobile` is the v5 name for the lightweight tier
/// (4.7MB detection) and is this config's default, so resolving it to v6 `medium` (62MB) handed
/// every caller who never set `model_tier` the heaviest available model under a name that
/// promises the lightest — the cause of GH#1602, where a 21-page document took over ten minutes.
/// `small` (9.9MB detection) is the nearest v6 analogue and, unlike `tiny`, keeps the full
/// 18,708-char CJK+Latin+JA/KO recognition dictionary, so the remap cannot silently narrow which
/// scripts a caller can read. `server` keeps mapping to `medium`: it is the v5 name for the
/// high-accuracy tier, so the largest v6 model is what it asks for. Unknown values also resolve
/// to `medium` rather than guessing. ~keep
#[cfg(paddle_ocr)]
fn effective_v6_tier(tier: &str) -> &str {
    match tier {
        "medium" | "small" | "tiny" => tier,
        "mobile" => "small",
        _ => "medium",
    }
}
#[cfg_attr(alef, alef(skip))]
/// Resolved recognition model with engine pool key for sharing.
#[derive(Debug, Clone)]
pub struct ResolvedRecModel {
    /// Exact path to the recognition ONNX model in the Hugging Face snapshot.
    pub model_dir: PathBuf,
    /// Path to the character dictionary file.
    pub dict_file: PathBuf,
    /// Engine pool key for sharing engines across script families.
    /// Multiple families may share the same key (e.g. chinese and japanese
    /// both map to "v2:unified_server" when using server tier).
    pub model_key: String,
}

/// Paths to shared models (detection + classification).
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct SharedModelPaths {
    /// Exact path to the detection ONNX model in the Hugging Face snapshot.
    pub det_model: PathBuf,
    /// Exact path to the classification ONNX model in the Hugging Face snapshot.
    pub cls_model: PathBuf,
}

/// Paths to a recognition model and its character dictionary.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct RecModelPaths {
    /// Exact path to the recognition ONNX model in the Hugging Face snapshot.
    pub rec_model: PathBuf,
    /// Path to the character dictionary file.
    pub dict_file: PathBuf,
}

/// Combined paths to all models needed for OCR (backward compatibility).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelPaths {
    /// Exact path to the detection ONNX model in the Hugging Face snapshot.
    pub det_model: PathBuf,
    /// Exact path to the classification ONNX model in the Hugging Face snapshot.
    pub cls_model: PathBuf,
    /// Exact path to the recognition ONNX model in the Hugging Face snapshot.
    pub rec_model: PathBuf,
    /// Path to the character dictionary file.
    pub dict_file: PathBuf,
}
#[cfg_attr(alef, alef(skip))]
/// A single downloadable model entry.
#[cfg(paddle_ocr)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelManifestEntry {
    /// Stable logical path used by manifest consumers; the runtime artifact remains
    /// in Hugging Face's snapshot layout rather than at this path.
    pub relative_path: String,
    /// SHA256 checksum of the model file.
    pub sha256: String,
    /// Expected file size in bytes.
    pub size_bytes: u64,
    /// HuggingFace source URL for downloading.
    pub source_url: String,
}

#[cfg(paddle_ocr)]
#[cfg_attr(not(any(sceptre_ocr, test)), allow(dead_code))]
/// Internal stable coordinates for one Paddle artifact in the classic OCR catalog.
pub(crate) struct PaddleCatalogEntry {
    /// Logical model identifier shared by companion artifacts.
    pub(crate) catalog_id: String,
    /// Pipeline role: detector, classifier, or recognizer.
    pub(crate) role: String,
    /// Artifact kind: model or dictionary.
    pub(crate) artifact_kind: String,
    /// Existing public download manifest entry.
    pub(crate) manifest: ModelManifestEntry,
}
#[cfg_attr(alef, alef(skip))]
/// Statistics about the PaddleOCR model cache.
#[cfg(paddle_ocr)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelCacheStats {
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
#[cfg(paddle_ocr)]
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
pub struct ModelManager {
    /// Explicit Hugging Face cache root. The default is resolved from the standard
    /// `HF_HUB_CACHE` / `HUGGINGFACE_HUB_CACHE` / `HF_HOME` conventions.
    cache_dir: PathBuf,
}

#[cfg(paddle_ocr)]
impl Default for ModelManager {
    fn default() -> Self {
        Self::new(hf_hub::resolve_cache_dir())
    }
}

#[cfg(paddle_ocr)]
fn artifact_size(remote_filename: &str) -> u64 {
    match remote_filename {
        "v2/det/server.onnx" => 88_047_983,
        "v2/det/mobile.onnx" => 4_766_440,
        "v2/classifiers/PP-LCNet_x1_0_textline_ori.onnx" => 6_775_212,
        "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx" => 6_785_465,
        "v2/rec/unified_server/model.onnx" => 84_480_012,
        "v2/rec/unified_server/dict.txt" => 74_015,
        "v2/rec/unified_mobile/model.onnx" => 16_529_870,
        "v2/rec/unified_mobile/dict.txt" => 74_015,
        "v2/rec/en_mobile/model.onnx" => 7_843_511,
        "v2/rec/en_mobile/dict.txt" => 1_419,
        "rec/latin/model.onnx" => 7_862_832,
        "rec/latin/dict.txt" => 1_638,
        "rec/korean/model.onnx" => 13_401_252,
        "rec/korean/dict.txt" => 47_455,
        "rec/eslav/model.onnx" => 7_870_092,
        "rec/eslav/dict.txt" => 1_667,
        "rec/thai/model.onnx" => 7_873_480,
        "rec/thai/dict.txt" => 1_771,
        "rec/greek/model.onnx" => 7_791_200,
        "rec/greek/dict.txt" => 1_107,
        "rec/arabic/model.onnx" => 8_022_231,
        "rec/arabic/dict.txt" => 2_369,
        "rec/devanagari/model.onnx" => 7_935_595,
        "rec/devanagari/dict.txt" => 1_943,
        "rec/tamil/model.onnx" => 7_908_975,
        "rec/tamil/dict.txt" => 1_723,
        "rec/telugu/model.onnx" => 7_922_043,
        "rec/telugu/dict.txt" => 1_831,
        "v6/det/medium/model.onnx" => 62_064_319,
        "v6/det/small/model.onnx" => 9_893_093,
        "v6/det/tiny/model.onnx" => 1_793_140,
        "v6/rec/medium/model.onnx" => 76_613_672,
        "v6/rec/medium/dict.txt" => 74_947,
        "v6/rec/small/model.onnx" => 21_218_539,
        "v6/rec/small/dict.txt" => 74_947,
        "v6/rec/tiny/model.onnx" => 4_484_067,
        "v6/rec/tiny/dict.txt" => 27_156,
        _ => unreachable!("missing pinned PaddleOCR artifact size for {remote_filename}"),
    }
}

#[cfg(paddle_ocr)]
fn catalog_entry(
    catalog_id: String,
    role: &str,
    artifact_kind: &str,
    remote_filename: String,
    sha256: &str,
) -> PaddleCatalogEntry {
    PaddleCatalogEntry {
        catalog_id,
        role: role.to_string(),
        artifact_kind: artifact_kind.to_string(),
        manifest: ModelManifestEntry {
            size_bytes: artifact_size(&remote_filename),
            source_url: format!("https://huggingface.co/{HF_REPO_ID}/resolve/{HF_REPO_REVISION}/{remote_filename}"),
            relative_path: remote_filename,
            sha256: sha256.to_string(),
        },
    }
}

#[cfg(paddle_ocr)]
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

    #[cfg(paddle_ocr)]
    fn manifest_cache_path(&self, relative_path: &str) -> PathBuf {
        self.cache_dir
            .join("models--xberg-io--paddleocr-onnx-models")
            .join("snapshots")
            .join(HF_REPO_REVISION)
            .join(relative_path)
    }

    #[cfg(paddle_ocr)]
    pub(crate) fn probe_manifest_presence(&self) -> (usize, usize, usize) {
        let mut present = 0;
        let mut missing = 0;
        let mut invalid = 0;
        for entry in Self::manifest() {
            let path = self.manifest_cache_path(&entry.relative_path);
            let metadata = match fs::metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    missing += 1;
                    continue;
                }
                Err(_) => {
                    invalid += 1;
                    continue;
                }
            };
            if metadata.len() != entry.size_bytes {
                invalid += 1;
                continue;
            }
            let readable = fs::File::open(path)
                .and_then(|mut file| {
                    let mut byte = [0_u8; 1];
                    file.read_exact(&mut byte)
                })
                .is_ok();
            if readable {
                present += 1;
            } else {
                invalid += 1;
            }
        }
        (present, missing, invalid)
    }

    /// Ensures a recognition model for the given script family exists locally.
    ///
    /// Downloads the model and character dictionary from HuggingFace if not cached.
    ///
    /// # Arguments
    ///
    /// * `family` - Script family name (e.g., "english", "chinese", "latin")
    pub(crate) fn ensure_rec_model(&self, family: &str) -> Result<RecModelPaths, XbergError> {
        let definition = Self::find_rec_definition(family).ok_or_else(|| XbergError::Plugin {
            message: format!("Unsupported script family: {family}"),
            plugin_name: "paddle-ocr".to_string(),
        })?;

        let (model_file, dict_file) = self.download_rec_model(definition)?;

        Ok(RecModelPaths {
            rec_model: model_file,
            dict_file,
        })
    }

    /// Find the recognition model definition for a script family.
    fn find_rec_definition(family: &str) -> Option<&'static RecModelDefinition> {
        REC_MODELS.iter().find(|d| d.script_family == family)
    }

    /// Download a recognition model + dict for a script family.
    fn download_rec_model(&self, definition: &RecModelDefinition) -> Result<(PathBuf, PathBuf), XbergError> {
        let family = definition.script_family;

        let remote_model = format!("rec/{family}/model.onnx");
        let model = self.hf_download(&remote_model, definition.model_sha256)?;

        let remote_dict = format!("rec/{family}/dict.txt");
        let dict = self.hf_download(&remote_dict, definition.dict_sha256)?;

        tracing::info!(
            family,
            "Recognition model and dictionary resolved from Hugging Face cache"
        );
        Ok((model, dict))
    }

    /// Resolve and validate a file in the standard Hugging Face cache.
    fn hf_download(&self, remote_filename: &str, sha256: &str) -> Result<PathBuf, XbergError> {
        model_download::hf_resolve_file(
            HF_REPO_ID,
            remote_filename,
            Some(HF_REPO_REVISION),
            Some(&self.cache_dir),
            Some(sha256),
        )
        .map_err(|e| XbergError::Plugin {
            message: e,
            plugin_name: "paddle-ocr".to_string(),
        })
    }

    /// Verify SHA256 checksum of a downloaded file.
    #[cfg(test)]
    fn verify_checksum(path: &Path, expected: &str, label: &str) -> Result<(), XbergError> {
        model_download::verify_sha256(path, expected, label).map_err(|e| XbergError::Validation {
            message: e,
            source: None,
        })
    }

    /// The immutable Hub revision the runtime pins its model set to.
    pub(crate) fn pinned_revision() -> &'static str {
        HF_REPO_REVISION
    }

    /// Resolve one catalog entry through the same pinned, checksum-verifying path
    /// used by runtime model initialization.
    #[cfg_attr(not(sceptre_ocr), allow(dead_code))]
    pub(crate) fn resolve_manifest_entry(&self, entry: &ModelManifestEntry) -> Result<PathBuf, XbergError> {
        self.hf_download(&entry.relative_path, &entry.sha256)
    }

    /// Cache status of every artifact a run with the given version, script
    /// family, and tier would need — WITHOUT downloading.
    ///
    /// Mirrors the resolution arms of [`Self::ensure_shared_models_versioned`]
    /// and [`Self::resolve_rec_model_versioned`] so doctor reports on exactly
    /// the files the runtime would request. Invalid version/tier/family
    /// combinations error the same way the runtime would.
    #[cfg(paddle_ocr)]
    pub(crate) fn check_models_cached(
        &self,
        version: &str,
        family: &str,
        tier: &str,
        include_doc_ori: bool,
    ) -> Result<Vec<(String, bool)>, XbergError> {
        let mut artifacts: Vec<(String, String, &str)> = Vec::new();

        let invalid_tier = |tier: &str| XbergError::Plugin {
            message: format!("Invalid model_tier \"{tier}\". Valid values: \"server\", \"mobile\""),
            plugin_name: "paddle-ocr".to_string(),
        };

        if version == "pp-ocrv6" {
            let tier = effective_v6_tier(tier);
            let det = V6_DET_MODELS
                .iter()
                .find(|d| d.tier == tier)
                .ok_or_else(|| invalid_tier(tier))?;
            artifacts.push((
                format!("det ({version}/{tier})"),
                det.remote_filename.to_string(),
                det.sha256_checksum,
            ));
        } else {
            let det = V2_DET_MODELS
                .iter()
                .find(|d| d.tier == tier)
                .ok_or_else(|| invalid_tier(tier))?;
            artifacts.push((
                format!("det ({tier})"),
                det.remote_filename.to_string(),
                det.sha256_checksum,
            ));
        }
        artifacts.push((
            "cls (PP-LCNet textline_ori)".to_string(),
            V2_CLS_MODEL.remote_filename.to_string(),
            V2_CLS_MODEL.sha256_checksum,
        ));

        if version == "pp-ocrv6" && V6_UNIFIED_FAMILIES.contains(&family) {
            let tier = effective_v6_tier(tier);
            let rec = V6_REC_MODELS
                .iter()
                .find(|d| d.tier == tier)
                .ok_or_else(|| invalid_tier(tier))?;
            artifacts.push((
                format!("rec ({version}/{tier})"),
                rec.remote_model.to_string(),
                rec.model_sha256,
            ));
            artifacts.push((
                format!("dict ({version}/{tier})"),
                rec.remote_dict.to_string(),
                rec.dict_sha256,
            ));
        } else {
            let model_key = match (family, tier) {
                ("english", "server") | ("chinese", "server") => Some("unified_server"),
                ("english", "mobile") | ("chinese", "mobile") => Some("unified_mobile"),
                _ => None,
            };
            if let Some(model_key) = model_key {
                let rec =
                    V2_REC_MODELS
                        .iter()
                        .find(|d| d.model_key == model_key)
                        .ok_or_else(|| XbergError::Plugin {
                            message: format!("Unknown v2 rec model key: {model_key}"),
                            plugin_name: "paddle-ocr".to_string(),
                        })?;
                artifacts.push((
                    format!("rec ({model_key})"),
                    rec.remote_model.to_string(),
                    rec.model_sha256,
                ));
                artifacts.push((
                    format!("dict ({model_key})"),
                    rec.remote_dict.to_string(),
                    rec.dict_sha256,
                ));
            } else {
                let rec = Self::find_rec_definition(family).ok_or_else(|| XbergError::Plugin {
                    message: format!("Unsupported script family: {family}"),
                    plugin_name: "paddle-ocr".to_string(),
                })?;
                artifacts.push((
                    format!("rec ({family})"),
                    format!("rec/{family}/model.onnx"),
                    rec.model_sha256,
                ));
                artifacts.push((
                    format!("dict ({family})"),
                    format!("rec/{family}/dict.txt"),
                    rec.dict_sha256,
                ));
            }
        }

        if include_doc_ori {
            artifacts.push((
                "doc orientation (PP-LCNet doc_ori)".to_string(),
                V2_DOC_ORI_MODEL.remote_filename.to_string(),
                V2_DOC_ORI_MODEL.sha256_checksum,
            ));
        }

        Ok(artifacts
            .into_iter()
            .map(|(label, remote, sha)| (label, self.cached_artifact_ok(&remote, sha)))
            .collect())
    }

    /// Whether a pinned artifact is present in the local HF cache and passes
    /// its checksum. Never downloads.
    #[cfg(paddle_ocr)]
    fn cached_artifact_ok(&self, remote_filename: &str, sha256: &str) -> bool {
        let Ok(api) = model_download::hf_client(Some(&self.cache_dir)) else {
            return false;
        };
        model_download::hf_cached_revision_with_client(&api, HF_REPO_ID, remote_filename, Some(HF_REPO_REVISION))
            .ok()
            .flatten()
            .is_some_and(|path| model_download::verify_sha256(&path, sha256, remote_filename).is_ok())
    }

    /// Returns the manifest of all PaddleOCR model files with checksums and sizes.
    ///
    /// Entries are the exact pinned Hub artifacts used by the runtime. Paths are
    /// repository-relative paths within the immutable Hugging Face snapshot.
    pub fn manifest() -> Vec<ModelManifestEntry> {
        Self::classic_ocr_catalog()
            .into_iter()
            .map(|entry| entry.manifest)
            .collect()
    }

    /// Returns canonical classic-OCR metadata without changing the public manifest shape.
    pub(crate) fn classic_ocr_catalog() -> Vec<PaddleCatalogEntry> {
        let mut entries = Vec::new();

        for det in V2_DET_MODELS {
            entries.push(catalog_entry(
                format!("paddle-v2-det-{}", det.tier),
                "detector",
                "model",
                det.remote_filename.to_string(),
                det.sha256_checksum,
            ));
        }
        entries.push(catalog_entry(
            "paddle-v2-cls-textline-orientation".to_string(),
            "classifier",
            "model",
            V2_CLS_MODEL.remote_filename.to_string(),
            V2_CLS_MODEL.sha256_checksum,
        ));
        entries.push(catalog_entry(
            "paddle-v2-cls-document-orientation".to_string(),
            "classifier",
            "model",
            V2_DOC_ORI_MODEL.remote_filename.to_string(),
            V2_DOC_ORI_MODEL.sha256_checksum,
        ));
        for rec in V2_REC_MODELS {
            let catalog_id = format!("paddle-v2-rec-{}", rec.model_key);
            entries.push(catalog_entry(
                catalog_id.clone(),
                "recognizer",
                "model",
                rec.remote_model.to_string(),
                rec.model_sha256,
            ));
            entries.push(catalog_entry(
                catalog_id,
                "recognizer",
                "dictionary",
                rec.remote_dict.to_string(),
                rec.dict_sha256,
            ));
        }

        for rec in REC_MODELS {
            let catalog_id = format!("paddle-v5-rec-{}", rec.script_family);
            entries.push(catalog_entry(
                catalog_id.clone(),
                "recognizer",
                "model",
                format!("rec/{}/model.onnx", rec.script_family),
                rec.model_sha256,
            ));
            entries.push(catalog_entry(
                catalog_id,
                "recognizer",
                "dictionary",
                format!("rec/{}/dict.txt", rec.script_family),
                rec.dict_sha256,
            ));
        }

        for det in V6_DET_MODELS {
            entries.push(catalog_entry(
                format!("paddle-v6-det-{}", det.tier),
                "detector",
                "model",
                det.remote_filename.to_string(),
                det.sha256_checksum,
            ));
        }
        for rec in V6_REC_MODELS {
            let catalog_id = format!("paddle-v6-rec-{}", rec.tier);
            entries.push(catalog_entry(
                catalog_id.clone(),
                "recognizer",
                "model",
                rec.remote_model.to_string(),
                rec.model_sha256,
            ));
            entries.push(catalog_entry(
                catalog_id,
                "recognizer",
                "dictionary",
                rec.remote_dict.to_string(),
                rec.dict_sha256,
            ));
        }

        entries
    }

    /// Ensures every PaddleOCR model the runtime can request is downloaded and cached.
    ///
    /// Downloads:
    /// - Both v2 detection tiers (server + mobile)
    /// - Classification model (PP-LCNet textline_ori)
    /// - Document orientation model (PP-LCNet doc_ori)
    /// - All v2 unified rec models (server, mobile, en_mobile)
    /// - All per-script rec models for uncovered scripts
    /// - All PP-OCRv6 detection tiers (medium, small, tiny)
    /// - All PP-OCRv6 unified recognition tiers (model + dict)
    ///
    /// PP-OCRv6 is the default `model_version`, so warming must include the v6
    /// set — otherwise an airgapped server running the default config fails to
    /// resolve `v6/det/...` and `v6/rec/...` at request time even after a
    /// successful `cache warm` (issue #1279). The six groups below mirror the
    /// six groups enumerated by [`Self::manifest`].
    pub fn ensure_all_models(&self) -> Result<(), XbergError> {
        self.ensure_v2_det_model("server")?;
        self.ensure_v2_det_model("mobile")?;
        self.ensure_v2_cls_model()?;

        self.ensure_doc_ori_model()?;

        for v2_rec in V2_REC_MODELS {
            self.ensure_v2_rec_model(v2_rec.model_key)?;
        }

        for rec in REC_MODELS {
            self.ensure_rec_model(rec.script_family)?;
        }

        for det in V6_DET_MODELS {
            self.ensure_v6_det_model(det.tier)?;
        }

        for rec in V6_REC_MODELS {
            self.ensure_v6_rec_model(rec.tier)?;
        }

        tracing::info!(
            "All PaddleOCR models ready ({} v2 rec + {} per-script families + {} v6 det + {} v6 rec)",
            V2_REC_MODELS.len(),
            REC_MODELS.len(),
            V6_DET_MODELS.len(),
            V6_REC_MODELS.len()
        );
        Ok(())
    }

    /// Ensures the v2 detection model for the given tier is cached locally.
    ///
    /// Returns the exact ONNX file in the Hugging Face snapshot.
    pub(crate) fn ensure_v2_det_model(&self, tier: &str) -> Result<PathBuf, XbergError> {
        let definition = V2_DET_MODELS
            .iter()
            .find(|d| d.tier == tier)
            .ok_or_else(|| XbergError::Plugin {
                message: format!("Invalid model_tier \"{tier}\". Valid values: \"server\", \"mobile\""),
                plugin_name: "paddle-ocr".to_string(),
            })?;

        self.hf_download(definition.remote_filename, definition.sha256_checksum)
    }

    /// Ensures the v2 classification model is cached locally.
    ///
    /// The cls model is the same for both tiers.
    pub(crate) fn ensure_v2_cls_model(&self) -> Result<PathBuf, XbergError> {
        self.hf_download(V2_CLS_MODEL.remote_filename, V2_CLS_MODEL.sha256_checksum)
    }

    /// Ensures the v2 document orientation model is cached locally.
    ///
    /// Used for page-level auto_rotate when PaddleOCR backend is active.
    pub(crate) fn ensure_doc_ori_model(&self) -> Result<PathBuf, XbergError> {
        self.hf_download(V2_DOC_ORI_MODEL.remote_filename, V2_DOC_ORI_MODEL.sha256_checksum)
    }

    /// Ensures shared models (det + cls) are cached for the given tier.
    pub(crate) fn ensure_shared_models(&self, tier: &str) -> Result<SharedModelPaths, XbergError> {
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
    pub(crate) fn resolve_rec_model(&self, family: &str, tier: &str) -> Result<ResolvedRecModel, XbergError> {
        match (family, tier) {
            ("english", "server") | ("chinese", "server") => self.ensure_v2_rec_model("unified_server"),
            ("english", "mobile") | ("chinese", "mobile") => self.ensure_v2_rec_model("unified_mobile"),

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
    fn ensure_v2_rec_model(&self, model_key: &str) -> Result<ResolvedRecModel, XbergError> {
        let definition = V2_REC_MODELS
            .iter()
            .find(|d| d.model_key == model_key)
            .ok_or_else(|| XbergError::Plugin {
                message: format!("Unknown v2 rec model key: {model_key}"),
                plugin_name: "paddle-ocr".to_string(),
            })?;

        let model_file = self.hf_download(definition.remote_model, definition.model_sha256)?;
        let dict_file = self.hf_download(definition.remote_dict, definition.dict_sha256)?;

        Ok(ResolvedRecModel {
            model_dir: model_file,
            dict_file,
            model_key: format!("v2:{model_key}"),
        })
    }

    /// Ensures the PP-OCRv6 detection model for the given tier is cached locally.
    ///
    /// The v6 detector is script-agnostic; `tier` is one of `medium`/`small`/`tiny`.
    pub(crate) fn ensure_v6_det_model(&self, tier: &str) -> Result<PathBuf, XbergError> {
        let tier = effective_v6_tier(tier);
        let definition = V6_DET_MODELS
            .iter()
            .find(|d| d.tier == tier)
            .ok_or_else(|| XbergError::Plugin {
                message: format!("Invalid PP-OCRv6 tier \"{tier}\". Valid values: \"medium\", \"small\", \"tiny\""),
                plugin_name: "paddle-ocr".to_string(),
            })?;

        self.hf_download(definition.remote_filename, definition.sha256_checksum)
    }

    /// Ensures the PP-OCRv6 unified recognition model for the given tier is cached locally.
    fn ensure_v6_rec_model(&self, tier: &str) -> Result<ResolvedRecModel, XbergError> {
        let tier = effective_v6_tier(tier);
        let definition = V6_REC_MODELS
            .iter()
            .find(|d| d.tier == tier)
            .ok_or_else(|| XbergError::Plugin {
                message: format!("Invalid PP-OCRv6 tier \"{tier}\". Valid values: \"medium\", \"small\", \"tiny\""),
                plugin_name: "paddle-ocr".to_string(),
            })?;

        let model_file = self.hf_download(definition.remote_model, definition.model_sha256)?;
        let dict_file = self.hf_download(definition.remote_dict, definition.dict_sha256)?;

        Ok(ResolvedRecModel {
            model_dir: model_file,
            dict_file,
            model_key: format!("v6:{tier}"),
        })
    }

    /// Ensures shared models (det + cls) for the given model version and tier.
    ///
    /// For `pp-ocrv6` the detector comes from the v6 tree (`medium`/`small`/`tiny`) while the
    /// classifier reuses the PP-LCNet textline-orientation model (v6 ships no dedicated cls).
    /// Any other version resolves to the PP-OCRv5 shared models.
    pub(crate) fn ensure_shared_models_versioned(
        &self,
        version: &str,
        tier: &str,
    ) -> Result<SharedModelPaths, XbergError> {
        if version == "pp-ocrv6" {
            let det_model = self.ensure_v6_det_model(tier)?;
            let cls_model = self.ensure_v2_cls_model()?;
            Ok(SharedModelPaths { det_model, cls_model })
        } else {
            self.ensure_shared_models(tier)
        }
    }

    /// Resolves the recognition model for a script family, model version, and tier.
    ///
    /// For `pp-ocrv6`, families covered by the unified model (English, Chinese, Latin)
    /// resolve to the v6 recognition model for the effective tier; all other scripts fall back
    /// to the PP-OCRv5 per-script models. Any other version resolves entirely via PP-OCRv5.
    pub(crate) fn resolve_rec_model_versioned(
        &self,
        version: &str,
        family: &str,
        tier: &str,
    ) -> Result<ResolvedRecModel, XbergError> {
        if version == "pp-ocrv6" && V6_UNIFIED_FAMILIES.contains(&family) {
            self.ensure_v6_rec_model(tier)
        } else {
            self.resolve_rec_model(family, tier)
        }
    }
}

#[cfg(all(test, paddle_ocr))]
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
    fn test_runtime_shared_model_definitions() {
        assert_eq!(V2_DET_MODELS.len(), 2);
        assert_eq!(
            V2_CLS_MODEL.remote_filename,
            "v2/classifiers/PP-LCNet_x1_0_textline_ori.onnx"
        );
        assert_eq!(
            V2_DOC_ORI_MODEL.remote_filename,
            "v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx"
        );
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
        let paths1 = ModelPaths {
            det_model: temp_dir.path().join("server.onnx"),
            cls_model: temp_dir.path().join("cls.onnx"),
            rec_model: temp_dir.path().join("rec.onnx"),
            dict_file: temp_dir.path().join("dict.txt"),
        };
        let paths2 = paths1.clone();
        assert_eq!(paths1.det_model, paths2.det_model);
        assert_eq!(paths1.cls_model, paths2.cls_model);
        assert_eq!(paths1.rec_model, paths2.rec_model);
        assert_eq!(paths1.dict_file, paths2.dict_file);
    }

    #[test]
    fn test_shared_model_paths_hold_exact_artifacts() {
        let temp_dir = TempDir::new().unwrap();
        let paths = SharedModelPaths {
            det_model: temp_dir.path().join("server.onnx"),
            cls_model: temp_dir.path().join("cls.onnx"),
        };
        assert!(paths.det_model.ends_with("server.onnx"));
        assert!(paths.cls_model.ends_with("cls.onnx"));
    }

    #[test]
    fn test_rec_model_paths_hold_exact_artifacts() {
        let temp_dir = TempDir::new().unwrap();
        let paths = RecModelPaths {
            rec_model: temp_dir.path().join("rec.onnx"),
            dict_file: temp_dir.path().join("dict.txt"),
        };
        assert!(paths.rec_model.ends_with("rec.onnx"));
        assert!(paths.dict_file.ends_with("dict.txt"));
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

        assert_eq!(entries.len(), 2 + 2 + 3 * 2 + 9 * 2 + 3 + 3 * 2);

        let paths: Vec<&str> = entries.iter().map(|e| e.relative_path.as_str()).collect();
        for tier in &["medium", "small", "tiny"] {
            assert!(paths.contains(&format!("v6/det/{tier}/model.onnx").as_str()));
            assert!(paths.contains(&format!("v6/rec/{tier}/model.onnx").as_str()));
            assert!(paths.contains(&format!("v6/rec/{tier}/dict.txt").as_str()));
        }

        assert!(paths.contains(&"v2/det/server.onnx"));
        assert!(paths.contains(&"v2/det/mobile.onnx"));
        assert!(paths.contains(&"v2/classifiers/PP-LCNet_x1_0_textline_ori.onnx"));
        assert!(paths.contains(&"v2/classifiers/PP-LCNet_x1_0_doc_ori.onnx"));
        assert!(paths.contains(&"v2/rec/unified_server/model.onnx"));
        assert!(paths.contains(&"v2/rec/unified_server/dict.txt"));

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
            let model_path = format!("rec/{family}/model.onnx");
            let dict_path = format!("rec/{family}/dict.txt");
            assert!(paths.contains(&model_path.as_str()), "Missing model for {family}");
            assert!(paths.contains(&dict_path.as_str()), "Missing dict for {family}");
        }
    }

    #[test]
    fn manifest_has_the_exact_stable_catalog_and_artifact_census() {
        use std::collections::{BTreeMap, BTreeSet};

        let entries = ModelManager::classic_ocr_catalog();
        let expected_ids = BTreeSet::from([
            "paddle-v2-cls-document-orientation",
            "paddle-v2-cls-textline-orientation",
            "paddle-v2-det-mobile",
            "paddle-v2-det-server",
            "paddle-v2-rec-en_mobile",
            "paddle-v2-rec-unified_mobile",
            "paddle-v2-rec-unified_server",
            "paddle-v5-rec-arabic",
            "paddle-v5-rec-devanagari",
            "paddle-v5-rec-eslav",
            "paddle-v5-rec-greek",
            "paddle-v5-rec-korean",
            "paddle-v5-rec-latin",
            "paddle-v5-rec-tamil",
            "paddle-v5-rec-telugu",
            "paddle-v5-rec-thai",
            "paddle-v6-det-medium",
            "paddle-v6-det-small",
            "paddle-v6-det-tiny",
            "paddle-v6-rec-medium",
            "paddle-v6-rec-small",
            "paddle-v6-rec-tiny",
        ]);
        let actual_ids: BTreeSet<_> = entries.iter().map(|entry| entry.catalog_id.as_str()).collect();
        assert_eq!(actual_ids, expected_ids);

        let mut artifacts_by_id: BTreeMap<&str, (&str, BTreeSet<&str>)> = BTreeMap::new();
        for entry in &entries {
            let grouped = artifacts_by_id
                .entry(&entry.catalog_id)
                .or_insert((&entry.role, BTreeSet::new()));
            assert_eq!(grouped.0, entry.role.as_str());
            assert!(grouped.1.insert(&entry.artifact_kind));
        }
        assert_eq!(artifacts_by_id.len(), 22);
        assert_eq!(
            artifacts_by_id.values().filter(|(role, _)| *role == "detector").count(),
            5
        );
        assert_eq!(
            artifacts_by_id
                .values()
                .filter(|(role, _)| *role == "classifier")
                .count(),
            2
        );
        assert_eq!(
            artifacts_by_id
                .values()
                .filter(|(role, _)| *role == "recognizer")
                .count(),
            15
        );
        for (role, artifact_kinds) in artifacts_by_id.values() {
            if *role == "recognizer" {
                assert_eq!(artifact_kinds, &BTreeSet::from(["dictionary", "model"]));
            } else {
                assert_eq!(artifact_kinds, &BTreeSet::from(["model"]));
            }
        }
        assert_eq!(entries.len(), 37);
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
                entry.source_url.contains(HF_REPO_REVISION),
                "Source URL should pin the immutable Hub revision for {}",
                entry.relative_path
            );
            assert!(
                entry.size_bytes > 0,
                "Artifact size should be authoritative for {}",
                entry.relative_path
            );
            assert!(entry.source_url.ends_with(&entry.relative_path));
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
    fn test_effective_v6_tier_mapping() {
        assert_eq!(effective_v6_tier("medium"), "medium");
        assert_eq!(effective_v6_tier("small"), "small");
        assert_eq!(effective_v6_tier("tiny"), "tiny");
        // GH#1602: `mobile` is the DEFAULT model_tier and the v5 name for the lightweight
        // tier, so it must not resolve to the heaviest v6 model. `small` keeps the full
        // recognition dictionary; `tiny` would narrow it to ~zh/en. ~keep
        assert_eq!(effective_v6_tier("mobile"), "small");
        assert_eq!(effective_v6_tier("server"), "medium");
        assert_eq!(effective_v6_tier("bogus"), "medium");
    }

    #[test]
    fn test_v6_model_definitions_complete() {
        assert_eq!(V6_DET_MODELS.len(), 3);
        assert_eq!(V6_REC_MODELS.len(), 3);
        for tier in &["medium", "small", "tiny"] {
            assert!(V6_DET_MODELS.iter().any(|d| d.tier == *tier));
            assert!(V6_REC_MODELS.iter().any(|d| d.tier == *tier));
        }
        let medium = V6_REC_MODELS.iter().find(|d| d.tier == "medium").unwrap();
        let small = V6_REC_MODELS.iter().find(|d| d.tier == "small").unwrap();
        let tiny = V6_REC_MODELS.iter().find(|d| d.tier == "tiny").unwrap();
        assert_eq!(medium.dict_sha256, small.dict_sha256);
        assert_ne!(tiny.dict_sha256, medium.dict_sha256);
    }

    #[test]
    fn test_v6_unified_family_routing() {
        assert_eq!(V6_UNIFIED_FAMILIES, &["english", "chinese", "latin"]);
        for uncovered in &[
            "korean",
            "arabic",
            "eslav",
            "thai",
            "greek",
            "devanagari",
            "tamil",
            "telugu",
        ] {
            assert!(!V6_UNIFIED_FAMILIES.contains(uncovered));
        }
    }
}

#[cfg(all(test, paddle_ocr))]
mod check_cached_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn empty_cache_reports_everything_missing() {
        let temp = TempDir::new().unwrap();
        let manager = ModelManager::new(temp.path().to_path_buf());
        let artifacts = manager
            .check_models_cached("pp-ocrv6", "english", "server", false)
            .unwrap();
        assert!(artifacts.len() >= 4, "det + cls + rec + dict, got {artifacts:?}");
        assert!(artifacts.iter().all(|(_, cached)| !cached));
    }

    #[test]
    fn bounded_manifest_probe_does_not_hash_exact_size_artifact() {
        let temp = TempDir::new().unwrap();
        let manager = ModelManager::new(temp.path().to_path_buf());
        let entry = ModelManager::manifest().into_iter().next().unwrap();
        let path = manager.manifest_cache_path(&entry.relative_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let file = fs::File::create(path).unwrap();
        file.set_len(entry.size_bytes).unwrap();

        let (present, missing, invalid) = manager.probe_manifest_presence();
        assert_eq!(present, 1);
        assert_eq!(missing, ModelManager::manifest().len() - 1);
        assert_eq!(invalid, 0);
    }

    #[test]
    fn bounded_manifest_probe_reports_wrong_size_artifact_invalid() {
        let temp = TempDir::new().unwrap();
        let manager = ModelManager::new(temp.path().to_path_buf());
        let entry = ModelManager::manifest().into_iter().next().unwrap();
        let path = manager.manifest_cache_path(&entry.relative_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"truncated").unwrap();

        let (present, missing, invalid) = manager.probe_manifest_presence();
        assert_eq!(present, 0);
        assert_eq!(missing, ModelManager::manifest().len() - 1);
        assert_eq!(invalid, 1);
    }

    #[test]
    fn per_script_family_resolves_its_own_model() {
        let temp = TempDir::new().unwrap();
        let manager = ModelManager::new(temp.path().to_path_buf());
        let artifacts = manager
            .check_models_cached("pp-ocrv5", "eslav", "server", false)
            .unwrap();
        assert!(artifacts.iter().any(|(label, _)| label.contains("eslav")));
    }

    #[test]
    fn invalid_tier_and_family_error_like_the_runtime() {
        let temp = TempDir::new().unwrap();
        let manager = ModelManager::new(temp.path().to_path_buf());
        assert!(
            manager
                .check_models_cached("pp-ocrv5", "english", "huge", false)
                .is_err()
        );
        assert!(
            manager
                .check_models_cached("pp-ocrv5", "klingon", "server", false)
                .is_err()
        );
    }
}
