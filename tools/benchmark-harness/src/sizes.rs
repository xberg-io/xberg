//! Framework size measurement
//!
//! Measures the installation footprint of document extraction frameworks.
//!
//! Xberg bindings are measured dynamically from local build artifacts.
//! Third-party frameworks use hardcoded verified sizes (package + transitive
//! deps + system deps + auto-downloaded ML models) because dynamic measurement
//! is unreliable: pip-weigh times out for large packages (torch, transformers),
//! and dpkg-query returns partial results when package names vary across distros.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Information about a framework's disk size
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkSize {
    /// Size in bytes (package + system deps + models combined)
    pub size_bytes: u64,
    /// Package-only size in bytes (Python/npm package + transitive deps)
    #[serde(default)]
    pub package_bytes: u64,
    /// System dependency size in bytes (libreoffice, tesseract, ffmpeg, etc.)
    #[serde(default)]
    pub system_deps_bytes: u64,
    /// ML model size in bytes (auto-downloaded on first use: torch models, OCR weights, etc.)
    #[serde(default, skip_serializing_if = "is_zero")]
    pub model_bytes: u64,
    /// Method used to measure (pip_package, npm_package, binary_size, jar_size, etc.)
    pub method: String,
    /// Human-readable description
    pub description: String,
    /// Breakdown of system dependency sizes by package name.
    /// Populated when runtime measurement via dpkg-query succeeds.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub system_deps_detail: HashMap<String, u64>,
}

fn is_zero(v: &u64) -> bool {
    *v == 0
}

/// Framework size measurement results
pub type FrameworkSizes = HashMap<String, FrameworkSize>;

/// Known frameworks with their measurement methods and descriptions
const FRAMEWORKS: &[(&str, &str, &str)] = &[
    ("xberg-rust", "binary_size", "Native Rust core binary"),
    ("xberg-python", "pip_package", "Python wheel package"),
    ("xberg-node", "npm_package", "Node.js native addon"),
    ("xberg-wasm", "wasm_bundle", "WebAssembly binary"),
    ("xberg-ruby", "gem_package", "Ruby gem native extension"),
    ("xberg-go", "binary_size", "Go binary with CGO"),
    ("xberg-java", "jar_size", "Java JAR with JNI"),
    ("xberg-csharp", "nuget_package", ".NET NuGet package"),
    ("xberg-elixir", "hex_package", "Elixir hex package with NIF"),
    ("xberg-php", "php_extension", "PHP extension"),
    ("xberg-c", "binary_size", "C FFI binding"),
    ("xberg-rust-paddle", "binary_size", "Native Rust core with PaddleOCR"),
    ("docling", "pip_package", "IBM Docling document processing"),
    ("markitdown", "pip_package", "Mark It Down markdown converter"),
    ("unstructured", "pip_package", "Unstructured document processing"),
    ("tika", "jar_size", "Apache Tika content analysis"),
    ("pymupdf4llm", "pip_package", "PyMuPDF for LLM"),
    ("mineru", "pip_package", "MinerU document intelligence"),
    ("liteparse", "binary_size", "LiteParse (run-llama) Rust PDF parser"),
];

/// Verified installation footprints for third-party frameworks.
///
/// Each entry: (name, package_bytes, system_deps_bytes, model_bytes, description).
///
/// - **package_bytes**: Python package + all transitive pip dependencies (from pip-weigh).
/// - **system_deps_bytes**: Required system packages (poppler, libreoffice, JRE, ffmpeg, etc.).
/// - **model_bytes**: ML models auto-downloaded on first use (HuggingFace, PaddleOCR, etc.).
///
/// Values measured on Linux x86_64 (Ubuntu 22.04) in March 2026.
/// Sources: pip-weigh --json, PyPI wheel sizes, HuggingFace model pages, apt show.
const KNOWN_THIRD_PARTY_SIZES: &[(&str, u64, u64, u64, &str)] = &[
    ("pymupdf4llm", 51_500_000, 0, 0, "PyMuPDF for LLM"),
    (
        "markitdown",
        80_000_000,
        125_000_000,
        0,
        "Mark It Down markdown converter",
    ),
    ("tika", 57_000_000, 215_000_000, 0, "Apache Tika content analysis"),
    (
        "docling",
        2_500_000_000,
        0,
        470_000_000,
        "IBM Docling document processing",
    ),
    (
        "unstructured",
        300_000_000,
        840_000_000,
        217_000_000,
        "Unstructured document processing",
    ),
    ("mineru", 2_000_000_000, 0, 650_000_000, "MinerU document intelligence"),
    ("liteparse", 35_000_000, 0, 0, "LiteParse (run-llama) Rust PDF parser"),
];

/// Look up a hardcoded third-party size entry.
fn lookup_known_size(name: &str) -> Option<FrameworkSize> {
    KNOWN_THIRD_PARTY_SIZES
        .iter()
        .find(|(n, ..)| *n == name)
        .map(|(_, pkg, sys, models, desc)| FrameworkSize {
            size_bytes: pkg + sys + models,
            package_bytes: *pkg,
            system_deps_bytes: *sys,
            model_bytes: *models,
            method: "known_size".to_string(),
            description: desc.to_string(),
            system_deps_detail: HashMap::new(),
        })
}

/// Measure framework sizes.
///
/// Third-party frameworks use hardcoded verified values (package + deps + models).
/// Xberg bindings are measured dynamically from local build artifacts.
/// Frameworks that are not installed are silently skipped.
pub fn measure_framework_sizes() -> Result<FrameworkSizes> {
    let mut sizes = HashMap::new();

    for (name, method, description) in FRAMEWORKS {
        if let Some(known) = lookup_known_size(name) {
            sizes.insert(name.to_string(), known);
            continue;
        }

        // The native xberg CLI is measured with a shipped-vs-model breakdown so
        // benchmark rows can report install size fairly (heuristic rows exclude
        // the on-demand model cache; ML rows include it). ~keep
        if *name == "xberg-rust" {
            match measure_xberg_framework_size(description) {
                Some(fs) => {
                    sizes.insert(name.to_string(), fs);
                }
                None => eprintln!("Size measurement: xberg-rust - binary not found, skipping"),
            }
            continue;
        }

        match measure_framework(name, method) {
            Ok(Some(pkg_size)) => {
                sizes.insert(
                    name.to_string(),
                    FrameworkSize {
                        size_bytes: pkg_size,
                        package_bytes: pkg_size,
                        system_deps_bytes: 0,
                        model_bytes: 0,
                        method: method.to_string(),
                        description: description.to_string(),
                        system_deps_detail: HashMap::new(),
                    },
                );
            }
            Ok(None) => {
                eprintln!("Size measurement: {} ({}) - not installed, skipping", name, method);
            }
            Err(e) => {
                eprintln!("Size measurement: {} ({}) - failed: {}", name, method, e);
            }
        }
    }

    Ok(sizes)
}

/// Measure framework sizes, failing if any xberg binding cannot be measured.
///
/// Third-party frameworks always succeed (hardcoded values).
/// Xberg bindings must be measurable or an error is returned.
pub fn measure_framework_sizes_strict() -> Result<FrameworkSizes> {
    let mut sizes = HashMap::new();
    let mut errors = Vec::new();

    for (name, method, description) in FRAMEWORKS {
        if let Some(known) = lookup_known_size(name) {
            sizes.insert(name.to_string(), known);
            continue;
        }

        if *name == "xberg-rust" {
            match measure_xberg_framework_size(description) {
                Some(fs) => {
                    sizes.insert(name.to_string(), fs);
                }
                None => errors.push(format!("{} ({})", name, method)),
            }
            continue;
        }

        match measure_framework(name, method) {
            Ok(Some(pkg_size)) => {
                sizes.insert(
                    name.to_string(),
                    FrameworkSize {
                        size_bytes: pkg_size,
                        package_bytes: pkg_size,
                        system_deps_bytes: 0,
                        model_bytes: 0,
                        method: method.to_string(),
                        description: description.to_string(),
                        system_deps_detail: HashMap::new(),
                    },
                );
            }
            Ok(None) | Err(_) => {
                errors.push(format!("{} ({})", name, method));
            }
        }
    }

    if !errors.is_empty() {
        return Err(Error::Benchmark(format!(
            "Failed to measure sizes for frameworks: {}. Install these frameworks or use measure_framework_sizes() for lenient mode.",
            errors.join(", ")
        )));
    }

    Ok(sizes)
}

/// Measure a single framework.
/// Returns Ok(Some(size)) for successful measurement, Ok(None) for frameworks
/// that aren't installed, or Err for measurement failures.
fn measure_framework(name: &str, method: &str) -> Result<Option<u64>> {
    match method {
        "pip_package" => measure_pip_package(extract_package_name(name)),
        "npm_package" => measure_npm_package(extract_package_name(name)),
        // Note: `xberg-rust` is measured separately via `measure_xberg_framework_size`
        // (shipped-vs-model breakdown) before this generic dispatch is reached. ~keep
        "binary_size" => measure_binary(name),
        "jar_size" => measure_jar(name),
        "gem_package" => measure_gem_package(extract_package_name(name)),
        "wasm_bundle" => measure_wasm_bundle(name),
        "nuget_package" => measure_nuget_package(name),
        "hex_package" => measure_hex_package(name),
        "php_extension" => measure_php_extension(name),
        _ => Err(Error::Benchmark(format!("Unknown measurement method: {}", method))),
    }
}

/// Extract Python/npm/gem package name from framework name
fn extract_package_name(framework: &str) -> &str {
    let name = framework.strip_suffix("-batch").unwrap_or(framework);

    match name {
        "xberg-python" => "xberg",
        "xberg-node" => "@xberg-io/xberg",
        "xberg-ruby" => "xberg_rb",
        "docling" => "docling",
        "markitdown" => "markitdown",
        "unstructured" => "unstructured",
        "pymupdf4llm" => "pymupdf4llm",
        "mineru" => "mineru",
        _ => name,
    }
}

/// Measure Python package size via `uv pip show`.
///
/// Packages must be installed in the project .venv via `uv sync --group bench-*`.
/// Returns an error if the package cannot be found or measured.
///
/// For xberg: measures the single package directory (includes native .so).
/// For third-party frameworks (docling, unstructured, mineru, etc.): uses
/// `pip-weigh` to measure the package + full transitive dependency tree in an
/// isolated venv, capturing deps like torch/transformers that dominate the
/// actual installation footprint.
fn measure_pip_package(package: &str) -> Result<Option<u64>> {
    if package == "xberg"
        && let Some(size) = measure_pip_package_via_python(package)
    {
        return Ok(Some(size));
    }

    if package != "xberg"
        && let Some(size) = measure_pip_weigh(package)
    {
        return Ok(Some(size));
    }

    if let Some(size) = measure_pip_package_via_python(package) {
        return Ok(Some(size));
    }

    let output = match Command::new("uv").args(["pip", "show", "-f", package]).output() {
        Ok(output) => output,
        Err(_) => return Ok(None),
    };

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_pip_show_size(&stdout, package))
}

/// Use `pip-weigh --json <package>` to measure a package's total installation
/// footprint including all transitive dependencies. pip-weigh creates an
/// isolated venv, installs the package, and measures via .dist-info/RECORD.
/// Returns None if pip-weigh is not installed or the command fails.
fn measure_pip_weigh(package: &str) -> Option<u64> {
    let output = Command::new("pip-weigh").args(["--json", package]).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).ok()?;
    json.get("results")?.get(0)?.get("total_size_bytes")?.as_u64()
}

/// Parse pip show -f output to extract package size
fn parse_pip_show_size(stdout: &str, package: &str) -> Option<u64> {
    let location_line = stdout.lines().find(|l| l.starts_with("Location:"))?;
    let location = location_line.strip_prefix("Location:")?.trim();
    let location_path = Path::new(location);

    if let Some(editable_line) = stdout.lines().find(|l| l.starts_with("Editable project location:"))
        && let Some(editable_path) = editable_line
            .strip_prefix("Editable project location:")
            .map(|s| s.trim())
    {
        let project_dir = Path::new(editable_path);
        let pkg_dir = project_dir.join(package.replace('-', "_"));
        if pkg_dir.exists() {
            return Some(dir_size(&pkg_dir));
        }
        if project_dir.exists() {
            return Some(dir_size(project_dir));
        }
    }

    let package_dir = location_path.join(package.replace('-', "_"));
    if package_dir.exists() {
        return Some(dir_size(&package_dir));
    }

    let mut in_files_section = false;
    let mut total_size: u64 = 0;
    let mut found_files = false;
    for line in stdout.lines() {
        if line.starts_with("Files:") {
            in_files_section = true;
            continue;
        }
        if in_files_section {
            let file_rel = line.trim();
            if file_rel.is_empty() {
                continue;
            }
            if !line.starts_with(' ') && !line.starts_with('\t') {
                break;
            }
            let file_path = location_path.join(file_rel);
            if let Ok(metadata) = fs::metadata(&file_path) {
                total_size += metadata.len();
                found_files = true;
            }
        }
    }
    if found_files {
        return Some(total_size);
    }

    None
}

/// Measure npm package size including native addon binary
fn measure_npm_package(package: &str) -> Result<Option<u64>> {
    if package.contains("xberg") && package.contains("node") {
        let mut total: u64 = 0;

        let node_crate = Path::new("crates/xberg-node");
        if node_crate.exists() {
            if let Ok(entries) = fs::read_dir(node_crate) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str())
                        && name.ends_with(".node")
                        && let Ok(metadata) = fs::metadata(&path)
                    {
                        total += metadata.len();
                    }
                }
            }
            let dist_dir = node_crate.join("dist");
            if dist_dir.exists() {
                total += dir_size(&dist_dir);
            }
        }

        let npm_dir = node_crate.join("npm");
        if npm_dir.exists()
            && let Ok(entries) = fs::read_dir(&npm_dir)
        {
            for entry in entries.flatten() {
                let platform_dir = entry.path();
                if platform_dir.is_dir()
                    && let Ok(files) = fs::read_dir(&platform_dir)
                {
                    for file in files.flatten() {
                        if file.path().extension().and_then(|e| e.to_str()) == Some("node")
                            && let Ok(metadata) = file.metadata()
                        {
                            total += metadata.len();
                        }
                    }
                }
            }
        }

        if total > 0 {
            return Ok(Some(total));
        }
    }

    let output = Command::new("npm")
        .args(["pack", "--dry-run", "--json", package])
        .output()
        .ok();

    if let Some(output) = output
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout)
            && let Some(size) = json.get(0).and_then(|v| v.get("size")).and_then(|v| v.as_u64())
        {
            return Ok(Some(size));
        }
    }

    Ok(None)
}

/// Measure the native xberg CLI install footprint, split so benchmark rows can
/// report install size fairly against competitors.
///
/// - `package_bytes` = shipped footprint: the compiled `xberg`/`xberg-cli`
///   release binary plus any bundled native libraries (ONNX Runtime, tesseract,
///   tree-sitter). This is what a heuristic-only (no-ML) run needs on disk, and
///   the fair comparison point against model-free tools like LiteParse.
/// - `model_bytes` = the on-demand ML model cache (platform cache dir), pulled on
///   first use by the layout/OCR/embedding paths. Comparable to how Docling's
///   auto-downloaded models are reported separately.
/// - `size_bytes` = `package_bytes + model_bytes` (total, matching the
///   convention used for third-party frameworks).
///
/// Returns `None` if no binary is present.
fn measure_xberg_framework_size(description: &str) -> Option<FrameworkSize> {
    let binary_size = [
        "target/release/xberg",
        "target/release/xberg-cli",
        "target/debug/xberg",
        "target/debug/xberg-cli",
    ]
    .iter()
    .find_map(|path| fs::metadata(path).ok().map(|m| m.len()))
    .filter(|size| *size > 0)?;

    let ffi_size = measure_native_ffi_libs();

    let model_size = xberg_cache_base()
        .filter(|dir| dir.exists())
        .map(|dir| dir_size(&dir))
        .unwrap_or(0);

    let package_bytes = binary_size + ffi_size;
    eprintln!(
        "Xberg measurement: binary={} bytes, ffi_libs={} bytes, cached_models={} bytes (shipped={}, total={})",
        binary_size,
        ffi_size,
        model_size,
        package_bytes,
        package_bytes + model_size,
    );

    Some(FrameworkSize {
        size_bytes: package_bytes + model_size,
        package_bytes,
        system_deps_bytes: 0,
        model_bytes: model_size,
        method: "binary_size".to_string(),
        description: description.to_string(),
        system_deps_detail: HashMap::new(),
    })
}

/// Resolve the xberg model cache base directory, mirroring the core's
/// `cache_dir::resolve_cache_base`: honor `XBERG_CACHE_DIR`, else the
/// platform-appropriate global cache dir (`dirs::cache_dir()/xberg`).
///
/// This must match the core, or the measured `model_bytes` is wrong — notably
/// on macOS the cache lives at `~/Library/Caches/xberg`, not `~/.cache/xberg`.
fn xberg_cache_base() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("XBERG_CACHE_DIR") {
        return Some(PathBuf::from(env_path));
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return Some(Path::new(&home).join("Library/Caches/xberg"));
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return Some(Path::new(&local).join("xberg"));
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
            return Some(Path::new(&xdg).join("xberg"));
        }
        if let Ok(home) = std::env::var("HOME") {
            return Some(Path::new(&home).join(".cache/xberg"));
        }
    }

    None
}

/// Measure binary size
fn measure_binary(name: &str) -> Result<Option<u64>> {
    let binary_name = match name {
        "xberg-rust" => "xberg",
        s if s.starts_with("xberg-go") => "xberg-go",
        "xberg-c" | "xberg-rust-paddle" => name,
        _ => return Ok(None),
    };

    if matches!(name, "xberg-rust" | "xberg-c" | "xberg-rust-paddle") {
        let target_paths = [
            "target/release/libxberg_ffi.so",
            "target/release/libxberg_ffi.dylib",
            "target/release/xberg_ffi.dll",
            "target/release/libxberg_ffi.a",
            "target/release/xberg",
            "target/debug/xberg",
            "target/release/libxberg.so",
            "target/release/libxberg.dylib",
            "target/release/xberg.dll",
        ];
        for path in target_paths {
            if let Ok(metadata) = fs::metadata(path) {
                return Ok(Some(metadata.len()));
            }
        }
    }

    if name.starts_with("xberg-go") {
        let go_ffi_paths = [
            "target/release/libxberg_ffi.so",
            "target/release/libxberg_ffi.dylib",
            "target/release/xberg_ffi.dll",
        ];
        for path in go_ffi_paths {
            if let Ok(metadata) = fs::metadata(path) {
                return Ok(Some(metadata.len()));
            }
        }
        let ffi_size = measure_native_ffi_libs();
        if ffi_size > 0 {
            return Ok(Some(ffi_size));
        }
        return Ok(None);
    }

    let output = Command::new("which").arg(binary_name).output().ok();

    if let Some(output) = output
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if let Ok(metadata) = fs::metadata(&path) {
            return Ok(Some(metadata.len()));
        }
    }

    Ok(None)
}

/// Measure JAR size (Apache Tika)
fn measure_jar(name: &str) -> Result<Option<u64>> {
    let possible_paths = [
        "/usr/share/java/tika-app.jar",
        "/opt/tika/tika-app.jar",
        "~/.local/share/tika/tika-app.jar",
    ];

    if name.starts_with("tika") {
        for path in possible_paths {
            let expanded = shellexpand::tilde(path);
            let expanded_path: &str = expanded.as_ref();
            if let Ok(metadata) = fs::metadata(expanded_path) {
                return Ok(Some(metadata.len()));
            }
        }

        if let Ok(jar_path) = std::env::var("TIKA_JAR")
            && let Ok(metadata) = fs::metadata(&jar_path)
        {
            return Ok(Some(metadata.len()));
        }

        let libs_dir = Path::new("tools/benchmark-harness/libs");
        if let Ok(entries) = fs::read_dir(libs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.starts_with("tika-app-")
                    && name.ends_with(".jar")
                    && let Ok(metadata) = fs::metadata(&path)
                {
                    return Ok(Some(metadata.len()));
                }
            }
        }
    }

    if name.starts_with("xberg-java") {
        let mut total: u64 = 0;

        let classes_dir = Path::new("packages/java/target/classes");
        if classes_dir.exists() {
            total += dir_size(classes_dir);
        }

        let deps_dir = Path::new("packages/java/target/dependency");
        if deps_dir.exists() {
            total += dir_size(deps_dir);
        }

        let natives_dir = Path::new("packages/java/target/classes/natives");
        if !has_native_extension(natives_dir) {
            total += measure_native_ffi_libs();
        }

        if total > 0 {
            return Ok(Some(total));
        }

        let jar_path = Path::new("packages/java/target/xberg.jar");
        if let Ok(metadata) = fs::metadata(jar_path) {
            return Ok(Some(metadata.len()));
        }
    }

    Ok(None)
}

/// Measure Ruby gem size using bundle show or gem contents
fn measure_gem_package(package: &str) -> Result<Option<u64>> {
    let gem_name = match package {
        "xberg" | "xberg-ruby" => "xberg_rb",
        other => other,
    };

    if let Ok(output) = Command::new("bundle").args(["show", gem_name]).output()
        && output.status.success()
    {
        let gem_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !gem_path.is_empty() {
            let path = Path::new(&gem_path);
            if path.exists() {
                return Ok(Some(dir_size(path)));
            }
        }
    }

    if let Ok(output) = Command::new("ruby")
        .arg("-e")
        .arg(format!(
            "puts Gem::Specification.find_by_name('{}').gem_dir rescue nil",
            gem_name
        ))
        .output()
        && output.status.success()
    {
        let gem_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !gem_path.is_empty() {
            let path = Path::new(&gem_path);
            if path.exists() {
                return Ok(Some(dir_size(path)));
            }
        }
    }

    let ruby_pkg = Path::new("packages/ruby/pkg");
    if ruby_pkg.exists() {
        return Ok(Some(dir_size(ruby_pkg)));
    }
    let ruby_lib = Path::new("packages/ruby/lib");
    if ruby_lib.exists() {
        let lib_size = dir_size(ruby_lib);
        let mut total = lib_size;

        let has_substantial_native = has_native_extension(ruby_lib) && lib_size > 5_000_000;
        if !has_substantial_native {
            total += measure_native_ffi_libs();
        }

        if total > 0 {
            return Ok(Some(total));
        }
    }

    Ok(None)
}

/// Measure WebAssembly bundle size
fn measure_wasm_bundle(name: &str) -> Result<Option<u64>> {
    let wasm_paths = [
        "packages/wasm/pkg/xberg_bg.wasm",
        "packages/wasm/dist/xberg.wasm",
        "target/wasm32-unknown-unknown/release/xberg.wasm",
        "crates/xberg-wasm/pkg/xberg_wasm_bg.wasm",
    ];

    for path in wasm_paths {
        if let Ok(metadata) = fs::metadata(path) {
            return Ok(Some(metadata.len()));
        }
    }

    if name.contains("wasm") || name.contains("xberg") {
        let node_modules_paths = ["node_modules/@xberg-io/xberg-wasm"];
        for path in node_modules_paths {
            let dir = Path::new(path);
            if dir.exists() {
                return Ok(Some(dir_size(dir)));
            }
        }
    }

    Ok(None)
}

/// Measure .NET NuGet package size
///
/// Checks project build output directories first, then NuGet cache as fallback.
/// Always ensures native FFI libs are included in the total since the .NET
/// package depends on the Rust shared library at runtime.
fn measure_nuget_package(name: &str) -> Result<Option<u64>> {
    if name.starts_with("xberg-csharp") {
        let project_dirs = ["packages/csharp/Xberg", "packages/csharp/Xberg.Native"];
        for proj_dir_str in project_dirs {
            let proj_dir = Path::new(proj_dir_str);
            for config in ["Release", "Debug"] {
                let bin_dir = proj_dir.join("bin").join(config);
                if bin_dir.exists() {
                    let mut total = dir_size(&bin_dir);

                    if !has_native_extension(&bin_dir) {
                        total += measure_native_ffi_libs();
                    }

                    return Ok(Some(total));
                }
            }
        }

        for config in ["Release", "Debug"] {
            let bench_bin = Path::new("packages/csharp/Benchmark/bin").join(config);
            if bench_bin.exists() {
                let mut total = dir_size(&bench_bin);
                if !has_native_extension(&bench_bin) {
                    total += measure_native_ffi_libs();
                }
                return Ok(Some(total));
            }
        }

        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
        let nuget_cache_paths = [
            format!("{}/.nuget/packages/xberg", home),
            format!("{}/.nuget/packages/xberg.native", home),
        ];
        for path in nuget_cache_paths {
            let dir = Path::new(&path);
            if dir.exists() {
                let mut total = dir_size(dir);
                if !has_native_extension(dir) {
                    total += measure_native_ffi_libs();
                }
                return Ok(Some(total));
            }
        }

        let ffi_size = measure_native_ffi_libs();
        if ffi_size > 0 {
            return Ok(Some(ffi_size));
        }
    }

    Ok(None)
}

/// Measure Elixir Hex package size
fn measure_hex_package(name: &str) -> Result<Option<u64>> {
    let build_paths = [
        "packages/elixir/_build/prod/lib/xberg",
        "packages/elixir/_build/dev/lib/xberg",
    ];

    for path in build_paths {
        let dir = Path::new(path);
        if dir.exists() {
            return Ok(Some(dir_size(dir)));
        }
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let hex_paths = [
        format!("{}/.hex/packages/hexpm/xberg", home),
        format!("{}/.mix/archives/xberg", home),
    ];

    for path in hex_paths {
        let dir = Path::new(&path);
        if dir.exists() {
            return Ok(Some(dir_size(dir)));
        }
    }

    if name.starts_with("xberg-elixir") {
        let elixir_dir = Path::new("packages/elixir");
        if elixir_dir.exists() {
            return Ok(Some(dir_size(elixir_dir)));
        }
    }

    Ok(None)
}

/// Measure PHP extension size
fn measure_php_extension(name: &str) -> Result<Option<u64>> {
    if let Ok(output) = Command::new("php")
        .args(["-r", "echo ini_get('extension_dir');"])
        .output()
        && output.status.success()
    {
        let ext_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let ext_path = Path::new(&ext_dir).join("xberg.so");
        if let Ok(metadata) = fs::metadata(&ext_path) {
            return Ok(Some(metadata.len()));
        }
    }

    let workspace_paths = [
        "packages/php-ext/target/release/libxberg_php.so",
        "packages/php-ext/target/release/libxberg_php.dylib",
        "target/release/libxberg_php.so",
        "target/release/libxberg_php.dylib",
    ];

    for path in workspace_paths {
        if let Ok(metadata) = fs::metadata(path) {
            return Ok(Some(metadata.len()));
        }
    }

    if name.starts_with("xberg-php") {
        let php_dir = Path::new("packages/php-ext");
        if php_dir.exists() {
            return Ok(Some(dir_size(php_dir)));
        }
    }

    Ok(None)
}

/// Measure the native FFI library from target/release/.
/// Returns the total size of found native libs, or 0 if none are found.
/// Only counts one platform variant of each library (first match wins).
fn measure_native_ffi_libs() -> u64 {
    let mut total = 0u64;

    for path in [
        "target/release/libxberg_ffi.so",
        "target/release/libxberg_ffi.dylib",
        "target/release/xberg_ffi.dll",
    ] {
        if let Ok(m) = fs::metadata(path) {
            total += m.len();
            break;
        }
    }

    total
}

/// Measure a pip package by asking Python where it is installed.
/// This handles editable installs (maturin develop) where the native .so
/// is in the site-packages directory alongside the Python source files.
fn measure_pip_package_via_python(package: &str) -> Option<u64> {
    let module_name = package.replace('-', "_");
    let script = format!(
        "import {mod_name}, os; print(os.path.dirname({mod_name}.__file__))",
        mod_name = module_name
    );
    let output = Command::new("python3").args(["-c", &script]).output().ok()?;

    if !output.status.success() {
        return None;
    }

    let pkg_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if pkg_dir.is_empty() {
        return None;
    }

    let path = Path::new(&pkg_dir);
    if path.exists() {
        let size = dir_size(path);
        if size > 10_000 {
            return Some(size);
        }
    }

    None
}

/// Check if a directory (or one level of subdirectories) contains native
/// extension files (.so, .bundle, .dylib, .dll, .node).
fn has_native_extension(dir: &Path) -> bool {
    has_native_extension_inner(dir, 0)
}

fn has_native_extension_inner(dir: &Path, depth: u32) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension().and_then(|e| e.to_str())
            && matches!(ext, "so" | "bundle" | "dylib" | "dll" | "node")
        {
            return true;
        } else if path.is_dir() && depth < 2 && has_native_extension_inner(&path, depth + 1) {
            return true;
        }
    }
    false
}

/// Calculate total size of a directory
fn dir_size(path: &Path) -> u64 {
    let mut size = 0;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                size += dir_size(&path);
            } else if let Ok(metadata) = path.metadata() {
                size += metadata.len();
            }
        }
    }

    size
}

/// Load framework sizes from a JSON file
pub fn load_framework_sizes(path: &Path) -> Result<FrameworkSizes> {
    let contents = fs::read_to_string(path).map_err(Error::Io)?;
    serde_json::from_str(&contents).map_err(|e| Error::Benchmark(format!("Invalid JSON: {}", e)))
}

/// Save framework sizes to a JSON file
pub fn save_framework_sizes(sizes: &FrameworkSizes, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(sizes)
        .map_err(|e| Error::Benchmark(format!("JSON serialization failed: {}", e)))?;
    fs::write(path, json).map_err(Error::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_package_name() {
        assert_eq!(extract_package_name("xberg-python"), "xberg");
        assert_eq!(extract_package_name("docling"), "docling");
        assert_eq!(extract_package_name("docling-batch"), "docling");
        assert_eq!(extract_package_name("mineru-batch"), "mineru");
    }

    #[test]
    fn test_frameworks_list_complete() {
        assert_eq!(FRAMEWORKS.len(), 19);

        let names: Vec<&str> = FRAMEWORKS.iter().map(|(n, _, _)| *n).collect();
        assert!(names.contains(&"xberg-rust"));
        assert!(names.contains(&"xberg-python"));
        assert!(names.contains(&"xberg-node"));

        assert!(names.contains(&"docling"));
        assert!(names.contains(&"tika"));
        assert!(names.contains(&"unstructured"));
    }

    #[test]
    fn test_dir_size_empty() {
        let temp = tempfile::TempDir::new().unwrap();
        let size = dir_size(temp.path());
        assert_eq!(size, 0);
    }

    #[test]
    fn test_dir_size_with_files() {
        let temp = tempfile::TempDir::new().unwrap();
        fs::write(temp.path().join("a.txt"), "hello").unwrap();
        fs::write(temp.path().join("b.txt"), "world!").unwrap();

        let size = dir_size(temp.path());
        assert_eq!(size, 11);
    }

    #[test]
    fn test_measure_native_ffi_libs_does_not_panic() {
        let _size = measure_native_ffi_libs();
    }

    #[test]
    fn test_measure_pip_package_via_python_nonexistent() {
        let result = measure_pip_package_via_python("nonexistent_package_xyz_123");
        assert!(result.is_none());
    }

    #[test]
    fn test_has_native_extension_empty_dir() {
        let temp = tempfile::TempDir::new().unwrap();
        assert!(!has_native_extension(temp.path()));
    }

    #[test]
    fn test_has_native_extension_with_so() {
        let temp = tempfile::TempDir::new().unwrap();
        fs::write(temp.path().join("module.so"), "fake").unwrap();
        assert!(has_native_extension(temp.path()));
    }

    #[test]
    fn test_has_native_extension_nested() {
        let temp = tempfile::TempDir::new().unwrap();
        let sub = temp.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("lib.dylib"), "fake").unwrap();
        assert!(has_native_extension(temp.path()));
    }

    #[test]
    fn test_has_native_extension_no_match() {
        let temp = tempfile::TempDir::new().unwrap();
        fs::write(temp.path().join("file.txt"), "text").unwrap();
        fs::write(temp.path().join("lib.py"), "python").unwrap();
        assert!(!has_native_extension(temp.path()));
    }

    #[test]
    fn test_known_third_party_sizes_all_present() {
        let known_names: Vec<&str> = KNOWN_THIRD_PARTY_SIZES.iter().map(|(n, ..)| *n).collect();
        for (name, _, _) in FRAMEWORKS {
            if !name.starts_with("xberg-") {
                assert!(
                    known_names.contains(name),
                    "Third-party framework '{}' missing from KNOWN_THIRD_PARTY_SIZES",
                    name,
                );
            }
        }
    }

    #[test]
    fn test_known_sizes_are_reasonable() {
        for (name, pkg, sys, models, _) in KNOWN_THIRD_PARTY_SIZES {
            let total = pkg + sys + models;
            assert!(total > 0, "Framework '{}' has zero total size", name,);
            assert!(
                total < 10_000_000_000,
                "Framework '{}' total {} bytes seems too large",
                name,
                total,
            );
        }
    }

    #[test]
    fn test_lookup_known_size_found() {
        let size = lookup_known_size("pymupdf4llm").unwrap();
        assert_eq!(size.package_bytes, 51_500_000);
        assert_eq!(size.system_deps_bytes, 0);
        assert_eq!(size.model_bytes, 0);
        assert_eq!(size.size_bytes, 51_500_000);
        assert_eq!(size.method, "known_size");
    }

    #[test]
    fn test_lookup_known_size_not_found() {
        assert!(lookup_known_size("xberg-rust").is_none());
        assert!(lookup_known_size("nonexistent").is_none());
    }

    #[test]
    fn test_docling_includes_models() {
        let size = lookup_known_size("docling").unwrap();
        assert!(size.model_bytes > 0, "docling should have model_bytes > 0");
        assert_eq!(
            size.size_bytes,
            size.package_bytes + size.system_deps_bytes + size.model_bytes
        );
    }
}
