//! Model dependencies shared by OCR result caching and native handle ownership.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::script_retry::{self, RetryModels, hash_language_models};
use super::validation::{TessdataEnv, resolve_tessdata_path_in, tessdata_search_dirs};
use crate::ocr::error::OcrError;
use crate::ocr::types::TesseractConfig;

fn invalidate_script_cache(invalidated: &AtomicBool, script: &str, error: &OcrError) {
    if !invalidated.swap(true, Ordering::AcqRel) {
        crate::core::extractor::cacheability::invalidate_current();
        tracing::warn!(script, %error, "script retry skipped or changed; bypassing result cache");
    }
}

#[derive(Clone)]
pub(super) struct ModelContext {
    pub(super) directory: String,
    pub(super) osd_revision: Option<blake3::Hash>,
    primary: Option<PrimaryModels>,
    retry: Option<RetryContext>,
    invalidated: Arc<AtomicBool>,
    han_cache_invalidated: Arc<AtomicBool>,
    khmer_cache_invalidated: Arc<AtomicBool>,
    operation: Option<crate::core::extractor::cacheability::Cacheability>,
}

#[derive(Clone)]
struct PrimaryModels {
    languages: Vec<String>,
    revision: blake3::Hash,
}

#[derive(Clone)]
struct RetryContext {
    directories: Vec<String>,
    models: Option<RetryModels>,
    noisy_below: f64,
}

impl ModelContext {
    #[cfg(test)]
    pub(super) fn resolve(config: &TesseractConfig) -> Result<Self, OcrError> {
        Self::resolve_with_retry(config, None)
    }

    pub(super) fn resolve_with_retry(config: &TesseractConfig, retry_threshold: Option<f64>) -> Result<Self, OcrError> {
        let languages = config
            .language
            .split('+')
            .map(|lang| lang.trim().to_string())
            .collect::<Vec<_>>();
        let env = TessdataEnv::from_process();
        let override_path = config.tessdata_path.as_deref();
        let directory = resolve_tessdata_path_in(&languages, override_path, &env).inspect_err(|_| {
            if matches!(config.psm, 0 | 1 | 12) {
                crate::core::extractor::cacheability::invalidate_current();
            }
        })?;
        let mut context = Self::for_path(directory, config.psm)?;
        if let Some(noisy_below) = retry_threshold.filter(|_| script_retry::supports_retry(config))
            && !crate::core::extractor::cacheability::ocr_stop_requested()
        {
            context.select_retry(config, tessdata_search_dirs(override_path, &env), noisy_below);
        }
        Ok(context)
    }

    fn select_retry(&mut self, config: &TesseractConfig, directories: Vec<String>, noisy_below: f64) {
        let han = match RetryModels::resolve_han(&directories, &config.language, config.psm) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.invalidate_han_cache(&error);
                None
            }
        };
        let khmer = match RetryModels::resolve_requested_khmer(&directories, &config.language, config.psm) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.invalidate_khmer_cache(&error);
                None
            }
        };
        let models = RetryModels::from_snapshots(han, khmer);
        if models.is_some() {
            // ~keep The first pass can use a different directory from the complete retry set.
            let languages: Vec<String> = config.language.split('+').map(|name| name.trim().into()).collect();
            match self.read_primary_revision(&languages) {
                Ok(revision) => self.primary = Some(PrimaryModels { languages, revision }),
                Err(error) => return self.invalidate_retry(&error),
            }
        }
        self.retry = Some(RetryContext {
            directories,
            models,
            noisy_below,
        });
    }

    pub(super) fn primary_revision(&self) -> Option<blake3::Hash> {
        self.primary.as_ref().map(|primary| primary.revision)
    }

    fn read_primary_revision(&self, languages: &[String]) -> Result<blake3::Hash, OcrError> {
        hash_language_models(
            std::path::Path::new(&self.directory),
            languages.iter().map(String::as_str),
        )
        .map(|hash| hash.finalize())
    }

    fn primary_is_current(&self) -> bool {
        let Some(primary) = self.primary.as_ref() else {
            return true;
        };
        let error = match self.read_primary_revision(&primary.languages) {
            Ok(revision) if revision == primary.revision => return true,
            Ok(_) => OcrError::ProcessingFailed("Primary Tesseract models changed during OCR".into()),
            Err(error) => error,
        };
        self.invalidate_retry(&error);
        false
    }

    pub(super) fn invalidate_retry(&self, error: &OcrError) {
        self.invalidated.store(true, Ordering::Release);
        crate::core::extractor::cacheability::invalidate_current();
        tracing::warn!(%error, "script retry skipped or changed; bypassing result cache");
    }

    pub(super) fn invalidate_han_cache(&self, error: &OcrError) {
        invalidate_script_cache(&self.han_cache_invalidated, "Han", error);
    }

    pub(super) fn invalidate_khmer_cache(&self, error: &OcrError) {
        invalidate_script_cache(&self.khmer_cache_invalidated, "Khmer", error);
    }

    pub(super) fn retry_models(
        &self,
        config: &TesseractConfig,
        mean_confidence: f64,
        han_is_plausible: impl FnOnce() -> bool,
    ) -> Option<RetryModels> {
        let retry = self.retry.as_ref()?;
        if mean_confidence >= retry.noisy_below || self.invalidated.load(Ordering::Acquire) {
            return None;
        }
        let mut models = retry.models.clone()?;
        if models.han.is_some() && (!han_is_plausible() || !self.han_retry_is_current(config)) {
            models.han = None;
        }
        if models.khmer.is_some() && !self.khmer_retry_is_current(config) {
            models.khmer = None;
        }
        RetryModels::from_snapshots(models.han, models.khmer)
    }

    pub(super) fn han_retry_is_current(&self, config: &TesseractConfig) -> bool {
        if self.invalidated.load(Ordering::Acquire) || self.han_cache_invalidated.load(Ordering::Acquire) {
            return false;
        }
        let Some(retry) = &self.retry else {
            return true;
        };
        let expected = retry.models.as_ref().and_then(|models| models.han.as_ref());
        let error = match RetryModels::resolve_han(&retry.directories, &config.language, config.psm) {
            Ok(current) if current.as_ref() == expected => return true,
            Ok(_) => OcrError::ProcessingFailed("Selected Han retry models changed during OCR".into()),
            Err(error) => error,
        };
        self.invalidate_han_cache(&error);
        false
    }

    pub(super) fn khmer_retry_is_current(&self, config: &TesseractConfig) -> bool {
        if self.invalidated.load(Ordering::Acquire) || self.khmer_cache_invalidated.load(Ordering::Acquire) {
            return false;
        }
        let Some(retry) = &self.retry else {
            return true;
        };
        let expected = retry.models.as_ref().and_then(|models| models.khmer.as_ref());
        let error = match RetryModels::resolve_requested_khmer(&retry.directories, &config.language, config.psm) {
            Ok(current) if current.as_ref() == expected => return true,
            Ok(_) => OcrError::ProcessingFailed("Selected Khmer retry models changed during OCR".into()),
            Err(error) => error,
        };
        self.invalidate_khmer_cache(&error);
        false
    }

    pub(super) fn if_result_current<T>(&self, config: &TesseractConfig, action: impl FnOnce() -> T) -> Option<T> {
        if !self.han_retry_is_current(config) || !self.khmer_retry_is_current(config) {
            return None;
        }
        self.if_current(action)
    }

    pub(super) fn for_provisioned(config: &TesseractConfig) -> Result<Self, OcrError> {
        let directory = config
            .tessdata_path
            .as_deref()
            .and_then(std::path::Path::to_str)
            .filter(|directory| !directory.is_empty())
            .ok_or_else(|| {
                OcrError::InvalidConfiguration("Provisioned OCR requires an explicit model directory".into())
            })?;
        super::validation::validate_language_and_traineddata(&config.language, directory)?;
        Self::for_path(directory.to_owned(), config.psm)
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
            primary: None,
            retry: None,
            invalidated: Arc::new(AtomicBool::new(false)),
            han_cache_invalidated: Arc::new(AtomicBool::new(false)),
            khmer_cache_invalidated: Arc::new(AtomicBool::new(false)),
            operation: osd_revision.and_then(|_| crate::core::extractor::cacheability::current()),
        })
    }

    pub(super) fn if_current<T>(&self, action: impl FnOnce() -> T) -> Option<T> {
        if self.invalidated.load(Ordering::Acquire)
            || self
                .operation
                .as_ref()
                .is_some_and(|operation| !operation.is_cacheable())
            || !self.primary_is_current()
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
        let mut base = super::config::hash_config(config, &self.directory);
        if let Some(retry) = self.retry.as_ref() {
            let mut hash = blake3::Hasher::new();
            hash.update(b"tesseract-script-retry-v7");
            hash.update(base.as_bytes());
            hash.update(&retry.noisy_below.to_bits().to_le_bytes());
            if script_retry::mixed_pdf_khmer_retry_requested() {
                hash.update(b"mixed-pdf-khmer-supplement-v6");
            }
            if let Some(models) = &retry.models {
                models.hash_into(&mut hash);
            }
            if let Some(revision) = self.primary_revision() {
                hash.update(revision.as_bytes());
            }
            base = hash.finalize().to_hex().to_string();
        }
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

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn khmer_snapshot_io_failure_keeps_han_and_blocks_cache() {
        use crate::core::extractor::cacheability;

        cacheability::scope(async {
            let root = tempfile::tempdir().unwrap();
            let directory = root.path().to_str().unwrap().to_owned();
            for name in ["eng", "osd", "chi_sim"] {
                std::fs::write(root.path().join(format!("{name}.traineddata")), name).unwrap();
            }
            std::fs::create_dir(root.path().join("khm.traineddata")).unwrap();
            script_retry::MIXED_PDF_KHMER_RETRY.sync_scope(true, || {
                let config = TesseractConfig {
                    psm: 3,
                    ..Default::default()
                };
                let mut context = ModelContext::for_path(directory.clone(), config.psm).unwrap();
                context.select_retry(&config, vec![directory], 75.0);
                assert!(
                    context.retry_models(&config, 30.0, || true).is_some(),
                    "Han must remain selectable"
                );
                assert!(context.if_result_current(&config, || ()).is_none());
                assert!(!cacheability::can_publish_outer_result());
            });
        })
        .await;
    }

    #[cfg(feature = "bundle-tessdata-eng")]
    #[test]
    fn marks_only_first_pass_selects_han_osd() {
        let api = xberg_tesseract::TesseractAPI::new().unwrap();
        let root = tempfile::tempdir().unwrap();
        for name in ["eng", "osd", "chi_sim"] {
            std::fs::write(root.path().join(format!("{name}.traineddata")), name).unwrap();
        }
        let directory = root.path().to_str().unwrap().to_owned();
        let config = TesseractConfig {
            psm: 3,
            ..Default::default()
        };
        let mut context = ModelContext::for_path(directory.clone(), config.psm).unwrap();
        context.select_retry(&config, vec![directory], 75.0);
        let header = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext";
        for (token, confidence, expected) in [("|", 20, true), ("|", 65, false), ("1", 20, false)] {
            let tsv = format!("{header}\n5\t1\t1\t1\t1\t1\t0\t100\t3\t40\t{confidence}\t{token}");
            let retry = context.retry_models(&config, f64::from(confidence), || {
                script_retry::first_read_suggests_han(&api, Some(&tsv))
            });
            assert_eq!(
                retry.is_some_and(|models| models.han.is_some()),
                expected,
                "{token} at {confidence}"
            );
        }
    }

    #[test]
    fn han_gate_keeps_requested_khmer_retry() {
        let root = tempfile::tempdir().unwrap();
        for name in ["eng", "osd", "chi_sim", "khm"] {
            std::fs::write(root.path().join(format!("{name}.traineddata")), name).unwrap();
        }
        let directory = root.path().to_str().unwrap().to_owned();
        let config = TesseractConfig {
            psm: 3,
            ..Default::default()
        };
        script_retry::MIXED_PDF_KHMER_RETRY.sync_scope(true, || {
            let mut context = ModelContext::for_path(directory.clone(), config.psm).unwrap();
            context.select_retry(&config, vec![directory], 75.0);
            let models = context.retry_models(&config, 30.0, || false).unwrap();
            assert!(models.han.is_none());
            assert!(models.khmer.is_some());
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn han_snapshot_io_failure_keeps_khmer_and_blocks_cache() {
        assert_han_snapshot_failure_keeps_khmer(true).await;
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn han_pre_read_drift_keeps_khmer_and_blocks_cache() {
        assert_han_snapshot_failure_keeps_khmer(false).await;
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn assert_han_snapshot_failure_keeps_khmer(fail_at_selection: bool) {
        use crate::core::extractor::cacheability;

        cacheability::scope(async {
            let root = tempfile::tempdir().unwrap();
            let directory = root.path().to_str().unwrap().to_owned();
            let han = root.path().join("chi_sim.traineddata");
            for name in ["eng", "osd", "khm"] {
                std::fs::write(root.path().join(format!("{name}.traineddata")), name).unwrap();
            }
            if fail_at_selection {
                std::fs::create_dir(&han).unwrap();
            } else {
                std::fs::write(&han, b"Han revision one").unwrap();
            }
            script_retry::MIXED_PDF_KHMER_RETRY.sync_scope(true, || {
                let config = TesseractConfig {
                    psm: 3,
                    ..Default::default()
                };
                let mut context = ModelContext::for_path(directory.clone(), config.psm).unwrap();
                context.select_retry(&config, vec![directory], 75.0);
                if !fail_at_selection {
                    assert!(context.if_result_current(&config, || ()).is_some());
                    std::fs::write(&han, b"Han revision two").unwrap();
                }
                let models = context
                    .retry_models(&config, 30.0, || true)
                    .expect("Khmer must remain selectable");
                assert!(models.han.is_none());
                assert!(models.khmer.is_some());
                assert!(context.khmer_retry_is_current(&config));
                assert!(context.if_result_current(&config, || ()).is_none());
                assert!(!cacheability::can_publish_outer_result());
            });
        })
        .await;
    }

    #[test]
    fn retry_cache_misses_when_a_complete_set_appears_moves_or_changes() {
        let root = tempfile::tempdir().unwrap();
        let primary = root.path().join("primary");
        let fallback = root.path().join("fallback");
        for dir in [&primary, &fallback] {
            std::fs::create_dir(dir).unwrap();
        }
        let write = |dir: &std::path::Path, lang: &str, bytes: &[u8]| {
            std::fs::write(dir.join(format!("{lang}.traineddata")), bytes).unwrap();
        };
        write(&primary, "eng", b"eng");
        write(&fallback, "osd", b"osd");
        write(&fallback, "chi_sim", b"han1");
        let directories: Vec<String> = vec![primary.to_str().unwrap().into(), fallback.to_str().unwrap().into()];
        let config = TesseractConfig {
            psm: 3,
            ..Default::default()
        };
        let snapshot = |threshold| {
            let mut context = ModelContext::for_path(directories[0].clone(), config.psm).unwrap();
            context.select_retry(&config, directories.clone(), threshold);
            context
        };
        let cache = crate::ocr::cache::OcrCache::new(Some(root.path().join("results"))).unwrap();
        let result = crate::types::OcrExtractionResult {
            content: "cached OCR output".into(),
            mime_type: "text/plain".into(),
            metadata: Default::default(),
            tables: vec![],
            ocr_elements: None,
            internal_document: None,
            raw_ocr_source_frame: None,
        };
        let lookup = |context: &ModelContext| {
            cache
                .get_cached_result("page", "tesseract", &context.result_key(&config), None)
                .unwrap()
        };
        let seed = |context: &ModelContext| {
            cache
                .set_cached_result("page", "tesseract", &context.result_key(&config), None, &result)
                .unwrap();
            assert_eq!(lookup(context).unwrap().content, result.content);
        };

        let missing = snapshot(75.0);
        assert!(missing.retry_models(&config, 30.0, || true).is_none());
        seed(&missing);
        write(&fallback, "eng", b"eng");
        assert!(missing.if_result_current(&config, || ()).is_none());
        let complete = snapshot(75.0);
        assert!(lookup(&complete).is_none());
        assert!(complete.retry_models(&config, 30.0, || true).is_some());
        assert!(complete.retry_models(&config, 75.0, || true).is_none());
        seed(&complete);
        assert!(lookup(&snapshot(80.0)).is_none());

        write(&primary, "eng", b"ENG");
        let replaced_primary = snapshot(75.0);
        assert!(replaced_primary.retry_models(&config, 30.0, || true).is_some());
        assert!(lookup(&replaced_primary).is_none());
        assert!(complete.if_result_current(&config, || ()).is_none());
        seed(&replaced_primary);

        write(&primary, "chi_sim", b"han1");
        let moved = snapshot(75.0);
        assert!(lookup(&moved).is_none());
        seed(&moved);
        write(&primary, "chi_sim", b"han2");
        assert!(moved.if_result_current(&config, || ()).is_none());
        let replaced_han = snapshot(75.0);
        assert!(lookup(&replaced_han).is_none());
        seed(&replaced_han);
        write(&fallback, "osd", b"OSD");
        assert!(lookup(&snapshot(75.0)).is_none());
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
            raw_ocr_source_frame: None,
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
            raw_ocr_source_frame: None,
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
