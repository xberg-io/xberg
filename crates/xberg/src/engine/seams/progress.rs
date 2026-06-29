//! The [`ProgressSink`] seam: a sink for coarse extraction progress events.
//!
//! The in-core default is [`NoopProgressSink`], which discards every event —
//! exactly the behavior xberg exhibits today, where the extraction path emits
//! no progress. Alternative sinks (a channel, a logger, a metrics bridge)
//! implement this trait and are injected via
//! [`EngineBuilder::with_progress_sink`](super::super::EngineBuilder::with_progress_sink).

/// A coarse progress event emitted during extraction.
///
/// Intentionally minimal: a stage label plus optional detail and completion
/// fraction. Richer event shapes are layered on by the sink implementation.
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    /// Short, stable identifier for the current stage (e.g. `"download"`).
    pub stage: String,
    /// Optional human-readable detail for this event.
    pub message: Option<String>,
    /// Optional completion fraction in `0.0..=1.0`, when known.
    pub fraction: Option<f64>,
}

/// A sink for [`ProgressEvent`]s emitted during extraction.
///
/// # Thread safety
///
/// Implementations are `Send + Sync + 'static` and held behind
/// `Arc<dyn ProgressSink>`; they may be called concurrently.
pub trait ProgressSink: Send + Sync + 'static {
    /// Record a progress event. Implementations must not block.
    fn emit(&self, event: ProgressEvent);
}

/// In-core default: a sink that discards every event.
///
/// This reproduces today's behavior exactly — the default extraction path emits
/// no progress, so [`emit`](ProgressSink::emit) is inert.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProgressSink;

impl ProgressSink for NoopProgressSink {
    fn emit(&self, _event: ProgressEvent) {}
}
