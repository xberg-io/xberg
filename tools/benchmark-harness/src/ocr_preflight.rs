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

use std::path::{Path, PathBuf};

use crate::cohort::CohortManifest;
use crate::error::{Error, Result};
use crate::fixture::{Fixture, FixtureManager};

/// Tessdata directories xberg searches for `model`, mirroring the fallback chain in
/// `xberg`'s `resolve_tessdata_path` (minus the per-call `tessdata_path`
/// override, which the benchmark never sets). Kept intentionally in sync.
fn candidate_tessdata_dirs(model: xberg::TessdataModel) -> Vec<PathBuf> {
    let prefix = std::env::var("TESSDATA_PREFIX")
        .ok()
        .filter(|prefix| !prefix.is_empty())
        .map(PathBuf::from);
    // XBERG_CACHE_DIR, else the platform cache dir under `xberg`, else
    // `~/.cache/xberg`, else `./.xberg`.
    let cache_base = if let Ok(cache) = std::env::var("XBERG_CACHE_DIR")
        && !cache.is_empty()
    {
        Some(PathBuf::from(cache))
    } else if let Some(cache) = dirs::cache_dir() {
        Some(cache.join("xberg"))
    } else if let Some(home) = dirs::home_dir() {
        Some(home.join(".cache").join("xberg"))
    } else {
        std::env::current_dir().ok().map(|cwd| cwd.join(".xberg"))
    };
    candidate_tessdata_dirs_for(model, prefix, cache_base)
}

/// The search chain for `model` from the given `TESSDATA_PREFIX` and cache base, in the order of
/// xberg's `tessdata_search_dirs` (`crates/xberg/src/ocr/processor/validation.rs`, crate-private,
/// so mirrored here): `fast` reads the flat `<cache>/tessdata` older releases filled,
/// `<cache>/tessdata/fast` and the system packages; `best` reads only `<cache>/tessdata/best` and,
/// on Unix, the read-only image directory, so a `best` run whose packs are missing fails here
/// instead of downloading inside timed rows.
fn candidate_tessdata_dirs_for(
    model: xberg::TessdataModel,
    prefix: Option<PathBuf>,
    cache_base: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = prefix.into_iter().collect();
    match model {
        xberg::TessdataModel::Best => {
            if let Some(cache_base) = &cache_base {
                dirs.push(cache_base.join("tessdata").join(model.as_str()));
            }
            if cfg!(unix) {
                dirs.push(PathBuf::from("/usr/share/xberg/tessdata/best"));
            }
        }
        xberg::TessdataModel::Fast => {
            if let Some(cache_base) = &cache_base {
                dirs.push(cache_base.join("tessdata"));
                dirs.push(cache_base.join("tessdata").join(model.as_str()));
            }
            dirs.extend(
                [
                    "/opt/homebrew/share/tessdata",
                    "/opt/homebrew/opt/tesseract/share/tessdata",
                    "/usr/local/opt/tesseract/share/tessdata",
                    "/usr/share/tesseract-ocr/5/tessdata",
                    "/usr/share/tesseract-ocr/4/tessdata",
                    "/usr/share/tessdata",
                    "/usr/local/share/tessdata",
                ]
                .map(PathBuf::from),
            );
        }
    }
    dirs
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
    dirs.iter()
        .any(|dir| languages.iter().all(|language| traineddata_present(dir, language)))
}

/// xberg's default OCR language when a fixture pins none. It is NOT bundled on
/// the native harness build (see module docs), so it is checked like any pack.
const DEFAULT_OCR_LANGUAGE: &str = "eng";

/// Split a Tesseract language request (`"deu"`, `"deu+eng"`) into its codes.
fn requested_languages(language: &str) -> impl Iterator<Item = &str> {
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
    ensure_resolvable_in(fixtures, ocr_enabled, &candidate_tessdata_dirs(model))
}

/// Core check against an explicit set of tessdata dirs (dependency-injected so
/// tests are deterministic and never depend on the host's installed packs).
fn ensure_resolvable_in(fixtures: &[(PathBuf, Fixture)], ocr_enabled: bool, dirs: &[PathBuf]) -> Result<()> {
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
            return Err(missing_pack_error(path, explicit, missing, dirs));
        }
        return Err(split_pack_error(path, explicit, &requested, dirs));
    }

    Ok(())
}

/// Build the actionable error for a missing pack, distinguishing an explicit pin
/// from the implicit default so the message points at the real cause.
fn missing_pack_error(path: &Path, explicit: Option<&str>, lang: &str, dirs: &[PathBuf]) -> Error {
    let searched = dirs
        .iter()
        .map(|d| d.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let requested = match explicit {
        Some(language) => format!("requests OCR language '{language}'"),
        None => format!("defaults to OCR language '{DEFAULT_OCR_LANGUAGE}'"),
    };
    Error::Config(format!(
        "fixture '{}' {}, but '{}.traineddata' was not found in any known tessdata directory ({}). \
         Install the pack before benchmarking so it is not downloaded inside the timed run (Linux: \
         `apt-get install tesseract-ocr-{}`; macOS: `brew install tesseract-lang`), or set TESSDATA_PREFIX \
         to a directory that contains it.",
        path.display(),
        requested,
        lang,
        searched,
        lang,
    ))
}

fn split_pack_error(path: &Path, explicit: Option<&str>, languages: &[&str], dirs: &[PathBuf]) -> Error {
    let searched = dirs
        .iter()
        .map(|dir| dir.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Error::Config(format!(
        "fixture '{}' requests combined OCR language '{}', but no single tessdata directory contains all packs ({searched}). \
         Place {} together in one directory and set TESSDATA_PREFIX to it before benchmarking.",
        path.display(),
        explicit.unwrap_or(DEFAULT_OCR_LANGUAGE),
        languages
            .iter()
            .map(|language| format!("{language}.traineddata"))
            .collect::<Vec<_>>()
            .join(", "),
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
    fn a_best_run_never_counts_fast_directories() {
        let cache = TempDir::new().unwrap();
        let base = Some(cache.path().to_path_buf());
        let best = candidate_tessdata_dirs_for(xberg::TessdataModel::Best, None, base.clone());
        let fast = candidate_tessdata_dirs_for(xberg::TessdataModel::Fast, None, base);

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
        assert!(ensure_resolvable_in(&fixtures, true, &fast).is_ok());
        assert!(
            ensure_resolvable_in(&fixtures, true, &best).is_err(),
            "fast packs must not satisfy a best run"
        );
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
        let error = ensure_resolvable_in(&fixtures, false, &[dir.path().to_path_buf()]).unwrap_err();
        assert!(error.to_string().contains("deu.traineddata"));
    }

    #[test]
    fn should_require_default_eng_for_unpinned_ocr_fixture() {
        // No explicit language + Tesseract backend defaults to eng, which is not
        // embedded on native, so a missing eng pack must fail.
        let dir = TempDir::new().unwrap();
        let fixtures = vec![fixture_with_language(None)];
        let error = ensure_resolvable_in(&fixtures, true, &[dir.path().to_path_buf()]).unwrap_err();
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
        assert!(ensure_resolvable_in(&fixtures, true, &[dir.path().to_path_buf()]).is_ok());
    }

    #[test]
    fn should_pass_for_explicit_pack_present_on_disk() {
        let dir = tessdata_dir_with(&["deu"]);
        let fixtures = vec![fixture_with_language(Some("deu"))];
        assert!(ensure_resolvable_in(&fixtures, true, &[dir.path().to_path_buf()]).is_ok());
    }

    #[test]
    fn fixture_backend_metadata_does_not_bypass_selected_tesseract_preflight() {
        let dir = TempDir::new().unwrap();
        let fixtures = vec![fixture_with(Some("zzz"), Some("paddle"))];
        let error = ensure_resolvable_in(&fixtures, true, &[dir.path().to_path_buf()]).unwrap_err();
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
        let error = ensure_resolvable_in(&fixtures, true, &[dir.path().to_path_buf()]).unwrap_err();
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
        let error = ensure_resolvable_in(&fixtures, true, &[dir.path().to_path_buf()]).unwrap_err();
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
        let error = ensure_resolvable_in(&fixtures, true, &dirs).unwrap_err();
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
