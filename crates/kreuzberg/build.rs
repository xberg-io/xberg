use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

/// PDFium linking strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PdfiumLinkStrategy {
    /// Download and link statically (static-pdfium feature)
    DownloadStatic,
    /// Download, link dynamically, and embed in binary (bundled-pdfium feature)
    Bundled,
    /// Use system-installed pdfium via pkg-config (system-pdfium feature)
    System,
}

fn main() {
    let target = env::var("TARGET").unwrap();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo::rustc-check-cfg=cfg(coverage)");

    if !cfg!(feature = "pdf") {
        tracing::debug!("PDF feature not enabled, skipping pdfium linking");
        return;
    }

    let strategy = determine_link_strategy(&target);

    tracing::debug!("Using PDFium linking strategy: {:?}", strategy);

    match strategy {
        PdfiumLinkStrategy::DownloadStatic => {
            let pdfium_dir = download_or_use_prebuilt(&target, &out_dir);
            link_statically(&pdfium_dir, &target);
        }
        PdfiumLinkStrategy::Bundled => {
            let pdfium_dir = download_or_use_prebuilt(&target, &out_dir);
            link_bundled(&pdfium_dir, &target, &out_dir);
        }
        PdfiumLinkStrategy::System => {
            link_system(&target);
        }
    }

    link_system_frameworks(&target);
    println!("cargo:rerun-if-changed=build.rs");
}

/// Determine which linking strategy to use based on features and target
fn determine_link_strategy(target: &str) -> PdfiumLinkStrategy {
    if target.contains("wasm") {
        if let Ok(wasm_lib) = env::var("PDFIUM_WASM_LIB") {
            println!("cargo:rustc-link-search=native={}", wasm_lib);
            println!("cargo:rustc-link-lib=static=pdfium");
            return PdfiumLinkStrategy::DownloadStatic;
        }
        println!("cargo:warning=WASM build using bundled PDFium (set PDFIUM_WASM_LIB to link custom WASM PDFium)");
        return PdfiumLinkStrategy::Bundled;
    }

    let system_pdfium = cfg!(feature = "system-pdfium");
    let bundled_pdfium = cfg!(feature = "bundled-pdfium");
    let static_pdfium = cfg!(feature = "static-pdfium");

    let enabled_count = usize::from(system_pdfium) + usize::from(bundled_pdfium) + usize::from(static_pdfium);
    if enabled_count > 1 {
        println!(
            "cargo:warning=Multiple PDFium linking strategies enabled (static-pdfium={}, bundled-pdfium={}, system-pdfium={}); using bundled-pdfium for this build",
            static_pdfium, bundled_pdfium, system_pdfium
        );
    }

    if bundled_pdfium {
        return PdfiumLinkStrategy::Bundled;
    }
    if system_pdfium {
        return PdfiumLinkStrategy::System;
    }
    if static_pdfium {
        return PdfiumLinkStrategy::DownloadStatic;
    }

    PdfiumLinkStrategy::Bundled
}

/// Download PDFium or use prebuilt directory
///
/// This is the main orchestrator function that:
/// 1. Checks for `KREUZBERG_PDFIUM_PREBUILT` environment variable
/// 2. If set and valid, uses prebuilt pdfium directory
/// 3. If not set, downloads pdfium to out_dir (with caching)
/// 4. Returns PathBuf to pdfium directory
///
/// Reuses all existing helper functions:
/// - `get_pdfium_url_and_lib()` - determines download URL for target
/// - `download_and_extract_pdfium()` - downloads with retry logic
/// - `runtime_library_info()` - platform-specific library names
/// - `prepare_prebuilt_pdfium()` - handles prebuilt copy
fn download_or_use_prebuilt(target: &str, out_dir: &Path) -> PathBuf {
    let (download_url, _lib_name) = get_pdfium_url_and_lib(target);
    let pdfium_dir = out_dir.join("pdfium");

    if let Some(prebuilt) = env::var_os("KREUZBERG_PDFIUM_PREBUILT") {
        let prebuilt_path = PathBuf::from(prebuilt);
        if prebuilt_path.exists() {
            prepare_prebuilt_pdfium(&prebuilt_path, &pdfium_dir)
                .unwrap_or_else(|err| panic!("Failed to copy Pdfium from {}: {}", prebuilt_path.display(), err));
            if target.contains("windows") {
                ensure_windows_import_library(&pdfium_dir);
            }
            return pdfium_dir;
        } else {
            panic!(
                "Environment variable KREUZBERG_PDFIUM_PREBUILT points to '{}' but the directory does not exist",
                prebuilt_path.display()
            );
        }
    }

    let (runtime_lib_name, runtime_subdir) = runtime_library_info(target);
    let lib_found = find_pdfium_library(&pdfium_dir, &runtime_lib_name, runtime_subdir).is_ok();

    let import_lib_exists = if target.contains("windows") {
        let lib_dir = pdfium_dir.join("lib");
        lib_dir.join("pdfium.lib").exists() || lib_dir.join("pdfium.dll.lib").exists()
    } else {
        true
    };

    if !lib_found || !import_lib_exists {
        tracing::debug!("Pdfium library not found, downloading for target: {}", target);
        tracing::debug!("Download URL: {}", download_url);
        download_and_extract_pdfium(&download_url, &pdfium_dir);
    } else {
        tracing::debug!("Pdfium library already cached at {}", pdfium_dir.display());
    }

    if target.contains("windows") {
        ensure_windows_import_library(&pdfium_dir);
    }

    pdfium_dir
}

fn ensure_windows_import_library(pdfium_dir: &Path) {
    let lib_dir = pdfium_dir.join("lib");
    let dll_lib = lib_dir.join("pdfium.dll.lib");
    let expected_lib = lib_dir.join("pdfium.lib");

    if dll_lib.exists() && !expected_lib.exists() {
        tracing::debug!(
            "Ensuring Windows import library at {} (source: {})",
            expected_lib.display(),
            dll_lib.display()
        );
        fs::copy(&dll_lib, &expected_lib).unwrap_or_else(|err| {
            panic!(
                "Failed to copy Windows import library from {} to {}: {}",
                dll_lib.display(),
                expected_lib.display(),
                err
            )
        });
    }
}

/// Fetch the latest release version from a GitHub repository
///
/// Uses curl to query the GitHub API and extract the tag_name from the
/// latest release JSON response. Uses improved JSON parsing with fallback logic.
///
/// For WASM (paulocoutinhox/pdfium-lib), falls back to known stable versions.
/// For non-WASM (bblanchon/pdfium-binaries), uses a different fallback strategy.
fn get_latest_version(repo: &str) -> String {
    let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo);

    let output = Command::new("curl").args(["-s", &api_url]).output();

    if let Ok(output) = output
        && output.status.success()
    {
        let json = String::from_utf8_lossy(&output.stdout);

        if let Some(tag) = extract_tag_from_json(&json) {
            return tag;
        }
    }

    if repo.contains("paulocoutinhox") {
        eprintln!(
            "cargo:warning=Failed to fetch latest PDFium WASM version from GitHub API, using fallback version 7442b"
        );
        "7442b".to_string()
    } else if repo.contains("bblanchon") {
        eprintln!(
            "cargo:warning=Failed to fetch latest PDFium binaries version from GitHub API, using fallback version 7568"
        );
        "7568".to_string()
    } else {
        eprintln!(
            "cargo:warning=Failed to fetch latest PDFium version from GitHub API (unknown repository: {})",
            repo
        );
        String::new()
    }
}

/// Extract tag_name from GitHub API JSON response
///
/// Parses JSON by finding the tag_name field and extracting the value between quotes.
/// Handles various JSON formatting variations.
fn extract_tag_from_json(json: &str) -> Option<String> {
    if let Some(start) = json.find("\"tag_name\"") {
        let after_colon = &json[start + "\"tag_name\"".len()..];

        let after_colon = after_colon.trim_start();
        let after_colon = after_colon.strip_prefix(':')?;
        let after_colon = after_colon.trim_start();

        if let Some(opening_quote) = after_colon.find('"') {
            let value_start = opening_quote + 1;
            if let Some(closing_quote) = after_colon[value_start..].find('"') {
                let tag = &after_colon[value_start..value_start + closing_quote];
                return Some(tag.split('/').next_back().unwrap_or(tag).to_string());
            }
        }
    }

    None
}

/// Get the download URL and library name for the target platform
///
/// Determines platform/architecture from target triple and constructs
/// the appropriate GitHub release download URL. Supports:
/// - WASM: paulocoutinhox/pdfium-lib
/// - Other platforms: bblanchon/pdfium-binaries
fn get_pdfium_url_and_lib(target: &str) -> (String, String) {
    if target.contains("wasm") {
        let version = env::var("PDFIUM_WASM_VERSION")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| get_latest_version("paulocoutinhox/pdfium-lib"));
        tracing::debug!("Using pdfium-lib version: {}", version);

        return (
            format!(
                "https://github.com/paulocoutinhox/pdfium-lib/releases/download/{}/wasm.tgz",
                version
            ),
            "pdfium".to_string(),
        );
    }

    let (platform, arch) = if target.contains("darwin") {
        let arch = if target.contains("aarch64") { "arm64" } else { "x64" };
        ("mac", arch)
    } else if target.contains("linux") {
        let arch = if target.contains("aarch64") {
            "arm64"
        } else if target.contains("arm") {
            "arm"
        } else {
            "x64"
        };
        ("linux", arch)
    } else if target.contains("windows") {
        let arch = if target.contains("aarch64") {
            "arm64"
        } else if target.contains("i686") {
            "x86"
        } else {
            "x64"
        };
        ("win", arch)
    } else {
        panic!("Unsupported target platform: {}", target);
    };

    let version = env::var("PDFIUM_VERSION")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| get_latest_version("bblanchon/pdfium-binaries"));
    tracing::debug!("Using pdfium-binaries version: {}", version);

    let url = format!(
        "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/{}/pdfium-{}-{}.tgz",
        version, platform, arch
    );

    (url, "pdfium".to_string())
}

/// Download and extract PDFium archive with retry logic
///
/// Features:
/// - Exponential backoff retry (configurable via env vars)
/// - File type validation (gzip check)
/// - Windows-specific import library handling (pdfium.dll.lib -> pdfium.lib)
/// - Environment variables:
///   - KREUZBERG_PDFIUM_DOWNLOAD_RETRIES: number of retries (default: 5)
///   - KREUZBERG_PDFIUM_DOWNLOAD_BACKOFF_SECS: initial backoff in seconds (default: 2)
fn download_and_extract_pdfium(url: &str, dest_dir: &Path) {
    fs::create_dir_all(dest_dir).expect("Failed to create pdfium directory");

    let archive_path = dest_dir.join("pdfium.tar.gz");
    let retries = env::var("KREUZBERG_PDFIUM_DOWNLOAD_RETRIES")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(5);
    let base_delay = env::var("KREUZBERG_PDFIUM_DOWNLOAD_BACKOFF_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(2);

    let archive_path_str = archive_path
        .to_str()
        .unwrap_or_else(|| panic!("Non-UTF8 path for archive: {}", archive_path.display()));
    let mut last_error = String::new();

    for attempt in 1..=retries {
        let _ = fs::remove_file(&archive_path);
        tracing::debug!(
            "Downloading Pdfium archive from: {} (attempt {}/{})",
            url,
            attempt,
            retries
        );

        let status = Command::new("curl")
            .args(["-f", "-L", "-o", archive_path_str, url])
            .status();

        match status {
            Ok(code) if code.success() => {
                last_error.clear();
                break;
            }
            Ok(code) => {
                last_error = format!("curl exited with {:?}", code.code());
            }
            Err(err) => {
                last_error = format!("failed to spawn curl: {err}");
            }
        }

        if attempt == retries {
            panic!(
                "Failed to download Pdfium from {} after {} attempts. Last error: {}",
                url, retries, last_error
            );
        }

        let exponent = u32::min(attempt, 5);
        let multiplier = 1u64 << exponent;
        let delay_secs = base_delay.saturating_mul(multiplier).min(30);
        println!(
            "cargo:warning=Pdfium download failed (attempt {}/{}) - {}. Retrying in {}s",
            attempt, retries, last_error, delay_secs
        );
        thread::sleep(Duration::from_secs(delay_secs));
    }

    let file_type = Command::new("file")
        .arg(archive_path.to_str().unwrap())
        .output()
        .expect("Failed to check file type");

    let file_type_output = String::from_utf8_lossy(&file_type.stdout);
    tracing::debug!("Downloaded file type: {}", file_type_output.trim());

    if !file_type_output.to_lowercase().contains("gzip") && !file_type_output.to_lowercase().contains("compressed") {
        fs::remove_file(&archive_path).ok();
        panic!(
            "Downloaded file is not a valid gzip archive. URL may be incorrect or version unavailable: {}",
            url
        );
    }

    tracing::debug!("Extracting Pdfium archive...");
    let status = Command::new("tar")
        .args(["-xzf", archive_path.to_str().unwrap(), "-C", dest_dir.to_str().unwrap()])
        .status()
        .expect("Failed to execute tar");

    if !status.success() {
        fs::remove_file(&archive_path).ok();
        panic!("Failed to extract Pdfium archive from {}", url);
    }

    fs::remove_file(&archive_path).ok();

    let target = env::var("TARGET").unwrap();
    if target.contains("windows") {
        let lib_dir = dest_dir.join("lib");
        let dll_lib = lib_dir.join("pdfium.dll.lib");
        let expected_lib = lib_dir.join("pdfium.lib");

        if dll_lib.exists() {
            tracing::debug!("Ensuring Windows import library at {}", expected_lib.display());
            if let Err(err) = fs::copy(&dll_lib, &expected_lib) {
                panic!("Failed to copy pdfium.dll.lib to pdfium.lib: {err}");
            }
        } else {
            tracing::debug!("Warning: Expected {} not found after extraction", dll_lib.display());
        }
    }

    tracing::debug!("Pdfium downloaded and extracted successfully");
}

/// Prepare prebuilt PDFium by copying to destination directory
///
/// Removes existing destination if present, then recursively copies
/// all files from prebuilt source to destination.
fn prepare_prebuilt_pdfium(prebuilt_src: &Path, dest_dir: &Path) -> io::Result<()> {
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir)?;
    }
    copy_dir_all(prebuilt_src, dest_dir)
}

/// Recursively copy directory tree
///
/// Used by `prepare_prebuilt_pdfium()` to copy entire pdfium directory
/// structure, preserving all files and subdirectories.
fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target_path)?;
        } else {
            fs::copy(entry.path(), &target_path)?;
        }
    }
    Ok(())
}

/// Get platform-specific runtime library name and subdirectory
///
/// Returns tuple of (library_name, subdirectory) for the target platform:
/// - WASM: ("libpdfium.a", "release/lib")
/// - Windows: ("pdfium.dll", "bin")
/// - macOS: ("libpdfium.dylib", "lib")
/// - Linux: ("libpdfium.so", "lib")
fn runtime_library_info(target: &str) -> (String, &'static str) {
    if target.contains("wasm") {
        ("libpdfium.a".to_string(), "release/lib")
    } else if target.contains("windows") {
        ("pdfium.dll".to_string(), "bin")
    } else if target.contains("darwin") {
        ("libpdfium.dylib".to_string(), "lib")
    } else {
        ("libpdfium.so".to_string(), "lib")
    }
}

/// Find PDFium library in archive with flexible directory detection
///
/// Attempts to locate the library at multiple possible locations:
/// - {subdir}/{lib_name} (standard location)
/// - {lib_name} (root of archive)
/// - bin/{lib_name} (alternative location)
/// - lib/{lib_name} (explicit lib directory)
///
/// This handles variations in archive structure across different platform builds,
/// particularly macOS ARM64 where the archive structure may differ.
///
/// Returns the full path to the library if found, or an error with available files.
fn find_pdfium_library(pdfium_dir: &Path, lib_name: &str, expected_subdir: &str) -> Result<PathBuf, String> {
    let candidates = [
        pdfium_dir.join(expected_subdir).join(lib_name),
        pdfium_dir.join(lib_name),
        pdfium_dir.join("bin").join(lib_name),
        pdfium_dir.join("lib").join(lib_name),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            tracing::debug!("Found PDFium library at: {}", candidate.display());
            return Ok(candidate.clone());
        }
    }

    let mut error_msg = format!(
        "PDFium library not found at expected location: {}/{}\n\n",
        pdfium_dir.display(),
        expected_subdir
    );
    error_msg.push_str("Attempted locations:\n");
    for candidate in &candidates {
        error_msg.push_str(&format!("  - {}\n", candidate.display()));
    }

    error_msg.push_str("\nActual archive contents:\n");
    if let Ok(entries) = fs::read_dir(pdfium_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_type = if path.is_dir() { "dir" } else { "file" };
            error_msg.push_str(&format!("  {} ({})\n", path.display(), file_type));

            if path.is_dir()
                && let Ok(sub_entries) = fs::read_dir(&path)
            {
                for sub_entry in sub_entries.flatten() {
                    let sub_path = sub_entry.path();
                    let sub_type = if sub_path.is_dir() { "dir" } else { "file" };
                    error_msg.push_str(&format!("    {} ({})\n", sub_path.display(), sub_type));
                }
            }
        }
    }

    Err(error_msg)
}

/// Link PDFium dynamically (default)
///
/// Sets up linker to use PDFium as a dynamic library (.dylib/.so/.dll)
/// with platform-specific rpath configuration for runtime library discovery.
/// Supports flexible archive structures by adding multiple possible lib directories.
fn link_dynamically(pdfium_dir: &Path, target: &str) {
    let (runtime_lib_name, runtime_subdir) = runtime_library_info(target);

    let lib_path = match find_pdfium_library(pdfium_dir, &runtime_lib_name, runtime_subdir) {
        Ok(path) => path.parent().unwrap_or(pdfium_dir).to_path_buf(),
        Err(err) => panic!("{}", err),
    };

    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=dylib=pdfium");

    let std_lib_dir = pdfium_dir.join("lib");
    if std_lib_dir.exists() && std_lib_dir != lib_path {
        println!("cargo:rustc-link-search=native={}", std_lib_dir.display());
    }

    let bin_dir = pdfium_dir.join("bin");
    if bin_dir.exists() && bin_dir != lib_path {
        println!("cargo:rustc-link-search=native={}", bin_dir.display());
    }

    if target.contains("darwin") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/.");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/.");
    }
}

/// Link PDFium statically (static-pdfium feature)
///
/// Embeds PDFium into the binary as a static library. Adds system
/// dependencies required for static linking on Linux.
/// Supports flexible archive structures by finding library in multiple locations.
///
/// Environment Variables:
/// - `PDFIUM_STATIC_LIB_PATH`: Path to directory containing libpdfium.a (for Docker/musl builds)
///
/// Note: bblanchon/pdfium-binaries only provides dynamic libraries.
/// On macOS, this will fallback to dynamic linking with a warning.
/// On Linux, you must provide PDFIUM_STATIC_LIB_PATH pointing to a static build.
fn link_statically(pdfium_dir: &Path, target: &str) {
    let static_lib_name = "libpdfium.a";
    let lib_subdir = if target.contains("wasm") { "release/lib" } else { "lib" };

    if let Ok(custom_path) = env::var("PDFIUM_STATIC_LIB_PATH") {
        let custom_lib_dir = PathBuf::from(&custom_path);

        if !custom_lib_dir.exists() {
            panic!(
                "PDFIUM_STATIC_LIB_PATH points to '{}' but the directory does not exist",
                custom_path
            );
        }

        let custom_lib = custom_lib_dir.join(static_lib_name);
        if !custom_lib.exists() {
            panic!(
                "PDFIUM_STATIC_LIB_PATH points to '{}' but {} not found.\n\
                 Expected to find: {}",
                custom_path,
                static_lib_name,
                custom_lib.display()
            );
        }

        tracing::debug!("Using custom static PDFium from: {}", custom_lib.display());
        println!("cargo:rustc-link-search=native={}", custom_lib_dir.display());
        println!("cargo:rustc-link-lib=static=pdfium");

        if target.contains("linux") {
            println!("cargo:rustc-link-lib=dylib=pthread");
            println!("cargo:rustc-link-lib=dylib=dl");
        } else if target.contains("windows") {
            println!("cargo:rustc-link-lib=dylib=ws2_32");
            println!("cargo:rustc-link-lib=dylib=userenv");
        }

        return;
    }

    let lib_path = match find_pdfium_library(pdfium_dir, static_lib_name, lib_subdir) {
        Ok(path) => path.parent().unwrap_or(pdfium_dir).to_path_buf(),
        Err(_err) => {
            if target.contains("darwin") {
                eprintln!("cargo:warning=Static PDFium library (libpdfium.a) not found for macOS.");
                eprintln!("cargo:warning=bblanchon/pdfium-binaries only provides dynamic libraries.");
                eprintln!("cargo:warning=Falling back to dynamic linking for local development.");
                eprintln!("cargo:warning=Production Linux builds require PDFIUM_STATIC_LIB_PATH.");

                link_dynamically(pdfium_dir, target);
                return;
            } else {
                panic!(
                    "Static PDFium library (libpdfium.a) not found.\n\n\
                     bblanchon/pdfium-binaries only provides dynamic libraries.\n\n\
                     For static linking (required for Docker with musl), you must:\n\n\
                     1. Build static PDFium or obtain from a source that provides it\n\
                        - See: https://github.com/ajrcarey/pdfium-render/issues/53\n\
                        - Or use: https://github.com/paulocoutinhox/pdfium-lib (provides static builds)\n\n\
                     2. Set environment variable pointing to the directory containing libpdfium.a:\n\
                        export PDFIUM_STATIC_LIB_PATH=/path/to/pdfium/lib\n\n\
                     3. Or use alternative features:\n\
                        - 'pdf' (dynamic linking, requires .so at runtime)\n\
                        - 'bundled-pdfium' (embeds dynamic library in binary)\n\
                        - 'system-pdfium' (use system-installed pdfium)\n\n\
                     Example Dockerfile pattern:\n\
                        FROM alpine:latest as pdfium-builder\n\
                        # Download/build static libpdfium.a\n\
                        \n\
                        FROM rust:alpine as builder\n\
                        ENV PDFIUM_STATIC_LIB_PATH=/pdfium/lib\n\
                        COPY --from=pdfium-builder /path/to/libpdfium.a /pdfium/lib/"
                );
            }
        }
    };

    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=static=pdfium");

    let std_lib_dir = pdfium_dir.join("lib");
    if std_lib_dir.exists() && std_lib_dir != lib_path {
        println!("cargo:rustc-link-search=native={}", std_lib_dir.display());
    }

    let bin_dir = pdfium_dir.join("bin");
    if bin_dir.exists() && bin_dir != lib_path {
        println!("cargo:rustc-link-search=native={}", bin_dir.display());
    }

    if target.contains("linux") {
        println!("cargo:rustc-link-lib=dylib=pthread");
        println!("cargo:rustc-link-lib=dylib=dl");
    } else if target.contains("windows") {
        println!("cargo:rustc-link-lib=dylib=ws2_32");
        println!("cargo:rustc-link-lib=dylib=userenv");
    }
}

/// Link PDFium bundled (bundled-pdfium feature)
///
/// Links dynamically but copies library to OUT_DIR for embedding in binary.
/// Each binary extracts and uses its own copy of the PDFium library.
/// Supports flexible archive structures by finding library in multiple locations.
///
/// For WASM targets, links statically using the bundled static library.
fn link_bundled(pdfium_dir: &Path, target: &str, out_dir: &Path) {
    let (runtime_lib_name, runtime_subdir) = runtime_library_info(target);
    let src_lib = match find_pdfium_library(pdfium_dir, &runtime_lib_name, runtime_subdir) {
        Ok(path) => path,
        Err(err) => panic!("{}", err),
    };
    let bundled_lib = out_dir.join(&runtime_lib_name);

    fs::copy(&src_lib, &bundled_lib)
        .unwrap_or_else(|err| panic!("Failed to copy library to OUT_DIR for bundling: {}", err));

    let bundled_path = bundled_lib
        .to_str()
        .unwrap_or_else(|| panic!("Non-UTF8 path for bundled library: {}", bundled_lib.display()));
    println!("cargo:rustc-env=KREUZBERG_PDFIUM_BUNDLED_PATH={}", bundled_path);

    if target.contains("wasm") {
        let lib_dir = bundled_lib
            .parent()
            .unwrap_or_else(|| panic!("Invalid bundled library path: {}", bundled_lib.display()));
        println!("cargo:rustc-link-search=native={}", lib_dir.display());
        println!("cargo:rustc-link-lib=static=pdfium");
        tracing::debug!("Bundled PDFium static library linked for WASM at: {}", bundled_path);
    } else {
        tracing::debug!("Bundled PDFium library at: {}", bundled_path);
    }
}

/// Link system-installed PDFium (system-pdfium feature)
///
/// Attempts to find PDFium via pkg-config first, then falls back to
/// environment variables (KREUZBERG_PDFIUM_SYSTEM_PATH, KREUZBERG_PDFIUM_SYSTEM_INCLUDE).
fn link_system(_target: &str) {
    match pkg_config::Config::new().atleast_version("5.0").probe("pdfium") {
        Ok(library) => {
            tracing::debug!("Found system pdfium via pkg-config");
            for include_path in &library.include_paths {
                println!("cargo:include={}", include_path.display());
            }
            return;
        }
        Err(err) => {
            tracing::debug!("pkg-config probe failed: {}", err);
        }
    }

    let lib_path = env::var("KREUZBERG_PDFIUM_SYSTEM_PATH").ok();
    let include_path = env::var("KREUZBERG_PDFIUM_SYSTEM_INCLUDE").ok();

    if let Some(lib_dir) = lib_path {
        let lib_dir_path = PathBuf::from(&lib_dir);
        if !lib_dir_path.exists() {
            panic!(
                "KREUZBERG_PDFIUM_SYSTEM_PATH points to '{}' but the directory does not exist",
                lib_dir
            );
        }

        println!("cargo:rustc-link-search=native={}", lib_dir);
        println!("cargo:rustc-link-lib=dylib=pdfium");

        if let Some(inc_dir) = include_path {
            println!("cargo:include={}", inc_dir);
        }

        tracing::debug!("Using system pdfium from: {}", lib_dir);
        return;
    }

    panic!(
        "system-pdfium feature enabled but pdfium not found.\n\
         \n\
         Please install pdfium system-wide or provide:\n\
         - KREUZBERG_PDFIUM_SYSTEM_PATH: path to directory containing libpdfium\n\
         - KREUZBERG_PDFIUM_SYSTEM_INCLUDE: path to pdfium headers (optional)\n\
         \n\
         Alternatively, use a different linking strategy:\n\
         - Default (dynamic): cargo build --features pdf\n\
         - Static linking: cargo build --features pdf,static-pdfium\n\
         - Bundled: cargo build --features pdf,bundled-pdfium"
    );
}

/// Link system frameworks and standard libraries
///
/// Adds platform-specific system libraries required for PDFium linking:
/// - macOS: CoreFoundation, CoreGraphics, CoreText, AppKit, libc++
/// - Linux: stdc++, libm
/// - Windows: gdi32, user32, advapi32
fn link_system_frameworks(target: &str) {
    if target.contains("darwin") {
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=CoreText");
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if target.contains("linux") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=dylib=m");
    } else if target.contains("windows") {
        println!("cargo:rustc-link-lib=dylib=gdi32");
        println!("cargo:rustc-link-lib=dylib=user32");
        println!("cargo:rustc-link-lib=dylib=advapi32");
    }
}
