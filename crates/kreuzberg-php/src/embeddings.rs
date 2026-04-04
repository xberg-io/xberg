//! Embedding preset functions for PHP bindings
//!
//! Provides functions to list and retrieve embedding model presets.

use ext_php_rs::prelude::*;
use kreuzberg::{EmbeddingConfig, embed_texts};

/// Embedding preset configuration.
///
/// Contains all settings for a specific embedding model preset including chunk size,
/// overlap, model name, embedding dimensions, and description.
///
/// # Properties
///
/// - `name` (string): Name of the preset
/// - `chunk_size` (int): Recommended chunk size in characters
/// - `overlap` (int): Recommended overlap in characters
/// - `model_name` (string): Model identifier
/// - `dimensions` (int): Embedding vector dimensions
/// - `description` (string): Human-readable description
///
/// # Example
///
/// ```php
/// $preset = kreuzberg_get_embedding_preset("balanced");
/// echo "Model: {$preset->model_name}, Dims: {$preset->dimensions}\n";
/// ```
#[php_class]
#[php(name = "Kreuzberg\\Embeddings\\EmbeddingPreset")]
#[derive(Clone)]
pub struct EmbeddingPreset {
    #[php(prop)]
    pub name: String,
    #[php(prop)]
    pub chunk_size: i64,
    #[php(prop)]
    pub overlap: i64,
    #[php(prop)]
    pub model_name: String,
    #[php(prop)]
    pub dimensions: i64,
    #[php(prop)]
    pub description: String,
}

#[php_impl]
impl EmbeddingPreset {}

/// List all available embedding preset names.
///
/// Returns an array of preset names that can be used with kreuzberg_get_embedding_preset().
///
/// # Returns
///
/// Array of preset names
///
/// # Available Presets
///
/// - "fast": AllMiniLML6V2Q (384 dimensions) - Quick prototyping, low-latency
/// - "balanced": BGEBaseENV15 (768 dimensions) - General-purpose RAG
/// - "quality": BGELargeENV15 (1024 dimensions) - High-quality embeddings
/// - "multilingual": MultilingualE5Base (768 dimensions) - Multi-language support
///
/// # Example
///
/// ```php
/// $presets = kreuzberg_list_embedding_presets();
/// print_r($presets); // ["fast", "balanced", "quality", "multilingual"]
/// ```
#[php_function]
pub fn kreuzberg_list_embedding_presets() -> Vec<String> {
    kreuzberg::embeddings::list_presets()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}

/// Get a specific embedding preset by name.
///
/// Returns a preset configuration object, or NULL if the preset name is not found.
///
/// # Parameters
///
/// - `name` (string): The preset name (case-sensitive)
///
/// # Returns
///
/// EmbeddingPreset object or NULL if not found
///
/// # Example
///
/// ```php
/// $preset = kreuzberg_get_embedding_preset("balanced");
/// if ($preset !== null) {
///     echo "Model: {$preset->model_name}\n";
///     echo "Dimensions: {$preset->dimensions}\n";
///     echo "Chunk size: {$preset->chunk_size}\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_get_embedding_preset(name: String) -> Option<EmbeddingPreset> {
    let preset = kreuzberg::embeddings::get_preset(&name)?;

    let model_name = preset.model_repo.to_string();

    Some(EmbeddingPreset {
        name: preset.name.to_string(),
        chunk_size: preset.chunk_size as i64,
        overlap: preset.overlap as i64,
        model_name,
        dimensions: preset.dimensions as i64,
        description: preset.description.to_string(),
    })
}

/// Generate text embeddings for a list of strings.
///
/// Returns a 2D array (list of lists) of floating point numbers representing
/// the embedding vectors for each input string.
///
/// # Parameters
///
/// - `texts` (array<string>): List of strings to embed
/// - `config_json` (string|null): Optional JSON-encoded EmbeddingConfig
///
/// # Returns
///
/// Array of float arrays (2D array)
///
/// # Throws
///
/// KreuzbergException if generation fails
///
/// # Example
///
/// ```php
/// $embeddings = kreuzberg_embed(["hello", "world"], null);
/// foreach ($embeddings as $vector) {
///     echo "Dimensions: " . count($vector) . "\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_embed(texts: Vec<String>, config_json: Option<String>) -> PhpResult<Vec<Vec<f32>>> {
    let config = match config_json {
        Some(json) => serde_json::from_str::<EmbeddingConfig>(&json)
            .map_err(|e| PhpException::default(format!("Failed to parse config: {e}")))?,
        None => EmbeddingConfig::default(),
    };

    embed_texts(&texts, &config).map_err(|e| PhpException::default(format!("[Embedding] {e}")))
}

/// Generate text embeddings asynchronously.
///
/// Runs the embedding computation on the background Tokio worker pool.
/// Blocks the calling PHP thread until the result is ready.
///
/// # Parameters
///
/// - `texts` (array<string>): List of strings to embed
/// - `config_json` (string|null): Optional JSON-encoded EmbeddingConfig
///
/// # Returns
///
/// Array of float arrays (2D array)
///
/// # Throws
///
/// KreuzbergException if generation fails
///
/// # Example
///
/// ```php
/// $embeddings = kreuzberg_embed_async(["test"], null);
/// foreach ($embeddings as $vector) {
///     echo "Dimensions: " . count($vector) . "\n";
/// }
/// ```
#[php_function]
pub fn kreuzberg_embed_async(texts: Vec<String>, config_json: Option<String>) -> PhpResult<Vec<Vec<f32>>> {
    let config = match config_json {
        Some(json) => serde_json::from_str::<EmbeddingConfig>(&json)
            .map_err(|e| PhpException::default(format!("Failed to parse config: {e}")))?,
        None => EmbeddingConfig::default(),
    };

    let runtime = crate::worker_runtime()?;
    runtime
        .block_on(async { kreuzberg::embed_texts_async(texts, &config).await })
        .map_err(|e| PhpException::default(format!("[Embedding] {e}")))
}

/// Returns all function builders for the embeddings module.
pub fn get_function_builders() -> Vec<ext_php_rs::builders::FunctionBuilder<'static>> {
    vec![
        wrap_function!(kreuzberg_list_embedding_presets),
        wrap_function!(kreuzberg_get_embedding_preset),
        wrap_function!(kreuzberg_embed),
        wrap_function!(kreuzberg_embed_async),
    ]
}
