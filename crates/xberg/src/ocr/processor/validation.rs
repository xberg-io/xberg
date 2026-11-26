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
    for dir in tessdata_search_dirs(override_path, tessdata_env) {
        if all_languages_exist(&dir, languages)? {
            log_resolved(&dir, override_path, tessdata_env);
            return Ok(dir);
        }
    }

    let cache_base = crate::cache_dir::cache_base_for(tessdata_env.cache_dir.as_deref());
    let download_dest = model_cache_dir(&cache_base, tessdata_env.model);
    std::fs::create_dir_all(&download_dest).map_err(|e| {
        OcrError::TesseractInitializationFailed(format!(
            "Failed to create tessdata cache directory '{}': {}",
            download_dest.display(),
            e
        ))
    })?;

    let legacy = (tessdata_env.model == TessdataModel::Fast).then(|| cache_base.join("tessdata"));
    materialize_missing_languages(&download_dest, languages, tessdata_env.model, legacy.as_deref())?;

    let dest_str = download_dest
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

fn log_resolved(directory: &str, override_path: Option<&Path>, tessdata_env: &TessdataEnv) {
    let explicit = override_path.and_then(Path::to_str) == Some(directory)
        || tessdata_env.tessdata_prefix.as_deref() == Some(directory);
    tracing::debug!(
        directory,
        model = %tessdata_env.model,
        model_applies = !explicit,
        "Resolved tessdata directory"
    );
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
/// then, for [`TessdataModel::Fast`] only, the flat `<cache>/tessdata` and the system paths
/// unless [`TessdataEnv::system_paths`] is false.
///
/// Shared by [`resolve_tessdata_path_in`] and doctor's check-only probe so both
/// report on the same search chain.
pub(crate) fn tessdata_search_dirs(override_path: Option<&Path>, tessdata_env: &TessdataEnv) -> Vec<String> {
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
    if tessdata_env.model == TessdataModel::Fast {
        dirs.push(cache_base.join("tessdata").to_string_lossy().into_owned());
        if tessdata_env.system_paths {
            dirs.extend(SYSTEM_TESSDATA_PATHS.iter().map(|path| (*path).to_string()));
        }
    }

    dirs
}

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
/// For [`TessdataModel::Fast`], a file in the flat `legacy` cache that matches its pin is adopted
/// and `eng` can come from the bundled bytes; everything else is a pinned, verified download.
/// A `best` request never uses either, both being tessdata_fast.
#[cfg(not(target_arch = "wasm32"))]
fn materialize_missing_languages(
    tessdata_path: &Path,
    languages: &[String],
    model: TessdataModel,
    legacy: Option<&Path>,
) -> Result<(), OcrError> {
    use crate::ocr::tessdata_download::{download_language_pack, matches_pinned_pack};

    for lang in languages {
        let traineddata_path = tessdata_path.join(format!("{}.traineddata", lang));
        if traineddata_path.exists() {
            continue;
        }

        if let Some(legacy) = legacy {
            let legacy_path = legacy.join(format!("{lang}.traineddata"));
            if matches_pinned_pack(lang, model, &legacy_path) && adopt_file(&legacy_path, &traineddata_path).is_ok() {
                tracing::debug!(lang, from = %legacy_path.display(), "Adopted verified tessdata from the flat cache");
                continue;
            }
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

        download_language_pack(lang, model, tessdata_path)?;
    }

    Ok(())
}

/// Publish `source` at `target` without replacing an existing file: a hard link when the two
/// share a file system, otherwise a copy.
#[cfg(not(target_arch = "wasm32"))]
fn adopt_file(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::io::ErrorKind;

    match std::fs::hard_link(source, target) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == ErrorKind::AlreadyExists => return Ok(()),
        Err(error) => tracing::debug!(%error, "Cannot hard-link the tessdata file; copying it instead"),
    }
    let directory = target
        .parent()
        .ok_or_else(|| std::io::Error::new(ErrorKind::InvalidInput, "missing tessdata directory"))?;
    let temporary = tempfile::NamedTempFile::new_in(directory)?;
    std::fs::copy(source, temporary.path())?;
    match temporary.persist_noclobber(target) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.error),
    }
}

/// WASM stub: no network access, no file writes.
#[cfg(target_arch = "wasm32")]
fn materialize_missing_languages(
    _tessdata_path: &Path,
    languages: &[String],
    _model: TessdataModel,
    _legacy: Option<&Path>,
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
mod tests {
    use super::*;

    #[test]
    fn test_resolve_all_installed_languages_success() {
        let dir = tempfile::tempdir().unwrap();
        let tessdata = dir.path();

        std::fs::write(tessdata.join("eng.traineddata"), b"").unwrap();
        std::fs::write(tessdata.join("fra.traineddata"), b"").unwrap();
        std::fs::write(tessdata.join("deu.traineddata"), b"").unwrap();

        let result = resolve_all_installed_languages(tessdata.to_str().unwrap()).unwrap();
        assert_eq!(result, "deu+eng+fra");
    }

    #[test]
    fn test_resolve_all_installed_languages_excludes_osd() {
        let dir = tempfile::tempdir().unwrap();
        let tessdata = dir.path();

        std::fs::write(tessdata.join("eng.traineddata"), b"").unwrap();
        std::fs::write(tessdata.join("osd.traineddata"), b"").unwrap();

        let result = resolve_all_installed_languages(tessdata.to_str().unwrap()).unwrap();
        assert_eq!(result, "eng");
    }

    #[test]
    fn test_resolve_all_installed_languages_excludes_equ() {
        let dir = tempfile::tempdir().unwrap();
        let tessdata = dir.path();

        std::fs::write(tessdata.join("eng.traineddata"), b"").unwrap();
        std::fs::write(tessdata.join("equ.traineddata"), b"").unwrap();

        let result = resolve_all_installed_languages(tessdata.to_str().unwrap()).unwrap();
        assert_eq!(result, "eng");
    }

    #[test]
    fn test_resolve_all_installed_languages_excludes_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let tessdata = dir.path();

        std::fs::write(tessdata.join("eng.traineddata"), b"").unwrap();
        std::fs::write(tessdata.join("notareal.traineddata"), b"").unwrap();

        let result = resolve_all_installed_languages(tessdata.to_str().unwrap()).unwrap();
        assert_eq!(result, "eng");
    }

    #[test]
    fn test_resolve_all_installed_languages_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve_all_installed_languages(dir.path().to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_all_installed_languages_empty_path() {
        let result = resolve_all_installed_languages("");
        assert!(result.is_err());
    }

    #[test]
    fn test_all_languages_exist_returns_true_when_present() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("eng.traineddata"), b"x").unwrap();
        std::fs::write(dir.path().join("fra.traineddata"), b"x").unwrap();

        let langs = vec!["eng".to_string(), "fra".to_string()];
        assert!(all_languages_exist(dir.path().to_str().unwrap(), &langs).unwrap());
    }

    #[test]
    fn test_all_languages_exist_returns_false_when_one_missing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("eng.traineddata"), b"x").unwrap();

        let langs = vec!["eng".to_string(), "deu".to_string()];
        assert!(!all_languages_exist(dir.path().to_str().unwrap(), &langs).unwrap());
    }

    #[test]
    fn test_all_languages_exist_false_for_empty_path_or_langs() {
        assert!(!all_languages_exist("", &["eng".to_string()]).unwrap());
        let dir = tempfile::tempdir().unwrap();
        assert!(!all_languages_exist(dir.path().to_str().unwrap(), &[]).unwrap());
    }

    #[test]
    fn test_all_languages_exist_false_for_missing_dir() {
        let langs = vec!["eng".to_string()];
        assert!(!all_languages_exist("/nonexistent/path/xyz", &langs).unwrap());
    }

    #[test]
    fn test_resolve_tessdata_path_prefers_override() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("eng.traineddata"), b"x").unwrap();

        let langs = vec!["eng".to_string()];
        for model in [TessdataModel::Fast, TessdataModel::Best] {
            let resolved = resolve_tessdata_path(&langs, Some(dir.path()), model).unwrap();
            assert_eq!(resolved, dir.path().to_str().unwrap(), "{model}");
        }
    }

    #[test]
    fn test_resolve_tessdata_path_skips_override_missing_lang() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("eng.traineddata"), b"x").unwrap();

        let langs = vec!["deu".to_string()];
        let result = resolve_tessdata_path(&langs, Some(dir.path()), TessdataModel::default());
        if let Ok(path) = result {
            assert_ne!(path, dir.path().to_str().unwrap());
        }
    }

    fn given_env(model: TessdataModel, system_paths: bool) -> TessdataEnv {
        TessdataEnv {
            tessdata_prefix: Some("/given/prefix".to_string()),
            cache_dir: Some(PathBuf::from("/given/cache")),
            system_paths,
            model,
        }
    }

    fn given_cache(parts: &[&str]) -> String {
        parts
            .iter()
            .fold(PathBuf::from("/given/cache"), |path, part| path.join(part))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn test_tessdata_search_dirs_uses_only_the_given_environment_values() {
        let dirs = tessdata_search_dirs(
            Some(Path::new("/given/override")),
            &given_env(TessdataModel::Fast, true),
        );

        let mut expected = vec![
            "/given/override".to_string(),
            "/given/prefix".to_string(),
            given_cache(&["tessdata", "fast"]),
            given_cache(&["tessdata"]),
        ];
        expected.extend(SYSTEM_TESSDATA_PATHS.iter().map(|path| (*path).to_string()));
        assert_eq!(dirs, expected);
    }

    #[test]
    fn test_best_search_dirs_hold_no_fast_only_directory() {
        let dirs = tessdata_search_dirs(
            Some(Path::new("/given/override")),
            &given_env(TessdataModel::Best, true),
        );

        assert_eq!(
            dirs,
            [
                "/given/override".to_string(),
                "/given/prefix".to_string(),
                given_cache(&["tessdata", "best"]),
            ]
        );
    }

    #[test]
    fn test_each_model_has_its_own_cache_directory() {
        let base = Path::new("/given/cache");
        let fast = model_cache_dir(base, TessdataModel::Fast);
        let best = model_cache_dir(base, TessdataModel::Best);

        assert_eq!(fast, base.join("tessdata").join("fast"));
        assert_eq!(best, base.join("tessdata").join("best"));
        assert!(!best.starts_with(&fast) && !fast.starts_with(&best));
    }

    #[test]
    fn test_best_request_never_resolves_to_a_fast_only_directory() {
        let cache = tempfile::tempdir().unwrap();
        let system = tempfile::tempdir().unwrap();
        for directory in [
            cache.path().join("tessdata"),
            cache.path().join("tessdata").join("fast"),
            system.path().to_path_buf(),
        ] {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join("eng.traineddata"), b"fast").unwrap();
        }
        let env = |model| TessdataEnv {
            tessdata_prefix: None,
            cache_dir: Some(cache.path().to_path_buf()),
            system_paths: true,
            model,
        };
        let langs = vec!["eng".to_string()];

        let fast = existing_tessdata_dir_for(&langs, None, &env(TessdataModel::Fast));
        let best = existing_tessdata_dir_for(&langs, None, &env(TessdataModel::Best));

        assert_eq!(fast.as_deref(), cache.path().join("tessdata").join("fast").to_str());
        assert_eq!(best, None, "a best request must not be served fast data");
    }

    #[test]
    fn test_fast_request_still_resolves_the_flat_cache() {
        let cache = tempfile::tempdir().unwrap();
        let flat = cache.path().join("tessdata");
        std::fs::create_dir_all(&flat).unwrap();
        std::fs::write(flat.join("eng.traineddata"), b"fast").unwrap();
        let env = TessdataEnv {
            tessdata_prefix: None,
            cache_dir: Some(cache.path().to_path_buf()),
            system_paths: false,
            model: TessdataModel::Fast,
        };

        let resolved = existing_tessdata_dir_for(&["eng".to_string()], None, &env);

        assert_eq!(resolved.as_deref(), flat.to_str());
    }

    #[test]
    fn test_override_and_tessdata_prefix_win_for_every_model() {
        let explicit = tempfile::tempdir().unwrap();
        let prefix = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        for directory in [explicit.path(), prefix.path()] {
            std::fs::write(directory.join("eng.traineddata"), b"x").unwrap();
        }
        let langs = vec!["eng".to_string()];
        for model in [TessdataModel::Fast, TessdataModel::Best] {
            let model_cache = model_cache_dir(cache.path(), model);
            std::fs::create_dir_all(&model_cache).unwrap();
            std::fs::write(model_cache.join("eng.traineddata"), b"x").unwrap();
            let env = TessdataEnv {
                tessdata_prefix: Some(prefix.path().to_str().unwrap().to_string()),
                cache_dir: Some(cache.path().to_path_buf()),
                system_paths: false,
                model,
            };

            assert_eq!(
                resolve_tessdata_path_in(&langs, Some(explicit.path()), &env).unwrap(),
                explicit.path().to_str().unwrap(),
                "{model}"
            );
            assert_eq!(
                resolve_tessdata_path_in(&langs, None, &env).unwrap(),
                prefix.path().to_str().unwrap(),
                "{model}"
            );
        }
    }

    #[test]
    fn test_tessdata_env_reads_tessdata_prefix_and_xberg_cache_dir() {
        let requested = std::cell::RefCell::new(Vec::new());
        let tessdata_env = TessdataEnv::from_lookup(|name| {
            requested.borrow_mut().push(name.to_string());
            match name {
                "TESSDATA_PREFIX" => Some("/looked/up/prefix".to_string()),
                "XBERG_CACHE_DIR" => Some("/looked/up/cache".to_string()),
                _ => None,
            }
        });

        assert_eq!(
            requested.into_inner(),
            ["TESSDATA_PREFIX", "XBERG_CACHE_DIR", "XBERG_TESSDATA_SYSTEM_PATHS"]
        );
        assert!(tessdata_env.system_paths);
        assert_eq!(tessdata_env.model, TessdataModel::default());
        let cache_tessdata = PathBuf::from("/looked/up/cache").join("tessdata");
        let dirs = tessdata_search_dirs(None, &tessdata_env.for_model(TessdataModel::Fast));
        assert_eq!(
            dirs[..3],
            [
                "/looked/up/prefix".to_string(),
                cache_tessdata.join("fast").to_string_lossy().into_owned(),
                cache_tessdata.to_string_lossy().into_owned(),
            ]
        );
    }

    #[test]
    fn test_tessdata_search_dirs_ends_with_system_paths_when_the_switch_is_unset() {
        let tessdata_env = TessdataEnv::from_lookup(|_| None).for_model(TessdataModel::Fast);

        let dirs = tessdata_search_dirs(None, &tessdata_env);

        let system: Vec<String> = SYSTEM_TESSDATA_PATHS.iter().map(|path| (*path).to_string()).collect();
        assert!(dirs.ends_with(&system), "{dirs:?}");
    }

    #[test]
    fn test_tessdata_search_dirs_omits_system_paths_when_the_switch_is_off() {
        let dirs = tessdata_search_dirs(
            Some(Path::new("/given/override")),
            &given_env(TessdataModel::Fast, false),
        );

        assert_eq!(
            dirs,
            [
                "/given/override".to_string(),
                "/given/prefix".to_string(),
                given_cache(&["tessdata", "fast"]),
                given_cache(&["tessdata"]),
            ]
        );
    }

    #[test]
    fn test_tessdata_env_parses_the_system_paths_switch() {
        let parse = |value: &'static str| {
            TessdataEnv::from_lookup(|name| (name == "XBERG_TESSDATA_SYSTEM_PATHS").then(|| value.to_string()))
                .system_paths
        };

        assert!(!parse("0"));
        assert!(!parse("false"));
        assert!(!parse("FALSE"));
        assert!(parse("1"));
        assert!(parse("true"));
        assert!(parse(""));
    }

    #[test]
    fn test_adopt_file_never_replaces_an_existing_target() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.traineddata");
        let target = directory.path().join("target.traineddata");
        std::fs::write(&source, b"source").unwrap();
        std::fs::write(&target, b"target").unwrap();

        adopt_file(&source, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"target");

        std::fs::remove_file(&target).unwrap();
        adopt_file(&source, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"source");
    }

    #[cfg(feature = "bundle-tessdata-eng")]
    #[test]
    fn test_materialize_eng_from_bundled_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let langs = vec!["eng".to_string()];

        materialize_missing_languages(dir.path(), &langs, TessdataModel::Fast, None).unwrap();
        assert!(dir.path().join("eng.traineddata").exists());
        assert!(all_languages_exist(dir.path().to_str().unwrap(), &langs).unwrap());
    }

    #[test]
    fn test_strip_control_characters() {
        let input = "Hello\x00World\x01Test";
        let output = strip_control_characters(input);
        assert_eq!(output, "HelloWorldTest");

        let input_with_newlines = "Hello\nWorld\rTest\t!";
        let output = strip_control_characters(input_with_newlines);
        assert_eq!(output, "Hello\nWorld\rTest\t!");
    }

    #[test]
    fn test_strip_control_characters_all_control() {
        let input = "\x00\x01\x02\x03";
        let output = strip_control_characters(input);
        assert_eq!(output, "");
    }

    #[test]
    fn test_strip_control_characters_no_control() {
        let input = "Hello World Test";
        let output = strip_control_characters(input);
        assert_eq!(output, "Hello World Test");
    }

    #[test]
    fn test_strip_control_characters_delete_char() {
        let input = "Hello\x7FWorld";
        let output = strip_control_characters(input);
        assert_eq!(output, "HelloWorld");
    }
}
