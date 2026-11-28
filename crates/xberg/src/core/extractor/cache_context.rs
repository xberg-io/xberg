//! Mutable model dependencies of OSD recognition, script retry, optional PDF recovery and the
//! standalone-image PaddleOCR fallback.

use crate::cancellation::{CancellationToken, check_cancelled};
use crate::core::config::ExtractionConfig;

#[derive(Clone, Default)]
pub(crate) struct CacheDependency {
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    model: Option<ModelSelection>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    stage_models: Vec<ModelSelection>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    script_retry: Option<ModelSelection>,
    /// The Tesseract models a request may read when OCR runs. Their resolved directory and the
    /// identity of their files enter the key, so text read with one set of model files is never
    /// served for another.
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    tessdata: Vec<ModelSelection>,
    #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
    uncertifiable: bool,
    /// The OCR configuration of a request the image PaddleOCR fallback may run for; its model
    /// files' identity enters the key, so cold and warm model caches never share an entry.
    #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
    paddle_image_fallback: Option<crate::core::config::OcrConfig>,
}

impl CacheDependency {
    pub(crate) fn for_config(config: &ExtractionConfig, mime: Option<&str>) -> Self {
        #[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
        {
            let dependency = Self {
                model: ModelSelection::for_config(config, mime),
                script_retry: ModelSelection::for_script_retry(config, mime),
                tessdata: ModelSelection::for_tessdata(config, mime),
                #[cfg(all(paddle_ocr, feature = "ocr", feature = "pdf", not(target_arch = "wasm32")))]
                paddle_image_fallback: crate::extractors::image::paddle_fallback_model_request(config, mime),
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
    let has_paddle_fallback = paddle_fallback_requested(dependencies);
    let has_tessdata = dependencies.iter().any(|dependency| !dependency.tessdata.is_empty());
    // ~keep Tessdata identities and the script-retry context are stat calls once the memos hold
    // the packs' digests and the vertical model's revision, so they skip the blocking-pool hop
    // every cached PDF and image would pay. Orientation and PaddleOCR models are always read, and
    // off Unix so is every pack, since no memo is kept there.
    if (has_tessdata || has_script_retry)
        && !(has_orientation_models || has_paddle_fallback)
        && let Some(result) =
            crate::ocr::tessdata_download::without_reading_packs(|| retry_and_tessdata_context(dependencies, cancel))
    {
        return match result {
            Ok(Identified::Context(context)) => Ok(Some(context)),
            Ok(Identified::Bypass) => Ok(None),
            Ok(Identified::Unidentified) => Ok(unidentified_model()),
            Err(crate::XbergError::Cancelled) => Err(crate::XbergError::Cancelled),
            Err(error) => {
                super::cacheability::invalidate_current();
                tracing::warn!(%error, "Cannot identify the Tesseract model files; bypassing result cache");
                Ok(None)
            }
        };
    }
    if has_orientation_models || has_script_retry || has_paddle_fallback || has_tessdata {
        let dependencies = dependencies.to_vec();
        let worker_cancel = cancel.cloned();
        let result = tokio::task::spawn_blocking(super::cacheability::blocking(move || {
            check_cancelled(worker_cancel.as_ref())?;
            // Retry models can appear or change during extraction; check again before an outer write. ~keep
            let Some(retry_context) = script_retry_cache_context(&dependencies, worker_cancel.as_ref())? else {
                return Ok(None);
            };
            let mut context = if has_orientation_models {
                read_models(&dependencies, worker_cancel.as_ref())?
            } else {
                String::new()
            };
            context.push_str(&retry_context);
            context.push_str(&paddle_fallback_context(&dependencies));
            let Some(tessdata) = tessdata_context(&dependencies, worker_cancel.as_ref())? else {
                return Ok(unidentified_model());
            };
            context.push_str(&tessdata);
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
enum Identified {
    Context(String),
    /// The script retry may read models the outer key cannot follow; nothing is cached.
    Bypass,
    /// A model file cannot be identified.
    Unidentified,
}

/// The context of requests with no orientation or PaddleOCR models: the script-retry context,
/// then the tessdata identities, as the blocking path joins them.
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn retry_and_tessdata_context(
    dependencies: &[CacheDependency],
    cancel: Option<&CancellationToken>,
) -> crate::Result<Identified> {
    let Some(mut context) = script_retry_cache_context(dependencies, cancel)? else {
        return Ok(Identified::Bypass);
    };
    let Some(tessdata) = tessdata_context(dependencies, cancel)? else {
        return Ok(Identified::Unidentified);
    };
    context.push_str(&tessdata);
    Ok(Identified::Context(context))
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn paddle_fallback_requested(dependencies: &[CacheDependency]) -> bool {
    #[cfg(all(paddle_ocr, feature = "pdf"))]
    {
        dependencies
            .iter()
            .any(|dependency| dependency.paddle_image_fallback.is_some())
    }
    #[cfg(not(all(paddle_ocr, feature = "pdf")))]
    {
        let _ = dependencies;
        false
    }
}

/// The PaddleOCR model state of every request the image fallback may run for, or nothing when
/// none may. Stats the model files on the caller's blocking thread; never reads or downloads.
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn paddle_fallback_context(dependencies: &[CacheDependency]) -> String {
    #[cfg(all(paddle_ocr, feature = "pdf"))]
    {
        let mut hash = blake3::Hasher::new();
        let mut any = false;
        for ocr in dependencies
            .iter()
            .filter_map(|dependency| dependency.paddle_image_fallback.as_ref())
        {
            any = true;
            hash.update(crate::extractors::image::paddle_fallback_model_state(ocr).as_bytes());
        }
        if any {
            format!(":paddle-image-fallback-v1:{}", hash.finalize().to_hex())
        } else {
            String::new()
        }
    }
    #[cfg(not(all(paddle_ocr, feature = "pdf")))]
    {
        let _ = dependencies;
        String::new()
    }
}

/// The resolved directory and model-file identity of every Tesseract selection the requests may
/// run OCR with, or nothing when none may. `None` when a model file cannot be identified.
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn tessdata_context(
    dependencies: &[CacheDependency],
    cancel: Option<&CancellationToken>,
) -> crate::Result<Option<String>> {
    use crate::ocr::processor::validation::TessdataEnv;

    let mut hash = blake3::Hasher::new();
    let mut any = false;
    for (index, dependency) in dependencies.iter().enumerate() {
        for selection in &dependency.tessdata {
            check_cancelled(cancel)?;
            any = true;
            hash.update(&(index as u64).to_le_bytes());
            let env = TessdataEnv::from_process().for_model(selection.tessdata_model);
            if !hash_tessdata_selection(&mut hash, selection, &env) {
                return Ok(None);
            }
        }
    }
    Ok(Some(if any {
        format!(":tessdata-v1:{}", hash.finalize().to_hex())
    } else {
        String::new()
    }))
}

/// Make the operation uncacheable because a model file cannot be identified, and return the
/// context that says so.
///
/// A platform that reports no modification time does so for every file and request, so the
/// warning is given once per process; the results stay uncached every time.
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn unidentified_model() -> Option<String> {
    static WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    super::cacheability::invalidate_current();
    if !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        tracing::warn!(
            "Cannot identify a Tesseract model file (no modification time, or unreadable); results that \
             may read it are not cached"
        );
    }
    None
}

/// Hash the model set of `selection`, the directory that serves it and the identity
/// ([`hash_pack_identities`](crate::ocr::processor::validation::hash_pack_identities)) of each of
/// its model files there. Returns false when a model file cannot be identified, so the key cannot
/// be trusted.
///
/// Stats files: it never creates a directory or downloads, and reads a model only to identify it
/// (a pinned-size pack against its pin, and off Unix any other pack by its bytes), on Unix once
/// per file version, since it runs for requests that may not reach OCR at all. A selection that no directory serves yet (the engine
/// would download its packs) is keyed on its set, languages, candidate directories and the
/// identity of the packs already in them instead; once the packs exist the key changes, so
/// nothing read before is served after.
#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
#[must_use]
fn hash_tessdata_selection(
    hash: &mut blake3::Hasher,
    selection: &ModelSelection,
    env: &crate::ocr::processor::validation::TessdataEnv,
) -> bool {
    use crate::ocr::processor::validation::{
        canonical_tessdata_dir, existing_tessdata_dir_for, hash_pack_identities, tessdata_search_dirs,
    };

    fn frame(hash: &mut blake3::Hasher, bytes: &[u8]) {
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }

    frame(hash, selection.tessdata_model.as_str().as_bytes());
    let languages: Vec<&str> = selection
        .languages
        .iter()
        .flat_map(|entry| entry.split('+'))
        .map(str::trim)
        .collect();
    // ~keep The engine expands the wildcard to every model installed beside English.
    let wildcard = matches!(languages.as_slice(), [all] if *all == "*" || all.eq_ignore_ascii_case("all"));
    let required: Vec<String> = if wildcard {
        vec![crate::core::config::ocr::DEFAULT_OCR_LANGUAGE.to_owned()]
    } else {
        languages.iter().map(|language| (*language).to_owned()).collect()
    };
    let override_path = selection.directory.as_deref();
    let Some(directory) = existing_tessdata_dir_for(&required, override_path, env) else {
        hash.update(&[0]);
        let mut identified = true;
        for candidate in tessdata_search_dirs(override_path, env) {
            frame(hash, candidate.as_bytes());
            // ~keep A wrong pack in a verified directory that cannot be moved aside serves no key
            // here but is read as given, so its identity has to follow the file.
            identified &= hash_pack_identities(
                hash,
                std::path::Path::new(&candidate),
                required.iter().map(String::as_str),
            );
        }
        for language in &languages {
            frame(hash, language.as_bytes());
        }
        return identified;
    };
    let directory = canonical_tessdata_dir(&directory);
    hash.update(&[1]);
    frame(hash, directory.as_os_str().as_encoded_bytes());
    if wildcard {
        let mut installed: Vec<String> = std::fs::read_dir(&directory)
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let name = entry.ok()?.file_name().into_string().ok()?;
                name.strip_suffix(".traineddata").map(str::to_owned)
            })
            .collect();
        installed.sort();
        hash_pack_identities(hash, &directory, installed.iter().map(String::as_str))
    } else {
        hash_pack_identities(hash, &directory, languages.iter().copied())
    }
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
fn script_retry_cache_context(
    dependencies: &[CacheDependency],
    cancel: Option<&CancellationToken>,
) -> crate::Result<Option<String>> {
    use crate::ocr::processor::{script_retry, validation::TessdataEnv};

    let mut checked = std::collections::HashSet::new();
    let mut vertical = blake3::Hasher::new();
    for model in dependencies
        .iter()
        .filter_map(|dependency| dependency.script_retry.as_ref())
    {
        check_cancelled(cancel)?;
        if !checked.insert(model) {
            continue;
        }
        let env = TessdataEnv::from_process().for_model(model.tessdata_model);
        let languages = model.languages.join("+");
        let directory = model.directory.as_deref();
        if checked.len() > MAX_MODEL_SELECTIONS
            || script_retry::requires_outer_cache_bypass(&languages, directory, &env)
            || (model.mixed_pdf_khmer && script_retry::requires_khmer_outer_cache_bypass(&languages, directory, &env))
        {
            return Ok(None);
        }
        let revision = if model.vertical_japanese_retry {
            script_retry::outer_vertical_model_fingerprint(&languages, directory, &env)?
        } else {
            None
        };
        if let Some(revision) = revision {
            vertical.update(&[1]);
            vertical.update(revision.as_bytes());
        } else {
            vertical.update(&[0]);
        }
    }
    if checked.is_empty() {
        return Ok(Some(String::new()));
    }
    Ok(Some(format!(":script-retry-outer-v2:{}", vertical.finalize().to_hex())))
}

#[cfg(all(feature = "ocr", not(target_arch = "wasm32")))]
#[derive(Clone, PartialEq, Eq, Hash)]
struct ModelSelection {
    languages: Vec<String>,
    directory: Option<std::path::PathBuf>,
    tessdata_model: crate::core::config::TessdataModel,
    mixed_pdf_khmer: bool,
    vertical_japanese_retry: bool,
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
        let default_ocr = config.ocr_or_default();
        let ocr: &crate::core::config::OcrConfig = &default_ocr;
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
            tessdata_model: ocr.effective_tessdata_model(),
            mixed_pdf_khmer,
            vertical_japanese_retry: ocr
                .tesseract_config
                .as_ref()
                .and_then(|tesseract| tesseract.psm)
                .is_none_or(|psm| matches!(psm, 3 | 11)),
        })
    }

    /// The Tesseract models the request may read when OCR runs: those of the `ocr` section, or of
    /// the default one OCR runs with when there is none, and of each Tesseract pipeline stage.
    /// Empty when OCR cannot run for the request. Read from the config alone.
    ///
    /// The backend is not asked: other backends still read with Tesseract for orientation and
    /// recovery probes, and a selection whose models are never used only costs a few `stat` calls.
    fn for_tessdata(config: &ExtractionConfig, mime: Option<&str>) -> Vec<Self> {
        if !config.use_cache || config.effective_disable_ocr() {
            return Vec::new();
        }
        let mime = mime.map(|mime| mime.split(';').next().unwrap_or_default().trim());
        if mime.is_some_and(|mime| mime.eq_ignore_ascii_case("text/plain")) {
            return Vec::new();
        }
        let image = mime.is_some_and(|mime| {
            mime.split('/')
                .next()
                .is_some_and(|kind| kind.eq_ignore_ascii_case("image"))
        });
        let pdf = cfg!(feature = "pdf") && mime.is_some_and(|mime| mime.eq_ignore_ascii_case("application/pdf"));
        if !(image || pdf || mime.is_none_or(str::is_empty) || config.runs_ocr_on_embedded_images()) {
            return Vec::new();
        }
        let default_ocr = config.ocr_or_default();
        let ocr: &crate::core::config::OcrConfig = &default_ocr;
        let tessdata_model = ocr.effective_tessdata_model();
        let selection = move |languages, directory| Self {
            languages,
            directory,
            tessdata_model,
            mixed_pdf_khmer: false,
            vertical_japanese_retry: false,
        };
        #[cfg_attr(not(feature = "pdf"), allow(unused_mut))]
        let mut selections = vec![selection(ocr.effective_tesseract_language(), ocr.tessdata_path.clone())];
        #[cfg(feature = "pdf")]
        if let Some(pipeline) = &ocr.pipeline {
            for stage in pipeline.stages.iter().take(MAX_MODEL_SELECTIONS) {
                let inputs = crate::extractors::pdf::ocr::stage_model_inputs::StageModelInputs::new(ocr, stage);
                if inputs.backend() != "tesseract" {
                    continue;
                }
                let stage = selection(inputs.languages(), inputs.directory().map(std::path::Path::to_path_buf));
                if !selections.contains(&stage) {
                    selections.push(stage);
                }
            }
        }
        selections
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
            tessdata_model: config.effective_tessdata_model(),
            mixed_pdf_khmer: false,
            vertical_japanese_retry: false,
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
        &TessdataEnv::from_process().for_model(model.tessdata_model),
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
        revisions.insert(directory.clone(), revision);
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
                tessdata_model: config.effective_tessdata_model(),
                mixed_pdf_khmer: false,
                vertical_japanese_retry: false,
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
        dependency.tessdata.clear();
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

    #[test]
    fn every_request_that_may_run_ocr_selects_its_tesseract_models() {
        let selected = |config: &ExtractionConfig, mime: Option<&str>| {
            CacheDependency::for_config(config, mime)
                .tessdata
                .iter()
                .map(|selection| (selection.languages.join("+"), selection.tessdata_model))
                .collect::<Vec<_>>()
        };
        let fast = crate::core::config::TessdataModel::Fast;
        let best = crate::core::config::TessdataModel::Best;
        let docx = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

        let default = ExtractionConfig::default();
        for mime in [
            Some("application/pdf"),
            Some("image/png"),
            Some("IMAGE/JPEG; q=1"),
            None,
        ] {
            assert_eq!(selected(&default, mime), [("eng".to_string(), fast)], "{mime:?}");
        }
        assert!(selected(&default, Some(docx)).is_empty(), "no OCR, no model dependency");
        assert!(selected(&default, Some("text/plain")).is_empty());

        let top_level = ExtractionConfig {
            tessdata_model: Some(best),
            ..Default::default()
        };
        assert_eq!(
            selected(&top_level, Some("application/pdf")),
            [("eng".to_string(), best)],
            "the inherited set reaches a request with no ocr section"
        );

        let mut with_section = ExtractionConfig {
            ocr: Some(OcrConfig {
                language: vec!["deu".into(), "fra".into()],
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(selected(&with_section, Some(docx)), [("deu+fra".to_string(), fast)]);
        with_section.use_cache = false;
        assert!(selected(&with_section, Some("image/png")).is_empty());
        with_section.use_cache = true;
        with_section.disable_ocr = true;
        assert!(selected(&with_section, Some("image/png")).is_empty());
    }

    #[test]
    fn tessdata_context_follows_the_model_files_without_creating_or_reading_anything() {
        use crate::ocr::processor::validation::TessdataEnv;

        let models = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let english = models.path().join("eng.traineddata");
        std::fs::write(&english, b"first model").unwrap();
        let selection = |tessdata_model| ModelSelection {
            languages: vec!["eng".into()],
            directory: None,
            tessdata_model,
            mixed_pdf_khmer: false,
            vertical_japanese_retry: false,
        };
        let env = |prefix: Option<String>, tessdata_model| TessdataEnv {
            tessdata_prefix: prefix,
            cache_dir: Some(cache.path().into()),
            system_paths: false,
            model: tessdata_model,
        };
        let context = |prefix: Option<String>, tessdata_model| {
            let mut hash = blake3::Hasher::new();
            assert!(hash_tessdata_selection(
                &mut hash,
                &selection(tessdata_model),
                &env(prefix, tessdata_model)
            ));
            hash.finalize()
        };
        let fast = crate::core::config::TessdataModel::Fast;
        let best = crate::core::config::TessdataModel::Best;
        let prefix = || Some(models.path().to_str().unwrap().to_owned());

        let through_prefix = context(prefix(), best);
        assert_eq!(through_prefix, context(prefix(), best));
        assert_eq!(
            through_prefix,
            context(
                prefix().map(|path| format!("{path}{}", std::path::MAIN_SEPARATOR)),
                best
            ),
            "one directory, one context"
        );
        assert_ne!(
            through_prefix,
            context(prefix(), fast),
            "the requested set is part of the context"
        );

        let unset = context(None, best);
        assert_ne!(
            through_prefix, unset,
            "text read through TESSDATA_PREFIX must not be served once the prefix is unset"
        );
        assert_eq!(unset, context(None, best));
        assert_ne!(unset, context(None, fast));
        assert_eq!(
            cache.path().read_dir().unwrap().count(),
            0,
            "computing a key must not create cache directories or download"
        );

        // ~keep A different length: two writes within one clock tick can share a modification time.
        std::fs::write(&english, b"a replaced model").unwrap();
        assert_ne!(through_prefix, context(prefix(), best), "a pack replaced in place");
    }

    #[test]
    fn a_wrong_best_pack_served_as_given_is_part_of_the_tessdata_context() {
        use crate::core::config::TessdataModel;
        use crate::ocr::processor::validation::{TessdataEnv, model_cache_dir};

        let cache = tempfile::tempdir().unwrap();
        let managed = model_cache_dir(cache.path(), TessdataModel::Best);
        std::fs::create_dir_all(&managed).unwrap();
        let pack = managed.join("eng.traineddata");
        std::fs::write(&pack, b"not the pinned best pack").unwrap();
        let selection = ModelSelection {
            languages: vec!["eng".into()],
            directory: None,
            tessdata_model: TessdataModel::Best,
            mixed_pdf_khmer: false,
            vertical_japanese_retry: false,
        };
        let env = TessdataEnv {
            tessdata_prefix: None,
            cache_dir: Some(cache.path().into()),
            system_paths: false,
            model: TessdataModel::Best,
        };
        assert!(
            crate::ocr::processor::validation::existing_tessdata_dir_for(&["eng".to_owned()], None, &env).is_none(),
            "a wrong pack serves no verified directory"
        );
        let context = || {
            let mut hash = blake3::Hasher::new();
            assert!(hash_tessdata_selection(&mut hash, &selection, &env));
            hash.finalize()
        };

        let first = context();
        assert_eq!(first, context());
        // ~keep A different length: two writes within one clock tick can share a modification time.
        std::fs::write(&pack, b"another pack that is not the pinned one").unwrap();
        assert_ne!(
            first,
            context(),
            "text read from the pack used as given must not be served once it changes"
        );
    }

    #[test]
    fn wildcard_tessdata_context_follows_every_installed_model() {
        use crate::ocr::processor::validation::TessdataEnv;

        let models = tempfile::tempdir().unwrap();
        std::fs::write(models.path().join("eng.traineddata"), b"english").unwrap();
        let selection = ModelSelection {
            languages: vec!["all".into()],
            directory: Some(models.path().into()),
            tessdata_model: Default::default(),
            mixed_pdf_khmer: false,
            vertical_japanese_retry: false,
        };
        let context = || {
            let mut hash = blake3::Hasher::new();
            assert!(hash_tessdata_selection(
                &mut hash,
                &selection,
                &TessdataEnv::from_lookup(|_| None)
            ));
            hash.finalize()
        };

        let english_only = context();
        std::fs::write(models.path().join("deu.traineddata"), b"german").unwrap();
        assert_ne!(
            english_only,
            context(),
            "an added model changes what the wildcard reads"
        );
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
    // ~keep Off Unix no pack digest is memoized, so a pack that exists is always read on the pool.
    #[cfg(unix)]
    #[test]
    fn tessdata_identities_alone_do_not_wait_for_the_blocking_pool() {
        use std::future::Future;
        use std::sync::mpsc;

        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("eng.traineddata"), b"model fixture").unwrap();
        let dependencies = [CacheDependency {
            tessdata: vec![ModelSelection {
                languages: vec!["eng".into()],
                directory: Some(root.path().into()),
                tessdata_model: crate::core::config::TessdataModel::Fast,
                mixed_pdf_khmer: false,
                vertical_japanese_retry: false,
            }],
            ..Default::default()
        }];
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

        let context = runtime.block_on(async {
            let mut pending = Box::pin(fingerprint(&dependencies, None));
            let first_poll =
                std::future::poll_fn(|context| std::task::Poll::Ready(pending.as_mut().poll(context))).await;
            release_tx.send(()).unwrap();
            blocker.await.unwrap();
            first_poll
        });
        let std::task::Poll::Ready(context) = context else {
            panic!("the tessdata context must not queue behind the occupied blocking worker");
        };
        assert!(context.unwrap().unwrap().starts_with(":tessdata-v1:"));
    }

    // ~keep Off Unix no pack digest is memoized, so a pack that exists is always read on the pool.
    #[cfg(unix)]
    #[test]
    fn an_ordinary_request_does_not_wait_for_the_blocking_pool_once_its_models_were_read() {
        use std::future::Future;
        use std::sync::mpsc;

        let dependencies = [CacheDependency::for_config(
            &ExtractionConfig::default(),
            Some("application/pdf"),
        )];
        assert!(
            dependencies[0].script_retry.is_some() && !dependencies[0].tessdata.is_empty(),
            "the default request may retry its script and read the default models"
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .max_blocking_threads(1)
            .build()
            .unwrap();
        let read = runtime.block_on(fingerprint(&dependencies, None)).unwrap();
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let blocker = runtime.spawn_blocking(move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        });
        started_rx.recv().unwrap();

        let context = runtime.block_on(async {
            let mut pending = Box::pin(fingerprint(&dependencies, None));
            let first_poll =
                std::future::poll_fn(|context| std::task::Poll::Ready(pending.as_mut().poll(context))).await;
            release_tx.send(()).unwrap();
            blocker.await.unwrap();
            first_poll
        });
        let std::task::Poll::Ready(context) = context else {
            panic!("a request whose models were read before must not queue behind the occupied blocking worker");
        };
        assert_eq!(
            context.unwrap(),
            read,
            "the inline context is the one the blocking pool computes"
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
        let mut no_retry = CacheDependency::for_config(&no_retry, Some("image/png"));
        no_retry.tessdata.clear();
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
            let mut dependency = CacheDependency::for_config(&config, Some("image/png"));
            assert!(dependency.model.is_none());
            dependency.tessdata.clear();
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
