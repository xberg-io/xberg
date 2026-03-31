//! Kreuzberg language binding adapters
//!
//! Factory functions for creating adapters for different language bindings and modes:
//! - Python: single, batch
//! - TypeScript/Node: single, batch
//! - Ruby: single, batch
//! - Elixir: single, batch
//! - PHP: single, batch
//! - Go: single, batch
//! - Java: single, batch
//! - C#: single, batch
//! - C: single, batch
//! - WASM: single, batch

use crate::Result;
use crate::adapters::subprocess::SubprocessAdapter;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Shared format lists
// ---------------------------------------------------------------------------

/// Base formats supported by all Kreuzberg bindings (native and WASM).
const BASE_FORMATS: &[&str] = &[
    // Documents
    "pdf",
    "docx",
    "doc",
    "odt",
    "pptx",
    "ppsx",
    "pptm",
    "ppt",
    "pages",
    "key",
    "xlsx",
    "xlsm",
    "xlsb",
    "xlam",
    "xla",
    "xls",
    "ods",
    "numbers",
    // Text formats
    "txt",
    "md",
    "markdown",
    "commonmark",
    "html",
    "htm",
    "xml",
    "rtf",
    "rst",
    "org",
    // Data formats
    "json",
    "yaml",
    "yml",
    "toml",
    "csv",
    "tsv",
    // Email
    "eml",
    "msg",
    // Archives
    "zip",
    "tar",
    "gz",
    "tgz",
    "7z",
    // Images (OCR supported)
    "bmp",
    "gif",
    "jpg",
    "jpeg",
    "png",
    "tiff",
    "tif",
    "webp",
    "jp2",
    "jpx",
    "jpm",
    "mj2",
    // Academic/Publishing
    "epub",
    "bib",
    "ipynb",
    "tex",
    "latex",
    "typst",
    "typ",
    "fb2",
    // Other
    "svg",
    "djot",
];

/// Extra formats only available in native (non-WASM) builds.
const NATIVE_EXTRA_FORMATS: &[&str] = &["xltm"];

fn formats_to_vec(base: &[&str], extra: &[&str]) -> Vec<String> {
    base.iter().chain(extra.iter()).map(|s| (*s).to_string()).collect()
}

/// Get supported formats for native Kreuzberg bindings.
fn get_kreuzberg_supported_formats() -> Vec<String> {
    formats_to_vec(BASE_FORMATS, NATIVE_EXTRA_FORMATS)
}

/// Get supported formats for Kreuzberg WASM bindings.
fn get_kreuzberg_wasm_supported_formats() -> Vec<String> {
    formats_to_vec(BASE_FORMATS, &[])
}

// ---------------------------------------------------------------------------
// Shared helpers (kept as-is)
// ---------------------------------------------------------------------------

/// Convert boolean OCR flag to command-line argument string
fn ocr_flag(ocr_enabled: bool) -> String {
    if ocr_enabled {
        "--ocr".to_string()
    } else {
        "--no-ocr".to_string()
    }
}

/// Get the path to a script in the scripts directory
fn get_script_path(script_name: &str) -> Result<PathBuf> {
    // Try CARGO_MANIFEST_DIR first (development builds)
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let script_path = PathBuf::from(manifest_dir).join("scripts").join(script_name);
        if script_path.exists() {
            return Ok(script_path);
        }
    }

    // Try relative path from current directory (common case)
    let script_path = PathBuf::from("tools/benchmark-harness/scripts").join(script_name);
    if script_path.exists() {
        return Ok(script_path);
    }

    // Try using workspace_root() for absolute path resolution (CI/production builds)
    if let Ok(root) = workspace_root() {
        let script_path = root.join("tools/benchmark-harness/scripts").join(script_name);
        if script_path.exists() {
            return Ok(script_path);
        }
    }

    Err(crate::Error::Config(format!("Script not found: {}", script_name)))
}

/// Helper to find Python interpreter (prefers uv)
fn find_python() -> Result<(PathBuf, Vec<String>)> {
    if which::which("uv").is_ok() {
        Ok((PathBuf::from("uv"), vec!["run".to_string(), "python".to_string()]))
    } else if which::which("python3").is_ok() {
        Ok((PathBuf::from("python3"), vec![]))
    } else {
        Err(crate::Error::Config("Python not found".to_string()))
    }
}

/// Helper to find Node/TypeScript interpreter (tsx)
///
/// Prefers `pnpm exec tsx` because it correctly resolves pnpm workspace-linked
/// packages (e.g. `@kreuzberg/wasm`). Bun cannot resolve these workspace deps.
fn find_node() -> Result<(PathBuf, Vec<String>)> {
    if which::which("pnpm").is_ok() {
        return Ok((PathBuf::from("pnpm"), vec!["exec".to_string(), "tsx".to_string()]));
    }

    if which::which("tsx").is_ok() {
        return Ok((PathBuf::from("tsx"), vec![]));
    }

    if which::which("ts-node").is_ok() {
        return Ok((PathBuf::from("ts-node"), vec![]));
    }

    Err(crate::Error::Config(
        "TypeScript runtime (tsx or ts-node) not found – ensure pnpm install has run".to_string(),
    ))
}

/// Helper to find Ruby interpreter
fn find_ruby() -> Result<(PathBuf, Vec<String>)> {
    if which::which("ruby").is_ok() {
        Ok((PathBuf::from("ruby"), vec![]))
    } else {
        Err(crate::Error::Config("Ruby not found".to_string()))
    }
}

/// Find a tool by name in PATH, returning a descriptive error if not found.
fn find_tool(name: &str) -> Result<std::path::PathBuf> {
    which::which(name).map_err(|_| crate::Error::Benchmark(format!("{} not found in PATH", name)))
}

/// Helper to find PHP interpreter
fn find_php() -> Result<(PathBuf, Vec<String>)> {
    if which::which("php").is_ok() {
        let mut args = Vec::new();
        // Load kreuzberg PHP extension if ini file exists
        if let Ok(root) = workspace_root() {
            let ini_path = root.join("php-kreuzberg.ini");
            if ini_path.exists() {
                args.push("-c".to_string());
                args.push(ini_path.to_string_lossy().to_string());
            }
        }
        Ok((PathBuf::from("php"), args))
    } else {
        Err(crate::Error::Config("PHP not found".to_string()))
    }
}

/// Helper to find Ruby gem installation directory
///
/// Attempts to locate the kreuzberg gem's lib directory by:
/// 1. Checking workspace packages/ruby/lib directory
/// 2. Checking installed gem location via `gem which kreuzberg`
fn get_ruby_gem_lib_path() -> Result<PathBuf> {
    let workspace_root = workspace_root()?;
    let workspace_gem_lib = workspace_root.join("packages/ruby/lib");
    if workspace_gem_lib.exists() {
        return Ok(workspace_gem_lib);
    }

    use std::process::Command;
    if let Ok(output) = Command::new("ruby")
        .arg("-e")
        .arg("puts Gem.loaded_specs['kreuzberg_rb']&.lib_dirs&.first || ''")
        .output()
        && output.status.success()
    {
        let gem_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !gem_path.is_empty() {
            return Ok(PathBuf::from(gem_path));
        }
    }

    Err(crate::Error::Config(
        "Could not find kreuzberg gem lib directory. Install the gem or use workspace build.".to_string(),
    ))
}

/// Build Java classpath including compiled classes and dependency JARs.
///
/// Returns a colon-separated (Unix) or semicolon-separated (Windows) classpath string
/// containing `target/classes` and all JARs in `target/dependency/`.
fn build_java_classpath() -> Result<String> {
    let root = workspace_root()?;
    let classes_dir = root.join("packages/java/target/classes");
    if !classes_dir.exists() {
        return Err(crate::Error::Config(format!(
            "Java classes not found at {} – run `task java:build:bindings` first",
            classes_dir.display()
        )));
    }

    let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
    let mut parts = vec![classes_dir.to_string_lossy().to_string()];

    let dep_dir = root.join("packages/java/target/dependency");
    if dep_dir.exists()
        && let Ok(entries) = std::fs::read_dir(&dep_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "jar") {
                parts.push(path.to_string_lossy().to_string());
            }
        }
    }

    Ok(parts.join(sep))
}

fn workspace_root() -> Result<PathBuf> {
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let path = PathBuf::from(manifest_dir);
        if let Some(root) = path.parent().and_then(|p| p.parent())
            && root.exists()
        {
            return Ok(root.to_path_buf());
        }
    }

    if let Ok(exe_path) = std::env::current_exe()
        && let Some(parent) = exe_path.parent()
        && let Some(target_dir) = parent.parent()
        && target_dir.file_name().is_some_and(|n| n == "target")
        && let Some(workspace_dir) = target_dir.parent()
        && workspace_dir.exists()
    {
        return Ok(workspace_dir.to_path_buf());
    }

    let cwd = std::env::current_dir()?;
    if cwd.join("target/release").exists() || cwd.join("target/debug").exists() {
        return Ok(cwd);
    }

    Err(crate::Error::Config(
        "Unable to resolve workspace root. Set CARGO_MANIFEST_DIR or run from workspace root.".to_string(),
    ))
}

fn native_library_dir() -> Result<PathBuf> {
    let root = workspace_root()?;
    let release = root.join("target/release");
    if release.exists() {
        return Ok(release);
    }
    let debug = root.join("target/debug");
    if debug.exists() {
        return Ok(debug);
    }
    Err(crate::Error::Config(
        "Native library directory not found in target/".to_string(),
    ))
}

fn prepend_env(var: &str, value: &str, separator: &str) -> String {
    match env::var(var) {
        Ok(existing) if !existing.is_empty() => format!("{value}{separator}{existing}"),
        _ => value.to_string(),
    }
}

fn build_library_env() -> Result<Vec<(String, String)>> {
    let lib_dir = native_library_dir()?;
    let lib_str = lib_dir.to_string_lossy().to_string();
    let mut envs = vec![
        (
            "LD_LIBRARY_PATH".to_string(),
            prepend_env("LD_LIBRARY_PATH", &lib_str, ":"),
        ),
        (
            "DYLD_LIBRARY_PATH".to_string(),
            prepend_env("DYLD_LIBRARY_PATH", &lib_str, ":"),
        ),
        ("CGO_ENABLED".to_string(), "1".to_string()),
    ];
    if cfg!(target_os = "windows") {
        envs.push(("PATH".to_string(), prepend_env("PATH", &lib_str, ";")));
    }
    Ok(envs)
}

// ---------------------------------------------------------------------------
// Data-driven adapter factory
// ---------------------------------------------------------------------------

/// Function pointer returning environment variable pairs.
type EnvFn = fn() -> Result<Vec<(String, String)>>;

/// Specification for a script-based language adapter.
///
/// Each language binding defines one of these to capture the differences;
/// the shared `create_from_spec` function handles the common wiring.
struct AdapterSpec {
    /// Framework name used in benchmark results (e.g. "kreuzberg-python").
    name: &'static str,
    /// Script filename under `tools/benchmark-harness/scripts/`.
    script: &'static str,
    /// How to locate the runtime binary and its base arguments.
    find_runtime: fn() -> Result<(PathBuf, Vec<String>)>,
    /// Extra environment variables beyond the defaults.
    extra_env: Option<EnvFn>,
    /// Post-creation hook for adapter-specific tweaks (e.g. set_working_dir).
    extra_setup: Option<fn(&mut SubprocessAdapter) -> Result<()>>,
    /// Override supported formats (defaults to `get_kreuzberg_supported_formats`).
    supported_formats: Option<fn() -> Vec<String>>,
    /// Extra args inserted *before* the script path (e.g. Ruby `-I` flag).
    pre_script_args: Option<fn() -> Result<Vec<String>>>,
    /// Override max timeout (e.g. WASM needs longer).
    max_timeout: Option<Duration>,
}

/// Create a `SubprocessAdapter` from a spec, for either server or batch mode.
fn create_from_spec(spec: &AdapterSpec, ocr_enabled: bool, batch: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path(spec.script)?;
    let (command, mut args) = (spec.find_runtime)()?;

    if let Some(pre_args_fn) = spec.pre_script_args {
        args.extend(pre_args_fn()?);
    }

    args.push(script_path.to_string_lossy().to_string());
    args.push(ocr_flag(ocr_enabled));
    args.push(if batch { "batch" } else { "server" }.to_string());

    let env = if let Some(env_fn) = spec.extra_env {
        env_fn()?
    } else {
        vec![]
    };

    let formats = if let Some(fmt_fn) = spec.supported_formats {
        fmt_fn()
    } else {
        get_kreuzberg_supported_formats()
    };

    let mut adapter = if batch {
        SubprocessAdapter::with_batch_support(spec.name, command, args, env, formats)
    } else {
        SubprocessAdapter::with_persistent_mode(spec.name, command, args, env, formats)
    };

    if let Some(timeout) = spec.max_timeout {
        adapter = adapter.with_max_timeout(timeout);
    }

    if let Some(setup_fn) = spec.extra_setup {
        setup_fn(&mut adapter)?;
    }

    Ok(adapter)
}

// ---------------------------------------------------------------------------
// Language adapter specs
// ---------------------------------------------------------------------------

fn python_spec() -> AdapterSpec {
    AdapterSpec {
        name: "kreuzberg-python",
        script: "kreuzberg_extract.py",
        find_runtime: find_python,
        extra_env: None,
        extra_setup: None,
        supported_formats: None,
        pre_script_args: None,
        max_timeout: None,
    }
}

fn node_spec() -> AdapterSpec {
    AdapterSpec {
        name: "kreuzberg-node",
        script: "kreuzberg_extract.ts",
        find_runtime: find_node,
        extra_env: None,
        extra_setup: None,
        supported_formats: None,
        pre_script_args: None,
        max_timeout: None,
    }
}

fn wasm_spec() -> AdapterSpec {
    AdapterSpec {
        name: "kreuzberg-wasm",
        script: "kreuzberg_extract_wasm.ts",
        find_runtime: find_node,
        extra_env: None,
        extra_setup: None,
        supported_formats: Some(get_kreuzberg_wasm_supported_formats),
        pre_script_args: None,
        // WASM execution is significantly slower than native — use a higher timeout
        // to avoid restart loops that waste the entire CI budget
        max_timeout: Some(Duration::from_secs(600)),
    }
}

fn ruby_spec() -> AdapterSpec {
    AdapterSpec {
        name: "kreuzberg-ruby",
        script: "kreuzberg_extract.rb",
        find_runtime: find_ruby,
        extra_env: Some(build_library_env),
        extra_setup: None,
        supported_formats: None,
        pre_script_args: Some(ruby_pre_script_args),
        max_timeout: None,
    }
}

fn ruby_pre_script_args() -> Result<Vec<String>> {
    if let Ok(gem_lib_path) = get_ruby_gem_lib_path() {
        Ok(vec!["-I".to_string(), gem_lib_path.to_string_lossy().to_string()])
    } else {
        Ok(vec![])
    }
}

fn r_spec() -> AdapterSpec {
    AdapterSpec {
        name: "kreuzberg-r",
        script: "kreuzberg_extract.R",
        find_runtime: find_r,
        extra_env: Some(build_library_env),
        extra_setup: None,
        supported_formats: None,
        pre_script_args: None,
        max_timeout: None,
    }
}

fn find_r() -> Result<(PathBuf, Vec<String>)> {
    Ok((find_tool("Rscript")?, vec![]))
}

fn php_spec() -> AdapterSpec {
    AdapterSpec {
        name: "kreuzberg-php",
        script: "kreuzberg_extract.php",
        find_runtime: find_php,
        extra_env: Some(build_library_env),
        extra_setup: None,
        supported_formats: None,
        pre_script_args: None,
        max_timeout: None,
    }
}

fn elixir_spec() -> AdapterSpec {
    AdapterSpec {
        name: "kreuzberg-elixir",
        script: "kreuzberg_extract.exs",
        find_runtime: find_elixir,
        extra_env: Some(build_elixir_env),
        extra_setup: None,
        supported_formats: None,
        pre_script_args: None,
        max_timeout: None,
    }
}

fn find_elixir() -> Result<(PathBuf, Vec<String>)> {
    Ok((find_tool("elixir")?, vec![]))
}

fn build_elixir_env() -> Result<Vec<(String, String)>> {
    let mut env = build_library_env()?;

    // Ensure the Erlang VM uses UTF-8 for filenames and string encoding.
    // Without this, Rust NIFs returning UTF-8 strings corrupt when the VM
    // interprets them as latin1.
    env.push(("ELIXIR_ERL_OPTIONS".to_string(), "+fnu".to_string()));
    env.push(("LC_ALL".to_string(), "C.UTF-8".to_string()));
    env.push(("LANG".to_string(), "C.UTF-8".to_string()));

    // Add Elixir package path for the compiled kreuzberg package
    let elixir_pkg_path = workspace_root()?.join("packages/elixir");
    if elixir_pkg_path.exists() {
        env.push(("MIX_EXTS".to_string(), elixir_pkg_path.to_string_lossy().to_string()));

        // Set ERL_LIBS to include _build/prod or _build/dev lib path.
        // Verify the .app manifest exists to avoid using incomplete builds.
        let dev_path = elixir_pkg_path.join("_build/dev/lib");
        let prod_path = elixir_pkg_path.join("_build/prod/lib");

        let erl_libs = if prod_path.join("kreuzberg/ebin/kreuzberg.app").exists() {
            prod_path.to_string_lossy().to_string()
        } else if dev_path.join("kreuzberg/ebin/kreuzberg.app").exists() {
            dev_path.to_string_lossy().to_string()
        } else {
            String::new()
        };

        if !erl_libs.is_empty() {
            env.push(("ERL_LIBS".to_string(), prepend_env("ERL_LIBS", &erl_libs, ":")));
        }
    }

    Ok(env)
}

// ---------------------------------------------------------------------------
// Public factory functions — script-based languages via specs
// ---------------------------------------------------------------------------

/// Create Python adapter (persistent server mode)
pub fn create_python_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&python_spec(), ocr_enabled, false)
}

/// Create Python batch adapter (batch_extract_file)
pub fn create_python_batch_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&python_spec(), ocr_enabled, true)
}

/// Create Node adapter (persistent server mode)
pub fn create_node_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&node_spec(), ocr_enabled, false)
}

/// Create Node batch adapter (batchExtractFile)
pub fn create_node_batch_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&node_spec(), ocr_enabled, true)
}

/// Create WASM adapter (persistent server mode)
pub fn create_wasm_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&wasm_spec(), ocr_enabled, false)
}

/// Create WASM batch adapter (Promise.all extractFile via @kreuzberg/wasm)
pub fn create_wasm_batch_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&wasm_spec(), ocr_enabled, true)
}

/// Create Ruby adapter (persistent server mode)
pub fn create_ruby_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&ruby_spec(), ocr_enabled, false)
}

/// Create Ruby batch adapter (batch_extract_file)
pub fn create_ruby_batch_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&ruby_spec(), ocr_enabled, true)
}

/// Create R adapter (persistent server mode)
pub fn create_r_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&r_spec(), ocr_enabled, false)
}

/// Create R batch adapter (batch_extract_files_sync)
pub fn create_r_batch_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&r_spec(), ocr_enabled, true)
}

/// Create PHP adapter (persistent server mode)
pub fn create_php_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&php_spec(), ocr_enabled, false)
}

/// Create PHP batch adapter (batch_extract_files)
pub fn create_php_batch_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&php_spec(), ocr_enabled, true)
}

/// Create Elixir adapter (persistent server mode)
pub fn create_elixir_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&elixir_spec(), ocr_enabled, false)
}

/// Create Elixir batch adapter (batch_extract_files)
pub fn create_elixir_batch_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_from_spec(&elixir_spec(), ocr_enabled, true)
}

// ---------------------------------------------------------------------------
// Go adapter — uses spec pattern but needs set_working_dir and custom args
// ---------------------------------------------------------------------------

/// Create Go adapter (persistent server mode)
pub fn create_go_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_go_impl(ocr_enabled, false)
}

/// Create Go batch adapter
pub fn create_go_batch_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_go_impl(ocr_enabled, true)
}

fn create_go_impl(ocr_enabled: bool, batch: bool) -> Result<SubprocessAdapter> {
    let script_path = get_script_path("kreuzberg_extract_go.go")?;
    let scripts_dir = script_path
        .parent()
        .ok_or_else(|| crate::Error::Config("Unable to determine scripts directory".to_string()))?
        .to_path_buf();
    let command = find_tool("go")?;
    let mode = if batch { "batch" } else { "server" };
    let args = vec![
        "run".to_string(),
        "-tags".to_string(),
        "kreuzberg_dev".to_string(),
        "kreuzberg_extract_go.go".to_string(),
        ocr_flag(ocr_enabled),
        mode.to_string(),
    ];
    let mut env = build_library_env()?;
    if env::var("KREUZBERG_BENCHMARK_DEBUG").is_ok() {
        env.push(("KREUZBERG_BENCHMARK_DEBUG".to_string(), "true".to_string()));
    }
    let supported_formats = get_kreuzberg_supported_formats();
    let mut adapter = if batch {
        SubprocessAdapter::with_batch_support("kreuzberg-go", command, args, env, supported_formats)
    } else {
        SubprocessAdapter::with_persistent_mode("kreuzberg-go", command, args, env, supported_formats)
    };
    adapter.set_working_dir(scripts_dir);
    Ok(adapter)
}

// ---------------------------------------------------------------------------
// Java adapter — custom classpath and JVM flags
// ---------------------------------------------------------------------------

/// Create Java adapter (persistent server mode)
///
/// Uses persistent mode to keep the JVM alive, avoiding per-file JVM startup overhead.
pub fn create_java_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_java_impl(ocr_enabled, false)
}

/// Create Java batch adapter
pub fn create_java_batch_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_java_impl(ocr_enabled, true)
}

fn create_java_impl(ocr_enabled: bool, batch: bool) -> Result<SubprocessAdapter> {
    let _script_path = get_script_path("KreuzbergExtractJava.java")?;
    let command = find_tool("java")?;
    let classpath = build_java_classpath()?;
    let lib_dir = native_library_dir()?;
    let lib_dir_str = lib_dir.to_string_lossy().to_string();
    let mut env = build_library_env()?;
    env.push(("KREUZBERG_FFI_DIR".to_string(), lib_dir_str.clone()));
    let mode = if batch { "batch" } else { "server" };
    let args = vec![
        "--enable-native-access=ALL-UNNAMED".to_string(),
        format!("-Djava.library.path={}", lib_dir.display()),
        "--class-path".to_string(),
        classpath,
        "KreuzbergExtractJava".to_string(),
        ocr_flag(ocr_enabled),
        mode.to_string(),
    ];
    let supported_formats = get_kreuzberg_supported_formats();
    Ok(if batch {
        SubprocessAdapter::with_batch_support("kreuzberg-java", command, args, env, supported_formats)
    } else {
        SubprocessAdapter::with_persistent_mode("kreuzberg-java", command, args, env, supported_formats)
    })
}

// ---------------------------------------------------------------------------
// C# adapter — dotnet run --project pattern
// ---------------------------------------------------------------------------

/// Create C# adapter (persistent server mode)
pub fn create_csharp_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_csharp_impl(ocr_enabled, false)
}

/// Create C# batch adapter
pub fn create_csharp_batch_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_csharp_impl(ocr_enabled, true)
}

fn create_csharp_impl(ocr_enabled: bool, batch: bool) -> Result<SubprocessAdapter> {
    let command = find_tool("dotnet")?;
    let project = workspace_root()?.join("packages/csharp/Benchmark/Benchmark.csproj");
    if !project.exists() {
        return Err(crate::Error::Config(format!(
            "C# benchmark project missing at {}",
            project.display()
        )));
    }
    let mode = if batch { "batch" } else { "server" };
    let args = vec![
        "run".to_string(),
        "--project".to_string(),
        project.to_string_lossy().to_string(),
        "--".to_string(),
        ocr_flag(ocr_enabled),
        mode.to_string(),
    ];
    let lib_dir = native_library_dir()?;
    let mut env = build_library_env()?;
    env.push(("KREUZBERG_FFI_DIR".to_string(), lib_dir.to_string_lossy().to_string()));
    let supported_formats = get_kreuzberg_supported_formats();
    Ok(if batch {
        SubprocessAdapter::with_batch_support("kreuzberg-csharp", command, args, env, supported_formats)
    } else {
        SubprocessAdapter::with_persistent_mode("kreuzberg-csharp", command, args, env, supported_formats)
    })
}

// ---------------------------------------------------------------------------
// Rust adapters — use binary path, not script path
// ---------------------------------------------------------------------------

/// Find the kreuzberg-extract binary
fn find_kreuzberg_extract_binary() -> Result<PathBuf> {
    // Check in target/release first
    if let Ok(root) = workspace_root() {
        let release_path = root.join("target/release/kreuzberg-extract");
        if release_path.exists() {
            return Ok(release_path);
        }
        let debug_path = root.join("target/debug/kreuzberg-extract");
        if debug_path.exists() {
            return Ok(debug_path);
        }
    }

    // Try which
    if let Ok(path) = which::which("kreuzberg-extract") {
        return Ok(path);
    }

    Err(crate::Error::Config(
        "kreuzberg-extract binary not found. Build with: cargo build -p benchmark-harness --bin kreuzberg-extract"
            .to_string(),
    ))
}

/// Create Rust subprocess adapter (persistent server mode)
///
/// Runs kreuzberg extraction in a subprocess for fair timing comparisons.
/// Uses the `kreuzberg-extract` binary built from the benchmark harness crate.
pub fn create_rust_subprocess_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let binary_path = find_kreuzberg_extract_binary()?;

    let args = vec![ocr_flag(ocr_enabled)];

    let supported_formats = get_kreuzberg_supported_formats();
    Ok(SubprocessAdapter::with_persistent_mode(
        "kreuzberg-rust",
        binary_path,
        args,
        vec![],
        supported_formats,
    ))
}

/// Create Rust subprocess adapter with PaddleOCR backend (persistent server mode)
///
/// Same as `create_rust_subprocess_adapter` but uses PaddleOCR instead of Tesseract.
/// Registered as framework name `kreuzberg-rust-paddle` for separate aggregation.
pub fn create_rust_paddle_subprocess_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let binary_path = find_kreuzberg_extract_binary()?;

    let mut args = vec![ocr_flag(ocr_enabled)];
    if ocr_enabled {
        args.push("--ocr-backend".to_string());
        args.push("paddle-ocr".to_string());
    }

    let supported_formats = get_kreuzberg_supported_formats();
    Ok(SubprocessAdapter::with_persistent_mode(
        "kreuzberg-rust-paddle",
        binary_path,
        args,
        vec![],
        supported_formats,
    ))
}

/// Create Rust batch adapter (batch_extract_file_sync via subprocess)
///
/// Uses the `kreuzberg-extract` binary with `batch` subcommand for fair
/// subprocess overhead comparison with other language batch adapters.
pub fn create_rust_batch_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    let binary_path = find_kreuzberg_extract_binary()?;

    let mut args = vec![ocr_flag(ocr_enabled)];
    args.push("batch".to_string());

    let supported_formats = get_kreuzberg_supported_formats();
    Ok(SubprocessAdapter::with_batch_support(
        "kreuzberg-rust",
        binary_path,
        args,
        vec![],
        supported_formats,
    ))
}

// ---------------------------------------------------------------------------
// C adapter — compiles from source, very different pattern
// ---------------------------------------------------------------------------

/// Find a C compiler (cc, gcc, or clang)
fn find_c_compiler() -> Result<PathBuf> {
    for name in &["cc", "gcc", "clang"] {
        if let Ok(path) = which::which(name) {
            return Ok(path);
        }
    }
    Err(crate::Error::Config(
        "C compiler not found (tried cc, gcc, clang)".to_string(),
    ))
}

/// Compile the C extraction binary from source.
///
/// The binary is placed alongside the native library in target/release or target/debug.
/// Compilation is skipped if the binary is newer than the source file.
fn compile_c_extraction_binary(source: &Path) -> Result<PathBuf> {
    let lib_dir = native_library_dir()?;
    let binary_path = lib_dir.join("kreuzberg_extract_c");

    // Skip compilation if binary exists and is newer than source
    if binary_path.exists()
        && let (Ok(src_meta), Ok(bin_meta)) = (std::fs::metadata(source), std::fs::metadata(&binary_path))
        && let (Ok(src_time), Ok(bin_time)) = (src_meta.modified(), bin_meta.modified())
        && bin_time >= src_time
    {
        eprintln!(
            "[adapter] kreuzberg-c: using cached binary at {}",
            binary_path.display()
        );
        return Ok(binary_path);
    }

    let compiler = find_c_compiler()?;
    let header_dir = workspace_root()?.join("crates/kreuzberg-ffi");
    let lib_dir_str = lib_dir.to_string_lossy().to_string();

    eprintln!(
        "[adapter] kreuzberg-c: compiling {} -> {}",
        source.display(),
        binary_path.display()
    );

    let mut cmd = std::process::Command::new(&compiler);
    cmd.arg("-O2")
        .arg("-o")
        .arg(&binary_path)
        .arg(source)
        .arg(format!("-I{}", header_dir.display()))
        .arg(format!("-L{}", lib_dir_str))
        .arg("-lkreuzberg_ffi")
        .arg("-lpthread")
        .arg("-lm");

    // Platform-specific linker flags
    if cfg!(target_os = "macos") {
        cmd.arg("-framework").arg("CoreFoundation");
        cmd.arg("-framework").arg("Security");
        cmd.arg("-framework").arg("SystemConfiguration");
    } else if cfg!(target_os = "linux") {
        cmd.arg("-ldl");
    }

    // Set library path so the compiler can find the shared library
    if cfg!(target_os = "macos") {
        cmd.env("DYLD_LIBRARY_PATH", prepend_env("DYLD_LIBRARY_PATH", &lib_dir_str, ":"));
    } else {
        cmd.env("LD_LIBRARY_PATH", prepend_env("LD_LIBRARY_PATH", &lib_dir_str, ":"));
    }

    let output = cmd
        .output()
        .map_err(|e| crate::Error::Config(format!("Failed to run C compiler: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::Error::Config(format!(
            "C compilation failed ({}): {}",
            output.status, stderr
        )));
    }

    Ok(binary_path)
}

/// Create C adapter (persistent server mode)
pub fn create_c_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_c_impl(ocr_enabled, false)
}

/// Create C batch adapter
pub fn create_c_batch_adapter(ocr_enabled: bool) -> Result<SubprocessAdapter> {
    create_c_impl(ocr_enabled, true)
}

fn create_c_impl(ocr_enabled: bool, batch: bool) -> Result<SubprocessAdapter> {
    let source_path = get_script_path("c/kreuzberg_extract_c.c")?;
    let binary_path = compile_c_extraction_binary(&source_path)?;

    let mode = if batch { "batch" } else { "server" };
    let args = vec![ocr_flag(ocr_enabled), mode.to_string()];

    let mut env = build_library_env()?;
    if env::var("KREUZBERG_BENCHMARK_DEBUG").is_ok() {
        env.push(("KREUZBERG_BENCHMARK_DEBUG".to_string(), "true".to_string()));
    }

    let supported_formats = get_kreuzberg_supported_formats();
    Ok(if batch {
        SubprocessAdapter::with_batch_support("kreuzberg-c", binary_path, args, env, supported_formats)
    } else {
        SubprocessAdapter::with_persistent_mode("kreuzberg-c", binary_path, args, env, supported_formats)
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_script_path() {
        let result = get_script_path("kreuzberg_extract.py");
        if let Ok(path) = result {
            assert!(path.exists());
        }
    }

    #[test]
    fn test_find_python() {
        let result = find_python();
        assert!(result.is_ok() || which::which("python3").is_err());
    }

    #[test]
    fn test_find_node() {
        let result = find_node();
        if let Ok((cmd, _args)) = result {
            assert!(!cmd.as_os_str().is_empty());
        } else {
            assert!(which::which("tsx").is_err());
            assert!(which::which("ts-node").is_err());
            assert!(which::which("pnpm").is_err());
        }
    }

    #[test]
    fn test_find_tool_nonexistent() {
        let result = find_tool("definitely_not_a_real_tool_12345");
        assert!(result.is_err());
    }
}
