//! Kreuzberg CLI - Command-line interface for document intelligence.
//!
//! This binary provides a command-line interface to the Kreuzberg document intelligence
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
//! - Auto-discovery: Searches for `kreuzberg.{toml,yaml,json}` in current and parent directories
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
//! kreuzberg extract document.pdf
//!
//! # Extract with OCR enabled
//! kreuzberg extract scanned.pdf --ocr true
//!
//! # Extract with inline JSON config
//! kreuzberg extract doc.pdf --config-json '{"ocr":{"backend":"tesseract"}}'
//!
//! # Batch processing
//! kreuzberg batch *.pdf --output-format json
//!
//! # Detect MIME type
//! kreuzberg detect unknown-file.bin
//! ```

#![deny(unsafe_code)]

mod commands;

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use clap::{CommandFactory, Parser, Subcommand};
#[cfg(feature = "embeddings")]
use commands::embed_command;
#[cfg(feature = "mcp")]
use commands::mcp_command;
#[cfg(feature = "api")]
use commands::serve_command;
use commands::{
    apply_extraction_overrides, batch_command, chunk_command, clear_command, extract_command, load_config,
    manifest_command, stats_command, warm_command,
};
use kreuzberg::{OutputFormat as ContentOutputFormat, detect_mime_type};
use serde_json::json;
use std::path::{Path, PathBuf};
use tracing_subscriber::EnvFilter;

/// Kreuzberg document intelligence CLI
#[derive(Parser)]
#[command(name = "kreuzberg")]
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
        /// Path to the document
        path: PathBuf,

        /// Path to config file (TOML, YAML, or JSON). If not specified, searches for kreuzberg.toml/yaml/json in current and parent directories.
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
        format: OutputFormat,

        /// Enable OCR (overrides config file)
        #[arg(long)]
        ocr: Option<bool>,

        /// OCR backend to use when --ocr is enabled (tesseract, paddle-ocr, easyocr)
        #[arg(long)]
        ocr_backend: Option<String>,

        /// OCR language code. Tesseract: ISO 639-3 (eng, fra, deu). PaddleOCR: flexible (en, ch, french, korean).
        #[arg(long)]
        ocr_language: Option<String>,

        /// Force OCR even if text extraction succeeds (overrides config file)
        #[arg(long)]
        force_ocr: Option<bool>,

        /// Disable caching (overrides config file)
        #[arg(long)]
        no_cache: Option<bool>,

        /// Enable chunking (overrides config file)
        #[arg(long)]
        chunk: Option<bool>,

        /// Chunk size in characters (overrides config file)
        #[arg(long)]
        chunk_size: Option<usize>,

        /// Chunk overlap in characters (overrides config file)
        #[arg(long)]
        chunk_overlap: Option<usize>,

        /// Tokenizer model for token-based chunk sizing (e.g., "Xenova/gpt-4o", "bert-base-uncased").
        /// Implicitly enables chunking. Requires the chunking-tokenizers feature.
        #[arg(long)]
        chunking_tokenizer: Option<String>,

        /// Enable quality processing (overrides config file)
        #[arg(long)]
        quality: Option<bool>,

        /// Enable language detection (overrides config file)
        #[arg(long)]
        detect_language: Option<bool>,

        /// Content output format (plain, markdown, djot, html). Canonical flag.
        ///
        /// Controls the format of the extracted content.
        /// Note: This is different from --format which controls CLI output (text/json).
        #[arg(long, value_enum)]
        output_format: Option<ContentOutputFormatArg>,

        /// Content output format (DEPRECATED: Use --output-format instead).
        ///
        /// This flag is maintained for backward compatibility. Use --output-format for new code.
        #[arg(long, value_enum, hide = true)]
        content_format: Option<ContentOutputFormatArg>,

        /// Password(s) for encrypted PDFs. Can be specified multiple times.
        #[arg(long)]
        pdf_password: Vec<String>,
    },

    /// Batch extract from multiple documents
    Batch {
        /// Paths to documents
        paths: Vec<PathBuf>,

        /// Path to config file (TOML, YAML, or JSON). If not specified, searches for kreuzberg.toml/yaml/json in current and parent directories.
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
        format: OutputFormat,

        /// Enable OCR (overrides config file)
        #[arg(long)]
        ocr: Option<bool>,

        /// OCR backend to use when --ocr is enabled (tesseract, paddle-ocr, easyocr)
        #[arg(long)]
        ocr_backend: Option<String>,

        /// OCR language code. Tesseract: ISO 639-3 (eng, fra, deu). PaddleOCR: flexible (en, ch, french, korean).
        #[arg(long)]
        ocr_language: Option<String>,

        /// Force OCR even if text extraction succeeds (overrides config file)
        #[arg(long)]
        force_ocr: Option<bool>,

        /// Disable caching (overrides config file)
        #[arg(long)]
        no_cache: Option<bool>,

        /// Enable chunking (overrides config file)
        #[arg(long)]
        chunk: Option<bool>,

        /// Chunk size in characters (overrides config file)
        #[arg(long)]
        chunk_size: Option<usize>,

        /// Chunk overlap in characters (overrides config file)
        #[arg(long)]
        chunk_overlap: Option<usize>,

        /// Tokenizer model for token-based chunk sizing (e.g., "Xenova/gpt-4o", "bert-base-uncased").
        /// Implicitly enables chunking. Requires the chunking-tokenizers feature.
        #[arg(long)]
        chunking_tokenizer: Option<String>,

        /// Enable quality processing (overrides config file)
        #[arg(long)]
        quality: Option<bool>,

        /// Enable language detection (overrides config file)
        #[arg(long)]
        detect_language: Option<bool>,

        /// Content output format (plain, markdown, djot, html). Canonical flag.
        ///
        /// Controls the format of the extracted content.
        /// Note: This is different from --format which controls CLI output (text/json).
        #[arg(long, value_enum)]
        output_format: Option<ContentOutputFormatArg>,

        /// Content output format (DEPRECATED: Use --output-format instead).
        ///
        /// This flag is maintained for backward compatibility. Use --output-format for new code.
        #[arg(long, value_enum, hide = true)]
        content_format: Option<ContentOutputFormatArg>,

        /// Password(s) for encrypted PDFs. Can be specified multiple times.
        #[arg(long)]
        pdf_password: Vec<String>,

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
        format: OutputFormat,
    },

    /// List all supported document formats
    Formats {
        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },

    /// Show version information
    Version {
        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
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
    /// 2. Environment variables (KREUZBERG_HOST, KREUZBERG_PORT)
    /// 3. Config file (TOML, YAML, or JSON)
    /// 4. Built-in defaults (127.0.0.1:8000)
    ///
    /// The config file can contain both extraction and server settings under [server] section.
    #[cfg(feature = "api")]
    Serve {
        /// Host to bind to (e.g., "127.0.0.1" or "0.0.0.0"). CLI arg overrides config file and env vars.
        #[arg(short = 'H', long)]
        host: Option<String>,

        /// Port to bind to. CLI arg overrides config file and env vars.
        #[arg(short, long)]
        port: Option<u16>,

        /// Path to config file (TOML, YAML, or JSON). If not specified, searches for kreuzberg.toml/yaml/json in current and parent directories.
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Start the MCP (Model Context Protocol) server
    #[cfg(feature = "mcp")]
    Mcp {
        /// Path to config file (TOML, YAML, or JSON). If not specified, searches for kreuzberg.toml/yaml/json in current and parent directories.
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

    /// Generate embeddings for text
    ///
    /// Generates vector embeddings for one or more text inputs using a specified preset model.
    /// Reads from --text flag or stdin if no text is provided.
    #[cfg(feature = "embeddings")]
    Embed {
        /// Text to embed. Can be specified multiple times for batch embedding.
        #[arg(long)]
        text: Vec<String>,

        /// Embedding preset (fast, balanced, quality, multilingual)
        #[arg(long, default_value = "balanced")]
        preset: String,

        /// Output format (text or json)
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,
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

        /// Chunker type: text or markdown
        #[arg(long, default_value = "text")]
        chunker_type: String,

        /// Tokenizer model for token-based chunk sizing (e.g., "Xenova/gpt-4o").
        /// Requires the chunking-tokenizers feature.
        #[arg(long)]
        chunking_tokenizer: Option<String>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,
    },

    /// Generate shell completions
    ///
    /// Outputs shell completion scripts for the specified shell.
    /// Install with: eval "$(kreuzberg completions bash)"
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Show cache statistics
    Stats {
        /// Cache directory (default: .kreuzberg in current directory)
        #[arg(short, long)]
        cache_dir: Option<PathBuf>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },

    /// Clear the cache
    Clear {
        /// Cache directory (default: .kreuzberg in current directory)
        #[arg(short, long)]
        cache_dir: Option<PathBuf>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },

    /// Output model manifest (expected model files, checksums, sizes)
    ///
    /// Outputs a JSON manifest of all model files required by kreuzberg,
    /// including their relative paths, SHA256 checksums, and sizes.
    /// Used for pre-populating model caches in containerized deployments.
    Manifest {
        /// Output format (text or json)
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,
    },

    /// Download all models eagerly
    ///
    /// Downloads all PaddleOCR and layout detection models for all supported
    /// languages. Unlike normal operation which downloads lazily on first use,
    /// this ensures all models are present in the cache directory.
    ///
    /// Use --all-embeddings to also download all 4 embedding model presets,
    /// or --embedding-model <preset> to download a specific one.
    Warm {
        /// Cache directory (default: .kreuzberg in current directory, or KREUZBERG_CACHE_DIR)
        #[arg(short, long)]
        cache_dir: Option<PathBuf>,

        /// Output format (text or json)
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,

        /// Download all embedding model presets (fast, balanced, quality, multilingual)
        #[arg(long)]
        all_embeddings: bool,

        /// Download a specific embedding model preset
        #[arg(long, value_name = "PRESET")]
        embedding_model: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!("Invalid format: {}. Use 'text' or 'json'", s)),
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
}

impl From<ContentOutputFormatArg> for ContentOutputFormat {
    fn from(arg: ContentOutputFormatArg) -> Self {
        match arg {
            ContentOutputFormatArg::Plain => ContentOutputFormat::Plain,
            ContentOutputFormatArg::Markdown => ContentOutputFormat::Markdown,
            ContentOutputFormatArg::Djot => ContentOutputFormat::Djot,
            ContentOutputFormatArg::Html => ContentOutputFormat::Html,
        }
    }
}

/// Validates that a file exists and is accessible.
///
/// Checks that the path exists in the filesystem and points to a regular file
/// (not a directory or special file). Provides user-friendly error messages if validation fails.
///
/// # Errors
///
/// Returns an error if:
/// - The path does not exist in the filesystem
/// - The path exists but is not a regular file (e.g., is a directory)
fn validate_file_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!(
            "File not found: '{}'. Please check that the file exists and is accessible.",
            path.display()
        );
    }
    if !path.is_file() {
        anyhow::bail!(
            "Path is not a file: '{}'. Please provide a path to a regular file.",
            path.display()
        );
    }
    Ok(())
}

/// Validates chunking parameters for correctness.
///
/// Ensures that chunking configuration makes sense: size must be positive and reasonable,
/// and overlap must be smaller than chunk size. This prevents common configuration errors
/// that would lead to cryptic failures from the underlying library.
///
/// # Errors
///
/// Returns an error if:
/// - `chunk_size` is 0 (must be at least 1 character)
/// - `chunk_size` exceeds 1,000,000 characters (to prevent excessive memory usage)
/// - `chunk_overlap` is greater than or equal to `chunk_size` (overlap must be smaller)
fn validate_chunk_params(chunk_size: Option<usize>, chunk_overlap: Option<usize>) -> Result<()> {
    if let Some(size) = chunk_size {
        if size == 0 {
            anyhow::bail!("Invalid chunk size: {}. Chunk size must be greater than 0.", size);
        }
        if size > 1_000_000 {
            anyhow::bail!(
                "Invalid chunk size: {}. Chunk size must be less than 1,000,000 characters to avoid excessive memory usage.",
                size
            );
        }
    }

    if let Some(overlap) = chunk_overlap
        && let Some(size) = chunk_size
        && overlap >= size
    {
        anyhow::bail!(
            "Invalid chunk overlap: {}. Overlap ({}) must be less than chunk size ({}).",
            overlap,
            overlap,
            size
        );
    }

    Ok(())
}

/// Validates batch extraction paths for correctness.
///
/// Ensures that at least one file path is provided and that all paths point to valid,
/// accessible files. This prevents processing empty batches or failing mid-batch due
/// to invalid paths.
///
/// # Errors
///
/// Returns an error if:
/// - The paths array is empty (at least one file is required)
/// - Any path does not exist or is not a regular file
fn validate_batch_paths(paths: &[PathBuf]) -> Result<()> {
    if paths.is_empty() {
        anyhow::bail!("No files provided for batch extraction. Please provide at least one file path.");
    }

    for (i, path) in paths.iter().enumerate() {
        validate_file_exists(path).with_context(|| format!("Invalid file at position {}", i + 1))?;
    }

    Ok(())
}

/// Merges a JSON value into an existing extraction config.
///
/// This function performs a field-by-field merge where JSON fields override
/// config fields when present. Unspecified fields in the JSON retain their
/// values from the base config.
///
/// # Strategy
///
/// For each field in the JSON:
/// - If present in JSON, override the config value
/// - If not present in JSON, keep the config value
/// - This enables partial config updates via CLI flags
fn merge_json_into_config(
    base_config: &kreuzberg::ExtractionConfig,
    json_value: serde_json::Value,
) -> Result<kreuzberg::ExtractionConfig> {
    // Serialize base config to JSON
    let mut config_json = serde_json::to_value(base_config).context("Failed to serialize base config to JSON")?;

    // Merge JSON value into config JSON (simple recursive merge)
    // For each key in the provided JSON, override the corresponding key in config JSON
    if let serde_json::Value::Object(json_obj) = json_value
        && let serde_json::Value::Object(ref mut config_obj) = config_json
    {
        for (key, value) in json_obj {
            config_obj.insert(key, value);
        }
    }

    // Deserialize merged JSON back to ExtractionConfig
    let merged_config: kreuzberg::ExtractionConfig =
        serde_json::from_value(config_json).context("Failed to deserialize merged config")?;

    Ok(merged_config)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let env_filter = if let Some(ref level) = cli.log_level {
        EnvFilter::new(level)
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .try_init();

    match cli.command {
        Commands::Extract {
            path,
            config: config_path,
            config_json,
            config_json_base64,
            mime_type,
            format,
            ocr,
            ocr_backend,
            ocr_language,
            force_ocr,
            no_cache,
            chunk,
            chunk_size,
            chunk_overlap,
            chunking_tokenizer,
            quality,
            detect_language,
            output_format,
            content_format,
            pdf_password,
        } => {
            validate_file_exists(&path)?;
            validate_chunk_params(chunk_size, chunk_overlap)?;

            let mut config = load_config(config_path)?;

            // Apply inline JSON config if provided (merge with file config)
            if let Some(json_str) = config_json {
                let json_value: serde_json::Value =
                    serde_json::from_str(&json_str).context("Failed to parse --config-json as JSON")?;
                // Merge inline JSON with file config
                config = merge_json_into_config(&config, json_value)
                    .context("Failed to merge --config-json with file config")?;
            } else if let Some(base64_str) = config_json_base64 {
                let json_bytes = STANDARD
                    .decode(&base64_str)
                    .context("Failed to decode base64 in --config-json-base64")?;
                let json_str = String::from_utf8(json_bytes).context("Base64-decoded content is not valid UTF-8")?;
                let json_value: serde_json::Value =
                    serde_json::from_str(&json_str).context("Failed to parse decoded --config-json-base64 as JSON")?;
                // Merge inline JSON with file config
                config = merge_json_into_config(&config, json_value)
                    .context("Failed to merge --config-json-base64 with file config")?;
            }

            apply_extraction_overrides(
                &mut config,
                ocr,
                ocr_backend.as_deref(),
                ocr_language.as_deref(),
                force_ocr,
                no_cache,
                chunk,
                chunk_size,
                chunk_overlap,
                chunking_tokenizer.as_deref(),
                quality,
                detect_language,
                output_format,
                content_format,
            );

            if !pdf_password.is_empty() {
                let pdf_opts = config.pdf_options.get_or_insert_with(Default::default);
                pdf_opts.passwords = Some(pdf_password);
            }

            extract_command(path, config, mime_type, format)?;
        }

        Commands::Batch {
            paths,
            config: config_path,
            config_json,
            config_json_base64,
            format,
            ocr,
            ocr_backend,
            ocr_language,
            force_ocr,
            no_cache,
            chunk,
            chunk_size,
            chunk_overlap,
            chunking_tokenizer,
            quality,
            detect_language,
            output_format,
            content_format,
            pdf_password,
            file_configs,
        } => {
            validate_batch_paths(&paths)?;
            validate_chunk_params(chunk_size, chunk_overlap)?;

            let mut config = load_config(config_path)?;

            // Apply inline JSON config if provided (merge with file config)
            if let Some(json_str) = config_json {
                let json_value: serde_json::Value =
                    serde_json::from_str(&json_str).context("Failed to parse --config-json as JSON")?;
                // Merge inline JSON with file config
                config = merge_json_into_config(&config, json_value)
                    .context("Failed to merge --config-json with file config")?;
            } else if let Some(base64_str) = config_json_base64 {
                let json_bytes = STANDARD
                    .decode(&base64_str)
                    .context("Failed to decode base64 in --config-json-base64")?;
                let json_str = String::from_utf8(json_bytes).context("Base64-decoded content is not valid UTF-8")?;
                let json_value: serde_json::Value =
                    serde_json::from_str(&json_str).context("Failed to parse decoded --config-json-base64 as JSON")?;
                // Merge inline JSON with file config
                config = merge_json_into_config(&config, json_value)
                    .context("Failed to merge --config-json-base64 with file config")?;
            }

            apply_extraction_overrides(
                &mut config,
                ocr,
                ocr_backend.as_deref(),
                ocr_language.as_deref(),
                force_ocr,
                no_cache,
                chunk,
                chunk_size,
                chunk_overlap,
                chunking_tokenizer.as_deref(),
                quality,
                detect_language,
                output_format,
                content_format,
            );

            if !pdf_password.is_empty() {
                let pdf_opts = config.pdf_options.get_or_insert_with(Default::default);
                pdf_opts.passwords = Some(pdf_password);
            }

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
            batch_command(paths, file_configs_map, config, format)?;
        }

        Commands::Detect { path, format } => {
            validate_file_exists(&path)?;

            let path_str = path.to_string_lossy().to_string();
            let mime_type = detect_mime_type(&path_str, true).with_context(|| {
                format!(
                    "Failed to detect MIME type for file '{}'. Ensure the file is readable.",
                    path.display()
                )
            })?;

            match format {
                OutputFormat::Text => {
                    println!("{}", mime_type);
                }
                OutputFormat::Json => {
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
            }
        }

        Commands::Formats { format } => {
            let formats = kreuzberg::list_supported_formats();
            match format {
                OutputFormat::Text => {
                    println!("{:<15} MIME TYPE", "EXTENSION");
                    println!("{:<15} ---------", "---------");
                    for f in &formats {
                        println!(".{:<14} {}", f.extension, f.mime_type);
                    }
                }
                OutputFormat::Json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&formats).context("Failed to serialize formats to JSON")?
                    );
                }
            }
        }

        Commands::Version { format } => {
            let version = env!("CARGO_PKG_VERSION");
            let name = env!("CARGO_PKG_NAME");

            match format {
                OutputFormat::Text => {
                    println!("{} {}", name, version);
                }
                OutputFormat::Json => {
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
            }
        }

        #[cfg(feature = "api")]
        Commands::Serve {
            host: cli_host,
            port: cli_port,
            config: config_path,
        } => {
            let extraction_config = load_config(config_path.clone())?;
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
            let config = load_config(config_path)?;
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
            } => {
                warm_command(cache_dir, format, all_embeddings, embedding_model)?;
            }
        },

        #[cfg(feature = "embeddings")]
        Commands::Embed { text, preset, format } => {
            let texts = if text.is_empty() {
                vec![commands::read_stdin()?]
            } else {
                text
            };
            embed_command(texts, &preset, format)?;
        }

        Commands::Chunk {
            text,
            config: config_path,
            chunk_size,
            chunk_overlap,
            chunker_type,
            chunking_tokenizer,
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
                "markdown" => chunking_config.chunker_type = kreuzberg::ChunkerType::Markdown,
                _ => chunking_config.chunker_type = kreuzberg::ChunkerType::Text,
            }
            if let Some(ref tokenizer) = chunking_tokenizer {
                chunking_config.sizing = kreuzberg::ChunkSizing::Tokenizer {
                    model: tokenizer.clone(),
                    cache_dir: None,
                };
            }

            chunk_command(input, chunking_config, format)?;
        }

        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "kreuzberg", &mut std::io::stdout());
        }
    }

    Ok(())
}
