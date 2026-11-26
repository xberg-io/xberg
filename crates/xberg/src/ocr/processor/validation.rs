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
/// they hold; `model` applies only to the directories xberg manages.
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
    resolve_tessdata_path_with(languages, override_path, tessdata_env, &SYSTEM_TESSDATA_DIRS)
}

/// [`resolve_tessdata_path_in`] with the system directories given as an argument.
fn resolve_tessdata_path_with(
    languages: &[String],
    override_path: Option<&Path>,
    tessdata_env: &TessdataEnv,
    system: &SystemTessdataDirs<'_>,
) -> Result<String, OcrError> {
    let cache_base = crate::cache_dir::cache_base_for(tessdata_env.cache_dir.as_deref());
    let managed = model_cache_dir(&cache_base, tessdata_env.model);
    let sources = adoption_sources(&cache_base, tessdata_env, system);

    for dir in tessdata_search_dirs_with(override_path, tessdata_env, system) {
        if all_languages_exist(&dir, languages)? {
            if Path::new(&dir) == managed.as_path() {
                adopt_shared_packs(&managed, tessdata_env.model, &sources);
            }
            log_resolved(&dir, override_path, tessdata_env);
            return Ok(dir);
        }
    }

    std::fs::create_dir_all(&managed).map_err(|e| {
        OcrError::TesseractInitializationFailed(format!(
            "Failed to create tessdata cache directory '{}': {}",
            managed.display(),
            e
        ))
    })?;

    materialize_missing_languages(&managed, languages, tessdata_env.model, &sources)?;
    adopt_shared_packs(&managed, tessdata_env.model, &sources);

    let dest_str = managed
        .to_str()
        .ok_or_else(|| OcrError::TesseractInitializationFailed("Tessdata path is not valid UTF-8".to_string()))?;

    if all_languages_exist(dest_str, languages)? {
        log_resolved(dest_str, override_path, tessdata_env);
        Ok(dest_str.to_string())
    } else {
        Err(OcrError::TesseractInitializationFailed(format!(
            "Failed to resolve all requested languages: {:?}",
            languages
        )))
    }
}

/// Local directories a missing pack of the requested set may be copied from after its pin is
/// verified: the flat cache and, for `best`, the `fast` cache, since `osd` and `equ` are the same
/// bytes in both sets; then the built-in `best` directory and the system paths.
fn adoption_sources(cache_base: &Path, tessdata_env: &TessdataEnv, system: &SystemTessdataDirs<'_>) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    if tessdata_env.model == TessdataModel::Best {
        sources.push(model_cache_dir(cache_base, TessdataModel::Fast));
    }
    sources.push(cache_base.join("tessdata"));
    if tessdata_env.system_paths {
        if tessdata_env.model == TessdataModel::Best {
            sources.extend(system.best.iter().map(PathBuf::from));
        }
        sources.extend(system.fast.iter().map(PathBuf::from));
    }
    sources
}

/// Copy the packs both sets share (`osd`, `equ`) into the managed directory from verified local
/// copies, once per directory and process, so OSD keeps finding `osd` beside the primary models.
/// Never downloads.
#[cfg(not(target_arch = "wasm32"))]
fn adopt_shared_packs(directory: &Path, model: TessdataModel, sources: &[PathBuf]) {
    use crate::ocr::tessdata_download::{adopt_pinned_pack, shared_languages};

    static ATTEMPTED: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<PathBuf>>> =
        std::sync::LazyLock::new(Default::default);
    let first = ATTEMPTED
        .lock()
        .map(|mut attempted| attempted.insert(directory.to_path_buf()))
        .unwrap_or(false);
    if !first {
        return;
    }
    for lang in shared_languages() {
        if !directory.join(format!("{lang}.traineddata")).is_file() {
            adopt_pinned_pack(lang, model, directory, sources);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn adopt_shared_packs(_directory: &Path, _model: TessdataModel, _sources: &[PathBuf]) {}

fn log_resolved(directory: &str, override_path: Option<&Path>, tessdata_env: &TessdataEnv) {
    let explicit = override_path.and_then(Path::to_str) == Some(directory)
        || tessdata_env.tessdata_prefix.as_deref() == Some(directory);
    tracing::debug!(
        directory,
        model = %tessdata_env.model,
        model_applies = !explicit,
        "Resolved tessdata directory"
    );
    if explicit && tessdata_env.model == TessdataModel::Best && first_best_warning_for(directory) {
        tracing::warn!(
            directory,
            "tessdata_model 'best' was requested, but this explicit tessdata directory \
             (tessdata_path or TESSDATA_PREFIX) is used as given, whichever model set it holds"
        );
    }
}

/// Whether `directory` has not yet had the explicit-directory warning in this process.
fn first_best_warning_for(directory: &str) -> bool {
    static WARNED: std::sync::LazyLock<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::LazyLock::new(Default::default);
    WARNED
        .lock()
        .map(|mut warned| warned.insert(directory.to_owned()))
        .unwrap_or(false)
}

/// Whether a `best` request would be served from `directory` although it holds no
/// xberg-verified `best` data: `TESSDATA_PREFIX` or a system path, which hold `tessdata_fast`
/// unless the operator installed otherwise. For doctor.
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
    first_directory_holding(&tessdata_search_dirs(override_path, tessdata_env), languages)
}

/// The first of `directories` that holds a `.traineddata` for every one of `languages`.
pub(crate) fn first_directory_holding(directories: &[String], languages: &[String]) -> Option<String> {
    directories
        .iter()
        .find(|dir| all_languages_exist(dir, languages).unwrap_or(false))
        .cloned()
}

/// Candidate tessdata directories in the resolver's priority order:
/// `override_path` (`OcrConfig.tessdata_path`), `TESSDATA_PREFIX`, `<cache>/tessdata/<model>`,
/// then, unless [`TessdataEnv::system_paths`] is false, the read-only `best` directory an image
/// can provide for [`TessdataModel::Best`]; for [`TessdataModel::Fast`], the flat
/// `<cache>/tessdata` and then the system paths.
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
    dirs.push(
        model_cache_dir(&cache_base, tessdata_env.model)
            .to_string_lossy()
            .into_owned(),
    );

    // ~keep The flat cache and system packages hold tessdata_fast; searching them for `best` would
    // silently serve fast models and make results depend on what happens to be installed.
    match tessdata_env.model {
        TessdataModel::Fast => {
            dirs.push(cache_base.join("tessdata").to_string_lossy().into_owned());
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
    best: &[BUILTIN_BEST_TESSDATA_DIR],
};

/// Where an image can ship read-only `tessdata_best` packs.
const BUILTIN_BEST_TESSDATA_DIR: &str = "/usr/share/xberg/tessdata/best";

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
        if !traineddata_path.exists() {
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
) -> Result<(), OcrError> {
    use crate::ocr::tessdata_download::{adopt_pinned_pack, ensure_language_pack};

    for lang in languages {
        let traineddata_path = tessdata_path.join(format!("{}.traineddata", lang));
        if traineddata_path.exists() || adopt_pinned_pack(lang, model, tessdata_path, sources) {
            continue;
        }

        if model == TessdataModel::Fast
            && lang == "eng"
            && let Some(bundled_bytes) = xberg_tesseract::bundled_eng_traineddata()
        {
            std::fs::write(&traineddata_path, bundled_bytes).map_err(|e| {
                OcrError::TesseractInitializationFailed(format!(
                    "Failed to write bundled eng.traineddata to '{}': {}",
                    traineddata_path.display(),
                    e
                ))
            })?;
            tracing::info!("Materialized bundled eng.traineddata to '{}'", tessdata_path.display());
            continue;
        }

        ensure_language_pack(lang, model, tessdata_path, &[])?;
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
