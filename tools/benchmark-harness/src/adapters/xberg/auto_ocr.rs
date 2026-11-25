//! Auto-OCR xberg variants: an OCR engine is configured but OCR is not forced, so xberg's own
//! per-page trigger decides which pages are OCR'd.

use super::{
    NATIVE_BENCHMARK_CONFIG_JSON, assemble_adapter, base_args_with_config, format_slug, locate_xberg_cli,
    probe_xberg_build_id,
};
use crate::{
    adapter::ExecutableBuildIdentity,
    adapters::subprocess::SubprocessAdapter,
    error::Result,
    types::{OutputFormat, XbergPdfBackend},
};

/// Tesseract is configured but OCR is not forced, so xberg's own per-page trigger decides which
/// pages get OCR'd.
const AUTO_OCR_TESSERACT_BENCHMARK_CONFIG_JSON: &str = r#"{
    "extraction_timeout_secs":1740,
    "use_cache":false,
    "ocr":{
        "enabled":true,
        "backend":"tesseract"
    }
}"#;

/// OCR engine behind an auto-OCR variant, which leaves `force_ocr` unset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoOcrBackend {
    Tesseract,
    Paddle,
}

impl AutoOcrBackend {
    /// Both auto-OCR variants, in registration order.
    pub const ALL: [AutoOcrBackend; 2] = [AutoOcrBackend::Tesseract, AutoOcrBackend::Paddle];

    /// Pipeline slug used in framework names and artifact names.
    pub fn slug(self) -> &'static str {
        match self {
            AutoOcrBackend::Tesseract => "auto-ocr",
            AutoOcrBackend::Paddle => "auto-ocr-paddle",
        }
    }
}

/// Creates an Xberg adapter that configures an OCR engine but does not force OCR, so xberg's own
/// per-page trigger decides which pages are OCR'd.
pub fn create_xberg_auto_ocr_adapter(
    backend: AutoOcrBackend,
    output_format: OutputFormat,
    batch: bool,
    ocr_enabled: bool,
) -> Result<SubprocessAdapter> {
    if !ocr_enabled {
        return Err(crate::Error::Config(format!(
            "xberg pipeline '{}' requires OCR, but OCR is disabled",
            backend.slug()
        )));
    }

    let cli_path = locate_xberg_cli()?;
    let build_id = probe_xberg_build_id(&cli_path)?;
    let build_identity = ExecutableBuildIdentity {
        build_id,
        path: cli_path,
    };
    auto_ocr_adapter_with_identity(backend, output_format, batch, build_identity)
}

/// Builds the auto-OCR adapter for a located executable. It ignores a fixture's `requires_ocr`,
/// since forcing OCR would defeat the point of the variant.
pub(crate) fn auto_ocr_adapter_with_identity(
    backend: AutoOcrBackend,
    output_format: OutputFormat,
    batch: bool,
    build_identity: ExecutableBuildIdentity,
) -> Result<SubprocessAdapter> {
    let mut args = auto_ocr_args(backend, output_format, batch);
    args.push("--pdf-backend".to_string());
    args.push(XbergPdfBackend::Native.as_str().to_string());

    let framework_name = auto_ocr_framework_name(backend, output_format, batch);
    assemble_adapter(framework_name, build_identity, args, output_format, batch, false)
        .map(SubprocessAdapter::with_ignored_fixture_force_ocr)
}

/// Framework name of an auto-OCR variant, e.g. `xberg-markdown-auto-ocr-paddle-batch`.
pub fn auto_ocr_framework_name(backend: AutoOcrBackend, output_format: OutputFormat, batch: bool) -> String {
    let batch_suffix = if batch { "-batch" } else { "" };
    format!("xberg-{}-{}{batch_suffix}", format_slug(output_format), backend.slug())
}

/// Whether `framework` names an auto-OCR variant in any output format and execution mode.
pub fn is_auto_ocr_framework(framework: &str) -> bool {
    AutoOcrBackend::ALL.iter().any(|backend| {
        [OutputFormat::Markdown, OutputFormat::Plaintext]
            .into_iter()
            .any(|format| {
                [false, true]
                    .into_iter()
                    .any(|batch| auto_ocr_framework_name(*backend, format, batch) == framework)
            })
    })
}

pub(super) fn auto_ocr_args(backend: AutoOcrBackend, output_format: OutputFormat, batch: bool) -> Vec<String> {
    let content_format = match output_format {
        OutputFormat::Markdown => "markdown",
        OutputFormat::Plaintext => "plain",
    };
    match backend {
        AutoOcrBackend::Tesseract => {
            base_args_with_config(batch, content_format, AUTO_OCR_TESSERACT_BENCHMARK_CONFIG_JSON)
        }
        AutoOcrBackend::Paddle => {
            let mut args = base_args_with_config(batch, content_format, NATIVE_BENCHMARK_CONFIG_JSON);
            args.extend([
                "--ocr".to_string(),
                "true".to_string(),
                "--ocr-backend".to_string(),
                "paddle-ocr".to_string(),
            ]);
            args
        }
    }
}
