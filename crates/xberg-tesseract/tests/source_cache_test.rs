#[path = "../build_support/source_cache.rs"]
mod source_cache;

use source_cache::{
    SourceArtifact, copy_verified_artifact, ensure_private_cache_root, prepare_source_tree, prepare_verified_artifact,
    read_exact_size, read_regular_file_exact_with_swap,
};
use std::cell::Cell;
use std::fs;
use std::io;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::time::{SystemTime, UNIX_EPOCH};

const ARCHIVE_BYTES: &[u8] = b"verified archive bytes";
const ARCHIVE_SHA256: &str = "74e79ddd5e5b690dbfb0995c1af7c4a86feaeff0b5f5b8d667fc2cda3360a489";
const ALTERED_ARCHIVE_BYTES: &[u8] = b"altered archive bytes";
const MODEL_BYTES: &[u8] = b"verified model bytes";
const MODEL_SHA256: &str = "03cfa25d83f5eaa1faac98ed6ceaaf0e7afe3c273a1e1502c2714ebe10b8263e";
const ALTERED_MODEL_BYTES: &[u8] = b"altered model bytes";
const BUILD_SCRIPT: &str = include_str!("../build.rs");
const LINK_SEARCH_FORMAT: &str = "\"cargo:rustc-link-search=native={}\"";
const WINDOWS_MAX_PATH: usize = 260;
/// `OUT_DIR` as GitHub Actions lays it out for the Windows MSVC release job, with the sixteen
/// hexadecimal digits Cargo appends to the build directory name stood in for.
const CI_BUILD_OUT_DIR: &str =
    r"D:\a\xberg\xberg\target\x86_64-pc-windows-msvc\release\build\xberg-tesseract-0123456789abcdef\out";
#[cfg(unix)]
const OWNER_ONLY_DIRECTORY_MODE: u32 = 0o700;
#[cfg(unix)]
const ALL_DIRECTORY_PERMISSION_BITS: u32 = 0o777;

const PINNED_BUILD_INPUTS: &[&str] = &[
    "13275a278eb55b5746e33f95fbf5a2c8f604b3ab",
    "0febcd4fc5cdc9c52d59509b45483d107f9f40922899e3f134ea615094ecbc77",
    "4292f6890aa86ef64eba32090358f40f69dd6d28",
    "d590ac276138238eb80e4d5833f65cb057cb90d02088d78a05027190a5318e41",
    "87416418657359cb625c412a48b6e1d6d41c29bd",
    "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2",
];

#[test]
fn should_store_verified_download_in_digest_addressed_cache() {
    let temp_dir = TestDir::new();
    let artifact = source_artifact();

    let prepared = prepare_verified_artifact(temp_dir.path(), &artifact, || Ok(ARCHIVE_BYTES.to_vec()))
        .expect("prepare verified archive");

    assert_eq!(
        prepared.path,
        temp_dir
            .path()
            .join("native-sources")
            .join(ARCHIVE_SHA256)
            .join("tesseract.zip")
    );
    assert_eq!(prepared.bytes, ARCHIVE_BYTES);
    assert_eq!(fs::read(&prepared.path).expect("read cached archive"), ARCHIVE_BYTES);
    assert!(prepared.downloaded, "new archive should be reported as downloaded");
}

#[test]
fn should_wire_every_pinned_build_input_through_verification() {
    validate_build_source_contract(BUILD_SCRIPT).expect("build source contract must hold");
}

#[test]
fn should_normalize_every_cmake_source_path() {
    assert_eq!(BUILD_SCRIPT.matches("cmake_source_path(").count(), 4);
    assert!(!BUILD_SCRIPT.contains("Config::new(leptonica_src)"));
}

#[test]
fn should_strip_verbatim_prefix_from_every_cmake_definition() {
    assert!(
        BUILD_SCRIPT.contains("windows_cmake_argument_path(&path.to_string_lossy())"),
        "CMake -D values must strip the Windows verbatim prefix rather than only swapping separators, \
         or `install(EXPORT ...)` package files resolve no IMPORTED_LOCATION for any configuration"
    );
}

#[cfg(windows)]
#[test]
fn should_preserve_source_identity_when_normalizing_for_cmake() {
    let temp_dir = TestDir::new();
    let canonical = fs::canonicalize(temp_dir.path()).expect("canonical source directory");

    let normalized = source_cache::cmake_source_path(&canonical).expect("identity-preserving CMake source path");

    assert!(same_file::is_same_file(canonical, normalized).expect("compare source directory identity"));
}

#[test]
fn should_remove_windows_verbatim_drive_prefix_for_cmake() {
    let path = r"\\?\C:\cargo\target\third_party\tesseract";

    let normalized = String::from_utf16(&source_cache::windows_cmake_source_units(
        &path.encode_utf16().collect::<Vec<_>>(),
    ))
    .expect("normalized Windows path");

    assert_eq!(normalized, r"C:\cargo\target\third_party\tesseract");
}

#[test]
fn should_convert_windows_verbatim_unc_prefix_for_cmake() {
    let path = r"\\?\UNC\server\share\third_party\tesseract";

    let normalized = String::from_utf16(&source_cache::windows_cmake_source_units(
        &path.encode_utf16().collect::<Vec<_>>(),
    ))
    .expect("normalized Windows UNC path");

    assert_eq!(normalized, r"\\server\share\third_party\tesseract");
}

#[test]
fn should_strip_verbatim_prefix_from_cmake_argument_path() {
    let path = r"\\?\D:\a\xberg\xberg\target\debug\build\xberg-tesseract-abc\out\leptonica";

    let normalized = source_cache::windows_cmake_argument_path(path);

    assert_eq!(
        normalized,
        "D:/a/xberg/xberg/target/debug/build/xberg-tesseract-abc/out/leptonica"
    );
    assert!(
        !normalized.starts_with("//?/"),
        "CMake reads a //?/ prefix as a UNC root, which breaks install(EXPORT) package files: {normalized}"
    );
}

#[test]
fn should_convert_verbatim_unc_prefix_in_cmake_argument_path() {
    let path = r"\\?\UNC\server\share\out\leptonica";

    let normalized = source_cache::windows_cmake_argument_path(path);

    assert_eq!(normalized, "//server/share/out/leptonica");
}

#[test]
fn should_keep_plain_windows_path_in_cmake_argument_path() {
    let path = r"D:\a\xberg\out\leptonica\lib\cmake\leptonica";

    let normalized = source_cache::windows_cmake_argument_path(path);

    assert_eq!(normalized, "D:/a/xberg/out/leptonica/lib/cmake/leptonica");
}

#[test]
fn should_strip_verbatim_prefix_from_link_search_directory_keeping_native_separators() {
    let leptonica_lib_dir = format!(r"{CI_BUILD_OUT_DIR}\leptonica\lib");
    let verbatim = format!(r"\\?\{leptonica_lib_dir}");

    let normalized = String::from_utf16(&source_cache::windows_cmake_source_units(
        &verbatim.encode_utf16().collect::<Vec<_>>(),
    ))
    .expect("normalized Windows link search directory");

    assert_eq!(normalized, leptonica_lib_dir);
    assert!(
        !normalized.contains('/'),
        "a linker search directory keeps native separators, unlike a CMake -D value: {normalized}"
    );
}

#[test]
fn should_keep_stripped_link_search_directory_within_windows_max_path() {
    let leptonica_lib_dir = format!(r"{CI_BUILD_OUT_DIR}\leptonica\lib");
    let tesseract_include_dir = format!(r"{CI_BUILD_OUT_DIR}\tesseract\include");
    let deepest_header = format!(r"{tesseract_include_dir}\tesseract\capi.h");

    assert_eq!(leptonica_lib_dir.len(), 111);
    assert_eq!(tesseract_include_dir.len(), 115);
    assert_eq!(deepest_header.len(), 132);
    assert!(
        deepest_header.len() < WINDOWS_MAX_PATH,
        "dropping the verbatim prefix must not reintroduce a MAX_PATH failure: {} >= {WINDOWS_MAX_PATH}",
        deepest_header.len()
    );
}

#[test]
fn should_leave_paths_without_a_verbatim_prefix_unchanged_for_tool_arguments() {
    let temp_dir = TestDir::new();
    let canonical = fs::canonicalize(temp_dir.path()).expect("canonical directory");

    assert_eq!(source_cache::tool_argument_path(&canonical), canonical);
}

#[test]
fn should_route_every_canonicalized_link_search_through_tool_argument_path() {
    let unwrapped = link_search_arguments(BUILD_SCRIPT)
        .filter(|argument| !argument.starts_with("tool_argument_path("))
        .collect::<Vec<_>>();

    assert_eq!(
        unwrapped,
        vec![
            "parent.display()",
            "env::var(\"OUT_DIR\").unwrap()",
            "sysroot_lib.display()",
            "sysroot_lib_noeh.display()",
            "rt_dir.display()",
        ],
        "only search directories that never pass through fs::canonicalize may skip the verbatim-prefix strip"
    );
}

#[test]
fn should_report_a_link_search_argument_that_skips_tool_argument_path() {
    let snippet = r#"
        println!("cargo:rustc-link-search=native={}", leptonica_lib_dir.display());
        println!(
            "cargo:rustc-link-search=native={}",
            tool_argument_path(&tesseract_lib_dir).display()
        );
    "#;

    assert_eq!(
        link_search_arguments(snippet).collect::<Vec<_>>(),
        vec![
            "leptonica_lib_dir.display()",
            "tool_argument_path(&tesseract_lib_dir).display()",
        ],
        "the contract check must read both println! shapes the build script uses"
    );
}

#[test]
fn should_strip_verbatim_prefix_from_the_compiler_include_directory() {
    assert!(
        BUILD_SCRIPT.contains(".include(tool_argument_path(&tesseract_install_dir.join(\"include\")))"),
        "a verbatim /I directory cannot resolve #include <tesseract/capi.h>: the forward slash is an \
         ordinary filename character inside a \\\\?\\ path"
    );
}

#[test]
fn should_keep_automatic_cache_inside_cargo_build_tree() {
    assert!(!BUILD_SCRIPT.contains("C:\\tess"));
    assert!(!BUILD_SCRIPT.contains("env::temp_dir().join(\"xberg-tesseract-cache\")"));
    assert!(BUILD_SCRIPT.contains("ensure_private_cache_root(&preferred"));
    assert!(BUILD_SCRIPT.contains("ensure_private_cache_root(&fallback"));
}

#[test]
fn should_reject_build_contract_when_digest_is_removed() {
    let mutated = BUILD_SCRIPT.replace(PINNED_BUILD_INPUTS[1], "");

    let error = validate_build_source_contract(&mutated).expect_err("missing digest must break build contract");

    assert_eq!(error, "missing pinned build input");
}

#[test]
fn should_reject_build_contract_when_model_uses_mutable_branch() {
    let mutated = BUILD_SCRIPT.replace(PINNED_BUILD_INPUTS[4], "main");

    let error = validate_build_source_contract(&mutated).expect_err("mutable model branch must break build contract");

    assert_eq!(error, "missing pinned build input");
}

#[test]
fn should_reuse_verified_content_addressed_cache_entry_without_fetching() {
    let temp_dir = TestDir::new();
    let artifact = source_artifact();
    prepare_verified_artifact(temp_dir.path(), &artifact, || Ok(ARCHIVE_BYTES.to_vec()))
        .expect("seed verified archive");
    let fetch_called = Cell::new(false);

    let prepared = prepare_verified_artifact(temp_dir.path(), &artifact, || {
        fetch_called.set(true);
        Ok(Vec::new())
    })
    .expect("reuse verified archive");

    assert!(!fetch_called.get(), "verified cache reuse must not fetch");
    assert!(
        !prepared.downloaded,
        "reused archive must not be reported as downloaded"
    );
    assert_eq!(prepared.bytes, ARCHIVE_BYTES);
}

#[test]
fn should_reject_download_when_archive_bytes_do_not_match_digest() {
    let temp_dir = TestDir::new();
    let artifact = source_artifact();

    let error = prepare_verified_artifact(temp_dir.path(), &artifact, || Ok(ALTERED_ARCHIVE_BYTES.to_vec()))
        .expect_err("altered archive must be rejected");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(!expected_cache_path(temp_dir.path()).exists());
}

#[test]
fn should_accept_download_at_exact_expected_size() {
    let mut reader = Cursor::new(ARCHIVE_BYTES);

    let bytes = read_exact_size(&mut reader, ARCHIVE_BYTES.len() as u64, "archive")
        .expect("exact download size should be accepted");

    assert_eq!(bytes, ARCHIVE_BYTES);
}

#[test]
fn should_reject_download_below_expected_size() {
    let mut reader = Cursor::new(&ARCHIVE_BYTES[..ARCHIVE_BYTES.len() - 1]);

    let error = read_exact_size(&mut reader, ARCHIVE_BYTES.len() as u64, "archive")
        .expect_err("short download must be rejected");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn should_reject_download_above_expected_size_without_publishing() {
    let temp_dir = TestDir::new();
    let mut oversized = ARCHIVE_BYTES.to_vec();
    oversized.push(b'!');

    let error = prepare_verified_artifact(temp_dir.path(), &source_artifact(), || Ok(oversized))
        .expect_err("oversized download must be rejected");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(!expected_cache_path(temp_dir.path()).exists());
}

#[test]
fn should_reject_artifact_when_digest_is_missing_before_fetching() {
    let temp_dir = TestDir::new();
    let artifact = SourceArtifact {
        name: "tesseract.zip",
        cache_key: "native-sources",
        sha256: "",
        expected_size: ARCHIVE_BYTES.len() as u64,
    };
    let fetch_called = Cell::new(false);

    let error = prepare_verified_artifact(temp_dir.path(), &artifact, || {
        fetch_called.set(true);
        Ok(Vec::new())
    })
    .expect_err("missing digest must be rejected");

    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(!fetch_called.get(), "missing digest must fail before fetch");
}

#[test]
fn should_reject_corrupt_cache_entry_without_fetching() {
    let temp_dir = TestDir::new();
    let artifact = source_artifact();
    let prepared = prepare_verified_artifact(temp_dir.path(), &artifact, || Ok(ARCHIVE_BYTES.to_vec()))
        .expect("seed verified archive");
    fs::write(&prepared.path, ALTERED_ARCHIVE_BYTES).expect("corrupt cached archive");
    let fetch_called = Cell::new(false);

    let error = prepare_verified_artifact(temp_dir.path(), &artifact, || {
        fetch_called.set(true);
        Ok(Vec::new())
    })
    .expect_err("corrupt cache entry must fail closed");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(!fetch_called.get(), "corrupt cache must not trigger replacement fetch");
}

#[test]
fn should_reject_oversized_cache_entry_without_fetching() {
    let temp_dir = TestDir::new();
    let cache_path = expected_cache_path(temp_dir.path());
    fs::create_dir_all(cache_path.parent().expect("cache parent")).expect("create cache parent");
    let mut oversized = ARCHIVE_BYTES.to_vec();
    oversized.push(b'!');
    fs::write(&cache_path, oversized).expect("write oversized cache entry");
    let fetch_called = Cell::new(false);

    let error = prepare_verified_artifact(temp_dir.path(), &source_artifact(), || {
        fetch_called.set(true);
        Ok(ARCHIVE_BYTES.to_vec())
    })
    .expect_err("oversized cache entry must fail closed");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(!fetch_called.get(), "oversized cache must fail before fetch");
}

#[test]
fn should_publish_concurrent_verified_download_once() {
    let temp_dir = TestDir::new();
    let cache_root = temp_dir.path().to_path_buf();
    let barrier = Arc::new(Barrier::new(2));
    let mut threads = Vec::new();

    for _ in 0..2 {
        let cache_root = cache_root.clone();
        let barrier = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            prepare_verified_artifact(&cache_root, &source_artifact(), || {
                barrier.wait();
                Ok(ARCHIVE_BYTES.to_vec())
            })
            .expect("publish concurrent verified archive")
        }));
    }

    let mut downloaded = threads
        .into_iter()
        .map(|thread| thread.join().expect("join publisher").downloaded)
        .collect::<Vec<_>>();
    downloaded.sort_unstable();

    assert_eq!(downloaded, [false, true]);
    assert_eq!(
        fs::read(expected_cache_path(temp_dir.path())).expect("read cache"),
        ARCHIVE_BYTES
    );
    assert_eq!(
        fs::read_dir(expected_cache_path(temp_dir.path()).parent().expect("cache parent"))
            .expect("read cache parent")
            .count(),
        1
    );
}

#[cfg(unix)]
#[test]
fn should_not_follow_download_temporary_symlink_swapped_during_fetch() {
    use std::os::unix::fs::symlink;

    let temp_dir = TestDir::new();
    let victim = temp_dir.path().join("victim");
    fs::write(&victim, b"preserve").expect("write victim sentinel");
    let digest_dir = temp_dir.path().join("native-sources").join(ARCHIVE_SHA256);
    fs::create_dir_all(&digest_dir).expect("create digest directory");
    let old_temporary_path = digest_dir.join(format!(".tesseract.zip.{}.partial", std::process::id()));
    symlink(&victim, &old_temporary_path).expect("create predictable temporary symlink");

    prepare_verified_artifact(temp_dir.path(), &source_artifact(), || Ok(ARCHIVE_BYTES.to_vec()))
        .expect("prepare through an unpredictable temporary file");

    assert_eq!(fs::read(&victim).expect("read victim sentinel"), b"preserve");
    assert_eq!(
        fs::read_link(&old_temporary_path).expect("read old temporary symlink"),
        victim
    );
}

#[cfg(unix)]
#[test]
fn should_reject_symlinked_model_destination_without_reading_target() {
    use std::os::unix::fs::symlink;

    let temp_dir = TestDir::new();
    let model = SourceArtifact {
        name: "eng.traineddata",
        cache_key: "tessdata",
        sha256: MODEL_SHA256,
        expected_size: MODEL_BYTES.len() as u64,
    };
    let verified = prepare_verified_artifact(temp_dir.path(), &model, || Ok(MODEL_BYTES.to_vec()))
        .expect("prepare verified model");
    let destination = temp_dir.path().join("out").join("eng.traineddata");
    fs::create_dir_all(destination.parent().expect("model parent")).expect("create model output directory");
    let victim = temp_dir.path().join("model-victim");
    fs::write(&victim, MODEL_BYTES).expect("write model victim");
    symlink(&victim, &destination).expect("create model destination symlink");

    let error = copy_verified_artifact(&verified, &destination).expect_err("model symlinks must fail closed");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(fs::read(&victim).expect("read model victim"), MODEL_BYTES);
}

#[cfg(unix)]
#[test]
fn should_reject_file_swapped_to_symlink_between_check_and_open() {
    use std::os::unix::fs::symlink;

    let temp_dir = TestDir::new();
    let artifact_path = temp_dir.path().join("artifact");
    fs::write(&artifact_path, ARCHIVE_BYTES).expect("write initial artifact");
    let victim = temp_dir.path().join("victim");
    fs::write(&victim, ARCHIVE_BYTES).expect("write same-sized victim");

    let error = read_regular_file_exact_with_swap(&artifact_path, ARCHIVE_BYTES.len() as u64, || {
        fs::remove_file(&artifact_path)?;
        symlink(&victim, &artifact_path)
    })
    .expect_err("nofollow open must reject a swapped symlink");

    assert_ne!(error.kind(), io::ErrorKind::NotFound);
    assert_eq!(fs::read(&victim).expect("read victim"), ARCHIVE_BYTES);
}

#[cfg(unix)]
#[test]
fn should_restrict_automatic_cache_root_to_owner() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let temp_dir = TestDir::new();
    let cache_root = temp_dir.path().join("automatic-cache");
    fs::create_dir(&cache_root).expect("create permissive cache root");
    fs::set_permissions(&cache_root, fs::Permissions::from_mode(ALL_DIRECTORY_PERMISSION_BITS))
        .expect("make cache root permissive");

    ensure_private_cache_root(&cache_root, temp_dir.path()).expect("secure automatic cache root");

    let metadata = fs::symlink_metadata(&cache_root).expect("read secured cache root");
    assert_eq!(
        metadata.mode() & ALL_DIRECTORY_PERMISSION_BITS,
        OWNER_ONLY_DIRECTORY_MODE
    );
    assert_eq!(
        metadata.uid(),
        fs::metadata(temp_dir.path()).expect("read owner reference").uid()
    );
}

#[cfg(unix)]
#[test]
fn should_canonicalize_user_owned_symlinks_above_trusted_build_root() {
    use std::os::unix::fs::symlink;

    let temp_dir = TestDir::new();
    let real_workspace = temp_dir.path().join("real-workspace");
    let real_out_dir = real_workspace.join("target/debug/build/xberg-tesseract/out");
    fs::create_dir_all(&real_out_dir).expect("create real Cargo build root");
    let linked_workspace = temp_dir.path().join("linked-workspace");
    symlink(&real_workspace, &linked_workspace).expect("create user-owned workspace symlink");
    let linked_out_dir = linked_workspace.join("target/debug/build/xberg-tesseract/out");

    let trusted_root =
        source_cache::canonicalize_trusted_build_root(&linked_out_dir).expect("canonicalize trusted Cargo build root");

    assert_eq!(
        trusted_root,
        fs::canonicalize(&real_out_dir).expect("canonicalize expected root")
    );
    let private_cache = trusted_root.join("private-cache");
    ensure_private_cache_root(&private_cache, &trusted_root).expect("create cache below trusted root");

    let outside = temp_dir.path().join("outside-cache");
    fs::create_dir(&outside).expect("create outside cache directory");
    let redirected_cache = trusted_root.join("redirected-cache");
    symlink(&outside, &redirected_cache).expect("create cache symlink below trusted root");
    let error = ensure_private_cache_root(&redirected_cache, &trusted_root)
        .expect_err("symlink below trusted root must fail closed");
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
}

#[cfg(unix)]
#[test]
fn should_reject_symlinked_cache_component() {
    use std::os::unix::fs::symlink;

    let temp_dir = TestDir::new();
    let outside_dir = temp_dir.path().join("outside");
    fs::create_dir(&outside_dir).expect("create outside directory");
    symlink(&outside_dir, temp_dir.path().join("native-sources")).expect("create cache symlink");
    let fetch_called = Cell::new(false);

    let error = prepare_verified_artifact(temp_dir.path(), &source_artifact(), || {
        fetch_called.set(true);
        Ok(Vec::new())
    })
    .expect_err("symlinked cache component must fail closed");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(!fetch_called.get(), "symlink rejection must precede fetch");
    assert_eq!(fs::read_dir(&outside_dir).expect("read outside directory").count(), 0);
}

#[cfg(unix)]
#[test]
fn should_reject_cache_path_beneath_symlinked_root() {
    use std::os::unix::fs::symlink;

    let temp_dir = TestDir::new();
    let outside_dir = temp_dir.path().join("outside-root");
    fs::create_dir(&outside_dir).expect("create outside directory");
    let outside_artifacts = outside_dir.join("source-artifacts");
    fs::create_dir(&outside_artifacts).expect("create pre-existing outside artifact directory");
    let cache_root = temp_dir.path().join("cache-root");
    symlink(&outside_dir, &cache_root).expect("create cache-root symlink");
    let fetch_called = Cell::new(false);

    let error = prepare_verified_artifact(&cache_root.join("source-artifacts"), &source_artifact(), || {
        fetch_called.set(true);
        Ok(Vec::new())
    })
    .expect_err("cache path beneath a symlinked root must fail closed");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(!fetch_called.get(), "symlink rejection must precede fetch");
    assert_eq!(
        fs::read_dir(&outside_artifacts)
            .expect("read outside directory")
            .count(),
        0
    );
}

#[cfg(unix)]
#[test]
fn should_reject_symlinked_source_root_without_removing_target() {
    use std::os::unix::fs::symlink;

    let temp_dir = TestDir::new();
    let archive = prepare_verified_artifact(temp_dir.path(), &source_artifact(), || Ok(ARCHIVE_BYTES.to_vec()))
        .expect("prepare verified archive");
    let outside_dir = temp_dir.path().join("outside-source");
    fs::create_dir(&outside_dir).expect("create outside source directory");
    fs::write(outside_dir.join("sentinel"), "preserve").expect("write outside sentinel");
    let third_party_dir = temp_dir.path().join("third-party");
    symlink(&outside_dir, &third_party_dir).expect("create source-root symlink");

    let error = prepare_source_tree(&third_party_dir, "tesseract", &archive, |_, _| Ok(()))
        .expect_err("symlinked source root must fail closed");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(
        outside_dir.join("sentinel").is_file(),
        "outside target must remain intact"
    );
}

#[test]
fn should_reject_staging_directory_recreated_after_extraction() {
    let temp_dir = TestDir::new();
    let archive = prepare_verified_artifact(temp_dir.path(), &source_artifact(), || Ok(ARCHIVE_BYTES.to_vec()))
        .expect("prepare verified archive");
    let third_party_dir = temp_dir.path().join("third-party");

    let error = prepare_source_tree(&third_party_dir, "tesseract", &archive, |_, staging_dir| {
        let original_staging_dir = staging_dir.with_extension("replaced");
        fs::rename(staging_dir, &original_staging_dir)?;
        fs::create_dir(staging_dir)?;
        fs::write(staging_dir.join("CMakeLists.txt"), "replacement build")?;
        Ok(())
    })
    .expect_err("recreated staging directory must fail closed");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(!third_party_dir.join("tesseract").exists());
}

#[cfg(unix)]
#[test]
fn should_reject_staging_directory_replaced_after_extraction() {
    use std::os::unix::fs::symlink;

    let temp_dir = TestDir::new();
    let archive = prepare_verified_artifact(temp_dir.path(), &source_artifact(), || Ok(ARCHIVE_BYTES.to_vec()))
        .expect("prepare verified archive");
    let outside_dir = temp_dir.path().join("outside-staging");
    fs::create_dir(&outside_dir).expect("create outside staging directory");
    fs::write(outside_dir.join("sentinel"), "preserve").expect("write outside sentinel");
    fs::write(outside_dir.join("CMakeLists.txt"), "redirected build").expect("write redirected build");
    let third_party_dir = temp_dir.path().join("third-party");

    let error = prepare_source_tree(&third_party_dir, "tesseract", &archive, |_, staging_dir| {
        fs::write(staging_dir.join("CMakeLists.txt"), "verified build")?;
        fs::remove_file(staging_dir.join("CMakeLists.txt"))?;
        fs::remove_dir(staging_dir)?;
        symlink(&outside_dir, staging_dir)?;
        Ok(())
    })
    .expect_err("staging symlink swap must fail closed");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(!third_party_dir.join("tesseract").exists());
    assert_eq!(
        fs::read_to_string(outside_dir.join("CMakeLists.txt")).expect("read redirected build"),
        "redirected build"
    );
    assert_eq!(
        fs::read_to_string(outside_dir.join("sentinel")).expect("read outside sentinel"),
        "preserve"
    );
}

#[test]
fn should_replace_poisoned_complete_source_tree_before_extraction() {
    let temp_dir = TestDir::new();
    let artifact = prepare_verified_artifact(temp_dir.path(), &source_artifact(), || Ok(ARCHIVE_BYTES.to_vec()))
        .expect("prepare verified archive");
    let third_party_dir = temp_dir.path().join("third-party");
    let source_dir = third_party_dir.join("tesseract");
    fs::create_dir_all(&source_dir).expect("create poisoned source tree");
    fs::write(source_dir.join("CMakeLists.txt"), "poisoned build").expect("write poisoned marker");
    fs::write(source_dir.join("poisoned.cpp"), "malicious source").expect("write poisoned source");
    let extraction_called = Cell::new(false);

    let prepared = prepare_source_tree(&third_party_dir, "tesseract", &artifact, |bytes, destination| {
        extraction_called.set(true);
        assert_eq!(bytes, ARCHIVE_BYTES);
        let metadata = fs::symlink_metadata(destination)?;
        assert!(metadata.file_type().is_dir(), "staging must be a directory");
        assert!(!metadata.file_type().is_symlink(), "staging must not be a symlink");
        assert!(
            source_dir.join("poisoned.cpp").is_file(),
            "old tree remains until replacement is ready"
        );
        fs::write(destination.join("CMakeLists.txt"), "verified build")?;
        Ok(())
    })
    .expect("reconstruct source tree from verified archive");

    assert!(
        extraction_called.get(),
        "complete-looking source tree must not be reused"
    );
    assert_eq!(
        fs::read_to_string(prepared.path.join("CMakeLists.txt")).expect("read verified marker"),
        "verified build"
    );
    assert!(!prepared.path.join("poisoned.cpp").exists());
}

#[test]
fn should_fail_before_source_preparation_when_archive_is_altered() {
    let temp_dir = TestDir::new();
    let artifact = source_artifact();
    let extraction_called = Cell::new(false);

    let error = prepare_verified_artifact(temp_dir.path(), &artifact, || Ok(ALTERED_ARCHIVE_BYTES.to_vec()))
        .and_then(|verified| {
            prepare_source_tree(&temp_dir.path().join("third-party"), "tesseract", &verified, |_, _| {
                extraction_called.set(true);
                Ok(())
            })
        })
        .expect_err("altered archive must fail verification");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(
        !extraction_called.get(),
        "verification failure must precede source preparation"
    );
}

#[test]
fn should_reject_altered_existing_model_file() {
    let temp_dir = TestDir::new();
    let model = SourceArtifact {
        name: "eng.traineddata",
        cache_key: "tessdata",
        sha256: MODEL_SHA256,
        expected_size: MODEL_BYTES.len() as u64,
    };
    let verified = prepare_verified_artifact(temp_dir.path(), &model, || Ok(MODEL_BYTES.to_vec()))
        .expect("prepare verified model");
    let destination = temp_dir.path().join("out").join("eng.traineddata");
    fs::create_dir_all(destination.parent().expect("model parent")).expect("create model output directory");
    fs::write(&destination, ALTERED_MODEL_BYTES).expect("write altered installed model");

    let error = copy_verified_artifact(&verified, &destination).expect_err("altered model must fail closed");

    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(fs::read(&destination).expect("read altered model"), ALTERED_MODEL_BYTES);
}

fn source_artifact() -> SourceArtifact<'static> {
    SourceArtifact {
        name: "tesseract.zip",
        cache_key: "native-sources",
        sha256: ARCHIVE_SHA256,
        expected_size: ARCHIVE_BYTES.len() as u64,
    }
}

fn expected_cache_path(root: &Path) -> PathBuf {
    root.join("native-sources").join(ARCHIVE_SHA256).join("tesseract.zip")
}

/// Yield the argument expression of every `cargo:rustc-link-search=native=` directive the build
/// script emits, so a new search directory cannot quietly skip the verbatim-prefix strip.
fn link_search_arguments(build_script: &str) -> impl Iterator<Item = &str> {
    build_script.match_indices(LINK_SEARCH_FORMAT).map(|(index, marker)| {
        let tail = build_script[index + marker.len()..]
            .trim_start()
            .strip_prefix(',')
            .expect("a link search directive formats exactly one argument")
            .trim_start();
        let mut depth = 0usize;
        let end = tail
            .char_indices()
            .find(|(_, character)| match character {
                '(' => {
                    depth += 1;
                    false
                }
                ')' if depth == 0 => true,
                ')' => {
                    depth -= 1;
                    false
                }
                _ => false,
            })
            .map(|(offset, _)| offset)
            .expect("a link search argument closes the println! that formats it");
        tail[..end].trim_end()
    })
}

fn validate_build_source_contract(build_script: &str) -> Result<(), &'static str> {
    if PINNED_BUILD_INPUTS.iter().any(|input| !build_script.contains(input)) {
        return Err("missing pinned build input");
    }
    if build_script.contains("refs/tags") || build_script.contains("tessdata_fast/main") {
        return Err("mutable build input URL");
    }
    if build_script.matches("&LEPTONICA_SOURCE").count() != 2
        || build_script.matches("&TESSERACT_SOURCE").count() != 2
        || build_script
            .matches("prepare_eng_traineddata(&artifact_cache_dir")
            .count()
            != 2
        || !build_script.contains("prepare_verified_artifact(artifact_cache_dir, &source.artifact")
        || !build_script.contains("prepare_verified_artifact(artifact_cache_dir, &ENG_TRAINEDDATA_ARTIFACT")
    {
        return Err("build input bypasses verification");
    }
    Ok(())
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "xberg-source-cache-test-{}-{:?}-{unique}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::create_dir(&path).expect("create temporary test directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).expect("remove temporary test directory");
    }
}
