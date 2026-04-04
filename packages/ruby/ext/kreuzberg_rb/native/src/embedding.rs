//! Standalone embedding functions for Ruby.
//!
//! Exposes `embed_sync` and `embed` module functions that generate vector embeddings
//! from a list of text strings using the configured ONNX model.

use crate::error_handling::{kreuzberg_error, runtime_error};
use crate::helpers::ruby_value_to_json;
use magnus::{Error, RArray, RHash, Ruby, Value, scan_args::get_kwargs, scan_args::scan_args};

/// Parse an optional Ruby value (Hash or nil) into a `kreuzberg::EmbeddingConfig`.
fn parse_embedding_config(ruby: &Ruby, config_val: Option<Value>) -> Result<kreuzberg::EmbeddingConfig, Error> {
    match config_val {
        None => Ok(Default::default()),
        Some(val) => {
            if val.equal(ruby.qnil())? {
                return Ok(Default::default());
            }
            let json = ruby_value_to_json(val)?;
            serde_json::from_value(json)
                .map_err(|e| runtime_error(format!("Invalid embedding config: {}", e)))
        }
    }
}

/// Convert `Vec<Vec<f32>>` to a Ruby Array of Arrays of Floats.
fn embeddings_to_ruby(ruby: &Ruby, embeddings: Vec<Vec<f32>>) -> Result<RArray, Error> {
    let outer = ruby.ary_new_capa(embeddings.len());
    for inner_vec in embeddings {
        let inner = ruby.ary_new_capa(inner_vec.len());
        for v in inner_vec {
            inner.push(v as f64)?;
        }
        outer.push(inner)?;
    }
    Ok(outer)
}

/// Parse keyword args common to `embed_sync` and `embed`.
/// Returns `(texts, config)`.
fn parse_embed_args(
    ruby: &Ruby,
    args: &[Value],
) -> Result<(Vec<String>, kreuzberg::EmbeddingConfig), Error> {
    let parsed = scan_args::<(), (), (), (), RHash, ()>(args)?;
    let kw = parsed.keywords;

    let kw_args = get_kwargs::<_, (Value,), (Option<Value>,), ()>(kw, &["texts"], &["config"])?;
    let (texts_val,) = kw_args.required;
    let (config_opt,) = kw_args.optional;

    let texts_arr = RArray::try_convert(texts_val)
        .map_err(|_| runtime_error("texts must be an Array".to_string()))?;
    let texts: Vec<String> = texts_arr
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            String::try_convert(v)
                .map_err(|_| runtime_error(format!("texts[{}] must be a String", i)))
        })
        .collect::<Result<_, _>>()?;

    let config = parse_embedding_config(ruby, config_opt)?;
    Ok((texts, config))
}

/// Generate embeddings synchronously.
///
/// Keyword args: `texts:` (Array of String), `config:` (Hash, optional)
/// Returns: Array of Arrays of Float (one per input text).
pub fn embed_sync(args: &[Value]) -> Result<RArray, Error> {
    let ruby = Ruby::get().expect("Ruby not initialized");
    let (texts, config) = parse_embed_args(&ruby, args)?;
    let embeddings = kreuzberg::embed_texts(&texts, &config).map_err(kreuzberg_error)?;
    embeddings_to_ruby(&ruby, embeddings)
}

/// Generate embeddings (delegates to `embed_sync`).
///
/// Ruby's GVL prevents true async execution, so this simply delegates to
/// the synchronous implementation to avoid creating a throwaway Tokio runtime.
///
/// Keyword args: `texts:` (Array of String), `config:` (Hash, optional)
/// Returns: Array of Arrays of Float (one per input text).
pub fn embed(args: &[Value]) -> Result<RArray, Error> {
    embed_sync(args)
}
