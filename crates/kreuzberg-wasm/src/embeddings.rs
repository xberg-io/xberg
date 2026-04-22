//! Embedding preset utilities for WASM bindings
//!
//! This module provides functions for accessing and managing text embedding presets
//! in WebAssembly environments. Presets provide pre-configured models optimized for
//! different use cases (fast, balanced, quality, multilingual).

#[cfg(feature = "embeddings")]
use crate::errors::convert_error;
#[cfg(feature = "embeddings")]
use js_sys::Array;
#[cfg(feature = "embeddings")]
use kreuzberg::{EmbeddingConfig, embed_texts, embed_texts_async, utils::camel_to_snake};
#[cfg(feature = "embeddings")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "embeddings")]
fn parse_embedding_config(config: Option<JsValue>) -> Result<EmbeddingConfig, JsValue> {
    if let Some(js_config) = config.filter(|c| !c.is_null() && !c.is_undefined()) {
        let json_value: serde_json::Value = serde_wasm_bindgen::from_value(js_config)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse embedding config: {e}")))?;
        let snake_value = camel_to_snake(json_value);
        return serde_json::from_value(snake_value)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse embedding config: {e}")));
    }
    Ok(EmbeddingConfig::default())
}

/// List all available embedding preset names.
///
/// Returns a JavaScript Array of all available preset names. Each preset provides
/// a pre-configured model optimized for specific use cases. Current presets include:
/// - "fast": Quick prototyping with quantized models
/// - "balanced": Production-ready general-purpose embeddings
/// - "quality": High-quality embeddings with larger models
/// - "multilingual": Support for multiple languages
///
/// # JavaScript Parameters
///
/// None
///
/// # Returns
///
/// `string[]` - Array of available preset names
///
/// # Example
///
/// ```javascript
/// import { listEmbeddingPresets } from '@kreuzberg/wasm';
///
/// const presets = listEmbeddingPresets();
/// console.log(presets); // ["fast", "balanced", "quality", "multilingual"]
///
/// for (const preset of presets) {
///   console.log(`Available preset: ${preset}`);
/// }
/// ```
#[cfg(feature = "embeddings")]
#[wasm_bindgen(js_name = listEmbeddingPresets)]
pub fn list_embedding_presets() -> Array {
    let presets = kreuzberg::list_presets();
    let array = Array::new();

    for preset_name in presets {
        array.push(&JsValue::from_str(preset_name));
    }

    array
}

/// Get details about a specific embedding preset.
///
/// Retrieves configuration details for a named embedding preset, including
/// the model information, chunk size, overlap, dimensions, and description.
///
/// Returns None if the preset name is not found.
///
/// # JavaScript Parameters
///
/// * `name: string` - The name of the preset (e.g., "balanced", "fast")
///
/// # Returns
///
/// `object | null` - Preset object with properties or null if not found
///
/// The returned object has the following properties:
/// - `name: string` - Preset name
/// - `chunkSize: number` - Recommended chunk size in characters
/// - `overlap: number` - Recommended overlap between chunks in characters
/// - `dimensions: number` - Embedding vector dimensions
/// - `modelName: string` - Name of the embedding model
/// - `description: string` - Human-readable description of the preset
///
/// # Throws
///
/// Does not throw; returns null for unknown presets.
///
/// # Example
///
/// ```javascript
/// import { getEmbeddingPreset } from '@kreuzberg/wasm';
///
/// // Get balanced preset (general-purpose)
/// const balanced = getEmbeddingPreset('balanced');
/// if (balanced) {
///   console.log(balanced.name);        // "balanced"
///   console.log(balanced.dimensions);  // 768
///   console.log(balanced.chunkSize);   // 1024
///   console.log(balanced.overlap);     // 100
///   console.log(balanced.description); // "Balanced quality and speed..."
/// }
///
/// // Get fast preset (development)
/// const fast = getEmbeddingPreset('fast');
/// if (fast) {
///   console.log(fast.chunkSize);  // 512
///   console.log(fast.dimensions); // 384
/// }
///
/// // Preset not found
/// const unknown = getEmbeddingPreset('nonexistent');
/// console.log(unknown); // null
/// ```
#[cfg(feature = "embeddings")]
#[wasm_bindgen(js_name = getEmbeddingPreset)]
pub fn get_embedding_preset(name: String) -> Option<JsValue> {
    let preset = kreuzberg::get_preset(&name)?;

    let obj = js_sys::Object::new();

    js_sys::Reflect::set(&obj, &"name".into(), &preset.name.into()).ok()?;
    js_sys::Reflect::set(&obj, &"chunkSize".into(), &preset.chunk_size.into()).ok()?;
    js_sys::Reflect::set(&obj, &"overlap".into(), &preset.overlap.into()).ok()?;
    js_sys::Reflect::set(&obj, &"dimensions".into(), &preset.dimensions.into()).ok()?;

    js_sys::Reflect::set(&obj, &"modelRepo".into(), &preset.model_repo.into()).ok()?;
    js_sys::Reflect::set(&obj, &"modelFile".into(), &preset.model_file.into()).ok()?;

    js_sys::Reflect::set(&obj, &"description".into(), &preset.description.into()).ok()?;

    Some(obj.into())
}

/// Generate embeddings for an array of texts synchronously.
///
/// Computes embedding vectors for each input text using the specified model
/// configuration. This is a blocking operation and should only be used when
/// async is not available.
///
/// # JavaScript Parameters
///
/// * `texts: string[]` - Array of text strings to embed
/// * `config?: object` - Optional embedding configuration object with properties:
///   - `modelRepo?: string` - Hugging Face model repository
///   - `modelFile?: string` - ONNX model file name
///   - `chunkSize?: number` - Characters per chunk
///   - `overlap?: number` - Character overlap between chunks
///
/// # Returns
///
/// `number[][]` - Array of embedding vectors, one per input text
///
/// # Throws
///
/// Throws if the model cannot be loaded or embedding computation fails.
///
/// # Example
///
/// ```javascript
/// import { embedTextsSync } from '@kreuzberg/wasm';
///
/// const texts = ['Hello world', 'How are you?'];
/// const embeddings = embedTextsSync(texts);
/// console.log(embeddings.length); // 2
/// console.log(embeddings[0].length); // vector dimensions
///
/// // With custom config
/// const custom = embedTextsSync(texts, { chunkSize: 512 });
/// ```
#[cfg(feature = "embeddings")]
#[wasm_bindgen(js_name = embedTextsSync)]
pub fn embed_texts_sync(texts: Vec<String>, config: Option<JsValue>) -> Result<JsValue, JsValue> {
    let embed_cfg = parse_embedding_config(config)?;
    let results = embed_texts(&texts, &embed_cfg).map_err(convert_error)?;
    serde_wasm_bindgen::to_value(&results)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize embeddings: {}", e)))
}

/// Generate embeddings for an array of texts asynchronously.
///
/// Computes embedding vectors for each input text using the specified model
/// configuration. Returns a Promise that resolves with the embedding results.
/// This is the preferred method for WASM environments as it does not block
/// the main thread.
///
/// # JavaScript Parameters
///
/// * `texts: string[]` - Array of text strings to embed
/// * `config?: object` - Optional embedding configuration object with properties:
///   - `modelRepo?: string` - Hugging Face model repository
///   - `modelFile?: string` - ONNX model file name
///   - `chunkSize?: number` - Characters per chunk
///   - `overlap?: number` - Character overlap between chunks
///
/// # Returns
///
/// `Promise<number[][]>` - Promise resolving to array of embedding vectors
///
/// # Throws
///
/// Rejects if the model cannot be loaded or embedding computation fails.
///
/// # Example
///
/// ```javascript
/// import { embedTexts } from '@kreuzberg/wasm';
///
/// const texts = ['Hello world', 'How are you?'];
/// const embeddings = await embedTexts(texts);
/// console.log(embeddings.length); // 2
///
/// // With custom config
/// const custom = await embedTexts(texts, { chunkSize: 512 });
/// ```
#[cfg(feature = "embeddings")]
#[wasm_bindgen(js_name = embedTexts)]
pub fn embed_texts_async_wasm(texts: Vec<String>, config: Option<JsValue>) -> js_sys::Promise {
    wasm_bindgen_futures::future_to_promise(async move {
        let embed_cfg = parse_embedding_config(config)?;
        let results = embed_texts_async(texts, &embed_cfg).await.map_err(convert_error)?;
        serde_wasm_bindgen::to_value(&results)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize embeddings: {}", e)))
    })
}

#[cfg(all(test, feature = "embeddings"))]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_list_embedding_presets() {
        let presets = list_embedding_presets();
        assert!(presets.length() > 0);
    }

    #[test]
    #[cfg(target_arch = "wasm32")]
    fn test_get_embedding_preset_valid() {
        let preset = get_embedding_preset("balanced".to_string());
        assert!(preset.is_some());
    }

    #[test]
    fn test_get_embedding_preset_invalid() {
        let preset = get_embedding_preset("nonexistent".to_string());
        assert!(preset.is_none());
    }
}
