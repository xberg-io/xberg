//! Model dependencies shared by OCR result caching and native handle ownership.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::ocr::error::OcrError;
use crate::ocr::types::TesseractConfig;

#[derive(Clone)]
pub(super) struct ModelContext {
    pub(super) directory: String,
    pub(super) osd_revision: Option<blake3::Hash>,
    invalidated: Arc<AtomicBool>,
    operation: Option<crate::core::extractor::cacheability::Cacheability>,
}

impl ModelContext {
    pub(super) fn resolve(config: &TesseractConfig) -> Result<Self, OcrError> {
        let languages = config
            .language
            .split('+')
            .map(|lang| lang.trim().to_string())
            .collect::<Vec<_>>();
        let directory = super::validation::resolve_tessdata_path(&languages, config.tessdata_path.as_deref())
            .inspect_err(|_| {
                if matches!(config.psm, 0 | 1 | 12) {
                    crate::core::extractor::cacheability::invalidate_current();
                }
            })?;
        Self::for_path(directory, config.psm)
    }

    pub(super) fn for_path(directory: String, psm: u8) -> Result<Self, OcrError> {
        // Match pinned Tesseract's PSM_OSD_ENABLED: these modes load and retain osd.traineddata. ~keep
        let osd_revision = if matches!(psm, 0 | 1 | 12) {
            Some(osd_revision(std::path::Path::new(&directory))?)
        } else {
            None
        };
        Ok(Self {
            directory,
            osd_revision,
            invalidated: Arc::new(AtomicBool::new(false)),
            operation: osd_revision.and_then(|_| crate::core::extractor::cacheability::current()),
        })
    }

    pub(super) fn if_current<T>(&self, action: impl FnOnce() -> T) -> Option<T> {
        if self.invalidated.load(Ordering::Acquire)
            || self
                .operation
                .as_ref()
                .is_some_and(|operation| !operation.is_cacheable())
        {
            return None;
        }
        let Some(expected) = self.osd_revision else {
            return Some(action());
        };
        // Files can change while waiting for or using a handle; uncertain state must not be retained. ~keep
        match osd_revision(std::path::Path::new(&self.directory)) {
            Ok(current) if current == expected => Some(action()),
            Ok(_) => {
                self.invalidated.store(true, Ordering::Release);
                tracing::warn!(
                    "Tesseract OSD model changed during use; skipping cache reuse and discarding native state"
                );
                None
            }
            Err(error) => {
                self.invalidated.store(true, Ordering::Release);
                tracing::warn!(
                    %error,
                    "Cannot revalidate Tesseract OSD model; skipping cache reuse and discarding native state"
                );
                None
            }
        }
    }

    pub(super) fn result_key(&self, config: &TesseractConfig) -> String {
        let base = super::config::hash_config(config, &self.directory);
        let Some(revision) = self.osd_revision else {
            return base;
        };
        let mut hash = blake3::Hasher::new();
        hash.update(b"tesseract-osd-model-result-v1");
        hash.update(base.as_bytes());
        hash.update(revision.as_bytes());
        hash.finalize().to_hex().to_string()
    }
}

fn osd_revision(directory: &std::path::Path) -> Result<blake3::Hash, OcrError> {
    crate::core::extractor::cacheability::osd_revision(directory)
        .map_err(|error| model_read_error(&directory.join("osd.traineddata"), error))
}

fn model_read_error(path: &std::path::Path, error: impl std::fmt::Display) -> OcrError {
    OcrError::IOError(format!(
        "Cannot identify Tesseract OSD model '{}': {error}",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_present_and_non_file_osd_are_distinct_for_every_osd_mode() {
        let root = tempfile::tempdir().unwrap();
        let osd = root.path().join("osd.traineddata");
        for psm in [0, 1, 12] {
            let snapshot = || ModelContext::for_path(root.path().to_str().unwrap().into(), psm).unwrap();
            let missing = snapshot().osd_revision;
            std::fs::create_dir(&osd).unwrap();
            let not_file = snapshot().osd_revision;
            std::fs::remove_dir(&osd).unwrap();
            std::fs::write(&osd, b"orientation model fixture").unwrap();
            let present = snapshot().osd_revision;
            assert!(missing.is_some());
            assert_ne!(missing, not_file);
            assert_ne!(missing, present);
            assert_ne!(not_file, present);
            assert_eq!(present, snapshot().osd_revision);
            std::fs::remove_file(&osd).unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    fn read_errors_are_contextual_and_non_osd_modes_never_inspect_the_model() {
        let root = tempfile::tempdir().unwrap();
        let osd = root.path().join("osd.traineddata");
        std::os::unix::fs::symlink(&osd, &osd).unwrap();
        for psm in 0..14 {
            let snapshot = ModelContext::for_path(root.path().to_str().unwrap().into(), psm);
            if matches!(psm, 0 | 1 | 12) {
                assert!(matches!(snapshot, Err(OcrError::IOError(message))
                    if message.contains("Cannot identify Tesseract OSD model") && message.contains("osd.traineddata")));
            } else {
                let snapshot = snapshot.unwrap();
                assert!(snapshot.osd_revision.is_none());
                let config = TesseractConfig {
                    psm,
                    ..Default::default()
                };
                assert_eq!(
                    snapshot.result_key(&config),
                    super::super::config::hash_config(&config, &snapshot.directory)
                );
            }
        }
    }

    #[test]
    fn result_keys_use_the_captured_revision_without_reading_models_again() {
        let root = tempfile::tempdir().unwrap();
        let osd = root.path().join("osd.traineddata");
        std::fs::write(&osd, b"orientation model fixture").unwrap();
        let snapshot = ModelContext::for_path(root.path().to_str().unwrap().into(), 1).unwrap();
        let config = TesseractConfig {
            psm: 1,
            ..Default::default()
        };
        let key = snapshot.result_key(&config);
        std::fs::remove_file(&osd).unwrap();
        assert_eq!(
            key,
            snapshot.result_key(&config),
            "result identity must use its shared resolved snapshot"
        );
        let missing = ModelContext::for_path(root.path().to_str().unwrap().into(), 1).unwrap();
        assert_ne!(key, missing.result_key(&config));
    }

    #[test]
    fn changed_osd_bytes_invalidate_an_existing_serialized_ocr_result() {
        let root = tempfile::tempdir().unwrap();
        let models = root.path().join("models");
        std::fs::create_dir(&models).unwrap();
        let osd = models.join("osd.traineddata");
        std::fs::write(&osd, b"revision one").unwrap();
        let config = TesseractConfig {
            psm: 1,
            ..Default::default()
        };
        let before = ModelContext::for_path(models.to_str().unwrap().into(), config.psm).unwrap();
        let cache = crate::ocr::cache::OcrCache::new(Some(root.path().join("results"))).unwrap();
        let result = crate::types::OcrExtractionResult {
            content: "old model result".into(),
            mime_type: "text/plain".into(),
            metadata: Default::default(),
            tables: vec![],
            ocr_elements: None,
            internal_document: None,
        };
        cache
            .set_cached_result("page", "tesseract", &before.result_key(&config), None, &result)
            .unwrap();
        assert_eq!(cache.get_stats().unwrap().total_files, 1);
        assert_eq!(
            cache
                .get_cached_result("page", "tesseract", &before.result_key(&config), None)
                .unwrap()
                .unwrap()
                .content,
            "old model result"
        );
        std::fs::write(&osd, b"revision two").unwrap();
        assert_eq!(std::fs::metadata(&osd).unwrap().len(), 12);
        let after = ModelContext::for_path(models.to_str().unwrap().into(), config.psm).unwrap();
        assert!(
            cache
                .get_cached_result("page", "tesseract", &after.result_key(&config), None)
                .unwrap()
                .is_none(),
            "same-path equal-length OSD mutation must not return the old artifact"
        );
    }
    #[test]
    fn changed_revision_does_not_publish_a_result_under_the_old_key_after_rollback() {
        let root = tempfile::tempdir().unwrap();
        let osd = root.path().join("osd.traineddata");
        std::fs::write(&osd, b"revision one").unwrap();
        let config = TesseractConfig {
            psm: 1,
            ..Default::default()
        };
        let before = ModelContext::for_path(root.path().to_str().unwrap().into(), 1).unwrap();
        let cache = crate::ocr::cache::OcrCache::new(Some(root.path().join("results"))).unwrap();
        let result = crate::types::OcrExtractionResult {
            content: "seeded model B result".into(),
            mime_type: "text/plain".into(),
            metadata: Default::default(),
            tables: vec![],
            ocr_elements: None,
            internal_document: None,
        };
        std::fs::write(&osd, b"revision two").unwrap();
        let published = before
            .if_current(|| cache.set_cached_result("page", "tesseract", &before.result_key(&config), None, &result));
        std::fs::write(&osd, b"revision one").unwrap();
        assert!(published.is_none(), "changed OSD must skip the inner artifact write");
        let cached = || {
            cache
                .get_cached_result("page", "tesseract", &before.result_key(&config), None)
                .unwrap()
        };
        assert!(
            cached().is_none(),
            "rollback must not find the model B artifact under A"
        );
        assert!(before.if_current(|| cache.set_cached_result(
            "page", "tesseract", &before.result_key(&config), None, &result,
        )).is_none(), "rollback must not publish an output from an invalidated operation");
        let fresh = ModelContext::for_path(root.path().to_str().unwrap().into(), 1).unwrap();
        fresh
            .if_current(|| cache.set_cached_result("page", "tesseract", &fresh.result_key(&config), None, &result))
            .unwrap()
            .unwrap();
        assert_eq!(cached().unwrap().content, "seeded model B result");
    }

    #[cfg(unix)]
    #[test]
    fn revision_read_error_skips_reuse_but_non_osd_modes_do_not_read() {
        let root = tempfile::tempdir().unwrap();
        let osd = root.path().join("osd.traineddata");
        std::fs::write(&osd, b"revision one").unwrap();
        let before = ModelContext::for_path(root.path().to_str().unwrap().into(), 1).unwrap();
        let plain = ModelContext::for_path(root.path().to_str().unwrap().into(), 6).unwrap();
        std::fs::remove_file(&osd).unwrap();
        std::os::unix::fs::symlink(&osd, &osd).unwrap();
        assert!(
            before
                .if_current(|| panic!("uncertain revision must not publish or reuse"))
                .is_none()
        );
        assert_eq!(plain.if_current(|| 7), Some(7));
    }
}

#[cfg(all(
    test,
    feature = "pdf",
    feature = "ocr-pipeline",
    feature = "tokio-runtime",
    not(target_arch = "wasm32")
))]
#[path = "model_context/outer_cache_tests.rs"]
mod outer_cache_tests;
