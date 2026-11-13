//! Mutable model dependencies of OSD recognition, script retry and optional PDF recovery.

use crate::cancellation::CancellationToken;
use crate::core::config::ExtractionConfig;

#[derive(Clone, Default)]
pub(crate) struct CacheDependency {
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    model: Option<ModelSelection>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    stage_models: Vec<ModelSelection>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    script_retry: Option<ModelSelection>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    uncertifiable: bool,
}

impl CacheDependency {
    pub(crate) fn for_config(config: &ExtractionConfig, mime: Option<&str>) -> Self {
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        {
            let dependency = Self {
                model: ModelSelection::for_config(config, mime),
                script_retry: ModelSelection::for_script_retry(config, mime),
                ..Self::default()
            };
            #[cfg(feature = "pdf")]
            let dependency = {
                let mut dependency = dependency;
                dependency.collect_stage_models(config, mime);
                dependency
            };
            dependency
        }
        #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
        {
            let _ = (config, mime);
            Self::default()
        }
    }
}

pub(crate) fn apply_context(key: Option<String>, context: &Option<String>) -> Option<String> {
    let key = key?;
    let context = context.as_deref()?;
    if context.is_empty() {
        return Some(key);
    }
    let mut hash = blake3::Hasher::new();
    hash.update(b"runtime-cache-context-v1");
    hash.update(&(key.len() as u64).to_le_bytes());
    hash.update(key.as_bytes());
    hash.update(context.as_bytes());
    Some(hash.finalize().to_hex().to_string())
}

pub(crate) async fn fingerprint(
    dependencies: &[CacheDependency],
    cancel: Option<&CancellationToken>,
) -> crate::Result<Option<String>> {
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    {
        fingerprint_with_reader(dependencies, cancel, fingerprint_models).await
    }
    #[cfg(not(all(feature = "ocr", not(target_arch = "wasm32"))))]
    {
        check_cancelled(cancel)?;
        let _ = dependencies;
        Ok(Some(String::new()))
    }
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
async fn fingerprint_with_reader(
    dependencies: &[CacheDependency],
    cancel: Option<&CancellationToken>,
    read_models: impl FnOnce(&[CacheDependency], Option<&CancellationToken>) -> crate::Result<String> + Send + 'static,
) -> crate::Result<Option<String>> {
    check_cancelled(cancel)?;
    if dependencies.iter().any(|dependency| dependency.uncertifiable) {
        super::cacheability::invalidate_current();
        return Ok(None);
    }
    let has_orientation_models = dependencies
        .iter()
        .any(|dependency| dependency.model.is_some() || !dependency.stage_models.is_empty());
    let has_script_retry = dependencies.iter().any(|dependency| dependency.script_retry.is_some());
    if has_orientation_models || has_script_retry {
        let dependencies = dependencies.to_vec();
        let worker_cancel = cancel.cloned();
        let result = tokio::task::spawn_blocking(super::cacheability::blocking(move || {
            check_cancelled(worker_cancel.as_ref())?;
            // With a complete retry set, the inner OCR cache owns model identity. Recheck this
            // before an outer write as well, since extraction can make a model set available. ~keep
            if script_retry_requires_bypass(&dependencies, worker_cancel.as_ref())? {
                return Ok(None);
            }
            let mut context = if has_orientation_models {
                read_models(&dependencies, worker_cancel.as_ref())?
            } else {
                String::new()
            };
            if has_script_retry {
                // ~keep Legacy outer entries may contain a retry result even after its models are removed.
                context.push_str(":script-retry-outer-v1");
            }
            Ok(Some(context))
        }))
        .await;
        check_cancelled(cancel)?;
        return match result {
            Ok(Ok(context)) => Ok(context),
            Ok(Err(crate::XbergError::Cancelled)) => Err(crate::XbergError::Cancelled),
            Err(error) if error.is_cancelled() => Err(crate::XbergError::Cancelled),
            result => {
                super::cacheability::invalidate_current();
                tracing::warn!(
                    ?result,
                    "Cannot identify recognition-orientation model context; bypassing result cache"
                );
                Ok(None)
            }
        };
    }
    Ok(Some(String::new()))
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn script_retry_requires_bypass(
    dependencies: &[CacheDependency],
    cancel: Option<&CancellationToken>,
) -> crate::Result<bool> {
    use crate::ocr::processor::{script_retry, validation::TessdataEnv};

    let env = TessdataEnv::from_process();
    let mut checked = std::collections::HashSet::new();
    for model in dependencies
        .iter()
        .filter_map(|dependency| dependency.script_retry.as_ref())
    {
        check_cancelled(cancel)?;
        if checked.insert(model)
            && (checked.len() > MAX_MODEL_SELECTIONS
                || script_retry::requires_outer_cache_bypass(
                    &model.languages.join("+"),
                    model.directory.as_deref(),
                    &env,
                )
                || (model.mixed_pdf_khmer
                    && script_retry::requires_khmer_outer_cache_bypass(
                        &model.languages.join("+"),
                        model.directory.as_deref(),
                        &env,
                    )))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn check_cancelled(cancel: Option<&CancellationToken>) -> crate::Result<()> {
    if cancel.is_some_and(CancellationToken::is_cancelled) {
        Err(crate::XbergError::Cancelled)
    } else {
        Ok(())
    }
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
#[derive(Clone, PartialEq, Eq, Hash)]
struct ModelSelection {
    languages: Vec<String>,
    directory: Option<std::path::PathBuf>,
    mixed_pdf_khmer: bool,
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
impl ModelSelection {
    fn for_script_retry(config: &ExtractionConfig, mime: Option<&str>) -> Option<Self> {
        if !config.use_cache || config.effective_disable_ocr() {
            return None;
        }
        let mime = mime.map(|mime| mime.split(';').next().unwrap_or_default().trim());
        if mime.is_some_and(|mime| mime.eq_ignore_ascii_case("text/plain")) {
            return None;
        }
        let image = mime.is_some_and(|mime| {
            mime.split('/')
                .next()
                .is_some_and(|kind| kind.eq_ignore_ascii_case("image"))
        });
        let pdf = cfg!(feature = "pdf") && mime.is_some_and(|mime| mime.eq_ignore_ascii_case("application/pdf"));
        if !(image || pdf || mime.is_none_or(str::is_empty) || config.runs_ocr_on_embedded_images()) {
            return None;
        }
        let default_ocr = crate::core::config::OcrConfig::default();
        let ocr = config.ocr.as_ref().unwrap_or(&default_ocr);
        if ocr.backend != "tesseract" || ocr.pipeline.is_some() {
            return None;
        }

        // Layout region OCR can request text even when the whole-image OCR format is hOCR or TSV. ~keep
        #[cfg(feature = "layout-detection")]
        let region_ocr = (image || mime.is_none_or(str::is_empty)) && config.layout.is_some();
        #[cfg(not(feature = "layout-detection"))]
        let region_ocr = false;
        if !region_ocr
            && ocr.tesseract_config.as_ref().is_some_and(|tesseract| {
                !crate::ocr::processor::script_retry::supports_retry_format(&tesseract.output_format)
            })
        {
            return None;
        }
        let languages = ocr.effective_tesseract_language();
        let mixed_pdf_khmer = (pdf || mime.is_none_or(str::is_empty))
            && cfg!(feature = "pdf")
            && config.ocr.is_some()
            && if config.force_ocr {
                languages.as_slice() == ["eng"]
            } else {
                matches!(config.ocr_strategy, crate::core::config::OcrStrategy::Auto)
            };
        if !mixed_pdf_khmer && !crate::ocr::processor::script_retry::may_retry_language(&languages.join("+")) {
            return None;
        }
        Some(Self {
            languages,
            directory: ocr.tessdata_path.clone(),
            mixed_pdf_khmer,
        })
    }

    fn for_config(config: &ExtractionConfig, mime: Option<&str>) -> Option<Self> {
        if !config.use_cache || config.effective_disable_ocr() {
            return None;
        }
        let mime = mime
            .map(|mime| mime.split(';').next().unwrap_or_default().trim())
            .filter(|mime| !mime.is_empty());
        let pdf = mime.is_none_or(|mime| mime.eq_ignore_ascii_case("application/pdf"));
        let image = mime.is_some_and(|mime| {
            mime.split('/')
                .next()
                .is_some_and(|kind| kind.eq_ignore_ascii_case("image"))
        });
        let ocr = config.ocr.as_ref()?;
        #[cfg(feature = "pdf")]
        if ocr.pipeline.is_some() {
            return None;
        }
        if !(pdf || image) || !ocr.enabled || ocr.backend != "tesseract" {
            return None;
        }
        let explicit_osd = matches!(
            ocr.tesseract_config.as_ref().and_then(|tesseract| tesseract.psm),
            Some(0 | 1 | 12)
        );
        #[cfg(feature = "pdf")]
        let retry = pdf && crate::extractors::pdf::ocr::recognition_retry_eligible(ocr);
        #[cfg(not(feature = "pdf"))]
        let retry = false;
        if !explicit_osd && !retry {
            return None;
        }
        Some(Self {
            languages: ocr.effective_tesseract_language(),
            directory: ocr.tessdata_path.clone(),
            mixed_pdf_khmer: false,
        })
    }
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn fingerprint_models(dependencies: &[CacheDependency], cancel: Option<&CancellationToken>) -> crate::Result<String> {
    let mut revisions = std::collections::HashMap::new();
    fingerprint_models_using(dependencies, cancel, |model| {
        model_fingerprint_with_reader(model, &mut revisions, super::cacheability::osd_revision)
    })
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn fingerprint_models_using(
    dependencies: &[CacheDependency],
    cancel: Option<&CancellationToken>,
    mut read_model: impl FnMut(&ModelSelection) -> crate::Result<blake3::Hash>,
) -> crate::Result<String> {
    let mut hash = blake3::Hasher::new();
    let mut snapshots = std::collections::HashMap::new();
    hash.update(b"recognition-osd-cache-context-v4");
    for (index, dependency) in dependencies.iter().enumerate() {
        check_cancelled(cancel)?;
        for model in dependency.model.iter().chain(&dependency.stage_models) {
            check_cancelled(cancel)?;
            hash.update(&(index as u64).to_le_bytes());
            let fingerprint = if let Some(fingerprint) = snapshots.get(model) {
                *fingerprint
            } else {
                if snapshots.len() >= MAX_MODEL_SELECTIONS {
                    super::cacheability::invalidate_current();
                    return Err(crate::XbergError::validation(
                        "OSD cache dependency observation bound exceeded",
                    ));
                }
                let fingerprint = read_model(model)?;
                snapshots.insert(model.clone(), fingerprint);
                fingerprint
            };
            hash.update(fingerprint.as_bytes());
        }
    }
    Ok(hash.finalize().to_hex().to_string())
}

#[cfg(all(test, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
fn model_fingerprint(model: &ModelSelection) -> crate::Result<blake3::Hash> {
    model_fingerprint_with_reader(
        model,
        &mut std::collections::HashMap::new(),
        super::cacheability::osd_revision,
    )
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn model_fingerprint_with_reader(
    model: &ModelSelection,
    revisions: &mut std::collections::HashMap<std::path::PathBuf, blake3::Hash>,
    mut read_revision: impl FnMut(&std::path::Path) -> crate::Result<blake3::Hash>,
) -> crate::Result<blake3::Hash> {
    use crate::ocr::processor::validation::{TessdataEnv, existing_tessdata_dir_for};
    let mut hash = blake3::Hasher::new();
    let Some(directory) = existing_tessdata_dir_for(
        &model.languages,
        model.directory.as_deref(),
        &TessdataEnv::from_process(),
    ) else {
        super::cacheability::invalidate_current();
        hash.update(b"primary-language-unavailable");
        return Ok(hash.finalize());
    };
    let directory = std::fs::canonicalize(directory)?;
    let path = directory.as_os_str().as_encoded_bytes();
    hash.update(&(path.len() as u64).to_le_bytes());
    hash.update(path);
    let revision = if let Some(revision) = revisions.get(&directory) {
        *revision
    } else {
        let revision = read_revision(&directory)?;
        revisions.insert(directory, revision);
        revision
    };
    hash.update(revision.as_bytes());
    for language in &model.languages {
        let language = language.trim();
        let model_path = directory.join(format!("{language}.traineddata"));
        let digest = crate::cache::blake3_hash_file(&model_path).map_err(|error| {
            crate::XbergError::cache(format!(
                "Cannot fingerprint primary model '{}': {error}",
                model_path.display()
            ))
        })?;
        hash.update(&(language.len() as u64).to_le_bytes());
        hash.update(language.as_bytes());
        hash.update(digest.as_bytes());
    }
    Ok(hash.finalize())
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
const MAX_MODEL_SELECTIONS: usize = 64;

#[cfg(all(feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
impl CacheDependency {
    fn collect_stage_models(&mut self, config: &ExtractionConfig, mime: Option<&str>) {
        if !config.use_cache || config.effective_disable_ocr() {
            return;
        }
        let Some(ocr) = config.ocr.as_ref().filter(|ocr| ocr.enabled) else {
            return;
        };
        let Some(pipeline) = &ocr.pipeline else {
            return;
        };
        let supported = mime.is_none_or(|mime| {
            let mime = mime.split(';').next().unwrap_or_default().trim();
            mime.is_empty()
                || mime.eq_ignore_ascii_case("application/pdf")
                || mime
                    .split('/')
                    .next()
                    .is_some_and(|kind| kind.eq_ignore_ascii_case("image"))
        });
        if !supported {
            return;
        }
        if pipeline.stages.len() > MAX_MODEL_SELECTIONS {
            self.uncertifiable = true;
            return;
        }
        for stage in &pipeline.stages {
            let inputs = crate::extractors::pdf::ocr::stage_model_inputs::StageModelInputs::new(ocr, stage);
            if inputs.backend() != "tesseract"
                || !matches!(inputs.tesseract().and_then(|tesseract| tesseract.psm), Some(0 | 1 | 12))
            {
                continue;
            }
            let model = ModelSelection {
                languages: inputs.languages(),
                directory: inputs.directory().map(std::path::Path::to_path_buf),
                mixed_pdf_khmer: false,
            };
            if !self.stage_models.contains(&model) {
                self.stage_models.push(model);
            }
        }
    }
}

#[cfg(all(test, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::core::config::OcrConfig;

    fn configuration(directory: &std::path::Path) -> ExtractionConfig {
        ExtractionConfig {
            ocr: Some(OcrConfig {
                tessdata_path: Some(directory.into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    async fn orientation_context(config: &ExtractionConfig) -> Option<String> {
        let mut dependency = CacheDependency::for_config(config, Some("application/pdf"));
        dependency.script_retry = None;
        fingerprint(&[dependency], None).await.unwrap()
    }

    #[tokio::test]
    async fn model_context_changes_with_presence_equal_length_content_and_primary_root() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("eng.traineddata"), b"language fixture").unwrap();
        let config = configuration(root.path());
        let missing = orientation_context(&config).await;
        std::fs::write(root.path().join("osd.traineddata"), b"model first").unwrap();
        let present = orientation_context(&config).await;
        assert_ne!(missing, present);
        assert_eq!(present, orientation_context(&config).await);
        std::fs::write(root.path().join("osd.traineddata"), b"model other").unwrap();
        let changed = orientation_context(&config).await;
        assert_ne!(present, changed, "equal-length model bytes must affect result identity");
        let other = tempfile::tempdir().unwrap();
        for name in ["eng", "osd"] {
            std::fs::copy(
                root.path().join(format!("{name}.traineddata")),
                other.path().join(format!("{name}.traineddata")),
            )
            .unwrap();
        }
        assert_ne!(changed, orientation_context(&configuration(other.path())).await);
    }

    #[tokio::test]
    async fn explicit_policies_and_non_pdf_inputs_do_not_read_model_artifacts() {
        let root = tempfile::tempdir().unwrap();
        let mut config = configuration(root.path());
        config
            .ocr
            .as_mut()
            .unwrap()
            .tesseract_config
            .get_or_insert_default()
            .psm = Some(11);
        assert_eq!(orientation_context(&config).await.as_deref(), Some(""));
        config.ocr.as_mut().unwrap().tesseract_config = None;
        for explicit in [Some(true), Some(false)] {
            config.ocr.as_mut().unwrap().auto_rotate = explicit;
            assert!(
                CacheDependency::for_config(&config, Some("application/pdf"))
                    .model
                    .is_none(),
                "{explicit:?} never reads OSD"
            );
            assert_eq!(orientation_context(&config).await.as_deref(), Some(""));
        }
        config.ocr.as_mut().unwrap().auto_rotate = None;
        assert!(
            CacheDependency::for_config(&config, Some("application/pdf"))
                .model
                .is_some(),
            "an unset policy depends on the OSD pack"
        );
        config.ocr.as_mut().unwrap().language = vec!["jpn_vert".into()];
        assert!(
            CacheDependency::for_config(&config, Some("application/pdf"))
                .model
                .is_none()
        );
        assert_eq!(orientation_context(&config).await.as_deref(), Some(""));
        config.ocr.as_mut().unwrap().language = vec!["eng".into()];
        config.ocr_embedded_images = Some(false);
        let dependency = CacheDependency::for_config(&config, Some("text/plain"));
        assert!(dependency.model.is_none());
        assert_eq!(fingerprint(&[dependency], None).await.unwrap().as_deref(), Some(""));
        config.disable_ocr = true;
        assert_eq!(orientation_context(&config).await.as_deref(), Some(""));
        config.disable_ocr = false;
        config.ocr.as_mut().unwrap().enabled = false;
        assert!(
            CacheDependency::for_config(&config, Some("application/pdf"))
                .model
                .is_none()
        );
        assert_eq!(orientation_context(&config).await.as_deref(), Some(""));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unreadable_model_state_bypasses_caching_instead_of_becoming_stable_missing_osd() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("eng.traineddata"), b"language fixture").unwrap();
        let config = configuration(root.path());
        assert!(orientation_context(&config).await.is_some());
        std::os::unix::fs::symlink("osd.traineddata", root.path().join("osd.traineddata")).unwrap();
        assert!(orientation_context(&config).await.is_none());
    }

    #[tokio::test]
    async fn cancellation_precedes_model_lookup_and_invalidates_cache_work() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        let config = configuration(std::path::Path::new("/no-models-may-be-read"));
        assert!(matches!(
            fingerprint(&[CacheDependency::for_config(&config, None)], Some(&cancel)).await,
            Err(crate::XbergError::Cancelled)
        ));
    }

    #[test]
    fn context_application_preserves_independent_keys_and_bypasses_unidentified_models() {
        assert_eq!(
            apply_context(Some("original".into()), &Some(String::new())).as_deref(),
            Some("original")
        );
        assert!(apply_context(Some("original".into()), &None).is_none());
        assert!(apply_context(None, &Some("model".into())).is_none());
        assert_ne!(
            apply_context(Some("original".into()), &Some("first".into())),
            apply_context(Some("original".into()), &Some("other".into()))
        );
    }
    #[test]
    fn snapshot_local_dedup_hashes_each_distinct_model_once_and_respects_input_order() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        for (directory, bytes) in [(first.path(), b"first"), (second.path(), b"other")] {
            std::fs::write(directory.join("eng.traineddata"), b"language fixture").unwrap();
            std::fs::write(directory.join("osd.traineddata"), bytes).unwrap();
        }
        let dependency = |path| CacheDependency::for_config(&configuration(path), Some("application/pdf"));
        let first = dependency(first.path());
        let second = dependency(second.path());
        let mut reads = 0;
        fingerprint_models_using(&vec![first.clone(); 10], None, |model| {
            reads += 1;
            model_fingerprint(model)
        })
        .unwrap();
        assert_eq!(
            reads, 1,
            "ten identical dependencies require one actual streamed file hash"
        );
        reads = 0;
        let ordered = fingerprint_models_using(&[first.clone(), second.clone()], None, |model| {
            reads += 1;
            model_fingerprint(model)
        })
        .unwrap();
        assert_eq!(reads, 2, "different primary roots require separate hashes");
        assert_ne!(ordered, fingerprint_models(&[second, first.clone()], None).unwrap());
        let before = fingerprint_models(std::slice::from_ref(&first), None).unwrap();
        std::fs::write(
            first
                .model
                .as_ref()
                .unwrap()
                .directory
                .as_ref()
                .unwrap()
                .join("osd.traineddata"),
            b"other",
        )
        .unwrap();
        assert_ne!(
            before,
            fingerprint_models(&[first], None).unwrap(),
            "a new snapshot must re-read changed model bytes"
        );
    }
    #[test]
    fn cancellation_while_queued_prevents_the_model_job_and_all_stream_reads() {
        use std::future::Future;
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
            mpsc,
        };
        let root = tempfile::tempdir().unwrap();
        for name in ["eng", "osd"] {
            std::fs::write(root.path().join(format!("{name}.traineddata")), b"model fixture").unwrap();
        }
        let dependencies = [CacheDependency::for_config(
            &configuration(root.path()),
            Some("application/pdf"),
        )];
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(1)
            .build()
            .unwrap();
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let blocker = runtime.spawn_blocking(move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        started_rx.recv().unwrap();
        let cancel = CancellationToken::new();
        let jobs = Arc::new(AtomicUsize::new(0));
        let reads = Arc::new(AtomicUsize::new(0));
        let (worker_jobs, worker_reads) = (Arc::clone(&jobs), Arc::clone(&reads));
        let result = runtime.block_on(async {
            let mut queued = Box::pin(fingerprint_with_reader(
                &dependencies,
                Some(&cancel),
                move |models, token| {
                    worker_jobs.fetch_add(1, Ordering::SeqCst);
                    fingerprint_models_using(models, token, |model| {
                        worker_reads.fetch_add(1, Ordering::SeqCst);
                        model_fingerprint(model)
                    })
                },
            ));
            std::future::poll_fn(|context| {
                assert!(
                    queued.as_mut().poll(context).is_pending(),
                    "the model job must queue behind the occupied worker"
                );
                std::task::Poll::Ready(())
            })
            .await;
            cancel.cancel();
            release_tx.send(()).unwrap();
            blocker.await.unwrap();
            queued.await
        });
        assert!(matches!(result, Err(crate::XbergError::Cancelled)));
        assert_eq!(
            jobs.load(Ordering::SeqCst),
            0,
            "queued cancellation must precede all model-job I/O"
        );
        assert_eq!(
            reads.load(Ordering::SeqCst),
            0,
            "a cancelled queued job must never read the model"
        );
    }

    #[test]
    fn cancellation_after_one_distinct_model_prevents_reading_the_next_model() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        for directory in [first.path(), second.path()] {
            for name in ["eng", "osd"] {
                std::fs::write(directory.join(format!("{name}.traineddata")), b"model fixture").unwrap();
            }
        }
        let dependencies = [first.path(), second.path()]
            .map(|directory| CacheDependency::for_config(&configuration(directory), Some("application/pdf")));
        let cancel = CancellationToken::new();
        let mut reads = 0;
        let result = fingerprint_models_using(&dependencies, Some(&cancel), |model| {
            reads += 1;
            let result = model_fingerprint(model);
            cancel.cancel();
            result
        });
        assert!(matches!(result, Err(crate::XbergError::Cancelled)));
        assert_eq!(reads, 1, "cancellation must prevent the second distinct model read");
    }
}

#[cfg(all(test, feature = "ocr", not(target_arch = "wasm32")))]
mod explicit_mode_tests {
    use super::*;
    use crate::core::config::OcrConfig;

    fn config(psm: Option<i32>) -> ExtractionConfig {
        ExtractionConfig {
            ocr: Some(OcrConfig {
                tesseract_config: Some(crate::types::TesseractConfig {
                    psm,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn outer_cache_rejects_legacy_retry_entry_after_model_removal() {
        let root = tempfile::tempdir().unwrap();
        let language = "xberg_retry_cache_fixture";
        let mut config = config(Some(3));
        let ocr = config.ocr.as_mut().unwrap();
        ocr.language = vec![language.into()];
        ocr.tessdata_path = Some(root.path().into());
        let dependency = CacheDependency::for_config(&config, Some("image/png"));
        let context = || fingerprint(std::slice::from_ref(&dependency), None);
        assert!(config.runs_ocr_on_embedded_images());
        let plain = CacheDependency::for_config(&config, Some("Text/Plain; charset=utf-8"));

        std::fs::write(root.path().join(format!("{language}.traineddata")), b"primary").unwrap();
        std::fs::write(root.path().join("osd.traineddata"), b"orientation").unwrap();
        let missing = context().await.unwrap();
        let legacy_key = "warm";
        let cache = crate::cache::GenericCache::new(
            "extraction".into(),
            Some(root.path().to_str().unwrap().into()),
            30.0,
            500.0,
            1000.0,
        )
        .unwrap();
        let legacy_result = "汉字 from the old retry".as_bytes().to_vec();
        cache.set_default(legacy_key, legacy_result.clone(), None).unwrap();

        let han = root.path().join("chi_sim.traineddata");
        for bytes in [b"han-v1", b"han-v2"] {
            std::fs::write(&han, bytes).unwrap();
            super::super::cacheability::scope(async {
                assert!(apply_context(Some("warm".into()), &context().await.unwrap()).is_none());
                let plain_context = fingerprint(std::slice::from_ref(&plain), None).await.unwrap();
                assert_eq!(
                    apply_context(Some("warm".into()), &plain_context).as_deref(),
                    Some("warm")
                );
                assert!(
                    super::super::cacheability::is_cacheable(),
                    "the inner cache can still be used"
                );
            })
            .await;
        }

        let mut no_retry = config.clone();
        no_retry.ocr.as_mut().unwrap().language = vec!["chi_sim".into()];
        let no_retry = CacheDependency::for_config(&no_retry, Some("image/png"));
        assert_eq!(fingerprint(&[no_retry], None).await.unwrap().as_deref(), Some(""));

        std::fs::remove_file(han).unwrap();
        let after_removal = context().await.unwrap();
        assert_eq!(after_removal, missing);
        let current_key = apply_context(Some(legacy_key.into()), &after_removal)
            .expect("an image without the retry models remains cacheable");
        assert_eq!(cache.get_default(legacy_key, None).unwrap(), Some(legacy_result));
        assert_eq!(cache.get_default(&current_key, None).unwrap(), None);
    }

    #[tokio::test]
    async fn image_osd_identity_observes_equal_length_revisions_without_pdf_support() {
        let root = tempfile::tempdir().unwrap();
        let language = "xberg_osd_cache_fixture";
        std::fs::write(root.path().join(format!("{language}.traineddata")), b"language pack").unwrap();
        let osd = root.path().join("osd.traineddata");
        for psm in [0, 1, 12] {
            let mut config = config(Some(psm));
            let ocr = config.ocr.as_mut().unwrap();
            ocr.language = vec![language.into()];
            ocr.tessdata_path = Some(root.path().into());
            let dependency = CacheDependency::for_config(&config, Some("image/png"));
            std::fs::write(&osd, b"revision one").unwrap();
            let first = fingerprint(std::slice::from_ref(&dependency), None).await.unwrap();
            std::fs::write(&osd, b"revision two").unwrap();
            let second = fingerprint(std::slice::from_ref(&dependency), None).await.unwrap();
            assert_ne!(first, second);
            assert_eq!(second, fingerprint(&[dependency], None).await.unwrap());
        }
    }

    #[tokio::test]
    async fn non_osd_image_modes_keep_empty_context_without_model_io() {
        for psm in [None, Some(3), Some(6)] {
            let mut config = config(psm);
            let ocr = config.ocr.as_mut().unwrap();
            ocr.tesseract_config.as_mut().unwrap().output_format = "hocr".into();
            ocr.tessdata_path = Some("unavailable-model-root".into());
            let dependency = CacheDependency::for_config(&config, Some("image/png"));
            assert!(dependency.model.is_none());
            assert_eq!(fingerprint(&[dependency], None).await.unwrap().as_deref(), Some(""));
        }
    }

    #[test]
    fn explicit_osd_modes_apply_to_pdf_and_images_but_not_other_backends() {
        for psm in [0, 1, 12] {
            let mut config = config(Some(psm));
            for mime in ["application/pdf", "image/png", "IMAGE/JPEG; subtype=baseline"] {
                assert!(CacheDependency::for_config(&config, Some(mime)).model.is_some());
            }
            config.ocr.as_mut().unwrap().language = vec!["jpn_vert".into()];
            assert!(CacheDependency::for_config(&config, Some("image/png")).model.is_some());
            config.ocr.as_mut().unwrap().backend = "paddleocr".into();
            assert!(CacheDependency::for_config(&config, Some("image/png")).model.is_none());
        }
        let mut vertical = config(None);
        vertical.ocr.as_mut().unwrap().language = vec!["jpn_vert".into()];
        assert!(
            CacheDependency::for_config(&vertical, Some("image/png"))
                .model
                .is_none()
        );
        let config = config(Some(1));
        assert!(CacheDependency::for_config(&config, Some("text/plain")).model.is_none());
    }
}

#[cfg(all(test, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
#[path = "cache_context/stage_tests.rs"]
mod stage_tests;
