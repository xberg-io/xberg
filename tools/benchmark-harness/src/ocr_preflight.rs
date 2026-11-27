//! Pre-flight validation that fixture-requested OCR language packs are present
//! locally before a timed benchmark run.
//!
//! A fixture may pin a Tesseract language via `metadata.ocr_language` (e.g.
//! `"deu"`). If that pack is not already on disk, xberg materializes it by
//! downloading `<lang>.traineddata` from the tessdata_fast repository *inside*
//! the extraction call (see xberg's `resolve_tessdata_path` fallback chain).
//! During a benchmark that download lands in the timed region, inflating the
//! measured latency and risking a per-document timeout — silently corrupting
//! the result.
//!
//! No language is exempt — not even `eng`. The `bundle-tessdata-eng` feature
//! that embeds `eng.traineddata` is enabled only on xberg's `wasm32` target
//! dependency, never in `full` (which is how this harness links xberg). On the
//! native benchmark build `xberg_tesseract::bundled_eng_traineddata()` returns
//! `None`, so `materialize_missing_languages` (xberg `validation.rs`) downloads
//! `eng` from GitHub exactly like any other pack when it is absent on disk.
//! Every requested pack must therefore already be resolvable on disk.
//!
//! This guard fails fast, before any framework runs, with an actionable hint.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::cohort::CohortManifest;
use crate::error::{Error, Result};
use crate::fixture::{Fixture, FixtureManager};

/// The variable xberg's CLI reads for the Tesseract model set when no flag names one.
const TESSDATA_MODEL_ENV_VAR: &str = "XBERG_OCR_TESSDATA_MODEL";

/// Fail when the harness environment sets `XBERG_OCR_TESSDATA_MODEL` to anything but `requested`.
///
/// Subprocess rows name the set on argv, which overrides the variable, and in-process rows never
/// read it, so a differing value could not change a row; it is refused anyway because it says the
/// operator expects another set than the one this run records.
pub fn ensure_ambient_tessdata_model_matches(requested: xberg::TessdataModel) -> Result<()> {
    ambient_tessdata_model_matches(requested, std::env::var_os(TESSDATA_MODEL_ENV_VAR).as_deref())
}

fn ambient_tessdata_model_matches(requested: xberg::TessdataModel, ambient: Option<&OsStr>) -> Result<()> {
    let Some(ambient) = ambient else {
        return Ok(());
    };
    let parsed = ambient
        .to_str()
        .and_then(|value| value.parse::<xberg::TessdataModel>().ok());
    if parsed == Some(requested) {
        return Ok(());
    }
    let requested = requested.as_str();
    Err(Error::Config(format!(
        "{TESSDATA_MODEL_ENV_VAR}={} is set in the environment, but this run uses --tessdata-model {requested}.          Unset {TESSDATA_MODEL_ENV_VAR} or set it to '{requested}', or pass the --tessdata-model you mean to measure.",
        ambient.to_string_lossy(),
    )))
}

/// The environment values xberg's tessdata resolver reads: `TessdataEnv` in
/// `crates/xberg/src/ocr/processor/validation.rs`, crate-private, so mirrored here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TessdataEnvironment {
    /// `TESSDATA_PREFIX`; xberg skips an empty value.
    prefix: Option<PathBuf>,
    /// `XBERG_CACHE_DIR` exactly as set. xberg does not treat an empty value as unset: it becomes
    /// an empty cache base, so the cache directories are `tessdata[/<model>]` relative to the
    /// working directory.
    cache_dir: Option<PathBuf>,
    /// False when `XBERG_TESSDATA_SYSTEM_PATHS` is `0` or `false`: xberg then skips the system
    /// and image directories.
    system_paths: bool,
}

impl TessdataEnvironment {
    pub(crate) fn from_process() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        Self {
            prefix: lookup("TESSDATA_PREFIX")
                .filter(|prefix| !prefix.is_empty())
                .map(PathBuf::from),
            cache_dir: lookup("XBERG_CACHE_DIR").map(PathBuf::from),
            system_paths: !lookup("XBERG_TESSDATA_SYSTEM_PATHS").is_some_and(|value| {
                let value = value.trim();
                value == "0" || value.eq_ignore_ascii_case("false")
            }),
        }
    }

    /// A path-free name for one of the searched directories, for provenance: `TESSDATA_PREFIX`,
    /// `<cache>/tessdata[/<model>]`, or the system or image directory itself, which is the same
    /// on every machine.
    pub(crate) fn directory_label(&self, dir: &Path) -> String {
        if self.prefix.as_deref() == Some(dir) {
            return "TESSDATA_PREFIX".to_string();
        }
        let cache = self.cache_base().join("tessdata");
        if dir == cache {
            return "<cache>/tessdata".to_string();
        }
        for model in [xberg::TessdataModel::Fast, xberg::TessdataModel::Best] {
            if dir == cache.join(model.as_str()) {
                return format!("<cache>/tessdata/{}", model.as_str());
            }
        }
        dir.display().to_string()
    }

    /// xberg's cache base (`cache_base_for` in `crates/xberg/src/cache_dir.rs`): `XBERG_CACHE_DIR`
    /// as given, else the platform cache dir under `xberg`, else `~/.cache/xberg`, else `./.xberg`.
    fn cache_base(&self) -> PathBuf {
        if let Some(cache_dir) = &self.cache_dir {
            return cache_dir.clone();
        }
        if let Some(cache) = dirs::cache_dir() {
            return cache.join("xberg");
        }
        if let Some(home) = dirs::home_dir() {
            return home.join(".cache").join("xberg");
        }
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".xberg")
    }
}

/// Package-manager tessdata directories, which ship `tessdata_fast` (xberg's
/// `SYSTEM_TESSDATA_PATHS`).
const SYSTEM_FAST_TESSDATA_DIRS: [&str; 9] = [
    "/opt/homebrew/share/tessdata",
    "/opt/homebrew/opt/tesseract/share/tessdata",
    "/usr/local/opt/tesseract/share/tessdata",
    "/usr/share/tesseract-ocr/5/tessdata",
    "/usr/share/tesseract-ocr/4/tessdata",
    "/usr/share/tessdata",
    "/usr/local/share/tessdata",
    r"C:\Program Files\Tesseract-OCR\tessdata",
    r"C:\ProgramData\Tesseract-OCR\tessdata",
];

/// Where an image can ship read-only `tessdata_best` packs (xberg's `BUILTIN_BEST_TESSDATA_DIRS`).
const IMAGE_BEST_TESSDATA_DIR: &str = "/usr/share/xberg/tessdata/best";

/// The directories xberg searches for `model`, in the order of its `tessdata_search_dirs`
/// (`crates/xberg/src/ocr/processor/validation.rs`, crate-private, so mirrored here and kept in
/// sync), minus the per-call `tessdata_path` override, which the benchmark never sets:
/// `TESSDATA_PREFIX`, then for `fast` the flat `<cache>/tessdata` older releases filled,
/// `<cache>/tessdata/fast` and the system packages, and for `best` only `<cache>/tessdata/best`
/// and, on Unix, the read-only image directory. A run whose packs are in none of them fails here
/// instead of downloading inside timed rows.
pub(crate) fn candidate_tessdata_dirs_for(
    model: xberg::TessdataModel,
    environment: &TessdataEnvironment,
) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = environment.prefix.iter().cloned().collect();
    let cache = environment.cache_base().join("tessdata");
    match model {
        xberg::TessdataModel::Best => {
            dirs.push(cache.join(model.as_str()));
            if environment.system_paths && cfg!(unix) {
                dirs.push(PathBuf::from(IMAGE_BEST_TESSDATA_DIR));
            }
        }
        xberg::TessdataModel::Fast => {
            dirs.push(cache.clone());
            dirs.push(cache.join(model.as_str()));
            if environment.system_paths {
                dirs.extend(SYSTEM_FAST_TESSDATA_DIRS.map(PathBuf::from));
            }
        }
    }
    dirs
}

/// Refuse a `best` run while `TESSDATA_PREFIX` is set.
///
/// xberg uses an explicit `TESSDATA_PREFIX` directory as given for either model set once it holds
/// the requested languages, so a `best` run would read whatever that directory contains. xberg's
/// pinned pack digests are crate-private, so the harness cannot tell a `tessdata_best` pack from
/// a `tessdata_fast` one there and refuses instead of recording unverified rows as `best`.
fn refuse_unverifiable_best_prefix(model: xberg::TessdataModel, environment: &TessdataEnvironment) -> Result<()> {
    let (xberg::TessdataModel::Best, Some(prefix)) = (model, &environment.prefix) else {
        return Ok(());
    };
    Err(Error::Config(format!(
        "--tessdata-model best cannot run while TESSDATA_PREFIX is set ('{}'): xberg reads an explicit \
         TESSDATA_PREFIX directory as given for either model set, and the harness cannot verify that its packs \
         are tessdata_best, so the rows could be measured on fast models and recorded as best. Unset \
         TESSDATA_PREFIX and provision the packs where xberg keeps the best set, '{}' (`xberg cache warm \
         --tessdata-model best --tessdata-languages <codes>`{}).",
        prefix.display(),
        environment.cache_base().join("tessdata").join("best").display(),
        if cfg!(unix) {
            format!(", or the read-only image directory '{IMAGE_BEST_TESSDATA_DIR}'")
        } else {
            String::new()
        },
    )))
}

/// The first of `dirs` holding every pack of `languages`: the directory xberg resolves the
/// request to (`first_directory_holding` in xberg's `validation.rs`).
pub(crate) fn first_dir_holding<'a>(languages: &[&str], dirs: &'a [PathBuf]) -> Option<&'a Path> {
    dirs.iter()
        .find(|dir| languages.iter().all(|language| traineddata_present(dir, language)))
        .map(PathBuf::as_path)
}

/// True when `<lang>.traineddata` exists as a file in `dir`.
fn traineddata_present(dir: &Path, lang: &str) -> bool {
    dir.join(format!("{lang}.traineddata")).is_file()
}

/// True when `lang` has a `.traineddata` in any candidate dir. Nothing is
/// exempt: the native harness build embeds no language (see module docs).
fn language_pack_resolvable(lang: &str, dirs: &[PathBuf]) -> bool {
    dirs.iter().any(|dir| traineddata_present(dir, lang))
}

/// True when one candidate directory contains every pack in a combined request.
fn language_set_resolvable(languages: &[&str], dirs: &[PathBuf]) -> bool {
    first_dir_holding(languages, dirs).is_some()
}

/// xberg's default OCR language when a fixture pins none. It is NOT bundled on
/// the native harness build (see module docs), so it is checked like any pack.
pub(crate) const DEFAULT_OCR_LANGUAGE: &str = "eng";

/// Split a Tesseract language request (`"deu"`, `"deu+eng"`) into its codes.
pub(crate) fn requested_languages(language: &str) -> impl Iterator<Item = &str> {
    language.split('+').map(str::trim).filter(|code| !code.is_empty())
}

/// Fail fast if any fixture's OCR language pack is not installed on disk.
///
/// A fixture with no explicit `metadata.ocr_language` still requires xberg's
/// default (`eng`) because the native build downloads it rather than embedding
/// it. Fixtures that force OCR are checked even when the global flag is false.
pub fn ensure_ocr_languages_resolvable(
    fixtures: &[(PathBuf, Fixture)],
    ocr_enabled: bool,
    tesseract_selected: bool,
    model: xberg::TessdataModel,
) -> Result<()> {
    if !tesseract_selected {
        return Ok(());
    }
    ensure_resolvable_for(fixtures, ocr_enabled, model, &TessdataEnvironment::from_process())
}

/// The whole check for one model set and environment.
fn ensure_resolvable_for(
    fixtures: &[(PathBuf, Fixture)],
    ocr_enabled: bool,
    model: xberg::TessdataModel,
    environment: &TessdataEnvironment,
) -> Result<()> {
    refuse_unverifiable_best_prefix(model, environment)?;
    ensure_resolvable_in(
        fixtures,
        ocr_enabled,
        &candidate_tessdata_dirs_for(model, environment),
        model,
    )
}

/// Core check against an explicit set of tessdata dirs (dependency-injected so
/// tests are deterministic and never depend on the host's installed packs).
fn ensure_resolvable_in(
    fixtures: &[(PathBuf, Fixture)],
    ocr_enabled: bool,
    dirs: &[PathBuf],
    model: xberg::TessdataModel,
) -> Result<()> {
    for (path, fixture) in fixtures {
        if !ocr_enabled && !fixture.requires_ocr() {
            continue;
        }

        let explicit = fixture.ocr_language();
        let requested: Vec<&str> = match explicit {
            Some(language) => requested_languages(language).collect(),
            None => vec![DEFAULT_OCR_LANGUAGE],
        };

        if language_set_resolvable(&requested, dirs) {
            continue;
        }
        if let Some(missing) = requested
            .iter()
            .find(|language| !language_pack_resolvable(language, dirs))
        {
            return Err(missing_pack_error(path, explicit, missing, dirs, model));
        }
        return Err(split_pack_error(path, explicit, &requested, dirs, model));
    }

    Ok(())
}

/// Build the actionable error for a missing pack, distinguishing an explicit pin
/// from the implicit default so the message points at the real cause.
fn missing_pack_error(
    path: &Path,
    explicit: Option<&str>,
    lang: &str,
    dirs: &[PathBuf],
    model: xberg::TessdataModel,
) -> Error {
    let searched = dirs
        .iter()
        .map(|d| d.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let requested = match explicit {
        Some(language) => format!("requests OCR language '{language}'"),
        None => format!("defaults to OCR language '{DEFAULT_OCR_LANGUAGE}'"),
    };
    let remedy = match model {
        xberg::TessdataModel::Fast => format!(
            "Install the pack before benchmarking so it is not downloaded inside the timed run (Linux: \
             `apt-get install tesseract-ocr-{lang}`; macOS: `brew install tesseract-lang`), or set TESSDATA_PREFIX \
             to a directory that contains it."
        ),
        xberg::TessdataModel::Best => format!(
            "Provision the tessdata_best pack before benchmarking so it is not downloaded inside the timed run: \
             `xberg cache warm --tessdata-model best --tessdata-languages {lang}`. System tessdata packages and \
             TESSDATA_PREFIX are not read for --tessdata-model best."
        ),
    };
    Error::Config(format!(
        "fixture '{}' {requested}, but '{lang}.traineddata' was not found in any tessdata directory xberg searches \
         for the '{}' model set ({searched}). {remedy}",
        path.display(),
        model.as_str(),
    ))
}

fn split_pack_error(
    path: &Path,
    explicit: Option<&str>,
    languages: &[&str],
    dirs: &[PathBuf],
    model: xberg::TessdataModel,
) -> Error {
    let searched = dirs
        .iter()
        .map(|dir| dir.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let packs = languages
        .iter()
        .map(|language| format!("{language}.traineddata"))
        .collect::<Vec<_>>()
        .join(", ");
    let remedy = match model {
        xberg::TessdataModel::Fast => {
            format!("Place {packs} together in one directory and set TESSDATA_PREFIX to it before benchmarking.")
        }
        xberg::TessdataModel::Best => format!(
            "Provision {packs} together before benchmarking: `xberg cache warm --tessdata-model best \
             --tessdata-languages {}`.",
            languages.join(","),
        ),
    };
    Error::Config(format!(
        "fixture '{}' requests combined OCR language '{}', but no single tessdata directory contains all packs \
         ({searched}). {remedy}",
        path.display(),
        explicit.unwrap_or(DEFAULT_OCR_LANGUAGE),
    ))
}

/// Resolve the effective fixture set (mirroring the runner's load logic) and
/// validate every fixture-requested OCR language pack is present.
///
/// `cohort` selects the exact cohort manifest when set; otherwise every fixture
/// under `fixtures_path` is checked. This re-loads fixtures independently of the
/// runner so it stays decoupled from execution wiring.
pub fn run(
    fixtures_path: &Path,
    cohort: Option<&Path>,
    ocr_enabled: bool,
    tesseract_selected: bool,
    model: xberg::TessdataModel,
) -> Result<()> {
    if !tesseract_selected {
        return Ok(());
    }

    let manager = load_effective_fixtures(fixtures_path, cohort)?;
    ensure_ocr_languages_resolvable(manager.fixtures(), ocr_enabled, tesseract_selected, model)
}

/// Load the fixtures the runner would run, honoring an optional cohort manifest.
/// Mirrors `BenchmarkRunner::{load_cohort, load_fixtures}` using public APIs.
fn load_effective_fixtures(fixtures_path: &Path, cohort: Option<&Path>) -> Result<FixtureManager> {
    if let Some(manifest_path) = cohort {
        let resolved = resolve_cohort_manifest_path(fixtures_path, manifest_path);
        let manifest = CohortManifest::from_file(&resolved).map_err(|error| {
            Error::Config(format!(
                "ocr preflight: failed to load cohort manifest '{}': {error}",
                resolved.display()
            ))
        })?;
        manifest.load_fixtures(fixtures_path, &resolved)
    } else {
        let mut manager = FixtureManager::new();
        if fixtures_path.is_dir() {
            manager.load_fixtures_from_dir(fixtures_path)?;
        } else {
            manager.load_fixture(fixtures_path)?;
        }
        Ok(manager)
    }
}

/// Mirror of the runner's private cohort-path resolution: absolute or existing
/// paths are used as-is, otherwise the path is taken relative to the root.
fn resolve_cohort_manifest_path(fixture_root: &Path, manifest_path: &Path) -> PathBuf {
    if manifest_path.is_absolute() || manifest_path.exists() {
        manifest_path.to_path_buf()
    } else {
        fixture_root.join(manifest_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn fixture_with(language: Option<&str>, backend: Option<&str>) -> (PathBuf, Fixture) {
        let mut metadata = HashMap::new();
        if let Some(language) = language {
            metadata.insert(
                "ocr_language".to_string(),
                serde_json::Value::String(language.to_string()),
            );
        }
        if let Some(backend) = backend {
            metadata.insert(
                "ocr_backend".to_string(),
                serde_json::Value::String(backend.to_string()),
            );
        }
        let fixture = Fixture {
            document: PathBuf::from("doc.pdf"),
            file_type: "pdf".to_string(),
            file_size: 1,
            expected_frameworks: vec!["xberg".to_string()],
            metadata,
            ground_truth: None,
        };
        (PathBuf::from("fixture.json"), fixture)
    }

    fn fixture_with_language(language: Option<&str>) -> (PathBuf, Fixture) {
        fixture_with(language, None)
    }

    fn fixture_requiring_ocr(language: Option<&str>) -> (PathBuf, Fixture) {
        let (path, mut fixture) = fixture_with_language(language);
        fixture
            .metadata
            .insert("requires_ocr".to_string(), serde_json::Value::Bool(true));
        (path, fixture)
    }

    /// A temp dir seeded with the given `<lang>.traineddata` stub files.
    fn tessdata_dir_with(langs: &[&str]) -> TempDir {
        let dir = TempDir::new().unwrap();
        for lang in langs {
            std::fs::write(dir.path().join(format!("{lang}.traineddata")), b"stub").unwrap();
        }
        dir
    }

    #[test]
    fn an_unset_or_matching_ambient_tessdata_model_is_accepted() {
        for model in [xberg::TessdataModel::Fast, xberg::TessdataModel::Best] {
            assert!(ambient_tessdata_model_matches(model, None).is_ok());
            assert!(ambient_tessdata_model_matches(model, Some(OsStr::new(model.as_str()))).is_ok());
        }
    }

    #[test]
    fn a_conflicting_ambient_tessdata_model_is_refused() {
        for (requested, ambient) in [
            (xberg::TessdataModel::Fast, "best"),
            (xberg::TessdataModel::Best, "fast"),
            (xberg::TessdataModel::Fast, "medium"),
            (xberg::TessdataModel::Fast, ""),
            (xberg::TessdataModel::Best, "BEST"),
        ] {
            let error = ambient_tessdata_model_matches(requested, Some(OsStr::new(ambient))).unwrap_err();
            let message = error.to_string();
            assert!(
                message.contains(&format!("XBERG_OCR_TESSDATA_MODEL={ambient} ")),
                "{message}"
            );
            assert!(
                message.contains(&format!("--tessdata-model {}", requested.as_str())),
                "{message}"
            );
        }
    }

    fn environment_with(prefix: Option<&Path>, cache_dir: &Path) -> TessdataEnvironment {
        TessdataEnvironment {
            prefix: prefix.map(Path::to_path_buf),
            cache_dir: Some(cache_dir.to_path_buf()),
            system_paths: true,
        }
    }

    #[test]
    fn a_best_run_never_counts_fast_directories() {
        let cache = TempDir::new().unwrap();
        let environment = environment_with(None, cache.path());
        let best = candidate_tessdata_dirs_for(xberg::TessdataModel::Best, &environment);
        let fast = candidate_tessdata_dirs_for(xberg::TessdataModel::Fast, &environment);

        assert_eq!(best[0], cache.path().join("tessdata").join("best"));
        assert_eq!(best.len(), if cfg!(unix) { 2 } else { 1 });
        assert_eq!(
            fast[..2],
            [
                cache.path().join("tessdata"),
                cache.path().join("tessdata").join("fast")
            ]
        );

        let fast_cache = cache.path().join("tessdata").join("fast");
        std::fs::create_dir_all(&fast_cache).unwrap();
        std::fs::write(fast_cache.join("eng.traineddata"), b"stub").unwrap();
        let fixtures = vec![fixture_with_language(None)];
        assert!(ensure_resolvable_in(&fixtures, true, &fast, xberg::TessdataModel::Fast).is_ok());
        let error = ensure_resolvable_in(&fixtures, true, &best, xberg::TessdataModel::Best)
            .expect_err("fast packs must not satisfy a best run");
        let message = error.to_string();
        assert!(message.contains("xberg cache warm --tessdata-model best"), "{message}");
        assert!(!message.contains("apt-get"), "{message}");
    }

    #[test]
    fn a_best_run_is_refused_while_tessdata_prefix_is_set() {
        let cache = TempDir::new().unwrap();
        let best_cache = cache.path().join("tessdata").join("best");
        std::fs::create_dir_all(&best_cache).unwrap();
        std::fs::write(best_cache.join("eng.traineddata"), b"stub").unwrap();
        let prefix = tessdata_dir_with(&["eng"]);
        let fixtures = vec![fixture_with_language(None)];

        let with_prefix = environment_with(Some(prefix.path()), cache.path());
        let error = ensure_resolvable_for(&fixtures, true, xberg::TessdataModel::Best, &with_prefix).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("TESSDATA_PREFIX is set"), "{message}");
        assert!(message.contains(prefix.path().to_string_lossy().as_ref()), "{message}");
        assert!(message.contains(best_cache.to_string_lossy().as_ref()), "{message}");
        assert!(
            ensure_resolvable_for(&[], false, xberg::TessdataModel::Best, &with_prefix).is_err(),
            "the refusal does not depend on a fixture requesting OCR"
        );

        assert!(ensure_resolvable_for(&fixtures, true, xberg::TessdataModel::Fast, &with_prefix).is_ok());
        let without_prefix = environment_with(None, cache.path());
        assert!(ensure_resolvable_for(&fixtures, true, xberg::TessdataModel::Best, &without_prefix).is_ok());
    }

    #[test]
    fn a_best_run_reads_only_the_best_cache_once_the_prefix_is_unset() {
        let cache = TempDir::new().unwrap();
        let flat = cache.path().join("tessdata");
        std::fs::create_dir_all(&flat).unwrap();
        std::fs::write(flat.join("eng.traineddata"), b"stub").unwrap();
        let fixtures = vec![fixture_with_language(None)];
        let environment = TessdataEnvironment {
            system_paths: false,
            ..environment_with(None, cache.path())
        };

        assert!(ensure_resolvable_for(&fixtures, true, xberg::TessdataModel::Fast, &environment).is_ok());
        assert!(ensure_resolvable_for(&fixtures, true, xberg::TessdataModel::Best, &environment).is_err());
    }

    fn environment_from(values: &[(&str, &str)]) -> TessdataEnvironment {
        TessdataEnvironment::from_lookup(|name| {
            values
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_string())
        })
    }

    #[test]
    fn the_environment_is_read_the_way_xberg_reads_it() {
        let unset = environment_from(&[]);
        assert_eq!(unset.prefix, None);
        assert_eq!(unset.cache_dir, None);
        assert!(unset.system_paths);

        let empty = environment_from(&[("TESSDATA_PREFIX", ""), ("XBERG_CACHE_DIR", "")]);
        assert_eq!(empty.prefix, None, "xberg skips an empty TESSDATA_PREFIX");
        assert_eq!(
            candidate_tessdata_dirs_for(xberg::TessdataModel::Best, &empty)[0],
            Path::new("tessdata").join("best"),
            "an empty XBERG_CACHE_DIR is a working-directory-relative cache, as in xberg"
        );
        assert_eq!(
            candidate_tessdata_dirs_for(xberg::TessdataModel::Fast, &empty)[..2],
            [PathBuf::from("tessdata"), Path::new("tessdata").join("fast")]
        );

        for value in ["0", "false", " FALSE "] {
            let disabled = environment_from(&[("XBERG_CACHE_DIR", "cache"), ("XBERG_TESSDATA_SYSTEM_PATHS", value)]);
            let cache = Path::new("cache").join("tessdata");
            assert!(!disabled.system_paths, "{value}");
            assert_eq!(
                candidate_tessdata_dirs_for(xberg::TessdataModel::Fast, &disabled),
                [cache.clone(), cache.join("fast")]
            );
            assert_eq!(
                candidate_tessdata_dirs_for(xberg::TessdataModel::Best, &disabled),
                [cache.join("best")]
            );
        }
        assert!(environment_from(&[("XBERG_TESSDATA_SYSTEM_PATHS", "1")]).system_paths);
    }

    #[test]
    fn searched_directories_are_labelled_without_local_paths() {
        let cache = TempDir::new().unwrap();
        let prefix = TempDir::new().unwrap();
        let environment = environment_with(Some(prefix.path()), cache.path());
        let mut labels = Vec::new();
        for model in [xberg::TessdataModel::Fast, xberg::TessdataModel::Best] {
            for dir in candidate_tessdata_dirs_for(model, &environment) {
                labels.push(environment.directory_label(&dir));
            }
        }

        assert_eq!(
            labels[..3],
            ["TESSDATA_PREFIX", "<cache>/tessdata", "<cache>/tessdata/fast"]
        );
        assert!(labels.contains(&"<cache>/tessdata/best".to_string()), "{labels:?}");
        assert!(labels.contains(&"/usr/share/tessdata".to_string()), "{labels:?}");
        for label in &labels {
            assert!(!label.contains(cache.path().to_string_lossy().as_ref()), "{label}");
            assert!(!label.contains(prefix.path().to_string_lossy().as_ref()), "{label}");
        }
    }

    #[test]
    fn tessdata_prefix_is_searched_first_for_a_fast_run() {
        let cache = TempDir::new().unwrap();
        let prefix = TempDir::new().unwrap();
        let dirs = candidate_tessdata_dirs_for(
            xberg::TessdataModel::Fast,
            &environment_with(Some(prefix.path()), cache.path()),
        );
        assert_eq!(dirs[0], prefix.path());
        assert_eq!(dirs[1], cache.path().join("tessdata"));
    }

    #[test]
    fn should_pass_when_ocr_disabled_even_if_pack_missing() {
        let fixtures = vec![fixture_with_language(Some("zzz"))];
        assert!(ensure_ocr_languages_resolvable(&fixtures, false, true, xberg::TessdataModel::Fast).is_ok());
        assert!(ensure_ocr_languages_resolvable(&fixtures, true, false, xberg::TessdataModel::Fast).is_ok());
    }

    #[test]
    fn should_check_fixture_forced_ocr_when_global_ocr_is_disabled() {
        let dir = TempDir::new().unwrap();
        let fixtures = vec![fixture_requiring_ocr(Some("deu"))];
        let error = ensure_resolvable_in(
            &fixtures,
            false,
            &[dir.path().to_path_buf()],
            xberg::TessdataModel::Fast,
        )
        .unwrap_err();
        assert!(error.to_string().contains("deu.traineddata"));
    }

    #[test]
    fn should_require_default_eng_for_unpinned_ocr_fixture() {
        // No explicit language + Tesseract backend defaults to eng, which is not
        // embedded on native, so a missing eng pack must fail.
        let dir = TempDir::new().unwrap();
        let fixtures = vec![fixture_with_language(None)];
        let error =
            ensure_resolvable_in(&fixtures, true, &[dir.path().to_path_buf()], xberg::TessdataModel::Fast).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("eng.traineddata"), "should name eng: {message}");
        assert!(
            message.contains("defaults to OCR language 'eng'"),
            "should flag the default: {message}"
        );
    }

    #[test]
    fn should_pass_unpinned_ocr_fixture_when_eng_present() {
        let dir = tessdata_dir_with(&["eng"]);
        let fixtures = vec![fixture_with_language(None)];
        assert!(ensure_resolvable_in(&fixtures, true, &[dir.path().to_path_buf()], xberg::TessdataModel::Fast).is_ok());
    }

    #[test]
    fn should_pass_for_explicit_pack_present_on_disk() {
        let dir = tessdata_dir_with(&["deu"]);
        let fixtures = vec![fixture_with_language(Some("deu"))];
        assert!(ensure_resolvable_in(&fixtures, true, &[dir.path().to_path_buf()], xberg::TessdataModel::Fast).is_ok());
    }

    #[test]
    fn fixture_backend_metadata_does_not_bypass_selected_tesseract_preflight() {
        let dir = TempDir::new().unwrap();
        let fixtures = vec![fixture_with(Some("zzz"), Some("paddle"))];
        let error =
            ensure_resolvable_in(&fixtures, true, &[dir.path().to_path_buf()], xberg::TessdataModel::Fast).unwrap_err();
        assert!(error.to_string().contains("zzz.traineddata"));
    }

    #[test]
    fn should_fail_for_eng_when_no_pack_on_disk_because_native_build_does_not_embed_it() {
        // `bundle-tessdata-eng` is wasm-only; the native harness build downloads
        // eng like any other pack, so a missing eng pack must fail the preflight.
        let dir = TempDir::new().unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        assert!(!language_pack_resolvable("eng", &dirs));
    }

    #[test]
    fn should_pass_for_eng_when_pack_present_on_disk() {
        let dir = tessdata_dir_with(&["eng"]);
        let dirs = vec![dir.path().to_path_buf()];
        assert!(language_pack_resolvable("eng", &dirs));
    }

    #[test]
    fn should_fail_when_requested_pack_is_missing() {
        let dir = TempDir::new().unwrap();
        let fixtures = vec![fixture_with_language(Some("zzz"))];
        let error =
            ensure_resolvable_in(&fixtures, true, &[dir.path().to_path_buf()], xberg::TessdataModel::Fast).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("zzz.traineddata"),
            "message should name the missing pack: {message}"
        );
        assert!(
            message.contains("tesseract-ocr-zzz"),
            "message should include an install hint: {message}"
        );
    }

    #[test]
    fn should_resolve_pack_present_in_tessdata_prefix() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("deu.traineddata"), b"stub").unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        assert!(language_pack_resolvable("deu", &dirs));
        assert!(!language_pack_resolvable("fra", &dirs));
    }

    #[test]
    fn should_reject_only_the_missing_code_in_a_combined_request() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("eng.traineddata"), b"stub").unwrap();
        let fixtures = vec![fixture_with_language(Some("deu+eng"))];
        let error =
            ensure_resolvable_in(&fixtures, true, &[dir.path().to_path_buf()], xberg::TessdataModel::Fast).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("deu.traineddata"));
        assert!(!message.contains("eng.traineddata was not found"));
    }

    #[test]
    fn should_reject_combined_request_split_across_directories() {
        let eng = tessdata_dir_with(&["eng"]);
        let deu = tessdata_dir_with(&["deu"]);
        let fixtures = vec![fixture_with_language(Some("deu+eng"))];
        let dirs = vec![eng.path().to_path_buf(), deu.path().to_path_buf()];
        let error = ensure_resolvable_in(&fixtures, true, &dirs, xberg::TessdataModel::Fast).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("no single tessdata directory"));
        assert!(message.contains("deu.traineddata"));
        assert!(message.contains("eng.traineddata"));
    }

    #[test]
    fn requested_languages_splits_and_trims() {
        let codes: Vec<&str> = requested_languages(" deu + eng ").collect();
        assert_eq!(codes, vec!["deu", "eng"]);
    }
}
