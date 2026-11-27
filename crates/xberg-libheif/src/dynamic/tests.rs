use super::*;

const CHILD: &str = "XBERG_LIBHEIF_DYNAMIC_TEST_CHILD";

/// A fresh owner-only directory under the system temporary directory, unique to this process
/// and test.
fn scratch_directory(test: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let directory = std::env::temp_dir().join(format!("xberg-libheif-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("create a scratch directory");
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).expect("restrict it");
    directory
}

fn broken_library(directory: &Path, name: &str) -> PathBuf {
    let broken = directory.join(name);
    std::fs::write(&broken, b"not a shared library").expect("write a broken library");
    broken
}

#[test]
fn explicit_path_is_the_only_candidate() {
    assert_eq!(
        explicit_library(Some(OsStr::new("/opt/custom/libheif.so.1"))),
        Some(Ok(PathBuf::from("/opt/custom/libheif.so.1")))
    );
}

#[test]
fn empty_or_absent_explicit_path_falls_back_to_the_search() {
    assert_eq!(explicit_library(Some(OsStr::new(""))), None);
    assert_eq!(explicit_library(None), None);
}

#[test]
fn relative_explicit_path_is_refused() {
    let Some(Err(message)) = explicit_library(Some(OsStr::new("lib/libheif.so.1"))) else {
        panic!("a relative override must be refused");
    };
    assert!(message.contains("relative path `lib/libheif.so.1`"), "{message}");
}

#[test]
fn release_layout_then_the_system() {
    let candidates = library_candidates(Some(Path::new("/opt/xberg/bin/xberg")), &["/usr/lib/libheif.so.1"]);
    let mut expected: Vec<PathBuf> = LIBRARY_NAMES
        .iter()
        .map(|name| Path::new("/opt/xberg/bin").join(name))
        .collect();
    expected.extend(LIBRARY_NAMES.iter().map(|name| Path::new("/opt/xberg/lib").join(name)));
    expected.push(PathBuf::from("/usr/lib/libheif.so.1"));
    assert_eq!(candidates, expected);
}

#[test]
fn lib_beside_a_directory_not_named_bin_is_not_searched() {
    // `/tmp/x/xberg` must not reach `/tmp/lib`, which any local user can create.
    let candidates = library_candidates(Some(Path::new("/tmp/x/xberg")), &[]);
    let expected: Vec<PathBuf> = LIBRARY_NAMES
        .iter()
        .map(|name| Path::new("/tmp/x").join(name))
        .collect();
    assert_eq!(candidates, expected);
}

#[test]
fn missing_libraries_report_every_path_searched() {
    let directory = scratch_directory("missing");
    let absent = directory.join("libheif.absent");
    let result = open_first(std::slice::from_ref(&absent));
    std::fs::remove_dir_all(&directory).expect("remove the scratch directory");
    let Err(message) = result else {
        panic!("an absent library must not load");
    };
    assert!(message.contains(&absent.display().to_string()), "{message}");
    assert!(message.contains(LIBHEIF_PATH_ENV_VAR), "{message}");
}

#[test]
fn the_first_load_failure_is_reported_when_nothing_loads() {
    let directory = scratch_directory("broken");
    let first = broken_library(&directory, LIBRARY_NAMES[0]);
    let second = broken_library(&directory, LIBRARY_NAMES[1]);
    // What gets opened, and named in the error, is the path with symlinks resolved.
    let opened = std::fs::canonicalize(&first).expect("resolve the broken library");
    let result = open_first(&[first, second]);
    std::fs::remove_dir_all(&directory).expect("remove the scratch directory");
    let Err(message) = result else {
        panic!("a broken library must not load");
    };
    assert!(message.starts_with("could not load libheif"), "{message}");
    assert!(message.contains(&opened.display().to_string()), "{message}");
}

#[test]
fn a_broken_candidate_is_skipped_for_a_working_one() {
    let installed = SYSTEM_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
        .expect("libheif is installed at a system path");
    let directory = scratch_directory("skipped");
    let broken = broken_library(&directory, LIBRARY_NAMES[0]);
    let result = open_first(&[broken, installed]);
    std::fs::remove_dir_all(&directory).expect("remove the scratch directory");
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn a_library_another_user_could_write_is_skipped() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    for mode in [0o775, 0o757, 0o777] {
        let directory = scratch_directory("untrusted");
        let planted = broken_library(&directory, LIBRARY_NAMES[0]);
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(mode)).unwrap();
        // Group write through the user's own private group (umask 002 hosts) is not shared.
        // SAFETY: geteuid takes no arguments, cannot fail and has no side effects.
        let euid = unsafe { libc::geteuid() };
        let shared = mode & 0o002 != 0 || !is_private_group(std::fs::metadata(&directory).unwrap().gid(), euid);
        let result = open_first(std::slice::from_ref(&planted));
        std::fs::remove_dir_all(&directory).expect("remove the scratch directory");
        let Err(message) = result else {
            panic!("a broken library must not load");
        };
        if shared {
            assert!(message.starts_with("libheif not found"), "{mode:o}: {message}");
            assert!(message.contains("writable by group or others"), "{mode:o}: {message}");
        } else {
            assert!(message.starts_with("could not load libheif"), "{mode:o}: {message}");
        }
    }
}

#[test]
fn only_a_user_private_group_may_have_write_access() {
    use std::ffi::CStr;

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

#[test]
fn the_installed_libheif_provides_every_wrapped_function() {
    ensure_loaded().expect("libheif should load and export every function xberg-libheif calls");
}

/// Every root wrapper, the only libheif functions reachable without a libheif object, returns the
/// load error without calling into libheif, so the null arguments are never read.
fn root_functions_return_the_load_error() {
    macro_rules! unloaded {
        ($call:expr) => {
            // SAFETY: An unloaded root wrapper returns before using its arguments.
            match unsafe { $call } {
                Err(error) => assert!(
                    matches!(error.code, crate::HeifErrorCode::PluginLoadingError)
                        && error.message.contains(LIBHEIF_PATH_ENV_VAR),
                    "{}: {error:?}",
                    stringify!($call)
                ),
                Ok(_) => panic!("{} succeeded without libheif", stringify!($call)),
            }
        };
    }
    unloaded!(heif_check_filetype(std::ptr::null(), 0));
    unloaded!(heif_context_alloc());
    unloaded!(heif_context_get_encoder_for_format(
        std::ptr::null_mut(),
        0,
        std::ptr::null_mut()
    ));
    unloaded!(heif_decoding_options_alloc());
    unloaded!(heif_deinit());
    unloaded!(heif_encoding_options_alloc());
    unloaded!(heif_get_decoder_descriptors(0, std::ptr::null_mut(), 0));
    unloaded!(heif_get_encoder_descriptors(
        0,
        std::ptr::null(),
        std::ptr::null_mut(),
        0
    ));
    unloaded!(heif_get_version_number());
    unloaded!(heif_image_create(1, 1, 0, 0, std::ptr::null_mut()));
    unloaded!(heif_init(std::ptr::null_mut()));
    unloaded!(heif_load_plugins(
        std::ptr::null(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        0
    ));
    unloaded!(heif_nclx_color_profile_alloc());
    #[cfg(feature = "v1_19")]
    unloaded!(heif_color_conversion_options_set_defaults(std::ptr::null_mut()));
    #[cfg(feature = "v1_19")]
    unloaded!(heif_get_global_security_limits());
    #[cfg(feature = "v1_20")]
    unloaded!(heif_color_conversion_options_ext_alloc());
}

#[test]
fn entry_points_report_an_unloadable_library_instead_of_panicking() {
    let test = "entry_points_report_an_unloadable_library_instead_of_panicking";
    if std::env::var(CHILD).as_deref() == Ok(test) {
        assert!(
            ensure_loaded().is_err(),
            "the parent names a library that does not exist"
        );
        root_functions_return_the_load_error();
        assert!(crate::ensure_available().is_err());
        {
            let lib = crate::LibHeif::new();
            assert_eq!(lib.version(), [0; 3]);
            assert!(lib.decoder_descriptors(8, None).is_empty());
            assert!(lib.encoder_descriptors(8, None, None).is_empty());
            assert!(lib.encoder_for_format(crate::CompressionFormat::Hevc).is_err());
            assert!(lib.load_plugins("/nonexistent").is_err());
        }
        assert!(crate::LibHeif::new_checked().is_err());
        let error = crate::HeifContext::new().err().expect("no context without libheif");
        assert!(error.message.contains(LIBHEIF_PATH_ENV_VAR), "{}", error.message);
        assert!(crate::HeifContext::read_from_bytes(b"not a heif").is_err());
        assert_eq!(
            crate::check_file_type(b"\0\0\0\x18ftypheic\0\0\0\0"),
            crate::FileTypeResult::No
        );
        assert!(crate::Image::new(1, 1, crate::ColorSpace::Rgb(crate::RgbChroma::Rgb)).is_err());
        assert!(crate::EncodingOptions::new().is_err());
        assert!(crate::DecodingOptions::new().is_none());
        assert!(crate::ColorProfileNCLX::new().is_none());
        let _ = crate::ColorConversionOptions::default();
        return;
    }

    let directory = scratch_directory("unloadable");
    let output = std::process::Command::new(std::env::current_exe().expect("locate the test executable"))
        .args(["--exact", &format!("dynamic::tests::{test}"), "--nocapture"])
        .env(LIBHEIF_PATH_ENV_VAR, directory.join("libheif-missing"))
        .env(CHILD, test)
        .output()
        .expect("run the isolated entry-point test");
    std::fs::remove_dir_all(&directory).expect("remove the scratch directory");
    assert!(
        output.status.success() && String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
        "isolated entry-point test failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
