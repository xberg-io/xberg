//! Live model-cache eviction tests.
//!
//! These tests download real presets from Hugging Face, evict them through the
//! public API, and check that the caches release them and load them again on
//! the next call. They print the resident set size around each step so a
//! reviewer can see the memory move. Set `XBERG_SKIP_LIVE_HF=1` to skip them.

#![allow(clippy::print_stdout, clippy::print_stderr)] // ~keep: test binaries print by design; org logging policy exempts tests
#![cfg(feature = "embeddings")]

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};

use xberg::core::config::{EmbeddingConfig, EmbeddingModelType};
use xberg::embeddings;

/// The caches are process-wide, so the tests in this binary run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

fn should_skip() -> bool {
    if std::env::var("XBERG_SKIP_LIVE_HF").ok().as_deref() == Some("1") {
        eprintln!("XBERG_SKIP_LIVE_HF=1, skipping");
        return true;
    }
    false
}

fn cache_dir() -> Option<PathBuf> {
    std::env::var("XBERG_HF_CACHE_DIR").ok().map(PathBuf::from)
}

fn preset(name: &str) -> EmbeddingModelType {
    EmbeddingModelType::Preset { name: name.to_string() }
}

fn embed(name: &str) {
    let config = EmbeddingConfig {
        model: preset(name),
        cache_dir: cache_dir(),
        ..EmbeddingConfig::default()
    };
    let vectors = xberg::embed_texts(vec!["hello".to_string()], &config).expect("embedding succeeds");
    assert_eq!(vectors.len(), 1);
}

/// Resident set size in KiB from `/proc/self/status`, or 0 where that file does not exist.
fn rss_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|line| line.starts_with("VmRSS:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|kib| kib.parse().ok())
        })
        .unwrap_or(0)
}

#[test]
fn evicting_one_model_drops_it_and_a_fresh_call_loads_it_again() {
    let _guard = serial();
    if should_skip() {
        return;
    }
    embeddings::clear_engine_cache();

    embed("fast");
    assert_eq!(embeddings::evict_model(&preset("fast")).unwrap(), 1);
    assert_eq!(
        embeddings::evict_model(&preset("fast")).unwrap(),
        0,
        "nothing is resident after eviction"
    );

    embed("fast");
    assert_eq!(
        embeddings::evict_model(&preset("fast")).unwrap(),
        1,
        "a fresh call loaded the model again"
    );
}

#[test]
fn a_bound_of_one_keeps_only_the_most_recently_used_model() {
    let _guard = serial();
    if should_skip() {
        return;
    }
    embeddings::clear_engine_cache();
    embeddings::set_engine_cache_limit(NonZeroUsize::new(1));

    embed("fast");
    embed("balanced");
    assert_eq!(
        embeddings::evict_model(&preset("fast")).unwrap(),
        0,
        "the bound dropped the older model"
    );
    assert_eq!(embeddings::evict_model(&preset("balanced")).unwrap(), 1);

    embeddings::set_engine_cache_limit(None);
}

#[test]
fn evicting_each_preset_drops_its_engine_and_reports_the_resident_set_size() {
    let _guard = serial();
    if should_skip() {
        return;
    }
    embeddings::clear_engine_cache();

    println!("rss_kib start={}", rss_kib());
    for name in ["fast", "balanced", "quality"] {
        embed(name);
        let loaded = rss_kib();
        assert_eq!(embeddings::evict_model(&preset(name)).unwrap(), 1);
        println!("rss_kib preset={name} loaded={loaded} evicted={}", rss_kib());
    }
    assert_eq!(embeddings::clear_engine_cache(), 0, "every engine was already evicted");

    embed("fast");
    assert!(
        xberg::clear_engine_caches() >= 1,
        "the crate-wide clear drops the loaded engine"
    );
    assert_eq!(
        embeddings::clear_engine_cache(),
        0,
        "nothing is left after the crate-wide clear"
    );
}

#[cfg(feature = "static-embeddings")]
#[test]
fn evicting_a_static_model_drops_it_and_a_fresh_call_loads_it_again() {
    let _guard = serial();
    if should_skip() {
        return;
    }
    embeddings::clear_engine_cache();

    embed("lightweight");
    assert_eq!(embeddings::evict_model(&preset("lightweight")).unwrap(), 1);
    assert_eq!(embeddings::evict_model(&preset("lightweight")).unwrap(), 0);

    embed("lightweight");
    assert_eq!(embeddings::clear_engine_cache(), 1);
}

#[cfg(feature = "reranker")]
#[test]
fn evicting_a_reranker_drops_it_and_a_fresh_call_loads_it_again() {
    use xberg::core::config::{RerankerConfig, RerankerModelType};
    use xberg::reranking;

    let _guard = serial();
    if should_skip() {
        return;
    }
    reranking::clear_engine_cache();

    let model = RerankerModelType::Preset {
        name: "bge-reranker-base".to_string(),
    };
    let config = RerankerConfig {
        model: model.clone(),
        cache_dir: cache_dir(),
        ..RerankerConfig::default()
    };
    let rerank = || {
        let ranked =
            xberg::rerank("query".to_string(), vec!["document".to_string()], &config).expect("rerank succeeds");
        assert_eq!(ranked.len(), 1);
    };

    rerank();
    assert_eq!(reranking::evict_model(&model).unwrap(), 1);
    assert_eq!(reranking::evict_model(&model).unwrap(), 0);

    rerank();
    assert_eq!(reranking::clear_engine_cache(), 1);
}
