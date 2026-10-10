//! Deterministic provisioning for the bounded classical OCR model catalog.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Result, XbergError};

const SCHEMA_VERSION: u32 = 1;
const MODEL_LICENSE: &str = "Apache-2.0";
const PADDLE_REPO: &str = "xberg-io/paddleocr-onnx-models";

/// One checksummed model artifact in the classical OCR cache.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassicOcrModelEntry {
    /// OCR backend that consumes the artifact.
    pub backend: String,
    /// Stable logical model identifier shared by companion artifacts.
    pub catalog_id: String,
    /// Model role within that backend.
    pub role: String,
    /// Artifact kind, either `model` or `dictionary`.
    pub artifact_kind: String,
    /// Hugging Face repository containing the artifact.
    pub repo: String,
    /// Registry revision used to resolve the artifact.
    pub revision: String,
    /// Repository-relative artifact path.
    pub file: String,
    /// Expected lowercase SHA-256 digest.
    pub sha256: String,
    /// SPDX license identifier for the model weights.
    pub license: String,
    /// Verified artifact size in bytes.
    pub size_bytes: u64,
    /// Resulting local path used by the runtime.
    pub path: PathBuf,
}

/// Stable, status-free result of seeding the complete classical OCR catalog.
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassicOcrCacheManifest {
    /// Manifest schema version.
    pub schema_version: u32,
    /// Xberg version that defined the catalog.
    pub xberg_version: String,
    /// Explicit Hugging Face cache root populated by the operation.
    pub cache_dir: PathBuf,
    /// Number of distinct backend/catalog identifier pairs.
    pub catalog_count: usize,
    /// Number of artifacts in [`Self::models`].
    pub artifact_count: usize,
    /// Sum of verified artifact sizes.
    pub total_size_bytes: u64,
    /// Complete catalog, sorted by stable model coordinates.
    pub models: Vec<ClassicOcrModelEntry>,
}

fn validate_cache_root(cache_dir: &Path) -> Result<()> {
    if cache_dir.as_os_str().is_empty() {
        return Err(XbergError::validation("Classic OCR cache directory must not be empty"));
    }
    Ok(())
}

fn build_manifest(cache_dir: PathBuf, mut models: Vec<ClassicOcrModelEntry>) -> ClassicOcrCacheManifest {
    models.sort_by(|left, right| {
        (
            &left.backend,
            &left.catalog_id,
            &left.artifact_kind,
            &left.repo,
            &left.revision,
            &left.file,
        )
            .cmp(&(
                &right.backend,
                &right.catalog_id,
                &right.artifact_kind,
                &right.repo,
                &right.revision,
                &right.file,
            ))
    });
    let catalog_count = models
        .iter()
        .map(|model| (&model.backend, &model.catalog_id))
        .collect::<std::collections::HashSet<_>>()
        .len();
    ClassicOcrCacheManifest {
        schema_version: SCHEMA_VERSION,
        xberg_version: env!("CARGO_PKG_VERSION").to_string(),
        cache_dir,
        catalog_count,
        artifact_count: models.len(),
        total_size_bytes: models.iter().map(|model| model.size_bytes).sum(),
        models,
    }
}

fn seed_paddle(cache_dir: &Path) -> Result<Vec<ClassicOcrModelEntry>> {
    let manager = crate::paddle_ocr::ModelManager::new(cache_dir.to_path_buf());
    crate::paddle_ocr::ModelManager::classic_ocr_catalog()
        .into_iter()
        .map(|entry| {
            let path = manager.resolve_manifest_entry(&entry.manifest)?;
            let size_bytes = std::fs::metadata(&path)?.len();
            Ok(ClassicOcrModelEntry {
                backend: "paddle-ocr".to_string(),
                catalog_id: entry.catalog_id,
                role: entry.role,
                artifact_kind: entry.artifact_kind,
                repo: PADDLE_REPO.to_string(),
                revision: crate::paddle_ocr::ModelManager::pinned_revision().to_string(),
                file: entry.manifest.relative_path,
                sha256: entry.manifest.sha256,
                license: MODEL_LICENSE.to_string(),
                size_bytes,
                path,
            })
        })
        .collect()
}

fn sceptre_languages() -> Vec<sceptre::Language> {
    vec![
        sceptre::Language::English,
        sceptre::Language::Latin,
        sceptre::Language::ChineseSimplified,
        sceptre::Language::Japanese,
        sceptre::Language::Korean,
        sceptre::Language::Cyrillic,
        sceptre::Language::Telugu,
        sceptre::Language::Kannada,
    ]
}

fn sceptre_config(cache_dir: &Path) -> sceptre::OcrConfig {
    let mut config = sceptre::OcrConfig::default();
    config.model.cache_dir = Some(cache_dir.to_path_buf());
    config.model.languages = sceptre_languages();
    config
}

fn seed_sceptre(cache_dir: &Path) -> Result<Vec<ClassicOcrModelEntry>> {
    let config = sceptre_config(cache_dir);
    let descriptors = sceptre::model_descriptors(&config)
        .map_err(|error| XbergError::ocr_with_source("Failed to enumerate Sceptre OCR models", error))?;

    descriptors
        .into_iter()
        .map(|descriptor| {
            let path = crate::model_download::hf_resolve_file(
                &descriptor.repo,
                &descriptor.file,
                Some(&descriptor.revision),
                Some(cache_dir),
                Some(&descriptor.sha256),
            )
            .map_err(|error| XbergError::ocr(format!("Failed to seed Sceptre model `{}`: {error}", descriptor.name)))?;
            let size_bytes = std::fs::metadata(&path)?.len();
            let role = match descriptor.role {
                sceptre::ModelRole::Detector => "detector",
                sceptre::ModelRole::Recognizer(_) => "recognizer",
            };
            Ok(ClassicOcrModelEntry {
                backend: "sceptre".to_string(),
                catalog_id: descriptor.name,
                role: role.to_string(),
                artifact_kind: "model".to_string(),
                repo: descriptor.repo,
                revision: descriptor.revision,
                file: descriptor.file,
                sha256: descriptor.sha256,
                license: MODEL_LICENSE.to_string(),
                size_bytes,
                path,
            })
        })
        .collect()
}

/// Download and verify every PaddleOCR and Sceptre model supported by this build.
///
/// The explicit `cache_dir` is used only for these classical OCR model families.
/// Entries are status-free and sorted, so repeated successful calls against the
/// same directory serialize identically.
#[cfg_attr(alef, alef(skip))]
pub fn seed_classic_ocr_cache(cache_dir: impl AsRef<Path>) -> Result<ClassicOcrCacheManifest> {
    let cache_dir = cache_dir.as_ref();
    validate_cache_root(cache_dir)?;
    let mut models = seed_paddle(cache_dir)?;
    models.extend(seed_sceptre(cache_dir)?);
    Ok(build_manifest(cache_dir.to_path_buf(), models))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn entry(backend: &str, file: &str, size_bytes: u64) -> ClassicOcrModelEntry {
        ClassicOcrModelEntry {
            backend: backend.to_string(),
            catalog_id: format!("{backend}-model"),
            role: "recognizer".to_string(),
            artifact_kind: "model".to_string(),
            repo: format!("xberg-io/{backend}"),
            revision: "revision".to_string(),
            file: file.to_string(),
            sha256: "a".repeat(64),
            license: "Apache-2.0".to_string(),
            size_bytes,
            path: PathBuf::from("/cache").join(file),
        }
    }

    #[test]
    fn should_sort_entries_and_derive_manifest_totals() {
        let manifest = build_manifest(
            PathBuf::from("/cache"),
            vec![entry("sceptre", "z.onnx", 7), entry("paddle-ocr", "a.onnx", 5)],
        );

        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.catalog_count, 2);
        assert_eq!(manifest.artifact_count, 2);
        assert_eq!(manifest.total_size_bytes, 12);
        assert_eq!(manifest.models[0].backend, "paddle-ocr");
        assert_eq!(manifest.models[1].backend, "sceptre");
        assert!(manifest.models.iter().all(|model| model.license == "Apache-2.0"));
    }

    #[test]
    fn should_build_a_byte_stable_status_free_manifest() {
        let first = build_manifest(
            PathBuf::from("/cache"),
            vec![entry("sceptre", "z.onnx", 7), entry("paddle-ocr", "a.onnx", 5)],
        );
        let second = build_manifest(
            PathBuf::from("/cache"),
            vec![entry("paddle-ocr", "a.onnx", 5), entry("sceptre", "z.onnx", 7)],
        );

        assert_eq!(
            serde_json::to_vec_pretty(&first).unwrap(),
            serde_json::to_vec_pretty(&second).unwrap()
        );
        let value = serde_json::to_value(first).unwrap();
        assert!(value.get("downloaded").is_none());
        assert!(value.get("already_cached").is_none());
    }

    #[test]
    fn should_describe_the_exact_bounded_catalog_without_network_access() {
        use std::collections::HashSet;

        let paddle = crate::paddle_ocr::ModelManager::classic_ocr_catalog();
        let sceptre = sceptre::model_descriptors(&sceptre_config(Path::new("/cache"))).unwrap();

        assert_eq!(paddle.len(), 37);
        assert_eq!(sceptre.len(), 9);
        assert_eq!(paddle.len() + sceptre.len(), 46);
        assert_eq!(
            paddle
                .iter()
                .map(|model| model.catalog_id.as_str())
                .collect::<HashSet<_>>()
                .len(),
            22
        );
        assert_eq!(MODEL_LICENSE, "Apache-2.0");
        let valid_sha256 = |hash: &str| {
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        assert!(paddle.iter().all(|model| valid_sha256(&model.manifest.sha256)));
        assert!(sceptre.iter().all(|model| valid_sha256(&model.sha256)));
        assert!(sceptre.iter().all(|model| {
            model.revision.len() == 40
                && model
                    .revision
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }));
        let actual_sceptre_catalog = sceptre
            .iter()
            .map(|model| (model.name.as_str(), model.role.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            actual_sceptre_catalog,
            vec![
                ("craft_mlt_25k", sceptre::ModelRole::Detector),
                ("english_g2", sceptre::ModelRole::Recognizer(sceptre::Language::English),),
                ("latin_g2", sceptre::ModelRole::Recognizer(sceptre::Language::Latin),),
                (
                    "zh_sim_g2",
                    sceptre::ModelRole::Recognizer(sceptre::Language::ChineseSimplified),
                ),
                (
                    "japanese_g2",
                    sceptre::ModelRole::Recognizer(sceptre::Language::Japanese),
                ),
                ("korean_g2", sceptre::ModelRole::Recognizer(sceptre::Language::Korean),),
                (
                    "cyrillic_g2",
                    sceptre::ModelRole::Recognizer(sceptre::Language::Cyrillic),
                ),
                ("telugu_g2", sceptre::ModelRole::Recognizer(sceptre::Language::Telugu),),
                ("kannada_g2", sceptre::ModelRole::Recognizer(sceptre::Language::Kannada),),
            ]
        );

        let coordinates = paddle
            .iter()
            .map(|model| {
                format!(
                    "{PADDLE_REPO}@{}:{}",
                    crate::paddle_ocr::ModelManager::pinned_revision(),
                    model.manifest.relative_path
                )
            })
            .chain(
                sceptre
                    .iter()
                    .map(|model| format!("{}@{}:{}", model.repo, model.revision, model.file)),
            )
            .collect::<HashSet<_>>();
        assert_eq!(coordinates.len(), 46);
    }

    #[test]
    fn should_reject_an_empty_cache_root() {
        let error = validate_cache_root(PathBuf::new().as_path()).expect_err("empty path must fail");
        assert_eq!(
            error.to_string(),
            "Validation error: Classic OCR cache directory must not be empty"
        );
    }
}
