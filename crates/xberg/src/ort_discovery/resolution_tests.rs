use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use super::library::{ORT_LIBRARY_NAMES, OrtSearch, OrtSource, SearchPolicy, resolve_ort_library};

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
        set_mode(directory, 0o755);
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
    set_mode(&path, 0o644);
    path
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("set permissions");
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) {}

/// What gets loaded for a placed library: the path with symlinks resolved.
fn real(path: &Path) -> PathBuf {
    if cfg!(unix) {
        std::fs::canonicalize(path).expect("resolve a placed library")
    } else {
        path.to_path_buf()
    }
}

fn search(source: OrtSource) -> OrtSearch {
    match source {
        OrtSource::Search(search) => search,
        other => panic!("expected a search, got {other:?}"),
    }
}

fn strict(executable: Option<&Path>, system: &[&Path]) -> OrtSearch {
    search(resolve_ort_library(None, executable, system, SearchPolicy::Strict))
}

#[test]
fn explicit_path_wins_over_every_discovered_library() {
    let layout = layout();
    place(&layout.bin);
    let system = place(&layout.system);
    let explicit = layout.system.join("explicit-runtime");
    std::fs::write(&explicit, []).unwrap();

    for policy in [SearchPolicy::Strict, SearchPolicy::LoaderFallback] {
        let resolved = resolve_ort_library(
            Some(explicit.as_os_str()),
            Some(layout.executable.as_path()),
            &[system.as_path()],
            policy,
        );
        assert_eq!(resolved, OrtSource::Explicit(explicit.clone()), "{policy:?}");
    }
}

#[test]
fn missing_explicit_path_is_returned_rather_than_falling_back() {
    let layout = layout();
    place(&layout.bin);
    let explicit = layout.system.join("not-installed");

    let resolved = resolve_ort_library(
        Some(explicit.as_os_str()),
        Some(layout.executable.as_path()),
        &[],
        SearchPolicy::Strict,
    );

    assert_eq!(resolved, OrtSource::Explicit(explicit));
}

#[test]
fn relative_explicit_path_is_refused_by_the_strict_search_only() {
    let layout = layout();
    place(&layout.bin);
    let relative = OsStr::new("lib/libonnxruntime-custom");

    let strict = resolve_ort_library(
        Some(relative),
        Some(layout.executable.as_path()),
        &[],
        SearchPolicy::Strict,
    );
    assert_eq!(strict, OrtSource::RelativeOverride(PathBuf::from(relative)));

    let library = resolve_ort_library(
        Some(relative),
        Some(layout.executable.as_path()),
        &[],
        SearchPolicy::LoaderFallback,
    );
    assert_eq!(library, OrtSource::Explicit(PathBuf::from(relative)));
}

#[test]
fn empty_explicit_path_counts_as_unset() {
    let layout = layout();
    let beside = place(&layout.bin);

    let found = search(resolve_ort_library(
        Some(OsStr::new("")),
        Some(layout.executable.as_path()),
        &[],
        SearchPolicy::Strict,
    ))
    .found;

    assert_eq!(found, vec![real(&beside)]);
}

#[test]
fn library_beside_the_executable_wins_over_lib_and_system() {
    let layout = layout();
    let beside = place(&layout.bin);
    let sibling = place(&layout.lib);
    let system = place(&layout.system);

    let found = strict(Some(layout.executable.as_path()), &[system.as_path()]).found;

    assert_eq!(found, vec![real(&beside), real(&sibling), real(&system)]);
}

#[test]
fn sibling_lib_directory_wins_over_system_paths() {
    let layout = layout();
    let sibling = place(&layout.lib);
    let system = place(&layout.system);

    let found = strict(Some(layout.executable.as_path()), &[system.as_path()]).found;

    assert_eq!(found, vec![real(&sibling), real(&system)]);
}

#[test]
fn lib_beside_a_directory_not_named_bin_is_not_searched() {
    // `/tmp/x/xberg` must not reach `/tmp/lib`, which any local user can create.
    let layout = layout();
    let shared = layout.system.join("x");
    std::fs::create_dir(&shared).unwrap();
    let executable = shared.join("xberg");
    std::fs::write(&executable, []).unwrap();
    let planted = place(&layout.lib);
    let shared_lib = layout.system.join("lib");
    std::fs::create_dir(&shared_lib).unwrap();
    let planted_in_shared_lib = place(&shared_lib);

    let search = strict(Some(executable.as_path()), &[]);

    assert!(search.found.is_empty(), "{:?}", search.found);
    assert!(!search.tried.contains(&planted_in_shared_lib), "{:?}", search.tried);
    assert!(!search.tried.contains(&planted), "{:?}", search.tried);
    let expected: Vec<PathBuf> = ORT_LIBRARY_NAMES.iter().map(|name| shared.join(name)).collect();
    assert_eq!(search.tried, expected);
}

#[test]
fn system_path_is_the_last_resort() {
    let layout = layout();
    let system = place(&layout.system);

    let found = strict(Some(layout.executable.as_path()), &[system.as_path()]).found;

    assert_eq!(found, vec![real(&system)]);
}

#[test]
fn directory_named_like_the_library_is_not_loaded() {
    let layout = layout();
    std::fs::create_dir(layout.bin.join(library_name())).unwrap();
    let system = place(&layout.system);

    let found = strict(Some(layout.executable.as_path()), &[system.as_path()]).found;

    assert_eq!(found, vec![real(&system)]);
}

#[test]
fn nothing_found_reports_every_path_searched_in_order() {
    let layout = layout();
    let system = layout.system.join(library_name());

    let search = strict(Some(layout.executable.as_path()), &[system.as_path()]);

    let mut expected: Vec<PathBuf> = ORT_LIBRARY_NAMES.iter().map(|name| layout.bin.join(name)).collect();
    expected.extend(ORT_LIBRARY_NAMES.iter().map(|name| layout.lib.join(name)));
    expected.push(system);
    assert!(search.found.is_empty());
    assert_eq!(search.tried, expected);
    assert!(search.tried.iter().all(|path| path.is_absolute()), "{:?}", search.tried);
}

#[test]
fn unknown_executable_searches_only_system_paths() {
    let layout = layout();
    let system = layout.system.join(library_name());

    let search = strict(None, &[system.as_path()]);

    assert_eq!(search.tried, vec![system]);
}

#[test]
fn only_the_library_consumer_search_falls_back_to_the_platform_loader() {
    let layout = layout();

    assert_eq!(strict(Some(layout.executable.as_path()), &[]).loader_name, None);
    let library = search(resolve_ort_library(
        None,
        Some(layout.executable.as_path()),
        &[],
        SearchPolicy::LoaderFallback,
    ));
    assert_eq!(library.loader_name, Some(library_name()));
}

#[cfg(unix)]
#[test]
fn a_library_in_a_group_or_world_writable_directory_is_skipped() {
    use std::os::unix::fs::MetadataExt;

    for mode in [0o775, 0o757, 0o777] {
        let layout = layout();
        let planted = place(&layout.bin);
        set_mode(&layout.bin, mode);
        let system = place(&layout.system);
        // Group write through the user's own private group (umask 002 hosts) is not shared.
        let gid = std::fs::metadata(&layout.bin).unwrap().gid();
        let shared = mode & 0o002 != 0 || !super::library::is_private_group(gid, super::library::effective_uid());

        let search = strict(Some(layout.executable.as_path()), &[system.as_path()]);

        if !shared {
            assert_eq!(search.found, vec![real(&planted), real(&system)], "{mode:o}");
            continue;
        }
        assert_eq!(search.found, vec![real(&system)], "{mode:o}");
        assert_eq!(search.untrusted.len(), 1, "{mode:o}: {:?}", search.untrusted);
        assert_eq!(search.untrusted[0].0, planted, "{mode:o}");
        assert!(
            search.untrusted[0].1.contains("writable by group or others"),
            "{mode:o}: {}",
            search.untrusted[0].1
        );
    }
}

#[cfg(unix)]
#[test]
fn only_a_user_private_group_may_have_write_access() {
    use std::ffi::CStr;

    use super::library::private_group;

    let alice = CStr::from_bytes_with_nul(b"alice\0").unwrap();
    let staff = CStr::from_bytes_with_nul(b"staff\0").unwrap();
    assert!(
        private_group(alice, 1000, alice, false, 1000),
        "Debian-style private group"
    );
    assert!(
        !private_group(alice, 20, staff, false, 20),
        "a shared primary group such as macOS staff"
    );
    assert!(
        !private_group(alice, 1000, alice, true, 1000),
        "a same-named group with listed members"
    );
    assert!(
        !private_group(alice, 1000, alice, false, 1001),
        "not the user's primary group"
    );
}

#[cfg(unix)]
#[test]
fn a_group_or_world_writable_library_is_skipped() {
    let layout = layout();
    let planted = place(&layout.bin);
    set_mode(&planted, 0o666);

    let search = strict(Some(layout.executable.as_path()), &[]);

    assert!(search.found.is_empty(), "{:?}", search.found);
    assert_eq!(search.untrusted.len(), 1, "{:?}", search.untrusted);
    assert!(search.untrusted[0].1.contains("mode 666"), "{}", search.untrusted[0].1);
}

#[cfg(unix)]
#[test]
fn a_symlinked_library_is_loaded_through_its_target_and_checked_there() {
    let layout = layout();
    let target = layout.system.join("libonnxruntime.real");
    std::fs::write(&target, []).unwrap();
    set_mode(&target, 0o644);
    std::os::unix::fs::symlink(&target, layout.bin.join(library_name())).unwrap();

    let search = strict(Some(layout.executable.as_path()), &[]);
    assert_eq!(search.found, vec![real(&target)]);

    set_mode(&layout.system, 0o777);
    let search = strict(Some(layout.executable.as_path()), &[]);
    assert!(search.found.is_empty(), "{:?}", search.found);
    assert_eq!(search.untrusted.len(), 1, "{:?}", search.untrusted);
}

#[cfg(unix)]
#[test]
fn the_xberg_image_is_the_file_this_code_runs_from() {
    // A test binary links xberg statically, so its image is the executable itself.
    let executable = std::fs::canonicalize(std::env::current_exe().expect("current executable")).unwrap();
    match super::library::xberg_image() {
        Some(image) => assert_eq!(image, executable),
        // glibc reports the main program by `argv[0]`, which is unusable when relative.
        None => assert!(
            cfg!(target_os = "linux")
                && std::env::args_os()
                    .next()
                    .is_some_and(|arg| Path::new(&arg).is_relative()),
            "dladdr named no image for an absolute argv[0]"
        ),
    }
}
