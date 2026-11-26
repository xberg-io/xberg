use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::{ORT_LIBRARY_NAMES, resolve_ort_library};

/// `<root>/bin/xberg` plus empty `bin/` and `lib/` siblings, the release layout.
struct Layout {
    _root: tempfile::TempDir,
    executable: PathBuf,
    bin: PathBuf,
    lib: PathBuf,
    system: PathBuf,
}

fn layout() -> Layout {
    let root = tempfile::tempdir().expect("create layout root");
    let bin = root.path().join("bin");
    let lib = root.path().join("lib");
    let system = root.path().join("system");
    for directory in [&bin, &lib, &system] {
        std::fs::create_dir(directory).expect("create layout directory");
    }
    let executable = bin.join("xberg");
    std::fs::write(&executable, []).expect("write executable placeholder");
    Layout {
        _root: root,
        executable,
        bin,
        lib,
        system,
    }
}

fn library_name() -> &'static str {
    ORT_LIBRARY_NAMES
        .first()
        .expect("this platform names an ONNX Runtime library")
}

fn place(directory: &Path) -> PathBuf {
    let path = directory.join(library_name());
    std::fs::write(&path, []).expect("write library placeholder");
    path
}

#[test]
fn explicit_path_wins_over_every_discovered_library() {
    let layout = layout();
    place(&layout.bin);
    let system = place(&layout.system);
    let explicit = layout.system.join("explicit-runtime");
    std::fs::write(&explicit, []).unwrap();

    let resolved = resolve_ort_library(
        Some(explicit.as_os_str()),
        Some(layout.executable.as_path()),
        &[system.as_path()],
    );

    assert_eq!(resolved, Ok(explicit));
}

#[test]
fn missing_explicit_path_is_returned_rather_than_falling_back() {
    let layout = layout();
    place(&layout.bin);
    let explicit = layout.system.join("not-installed");

    let resolved = resolve_ort_library(Some(explicit.as_os_str()), Some(layout.executable.as_path()), &[]);

    assert_eq!(resolved, Ok(explicit));
}

#[test]
fn empty_explicit_path_counts_as_unset() {
    let layout = layout();
    let beside = place(&layout.bin);

    let resolved = resolve_ort_library(Some(OsStr::new("")), Some(layout.executable.as_path()), &[]);

    assert_eq!(resolved, Ok(beside));
}

#[test]
fn library_beside_the_executable_wins_over_lib_and_system() {
    let layout = layout();
    let beside = place(&layout.bin);
    place(&layout.lib);
    let system = place(&layout.system);

    let resolved = resolve_ort_library(None, Some(layout.executable.as_path()), &[system.as_path()]);

    assert_eq!(resolved, Ok(beside));
}

#[test]
fn sibling_lib_directory_wins_over_system_paths() {
    let layout = layout();
    let sibling = place(&layout.lib);
    let system = place(&layout.system);

    let resolved = resolve_ort_library(None, Some(layout.executable.as_path()), &[system.as_path()]);

    assert_eq!(resolved, Ok(sibling));
}

#[test]
fn system_path_is_the_last_resort() {
    let layout = layout();
    let system = place(&layout.system);

    let resolved = resolve_ort_library(None, Some(layout.executable.as_path()), &[system.as_path()]);

    assert_eq!(resolved, Ok(system));
}

#[test]
fn directory_named_like_the_library_is_not_loaded() {
    let layout = layout();
    std::fs::create_dir(layout.bin.join(library_name())).unwrap();
    let system = place(&layout.system);

    let resolved = resolve_ort_library(None, Some(layout.executable.as_path()), &[system.as_path()]);

    assert_eq!(resolved, Ok(system));
}

#[test]
fn nothing_found_reports_every_path_searched_in_order() {
    let layout = layout();
    let system = layout.system.join(library_name());

    let tried = resolve_ort_library(None, Some(layout.executable.as_path()), &[system.as_path()])
        .expect_err("no library is installed");

    let mut expected: Vec<PathBuf> = ORT_LIBRARY_NAMES.iter().map(|name| layout.bin.join(name)).collect();
    expected.extend(ORT_LIBRARY_NAMES.iter().map(|name| layout.lib.join(name)));
    expected.push(system);
    assert_eq!(tried, expected);
    assert!(tried.iter().all(|path| path.is_absolute()), "{tried:?}");
}

#[test]
fn unknown_executable_searches_only_system_paths() {
    let layout = layout();
    let system = layout.system.join(library_name());

    let tried = resolve_ort_library(None, None, &[system.as_path()]).expect_err("no library is installed");

    assert_eq!(tried, vec![system]);
}
