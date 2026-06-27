//! Xberg CLI - Command-line interface for document intelligence.
//!
//! This binary provides a command-line interface to the Xberg document intelligence
//! library, supporting document extraction, MIME type detection, caching, and batch operations.
//!
//! # Architecture
//!
//! The CLI is built using `clap` for argument parsing and provides five main commands:
//! - `extract`: Extract text/data from a single document
//! - `batch`: Process multiple documents in parallel
//! - `detect`: Identify MIME type of a file
//! - `cache`: Manage cache (clear, stats)
//! - `serve`: Start API server (requires `api` feature)
//! - `version`: Show version information
//!
//! # Configuration
//!
//! The CLI supports configuration files in TOML, YAML, or JSON formats:
//! - Explicit: `--config path/to/config.toml`
//! - Auto-discovery: Searches for `xberg.{toml,yaml,json}` in current and parent directories
//! - Inline JSON: `--config-json '{"ocr": {"backend": "tesseract"}}'`
//! - Command-line flags override config file settings
//!
//! Configuration precedence (highest to lowest):
//! 1. Individual CLI flags (--output-format, --ocr, etc.)
//! 2. Inline JSON config (--config-json or --config-json-base64)
//! 3. Config file (--config path.toml)
//! 4. Default values
//!
//! # Exit Codes
//!
//! - 0: Success
//! - Non-zero: Error (see stderr for details)
//!
//! # Examples
//!
//! ```bash
//! # Extract text from a PDF
//! xberg extract document.pdf
//!
//! # Extract with OCR enabled
//! xberg extract scanned.pdf --ocr true
//!
//! # Extract with inline JSON config
//! xberg extract doc.pdf --config-json '{"ocr":{"backend":"tesseract"}}'
//!
//! # Batch processing
//! xberg batch *.pdf --output-format json
//!
//! # Detect MIME type
//! xberg detect unknown-file.bin
//! ```

#![deny(unsafe_code)]

mod commands;
mod logging;
mod output;
mod style;

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use clap::{CommandFactory, Parser, Subcommand};
#[cfg(feature = "embeddings")]
use commands::embed_command;
#[cfg(feature = "mcp")]
use commands::mcp_command;
use commands::overrides::ExtractionOverrides;
#[cfg(feature = "api")]
use commands::serve_command;
use commands::{
    BatchInputFormat, ExtractInputSource, batch_command, chunk_command, clear_command, extract_command,
    extract_structured::{ExtractStructuredArgs, extract_structured_command},
    load_batch_input_manifest, load_config, manifest_command, stats_command, uri_to_local_path, validate_batch_paths,
    validate_chunk_params, validate_file_exists, validate_output_dir, warm_command,
};
use serde_json::json;
use std::path::PathBuf;
use xberg::{OutputFormat as ContentOutputFormat, detect_mime_type};

/// Xberg document intelligence CLI
#[derive(Parser)]
#[command(name = "xberg")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Set log level (trace, debug, info, warn, error). Overrides RUST_LOG env var.
    #[arg(long, global = true)]
    log_level: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract text from a document
    Extract {
        /// URI to the document. Local paths and file:// URIs are supported in this checkout.
        #[arg(value_name = "URI", required_unless_present_any = ["url", "stdin"])]
        uri: Option<String>,

        /// HTTP(S) URL to extract.
        #[arg(long, conflicts_with_all = ["uri", "stdin"])]
        url: Option<String>,

        /// Read document bytes from stdin.
        #[arg(long, conflicts_with_all = ["uri", "url"])]
        stdin: bool,

        /// Path to config file (TOML, YAML, or JSON). If not specified, searches for xberg.toml/yaml/json in current and parent directories.
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Inline JSON configuration. Applied after config file but before individual flags.
        ///
        /// Example: --config-json '{"ocr":{"backend":"tesseract"},"chunking":{"max_chars":1000}}'
        #[arg(long)]
        config_json: Option<String>,

        /// Base64-encoded JSON configuration. Useful for shell environments where quotes are problematic.
        ///
        /// Example: --config-json-base64 eyJvY3IiOnsiYmFja2VuZCI6InRlc3NlcmFjdCJ9fQ==
        #[arg(long)]
        config_json_base64: Option<String>,

        /// MIME type hint (auto-detected if not provided)
        #[arg(short, long)]
        mime_type: Option<String>,

        /// Output format for CLI results (text or json).
        ///
        /// Controls how the CLI displays results, not the extraction content format.
        #[arg(short, long, default_value = "text")]
        format: WireFormat,

        /// Directory where extracted image files are written (text/toon output only).
        ///
        /// When `--extract-images true` is used with text or toon format, the markdown content
        /// references image files by name (e.g. `image_0.png`). Pass this flag to control where
        /// those files land. Defaults to the current working directory when not specified.
        /// Ignored for `--format json` because JSON embeds image bytes inline.
        /// The directory must already exist.
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Extraction configuration overrides
        #[command(flatten)]
        overrides: ExtractionOverrides,
    },

    /// Extract structured data from a document using an LLM
    ExtractStructured {
        /// Path to the document file
        path: PathBuf,

        /// Path to JSON schema file defining the output structure
        #[arg(long)]
        schema: PathBuf,

        /// LLM model (e.g., "openai/gpt-4o")
        #[arg(long)]
        model: String,

        /// API key for the LLM provider
        #[arg(long)]
        api_key: Option<String>,

        /// Custom Jinja2 prompt template
        #[arg(long)]
        prompt: Option<String>,

        /// Schema name
        #[arg(long, default_value = "extraction")]
        schema_name: Option<String>,

        /// Enable strict mode
        #[arg(long)]
        strict: bool,

        /// Config file path
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "json")]
        format: WireFormat,
    },

    /// Batch extract from multiple documents
    Batch {
        /// Paths to documents
        paths: Vec<PathBuf>,

        /// JSON or JSONL manifest containing batch inputs.
        #[arg(long)]
        input: Option<PathBuf>,

        /// Format for --input.
        #[arg(long, value_enum)]
        input_format: Option<BatchInputFormat>,

        /// Path to config file (TOML, YAML, or JSON). If not specified, searches for xberg.toml/yaml/json in current and parent directories.
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Inline JSON configuration. Applied after config file but before individual flags.
        ///
        /// Example: --config-json '{"ocr":{"backend":"tesseract"},"chunking":{"max_chars":1000}}'
        #[arg(long)]
        config_json: Option<String>,

        /// Base64-encoded JSON configuration. Useful for shell environments where quotes are problematic.
        ///
        /// Example: --config-json-base64 eyJvY3IiOnsiYmFja2VuZCI6InRlc3NlcmFjdCJ9fQ==
        #[arg(long)]
        config_json_base64: Option<String>,

        /// Output format for CLI results (text or json).
        ///
        /// Controls how the CLI displays results, not the extraction content format.
        #[arg(short, long, default_value = "json")]
        format: WireFormat,

        /// Directory where extracted image files are written (text/toon output only).
        ///
        /// When `--extract-images true` is used with text or toon format, the markdown content
        /// references image files by name (e.g. `image_0.png`). Pass this flag to control where
        /// those files land. Defaults to the current working directory when not specified.
        /// Ignored for `--format json` because JSON embeds image bytes inline.
        /// The directory must already exist.
        #[arg(long)]
        output_dir: Option<PathBuf>,

        /// Extraction configuration overrides
        #[command(flatten)]
        overrides: ExtractionOverrides,

        /// Path to a JSON file mapping file paths to per-file extraction config overrides.
        /// The JSON should be an object where keys are file paths and values are FileExtractionConfig objects.
        /// Example: {"doc1.pdf": {"force_ocr": true}, "doc2.pdf": {"output_format": "markdown"}}
        #[arg(long)]
        file_configs: Option<PathBuf>,
    },

    /// Detect MIME type of a file
    Detect {
        /// Path to the file
        path: PathBuf,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: WireFormat,
    },

    /// List all supported document formats
    Formats {
        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: WireFormat,
    },

    /// Show version information
    Version {
        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: WireFormat,
    },

    /// Cache management operations
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },

    /// Start the API server
    ///
    /// Configuration is loaded with the following precedence (highest to lowest):
    /// 1. CLI arguments (--host, --port)
    /// 2. Environment variables (XBERG_HOST, XBERG_PORT)
    /// 3. Config file (TOML, YAML, or JSON)
    /// 4. Built-in defaults (127.0.0.1:8000)
    ///
    /// The config file can contain both extraction and server settings under `[server]` section.
    #[cfg(feature = "api")]
    Serve {
        /// Host to bind to (e.g., "127.0.0.1" or "0.0.0.0"). CLI arg overrides config file and env vars.
        #[arg(short = 'H', long)]
        host: Option<String>,

        /// Port to bind to. CLI arg overrides config file and env vars.
        #[arg(short, long)]
        port: Option<u16>,

        /// Path to config file (TOML, YAML, or JSON). If not specified, searches for xberg.toml/yaml/json in current and parent directories.
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Start the MCP (Model Context Protocol) server
    #[cfg(feature = "mcp")]
    Mcp {
        /// Path to config file (TOML, YAML, or JSON). If not specified, searches for xberg.toml/yaml/json in current and parent directories.
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Transport mode: stdio (default) or http
        #[arg(long, default_value = "stdio")]
        transport: String,

        /// HTTP host (only for --transport http)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// HTTP port (only for --transport http)
        #[arg(long, default_value = "8001")]
        port: u16,
    },

    /// API utilities
    #[cfg(feature = "api")]
    Api {
        #[command(subcommand)]
        command: ApiCommands,
    },

    /// Generate embeddings for text
    ///
    /// Generates vector embeddings for one or more text inputs using a specified preset model
    /// or an LLM provider. Reads from --text flag or stdin if no text is provided.
    #[cfg(feature = "embeddings")]
    Embed {
        /// Text to embed. Can be specified multiple times for batch embedding.
        #[arg(long)]
        text: Vec<String>,

        /// Embedding preset (fast, balanced, quality, multilingual). Used with --provider local.
        #[arg(long, default_value = "balanced")]
        preset: String,

        /// Embedding provider: "local" (default, ONNX), "llm" (liter-llm), or "plugin" (registered in-process backend)
        #[arg(long, default_value = "local")]
        provider: String,

        /// LLM model for provider-hosted embeddings (e.g., "openai/text-embedding-3-small").
        /// Required when --provider is "llm".
        #[arg(long)]
        model: Option<String>,

        /// API key for the LLM provider
        #[arg(long)]
        api_key: Option<String>,

        /// Name of a pre-registered in-process embedding backend.
        /// Required when --provider is "plugin". The backend must have been
        /// registered via `xberg::plugins::register_embedding_backend`
        /// before this command runs.
        #[arg(long)]
        plugin: Option<String>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "json")]
        format: WireFormat,
    },

    /// Chunk text for processing
    ///
    /// Splits text into chunks using configurable size and overlap.
    /// Reads from --text flag or stdin if no text is provided.
    Chunk {
        /// Text to chunk. If not provided, reads from stdin.
        #[arg(long)]
        text: Option<String>,

        /// Path to config file (TOML, YAML, or JSON)
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Chunk size in characters
        #[arg(long)]
        chunk_size: Option<usize>,

        /// Chunk overlap in characters
        #[arg(long)]
        chunk_overlap: Option<usize>,

        /// Chunker type: text, markdown, yaml, or semantic
        #[arg(long, default_value = "text")]
        chunker_type: String,

        /// Tokenizer model for token-based chunk sizing (e.g., "Xenova/gpt-4o").
        /// Requires the chunking-tokenizers feature.
        #[arg(long)]
        chunking_tokenizer: Option<String>,

        /// Topic threshold for semantic chunking (0.0-1.0, default: 0.75)
        #[arg(long)]
        topic_threshold: Option<f32>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "json")]
        format: WireFormat,
    },

    /// Generate shell completions
    ///
    /// Outputs shell completion scripts for the specified shell.
    /// Install with: eval "$(xberg completions bash)"
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[cfg(feature = "api")]
#[derive(Subcommand)]
enum ApiCommands {
    /// Output the OpenAPI schema (JSON)
    ///
    /// Prints the full OpenAPI 3.1 specification for the xberg REST API.
    /// Useful for code generation, documentation, and API client tooling.
    Schema,
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Show cache statistics
    Stats {
        /// Cache directory (default: .xberg in current directory)
        #[arg(short, long)]
        cache_dir: Option<PathBuf>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: WireFormat,
    },

    /// Clear the cache
    Clear {
        /// Cache directory (default: .xberg in current directory)
        #[arg(short, long)]
        cache_dir: Option<PathBuf>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: WireFormat,
    },

    /// Output model manifest (expected model files, checksums, sizes)
    ///
    /// Outputs a JSON manifest of all model files required by xberg,
    /// including their relative paths, SHA256 checksums, and sizes.
    /// Used for pre-populating model caches in containerized deployments.
    Manifest {
        /// Output format (text or json)
        #[arg(short, long, default_value = "json")]
        format: WireFormat,
    },

    /// Download model artifacts eagerly
    ///
    /// Downloads model artifacts for offline/container use. Unlike normal
    /// operation which downloads lazily on first use, this ensures selected
    /// models are present in the cache directory.
    ///
    /// Use --all-embeddings to also download all 4 embedding model presets,
    /// or `--embedding-model <preset>` to download a specific one.
    ///
    /// By default, only the core layout models (rtdetr + tatr) are downloaded.
    /// Use --all-table-models to also download SLANeXT variants (~730MB).
    ///
    /// Use --ner to download the default GLiNER NER model, --ner-model <MODEL>
    /// for a specific GLiNER alias/catalog id, or --all-ner-models for every
    /// known GLiNER NER model.
    Warm {
        /// Cache directory (default: .xberg in current directory, or XBERG_CACHE_DIR)
        #[arg(short, long)]
        cache_dir: Option<PathBuf>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: WireFormat,

        /// Download all embedding model presets (fast, balanced, quality, multilingual)
        #[arg(long)]
        all_embeddings: bool,

        /// Download a specific embedding model preset
        #[arg(long, value_name = "PRESET")]
        embedding_model: Option<String>,

        /// Download all table structure models including SLANeXT variants (~730MB)
        #[arg(
            long,
            help = "Download all table structure models including SLANeXT variants (~730MB)"
        )]
        all_table_models: bool,

        /// Download all tree-sitter grammar parsers
        #[arg(long)]
        all_grammars: bool,

        /// Download specific tree-sitter grammar groups (comma-separated: web,systems,scripting,data,jvm,functional)
        #[arg(long, value_name = "GROUPS", value_delimiter = ',')]
        grammar_groups: Option<Vec<String>>,

        /// Download specific tree-sitter grammars by language name (comma-separated)
        #[arg(long, value_name = "LANGUAGES", value_delimiter = ',')]
        grammars: Option<Vec<String>>,

        /// Download the default xberg GLiNER NER model alias
        #[cfg(feature = "ner-onnx")]
        #[arg(long)]
        ner: bool,

        /// Download a specific xberg GLiNER NER model alias or catalog id
        #[cfg(feature = "ner-onnx")]
        #[arg(long, value_name = "MODEL")]
        ner_model: Option<String>,

        /// Download every GLiNER NER model variant xberg knows about
        #[cfg(feature = "ner-onnx")]
        #[arg(long)]
        all_ner_models: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WireFormat {
    Text,
    Json,
    Toon,
}

impl std::str::FromStr for WireFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(WireFormat::Text),
            "json" => Ok(WireFormat::Json),
            "toon" => Ok(WireFormat::Toon),
            _ => Err(format!("Invalid format: {}. Use 'text', 'json', or 'toon'", s)),
        }
    }
}

/// Content output format for extraction results.
///
/// Controls the format of the extracted content (not the CLI output format).
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
enum ContentOutputFormatArg {
    /// Plain text (default)
    Plain,
    /// Markdown format
    Markdown,
    /// Djot markup format
    Djot,
    /// HTML format
    Html,
    /// JSON tree format with heading-driven sections
    Json,
}

impl From<ContentOutputFormatArg> for ContentOutputFormat {
    fn from(arg: ContentOutputFormatArg) -> Self {
        match arg {
            ContentOutputFormatArg::Plain => ContentOutputFormat::Plain,
            ContentOutputFormatArg::Markdown => ContentOutputFormat::Markdown,
            ContentOutputFormatArg::Djot => ContentOutputFormat::Djot,
            ContentOutputFormatArg::Html => ContentOutputFormat::Html,
            ContentOutputFormatArg::Json => ContentOutputFormat::Json,
        }
    }
}

/// Apply inline JSON or base64 JSON overrides to an extraction config.
fn apply_json_overrides(
    config: &mut xberg::ExtractionConfig,
    config_json: Option<String>,
    config_json_base64: Option<String>,
) -> Result<()> {
    if let Some(json_str) = config_json {
        let json_value: serde_json::Value =
            serde_json::from_str(&json_str).context("Failed to parse --config-json as JSON")?;
        *config =
            merge_json_into_config(config, json_value).context("Failed to merge --config-json with file config")?;
    } else if let Some(base64_str) = config_json_base64 {
        let json_bytes = STANDARD
            .decode(&base64_str)
            .context("Failed to decode base64 in --config-json-base64")?;
        let json_str = String::from_utf8(json_bytes).context("Base64-decoded content is not valid UTF-8")?;
        let json_value: serde_json::Value =
            serde_json::from_str(&json_str).context("Failed to parse decoded --config-json-base64 as JSON")?;
        *config = merge_json_into_config(config, json_value)
            .context("Failed to merge --config-json-base64 with file config")?;
    }
    Ok(())
}

/// Merges a JSON value into an existing extraction config via field-by-field override.
fn merge_json_into_config(
    base_config: &xberg::ExtractionConfig,
    json_value: serde_json::Value,
) -> Result<xberg::ExtractionConfig> {
    let json_str = serde_json::to_string(&json_value).map_err(|e| anyhow::anyhow!("{}", e))?;
    xberg::core::config::merge::merge_config_json(base_config, &json_str).map_err(|e| anyhow::anyhow!("{}", e))
}

fn resolve_extract_input(uri: Option<String>, url: Option<String>, stdin: bool) -> Result<ExtractInputSource> {
    match (uri, url, stdin) {
        (Some(uri), None, false) => Ok(ExtractInputSource::Uri(uri)),
        (None, Some(url), false) => Ok(ExtractInputSource::Uri(url)),
        (None, None, true) => Ok(ExtractInputSource::Stdin),
        _ => anyhow::bail!("Provide exactly one extraction input: URI, --url, or --stdin."),
    }
}

fn validate_extract_input(input: &ExtractInputSource) -> Result<()> {
    match input {
        ExtractInputSource::Stdin => Ok(()),
        ExtractInputSource::Uri(uri) => {
            if is_remote_uri(uri) {
                return Ok(());
            }
            let path = uri_to_local_path(uri)?;
            validate_file_exists(&path)
        }
    }
}

fn resolve_batch_inputs(
    paths: Vec<PathBuf>,
    input: Option<PathBuf>,
    input_format: Option<BatchInputFormat>,
) -> Result<Vec<String>> {
    let mut uris: Vec<String> = paths
        .into_iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect();

    if let Some(input_path) = input {
        let format = input_format.unwrap_or_else(|| infer_batch_input_format(&input_path));
        uris.extend(load_batch_input_manifest(&input_path, format)?);
    }

    if uris.is_empty() {
        anyhow::bail!("No files provided for batch extraction. Provide paths or --input.");
    }

    Ok(uris)
}

fn infer_batch_input_format(path: &std::path::Path) -> BatchInputFormat {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("jsonl") || ext.eq_ignore_ascii_case("ndjson") => BatchInputFormat::Jsonl,
        _ => BatchInputFormat::Json,
    }
}

fn validate_batch_input_uris(uris: &[String]) -> Result<()> {
    let local_paths: Vec<PathBuf> = uris
        .iter()
        .filter(|uri| !is_remote_uri(uri))
        .map(|uri| uri_to_local_path(uri))
        .collect::<Result<Vec<_>>>()?;
    if local_paths.is_empty() {
        return Ok(());
    }
    validate_batch_paths(&local_paths)
}

fn is_remote_uri(uri: &str) -> bool {
    uri.starts_with("http://") || uri.starts_with("https://")
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let env_filter = logging::build_env_filter(cli.log_level.as_deref());

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .try_init();

    match cli.command {
        Commands::Extract {
            uri,
            url,
            stdin,
            config: config_path,
            config_json,
            config_json_base64,
            mime_type,
            format,
            output_dir,
            overrides,
        } => {
            let input = resolve_extract_input(uri, url, stdin)?;
            validate_extract_input(&input)?;
            if let Some(ref dir) = output_dir {
                validate_output_dir(dir)?;
            }
            overrides.validate()?;

            let mut config = load_config(config_path)?;
            apply_json_overrides(&mut config, config_json, config_json_base64)?;
            overrides.apply(&mut config);

            extract_command(input, config, mime_type, format, output_dir)?;
        }

        Commands::ExtractStructured {
            path,
            schema,
            model,
            api_key,
            prompt,
            schema_name,
            strict,
            config,
            format,
        } => {
            validate_file_exists(&path)?;
            validate_file_exists(&schema)?;
            extract_structured_command(ExtractStructuredArgs {
                path,
                schema_path: schema,
                model,
                api_key,
                prompt,
                schema_name,
                strict,
                config_path: config,
                format,
            })?;
        }

        Commands::Batch {
            paths,
            input,
            input_format,
            config: config_path,
            config_json,
            config_json_base64,
            format,
            output_dir,
            overrides,
            file_configs,
        } => {
            let input_uris = resolve_batch_inputs(paths, input, input_format)?;
            validate_batch_input_uris(&input_uris)?;
            if let Some(ref dir) = output_dir {
                validate_output_dir(dir)?;
            }
            overrides.validate()?;

            let mut config = load_config(config_path)?;
            apply_json_overrides(&mut config, config_json, config_json_base64)?;
            overrides.apply(&mut config);

            let file_configs_map = if let Some(file_configs_path) = file_configs {
                let file_configs_json = std::fs::read_to_string(&file_configs_path)
                    .with_context(|| format!("Failed to read file configs from '{}'", file_configs_path.display()))?;
                let map: std::collections::HashMap<String, serde_json::Value> =
                    serde_json::from_str(&file_configs_json).with_context(|| {
                        format!(
                            "Failed to parse file configs JSON from '{}'",
                            file_configs_path.display()
                        )
                    })?;
                Some(map)
            } else {
                None
            };
            batch_command(input_uris, file_configs_map, config, format, output_dir)?;
        }

        Commands::Detect { path, format } => {
            validate_file_exists(&path)?;

            let path_str = path.to_string_lossy().to_string();
            let mime_type = detect_mime_type(path_str.clone(), true).with_context(|| {
                format!(
                    "Failed to detect MIME type for file '{}'. Ensure the file is readable.",
                    path.display()
                )
            })?;

            match format {
                WireFormat::Text => {
                    println!("{}", style::success(&mime_type));
                }
                WireFormat::Json => {
                    let output = json!({
                        "path": path_str,
                        "mime_type": mime_type,
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output)
                            .context("Failed to serialize MIME type detection result to JSON")?
                    );
                }
                WireFormat::Toon => {
                    let output = json!({
                        "path": path_str,
                        "mime_type": mime_type,
                    });
                    println!(
                        "{}",
                        serde_toon::to_string(&output)
                            .context("Failed to serialize MIME type detection result to TOON")?
                    );
                }
            }
        }

        Commands::Formats { format } => {
            let formats = xberg::core::mime::list_supported_formats();
            match format {
                WireFormat::Text => {
                    println!("{:<15} {}", style::label("EXTENSION"), style::label("MIME TYPE"));
                    println!("{}", style::dim(&format!("{:<15} ---------", "---------")));
                    for f in &formats {
                        println!("{:<15} {}", style::success(&format!(".{}", f.extension)), f.mime_type);
                    }
                }
                WireFormat::Json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&formats).context("Failed to serialize formats to JSON")?
                    );
                }
                WireFormat::Toon => {
                    println!(
                        "{}",
                        serde_toon::to_string(&formats).context("Failed to serialize formats to TOON")?
                    );
                }
            }
        }

        Commands::Version { format } => {
            let version = env!("CARGO_PKG_VERSION");
            let name = env!("CARGO_PKG_NAME");

            match format {
                WireFormat::Text => {
                    println!("{} {}", style::label(name), style::success(version));
                }
                WireFormat::Json => {
                    let output = json!({
                        "name": name,
                        "version": version,
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output)
                            .context("Failed to serialize version information to JSON")?
                    );
                }
                WireFormat::Toon => {
                    let output = json!({
                        "name": name,
                        "version": version,
                    });
                    println!(
                        "{}",
                        serde_toon::to_string(&output).context("Failed to serialize version information to TOON")?
                    );
                }
            }
        }

        #[cfg(feature = "api")]
        Commands::Serve {
            host: cli_host,
            port: cli_port,
            config: config_path,
        } => {
            let mut extraction_config = load_config(config_path.clone())?;
            extraction_config.apply_env_overrides()?;
            serve_command(cli_host, cli_port, extraction_config, config_path)?;
        }

        #[cfg(feature = "mcp")]
        Commands::Mcp {
            config: config_path,
            transport,
            #[cfg(feature = "mcp-http")]
            host,
            #[cfg(feature = "mcp-http")]
            port,
            #[cfg(not(feature = "mcp-http"))]
            host,
            #[cfg(not(feature = "mcp-http"))]
            port,
        } => {
            let mut config = load_config(config_path)?;
            config.apply_env_overrides()?;
            mcp_command(config, transport, host, port)?;
        }

        Commands::Cache { command } => match command {
            CacheCommands::Stats { cache_dir, format } => {
                stats_command(cache_dir, format)?;
            }
            CacheCommands::Clear { cache_dir, format } => {
                clear_command(cache_dir, format)?;
            }
            CacheCommands::Manifest { format } => {
                manifest_command(format)?;
            }
            CacheCommands::Warm {
                cache_dir,
                format,
                all_embeddings,
                embedding_model,
                all_table_models,
                all_grammars,
                grammar_groups,
                grammars,
                #[cfg(feature = "ner-onnx")]
                ner,
                #[cfg(feature = "ner-onnx")]
                ner_model,
                #[cfg(feature = "ner-onnx")]
                all_ner_models,
            } => {
                warm_command(
                    cache_dir.clone(),
                    format,
                    all_embeddings,
                    embedding_model,
                    all_table_models,
                    all_grammars,
                    grammar_groups,
                    grammars,
                    #[cfg(feature = "ner-onnx")]
                    ner,
                    #[cfg(feature = "ner-onnx")]
                    ner_model,
                    #[cfg(feature = "ner-onnx")]
                    all_ner_models,
                )?;
            }
        },

        #[cfg(feature = "api")]
        Commands::Api { command } => match command {
            ApiCommands::Schema => {
                println!("{}", xberg::api::openapi::openapi_json());
            }
        },

        #[cfg(feature = "embeddings")]
        Commands::Embed {
            text,
            preset,
            provider,
            model,
            api_key,
            plugin,
            format,
        } => {
            let texts = if text.is_empty() {
                vec![commands::read_stdin()?]
            } else {
                text
            };
            embed_command(texts, &preset, &provider, model, api_key, plugin, format)?;
        }

        Commands::Chunk {
            text,
            config: config_path,
            chunk_size,
            chunk_overlap,
            chunker_type,
            chunking_tokenizer,
            topic_threshold,
            format,
        } => {
            let input = match text {
                Some(t) => t,
                None => commands::read_stdin().context("No --text provided and failed to read from stdin")?,
            };

            validate_chunk_params(chunk_size, chunk_overlap)?;

            let base_config = load_config(config_path)?;
            let mut chunking_config = base_config.chunking.unwrap_or_default();

            if let Some(size) = chunk_size {
                chunking_config.max_characters = size;
                // If user set chunk_size but not overlap, clamp overlap to fit
                if chunk_overlap.is_none() && chunking_config.overlap >= size {
                    chunking_config.overlap = size / 4;
                }
            }
            if let Some(overlap) = chunk_overlap {
                chunking_config.overlap = overlap;
            }
            match chunker_type.as_str() {
                "markdown" => chunking_config.chunker_type = xberg::ChunkerType::Markdown,
                "yaml" => chunking_config.chunker_type = xberg::ChunkerType::Yaml,
                "semantic" => chunking_config.chunker_type = xberg::ChunkerType::Semantic,
                _ => chunking_config.chunker_type = xberg::ChunkerType::Text,
            }
            #[cfg(feature = "chunking-tokenizers")]
            if let Some(ref tokenizer) = chunking_tokenizer {
                chunking_config.sizing = xberg::ChunkSizing::Tokenizer {
                    model: tokenizer.clone(),
                    cache_dir: None,
                };
            }
            #[cfg(not(feature = "chunking-tokenizers"))]
            if chunking_tokenizer.is_some() {
                anyhow::bail!("--chunking-tokenizer requires the chunking-tokenizers feature");
            }
            if topic_threshold.is_some() {
                chunking_config.topic_threshold = topic_threshold;
            }

            chunk_command(input, chunking_config, format)?;
        }

        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "xberg", &mut std::io::stdout());
        }
    }

    Ok(())
}
