//! ONNX Runtime library auto-discovery and execution provider configuration.
//!
//! Preloads a discovered native runtime for dynamic ONNX consumers without
//! changing the process environment.
//!
//! Also provides `apply_execution_providers` for configuring GPU acceleration
//! on ORT session builders across all subsystems (layout, embeddings, OCR, etc.).

#[cfg(any(test, ort_session_providers))]
const XBERG_ORT_EP_ENV_VAR: &str = "XBERG_ORT_EP";

#[cfg(any(test, ort_session_providers))]
pub(crate) fn parse_execution_provider_override(
    value: &str,
) -> Option<crate::core::config::acceleration::ExecutionProviderType> {
    use crate::core::config::acceleration::ExecutionProviderType;

    match value.trim().to_ascii_lowercase().as_str() {
        "cpu" => Some(ExecutionProviderType::Cpu),
        "coreml" => Some(ExecutionProviderType::CoreMl),
        "cuda" => Some(ExecutionProviderType::Cuda),
        "tensorrt" => Some(ExecutionProviderType::TensorRt),
        "auto" => Some(ExecutionProviderType::Auto),
        _ => None,
    }
}

#[cfg(any(test, ort_session_providers))]
pub(crate) fn execution_provider_override() -> Option<crate::core::config::acceleration::ExecutionProviderType> {
    std::env::var(XBERG_ORT_EP_ENV_VAR)
        .ok()
        .and_then(|value| parse_execution_provider_override(&value))
}

/// Prepare the runtime before callers that may create their own session builders.
#[cfg(any(ort_session_providers, feature = "ner-onnx"))]
pub(crate) fn ensure_ort_available() {
    #[cfg(all(feature = "ort-bundled", not(feature = "ort-dynamic")))]
    tracing::debug!("ONNX Runtime is linked; skipping system library discovery");

    // `ort_environment` returns the same cached failure, so it is only noted here. ~keep
    #[cfg(ort_dynamic_discovery)]
    if let Err(error) = preload_ort_runtime() {
        tracing::debug!(%error, "ONNX Runtime is not loadable");
    }
}

/// Explicit ONNX Runtime library override; the name and its semantics are ort's own.
#[cfg(ort_dynamic_discovery)]
const ORT_DYLIB_PATH_ENV_VAR: &str = "ORT_DYLIB_PATH";

/// Why the dynamically loaded ONNX Runtime library is unavailable.
#[cfg(ort_dynamic_discovery)]
#[derive(Debug)]
pub(crate) enum OrtRuntimeError {
    /// No library exists at any location the search order names.
    NotFound { tried: Vec<std::path::PathBuf> },
    /// The resolved library exists (or was named explicitly) but could not be loaded.
    Load {
        path: std::path::PathBuf,
        source: ort::LoadDynamicError,
    },
}

#[cfg(ort_dynamic_discovery)]
impl std::fmt::Display for OrtRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { tried } => {
                write!(f, "ONNX Runtime library not found; searched:")?;
                for path in tried {
                    write!(f, " `{}`", path.display())?;
                }
                write!(
                    f,
                    ". Reinstall the release archive, which ships the library next to the xberg \
                     executable, or set {ORT_DYLIB_PATH_ENV_VAR} to the absolute path of an ONNX \
                     Runtime 1.{}+ library",
                    ort::MINOR_VERSION
                )
            }
            Self::Load { path, source } => write!(
                f,
                "could not load the ONNX Runtime library `{}`: {source}. Point \
                 {ORT_DYLIB_PATH_ENV_VAR} at a working ONNX Runtime 1.{}+ library",
                path.display(),
                ort::MINOR_VERSION
            ),
        }
    }
}

#[cfg(ort_dynamic_discovery)]
impl std::error::Error for OrtRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotFound { .. } => None,
            Self::Load { source, .. } => Some(source),
        }
    }
}

/// Load ONNX Runtime the first time a model needs it, exactly once per process.
///
/// The outcome, success or failure, is cached: later callers get the same result instead of
/// a second `dlopen`, and a failure never falls through to ort's own name-based loader search.
#[cfg(ort_dynamic_discovery)]
pub(crate) fn preload_ort_runtime() -> Result<(), std::sync::Arc<OrtRuntimeError>> {
    static RUNTIME: std::sync::OnceLock<Result<(), std::sync::Arc<OrtRuntimeError>>> = std::sync::OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            let explicit = std::env::var_os(ORT_DYLIB_PATH_ENV_VAR);
            let executable = running_executable();
            let system: Vec<&std::path::Path> = platform_candidates().iter().map(std::path::Path::new).collect();
            let path = preload_ort_runtime_from(explicit.as_deref(), executable.as_deref(), &system)
                .map_err(std::sync::Arc::new)?;
            tracing::debug!(path = %path.display(), "Loaded ONNX Runtime");
            Ok(())
        })
        .clone()
}

/// The running executable with symlinks resolved, so a symlinked install (a package manager's
/// `bin/` link) still finds the library shipped beside the real binary.
#[cfg(ort_dynamic_discovery)]
fn running_executable() -> Option<std::path::PathBuf> {
    let executable = std::env::current_exe().ok()?;
    // Windows canonical paths take the verbatim `\\?\` form; the loader needs no symlink pass. ~keep
    #[cfg(unix)]
    let executable = std::fs::canonicalize(&executable).unwrap_or(executable);
    Some(executable)
}

/// Resolve the library with [`resolve_ort_library`] and load it; uncached.
#[cfg(ort_dynamic_discovery)]
fn preload_ort_runtime_from(
    explicit: Option<&std::ffi::OsStr>,
    executable: Option<&std::path::Path>,
    system: &[&std::path::Path],
) -> Result<std::path::PathBuf, OrtRuntimeError> {
    let path =
        resolve_ort_library(explicit, executable, system).map_err(|tried| OrtRuntimeError::NotFound { tried })?;
    // Load the library and leave environment creation to the caller. ~keep
    match ort::init_from(&path) {
        Ok(_builder) => Ok(path),
        Err(source) => Err(OrtRuntimeError::Load { path, source }),
    }
}

/// Decide which ONNX Runtime library to load, without loading it.
///
/// 1. `ORT_DYLIB_PATH`, when set and non-empty, is authoritative: it is returned even when it
///    does not exist, so a mistyped override fails loudly instead of loading another runtime.
///    ort resolves a relative value against the executable's directory first.
/// 2. The release layout: [`ORT_LIBRARY_NAMES`] in the executable's directory, then in
///    `<executable dir>/../lib`.
/// 3. The documented system install locations (`platform_candidates`).
///
/// Every candidate is an absolute path, so the current directory and the loader's name search
/// are never consulted. Returns every path searched when none exists.
#[cfg(any(test, ort_dynamic_discovery))]
fn resolve_ort_library(
    explicit: Option<&std::ffi::OsStr>,
    executable: Option<&std::path::Path>,
    system: &[&std::path::Path],
) -> Result<std::path::PathBuf, Vec<std::path::PathBuf>> {
    if let Some(explicit) = explicit.filter(|value| !value.is_empty()) {
        return Ok(std::path::PathBuf::from(explicit));
    }
    let mut tried = executable_relative_candidates(executable);
    tried.extend(system.iter().map(|path| path.to_path_buf()));
    match tried.iter().find(|path| path.is_file()) {
        Some(found) => Ok(found.clone()),
        None => Err(tried),
    }
}

#[cfg(any(test, ort_dynamic_discovery))]
fn executable_relative_candidates(executable: Option<&std::path::Path>) -> Vec<std::path::PathBuf> {
    let Some(directory) = executable.and_then(std::path::Path::parent) else {
        return Vec::new();
    };
    let mut directories = vec![directory.to_path_buf()];
    if let Some(prefix) = directory.parent() {
        directories.push(prefix.join("lib"));
    }
    directories
        .iter()
        .flat_map(|directory| ORT_LIBRARY_NAMES.iter().map(move |name| directory.join(name)))
        .collect()
}

/// File names the runtime ships under, searched in this order within each directory.
#[cfg(all(any(test, ort_dynamic_discovery), target_os = "macos"))]
const ORT_LIBRARY_NAMES: &[&str] = &["libonnxruntime.dylib"];

#[cfg(all(any(test, ort_dynamic_discovery), target_os = "linux"))]
const ORT_LIBRARY_NAMES: &[&str] = &["libonnxruntime.so", "libonnxruntime.so.1"];

#[cfg(all(any(test, ort_dynamic_discovery), target_os = "windows"))]
const ORT_LIBRARY_NAMES: &[&str] = &["onnxruntime.dll"];

#[cfg(all(
    any(test, ort_dynamic_discovery),
    not(any(target_os = "macos", target_os = "linux", target_os = "windows"))
))]
const ORT_LIBRARY_NAMES: &[&str] = &[];

#[cfg(all(ort_dynamic_discovery, target_os = "macos"))]
fn platform_candidates() -> &'static [&'static str] {
    &[
        "/opt/homebrew/lib/libonnxruntime.dylib",
        "/usr/local/lib/libonnxruntime.dylib",
    ]
}

#[cfg(all(ort_dynamic_discovery, target_os = "linux"))]
fn platform_candidates() -> &'static [&'static str] {
    &[
        "/usr/lib/libonnxruntime.so",
        "/usr/local/lib/libonnxruntime.so",
        "/usr/lib/x86_64-linux-gnu/libonnxruntime.so",
        "/usr/lib/aarch64-linux-gnu/libonnxruntime.so",
    ]
}

#[cfg(all(ort_dynamic_discovery, target_os = "windows"))]
fn platform_candidates() -> &'static [&'static str] {
    &[
        "C:\\Program Files\\onnxruntime\\bin\\onnxruntime.dll",
        "C:\\Windows\\System32\\onnxruntime.dll",
    ]
}

#[cfg(all(
    ort_dynamic_discovery,
    not(any(target_os = "macos", target_os = "linux", target_os = "windows"))
))]
fn platform_candidates() -> &'static [&'static str] {
    &[]
}

/// Outcome of deciding how to register a GPU execution provider, given whether it was
/// explicitly requested and whether ONNX Runtime reports (compile-time) support for it.
///
/// `is_available()` on an [`ort::ep::ExecutionProvider`] only reports that ONNX Runtime was
/// *compiled with* the provider — it does not mean the provider can actually load at session
/// creation (e.g. missing CUDA runtime libraries). `RegisterStrict` accounts for that gap: the
/// caller asked for this provider by name, so a registration failure must be reported, not
/// swallowed by `ort`'s default `fail_silently` policy.
#[cfg(any(
    test,
    all(
        ort_session_providers,
        any(target_os = "macos", feature = "cuda", feature = "tensorrt")
    )
))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GpuEpOutcome {
    /// Register the provider and require registration to succeed; a load failure at session
    /// creation must surface as an error, since the caller explicitly asked for this provider.
    RegisterStrict,
    /// Register the provider on a best-effort basis; a load failure falls back to CPU silently
    /// (`ort`'s default policy). Only reachable for the unspecified/`auto` path.
    RegisterBestEffort,
    /// Do not attempt registration: the caller explicitly requested a provider ONNX Runtime was
    /// not compiled to support.
    ErrorUnavailable,
    /// Do not attempt registration: the provider is unavailable and nothing explicit was
    /// requested, so falling back to CPU is the documented, silent `auto` behavior.
    SkipToCpu,
}

// Keep the policy independently testable with simulated provider availability. ~keep
#[cfg(any(
    test,
    all(
        ort_session_providers,
        any(target_os = "macos", feature = "cuda", feature = "tensorrt")
    )
))]
pub(crate) fn decide_gpu_ep_outcome(explicit: bool, is_available: bool) -> GpuEpOutcome {
    match (explicit, is_available) {
        (true, true) => GpuEpOutcome::RegisterStrict,
        (true, false) => GpuEpOutcome::ErrorUnavailable,
        (false, true) => GpuEpOutcome::RegisterBestEffort,
        (false, false) => GpuEpOutcome::SkipToCpu,
    }
}

/// Build the error message for a GPU execution provider that was explicitly requested but is
/// not available in the loaded ONNX Runtime. Names both the provider and a concrete suggestion
/// so the caller can act on it immediately, per the error-context convention (operation,
/// input, root cause, suggestion).
#[cfg(any(
    test,
    all(
        ort_session_providers,
        any(target_os = "macos", feature = "cuda", feature = "tensorrt")
    )
))]
pub(crate) fn unavailable_gpu_ep_error_message(provider_name: &str, suggestion: &str) -> String {
    format!(
        "{provider_name} execution provider requested but not available in the loaded ONNX \
         Runtime. {suggestion}"
    )
}

/// Resolve the execution provider a session build will attempt for `accel`: the `XBERG_ORT_EP`
/// env override first, then `accel`'s configured provider, then
/// [`crate::core::config::acceleration::ExecutionProviderType::Auto`].
/// Mirrors the resolution `apply_execution_providers` performs internally; factored out so
/// callers that need this decision *before* invoking `apply_execution_providers` — e.g. deciding
/// whether a failed session build may retry CPU-only, see `OrtBackend::build_session` in
/// `inference::ort_backend` — do not re-derive it independently and risk it drifting out of sync.
#[cfg(any(test, ort_session_providers))]
pub(crate) fn resolve_execution_provider(
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
) -> crate::core::config::acceleration::ExecutionProviderType {
    use crate::core::config::acceleration::ExecutionProviderType;

    execution_provider_override()
        .unwrap_or_else(|| accel.map(|a| a.provider.clone()).unwrap_or(ExecutionProviderType::Auto))
}

/// Whether `accel` (plus any `XBERG_ORT_EP` override) resolves to an explicitly-requested
/// execution provider, as opposed to
/// [`crate::core::config::acceleration::ExecutionProviderType::Auto`]. `apply_execution_providers`
/// hard-errors on an explicit request it cannot satisfy rather than falling back to CPU — callers
/// that build a session must not swallow that error into a silent CPU retry.
#[cfg(any(test, all(ort_session_providers, inference_ort, any(layout_detection, auto_rotate))))]
pub(crate) fn is_explicit_provider_request(
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
) -> bool {
    resolve_execution_provider(accel) != crate::core::config::acceleration::ExecutionProviderType::Auto
}

/// A session-building failure that stays constructible when ONNX Runtime is not loaded.
///
/// Building an [`ort::Error`] calls `CreateStatus` through the ORT API, which with
/// `load-dynamic` makes ort `dlopen` its default library name and panic when that fails. A
/// runtime that could not be loaded is therefore carried as [`OrtSessionError::Runtime`] and
/// must never be converted into an [`ort::Error`]. [`OrtSessionError::Ort`] only ever holds an
/// error ort produced, which implies the runtime was loaded.
#[cfg(any(ort_session_providers, feature = "ner-onnx"))]
#[derive(Debug, thiserror::Error)]
pub(crate) enum OrtSessionError {
    /// The ONNX Runtime library could not be resolved or loaded.
    #[cfg(ort_dynamic_discovery)]
    #[error(transparent)]
    Runtime(std::sync::Arc<OrtRuntimeError>),
    /// ONNX Runtime itself reported an error.
    #[error(transparent)]
    Ort(#[from] ort::Error),
}

/// The process-wide ONNX Runtime environment every session builder in this crate shares; ort
/// rc.14 takes it as an argument and ONNX Runtime requires it to outlive every session.
/// The `xberg-paddle-ocr` and `xberg-gliner` session builders are handed this same
/// environment when it is created, so call this before they load a model. ~keep
///
/// This is the only way into ort for session builders: it loads the runtime first, so every
/// later ort call and [`ort::Error`] runs against the resolved library.
#[cfg(any(ort_session_providers, feature = "ner-onnx"))]
pub(crate) fn ort_environment() -> Result<&'static ort::environment::Environment, OrtSessionError> {
    #[cfg(not(feature = "sceptre-ocr-ort"))]
    static ENVIRONMENT: std::sync::OnceLock<ort::environment::Environment> = std::sync::OnceLock::new();
    #[cfg(feature = "sceptre-ocr-ort")]
    static ENVIRONMENT: std::sync::OnceLock<&'static ort::environment::Environment> = std::sync::OnceLock::new();
    static INIT: std::sync::Mutex<()> = std::sync::Mutex::new(());
    if let Some(environment) = ENVIRONMENT.get() {
        return Ok(environment);
    }
    // ONNX Runtime allows one environment per process, so a second concurrent `build()` fails
    // rather than duplicating work: serialize the first creation. ~keep
    let _init = INIT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(environment) = ENVIRONMENT.get() {
        return Ok(environment);
    }
    ensure_ort_available();
    // A failed dynamic load must stop here: any ort call would otherwise make ort retry with
    // its own name-based loader search. ~keep
    #[cfg(ort_dynamic_discovery)]
    preload_ort_runtime().map_err(OrtSessionError::Runtime)?;
    #[cfg(not(feature = "sceptre-ocr-ort"))]
    let environment = ort::init().build()?;
    // The runtime is loaded (or linked) by now, so building an `ort::Error` is safe here. ~keep
    #[cfg(feature = "sceptre-ocr-ort")]
    let environment = sceptre::ort_environment()
        .map_err(|error| ort::Error::from(Box::new(error) as Box<dyn std::error::Error + Send + Sync>))?;
    let environment = ENVIRONMENT.get_or_init(|| environment);
    #[cfg(feature = "paddle-ocr-ort")]
    if !xberg_paddle_ocr::adopt_ort_environment(environment) {
        tracing::warn!("PaddleOCR already holds its own ONNX Runtime environment");
    }
    #[cfg(feature = "ner-onnx")]
    if !xberg_gliner::adopt_ort_environment(environment) {
        tracing::warn!("GLiNER already holds its own ONNX Runtime environment");
    }
    Ok(environment)
}

/// The shared environment for a test that runs ONNX Runtime, or `None` after printing why when
/// the runtime cannot be loaded and `XBERG_REQUIRE_TRACT_PARITY` (set by CI's model jobs) does
/// not require it. The load is checked before [`ort_environment`]: building any `ort::Error`
/// calls into the runtime, so on a failed dynamic load ort falls back to a bare-name `dlopen`
/// and panics.
#[cfg(all(test, any(ort_session_providers, feature = "ner-onnx")))]
pub(crate) fn test_ort_environment(test: &str) -> Option<&'static ort::environment::Environment> {
    #[cfg(ort_dynamic_discovery)]
    if let Err(error) = preload_ort_runtime() {
        assert!(
            std::env::var_os("XBERG_REQUIRE_TRACT_PARITY").is_none(),
            "XBERG_REQUIRE_TRACT_PARITY is set but ONNX Runtime cannot be loaded for {test}: {error}"
        );
        eprintln!("SKIP: ONNX Runtime not available ({error}): {test}");
        return None;
    }
    Some(ort_environment().unwrap_or_else(|error| panic!("{test}: ONNX Runtime environment failed: {error}")))
}

/// Apply execution providers to an ORT session builder based on [`AccelerationConfig`].
///
/// Shared by all ORT consumers (layout detection, embeddings, PaddleOCR, doc orientation).
///
/// When a GPU provider is **explicitly requested** (e.g. `cuda`, `tensorrt`) this function
/// returns an error with an actionable message if either (a) ONNX Runtime was not compiled
/// with that provider, or (b) the provider fails to load at session-creation time (e.g. a
/// missing CUDA runtime library) — `is_available()` alone cannot detect the second case, so
/// explicit registrations are additionally marked `error_on_failure()`. When `auto` is used,
/// unavailable or unloadable GPU providers fall back to CPU silently, with an info-level log.
///
/// [`AccelerationConfig`]: crate::core::config::acceleration::AccelerationConfig
#[cfg(ort_session_providers)]
pub(crate) fn apply_execution_providers(
    builder: ort::session::builder::SessionBuilder,
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
) -> Result<ort::session::builder::SessionBuilder, ort::Error> {
    use crate::core::config::acceleration::ExecutionProviderType;

    let provider = resolve_execution_provider(accel);
    #[cfg(any(feature = "cuda", feature = "tensorrt"))]
    let device_id = accel.map(|a| a.device_id).unwrap_or(0);

    let builder = match provider {
        ExecutionProviderType::Cpu => {
            tracing::debug!("ORT session: CPU execution provider (explicit)");
            builder
        }
        #[cfg(target_os = "macos")]
        ExecutionProviderType::CoreMl => register_coreml_strict(builder)?,
        #[cfg(not(target_os = "macos"))]
        ExecutionProviderType::CoreMl => {
            return Err(ort::Error::new(
                "CoreML execution provider requested but this build target is not macOS. \
                 CoreML is only available on macOS.",
            ));
        }
        #[cfg(feature = "cuda")]
        ExecutionProviderType::Cuda => register_cuda_strict(builder, device_id)?,
        #[cfg(not(feature = "cuda"))]
        ExecutionProviderType::Cuda => {
            return Err(ort::Error::new(
                "CUDA execution provider requested but this build was compiled without CUDA \
                 support; rebuild with the `cuda` feature.",
            ));
        }
        #[cfg(feature = "tensorrt")]
        ExecutionProviderType::TensorRt => register_tensorrt_strict(builder, device_id)?,
        #[cfg(not(feature = "tensorrt"))]
        ExecutionProviderType::TensorRt => {
            return Err(ort::Error::new(
                "TensorRT execution provider requested but this build was compiled without \
                 TensorRT support; rebuild with the `tensorrt` feature.",
            ));
        }
        ExecutionProviderType::Auto => register_auto_providers(builder)?,
    };

    Ok(builder)
}

/// Whether a session built with `apply_execution_providers(builder, accel)` (or without it, when
/// `with_eps` is false) runs on the CPU execution provider alone. Mirrors the provider choice in
/// `apply_execution_providers` and `register_auto_providers`: `Auto` registers CoreML on macOS
/// and CUDA on Linux builds with the `cuda` feature, and nothing elsewhere.
#[cfg(all(ort_session_providers, model_download, unix))]
pub(crate) fn session_is_cpu_only(
    accel: Option<&crate::core::config::acceleration::AccelerationConfig>,
    with_eps: bool,
) -> bool {
    use crate::core::config::acceleration::ExecutionProviderType;

    if !with_eps {
        return true;
    }
    match resolve_execution_provider(accel) {
        ExecutionProviderType::Cpu => true,
        ExecutionProviderType::Auto => !cfg!(any(target_os = "macos", all(target_os = "linux", feature = "cuda"))),
        ExecutionProviderType::CoreMl | ExecutionProviderType::Cuda | ExecutionProviderType::TensorRt => false,
    }
}

/// Build the CoreML execution provider, honouring the `XBERG_COREML_FORMAT` and
/// `XBERG_COREML_UNITS` environment overrides. An unrecognised value is warned about and
/// ignored rather than failing session creation. ~keep
#[cfg(ort_session_providers)]
#[cfg(target_os = "macos")]
fn build_coreml_ep() -> ort::ep::CoreML {
    use ort::ep::coreml::{ComputeUnits, ModelFormat};
    let mut ep = ort::ep::CoreML::default();
    if let Ok(fmt) = std::env::var("XBERG_COREML_FORMAT") {
        match fmt.trim().to_ascii_lowercase().as_str() {
            "mlprogram" => ep = ep.with_model_format(ModelFormat::MLProgram),
            "neuralnetwork" | "nn" => ep = ep.with_model_format(ModelFormat::NeuralNetwork),
            other => tracing::warn!(value = other, "ignoring unknown XBERG_COREML_FORMAT"),
        }
    }
    if let Ok(units) = std::env::var("XBERG_COREML_UNITS") {
        match units.trim().to_ascii_lowercase().as_str() {
            "all" => ep = ep.with_compute_units(ComputeUnits::All),
            "cpu_and_ne" => ep = ep.with_compute_units(ComputeUnits::CPUAndNeuralEngine),
            "cpu_and_gpu" => ep = ep.with_compute_units(ComputeUnits::CPUAndGPU),
            "cpu_only" => ep = ep.with_compute_units(ComputeUnits::CPUOnly),
            other => tracing::warn!(value = other, "ignoring unknown XBERG_COREML_UNITS"),
        }
    }
    ep
}

/// Register CoreML as an explicitly requested provider: unavailable or unloadable is an error,
/// never a silent CPU fallback. ~keep
#[cfg(ort_session_providers)]
#[cfg(target_os = "macos")]
fn register_coreml_strict(
    builder: ort::session::builder::SessionBuilder,
) -> Result<ort::session::builder::SessionBuilder, ort::Error> {
    use ort::ep::ExecutionProvider;

    let ep = build_coreml_ep();
    match decide_gpu_ep_outcome(
        true,
        <ort::ep::CoreML as ExecutionProvider>::is_available().unwrap_or(false),
    ) {
        GpuEpOutcome::RegisterStrict => {
            tracing::info!("ORT session: CoreML execution provider available, using GPU");
            builder
                .with_execution_providers([ep.build().error_on_failure()])
                .map_err(|e| ort::Error::new(e.message()))
        }
        _ => Err(ort::Error::new(unavailable_gpu_ep_error_message(
            "CoreML",
            "Set ORT_DYLIB_PATH to an ONNX Runtime build that includes CoreML support.",
        ))),
    }
}

/// Register CUDA as an explicitly requested provider: unavailable or unloadable is an error,
/// never a silent CPU fallback. ~keep
#[cfg(ort_session_providers)]
#[cfg(feature = "cuda")]
fn register_cuda_strict(
    builder: ort::session::builder::SessionBuilder,
    device_id: u32,
) -> Result<ort::session::builder::SessionBuilder, ort::Error> {
    use ort::ep::ExecutionProvider;

    let ep = ort::ep::CUDA::default().with_device_id(device_id as i32);
    match decide_gpu_ep_outcome(
        true,
        <ort::ep::CUDA as ExecutionProvider>::is_available().unwrap_or(false),
    ) {
        GpuEpOutcome::RegisterStrict => {
            tracing::info!(device_id, "ORT session: CUDA execution provider available, using GPU");
            builder
                .with_execution_providers([ep.build().error_on_failure()])
                .map_err(|e| ort::Error::new(e.message()))
        }
        _ => Err(ort::Error::new(unavailable_gpu_ep_error_message(
            "CUDA",
            "Install a CUDA-enabled ONNX Runtime and set ORT_DYLIB_PATH to point at it \
             (see https://github.com/microsoft/onnxruntime/releases).",
        ))),
    }
}

/// Register TensorRT as an explicitly requested provider: unavailable or unloadable is an error,
/// never a silent CPU fallback. ~keep
#[cfg(ort_session_providers)]
#[cfg(feature = "tensorrt")]
fn register_tensorrt_strict(
    builder: ort::session::builder::SessionBuilder,
    device_id: u32,
) -> Result<ort::session::builder::SessionBuilder, ort::Error> {
    use ort::ep::ExecutionProvider;

    let ep = ort::ep::TensorRT::default().with_device_id(device_id as i32);
    match decide_gpu_ep_outcome(
        true,
        <ort::ep::TensorRT as ExecutionProvider>::is_available().unwrap_or(false),
    ) {
        GpuEpOutcome::RegisterStrict => {
            tracing::info!(
                device_id,
                "ORT session: TensorRT execution provider available, using GPU"
            );
            builder
                .with_execution_providers([ep.build().error_on_failure()])
                .map_err(|e| ort::Error::new(e.message()))
        }
        _ => Err(ort::Error::new(unavailable_gpu_ep_error_message(
            "TensorRT",
            "Install a TensorRT-enabled ONNX Runtime and set ORT_DYLIB_PATH to point at it \
             (see https://github.com/microsoft/onnxruntime/releases).",
        ))),
    }
}

/// `auto`: try the platform GPU provider best-effort and fall back to CPU with an info-level log
/// when it is unavailable or fails to load. ~keep
#[cfg(ort_session_providers)]
fn register_auto_providers(
    builder: ort::session::builder::SessionBuilder,
) -> Result<ort::session::builder::SessionBuilder, ort::Error> {
    #[cfg(any(target_os = "macos", all(target_os = "linux", feature = "cuda")))]
    use ort::ep::ExecutionProvider;

    #[cfg(target_os = "macos")]
    let builder = {
        let ep = build_coreml_ep();
        match decide_gpu_ep_outcome(
            false,
            <ort::ep::CoreML as ExecutionProvider>::is_available().unwrap_or(false),
        ) {
            GpuEpOutcome::RegisterBestEffort => {
                tracing::info!("ORT session: auto — CoreML available, using GPU");
                builder
                    .with_execution_providers([ep.build()])
                    .map_err(|e| ort::Error::new(e.message()))?
            }
            _ => {
                tracing::info!("ORT session: auto — CoreML not available, using CPU");
                builder
            }
        }
    };
    #[cfg(all(target_os = "linux", feature = "cuda"))]
    let builder = {
        let ep = ort::ep::CUDA::default();
        match decide_gpu_ep_outcome(
            false,
            <ort::ep::CUDA as ExecutionProvider>::is_available().unwrap_or(false),
        ) {
            GpuEpOutcome::RegisterBestEffort => {
                tracing::info!("ORT session: auto — CUDA available, using GPU");
                builder
                    .with_execution_providers([ep.build()])
                    .map_err(|e| ort::Error::new(e.message()))?
            }
            _ => {
                tracing::info!(
                    "ORT session: auto — CUDA not available, using CPU. \
                     For GPU support, set ORT_DYLIB_PATH to a CUDA-enabled ONNX Runtime."
                );
                builder
            }
        }
    };
    #[cfg(all(target_os = "linux", not(feature = "cuda")))]
    let builder = {
        tracing::debug!("ORT session: auto — using CPU. Rebuild with the `cuda` feature for GPU support.");
        builder
    };
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let builder = {
        tracing::debug!("ORT session: auto — no platform GPU EP, using CPU");
        builder
    };

    Ok(builder)
}

#[cfg(test)]
mod tests {
    use super::{
        GpuEpOutcome, decide_gpu_ep_outcome, is_explicit_provider_request, parse_execution_provider_override,
        resolve_execution_provider, unavailable_gpu_ep_error_message,
    };
    use crate::core::config::acceleration::{AccelerationConfig, ExecutionProviderType};

    // These tests exercise `decide_gpu_ep_outcome` with an injected `is_available` bool rather
    // than a real `ort::ep::ExecutionProvider::is_available()` call. Real EP availability
    // varies by machine (GPU box vs CI runner), so asserting against actual hardware would be
    // flaky in one direction or the other; the decision logic itself has no such dependency.

    #[test]
    fn explicit_request_for_unavailable_provider_errors_instead_of_falling_back() {
        // Fails against the unfixed code: before this change, an explicit GPU request that
        // fails `is_available()` returned an error already (that part was correct) but nothing
        // distinguished "explicit + unavailable" from "explicit + available-but-fails-to-load"
        // — the latter fell back to CPU silently because `error_on_failure()` was never called.
        // This test pins the first half of the decision the fix makes explicit and reusable.
        assert_eq!(decide_gpu_ep_outcome(true, false), GpuEpOutcome::ErrorUnavailable);
    }

    #[test]
    fn explicit_request_for_available_provider_is_registered_strictly() {
        // Fails against the unfixed code: `decide_gpu_ep_outcome` does not exist prior to this
        // fix, and the production arms it now drives called `.build()` without
        // `.error_on_failure()`, so an available-but-unloadable provider fell back to CPU
        // silently instead of surfacing the registration failure. `RegisterStrict` is the
        // outcome that carries `error_on_failure()` into the actual EP dispatch (see the CUDA,
        // TensorRT, and CoreML arms of `apply_execution_providers`).
        assert_eq!(decide_gpu_ep_outcome(true, true), GpuEpOutcome::RegisterStrict);
    }

    #[test]
    fn unspecified_provider_falls_back_to_cpu_silently_regardless_of_availability() {
        // Control: the legitimate `auto` path must never error and must never request strict
        // registration, whether or not the provider happens to be available. Fails against the
        // unfixed code because `decide_gpu_ep_outcome` does not exist; once it exists, this
        // guards against a future change accidentally tightening `auto` to `error_on_failure`.
        assert_eq!(decide_gpu_ep_outcome(false, false), GpuEpOutcome::SkipToCpu);
        assert_eq!(decide_gpu_ep_outcome(false, true), GpuEpOutcome::RegisterBestEffort);
        for is_available in [false, true] {
            let outcome = decide_gpu_ep_outcome(false, is_available);
            assert_ne!(outcome, GpuEpOutcome::ErrorUnavailable, "auto must never error");
            assert_ne!(outcome, GpuEpOutcome::RegisterStrict, "auto must never require success");
        }
    }

    #[test]
    fn unavailable_gpu_ep_error_names_the_provider_and_the_suggestion() {
        // Fails against the unfixed code: `unavailable_gpu_ep_error_message` does not exist
        // prior to this fix. It pins the constraint that the error names both the EP that was
        // requested and why it is unavailable, as used by the CUDA, TensorRT, and CoreML arms.
        let message = unavailable_gpu_ep_error_message("CUDA", "Install a CUDA-enabled ONNX Runtime.");
        assert!(
            message.contains("CUDA execution provider requested"),
            "message: {message:?}"
        );
        assert!(
            message.contains("not available in the loaded ONNX Runtime"),
            "message: {message:?}"
        );
        assert!(
            message.contains("Install a CUDA-enabled ONNX Runtime."),
            "message: {message:?}"
        );
    }

    #[test]
    fn unavailable_gpu_ep_error_distinguishes_providers() {
        let cuda = unavailable_gpu_ep_error_message("CUDA", "suggestion");
        let tensorrt = unavailable_gpu_ep_error_message("TensorRT", "suggestion");
        let coreml = unavailable_gpu_ep_error_message("CoreML", "suggestion");
        assert_ne!(cuda, tensorrt);
        assert_ne!(cuda, coreml);
        assert_ne!(tensorrt, coreml);
    }

    #[test]
    fn blank_execution_provider_overrides_are_absent() {
        for value in ["", " ", "\t\r\n"] {
            assert_eq!(parse_execution_provider_override(value), None, "value: {value:?}");
        }
    }

    #[test]
    fn unrecognized_execution_provider_overrides_are_absent() {
        for value in ["invalid", "gpu", "core-ml", "cpu,cuda"] {
            assert_eq!(parse_execution_provider_override(value), None, "value: {value:?}");
        }
    }

    #[test]
    fn explicit_gpu_providers_resolve_to_explicit_request() {
        for provider in [
            ExecutionProviderType::CoreMl,
            ExecutionProviderType::Cuda,
            ExecutionProviderType::TensorRt,
        ] {
            let accel = AccelerationConfig { provider, device_id: 0 };
            assert_eq!(resolve_execution_provider(Some(&accel)), accel.provider);
            assert!(
                is_explicit_provider_request(Some(&accel)),
                "provider {:?} must resolve as explicit",
                accel.provider
            );
        }
    }

    #[test]
    fn auto_provider_and_absent_config_resolve_to_not_explicit() {
        // Fails against the unfixed code: prior to this fix there was no way to distinguish an
        // auto-detected provider from an explicitly-requested one outside of
        // `apply_execution_providers` itself, which is exactly why `OrtBackend::build_session`
        // retried CPU-only on ANY session-build error, including an explicit request that
        // `apply_execution_providers` deliberately hard-errored on (commit b0444fffdd).
        let auto = AccelerationConfig {
            provider: ExecutionProviderType::Auto,
            device_id: 0,
        };
        assert_eq!(resolve_execution_provider(Some(&auto)), ExecutionProviderType::Auto);
        assert!(!is_explicit_provider_request(Some(&auto)));
        assert_eq!(resolve_execution_provider(None), ExecutionProviderType::Auto);
        assert!(!is_explicit_provider_request(None));
    }

    #[test]
    fn recognized_execution_provider_overrides_are_parsed() {
        for (value, expected) in [
            ("cpu", ExecutionProviderType::Cpu),
            (" COREML ", ExecutionProviderType::CoreMl),
            ("cuda", ExecutionProviderType::Cuda),
            ("TensorRT", ExecutionProviderType::TensorRt),
            ("auto", ExecutionProviderType::Auto),
        ] {
            assert_eq!(
                parse_execution_provider_override(value),
                Some(expected),
                "value: {value:?}"
            );
        }
    }
}

#[cfg(test)]
#[path = "ort_discovery/resolution_tests.rs"]
mod resolution_tests;

#[cfg(all(
    test,
    any(
        feature = "paddle-ocr-ort",
        feature = "ner-onnx",
        all(feature = "sceptre-ocr-ort", feature = "auto-rotate")
    )
))]
mod shared_environment_tests {
    fn field(number: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![(number << 3) | 2, payload.len() as u8];
        out.extend_from_slice(payload);
        out
    }

    fn varint_field(number: u8, value: u8) -> Vec<u8> {
        vec![number << 3, value]
    }

    fn value_info(name: &str) -> Vec<u8> {
        let dim = field(1, &varint_field(1, 1));
        let tensor = [varint_field(1, 1), field(2, &dim)].concat();
        let type_proto = field(1, &tensor);
        [field(1, name.as_bytes()), field(2, &type_proto)].concat()
    }

    /// A one-node `Identity` ONNX model: loads under any ONNX Runtime without model assets.
    pub(super) fn identity_model() -> Vec<u8> {
        let node = [field(1, b"x"), field(2, b"y"), field(4, b"Identity")].concat();
        let graph = [
            field(1, &node),
            field(2, b"g"),
            field(11, &value_info("x")),
            field(12, &value_info("y")),
        ]
        .concat();
        [varint_field(1, 8), field(7, &graph), field(8, &varint_field(2, 13))].concat()
    }

    #[test]
    fn repeated_calls_return_the_same_environment() {
        let first = super::ort_environment().expect("environment");
        let second = super::ort_environment().expect("second call must not rebuild");
        assert!(std::ptr::eq(first, second));
    }

    #[cfg(feature = "paddle-ocr-ort")]
    #[test]
    fn paddle_loads_under_the_shared_environment() {
        use xberg_paddle_ocr::base_net::BaseNet;
        super::ort_environment().expect("environment");
        let mut net = xberg_paddle_ocr::db_net::DbNet::new();
        // A PaddleOCR-created environment would fail here: ort allows one per process.
        net.init_model_from_memory_with_ort_builder(&identity_model(), 1, None)
            .expect("paddle must reuse the shared environment");
        assert!(!xberg_paddle_ocr::adopt_ort_environment(
            super::ort_environment().expect("environment")
        ));
    }

    #[cfg(feature = "ner-onnx")]
    #[test]
    fn gliner_loads_under_the_shared_environment() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tokenizer = dir.path().join("tokenizer.json");
        std::fs::write(
            &tokenizer,
            r#"{"version":"1.0","model":{"type":"WordLevel","vocab":{"[UNK]":0},"unk_token":"[UNK]"}}"#,
        )
        .expect("write tokenizer");
        let model = dir.path().join("model.onnx");
        std::fs::write(&model, identity_model()).expect("write model");
        super::ort_environment().expect("environment");
        let result = xberg_gliner::Gliner::with_runtime(
            xberg_gliner::Parameters::default(),
            xberg_gliner::RuntimeConfig::default(),
            &tokenizer,
            &model,
        );
        // The identity model has no GLiNER inputs, so loading must get past session creation and
        // stop at schema validation; an `Ort` error here means GLiNER built its own environment.
        assert!(
            matches!(result, Err(xberg_gliner::GlinerError::UnexpectedModelSchema { .. })),
            "unexpected result: {:?}",
            result.err()
        );
    }
}

#[cfg(all(test, feature = "sceptre-ocr-ort", feature = "auto-rotate"))]
#[path = "ort_discovery/sceptre_tests.rs"]
mod sceptre_tests;

#[cfg(all(test, feature = "sceptre-ocr-ort", ort_dynamic_discovery))]
#[path = "ort_discovery/dynamic_sceptre_tests.rs"]
mod dynamic_sceptre_tests;

#[cfg(all(
    test,
    ort_dynamic_discovery,
    any(ort_session_providers, feature = "ner-onnx"),
    any(target_os = "linux", target_os = "macos")
))]
#[path = "ort_discovery/unloaded_runtime_tests.rs"]
mod unloaded_runtime_tests;

#[cfg(all(
    test,
    feature = "ort-dynamic",
    any(feature = "sceptre-ocr-ort", feature = "auto-rotate")
))]
#[path = "ort_discovery/runtime_route_tests.rs"]
mod runtime_route_tests;
