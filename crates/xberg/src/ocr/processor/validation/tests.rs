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
        given_cache(&["tessdata"]),
        given_cache(&["tessdata", "fast"]),
    ];
    expected.extend(SYSTEM_TESSDATA_PATHS.iter().map(|path| (*path).to_string()));
    assert_eq!(dirs, expected);
}

const GIVEN_SYSTEM: SystemTessdataDirs<'static> = SystemTessdataDirs {
    fast: &["/given/system/fast"],
    best: &["/given/system/best"],
};

#[test]
fn test_best_search_dirs_hold_no_fast_only_directory() {
    let dirs = tessdata_search_dirs_with(
        Some(Path::new("/given/override")),
        &given_env(TessdataModel::Best, true),
        &GIVEN_SYSTEM,
    );

    assert_eq!(
        dirs,
        [
            "/given/override".to_string(),
            "/given/prefix".to_string(),
            given_cache(&["tessdata", "best"]),
            "/given/system/best".to_string(),
        ]
    );
    let without_system = tessdata_search_dirs_with(None, &given_env(TessdataModel::Best, false), &GIVEN_SYSTEM);
    assert!(!without_system.contains(&"/given/system/best".to_string()));
}

#[test]
fn test_fast_search_dirs_never_hold_the_built_in_best_directory() {
    let dirs = tessdata_search_dirs_with(None, &given_env(TessdataModel::Fast, true), &GIVEN_SYSTEM);

    assert!(dirs.contains(&"/given/system/fast".to_string()));
    assert!(!dirs.contains(&"/given/system/best".to_string()), "{dirs:?}");
    #[cfg(unix)]
    assert_eq!(SYSTEM_TESSDATA_DIRS.best, ["/usr/share/xberg/tessdata/best"]);
    #[cfg(not(unix))]
    assert!(
        SYSTEM_TESSDATA_DIRS.best.is_empty(),
        "the built-in best directory is Unix-only"
    );
    assert!(!SYSTEM_TESSDATA_DIRS.fast.contains(&"/usr/share/xberg/tessdata/best"));
}

#[test]
fn test_adoption_sources_per_model() {
    let base = Path::new("/given/cache");
    let sources = |model, system_paths| adoption_sources(base, &given_env(model, system_paths), &GIVEN_SYSTEM);

    assert_eq!(
        sources(TessdataModel::Best, true),
        [
            base.join("tessdata").join("fast"),
            base.join("tessdata"),
            PathBuf::from("/given/system/best"),
            PathBuf::from("/given/system/fast"),
        ]
    );
    assert_eq!(
        sources(TessdataModel::Fast, true),
        [base.join("tessdata"), PathBuf::from("/given/system/fast")]
    );
    assert_eq!(sources(TessdataModel::Fast, false), [base.join("tessdata")]);
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
    let fast_system = tempfile::tempdir().unwrap();
    let best_system = tempfile::tempdir().unwrap();
    for directory in [
        cache.path().join("tessdata"),
        cache.path().join("tessdata").join("fast"),
        fast_system.path().to_path_buf(),
    ] {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("eng.traineddata"), b"fast").unwrap();
    }
    let fast_dirs = [fast_system.path().to_str().unwrap()];
    let best_dirs = [best_system.path().to_str().unwrap()];
    let system = SystemTessdataDirs {
        fast: &fast_dirs,
        best: &best_dirs,
    };
    let env = |model| TessdataEnv {
        tessdata_prefix: None,
        cache_dir: Some(cache.path().to_path_buf()),
        system_paths: true,
        model,
    };
    let langs = vec!["eng".to_string()];
    let find = |model| first_directory_holding(&tessdata_search_dirs_with(None, &env(model), &system), &langs);

    assert_eq!(
        find(TessdataModel::Fast).as_deref(),
        cache.path().join("tessdata").to_str(),
        "the flat cache comes first for fast"
    );
    assert_eq!(
        find(TessdataModel::Best),
        None,
        "a best request must not be served fast data"
    );

    std::fs::remove_dir_all(cache.path().join("tessdata")).unwrap();
    assert_eq!(find(TessdataModel::Fast).as_deref(), fast_system.path().to_str());
    assert_eq!(find(TessdataModel::Best), None, "system packages hold fast data");

    std::fs::write(best_system.path().join("eng.traineddata"), b"best").unwrap();
    assert_eq!(find(TessdataModel::Best).as_deref(), best_system.path().to_str());
    assert_eq!(
        find(TessdataModel::Fast).as_deref(),
        fast_system.path().to_str(),
        "the built-in best directory is never searched for fast"
    );
}

#[test]
fn test_doctor_flags_best_served_by_tessdata_prefix_or_a_system_path() {
    let env = given_env(TessdataModel::Best, true);

    assert!(best_served_by_unverified_dir("/given/prefix", &env));
    assert!(best_served_by_unverified_dir(
        "/usr/share/tesseract-ocr/5/tessdata",
        &env
    ));
    assert!(!best_served_by_unverified_dir("/opt/models", &env));
    assert!(!best_served_by_unverified_dir(
        "/given/prefix",
        &given_env(TessdataModel::Fast, true)
    ));
}

#[test]
fn test_best_explicit_directory_warning_fires_once_per_directory() {
    let directory = tempfile::tempdir().unwrap();
    let directory = directory.path().to_str().unwrap();

    assert!(first_best_warning_for(directory));
    assert!(!first_best_warning_for(directory));
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
            cache_tessdata.to_string_lossy().into_owned(),
            cache_tessdata.join("fast").to_string_lossy().into_owned(),
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
            given_cache(&["tessdata"]),
            given_cache(&["tessdata", "fast"]),
        ]
    );
}

#[test]
fn test_tessdata_env_parses_the_system_paths_switch() {
    let parse = |value: &'static str| {
        TessdataEnv::from_lookup(|name| (name == "XBERG_TESSDATA_SYSTEM_PATHS").then(|| value.to_string())).system_paths
    };

    assert!(!parse("0"));
    assert!(!parse("false"));
    assert!(!parse("FALSE"));
    assert!(parse("1"));
    assert!(parse("true"));
    assert!(parse(""));
}

#[cfg(feature = "bundle-tessdata-eng")]
#[test]
fn test_materialize_eng_from_bundled_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let langs = vec!["eng".to_string()];

    materialize_missing_languages(dir.path(), &langs, TessdataModel::Fast, &[], &PINNED).unwrap();
    assert!(dir.path().join("eng.traineddata").exists());
    assert!(all_languages_exist(dir.path().to_str().unwrap(), &langs).unwrap());
}

const NO_SYSTEM: SystemTessdataDirs<'static> = SystemTessdataDirs { fast: &[], best: &[] };

fn cache_env(cache: &Path, model: TessdataModel) -> TessdataEnv {
    TessdataEnv {
        tessdata_prefix: None,
        cache_dir: Some(cache.to_path_buf()),
        system_paths: false,
        model,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest as _, Sha256};
    hex::encode(Sha256::digest(bytes))
}

#[test]
fn test_best_resolution_copies_a_verified_pack_from_the_fast_cache() {
    let cache = tempfile::tempdir().unwrap();
    let fast = model_cache_dir(cache.path(), TessdataModel::Fast);
    std::fs::create_dir_all(&fast).unwrap();
    std::fs::write(fast.join("zzz.traineddata"), b"shared model").unwrap();
    let digest = sha256_hex(b"shared model");
    let pin = [("zzz", digest.as_str(), 12)];
    let pins = PinTables { fast: &pin, best: &pin };

    let resolved = resolve_tessdata_path_with(
        &["zzz".to_string()],
        None,
        &cache_env(cache.path(), TessdataModel::Best),
        &NO_SYSTEM,
        &pins,
    )
    .unwrap();

    let best = model_cache_dir(cache.path(), TessdataModel::Best);
    assert_eq!(resolved, best.to_str().unwrap());
    assert_eq!(std::fs::read(best.join("zzz.traineddata")).unwrap(), b"shared model");
}

#[test]
fn test_shared_packs_reach_the_best_directory_once_a_source_appears() {
    let cache = tempfile::tempdir().unwrap();
    let best = model_cache_dir(cache.path(), TessdataModel::Best);
    let fast = model_cache_dir(cache.path(), TessdataModel::Fast);
    std::fs::create_dir_all(&best).unwrap();
    std::fs::write(best.join("eng.traineddata"), b"best english").unwrap();
    let digest = sha256_hex(b"orientation");
    let pin = [("osd", digest.as_str(), 11)];
    let pins = PinTables { fast: &pin, best: &pin };
    let resolve = || {
        resolve_tessdata_path_with(
            &["eng".to_string()],
            None,
            &cache_env(cache.path(), TessdataModel::Best),
            &NO_SYSTEM,
            &pins,
        )
        .unwrap()
    };

    assert_eq!(resolve(), best.to_str().unwrap());
    assert!(!best.join("osd.traineddata").exists(), "nothing to adopt yet");

    std::fs::create_dir_all(&fast).unwrap();
    std::fs::write(fast.join("osd.traineddata"), b"orientation").unwrap();
    assert_eq!(resolve(), best.to_str().unwrap());
    assert_eq!(
        std::fs::read(best.join("osd.traineddata")).unwrap(),
        b"orientation",
        "a directory is not skipped after an earlier attempt found no source"
    );
}

#[test]
fn test_shared_packs_never_adopt_an_unverified_file() {
    let directory = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("osd.traineddata"), b"wrong bytes").unwrap();
    let digest = sha256_hex(b"orientation");
    let pin = [("osd", digest.as_str(), 11)];
    let pins = PinTables { fast: &pin, best: &pin };

    adopt_shared_packs(
        directory.path(),
        TessdataModel::Best,
        &[source.path().to_path_buf()],
        &pins,
    );

    assert!(!directory.path().join("osd.traineddata").exists());
}

#[test]
fn test_a_custom_pack_in_the_flat_cache_keeps_serving_fast_requests() {
    let cache = tempfile::tempdir().unwrap();
    let flat = cache.path().join("tessdata");
    std::fs::create_dir_all(&flat).unwrap();
    std::fs::write(flat.join("eng.traineddata"), b"custom english").unwrap();
    std::fs::write(flat.join("fra.traineddata"), b"french").unwrap();
    let env = cache_env(cache.path(), TessdataModel::Fast);

    let resolved =
        resolve_tessdata_path_with(&["eng".to_string(), "fra".to_string()], None, &env, &NO_SYSTEM, &PINNED).unwrap();

    assert_eq!(resolved, flat.to_str().unwrap());
    assert_eq!(std::fs::read(flat.join("eng.traineddata")).unwrap(), b"custom english");
    assert!(!model_cache_dir(cache.path(), TessdataModel::Fast).exists());
}

#[test]
fn test_a_flat_cache_holding_part_of_a_request_stays_the_fast_destination() {
    let cache = tempfile::tempdir().unwrap();
    let flat = cache.path().join("tessdata");
    std::fs::create_dir_all(&flat).unwrap();
    std::fs::write(flat.join("eng.traineddata"), b"custom english").unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::fs::write(elsewhere.path().join("fra.traineddata"), b"french").unwrap();
    let elsewhere_dirs = [elsewhere.path().to_str().unwrap()];
    let system = SystemTessdataDirs {
        fast: &elsewhere_dirs,
        best: &[],
    };
    let digest = sha256_hex(b"french");
    let fast_pins = [("fra", digest.as_str(), 6)];
    let pins = PinTables {
        fast: &fast_pins,
        best: &[],
    };
    let env = TessdataEnv {
        system_paths: true,
        ..cache_env(cache.path(), TessdataModel::Fast)
    };

    let resolved =
        resolve_tessdata_path_with(&["eng".to_string(), "fra".to_string()], None, &env, &system, &pins).unwrap();

    assert_eq!(resolved, flat.to_str().unwrap());
    assert_eq!(std::fs::read(flat.join("eng.traineddata")).unwrap(), b"custom english");
    assert_eq!(std::fs::read(flat.join("fra.traineddata")).unwrap(), b"french");
    assert!(!model_cache_dir(cache.path(), TessdataModel::Fast).exists());
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
