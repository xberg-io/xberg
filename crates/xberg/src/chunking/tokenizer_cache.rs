//! In-memory cache for HuggingFace tokenizers.
//!
//! Tokenizers are cached in-memory for subsequent calls.  xberg ships no
//! bundled tokenizer — callers supply the tokenizer via [`TokenizerSource`]:
//!
//! - [`TokenizerSource::Pretrained`] — resolved via HuggingFace Hub (network).
//! - [`TokenizerSource::File`]       — loaded from a local `tokenizer.json` path.
//! - [`TokenizerSource::Bytes`]      — raw `tokenizer.json` bytes supplied by the
//!   caller (e.g. `include_bytes!` in their binary).  This is the primary path for
//!   offline embedders.
//!
//! The backwards-compatible [`count_tokens`] function accepts an optional model ID
//! string and routes to [`TokenizerSource::Pretrained`], defaulting to
//! [`DEFAULT_COUNT_TOKENS_MODEL`].

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash as _, Hasher as _};
use std::sync::{Arc, LazyLock, RwLock};

use ahash::AHashMap;

use crate::XbergError;

/// Default model used by [`count_tokens`] when no model is specified.
///
/// `Xenova/gpt-4o` encodes with the o200k_base vocabulary, making it the closest
/// widely-available HuggingFace proxy for GPT-4o and Claude token counts.
pub const DEFAULT_COUNT_TOKENS_MODEL: &str = "Xenova/gpt-4o";

/// Source from which a tokenizer is loaded.
///
/// Pass to [`try_count_tokens`] or [`preload_tokenizer`] to supply the tokenizer
/// without relying on the HuggingFace Hub.
///
/// # FFI / bindings note
///
/// This type is marked `#[cfg_attr(alef, alef(skip))]` on the functions that use
/// it — it is a Rust-only abstraction and is not surfaced in language bindings.
pub enum TokenizerSource<'a> {
    /// HuggingFace model ID — resolved via [`tokenizers::Tokenizer::from_pretrained`]
    /// (requires network access on the first call; result is disk-cached by `hf-hub`).
    Pretrained(&'a str),

    /// Path to a local `tokenizer.json` file.
    File(&'a std::path::Path),

    /// Raw `tokenizer.json` bytes (caller-embedded — no temp file or network needed).
    ///
    /// This is the primary path for offline embedders: embed the JSON in your binary
    /// via `include_bytes!` and pass a reference here.
    Bytes(&'a [u8]),
}

/// Cache key discriminator for [`TokenizerSource`] variants.
///
/// We need a stable string key so all three source types can share one
/// [`AHashMap`].  The discriminant prefix (`pretrained:`, `file:`, `bytes:`)
/// prevents collisions across source kinds.
fn cache_key(source: &TokenizerSource<'_>) -> String {
    match source {
        TokenizerSource::Pretrained(model) => format!("pretrained:{model}"),
        TokenizerSource::File(path) => format!("file:{}", path.display()),
        TokenizerSource::Bytes(b) => {
            // Hash the bytes with std's DefaultHasher.  We only need cache
            // identity within a process lifetime — cryptographic strength is
            // not required.
            let mut h = DefaultHasher::new();
            b.hash(&mut h);
            format!("bytes:{:016x}", h.finish())
        }
    }
}

/// Global in-memory cache for loaded tokenizers.
///
/// Keyed by the string produced by [`cache_key`].  Once a tokenizer is parsed
/// it is stored here to avoid re-parsing on subsequent calls.
static TOKENIZER_CACHE: LazyLock<RwLock<AHashMap<String, Arc<tokenizers::Tokenizer>>>> =
    LazyLock::new(|| RwLock::new(AHashMap::new()));

/// Load a tokenizer from `source` without consulting the cache.
fn load_tokenizer(source: &TokenizerSource<'_>) -> crate::Result<tokenizers::Tokenizer> {
    match source {
        // `from_pretrained` requires the tokenizers `http` feature (network fetch via hf-hub),
        // which is unavailable on wasm32. File/Bytes sources remain supported.
        #[cfg(not(target_arch = "wasm32"))]
        TokenizerSource::Pretrained(model) => tokenizers::Tokenizer::from_pretrained(model, None)
            .map_err(|e| XbergError::validation(format!("Failed to load tokenizer '{model}': {e}"))),
        #[cfg(target_arch = "wasm32")]
        TokenizerSource::Pretrained(model) => Err(XbergError::validation(format!(
            "Pretrained tokenizer loading is not supported on wasm32: {model}"
        ))),
        TokenizerSource::File(path) => tokenizers::Tokenizer::from_file(path)
            .map_err(|e| XbergError::validation(format!("Failed to load tokenizer from '{}': {e}", path.display()))),
        TokenizerSource::Bytes(b) => tokenizers::Tokenizer::from_bytes(b)
            .map_err(|e| XbergError::validation(format!("Failed to parse tokenizer from bytes: {e}"))),
    }
}

/// Get a cached tokenizer, or load and cache it on the first call.
///
/// Uses a two-phase locking strategy (read lock first, write lock on miss)
/// following the same pattern as the embeddings model cache in `embeddings.rs`.
///
/// # Arguments
///
/// * `source` - Where to load the tokenizer from (see [`TokenizerSource`]).
///
/// # Errors
///
/// Returns an error if the tokenizer cannot be loaded or parsed.
pub(crate) fn get_or_init_tokenizer_from_source(
    source: &TokenizerSource<'_>,
) -> crate::Result<Arc<tokenizers::Tokenizer>> {
    let key = cache_key(source);

    // Phase 1: try read lock (fast path for cache hits)
    {
        let cache = TOKENIZER_CACHE
            .read()
            .map_err(|e| XbergError::Other(format!("Tokenizer cache read lock poisoned: {e}")))?;
        if let Some(tok) = cache.get(&key) {
            return Ok(Arc::clone(tok));
        }
    }

    // Phase 2: write lock, double-check, then load
    let mut cache = TOKENIZER_CACHE
        .write()
        .map_err(|e| XbergError::Other(format!("Tokenizer cache write lock poisoned: {e}")))?;

    // Double-check after acquiring write lock (another thread may have loaded)
    if let Some(tok) = cache.get(&key) {
        return Ok(Arc::clone(tok));
    }

    let tokenizer = load_tokenizer(source)?;
    let arc = Arc::new(tokenizer);
    cache.insert(key, Arc::clone(&arc));
    Ok(arc)
}

/// Backwards-compatible helper: get a tokenizer by HuggingFace model ID.
///
/// Routes to [`TokenizerSource::Pretrained`].
pub(crate) fn get_or_init_tokenizer(model: &str) -> crate::Result<Arc<tokenizers::Tokenizer>> {
    get_or_init_tokenizer_from_source(&TokenizerSource::Pretrained(model))
}

/// Count the number of tokens in `text` for the given HuggingFace tokenizer model.
///
/// Reuses the global in-memory tokenizer cache — the tokenizer is downloaded and
/// parsed only on the first call for each model, then served from memory for all
/// subsequent calls.  File-level caching is handled by the `hf-hub` crate (defaults
/// to `~/.cache/huggingface/`).
///
/// # Arguments
///
/// * `text`  - The text to tokenize.
/// * `model` - HuggingFace model ID to use for tokenization.  Pass `None` to use
///   the default model ([`DEFAULT_COUNT_TOKENS_MODEL`], `"Xenova/gpt-4o"`), which
///   encodes with the o200k_base vocabulary — the closest widely-available proxy
///   for GPT-4o and Claude token counts.
///
/// # Returns
///
/// The number of tokens produced by the tokenizer.  If the tokenizer cannot be
/// loaded (e.g. network unavailable, model not found), falls back to a
/// whitespace-split heuristic so the function never panics or propagates an error.
///
/// # Example
///
/// ```rust,no_run
/// use xberg::chunking::count_tokens;
///
/// let n = count_tokens("Hello, world!", None);
/// assert!(n > 0);
///
/// let n_gpt4 = count_tokens("Hello, world!", Some("Xenova/gpt-4o"));
/// assert!(n_gpt4 > 0);
/// ```
///
/// # Note
///
/// This function is intentionally excluded from language bindings (alef-skipped) —
/// it is a Rust-only utility that relies on the cached tokenizer infrastructure.
#[cfg_attr(alef, alef(skip))]
pub fn count_tokens(text: &str, model: Option<&str>) -> usize {
    let model = model.unwrap_or(DEFAULT_COUNT_TOKENS_MODEL);
    match get_or_init_tokenizer(model) {
        Ok(tokenizer) => match tokenizer.encode(text, false) {
            Ok(encoding) => encoding.len(),
            Err(_) => whitespace_token_estimate(text),
        },
        Err(_) => whitespace_token_estimate(text),
    }
}

/// Tokenize `text` and return the token count, surfacing any load or encode error.
///
/// Unlike [`count_tokens`], this function propagates errors instead of falling back
/// to the whitespace heuristic.  Use it when you need to distinguish a genuine
/// tokenization result from a degraded fallback.
///
/// The caller supplies the tokenizer via [`TokenizerSource`]:
/// - `TokenizerSource::Bytes(bytes)` — offline, no network (primary path for embedders).
/// - `TokenizerSource::File(path)` — from a local file.
/// - `TokenizerSource::Pretrained(model)` — from HuggingFace Hub (requires network).
///
/// # Arguments
///
/// * `text`   - The text to tokenize.
/// * `source` - Where to load the tokenizer from.
///
/// # Errors
///
/// Returns an error if the tokenizer cannot be loaded or encoding fails.
///
/// # Example
///
/// ```rust,ignore
/// // Embed a tokenizer.json from your own crate (offline, no network).
/// use xberg::chunking::{try_count_tokens, TokenizerSource};
///
/// let bytes: &[u8] = include_bytes!("path/to/tokenizer.json");
/// let n = try_count_tokens("Hello, world!", TokenizerSource::Bytes(bytes)).unwrap();
/// assert!(n > 0);
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn try_count_tokens(text: &str, source: TokenizerSource<'_>) -> crate::Result<usize> {
    let tok = get_or_init_tokenizer_from_source(&source)?;
    tok.encode(text, false)
        .map(|e| e.len())
        .map_err(|e| XbergError::Other(format!("encode: {e}")))
}

/// Pre-warm the tokenizer cache for the given source.
///
/// Call this at application startup to eliminate first-call latency.  For
/// `TokenizerSource::Bytes` and `TokenizerSource::File` the tokenizer is parsed
/// once and stored in the in-process cache.  For `TokenizerSource::Pretrained` it
/// also triggers the network download from HuggingFace Hub.
///
/// # Arguments
///
/// * `source` - Where to load the tokenizer from.
///
/// # Errors
///
/// Returns an error if the tokenizer cannot be loaded or parsed.
///
/// # Example
///
/// ```rust,ignore
/// // Embed a tokenizer.json from your own crate (offline, no network).
/// use xberg::chunking::{preload_tokenizer, TokenizerSource};
///
/// let bytes: &[u8] = include_bytes!("path/to/tokenizer.json");
/// preload_tokenizer(TokenizerSource::Bytes(bytes)).unwrap();
/// ```
#[cfg_attr(alef, alef(skip))]
pub fn preload_tokenizer(source: TokenizerSource<'_>) -> crate::Result<()> {
    get_or_init_tokenizer_from_source(&source).map(|_| ())
}

/// Heuristic fallback: split on whitespace and count non-empty tokens.
///
/// Used when the HuggingFace tokenizer cannot be loaded.
fn whitespace_token_estimate(text: &str) -> usize {
    text.split_whitespace().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_returns_same_instance() {
        // This test requires network access to download a tokenizer.
        // Skip in CI by checking for a specific env var.
        if std::env::var("CI").is_ok() {
            return;
        }

        let model = "bert-base-uncased";
        let tok1 = get_or_init_tokenizer(model).unwrap();
        let tok2 = get_or_init_tokenizer(model).unwrap();

        // Same Arc instance (pointer equality)
        assert!(Arc::ptr_eq(&tok1, &tok2));
    }

    /// Verify that `count_tokens` returns a meaningful non-zero count and that the
    /// `None`-model path resolves to [`DEFAULT_COUNT_TOKENS_MODEL`].
    ///
    /// This test requires network access on the first run to download the tokenizer.
    /// Skipped in CI (no network) via the `CI` environment variable.
    #[test]
    fn test_count_tokens_none_defaults_to_gpt4o_and_returns_nonzero() {
        if std::env::var("CI").is_ok() {
            return;
        }

        let text = "Hello, world! This is a test sentence for token counting.";

        // None resolves to DEFAULT_COUNT_TOKENS_MODEL ("Xenova/gpt-4o")
        let count_via_none = count_tokens(text, None);
        assert!(
            count_via_none > 0,
            "count_tokens(text, None) must return a non-zero count"
        );

        // Explicit model must return the same value (same model, same cache entry)
        let count_via_explicit = count_tokens(text, Some(DEFAULT_COUNT_TOKENS_MODEL));
        assert_eq!(
            count_via_none, count_via_explicit,
            "None and Some(DEFAULT_COUNT_TOKENS_MODEL) must produce the same count"
        );
    }

    /// Verify that the fallback heuristic (whitespace split) is used when the
    /// tokenizer cannot be loaded (bogus model id), and that the function never
    /// panics or returns zero for non-empty text.
    #[test]
    fn test_count_tokens_falls_back_gracefully_on_invalid_model() {
        let text = "six distinct whitespace separated words here";
        // An obviously invalid model ID — `get_or_init_tokenizer` will fail.
        let count = count_tokens(text, Some("__invalid_model_that_does_not_exist__"));
        // Fallback: split_whitespace gives 6 tokens for the sentence above.
        assert_eq!(count, 6, "fallback whitespace estimator should count 6 words");
    }

    /// Ensure the whitespace heuristic itself handles edge cases.
    #[test]
    fn test_whitespace_token_estimate_edge_cases() {
        assert_eq!(whitespace_token_estimate(""), 0);
        assert_eq!(whitespace_token_estimate("   "), 0);
        assert_eq!(whitespace_token_estimate("one"), 1);
        assert_eq!(whitespace_token_estimate("one two three"), 3);
    }

    // ── Offline tests (no network required) ──────────────────────────────────
    //
    // These tests use the bert-base-uncased tokenizer.json bundled as a test
    // fixture under testdata/.  The bert WordPiece tokenizer is much smaller
    // than gpt-4o (~466 KB vs 2.9 MB) and was already present in the HuggingFace
    // disk cache, making it suitable as a lightweight offline fixture.

    const BERT_TOKENIZER_BYTES: &[u8] = include_bytes!("testdata/bert-base-uncased.tokenizer.json");

    /// Verify `TokenizerSource::Bytes` parses a real tokenizer.json without network.
    #[test]
    fn test_bytes_source_parses_offline() {
        let source = TokenizerSource::Bytes(BERT_TOKENIZER_BYTES);
        let tok =
            get_or_init_tokenizer_from_source(&source).expect("Bytes source must parse bert tokenizer without network");
        assert!(
            tok.get_vocab_size(true) > 1000,
            "expected a non-trivial vocabulary, got {}",
            tok.get_vocab_size(true)
        );
    }

    /// Verify `try_count_tokens` with `Bytes` source returns a deterministic count.
    ///
    /// bert-base-uncased tokenizes "Hello, world!" as:
    ///   [CLS] hello , world ! [SEP] — but with `add_special_tokens = false`
    ///   (which is what we pass) only: hello , world ! → 4 tokens.
    #[test]
    fn test_try_count_tokens_bytes_source_deterministic() {
        let n = try_count_tokens("Hello, world!", TokenizerSource::Bytes(BERT_TOKENIZER_BYTES))
            .expect("try_count_tokens with Bytes must not fail");
        // bert WordPiece without special tokens: "hello", ",", "world", "!" = 4
        assert_eq!(n, 4, "expected 4 tokens for 'Hello, world!' via bert WordPiece");
    }

    /// Verify `try_count_tokens` with `Bytes` source caches: second call returns same
    /// Arc (ptr equality via the cache).
    #[test]
    fn test_bytes_source_cache_hit() {
        let tok1 = get_or_init_tokenizer_from_source(&TokenizerSource::Bytes(BERT_TOKENIZER_BYTES))
            .expect("first call must succeed");
        let tok2 = get_or_init_tokenizer_from_source(&TokenizerSource::Bytes(BERT_TOKENIZER_BYTES))
            .expect("second call must succeed");
        assert!(Arc::ptr_eq(&tok1, &tok2), "second call must return cached Arc");
    }

    /// Verify `TokenizerSource::File` loads from a temp file written from the fixture bytes.
    #[test]
    fn test_file_source_loads_offline() {
        use std::io::Write as _;
        let mut tmp = tempfile::NamedTempFile::new().expect("create tempfile");
        tmp.write_all(BERT_TOKENIZER_BYTES).expect("write tokenizer bytes");
        let path = tmp.path();

        let n = try_count_tokens("Hello, world!", TokenizerSource::File(path))
            .expect("try_count_tokens with File must not fail");
        assert_eq!(n, 4, "File source must produce the same count as Bytes source");
    }

    /// Verify `preload_tokenizer` with `Bytes` source succeeds offline.
    #[test]
    fn test_preload_tokenizer_bytes_offline() {
        preload_tokenizer(TokenizerSource::Bytes(BERT_TOKENIZER_BYTES))
            .expect("preload_tokenizer(Bytes) must succeed offline");
    }

    /// Verify `try_count_tokens` with `Pretrained` source for an invalid model surfaces an error.
    #[test]
    fn test_try_count_tokens_pretrained_invalid_model_errors() {
        let result = try_count_tokens("some text", TokenizerSource::Pretrained("__invalid_model__"));
        assert!(
            result.is_err(),
            "try_count_tokens must surface errors for invalid Pretrained model"
        );
    }

    /// Verify `count_tokens` back-compat: `None` and `Some(DEFAULT_COUNT_TOKENS_MODEL)` both
    /// fall back to whitespace when offline (no cached tokenizer for gpt-4o in unit tests).
    #[test]
    fn test_count_tokens_backcmpat_fallback_when_offline() {
        // In a unit-test environment with no HF disk cache for Xenova/gpt-4o and CI=false,
        // count_tokens falls back to whitespace.  We just assert it is non-zero and doesn't panic.
        let text = "hello world test";
        let n = count_tokens(text, None);
        assert!(
            n > 0,
            "count_tokens must return > 0 for non-empty text (whitespace fallback)"
        );
    }

    /// Verify `cache_key` produces distinct keys across source kinds.
    #[test]
    fn test_cache_key_discriminant() {
        let k_pretrained = cache_key(&TokenizerSource::Pretrained("model-a"));
        let k_file = cache_key(&TokenizerSource::File(std::path::Path::new("model-a")));
        let k_bytes = cache_key(&TokenizerSource::Bytes(b"model-a"));

        assert_ne!(k_pretrained, k_file);
        assert_ne!(k_pretrained, k_bytes);
        assert_ne!(k_file, k_bytes);

        // Same value, same source → same key
        assert_eq!(
            cache_key(&TokenizerSource::Pretrained("model-a")),
            cache_key(&TokenizerSource::Pretrained("model-a"))
        );
        assert_eq!(
            cache_key(&TokenizerSource::Bytes(b"abc")),
            cache_key(&TokenizerSource::Bytes(b"abc"))
        );
    }
}
