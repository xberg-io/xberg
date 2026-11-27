//! Image and configuration validation logic.
//!
//! This module handles validation of images, language files, and Tesseract configuration
//! before OCR processing begins.

use crate::core::config::TessdataModel;
use crate::ocr::error::OcrError;
use crate::ocr::validation::TESSERACT_SUPPORTED_LANGUAGE_CODES;
use std::borrow::Cow;
use std::env;
use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use crate::ocr::tessdata_download::{PINNED, PinTables};
/// WASM stand-in: no pins, since nothing is downloaded or adopted there.
#[cfg(target_arch = "wasm32")]
struct PinTables<'a>(std::marker::PhantomData<&'a ()>);
#[cfg(target_arch = "wasm32")]
const PINNED: PinTables<'static> = PinTables(std::marker::PhantomData);

/// Validate language configuration and check for traineddata files.
///
/// This function validates that:
/// 1. Language string is not empty
/// 2. Traineddata files exist for all specified languages
///
/// # Arguments
///
/// * `language` - Language code(s) to validate (can be "eng" or "eng+fra" etc.)
/// * `tessdata_path` - Path to tessdata directory
///
/// # Returns
///
/// `Ok(())` if validation passes, otherwise returns an error
pub(super) fn validate_language_and_traineddata(language: &str, tessdata_path: &str) -> Result<(), OcrError> {
    // Validate language before initializing to prevent segfault ~keep
    if language.trim().is_empty() {
        return Err(OcrError::TesseractInitializationFailed(
            "Language cannot be empty. Please specify a valid language code (e.g., 'eng')".to_string(),
        ));
    }

    // Validate language file exists before initializing to prevent segfault ~keep
    if !tessdata_path.is_empty() {
        let languages: Vec<&str> = language.split('+').collect();
        for lang in languages {
            let lang = lang.trim();
            if lang.is_empty() {
                continue;
            }
            require_language_names(&[lang])?;
            let traineddata_path = Path::new(tessdata_path).join(format!("{}.traineddata", lang));
            if !traineddata_path.exists() {
                return Err(OcrError::TesseractInitializationFailed(format!(
                    "Language '{}' not found. Traineddata file does not exist: {}",
                    lang,
                    traineddata_path.display()
                )));
            }
        }
    }

    Ok(())
}

/// Resolve tessdata path with runtime fallback chain and language validation.
///
/// Implements a multi-step resolution order. The first location that contains
/// every requested language wins:
/// 1. `OcrConfig.tessdata_path` override (`override_path` argument)
/// 2. `TESSDATA_PREFIX` environment variable
/// 3. xberg's cache for `model`: `<cache>/tessdata/<model>`, where `<cache>` is `XBERG_CACHE_DIR`
///    or [`crate::cache_dir::cache_base_for`]'s platform default
/// 4. For [`TessdataModel::Fast`] only: the flat `<cache>/tessdata` older releases downloaded
///    into, then the system fallback paths (macOS, Linux, Windows) unless
///    `XBERG_TESSDATA_SYSTEM_PATHS` is `0` or `false`. Both hold tessdata_fast data, so they never
///    satisfy a [`TessdataModel::Best`] request.
/// 5. If languages are still missing, materialize them into `<cache>/tessdata/<model>`: for
///    `fast`, from a flat-cache file that matches its pin or the bundled `eng`, otherwise by a
///    pinned, SHA-256 verified download from the model's GitHub repository.
///
/// The override and `TESSDATA_PREFIX` are the caller's choice and are used whichever model set
/// they hold; `model` applies only to the directories xberg manages. In the managed and built-in
/// `best` directories a pinned language counts as present only when its file is the pinned
/// `tessdata_best` pack; in the managed one a wrong file is moved aside and the pack fetched.
///
/// # Arguments
///
/// * `languages` - List of language codes to resolve (e.g., `["eng", "fra"]`)
/// * `override_path` - Optional explicit tessdata directory from `OcrConfig.tessdata_path`,
///   checked before any environment or system location.
/// * `model` - The model set xberg's own directories are searched and filled for.
///
/// # Returns
///
/// `Ok(String)` with the path to a valid tessdata directory containing all
/// requested languages, or `Err(OcrError)` if resolution fails.
pub(crate) fn resolve_tessdata_path(
    languages: &[String],
    override_path: Option<&Path>,
    model: TessdataModel,
) -> Result<String, OcrError> {
    resolve_tessdata_path_in(languages, override_path, &TessdataEnv::from_process().for_model(model))
}

/// The environment values the tessdata search order reads, plus the model set xberg's own
/// directories are resolved for.
#[derive(Debug)]
pub(crate) struct TessdataEnv {
    /// The `TESSDATA_PREFIX` value.
    pub(crate) tessdata_prefix: Option<String>,
    /// The `XBERG_CACHE_DIR` value.
    pub(crate) cache_dir: Option<PathBuf>,
    /// Whether the system tessdata locations are searched: false when
    /// `XBERG_TESSDATA_SYSTEM_PATHS` is `0` or `false`, true otherwise.
    pub(crate) system_paths: bool,
    /// The model set (`OcrConfig.tessdata_model`); not read from the environment.
    pub(crate) model: TessdataModel,
}

impl TessdataEnv {
    /// Read the values from the process environment, for the default model set.
    pub(crate) fn from_process() -> Self {
        Self::from_lookup(|name| env::var(name).ok())
    }

    /// Read the values through `lookup`, which maps a variable name to its value.
    pub(crate) fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        Self {
            tessdata_prefix: lookup("TESSDATA_PREFIX"),
            cache_dir: lookup("XBERG_CACHE_DIR").map(PathBuf::from),
            system_paths: !lookup("XBERG_TESSDATA_SYSTEM_PATHS").is_some_and(|value| {
                let value = value.trim();
                value == "0" || value.eq_ignore_ascii_case("false")
            }),
            model: TessdataModel::default(),
        }
    }

    /// The same values, resolving xberg's own directories for `model`.
    #[must_use]
    pub(crate) fn for_model(mut self, model: TessdataModel) -> Self {
        self.model = model;
        self
    }
}

/// xberg's cache directory for `model`: `<cache_base>/tessdata/<model>`.
pub(crate) fn model_cache_dir(cache_base: &Path, model: TessdataModel) -> PathBuf {
    cache_base.join("tessdata").join(model.as_str())
}

/// [`resolve_tessdata_path`] with the environment values given as arguments.
pub(crate) fn resolve_tessdata_path_in(
    languages: &[String],
    override_path: Option<&Path>,
    tessdata_env: &TessdataEnv,
) -> Result<String, OcrError> {
    let provision = Provision {
        system: &SYSTEM_TESSDATA_DIRS,
        pins: &PINNED,
        fetch: &download_pack,
    };
    resolve_tessdata_path_with(languages, override_path, tessdata_env, &provision)
}

/// Where packs come from: the system directories, the pins they are checked against and the
/// downloader. Production uses the real ones; tests inject small tables and a downloader that
/// fails the test if it is called.
struct Provision<'a> {
    system: &'a SystemTessdataDirs<'a>,
    pins: &'a PinTables<'a>,
    fetch: &'a dyn Fn(&str, TessdataModel, &Path) -> Result<bool, OcrError>,
}

#[cfg(not(target_arch = "wasm32"))]
fn download_pack(lang: &str, model: TessdataModel, directory: &Path) -> Result<bool, OcrError> {
    crate::ocr::tessdata_download::ensure_language_pack(lang, model, directory, &[])
}

#[cfg(target_arch = "wasm32")]
fn download_pack(lang: &str, _model: TessdataModel, _directory: &Path) -> Result<bool, OcrError> {
    Err(OcrError::TesseractInitializationFailed(format!(
        "Cannot download language pack '{lang}' on WASM"
    )))
}

/// [`resolve_tessdata_path_in`] with the pack sources given as an argument.
fn resolve_tessdata_path_with(
    languages: &[String],
    override_path: Option<&Path>,
    tessdata_env: &TessdataEnv,
    provision: &Provision<'_>,
) -> Result<String, OcrError> {
    require_language_names(languages)?;
    let cache_base = crate::cache_dir::cache_base_for(tessdata_env.cache_dir.as_deref());
    let managed = model_cache_dir(&cache_base, tessdata_env.model);
    let flat = cache_base.join("tessdata");
    let sources = adoption_sources(&cache_base, tessdata_env, provision.system);
    let xberg_owned =
        |dir: &Path| dir == managed.as_path() || (tessdata_env.model == TessdataModel::Fast && dir == flat);

    if let Some(dir) = first_serving_dir(languages, override_path, tessdata_env, provision.system, provision.pins) {
        if xberg_owned(Path::new(&dir)) {
            adopt_shared_packs(Path::new(&dir), tessdata_env.model, &sources, provision.pins);
        }
        log_resolved(&dir, languages, override_path, tessdata_env, provision.pins);
        return Ok(dir);
    }

    // ~keep A flat cache that already serves some of the requested languages stays the fast
    // destination, as before model sets existed, so a pack placed there by hand keeps being used
    // beside the newly downloaded ones.
    let destination = if tessdata_env.model == TessdataModel::Fast && any_language_exists(&flat, languages) {
        flat
    } else {
        managed.clone()
    };
    std::fs::create_dir_all(&destination).map_err(|e| {
        OcrError::TesseractInitializationFailed(format!(
            "Failed to create tessdata cache directory '{}': {}",
            destination.display(),
            e
        ))
    })?;

    // ~keep A best request is only ever filled into the managed best directory, whose packs are
    // held to their pins.
    let destination_is_verified = tessdata_env.model == TessdataModel::Best;
    materialize_missing_languages(&destination, languages, tessdata_env.model, &sources, provision)?;
    adopt_shared_packs(&destination, tessdata_env.model, &sources, provision.pins);

    let dest_str = destination
        .to_str()
        .ok_or_else(|| OcrError::TesseractInitializationFailed("Tessdata path is not valid UTF-8".to_string()))?;

    if directory_serves(dest_str, languages, destination_is_verified, provision.pins) {
        log_resolved(dest_str, languages, override_path, tessdata_env, provision.pins);
        Ok(dest_str.to_string())
    } else {
        Err(OcrError::TesseractInitializationFailed(format!(
            "Failed to resolve all requested languages: {:?}",
            languages
        )))
    }
}

/// Refuse a language that is not a model name before it is joined into a file path.
///
/// `OcrConfig::validate` rejects these first; this covers callers that build a `TesseractConfig`
/// without it. ~keep
fn require_language_names<S: AsRef<str>>(languages: &[S]) -> Result<(), OcrError> {
    match languages
        .iter()
        .map(AsRef::as_ref)
        .find(|name| !crate::core::config::ocr::is_tesseract_language_name(name))
    {
        Some(name) => Err(OcrError::InvalidLanguageCode(format!(
            "Tesseract language '{name}' is not a model name; set tessdata_path to use models from another directory"
        ))),
        None => Ok(()),
    }
}

fn any_language_exists(directory: &Path, languages: &[String]) -> bool {
    languages
        .iter()
        .any(|lang| directory.join(format!("{lang}.traineddata")).is_file())
}

/// Local directories a missing pack of the requested set may be copied from after its pin is
/// verified: the flat cache and the `fast` cache (for `best` too, since `osd` and `equ` are the
/// same bytes in both sets; a directory is never its own source), then the built-in `best`
/// directory and the system paths.
fn adoption_sources(cache_base: &Path, tessdata_env: &TessdataEnv, system: &SystemTessdataDirs<'_>) -> Vec<PathBuf> {
    let mut sources = vec![
        cache_base.join("tessdata"),
        model_cache_dir(cache_base, TessdataModel::Fast),
    ];
    if tessdata_env.system_paths {
        if tessdata_env.model == TessdataModel::Best {
            sources.extend(system.best.iter().map(PathBuf::from));
        }
        sources.extend(system.fast.iter().map(PathBuf::from));
    }
    sources
}

/// The size and modification time of each candidate source file, to tell whether a failed
/// adoption is worth retrying.
type SourceState = Vec<Option<(u64, std::time::SystemTime)>>;

/// Copy the packs both sets share (`osd`, `equ`) into the managed directory from verified local
/// copies, so OSD keeps finding `osd` beside the primary models. Never downloads.
///
/// Presence is checked on every call, so concurrent first requests cannot skip it; a failed
/// attempt is retried only once a candidate source file changes, so an unusable source is not
/// re-hashed on every request.
#[cfg(not(target_arch = "wasm32"))]
fn adopt_shared_packs(directory: &Path, model: TessdataModel, sources: &[PathBuf], pins: &PinTables<'_>) {
    static FAILED: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<PathBuf, SourceState>>> =
        std::sync::LazyLock::new(Default::default);

    for lang in pins.shared_languages() {
        let file_name = format!("{lang}.traineddata");
        let target = directory.join(&file_name);
        if target.is_file() {
            continue;
        }
        let state: SourceState = sources
            .iter()
            .map(|source| {
                let metadata = std::fs::metadata(source.join(&file_name)).ok()?;
                Some((metadata.len(), metadata.modified().ok()?))
            })
            .collect();
        if FAILED.lock().is_ok_and(|failed| failed.get(&target) == Some(&state)) {
            continue;
        }
        if !pins.adopt(lang, model, directory, sources)
            && let Ok(mut failed) = FAILED.lock()
        {
            failed.insert(target, state);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn adopt_shared_packs(_directory: &Path, _model: TessdataModel, _sources: &[PathBuf], _pins: &PinTables<'_>) {}

fn log_resolved(
    directory: &str,
    languages: &[String],
    override_path: Option<&Path>,
    tessdata_env: &TessdataEnv,
    pins: &PinTables<'_>,
) {
    let explicit = override_path.and_then(Path::to_str) == Some(directory)
        || tessdata_env.tessdata_prefix.as_deref() == Some(directory);
    tracing::debug!(
        directory,
        model = %tessdata_env.model,
        model_applies = !explicit,
        "Resolved tessdata directory"
    );
    if explicit && tessdata_env.model == TessdataModel::Best {
        warn_unverified_best(directory, languages, pins);
    }
}

/// Warn that a `best` request is served from an explicit directory whose models are not verified
/// `tessdata_best`, once per directory and finding: naming the languages when their packs are
/// verified `tessdata_fast`, and with the general notice when a pack is no pinned one (a
/// user-trained model stays usable). Silent when every requested pack is verified `best`.
#[cfg(not(target_arch = "wasm32"))]
fn warn_unverified_best(directory: &str, languages: &[String], pins: &PinTables<'_>) {
    if pins.verified_model(Path::new(directory), languages) == Some(TessdataModel::Best) {
        return;
    }
    let fast = packs_verified_fast(directory, languages, pins);
    if fast.is_empty() {
        if first_best_warning_for(directory) {
            warn_explicit_directory_used_as_given(directory);
        }
        return;
    }
    let languages = fast.join("+");
    if first_best_warning_for(&format!("{directory}\0{languages}")) {
        tracing::warn!(
            directory,
            languages = %languages,
            "tessdata_model 'best' was requested, but this explicit tessdata directory (tessdata_path or \
             TESSDATA_PREFIX) holds tessdata_fast models for these languages, and they are used as given. For \
             tessdata_best, unset the directory so xberg downloads verified models (`xberg cache warm \
             --tessdata-model best` fetches them ahead of time), or replace the files with tessdata_best ones"
        );
    }
}

/// WASM has no pins to verify a pack against.
#[cfg(target_arch = "wasm32")]
fn warn_unverified_best(directory: &str, _languages: &[String], _pins: &PinTables<'_>) {
    if first_best_warning_for(directory) {
        warn_explicit_directory_used_as_given(directory);
    }
}

fn warn_explicit_directory_used_as_given(directory: &str) {
    tracing::warn!(
        directory,
        "tessdata_model 'best' was requested, but this explicit tessdata directory \
         (tessdata_path or TESSDATA_PREFIX) is used as given, whichever model set it holds"
    );
}

/// The requested languages whose pack in `directory` is verified to be the pinned
/// `tessdata_fast` one.
#[cfg(not(target_arch = "wasm32"))]
fn packs_verified_fast<'a>(directory: &str, languages: &'a [String], pins: &PinTables<'_>) -> Vec<&'a str> {
    languages
        .iter()
        .map(String::as_str)
        .filter(|lang| {
            let pack = Path::new(directory).join(format!("{lang}.traineddata"));
            pins.classify(lang, &pack) == crate::ocr::tessdata_download::VerifiedSet::Fast
        })
        .collect()
}

/// Whether `key` (a directory, or a directory and finding) has not yet had its explicit-directory
/// warning in this process.
fn first_best_warning_for(directory: &str) -> bool {
    static WARNED: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::LazyLock::new(Default::default);
    WARNED
        .lock()
        .map(|mut warned| warned.insert(directory.to_owned()))
        .unwrap_or(false)
}

/// Whether a `best` request would be served from `directory` although it holds no
/// xberg-verified `best` data: `TESSDATA_PREFIX`, or a `tessdata_path` naming a system package
/// directory (the search itself never visits those for `best`), which hold `tessdata_fast` unless
/// the operator installed otherwise. For doctor.
pub(crate) fn best_served_by_unverified_dir(directory: &str, tessdata_env: &TessdataEnv) -> bool {
    tessdata_env.model == TessdataModel::Best
        && (tessdata_env.tessdata_prefix.as_deref() == Some(directory)
            || SYSTEM_TESSDATA_DIRS.fast.contains(&directory))
}

/// The first candidate directory that already holds a `.traineddata` for every one of
/// `languages`, or `None` when none does.
///
/// [`resolve_tessdata_path_in`] without the fall-through that creates a cache directory and
/// downloads the missing packs. A capability probe must not be able to reach the network:
/// `OcrBackend::supports_language_for` answers a question about this machine, and asking it
/// about a language this machine has never seen would otherwise start a download. Shares
/// [`tessdata_search_dirs`] with the real resolver so the two still agree on priority order.
/// GH#1891. ~keep
pub(crate) fn existing_tessdata_dir_for(
    languages: &[String],
    override_path: Option<&Path>,
    tessdata_env: &TessdataEnv,
) -> Option<String> {
    first_serving_dir(languages, override_path, tessdata_env, &SYSTEM_TESSDATA_DIRS, &PINNED)
}

/// The first candidate directory that serves every one of `languages`, in the resolver's order.
fn first_serving_dir(
    languages: &[String],
    override_path: Option<&Path>,
    tessdata_env: &TessdataEnv,
    system: &SystemTessdataDirs<'_>,
    pins: &PinTables<'_>,
) -> Option<String> {
    tessdata_search_dirs_with(override_path, tessdata_env, system)
        .into_iter()
        .find(|dir| {
            let verified = holds_verified_best(dir, override_path, tessdata_env, system);
            directory_serves(dir, languages, verified, pins)
        })
}

/// Whether `directory` is one xberg vouches for as `tessdata_best`: the managed best cache or the
/// read-only best directory of an image, reached through the search order rather than named by
/// the caller. A pack there is held to its pin; everywhere else a file is used as it is.
fn holds_verified_best(
    directory: &str,
    override_path: Option<&Path>,
    tessdata_env: &TessdataEnv,
    system: &SystemTessdataDirs<'_>,
) -> bool {
    if tessdata_env.model != TessdataModel::Best
        || override_path.and_then(Path::to_str) == Some(directory)
        || tessdata_env.tessdata_prefix.as_deref() == Some(directory)
    {
        return false;
    }
    let cache_base = crate::cache_dir::cache_base_for(tessdata_env.cache_dir.as_deref());
    Path::new(directory) == model_cache_dir(&cache_base, TessdataModel::Best) || system.best.contains(&directory)
}

/// Whether `directory` holds a usable pack for every one of `languages`: a regular file, which in
/// a `verified_best` directory must also be the pinned `tessdata_best` pack when its language has
/// one. A wrong, empty or truncated file there counts as missing.
fn directory_serves(directory: &str, languages: &[String], verified_best: bool, pins: &PinTables<'_>) -> bool {
    !directory.is_empty()
        && !languages.is_empty()
        && languages.iter().all(|lang| {
            let pack = Path::new(directory).join(format!("{lang}.traineddata"));
            pack_serves(&pack, lang, verified_best, pins)
        })
}

#[cfg(not(target_arch = "wasm32"))]
fn pack_serves(pack: &Path, lang: &str, verified_best: bool, pins: &PinTables<'_>) -> bool {
    if verified_best && pins.pin(lang, TessdataModel::Best).is_some() {
        pins.classify(lang, pack).is(TessdataModel::Best)
    } else {
        pack.is_file()
    }
}

/// WASM has no pins to hold a pack to.
#[cfg(target_arch = "wasm32")]
fn pack_serves(pack: &Path, _lang: &str, _verified_best: bool, _pins: &PinTables<'_>) -> bool {
    pack.is_file()
}

/// The first of `directories` that holds a `.traineddata` for every one of `languages`.
pub(crate) fn first_directory_holding(directories: &[String], languages: &[String]) -> Option<String> {
    directories
        .iter()
        .find(|dir| all_languages_exist(dir, languages).unwrap_or(false))
        .cloned()
}

/// Candidate tessdata directories in the resolver's priority order:
/// `override_path` (`OcrConfig.tessdata_path`), `TESSDATA_PREFIX`, then for
/// [`TessdataModel::Fast`] the flat `<cache>/tessdata`, `<cache>/tessdata/fast` and the system
/// paths, and for [`TessdataModel::Best`] `<cache>/tessdata/best` and the read-only `best`
/// directory an image can provide; system and image directories only unless
/// [`TessdataEnv::system_paths`] is false.
///
/// Shared by [`resolve_tessdata_path_in`] and doctor's check-only probe so both
/// report on the same search chain.
pub(crate) fn tessdata_search_dirs(override_path: Option<&Path>, tessdata_env: &TessdataEnv) -> Vec<String> {
    tessdata_search_dirs_with(override_path, tessdata_env, &SYSTEM_TESSDATA_DIRS)
}

fn tessdata_search_dirs_with(
    override_path: Option<&Path>,
    tessdata_env: &TessdataEnv,
    system: &SystemTessdataDirs<'_>,
) -> Vec<String> {
    let mut dirs = Vec::new();

    if let Some(path) = override_path
        && let Some(path_str) = path.to_str()
        && !path_str.is_empty()
    {
        dirs.push(path_str.to_string());
    }

    if let Some(path) = &tessdata_env.tessdata_prefix
        && !path.is_empty()
    {
        dirs.push(path.clone());
    }

    let cache_base = crate::cache_dir::cache_base_for(tessdata_env.cache_dir.as_deref());
    // ~keep The flat cache comes first for fast so a cache filled before model sets existed,
    // including packs placed there by hand, resolves exactly as it did.
    if tessdata_env.model == TessdataModel::Fast {
        dirs.push(cache_base.join("tessdata").to_string_lossy().into_owned());
    }
    dirs.push(
        model_cache_dir(&cache_base, tessdata_env.model)
            .to_string_lossy()
            .into_owned(),
    );

    // ~keep The flat cache and system packages hold tessdata_fast; searching them for `best` would
    // silently serve fast models and make results depend on what happens to be installed.
    match tessdata_env.model {
        TessdataModel::Fast => {
            if tessdata_env.system_paths {
                dirs.extend(system.fast.iter().map(|path| (*path).to_string()));
            }
        }
        TessdataModel::Best => {
            if tessdata_env.system_paths {
                dirs.extend(system.best.iter().map(|path| (*path).to_string()));
            }
        }
    }

    dirs
}

/// System tessdata directories per model set.
pub(crate) struct SystemTessdataDirs<'a> {
    /// Package-manager directories; they ship `tessdata_fast`.
    pub(crate) fast: &'a [&'a str],
    /// Read-only `best` directories an image can provide.
    pub(crate) best: &'a [&'a str],
}

const SYSTEM_TESSDATA_DIRS: SystemTessdataDirs<'static> = SystemTessdataDirs {
    fast: SYSTEM_TESSDATA_PATHS,
    best: BUILTIN_BEST_TESSDATA_DIRS,
};

/// Where an image can ship read-only `tessdata_best` packs (Unix images only).
#[cfg(unix)]
const BUILTIN_BEST_TESSDATA_DIRS: &[&str] = &["/usr/share/xberg/tessdata/best"];
#[cfg(not(unix))]
const BUILTIN_BEST_TESSDATA_DIRS: &[&str] = &[];

const SYSTEM_TESSDATA_PATHS: &[&str] = &[
    "/opt/homebrew/share/tessdata",
    "/opt/homebrew/opt/tesseract/share/tessdata",
    "/usr/local/opt/tesseract/share/tessdata",
    "/usr/share/tesseract-ocr/5/tessdata",
    "/usr/share/tesseract-ocr/4/tessdata",
    "/usr/share/tessdata",
    "/usr/local/share/tessdata",
    r#"C:\Program Files\Tesseract-OCR\tessdata"#,
    r#"C:\ProgramData\Tesseract-OCR\tessdata"#,
];

/// Check if all languages in the list exist as traineddata files in the given directory.
fn all_languages_exist(tessdata_path: &str, languages: &[String]) -> Result<bool, OcrError> {
    if tessdata_path.is_empty() || languages.is_empty() {
        return Ok(false);
    }

    let tessdata_dir = Path::new(tessdata_path);
    if !tessdata_dir.exists() {
        return Ok(false);
    }

    for lang in languages {
        let traineddata_path = tessdata_dir.join(format!("{}.traineddata", lang));
        if !traineddata_path.is_file() {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Materialize missing `model` language files into `tessdata_path`.
///
/// A pack is copied from the first of `sources` holding a file that matches its pin; for
/// [`TessdataModel::Fast`], `eng` can come from the bundled bytes; everything else is a pinned,
/// verified download. A `best` request never uses the bundled model, which is tessdata_fast.
#[cfg(not(target_arch = "wasm32"))]
fn materialize_missing_languages(
    tessdata_path: &Path,
    languages: &[String],
    model: TessdataModel,
    sources: &[PathBuf],
    provision: &Provision<'_>,
) -> Result<(), OcrError> {
    // ~keep Best packs are only ever filled into the managed best directory, where a file that
    // is not the pinned pack (hand-copied fast data, an empty or truncated file) is replaced
    // rather than served.
    let verified_best = model == TessdataModel::Best;
    for lang in languages {
        let traineddata_path = tessdata_path.join(format!("{}.traineddata", lang));
        if pack_serves(&traineddata_path, lang, verified_best, provision.pins) {
            continue;
        }
        if verified_best {
            provision
                .pins
                .set_aside_unverified(lang, model, tessdata_path)
                .map_err(|e| {
                    OcrError::TesseractInitializationFailed(format!(
                        "'{}' is not the pinned {} language pack and could not be moved aside: {e}. Remove it",
                        traineddata_path.display(),
                        model.repository()
                    ))
                })?;
        }
        if provision.pins.adopt(lang, model, tessdata_path, sources) {
            continue;
        }

        if model == TessdataModel::Fast
            && lang == "eng"
            && let Some(bundled_bytes) = xberg_tesseract::bundled_eng_traineddata()
        {
            // ~keep Other threads accept this directory as soon as the file exists, so it must
            // never be visible half-written.
            crate::ocr::tessdata_download::publish(bundled_bytes, &traineddata_path).map_err(|e| {
                OcrError::TesseractInitializationFailed(format!(
                    "Failed to write bundled eng.traineddata to '{}': {}",
                    traineddata_path.display(),
                    e
                ))
            })?;
            tracing::info!("Materialized bundled eng.traineddata to '{}'", tessdata_path.display());
            continue;
        }

        (provision.fetch)(lang, model, tessdata_path)?;
    }

    Ok(())
}

/// WASM stub: no network access, no file writes.
#[cfg(target_arch = "wasm32")]
fn materialize_missing_languages(
    _tessdata_path: &Path,
    languages: &[String],
    _model: TessdataModel,
    _sources: &[PathBuf],
    _provision: &Provision<'_>,
) -> Result<(), OcrError> {
    Err(OcrError::TesseractInitializationFailed(format!(
        "Cannot download language packs on WASM. Requested: {:?}",
        languages
    )))
}

/// Resolve all installed Tesseract languages from the tessdata directory.
///
/// Scans the tessdata directory for `*.traineddata` files, filters against
/// known Tesseract language codes (excluding non-language files like `osd`),
/// and returns a `+`-separated language string (e.g., `"eng+fra+deu"`).
///
/// # Arguments
///
/// * `tessdata_path` - Path to the tessdata directory
///
/// # Returns
///
/// A `+`-separated string of installed language codes, or an error if no languages are found.
pub(super) fn resolve_all_installed_languages(tessdata_path: &str) -> Result<String, OcrError> {
    if tessdata_path.is_empty() {
        return Err(OcrError::TesseractInitializationFailed(
            "Cannot resolve installed languages: tessdata path is empty. \
             Set TESSDATA_PREFIX or install Tesseract with language data."
                .to_string(),
        ));
    }

    let tessdata_dir = Path::new(tessdata_path);
    if !tessdata_dir.exists() {
        return Err(OcrError::TesseractInitializationFailed(format!(
            "Tessdata directory does not exist: {}",
            tessdata_path
        )));
    }

    let entries = std::fs::read_dir(tessdata_dir).map_err(|e| {
        OcrError::TesseractInitializationFailed(format!("Failed to read tessdata directory '{}': {}", tessdata_path, e))
    })?;

    const EXCLUDED: &[&str] = &["osd", "equ"];

    let mut languages: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;
            let lang = file_name.strip_suffix(".traineddata")?;
            if EXCLUDED.contains(&lang) {
                return None;
            }
            if TESSERACT_SUPPORTED_LANGUAGE_CODES.contains(lang) {
                Some(lang.to_string())
            } else {
                None
            }
        })
        .collect();

    if languages.is_empty() {
        return Err(OcrError::TesseractInitializationFailed(format!(
            "No installed Tesseract languages found in '{}'",
            tessdata_path
        )));
    }

    languages.sort();
    Ok(languages.join("+"))
}

/// Strip control characters from text, preserving whitespace.
///
/// Removes control characters (0x00-0x1F, 0x7F) except for newlines, carriage returns, and tabs.
///
/// # Arguments
///
/// * `text` - Text to clean
///
/// # Returns
///
/// Cleaned text with control characters removed
pub(super) fn strip_control_characters(text: &str) -> Cow<'_, str> {
    if text
        .chars()
        .any(|c| matches!(c, '\u{0000}'..='\u{001F}' | '\u{007F}') && c != '\n' && c != '\r' && c != '\t')
    {
        Cow::Owned(
            text.chars()
                .filter(|c| !matches!(c, '\u{0000}'..='\u{001F}' | '\u{007F}') || matches!(c, '\n' | '\r' | '\t'))
                .collect(),
        )
    } else {
        Cow::Borrowed(text)
    }
}

#[cfg(test)]
mod tests;
