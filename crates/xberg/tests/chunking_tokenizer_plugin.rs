//! Integration tests for consumer-supplied tokenizer backends in token-budgeted chunking.
//!
//! A registered [`TokenizerBackend`] is resolved by name from
//! `ChunkSizing::Tokenizer { model }` before any HuggingFace lookup, so consumers
//! can budget chunks with the exact tokenizer their embedder uses — offline.
#![cfg(feature = "chunking-tokenizers")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use xberg::chunking::chunk_text;
use xberg::plugins::{Plugin, TokenizerBackend, register_tokenizer_backend, unregister_tokenizer_backend};
use xberg::{ChunkSizing, ChunkerType, ChunkingConfig, Result};

/// Deterministic stand-in for a digit-dense tokenizer: every 2 characters cost
/// one token (rounded up). Counts invocations so tests can prove the plugin —
/// not a HuggingFace download — sized the chunks.
struct HalfCharTokenizer {
    name: String,
    calls: AtomicUsize,
}

impl HalfCharTokenizer {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            calls: AtomicUsize::new(0),
        }
    }
}

impl Plugin for HalfCharTokenizer {
    fn name(&self) -> &str {
        &self.name
    }
    fn version(&self) -> String {
        "1.0.0".to_string()
    }
    fn initialize(&self) -> Result<()> {
        Ok(())
    }
    fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

impl TokenizerBackend for HalfCharTokenizer {
    fn count_tokens(&self, text: &str) -> usize {
        self.calls.fetch_add(1, Ordering::Relaxed);
        text.chars().count().div_ceil(2)
    }
}

/// Unique per-test backend name so parallel tests don't collide in the global registry.
fn unique_name(suffix: &str) -> String {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("test-tokenizer-{suffix}-{id}")
}

fn token_sized_config(backend_name: &str, max_tokens: usize) -> ChunkingConfig {
    ChunkingConfig {
        max_characters: max_tokens,
        overlap: 0,
        trim: true,
        chunker_type: ChunkerType::Text,
        sizing: ChunkSizing::Tokenizer {
            model: backend_name.to_string(),
            cache_dir: None,
        },
        ..Default::default()
    }
}

#[test]
fn registered_backend_sizes_chunks_by_its_token_count() {
    let name = unique_name("sizes");
    let backend = Arc::new(HalfCharTokenizer::new(&name));
    register_tokenizer_backend(backend.clone()).unwrap();

    // Digit-dense text: 40 space-separated pairs, ~120 chars. With a 10-token
    // budget at 2 chars/token, a char-interpreted budget of 10 would be far
    // smaller, and an unsized fallback would produce one giant chunk.
    let text = (0..40).map(|i| format!("{:02}", i)).collect::<Vec<_>>().join(" ");
    let config = token_sized_config(&name, 10);

    let result = chunk_text(&text, &config, None).unwrap();

    assert!(
        backend.calls.load(Ordering::Relaxed) > 0,
        "registered tokenizer backend was never invoked"
    );
    assert!(
        result.chunk_count > 1,
        "expected the token budget to split the text, got {} chunk(s)",
        result.chunk_count
    );
    for chunk in &result.chunks {
        let tokens = chunk.content.chars().count().div_ceil(2);
        assert!(
            tokens <= 10,
            "chunk exceeds the 10-token budget ({tokens} tokens): {:?}",
            chunk.content
        );
    }

    unregister_tokenizer_backend(&name).unwrap();
}

#[test]
fn plugin_budget_is_tokens_not_characters() {
    let name = unique_name("units");
    register_tokenizer_backend(Arc::new(HalfCharTokenizer::new(&name))).unwrap();

    // 30 chars of prose. A 16-CHAR budget would split it; a 16-TOKEN budget at
    // 2 chars/token (= 32 chars) must keep it whole.
    let text = "alpha bravo charlie delta echo";
    assert_eq!(text.chars().count(), 30);
    let config = token_sized_config(&name, 16);

    let result = chunk_text(text, &config, None).unwrap();
    assert_eq!(
        result.chunk_count, 1,
        "16-token budget (32 chars at 2 chars/token) must not split 30 chars of text"
    );

    unregister_tokenizer_backend(&name).unwrap();
}

#[test]
fn registered_backend_works_with_markdown_chunker() {
    let name = unique_name("markdown");
    let backend = Arc::new(HalfCharTokenizer::new(&name));
    register_tokenizer_backend(backend.clone()).unwrap();

    let markdown = "# Title\n\n".to_string() + &"word ".repeat(60) + "\n\n## Section\n\n" + &"data ".repeat(60);
    let config = ChunkingConfig {
        chunker_type: ChunkerType::Markdown,
        ..token_sized_config(&name, 25)
    };

    let result = chunk_text(&markdown, &config, None).unwrap();
    assert!(backend.calls.load(Ordering::Relaxed) > 0);
    assert!(result.chunk_count > 1);
    for chunk in &result.chunks {
        assert!(chunk.content.chars().count().div_ceil(2) <= 25);
    }

    unregister_tokenizer_backend(&name).unwrap();
}

#[test]
fn registered_backend_respects_overlap() {
    let name = unique_name("overlap");
    register_tokenizer_backend(Arc::new(HalfCharTokenizer::new(&name))).unwrap();

    let text = "one two three four five six seven eight nine ten eleven twelve";
    let config = ChunkingConfig {
        overlap: 3,
        ..token_sized_config(&name, 8)
    };

    let result = chunk_text(text, &config, None).unwrap();
    assert!(result.chunk_count > 1);
    // Consecutive chunks must overlap in the source text.
    for pair in result.chunks.windows(2) {
        assert!(
            pair[1].metadata.byte_start < pair[0].metadata.byte_end,
            "expected overlapping chunks, got [{}..{}] then [{}..{}]",
            pair[0].metadata.byte_start,
            pair[0].metadata.byte_end,
            pair[1].metadata.byte_start,
            pair[1].metadata.byte_end
        );
    }

    unregister_tokenizer_backend(&name).unwrap();
}

#[test]
fn zero_count_for_nonempty_text_is_clamped_not_trusted() {
    /// Passes the registration probe (single chars count as 1) but reports
    /// zero for anything longer — modeling a backend that starts failing
    /// mid-run (host-language bridges surface exceptions as a zero count).
    struct ZeroAfterProbe {
        name: String,
    }
    impl Plugin for ZeroAfterProbe {
        fn name(&self) -> &str {
            &self.name
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }
    impl TokenizerBackend for ZeroAfterProbe {
        fn count_tokens(&self, text: &str) -> usize {
            usize::from(text.chars().count() == 1)
        }
    }

    let name = unique_name("zero-clamp");
    register_tokenizer_backend(Arc::new(ZeroAfterProbe { name: name.clone() })).unwrap();

    // If zero counts were trusted, every span would appear to fit and the
    // whole text would come back as one chunk. The sizer substitutes the
    // character count instead, so the budget degrades to char semantics:
    // a 12-token budget over this 49-char text must split it.
    let text = "alpha bravo charlie delta echo foxtrot golf hotel";
    let config = token_sized_config(&name, 12);
    let result = chunk_text(text, &config, None).unwrap();
    assert!(
        result.chunk_count > 1,
        "zero token counts must not be trusted (got a single {}-char chunk)",
        result.chunks[0].content.len()
    );
    for chunk in &result.chunks {
        assert!(
            chunk.content.chars().count() <= 12,
            "fallback must budget by characters, got {}-char chunk",
            chunk.content.chars().count()
        );
    }

    unregister_tokenizer_backend(&name).unwrap();
}

#[test]
fn unregistered_backend_is_not_consulted_after_removal() {
    let name = unique_name("removed");
    let backend = Arc::new(HalfCharTokenizer::new(&name));
    register_tokenizer_backend(backend.clone()).unwrap();
    unregister_tokenizer_backend(&name).unwrap();
    // Registration itself probes count_tokens once; chunking after removal
    // must not add to that.
    let calls_after_removal = backend.calls.load(Ordering::Relaxed);

    // The name no longer resolves in the registry; chunking falls through to
    // the HuggingFace path, which cannot load this name and must error rather
    // than silently using the removed backend.
    let text = "some text to chunk";
    let config = token_sized_config(&name, 10);
    let result = chunk_text(text, &config, None);

    assert_eq!(
        backend.calls.load(Ordering::Relaxed),
        calls_after_removal,
        "removed backend must not be invoked by chunking"
    );
    let err = result.expect_err("chunking must fail when the tokenizer name resolves nowhere");
    let message = err.to_string();
    assert!(
        message.contains(&name),
        "error must name the tokenizer that failed to resolve: {message}"
    );
}
