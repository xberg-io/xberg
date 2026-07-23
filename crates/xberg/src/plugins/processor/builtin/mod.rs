//! Built-in post-processors that ship with xberg.
//!
//! Each submodule registers a single [`PostProcessor`](crate::plugins::PostProcessor)
//! implementation behind its own feature gate so non-OSS targets (no-ort-target,
//! wasm-target, android-target) compile out cleanly. Modules added by parallel
//! work streams plug in here without touching one another's files.

#[cfg(feature = "classification")]
pub mod classification;

#[cfg(feature = "classification")]
pub mod chunk_classification;

#[cfg(feature = "summarization")]
pub mod summarization;

#[cfg(feature = "translation")]
pub mod translation;

#[cfg(feature = "captioning")]
pub mod captioning;

#[cfg(feature = "qr-codes")]
pub mod qr;

#[cfg(feature = "ner")]
pub mod ner;

#[cfg(feature = "redaction")]
pub mod redaction;

/// Register every built-in post-processor enabled by the active feature set.
///
/// This is the single entry point that callers (including
/// `register_default_post_processors`) use to populate the global
/// post-processor registry with the in-tree built-ins. Each submodule's own
/// `register` function is gated by its feature flag so this aggregate stays
/// safe to call on any target.
pub fn register_builtin() -> crate::Result<()> {
    #[cfg(feature = "classification")]
    classification::register()?;

    #[cfg(feature = "classification")]
    chunk_classification::register()?;

    #[cfg(feature = "summarization")]
    summarization::register()?;

    #[cfg(feature = "translation")]
    translation::register()?;

    #[cfg(feature = "captioning")]
    captioning::register()?;

    #[cfg(feature = "qr-codes")]
    qr::register()?;

    #[cfg(feature = "ner")]
    ner::register()?;

    #[cfg(feature = "redaction")]
    redaction::register()?;

    Ok(())
}
